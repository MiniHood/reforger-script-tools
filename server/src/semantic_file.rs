//! Compiler-owned, immutable file semantic facts.
//!
//! It consumes the parser's typed declaration facade once and produces a
//! compact public contribution suitable for later workspace snapshots.

use crate::ast::{
    ClassMember, Declaration, DocCommentKind, FieldDecl, MethodDecl, MethodKind, TextValue,
};
use crate::lexer::TextSpan;
use crate::syntax::Parse;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SemanticDeclarationId(pub u32);

/// Snapshot-local identity for a conditional branch stack. The actual branch
/// details are interned once per file rather than cloned onto every record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SemanticConditionalContextId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticDeclarationKind {
    Class,
    TypeParameter,
    Enum,
    EnumMember,
    Typedef,
    Function,
    GlobalField,
    Field,
    Method,
    Constructor,
    Destructor,
    Parameter,
    LocalVariable,
    PreprocessorMacro,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticText {
    pub span: TextSpan,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticDocComment {
    pub span: TextSpan,
    pub kind: SemanticDocCommentKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticDocCommentKind {
    Line,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SemanticDeclarationDetail {
    pub type_text: Option<SemanticText>,
    pub return_type: Option<SemanticText>,
    pub base_type: Option<SemanticText>,
    pub default_value: Option<SemanticText>,
    pub enum_value: Option<SemanticText>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticDeclaration {
    pub id: SemanticDeclarationId,
    pub parent: Option<SemanticDeclarationId>,
    pub kind: SemanticDeclarationKind,
    pub name: Option<SemanticText>,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub detail: SemanticDeclarationDetail,
    pub modifiers: Vec<SemanticText>,
    pub attributes: Vec<SemanticText>,
    pub doc_comments: Vec<SemanticDocComment>,
    pub callable_form: Option<SemanticCallableForm>,
    pub conditional_context: SemanticConditionalContextId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticCallableForm {
    Implementation,
    Declaration,
    Prototype,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticConditionalBranchKind {
    If,
    Ifdef,
    Ifndef,
    Elif,
    Else,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticConditionalBranch {
    pub kind: SemanticConditionalBranchKind,
    pub directive_span: TextSpan,
    pub condition: Option<SemanticText>,
}

/// Complete declaration facts for one parsed file.  Local bindings and scope
/// regions are intentionally private; the public projection below retains
/// declaration facts needed to rebuild an external index without reopening the
/// source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SemanticFile {
    declarations: Vec<SemanticDeclaration>,
    conditional_contexts: Vec<Vec<SemanticConditionalBranch>>,
    local_regions: Vec<LocalSemanticRegion>,
    non_declaration_callable_fragments: usize,
    build_stats: SemanticBuildStats,
}

/// File-private callable scope facts. These never participate in workspace
/// contribution publication; bounded cursor queries consume them later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LocalSemanticRegion {
    callable: SemanticDeclarationId,
    span: TextSpan,
    bindings: Vec<SemanticDeclarationId>,
}

/// Source-free operation counters for scale regression tests and runtime
/// reports. They intentionally measure traversal work rather than elapsed
/// time so CI can prove the semantic builder remains near-linear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SemanticBuildStats {
    pub directive_lines: usize,
    /// Typed CST declarations consumed by the semantic builder's sole
    /// declaration traversal. This is deliberately separate from emitted
    /// declaration records because a class can emit members, parameters, and
    /// locals from one CST declaration visit.
    pub cst_declaration_visits: usize,
    pub declaration_records: usize,
    pub macro_definition_scan_lines: usize,
}

impl SemanticFile {
    /// Builds compiler semantic facts directly from parser output. The typed
    /// CST traversal is owned by `Parse`; production callers neither construct
    /// an AST-shaped input nor retain a second semantic representation between
    /// parsing and this build.
    pub fn build(source: &str, parse: &Parse) -> Self {
        let mut builder = SemanticFileBuilder {
            source,
            declarations: Vec::new(),
            local_regions: Vec::new(),
            non_declaration_callable_fragments: 0,
            cst_declaration_visits: 0,
            directive_contexts: DirectiveContextMap::for_source(source),
        };
        for declaration in parse.declaration_iter(source) {
            builder.cst_declaration_visits += 1;
            builder.add_declaration(declaration);
        }
        builder.add_preprocessor_macro_definitions();
        let directive_lines = builder.directive_contexts.line_count();
        let declaration_records = builder.declarations.len();
        let mut result = Self {
            declarations: builder.declarations,
            conditional_contexts: builder.directive_contexts.contexts(),
            local_regions: builder.local_regions,
            non_declaration_callable_fragments: builder.non_declaration_callable_fragments,
            build_stats: SemanticBuildStats {
                directive_lines,
                cst_declaration_visits: builder.cst_declaration_visits,
                declaration_records,
                macro_definition_scan_lines: source.lines().count(),
            },
        };
        // Keep the counters derived from the immutable output even if a future
        // builder implementation changes its internal storage order.
        result.build_stats.declaration_records = result.declarations.len();
        result
    }

    pub fn declarations(&self) -> &[SemanticDeclaration] {
        &self.declarations
    }

    pub fn declaration(&self, id: SemanticDeclarationId) -> Option<&SemanticDeclaration> {
        self.declarations.get(id.0 as usize)
    }

    pub fn conditional_context(
        &self,
        id: SemanticConditionalContextId,
    ) -> &[SemanticConditionalBranch] {
        self.conditional_contexts
            .get(id.0 as usize)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    #[cfg(test)]
    fn local_region_for_callable(
        &self,
        callable: SemanticDeclarationId,
    ) -> Option<&LocalSemanticRegion> {
        self.local_regions
            .iter()
            .find(|region| region.callable == callable)
    }

    pub fn non_declaration_callable_fragments(&self) -> usize {
        self.non_declaration_callable_fragments
    }

    pub fn build_stats(&self) -> SemanticBuildStats {
        self.build_stats
    }

    pub fn contribution(&self) -> FileContribution {
        FileContribution {
            schema_version: FILE_CONTRIBUTION_SCHEMA_VERSION,
            source_manifest_version: FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION,
            non_declaration_callable_fragments: self.non_declaration_callable_fragments,
            symbols: self
                .declarations
                .iter()
                .filter(|declaration| {
                    declaration.name.is_some()
                        && !matches!(declaration.kind, SemanticDeclarationKind::LocalVariable)
                })
                .map(|declaration| PublicSymbol {
                    id: declaration.id,
                    parent: declaration.parent,
                    kind: declaration.kind,
                    name: declaration.name.as_ref().map(|value| value.text.clone()),
                    container: declaration.parent.and_then(|parent| {
                        self.declaration(parent)
                            .and_then(|container| container.name.as_ref())
                            .map(|name| name.text.clone())
                    }),
                    detail: PublicSymbolDetail::from(&declaration.detail),
                    span: declaration.span,
                    selection_span: declaration.selection_span,
                    modifiers: declaration.modifiers.clone(),
                    attributes: declaration.attributes.clone(),
                    doc_comments: declaration.doc_comments.clone(),
                    conditional_context: self
                        .conditional_context(declaration.conditional_context)
                        .to_vec(),
                    callable_form: declaration.callable_form,
                })
                .collect(),
        }
        .with_contiguous_ids()
    }
}

/// The serializable, workspace-facing subset of a semantic file.  It contains
/// declarations that can participate in external lookup, not private scope
/// state or AST references.
pub const FILE_CONTRIBUTION_SCHEMA_VERSION: u32 = 3;
pub const FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FileContribution {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub source_manifest_version: u32,
    /// Parser fragments that look callable but do not form a declaration.
    /// Preserved for index diagnostics/telemetry without exposing local scope.
    pub non_declaration_callable_fragments: usize,
    pub symbols: Vec<PublicSymbol>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileContributionValidationError {
    UnsupportedSchema {
        found: u32,
        supported: u32,
    },
    UnsupportedSourceManifest {
        found: u32,
        supported: u32,
    },
    MissingName {
        kind: SemanticDeclarationKind,
    },
    NonContiguousSymbolId {
        expected: SemanticDeclarationId,
        found: SemanticDeclarationId,
    },
    MissingParent {
        symbol: SemanticDeclarationId,
        parent: SemanticDeclarationId,
    },
}

impl FileContribution {
    /// Validates a decoded contribution before it becomes visible to a
    /// workspace generation.  This keeps a stale or partial on-disk artifact
    /// from silently becoming a second, legacy fallback representation.
    pub fn validate(&self) -> Result<(), FileContributionValidationError> {
        if self.schema_version != FILE_CONTRIBUTION_SCHEMA_VERSION {
            return Err(FileContributionValidationError::UnsupportedSchema {
                found: self.schema_version,
                supported: FILE_CONTRIBUTION_SCHEMA_VERSION,
            });
        }
        if self.source_manifest_version != FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION {
            return Err(FileContributionValidationError::UnsupportedSourceManifest {
                found: self.source_manifest_version,
                supported: FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION,
            });
        }
        for symbol in &self.symbols {
            if symbol.name.as_deref().is_none_or(str::is_empty) {
                return Err(FileContributionValidationError::MissingName { kind: symbol.kind });
            }
        }
        let ids: BTreeSet<_> = self.symbols.iter().map(|symbol| symbol.id).collect();
        for (expected, symbol) in self.symbols.iter().enumerate() {
            let expected = SemanticDeclarationId(expected as u32);
            if symbol.id != expected {
                return Err(FileContributionValidationError::NonContiguousSymbolId {
                    expected,
                    found: symbol.id,
                });
            }
            if let Some(parent) = symbol.parent {
                if !ids.contains(&parent) {
                    return Err(FileContributionValidationError::MissingParent {
                        symbol: symbol.id,
                        parent,
                    });
                }
            }
        }
        Ok(())
    }

    /// Reassigns public declaration identities densely after a projection
    /// removes file-private records, preserving every retained parent edge.
    pub fn with_contiguous_ids(mut self) -> Self {
        let remapped_ids: BTreeMap<_, _> = self
            .symbols
            .iter()
            .enumerate()
            .map(|(next, symbol)| (symbol.id, SemanticDeclarationId(next as u32)))
            .collect();
        for symbol in &mut self.symbols {
            let original_id = symbol.id;
            let original_parent = symbol.parent;
            symbol.id = remapped_ids[&original_id];
            symbol.parent = original_parent.map(|parent| remapped_ids[&parent]);
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicSymbol {
    /// Snapshot-local declaration identity. It is valid only within this file
    /// contribution and lets an index reconstruct parent edges exactly.
    pub id: SemanticDeclarationId,
    pub parent: Option<SemanticDeclarationId>,
    pub kind: SemanticDeclarationKind,
    pub name: Option<String>,
    pub container: Option<String>,
    pub detail: PublicSymbolDetail,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub modifiers: Vec<SemanticText>,
    pub attributes: Vec<SemanticText>,
    pub doc_comments: Vec<SemanticDocComment>,
    pub conditional_context: Vec<SemanticConditionalBranch>,
    pub callable_form: Option<SemanticCallableForm>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PublicSymbolDetail {
    pub type_text: Option<PublicText>,
    pub return_type: Option<PublicText>,
    pub base_type: Option<PublicText>,
    pub default_value: Option<PublicText>,
    pub enum_value: Option<PublicText>,
}

/// Copied public text whose span is deliberately optional for compact runtime
/// caches. Source-built contributions always retain `Some(span)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicText {
    pub span: Option<TextSpan>,
    pub text: String,
}

impl From<&SemanticDeclarationDetail> for PublicSymbolDetail {
    fn from(detail: &SemanticDeclarationDetail) -> Self {
        Self {
            type_text: public_text(detail.type_text.as_ref()),
            return_type: public_text(detail.return_type.as_ref()),
            base_type: public_text(detail.base_type.as_ref()),
            default_value: public_text(detail.default_value.as_ref()),
            enum_value: public_text(detail.enum_value.as_ref()),
        }
    }
}

fn public_text(value: Option<&SemanticText>) -> Option<PublicText> {
    value.map(|value| PublicText {
        span: Some(value.span),
        text: value.text.clone(),
    })
}

struct SemanticFileBuilder<'source> {
    source: &'source str,
    declarations: Vec<SemanticDeclaration>,
    local_regions: Vec<LocalSemanticRegion>,
    non_declaration_callable_fragments: usize,
    cst_declaration_visits: usize,
    directive_contexts: DirectiveContextMap<'source>,
}

impl<'source> SemanticFileBuilder<'source> {
    fn add_preprocessor_macro_definitions(&mut self) {
        for (span, name) in preprocessor_macro_definitions(self.source) {
            self.push(
                None,
                SemanticDeclarationKind::PreprocessorMacro,
                Some(TextValue::from_span(self.source, name)),
                span,
                SemanticDeclarationDetail::default(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
        }
    }

    fn add_declaration(&mut self, declaration: Declaration<'source, '_>) {
        match declaration {
            Declaration::Class(class) => {
                let id = self.push(
                    None,
                    SemanticDeclarationKind::Class,
                    class.name(),
                    class.span(),
                    SemanticDeclarationDetail {
                        base_type: class.base_type().map(|value| self.text(value)),
                        ..Default::default()
                    },
                    text_values(class.modifiers(), self.source),
                    class
                        .attributes()
                        .into_iter()
                        .filter_map(|attribute| attribute.text())
                        .map(|value| self.attribute_text(value))
                        .collect(),
                    doc_comments(class.doc_comments()),
                );
                for parameter in class.type_parameters() {
                    self.push(
                        Some(id),
                        SemanticDeclarationKind::TypeParameter,
                        Some(parameter.name()),
                        parameter.span(),
                        SemanticDeclarationDetail {
                            type_text: parameter.constraint_text().map(|value| self.text(value)),
                            ..Default::default()
                        },
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                    );
                }
                for member in class.members() {
                    match member {
                        ClassMember::Field(field) => {
                            self.add_field(Some(id), SemanticDeclarationKind::Field, field)
                        }
                        ClassMember::Method(method) => {
                            let kind = match class.classify_method(method) {
                                MethodKind::Method => SemanticDeclarationKind::Method,
                                MethodKind::Constructor => SemanticDeclarationKind::Constructor,
                                MethodKind::Destructor => SemanticDeclarationKind::Destructor,
                            };
                            self.add_callable(Some(id), kind, method);
                        }
                        ClassMember::Empty(_) => {}
                    }
                }
            }
            Declaration::Enum(enumeration) => {
                let id = self.push(
                    None,
                    SemanticDeclarationKind::Enum,
                    enumeration.name(),
                    enumeration.span(),
                    SemanticDeclarationDetail {
                        base_type: enumeration.base_type().map(|value| self.text(value)),
                        ..Default::default()
                    },
                    Vec::new(),
                    enumeration
                        .attributes()
                        .into_iter()
                        .filter_map(|attribute| attribute.text())
                        .map(|value| self.attribute_text(value))
                        .collect(),
                    doc_comments(enumeration.doc_comments()),
                );
                for member in enumeration.members() {
                    self.push(
                        Some(id),
                        SemanticDeclarationKind::EnumMember,
                        member.name(),
                        member.span(),
                        SemanticDeclarationDetail {
                            enum_value: member.value_text().map(|value| self.text(value)),
                            ..Default::default()
                        },
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                    );
                }
            }
            Declaration::Typedef(typedef) => {
                self.push(
                    None,
                    SemanticDeclarationKind::Typedef,
                    typedef.name(),
                    typedef.text_span(),
                    SemanticDeclarationDetail {
                        type_text: typedef.type_text().map(|value| self.text(value)),
                        ..Default::default()
                    },
                    Vec::new(),
                    Vec::new(),
                    doc_comments(typedef.doc_comments()),
                );
            }
            Declaration::Function(function) => {
                self.add_callable(None, SemanticDeclarationKind::Function, function)
            }
            Declaration::Field(field) => {
                self.add_field(None, SemanticDeclarationKind::GlobalField, field)
            }
        }
    }

    fn add_field(
        &mut self,
        parent: Option<SemanticDeclarationId>,
        kind: SemanticDeclarationKind,
        field: FieldDecl<'source, '_>,
    ) {
        let modifiers = text_values(field.modifiers(), self.source);
        let attributes = field
            .attributes()
            .into_iter()
            .filter_map(|attribute| attribute.text())
            .map(|value| self.attribute_text(value))
            .collect::<Vec<_>>();
        let comments = doc_comments(field.doc_comments());
        for declarator in field.declarators() {
            self.push(
                parent,
                kind,
                Some(declarator.name()),
                declarator.span(),
                SemanticDeclarationDetail {
                    type_text: declarator.type_text().map(|value| self.text(value)),
                    ..Default::default()
                },
                modifiers.clone(),
                attributes.clone(),
                comments.clone(),
            );
        }
    }

    fn add_callable(
        &mut self,
        parent: Option<SemanticDeclarationId>,
        kind: SemanticDeclarationKind,
        method: MethodDecl<'source, '_>,
    ) {
        let id = self.push_with_callable_form(
            parent,
            kind,
            method.name(),
            method.span(),
            SemanticDeclarationDetail {
                return_type: method.return_type_text().map(|value| self.text(value)),
                ..Default::default()
            },
            text_values(method.modifiers(), self.source),
            method
                .attributes()
                .into_iter()
                .filter_map(|attribute| attribute.text())
                .map(|value| self.attribute_text(value))
                .collect(),
            doc_comments(method.doc_comments()),
            Some(callable_form(method)),
        );
        let mut bindings = Vec::new();
        for parameter in method.parameters() {
            bindings.push(self.push(
                Some(id),
                SemanticDeclarationKind::Parameter,
                parameter.name(),
                parameter.span(),
                SemanticDeclarationDetail {
                    type_text: parameter.type_text().map(|value| self.text(value)),
                    default_value: parameter.default_text().map(|value| self.text(value)),
                    ..Default::default()
                },
                text_values(parameter.modifiers(), self.source),
                Vec::new(),
                Vec::new(),
            ));
        }
        for local in method.local_variables() {
            bindings.push(self.push(
                Some(id),
                SemanticDeclarationKind::LocalVariable,
                Some(local.name()),
                local.span(),
                SemanticDeclarationDetail {
                    type_text: local.type_text().map(|value| self.text(value)),
                    default_value: local.default_text().map(|value| self.text(value)),
                    ..Default::default()
                },
                text_values(local.modifiers(), self.source),
                Vec::new(),
                Vec::new(),
            ));
        }
        self.local_regions.push(LocalSemanticRegion {
            callable: id,
            span: method.body_span().unwrap_or(method.span()),
            bindings,
        });
        self.non_declaration_callable_fragments += method.parameter_fragments().len();
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        parent: Option<SemanticDeclarationId>,
        kind: SemanticDeclarationKind,
        name: Option<TextValue<'source>>,
        span: TextSpan,
        detail: SemanticDeclarationDetail,
        modifiers: Vec<SemanticText>,
        attributes: Vec<SemanticText>,
        doc_comments: Vec<SemanticDocComment>,
    ) -> SemanticDeclarationId {
        self.push_with_callable_form(
            parent,
            kind,
            name,
            span,
            detail,
            modifiers,
            attributes,
            doc_comments,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn push_with_callable_form(
        &mut self,
        parent: Option<SemanticDeclarationId>,
        kind: SemanticDeclarationKind,
        name: Option<TextValue<'source>>,
        span: TextSpan,
        detail: SemanticDeclarationDetail,
        modifiers: Vec<SemanticText>,
        attributes: Vec<SemanticText>,
        doc_comments: Vec<SemanticDocComment>,
        callable_form: Option<SemanticCallableForm>,
    ) -> SemanticDeclarationId {
        let id = SemanticDeclarationId(self.declarations.len() as u32);
        let name = name.map(|value| self.text(value));
        self.declarations.push(SemanticDeclaration {
            id,
            parent,
            kind,
            selection_span: name.as_ref().map(|value| value.span).unwrap_or(span),
            name,
            span,
            detail,
            modifiers,
            attributes,
            doc_comments,
            callable_form,
            conditional_context: self.directive_contexts.context_id_at(span.start),
        });
        id
    }

    fn text(&self, value: TextValue<'source>) -> SemanticText {
        SemanticText {
            span: value.span,
            text: value.text().to_owned(),
        }
    }

    fn attribute_text(&self, value: TextValue<'source>) -> SemanticText {
        let value = self.text(value);
        SemanticText {
            text: format!("[{}]", value.text),
            ..value
        }
    }
}

fn callable_form(method: MethodDecl<'_, '_>) -> SemanticCallableForm {
    if method.body_span().is_some() {
        return SemanticCallableForm::Implementation;
    }
    if method
        .modifiers()
        .into_iter()
        .any(|modifier| matches!(modifier.text(), "proto" | "native" | "external"))
    {
        SemanticCallableForm::Prototype
    } else {
        SemanticCallableForm::Declaration
    }
}

fn preprocessor_macro_definitions(source: &str) -> Vec<(TextSpan, TextSpan)> {
    let mut result = Vec::new();
    let mut line_start = 0usize;
    for line in source.split_inclusive('\n') {
        let line_end = line_start + line.trim_end_matches(['\r', '\n']).len();
        if let Some(name) = preprocessor_define_name_span(source, line_start, line_end) {
            result.push((TextSpan::new(line_start, line_end), name));
        }
        line_start += line.len();
    }
    if line_start < source.len() {
        if let Some(name) = preprocessor_define_name_span(source, line_start, source.len()) {
            result.push((TextSpan::new(line_start, source.len()), name));
        }
    }
    result
}

fn preprocessor_define_name_span(
    source: &str,
    line_start: usize,
    line_end: usize,
) -> Option<TextSpan> {
    let line = &source[line_start..line_end];
    let trimmed_start = line
        .char_indices()
        .find_map(|(index, character)| (!character.is_whitespace()).then_some(index))?;
    let after_hash = line[trimmed_start..].strip_prefix('#')?;
    let after_hash_start = line_start + trimmed_start + 1;
    let directive_offset = after_hash
        .char_indices()
        .find_map(|(index, character)| (!character.is_whitespace()).then_some(index))?;
    let directive_start = after_hash_start + directive_offset;
    let directive = &source[directive_start..line_end];
    let define_tail = directive.strip_prefix("define")?;
    if define_tail
        .chars()
        .next()
        .is_some_and(is_identifier_continue)
    {
        return None;
    }
    let name_relative_start = define_tail
        .char_indices()
        .find_map(|(index, character)| (!character.is_whitespace()).then_some(index))?;
    let name_start = directive_start + "define".len() + name_relative_start;
    let mut name_end = name_start;
    for (index, character) in source[name_start..line_end].char_indices() {
        if (index == 0 && !is_identifier_start(character))
            || (index > 0 && !is_identifier_continue(character))
        {
            break;
        }
        name_end = name_start + index + character.len_utf8();
    }
    (name_end > name_start).then_some(TextSpan::new(name_start, name_end))
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

#[derive(Debug, Clone)]
struct DirectiveContextMap<'source> {
    line_contexts: Vec<(usize, SemanticConditionalContextId)>,
    contexts: Vec<Vec<SemanticConditionalBranch>>,
    _source: &'source str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawConditionalBranch {
    kind: SemanticConditionalBranchKind,
    directive_span: TextSpan,
    condition: Option<TextSpan>,
}

impl<'source> DirectiveContextMap<'source> {
    fn for_source(source: &'source str) -> Self {
        let mut line_contexts = Vec::new();
        let mut context = Vec::new();
        let mut raw_contexts = vec![Vec::new()];
        let mut line_start = 0usize;
        for line in source.split_inclusive('\n') {
            line_contexts.push((line_start, intern_context(&mut raw_contexts, &context)));
            let line_end = line_start + line.trim_end_matches(['\r', '\n']).len();
            apply_preprocessor_line(source, line_start, line_end, &mut context);
            line_start += line.len();
        }
        if line_start < source.len() {
            line_contexts.push((line_start, intern_context(&mut raw_contexts, &context)));
            apply_preprocessor_line(source, line_start, source.len(), &mut context);
        }
        Self {
            line_contexts,
            contexts: raw_contexts
                .into_iter()
                .map(|context| semantic_context(source, context))
                .collect(),
            _source: source,
        }
    }

    fn context_id_at(&self, offset: usize) -> SemanticConditionalContextId {
        let index = self
            .line_contexts
            .partition_point(|(line_start, _)| *line_start <= offset)
            .saturating_sub(1);
        self.line_contexts
            .get(index)
            .map(|(_, context)| *context)
            .unwrap_or(SemanticConditionalContextId(0))
    }

    fn contexts(&self) -> Vec<Vec<SemanticConditionalBranch>> {
        self.contexts.clone()
    }

    fn line_count(&self) -> usize {
        self.line_contexts.len()
    }
}

fn intern_context(
    contexts: &mut Vec<Vec<RawConditionalBranch>>,
    context: &[RawConditionalBranch],
) -> SemanticConditionalContextId {
    if let Some(index) = contexts.iter().position(|candidate| candidate == context) {
        return SemanticConditionalContextId(index as u32);
    }
    let id = SemanticConditionalContextId(contexts.len() as u32);
    contexts.push(context.to_vec());
    id
}

fn semantic_context(
    source: &str,
    context: Vec<RawConditionalBranch>,
) -> Vec<SemanticConditionalBranch> {
    context
        .into_iter()
        .map(|branch| SemanticConditionalBranch {
            kind: branch.kind,
            directive_span: branch.directive_span,
            condition: branch.condition.map(|span| SemanticText {
                span,
                text: source[span.start..span.end].to_owned(),
            }),
        })
        .collect()
}

fn apply_preprocessor_line(
    source: &str,
    line_start: usize,
    line_end: usize,
    context: &mut Vec<RawConditionalBranch>,
) {
    let line = &source[line_start..line_end];
    let leading_whitespace = line.len() - line.trim_start().len();
    let directive_start = line_start + leading_whitespace;
    let trimmed = &source[directive_start..line_end];
    for (text, kind) in [
        ("#ifdef", SemanticConditionalBranchKind::Ifdef),
        ("#ifndef", SemanticConditionalBranchKind::Ifndef),
        ("#elif", SemanticConditionalBranchKind::Elif),
        ("#else", SemanticConditionalBranchKind::Else),
        ("#endif", SemanticConditionalBranchKind::If),
        ("#if", SemanticConditionalBranchKind::If),
    ] {
        if !trimmed.starts_with(text) {
            continue;
        }
        if text == "#endif" {
            context.pop();
            return;
        }
        let directive_span = TextSpan::new(directive_start, directive_start + text.len());
        let condition = if text == "#else" {
            context.last().and_then(|branch| branch.condition)
        } else {
            preprocessor_condition_span(source, directive_span.end, line_end)
        };
        let branch = RawConditionalBranch {
            kind,
            directive_span,
            condition,
        };
        if matches!(text, "#elif" | "#else") {
            if let Some(previous) = context.last_mut() {
                *previous = branch;
            } else {
                context.push(branch);
            }
        } else {
            context.push(branch);
        }
        return;
    }
}

fn preprocessor_condition_span(source: &str, start: usize, line_end: usize) -> Option<TextSpan> {
    let value = &source[start..line_end];
    let leading = value.len() - value.trim_start().len();
    let trailing = value.len() - value.trim_end().len();
    let condition_start = start + leading;
    let condition_end = line_end.saturating_sub(trailing);
    (condition_start < condition_end).then_some(TextSpan::new(condition_start, condition_end))
}

fn text_values(values: Vec<TextValue<'_>>, source: &str) -> Vec<SemanticText> {
    values
        .into_iter()
        .map(|value| SemanticText {
            span: value.span,
            text: source[value.span.start..value.span.end].to_owned(),
        })
        .collect()
}

fn doc_comments(values: Vec<crate::ast::DocComment<'_>>) -> Vec<SemanticDocComment> {
    values
        .into_iter()
        .map(|value| SemanticDocComment {
            span: value.span(),
            kind: match value.kind() {
                DocCommentKind::Line => SemanticDocCommentKind::Line,
                DocCommentKind::Block => SemanticDocCommentKind::Block,
            },
            text: value.text().to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;

    fn semantic(source: &str) -> SemanticFile {
        let parse = parse_source(source);
        SemanticFile::build(source, &parse)
    }

    #[test]
    fn builds_declaration_tree_without_the_legacy_catalog() {
        let file = semantic("class Widget : Base { int count; void Run(string label = \"x\") {} }\nenum Mode { Idle = 2 }\ntypedef int Count;\nvoid Start() {}\n");
        let names: Vec<_> = file
            .declarations()
            .iter()
            .filter_map(|declaration| {
                declaration
                    .name
                    .as_ref()
                    .map(|name| (declaration.kind, name.text.as_str()))
            })
            .collect();
        assert_eq!(
            names,
            vec![
                (SemanticDeclarationKind::Class, "Widget"),
                (SemanticDeclarationKind::Field, "count"),
                (SemanticDeclarationKind::Method, "Run"),
                (SemanticDeclarationKind::Parameter, "label"),
                (SemanticDeclarationKind::Enum, "Mode"),
                (SemanticDeclarationKind::EnumMember, "Idle"),
                (SemanticDeclarationKind::Typedef, "Count"),
                (SemanticDeclarationKind::Function, "Start"),
            ]
        );
        assert_eq!(
            file.declarations()[0]
                .detail
                .base_type
                .as_ref()
                .unwrap()
                .text,
            "Base"
        );
        assert_eq!(
            file.declarations()[3]
                .detail
                .default_value
                .as_ref()
                .unwrap()
                .text,
            "\"x\""
        );
    }

    #[test]
    fn contribution_keeps_workspace_visible_symbols_and_signature_facts() {
        let file = semantic("class Widget { int count; void Run(string label) {} }");
        let contribution = file.contribution();
        assert_eq!(contribution.symbols.len(), 4);
        assert_eq!(contribution.symbols[0].name.as_deref(), Some("Widget"));
        assert_eq!(contribution.symbols[1].name.as_deref(), Some("count"));
        assert_eq!(contribution.symbols[1].container.as_deref(), Some("Widget"));
        assert_eq!(
            contribution.symbols[2]
                .detail
                .return_type
                .as_ref()
                .map(|value| value.text.as_str()),
            Some("void")
        );
        assert!(contribution
            .symbols
            .iter()
            .any(|symbol| symbol.name.as_deref() == Some("label")));
        assert_eq!(
            contribution.schema_version,
            FILE_CONTRIBUTION_SCHEMA_VERSION
        );
        assert_eq!(contribution.validate(), Ok(()));
    }

    #[test]
    fn contribution_rejects_stale_or_partial_workspace_artifacts() {
        let mut contribution = semantic("class Widget {}").contribution();
        contribution.schema_version = FILE_CONTRIBUTION_SCHEMA_VERSION + 1;
        assert_eq!(
            contribution.validate(),
            Err(FileContributionValidationError::UnsupportedSchema {
                found: FILE_CONTRIBUTION_SCHEMA_VERSION + 1,
                supported: FILE_CONTRIBUTION_SCHEMA_VERSION,
            })
        );

        contribution.source_manifest_version = FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION;
        contribution.schema_version = FILE_CONTRIBUTION_SCHEMA_VERSION;
        contribution.source_manifest_version = FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION + 1;
        assert_eq!(
            contribution.validate(),
            Err(FileContributionValidationError::UnsupportedSourceManifest {
                found: FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION + 1,
                supported: FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION,
            })
        );

        contribution.source_manifest_version = FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION;
        contribution.schema_version = FILE_CONTRIBUTION_SCHEMA_VERSION;
        contribution.symbols[0].name = None;
        assert_eq!(
            contribution.validate(),
            Err(FileContributionValidationError::MissingName {
                kind: SemanticDeclarationKind::Class,
            })
        );
    }

    #[test]
    fn contribution_compacts_public_ids_after_local_declarations() {
        let file = semantic(include_str!(
            "../../tools/fixtures/index/contribution_public_ids_after_local.c"
        ));
        let contribution = file.contribution();
        assert_eq!(
            contribution
                .symbols
                .iter()
                .map(|symbol| symbol.id.0)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(contribution.validate(), Ok(()));
    }

    #[test]
    fn semantic_build_operation_counts_scale_linearly_with_declarations() {
        const UNIT: &str =
            include_str!("../../tools/fixtures/semantic/semantic_scale_declaration_unit.c");

        fn source(scale: usize) -> String {
            UNIT.repeat(scale)
        }

        let one = semantic(&source(1)).build_stats();
        let two = semantic(&source(2)).build_stats();
        let four = semantic(&source(4)).build_stats();

        assert!(one.directive_lines > 0);
        assert!(one.cst_declaration_visits > 0);
        assert!(one.declaration_records > 0);
        assert!(one.macro_definition_scan_lines > 0);
        assert_eq!(two.directive_lines, one.directive_lines * 2);
        assert_eq!(four.directive_lines, one.directive_lines * 4);
        assert_eq!(two.cst_declaration_visits, one.cst_declaration_visits * 2);
        assert_eq!(four.cst_declaration_visits, one.cst_declaration_visits * 4);
        assert_eq!(two.declaration_records, one.declaration_records * 2);
        assert_eq!(four.declaration_records, one.declaration_records * 4);
        assert_eq!(
            two.macro_definition_scan_lines,
            one.macro_definition_scan_lines * 2
        );
        assert_eq!(
            four.macro_definition_scan_lines,
            one.macro_definition_scan_lines * 4
        );
    }

    #[test]
    fn retains_callable_shape_and_local_binding_facts() {
        let file = semantic(
            "class Widget { proto void Declared(); void Run() { int first = 1; for (int second = 0; second < 1; second++) {} } }",
        );
        let declared = file
            .declarations()
            .iter()
            .find(|declaration| {
                declaration
                    .name
                    .as_ref()
                    .is_some_and(|name| name.text == "Declared")
            })
            .unwrap();
        assert_eq!(
            declared.callable_form,
            Some(SemanticCallableForm::Prototype)
        );

        let locals: Vec<_> = file
            .declarations()
            .iter()
            .filter(|declaration| declaration.kind == SemanticDeclarationKind::LocalVariable)
            .filter_map(|declaration| declaration.name.as_ref().map(|name| name.text.as_str()))
            .collect();
        assert_eq!(locals, vec!["first", "second"]);
    }

    #[test]
    fn records_conditional_context_once_for_nested_branches() {
        let file = semantic("#ifdef ENABLE\nclass Enabled {}\n#else\nclass Disabled {}\n#endif\n");
        let enabled = file
            .declarations()
            .iter()
            .find(|declaration| {
                declaration
                    .name
                    .as_ref()
                    .is_some_and(|name| name.text == "Enabled")
            })
            .unwrap();
        let enabled_context = file.conditional_context(enabled.conditional_context);
        assert_eq!(enabled_context.len(), 1);
        assert_eq!(
            enabled_context[0].kind,
            SemanticConditionalBranchKind::Ifdef
        );
        assert_eq!(
            enabled_context[0]
                .condition
                .as_ref()
                .map(|condition| condition.text.as_str()),
            Some("ENABLE")
        );
        let disabled = file
            .declarations()
            .iter()
            .find(|declaration| {
                declaration
                    .name
                    .as_ref()
                    .is_some_and(|name| name.text == "Disabled")
            })
            .unwrap();
        assert_eq!(
            file.conditional_context(disabled.conditional_context)[0].kind,
            SemanticConditionalBranchKind::Else
        );
        assert_ne!(enabled.conditional_context, disabled.conditional_context);
    }

    #[test]
    fn groups_callable_locals_into_private_regions() {
        let file = semantic(
            "void Run(int parameter) { int first = 1; for (int second = 0; second < 1; second++) {} }",
        );
        let callable = file
            .declarations()
            .iter()
            .find(|declaration| declaration.kind == SemanticDeclarationKind::Function)
            .unwrap();
        let region = file.local_region_for_callable(callable.id).unwrap();
        assert_eq!(region.bindings.len(), 3);
        assert!(region.span.start >= callable.span.start);
        assert!(region.span.end <= callable.span.end);
    }
}
