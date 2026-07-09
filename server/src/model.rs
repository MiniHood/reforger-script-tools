use crate::ast::{
    AstSourceFile, ClassMember, Declaration, DocComment, DocCommentKind, FieldDecl, MethodDecl,
    MethodKind, TextValue,
};
use crate::lexer::TextSpan;
use std::path::{Path, PathBuf};

pub const SOURCE_PRIORITY_UNKNOWN: u16 = 0;
pub const SOURCE_PRIORITY_FIXTURE: u16 = 50;
pub const SOURCE_PRIORITY_GAME_DATA: u16 = 100;
pub const SOURCE_PRIORITY_WORKSPACE: u16 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceKind {
    Unknown,
    GameData,
    Workspace,
    Fixture,
}

impl SourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::GameData => "GameData",
            Self::Workspace => "Workspace",
            Self::Fixture => "Fixture",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceCategory {
    Workspace,
    Game,
    GameCode,
    GameLib,
    Core,
    Generated,
    Workbench,
    DocsDoxygen,
    TestAutotest,
    Unknown,
}

impl SourceCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Game => "game",
            Self::GameCode => "gamecode",
            Self::GameLib => "gamelib",
            Self::Core => "core",
            Self::Generated => "generated",
            Self::Workbench => "workbench",
            Self::DocsDoxygen => "docs/doxygen",
            Self::TestAutotest => "test/autotest",
            Self::Unknown => "unknown",
        }
    }

    pub const fn is_editor_completion_default(self) -> bool {
        matches!(
            self,
            Self::Workspace
                | Self::Game
                | Self::GameCode
                | Self::GameLib
                | Self::Core
                | Self::Generated
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileMetadata {
    pub kind: SourceKind,
    pub category: SourceCategory,
    pub absolute_path: Option<PathBuf>,
    pub root_path: Option<PathBuf>,
    pub relative_path: Option<PathBuf>,
    pub priority: u16,
}

impl SourceFileMetadata {
    pub const fn unknown() -> Self {
        Self {
            kind: SourceKind::Unknown,
            category: SourceCategory::Unknown,
            absolute_path: None,
            root_path: None,
            relative_path: None,
            priority: SOURCE_PRIORITY_UNKNOWN,
        }
    }
}

pub fn source_category_for_path(kind: SourceKind, path: Option<&Path>) -> SourceCategory {
    if kind == SourceKind::Workspace {
        return SourceCategory::Workspace;
    }

    let path = path
        .map(|path| path.to_string_lossy().replace('\\', "/").to_lowercase())
        .unwrap_or_default();

    if path.contains("/generated/") || path.starts_with("generated/") {
        SourceCategory::Generated
    } else if path.contains("docs") || path.contains("doxygen") {
        SourceCategory::DocsDoxygen
    } else if path.starts_with("autotest/")
        || path.contains("/autotest/")
        || path.contains("/tests/")
    {
        SourceCategory::TestAutotest
    } else if path.starts_with("workbench") {
        SourceCategory::Workbench
    } else if path.starts_with("gamecode/") {
        SourceCategory::GameCode
    } else if path.starts_with("gamelib/") {
        SourceCategory::GameLib
    } else if path.starts_with("game/") {
        SourceCategory::Game
    } else if path.starts_with("core/") {
        SourceCategory::Core
    } else {
        SourceCategory::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SymbolKind {
    Class,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolDetail {
    pub type_text: Option<TextSpan>,
    pub return_type_text: Option<TextSpan>,
    pub base_type: Option<TextSpan>,
    pub default_text: Option<TextSpan>,
    pub enum_value_text: Option<TextSpan>,
}

impl SymbolDetail {
    pub const fn empty() -> Self {
        Self {
            type_text: None,
            return_type_text: None,
            base_type: None,
            default_text: None,
            enum_value_text: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocCommentRecord {
    pub span: TextSpan,
    pub kind: DocCommentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreprocessorBranchKind {
    If,
    Ifdef,
    Ifndef,
    Elif,
    Else,
}

impl PreprocessorBranchKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::If => "#if",
            Self::Ifdef => "#ifdef",
            Self::Ifndef => "#ifndef",
            Self::Elif => "#elif",
            Self::Else => "#else",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalBranch {
    pub kind: PreprocessorBranchKind,
    pub directive_span: TextSpan,
    pub condition: Option<TextSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CallableForm {
    Implementation,
    Declaration,
    Prototype,
}

impl CallableForm {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implementation => "implementation",
            Self::Declaration => "declaration",
            Self::Prototype => "prototype",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRecord {
    pub id: SymbolId,
    pub parent: Option<SymbolId>,
    pub kind: SymbolKind,
    pub name: Option<TextSpan>,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub detail: SymbolDetail,
    pub attributes: Vec<TextSpan>,
    pub modifiers: Vec<TextSpan>,
    pub doc_comments: Vec<DocCommentRecord>,
    pub conditional_context: Vec<ConditionalBranch>,
    pub callable_form: Option<CallableForm>,
}

pub struct SymbolCatalog<'source> {
    source: &'source str,
    metadata: SourceFileMetadata,
    records: Vec<SymbolRecord>,
    non_declaration_callable_fragments: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeShape<'source> {
    source: &'source str,
    span: TextSpan,
    qualifiers: Vec<TextSpan>,
    base_name: Option<TextSpan>,
    generic_args: Vec<TypeShape<'source>>,
    array_suffixes: Vec<TextSpan>,
}

impl<'source> TypeShape<'source> {
    pub const fn span(&self) -> TextSpan {
        self.span
    }

    pub fn text(&self) -> &'source str {
        &self.source[self.span.start..self.span.end]
    }

    pub fn qualifier_spans(&self) -> &[TextSpan] {
        &self.qualifiers
    }

    pub fn qualifier_texts(&self) -> Vec<&'source str> {
        self.qualifiers
            .iter()
            .map(|span| &self.source[span.start..span.end])
            .collect()
    }

    pub const fn base_name_span(&self) -> Option<TextSpan> {
        self.base_name
    }

    pub fn base_name_text(&self) -> Option<&'source str> {
        self.base_name
            .map(|span| &self.source[span.start..span.end])
    }

    pub fn generic_args(&self) -> &[TypeShape<'source>] {
        &self.generic_args
    }

    pub fn array_suffix_spans(&self) -> &[TextSpan] {
        &self.array_suffixes
    }

    pub fn array_suffix_texts(&self) -> Vec<&'source str> {
        self.array_suffixes
            .iter()
            .map(|span| &self.source[span.start..span.end])
            .collect()
    }
}

impl<'source> SymbolCatalog<'source> {
    pub fn from_ast(source: &'source str, ast: &AstSourceFile<'source, '_>) -> Self {
        Self::from_ast_with_metadata(source, ast, SourceFileMetadata::unknown())
    }

    pub fn from_ast_with_metadata(
        source: &'source str,
        ast: &AstSourceFile<'source, '_>,
        metadata: SourceFileMetadata,
    ) -> Self {
        let mut builder = SymbolCatalogBuilder {
            source,
            metadata,
            records: Vec::new(),
            non_declaration_callable_fragments: 0,
        };
        builder.add_ast(ast);
        builder.finish()
    }

    pub const fn source(&self) -> &'source str {
        self.source
    }

    pub const fn metadata(&self) -> &SourceFileMetadata {
        &self.metadata
    }

    pub fn records(&self) -> &[SymbolRecord] {
        &self.records
    }

    pub fn record(&self, id: SymbolId) -> Option<&SymbolRecord> {
        self.records.get(id.0)
    }

    pub fn text(&self, span: TextSpan) -> &'source str {
        &self.source[span.start..span.end]
    }

    pub fn record_name(&self, record: &SymbolRecord) -> Option<&'source str> {
        record.name.map(|span| self.text(span))
    }

    pub fn attribute_name(&self, span: TextSpan) -> Option<&'source str> {
        attribute_name(self.text(span))
    }

    pub fn record_attribute_names(&self, record: &SymbolRecord) -> Vec<&'source str> {
        record
            .attributes
            .iter()
            .filter_map(|span| self.attribute_name(*span))
            .collect()
    }

    pub fn type_shape(&self, span: TextSpan) -> TypeShape<'source> {
        parse_type_shape(self.source, trim_span(self.source, span))
    }

    pub fn record_type_shape(&self, record: &SymbolRecord) -> Option<TypeShape<'source>> {
        let type_text = record.detail.type_text?;
        let mut shape = self.type_shape(type_text);
        if let Some(name) = record.name {
            shape
                .array_suffixes
                .extend(array_suffixes_after_name(self.source, record.span, name));
        }
        Some(shape)
    }

    pub const fn non_declaration_callable_fragments(&self) -> usize {
        self.non_declaration_callable_fragments
    }
}

struct SymbolCatalogBuilder<'source> {
    source: &'source str,
    metadata: SourceFileMetadata,
    records: Vec<SymbolRecord>,
    non_declaration_callable_fragments: usize,
}

impl<'source> SymbolCatalogBuilder<'source> {
    fn add_ast(&mut self, ast: &AstSourceFile<'source, '_>) {
        for declaration in ast.declarations() {
            self.add_declaration(declaration);
        }
    }

    fn finish(self) -> SymbolCatalog<'source> {
        SymbolCatalog {
            source: self.source,
            metadata: self.metadata,
            records: self.records,
            non_declaration_callable_fragments: self.non_declaration_callable_fragments,
        }
    }

    fn add_declaration(&mut self, declaration: Declaration<'source, '_>) {
        match declaration {
            Declaration::Class(class) => {
                let class_id = self.push_record(NewSymbol {
                    parent: None,
                    kind: SymbolKind::Class,
                    name: class.name(),
                    span: class.span(),
                    detail: SymbolDetail {
                        base_type: class.base_type().map(|value| value.span),
                        ..SymbolDetail::empty()
                    },
                    attributes: spans(class.attributes()),
                    modifiers: text_spans(class.modifiers()),
                    doc_comments: doc_comment_records(class.doc_comments()),
                    callable_form: None,
                });

                for member in class.members() {
                    match member {
                        ClassMember::Field(field) => {
                            self.add_field(Some(class_id), SymbolKind::Field, field);
                        }
                        ClassMember::Method(method) => {
                            let method_kind = match class.classify_method(method) {
                                MethodKind::Method => SymbolKind::Method,
                                MethodKind::Constructor => SymbolKind::Constructor,
                                MethodKind::Destructor => SymbolKind::Destructor,
                            };
                            self.add_callable(Some(class_id), method_kind, method);
                        }
                        ClassMember::Empty(_) => {}
                    }
                }
            }
            Declaration::Enum(enum_decl) => {
                let enum_id = self.push_record(NewSymbol {
                    parent: None,
                    kind: SymbolKind::Enum,
                    name: enum_decl.name(),
                    span: enum_decl.span(),
                    detail: SymbolDetail::empty(),
                    attributes: spans(enum_decl.attributes()),
                    modifiers: Vec::new(),
                    doc_comments: doc_comment_records(enum_decl.doc_comments()),
                    callable_form: None,
                });

                for member in enum_decl.members() {
                    self.push_record(NewSymbol {
                        parent: Some(enum_id),
                        kind: SymbolKind::EnumMember,
                        name: member.name(),
                        span: member.span(),
                        detail: SymbolDetail {
                            enum_value_text: member.value_text().map(|value| value.span),
                            ..SymbolDetail::empty()
                        },
                        attributes: Vec::new(),
                        modifiers: Vec::new(),
                        doc_comments: Vec::new(),
                        callable_form: None,
                    });
                }
            }
            Declaration::Typedef(typedef_decl) => {
                self.push_record(NewSymbol {
                    parent: None,
                    kind: SymbolKind::Typedef,
                    name: typedef_decl.name(),
                    span: typedef_decl.text_span(),
                    detail: SymbolDetail {
                        type_text: typedef_decl.type_text().map(|value| value.span),
                        ..SymbolDetail::empty()
                    },
                    attributes: Vec::new(),
                    modifiers: Vec::new(),
                    doc_comments: doc_comment_records(typedef_decl.doc_comments()),
                    callable_form: None,
                });
            }
            Declaration::Function(function) => {
                self.add_callable(None, SymbolKind::Function, function);
            }
            Declaration::Field(field) => {
                self.add_field(None, SymbolKind::GlobalField, field);
            }
        }
    }

    fn add_field(
        &mut self,
        parent: Option<SymbolId>,
        kind: SymbolKind,
        field: FieldDecl<'source, '_>,
    ) {
        let attributes = spans(field.attributes());
        let modifiers = text_spans(field.modifiers());
        let doc_comments = doc_comment_records(field.doc_comments());

        for declarator in field.declarators() {
            self.push_record(NewSymbol {
                parent,
                kind,
                name: Some(declarator.name()),
                span: declarator.span(),
                detail: SymbolDetail {
                    type_text: declarator.type_text().map(|value| value.span),
                    ..SymbolDetail::empty()
                },
                attributes: attributes.clone(),
                modifiers: modifiers.clone(),
                doc_comments: doc_comments.clone(),
                callable_form: None,
            });
        }
    }

    fn add_callable(
        &mut self,
        parent: Option<SymbolId>,
        kind: SymbolKind,
        method: MethodDecl<'source, '_>,
    ) {
        let callable_id = self.push_record(NewSymbol {
            parent,
            kind,
            name: method.name(),
            span: method.span(),
            detail: SymbolDetail {
                return_type_text: method.return_type_text().map(|value| value.span),
                ..SymbolDetail::empty()
            },
            attributes: spans(method.attributes()),
            modifiers: text_spans(method.modifiers()),
            doc_comments: doc_comment_records(method.doc_comments()),
            callable_form: Some(callable_form(method)),
        });

        for parameter in method.parameters() {
            self.push_record(NewSymbol {
                parent: Some(callable_id),
                kind: SymbolKind::Parameter,
                name: parameter.name(),
                span: parameter.span(),
                detail: SymbolDetail {
                    type_text: parameter.type_text().map(|value| value.span),
                    default_text: parameter.default_text().map(|value| value.span),
                    ..SymbolDetail::empty()
                },
                attributes: Vec::new(),
                modifiers: text_spans(parameter.modifiers()),
                doc_comments: Vec::new(),
                callable_form: None,
            });
        }

        for local in method.local_variables() {
            self.push_record(NewSymbol {
                parent: Some(callable_id),
                kind: SymbolKind::LocalVariable,
                name: Some(local.name()),
                span: local.span(),
                detail: SymbolDetail {
                    type_text: local.type_text().map(|value| value.span),
                    default_text: local.default_text().map(|value| value.span),
                    ..SymbolDetail::empty()
                },
                attributes: Vec::new(),
                modifiers: text_spans(local.modifiers()),
                doc_comments: Vec::new(),
                callable_form: None,
            });
        }

        self.non_declaration_callable_fragments += method.parameter_fragments().len();
    }

    fn push_record(&mut self, symbol: NewSymbol<'source>) -> SymbolId {
        let id = SymbolId(self.records.len());
        let name = symbol.name.map(|value| value.span);
        let selection_span = name.unwrap_or(symbol.span);
        self.records.push(SymbolRecord {
            id,
            parent: symbol.parent,
            kind: symbol.kind,
            name,
            span: symbol.span,
            selection_span,
            detail: symbol.detail,
            attributes: symbol.attributes,
            modifiers: symbol.modifiers,
            doc_comments: symbol.doc_comments,
            conditional_context: conditional_context_at(self.source, symbol.span.start),
            callable_form: symbol.callable_form,
        });
        id
    }
}

struct NewSymbol<'source> {
    parent: Option<SymbolId>,
    kind: SymbolKind,
    name: Option<TextValue<'source>>,
    span: TextSpan,
    detail: SymbolDetail,
    attributes: Vec<TextSpan>,
    modifiers: Vec<TextSpan>,
    doc_comments: Vec<DocCommentRecord>,
    callable_form: Option<CallableForm>,
}

fn spans(attributes: Vec<crate::ast::Attribute<'_, '_>>) -> Vec<TextSpan> {
    attributes
        .into_iter()
        .map(|attribute| attribute.span())
        .collect()
}

fn text_spans(values: Vec<TextValue<'_>>) -> Vec<TextSpan> {
    values.into_iter().map(|value| value.span).collect()
}

fn doc_comment_records(comments: Vec<DocComment<'_>>) -> Vec<DocCommentRecord> {
    comments
        .into_iter()
        .map(|comment| DocCommentRecord {
            span: comment.span(),
            kind: comment.kind(),
        })
        .collect()
}

fn callable_form(method: MethodDecl<'_, '_>) -> CallableForm {
    if method.body_span().is_some() {
        return CallableForm::Implementation;
    }

    if method
        .modifiers()
        .into_iter()
        .any(|modifier| matches!(modifier.text(), "proto" | "native" | "external"))
    {
        CallableForm::Prototype
    } else {
        CallableForm::Declaration
    }
}

fn conditional_context_at(source: &str, offset: usize) -> Vec<ConditionalBranch> {
    let mut context = Vec::new();
    let mut line_start = 0usize;

    while line_start < offset && line_start < source.len() {
        let line_end = source[line_start..]
            .find('\n')
            .map(|index| line_start + index)
            .unwrap_or(source.len());
        if line_start >= offset {
            break;
        }
        apply_preprocessor_line(source, line_start, line_end, &mut context);
        if line_end == source.len() {
            break;
        }
        line_start = line_end + 1;
    }

    context
}

fn apply_preprocessor_line(
    source: &str,
    line_start: usize,
    line_end: usize,
    context: &mut Vec<ConditionalBranch>,
) {
    let line = &source[line_start..line_end];
    let leading_whitespace = line.len() - line.trim_start().len();
    let directive_start = line_start + leading_whitespace;
    let trimmed = &source[directive_start..line_end];

    for (text, kind) in [
        ("#ifdef", PreprocessorBranchKind::Ifdef),
        ("#ifndef", PreprocessorBranchKind::Ifndef),
        ("#elif", PreprocessorBranchKind::Elif),
        ("#else", PreprocessorBranchKind::Else),
        ("#endif", PreprocessorBranchKind::If),
        ("#if", PreprocessorBranchKind::If),
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
        let branch = ConditionalBranch {
            kind,
            directive_span,
            condition,
        };

        if matches!(
            kind,
            PreprocessorBranchKind::Elif | PreprocessorBranchKind::Else
        ) {
            if let Some(current) = context.last_mut() {
                *current = branch;
            } else {
                context.push(branch);
            }
        } else {
            context.push(branch);
        }
        return;
    }
}

fn preprocessor_condition_span(source: &str, start: usize, end: usize) -> Option<TextSpan> {
    let span = trim_span(source, TextSpan::new(start, end));
    (!span.is_empty()).then_some(span)
}

fn attribute_name(text: &str) -> Option<&str> {
    let trimmed = text.trim_start();
    let trimmed = trimmed.strip_prefix('[').unwrap_or(trimmed).trim_start();
    let mut end = 0usize;
    for (index, value) in trimmed.char_indices() {
        if value.is_ascii_alphanumeric() || value == '_' {
            end = index + value.len_utf8();
        } else {
            break;
        }
    }

    if end == 0 {
        None
    } else {
        Some(&trimmed[..end])
    }
}

fn parse_type_shape(source: &str, span: TextSpan) -> TypeShape<'_> {
    let span = trim_span(source, span);
    let mut position = span.start;
    let mut qualifiers = Vec::new();

    loop {
        position = skip_whitespace(source, position, span.end);
        let Some(identifier) = identifier_span_at(source, position, span.end) else {
            break;
        };
        if !is_type_qualifier(&source[identifier.start..identifier.end]) {
            break;
        }
        qualifiers.push(identifier);
        position = identifier.end;
    }

    position = skip_whitespace(source, position, span.end);
    let base_name = identifier_span_at(source, position, span.end);
    if let Some(base) = base_name {
        position = base.end;
    }

    let mut generic_args = Vec::new();
    position = skip_whitespace(source, position, span.end);
    if position < span.end && source.as_bytes()[position] == b'<' {
        if let Some(generic_end) = matching_generic_end(source, position, span.end) {
            generic_args = parse_generic_args(source, position + 1, generic_end);
            position = generic_end + 1;
        }
    }

    let array_suffixes = array_suffixes_after_offset(source, position, span.end);

    TypeShape {
        source,
        span,
        qualifiers,
        base_name,
        generic_args,
        array_suffixes,
    }
}

fn trim_span(source: &str, span: TextSpan) -> TextSpan {
    let mut start = span.start;
    let mut end = span.end;

    while start < end {
        let Some(value) = source[start..end].chars().next() else {
            break;
        };
        if !value.is_whitespace() {
            break;
        }
        start += value.len_utf8();
    }

    while start < end {
        let Some(value) = source[start..end].chars().next_back() else {
            break;
        };
        if !value.is_whitespace() {
            break;
        }
        end -= value.len_utf8();
    }

    TextSpan::new(start, end)
}

fn skip_whitespace(source: &str, mut position: usize, end: usize) -> usize {
    while position < end {
        let Some(value) = source[position..end].chars().next() else {
            break;
        };
        if !value.is_whitespace() {
            break;
        }
        position += value.len_utf8();
    }
    position
}

fn identifier_span_at(source: &str, position: usize, end: usize) -> Option<TextSpan> {
    let mut chars = source[position..end].char_indices();
    let (_, first) = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }

    let mut identifier_end = position + first.len_utf8();
    for (index, value) in chars {
        if value.is_ascii_alphanumeric() || value == '_' {
            identifier_end = position + index + value.len_utf8();
        } else {
            break;
        }
    }

    Some(TextSpan::new(position, identifier_end))
}

fn is_type_qualifier(text: &str) -> bool {
    matches!(text, "ref" | "notnull" | "autoptr" | "owned")
}

fn matching_generic_end(source: &str, start: usize, end: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, value) in source[start..end].char_indices() {
        let offset = start + index;
        match value {
            '<' => depth += 1,
            '>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_generic_args(source: &str, start: usize, end: usize) -> Vec<TypeShape<'_>> {
    let mut args = Vec::new();
    let mut arg_start = start;
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    for (index, value) in source[start..end].char_indices() {
        let offset = start + index;
        match value {
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                let span = trim_span(source, TextSpan::new(arg_start, offset));
                if !span.is_empty() {
                    args.push(parse_type_shape(source, span));
                }
                arg_start = offset + value.len_utf8();
            }
            _ => {}
        }
    }

    let span = trim_span(source, TextSpan::new(arg_start, end));
    if !span.is_empty() {
        args.push(parse_type_shape(source, span));
    }

    args
}

fn array_suffixes_after_name(
    source: &str,
    record_span: TextSpan,
    name_span: TextSpan,
) -> Vec<TextSpan> {
    array_suffixes_after_offset(source, name_span.end, record_span.end)
}

fn array_suffixes_after_offset(source: &str, mut position: usize, end: usize) -> Vec<TextSpan> {
    let mut suffixes = Vec::new();

    loop {
        position = skip_whitespace(source, position, end);
        if position >= end || source.as_bytes()[position] != b'[' {
            break;
        }

        let suffix_start = position;
        position += 1;
        while position < end {
            let Some(value) = source[position..end].chars().next() else {
                break;
            };
            position += value.len_utf8();
            if value == ']' {
                suffixes.push(TextSpan::new(suffix_start, position));
                break;
            }
        }
    }

    suffixes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;
    use std::collections::BTreeMap;

    #[test]
    fn catalogs_top_level_declarations() {
        let source = r#"//! Global docs
Game g_Game;

[EnumBitFlag()]
enum EFlags
{
	None,
	One = 1,
}

typedef int ExampleAlias;

void GlobalFn(int value = 4);

class Example : Base
{
}
"#;
        let catalog = catalog(source);

        assert_eq!(count_kind(&catalog, SymbolKind::GlobalField), 1);
        assert_eq!(count_kind(&catalog, SymbolKind::Enum), 1);
        assert_eq!(count_kind(&catalog, SymbolKind::EnumMember), 2);
        assert_eq!(count_kind(&catalog, SymbolKind::Typedef), 1);
        assert_eq!(count_kind(&catalog, SymbolKind::Function), 1);
        assert_eq!(count_kind(&catalog, SymbolKind::Class), 1);

        let global = find(&catalog, SymbolKind::GlobalField, "g_Game");
        assert_eq!(catalog.text(global.detail.type_text.unwrap()), "Game");
        assert_eq!(global.doc_comments.len(), 1);

        let enum_symbol = find(&catalog, SymbolKind::Enum, "EFlags");
        assert_eq!(enum_symbol.attributes.len(), 1);
        assert_eq!(
            catalog.attribute_name(enum_symbol.attributes[0]),
            Some("EnumBitFlag")
        );
        assert_eq!(
            catalog.record_attribute_names(enum_symbol),
            vec!["EnumBitFlag"]
        );

        let enum_member = find(&catalog, SymbolKind::EnumMember, "One");
        assert_eq!(enum_member.parent, Some(enum_symbol.id));
        assert_eq!(
            catalog.text(enum_member.detail.enum_value_text.unwrap()),
            "1"
        );

        let typedef = find(&catalog, SymbolKind::Typedef, "ExampleAlias");
        assert_eq!(catalog.text(typedef.detail.type_text.unwrap()), "int");

        let class = find(&catalog, SymbolKind::Class, "Example");
        assert_eq!(catalog.text(class.detail.base_type.unwrap()), "Base");

        let function = find(&catalog, SymbolKind::Function, "GlobalFn");
        assert_eq!(
            catalog.text(function.detail.return_type_text.unwrap()),
            "void"
        );
        assert_eq!(child_count(&catalog, function.id, SymbolKind::Parameter), 1);
    }

    #[test]
    fn catalogs_class_members_and_parameters_with_parent_links() {
        let source = r#"class Example
{
	protected ref array<int> m_aValues;
	void Example(int value) {}
	void ~Example() {}
	static bool Run(out notnull array<ref SCR_Value> values);
}
"#;
        let catalog = catalog(source);
        let class = find(&catalog, SymbolKind::Class, "Example");
        let field = find(&catalog, SymbolKind::Field, "m_aValues");
        let constructor = find(&catalog, SymbolKind::Constructor, "Example");
        let destructor = find(&catalog, SymbolKind::Destructor, "Example");
        let method = find(&catalog, SymbolKind::Method, "Run");
        let parameter = find(&catalog, SymbolKind::Parameter, "values");

        assert_eq!(field.parent, Some(class.id));
        assert_eq!(constructor.parent, Some(class.id));
        assert_eq!(destructor.parent, Some(class.id));
        assert_eq!(method.parent, Some(class.id));
        assert_eq!(parameter.parent, Some(method.id));

        assert_eq!(
            catalog.text(field.detail.type_text.unwrap()),
            "ref array<int>"
        );
        assert_eq!(
            catalog.text(method.detail.return_type_text.unwrap()),
            "bool"
        );
        assert_eq!(
            catalog.text(parameter.detail.type_text.unwrap()),
            "array<ref SCR_Value>"
        );
        assert_eq!(
            parameter
                .modifiers
                .iter()
                .map(|span| catalog.text(*span))
                .collect::<Vec<_>>(),
            vec!["out", "notnull"]
        );
    }

    #[test]
    fn catalogs_local_variables_under_containing_callable() {
        let source = r#"class Example
{
	void Run()
	{
		array<int> values = {};
		const int count = values.Count();
		for (int i = 0, max = values.Count(); i < max; i++)
		{
			foreach (int index, auto value : values)
			{
				string label = value.ToString();
			}
		}
	}
}
"#;
        let catalog = catalog(source);
        let method = find(&catalog, SymbolKind::Method, "Run");

        assert_eq!(child_count(&catalog, method.id, SymbolKind::Parameter), 0);
        assert_eq!(
            child_count(&catalog, method.id, SymbolKind::LocalVariable),
            7
        );

        let values = find(&catalog, SymbolKind::LocalVariable, "values");
        assert_eq!(values.parent, Some(method.id));
        assert_eq!(catalog.text(values.detail.type_text.unwrap()), "array<int>");
        assert_eq!(catalog.text(values.detail.default_text.unwrap()), "{}");

        let count = find(&catalog, SymbolKind::LocalVariable, "count");
        assert_eq!(count.parent, Some(method.id));
        assert_eq!(catalog.text(count.detail.type_text.unwrap()), "int");
        assert_eq!(catalog.text(count.modifiers[0]), "const");

        let value = find(&catalog, SymbolKind::LocalVariable, "value");
        assert_eq!(value.parent, Some(method.id));
        assert_eq!(catalog.text(value.detail.type_text.unwrap()), "auto");
    }

    #[test]
    fn excludes_non_declaration_callable_fragments() {
        let source = r#"class Example
{
	void FilterOutStorages(false);
	void Real(bool enabled = true);
}
"#;
        let catalog = catalog(source);

        assert_eq!(catalog.non_declaration_callable_fragments(), 1);
        assert_eq!(count_kind(&catalog, SymbolKind::Parameter), 1);
        let parameter = find(&catalog, SymbolKind::Parameter, "enabled");
        assert_eq!(catalog.text(parameter.detail.type_text.unwrap()), "bool");
        assert_eq!(catalog.text(parameter.detail.default_text.unwrap()), "true");
    }

    #[test]
    fn catalogs_typedef_aliased_type_text() {
        let source = r#"typedef string FactionKey;
typedef func Callback;
typedef map<ref Managed, ref Managed> TManagedMap;
"#;
        let catalog = catalog(source);

        let faction_key = find(&catalog, SymbolKind::Typedef, "FactionKey");
        assert_eq!(
            catalog.text(faction_key.detail.type_text.unwrap()),
            "string"
        );

        let callback = find(&catalog, SymbolKind::Typedef, "Callback");
        assert_eq!(catalog.text(callback.detail.type_text.unwrap()), "func");

        let map = find(&catalog, SymbolKind::Typedef, "TManagedMap");
        assert_eq!(
            catalog.text(map.detail.type_text.unwrap()),
            "map<ref Managed, ref Managed>"
        );
    }

    #[test]
    fn exposes_structured_type_shapes() {
        let source = r#"typedef ScriptInvokerBase<Callback> ScriptInvoker;
typedef map<ref Managed, ref array<string>> TManagedNames;

class Example
{
	ref array<vector> m_aVectors;
	void Run(array<ref SCR_Thing> things, float val[4]);
}

// type-shape sample: autoptr SCR_Type
"#;
        let catalog = catalog(source);

        let invoker = find(&catalog, SymbolKind::Typedef, "ScriptInvoker");
        let invoker_shape = catalog.record_type_shape(invoker).unwrap();
        assert_eq!(invoker_shape.base_name_text(), Some("ScriptInvokerBase"));
        assert!(invoker_shape.qualifier_texts().is_empty());
        assert_eq!(invoker_shape.generic_args().len(), 1);
        assert_eq!(
            invoker_shape.generic_args()[0].base_name_text(),
            Some("Callback")
        );

        let names = find(&catalog, SymbolKind::Typedef, "TManagedNames");
        let names_shape = catalog.record_type_shape(names).unwrap();
        assert_eq!(names_shape.base_name_text(), Some("map"));
        assert_eq!(names_shape.generic_args().len(), 2);
        assert_eq!(names_shape.generic_args()[0].qualifier_texts(), vec!["ref"]);
        assert_eq!(
            names_shape.generic_args()[0].base_name_text(),
            Some("Managed")
        );
        assert_eq!(names_shape.generic_args()[1].qualifier_texts(), vec!["ref"]);
        assert_eq!(
            names_shape.generic_args()[1].base_name_text(),
            Some("array")
        );
        assert_eq!(
            names_shape.generic_args()[1].generic_args()[0].base_name_text(),
            Some("string")
        );

        let vectors = find(&catalog, SymbolKind::Field, "m_aVectors");
        let vectors_shape = catalog.record_type_shape(vectors).unwrap();
        assert_eq!(vectors_shape.text(), "ref array<vector>");
        assert_eq!(vectors_shape.qualifier_texts(), vec!["ref"]);
        assert_eq!(vectors_shape.base_name_text(), Some("array"));
        assert_eq!(
            vectors_shape.generic_args()[0].base_name_text(),
            Some("vector")
        );
        assert!(vectors_shape.array_suffix_texts().is_empty());

        let autoptr_start = source.find("autoptr SCR_Type").unwrap();
        let autoptr_shape = catalog.type_shape(TextSpan::new(
            autoptr_start,
            autoptr_start + "autoptr SCR_Type".len(),
        ));
        assert_eq!(autoptr_shape.qualifier_texts(), vec!["autoptr"]);
        assert_eq!(autoptr_shape.base_name_text(), Some("SCR_Type"));

        let things = find(&catalog, SymbolKind::Parameter, "things");
        let things_shape = catalog.record_type_shape(things).unwrap();
        assert_eq!(things_shape.base_name_text(), Some("array"));
        assert_eq!(things_shape.generic_args().len(), 1);
        assert_eq!(
            things_shape.generic_args()[0].qualifier_texts(),
            vec!["ref"]
        );
        assert_eq!(
            things_shape.generic_args()[0].base_name_text(),
            Some("SCR_Thing")
        );

        let val = find(&catalog, SymbolKind::Parameter, "val");
        let val_shape = catalog.record_type_shape(val).unwrap();
        assert_eq!(val_shape.base_name_text(), Some("float"));
        assert_eq!(val_shape.array_suffix_texts(), vec!["[4]"]);
    }

    #[test]
    fn catalogs_static_array_fields_with_correct_name_type_and_suffix() {
        let source = r#"class Example
{
	static const int COUNT = 4;
	static const string TAGS[COUNT] = {};
	LocalizedString NAMES[COUNT];
}
"#;
        let catalog = catalog(source);

        let count = find(&catalog, SymbolKind::Field, "COUNT");
        assert_eq!(catalog.text(count.detail.type_text.unwrap()), "int");
        assert!(catalog
            .record_type_shape(count)
            .unwrap()
            .array_suffix_texts()
            .is_empty());

        let tags = find(&catalog, SymbolKind::Field, "TAGS");
        assert_eq!(catalog.text(tags.detail.type_text.unwrap()), "string");
        assert_eq!(
            catalog
                .record_type_shape(tags)
                .unwrap()
                .array_suffix_texts(),
            vec!["[COUNT]"]
        );

        let names = find(&catalog, SymbolKind::Field, "NAMES");
        assert_eq!(
            catalog.text(names.detail.type_text.unwrap()),
            "LocalizedString"
        );
        assert_eq!(
            catalog
                .record_type_shape(names)
                .unwrap()
                .array_suffix_texts(),
            vec!["[COUNT]"]
        );
    }

    #[test]
    fn catalogs_comma_separated_field_declarators_individually() {
        let source = r#"Widget g_First, g_Second;

class Example
{
	protected Widget m_ContentWidget, m_ButtonPrevWidget, m_ButtonNextWidget;
	protected ref array<int> m_aValues, m_aOtherValues;
	protected int count, values[COUNT], other = 4;
	protected map<Widget, SCR_Item> m_mItems, m_mOtherItems;
}
"#;
        let catalog = catalog(source);

        assert_eq!(count_kind(&catalog, SymbolKind::GlobalField), 2);
        assert_eq!(count_kind(&catalog, SymbolKind::Field), 10);

        for name in ["g_First", "g_Second"] {
            let field = find(&catalog, SymbolKind::GlobalField, name);
            assert_eq!(catalog.text(field.detail.type_text.unwrap()), "Widget");
        }

        for name in [
            "m_ContentWidget",
            "m_ButtonPrevWidget",
            "m_ButtonNextWidget",
        ] {
            let field = find(&catalog, SymbolKind::Field, name);
            assert_eq!(catalog.text(field.detail.type_text.unwrap()), "Widget");
            assert_eq!(field.modifiers.len(), 1);
            assert_eq!(catalog.text(field.modifiers[0]), "protected");
        }

        for name in ["m_aValues", "m_aOtherValues"] {
            let field = find(&catalog, SymbolKind::Field, name);
            assert_eq!(
                catalog.text(field.detail.type_text.unwrap()),
                "ref array<int>"
            );
        }

        let count = find(&catalog, SymbolKind::Field, "count");
        let values = find(&catalog, SymbolKind::Field, "values");
        let other = find(&catalog, SymbolKind::Field, "other");
        assert_eq!(catalog.text(count.detail.type_text.unwrap()), "int");
        assert_eq!(catalog.text(values.detail.type_text.unwrap()), "int");
        assert_eq!(catalog.text(other.detail.type_text.unwrap()), "int");
        assert!(catalog
            .record_type_shape(count)
            .unwrap()
            .array_suffix_texts()
            .is_empty());
        assert_eq!(
            catalog
                .record_type_shape(values)
                .unwrap()
                .array_suffix_texts(),
            vec!["[COUNT]"]
        );
        assert!(catalog
            .record_type_shape(other)
            .unwrap()
            .array_suffix_texts()
            .is_empty());

        for name in ["m_mItems", "m_mOtherItems"] {
            let field = find(&catalog, SymbolKind::Field, name);
            assert_eq!(
                catalog.text(field.detail.type_text.unwrap()),
                "map<Widget, SCR_Item>"
            );
        }
    }

    #[test]
    fn from_ast_uses_unknown_metadata() {
        let catalog = catalog("class Example {}");

        assert_eq!(catalog.metadata().kind, SourceKind::Unknown);
        assert_eq!(catalog.metadata().category, SourceCategory::Unknown);
        assert_eq!(catalog.metadata().absolute_path, None);
        assert_eq!(catalog.metadata().root_path, None);
        assert_eq!(catalog.metadata().relative_path, None);
        assert_eq!(catalog.metadata().priority, SOURCE_PRIORITY_UNKNOWN);
    }

    #[test]
    fn metadata_constructor_preserves_file_identity_without_changing_records() {
        let source = "class Example {}";
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ast = AstSourceFile::new(source, &parse);
        let metadata = SourceFileMetadata {
            kind: SourceKind::GameData,
            category: SourceCategory::Game,
            absolute_path: Some(PathBuf::from("C:/scripts/Game/Example.c")),
            root_path: Some(PathBuf::from("C:/scripts")),
            relative_path: Some(PathBuf::from("Game/Example.c")),
            priority: SOURCE_PRIORITY_GAME_DATA,
        };

        let catalog = SymbolCatalog::from_ast_with_metadata(source, &ast, metadata.clone());

        assert_eq!(catalog.metadata(), &metadata);
        assert_eq!(catalog.records().len(), 1);
        let record = &catalog.records()[0];
        assert_eq!(record.kind, SymbolKind::Class);
        assert_eq!(catalog.record_name(record), Some("Example"));
        assert_eq!(record.parent, None);
    }

    #[test]
    fn records_conditional_context_and_callable_form() {
        let source = r#"#ifdef DISABLE_INVENTORY
class Example
{
	void Declared();
	proto void Prototype();
	void Implemented() {}
}
#else
class Other {}
#endif
"#;
        let catalog = catalog(source);

        let class = find(&catalog, SymbolKind::Class, "Example");
        assert_eq!(class.conditional_context.len(), 1);
        assert_eq!(
            class.conditional_context[0].kind,
            PreprocessorBranchKind::Ifdef
        );
        assert_eq!(
            catalog.text(class.conditional_context[0].condition.unwrap()),
            "DISABLE_INVENTORY"
        );

        let declared = find(&catalog, SymbolKind::Method, "Declared");
        assert_eq!(declared.callable_form, Some(CallableForm::Declaration));
        let prototype = find(&catalog, SymbolKind::Method, "Prototype");
        assert_eq!(prototype.callable_form, Some(CallableForm::Prototype));
        let implemented = find(&catalog, SymbolKind::Method, "Implemented");
        assert_eq!(
            implemented.callable_form,
            Some(CallableForm::Implementation)
        );

        let other = find(&catalog, SymbolKind::Class, "Other");
        assert_eq!(
            other.conditional_context[0].kind,
            PreprocessorBranchKind::Else
        );
        assert_eq!(
            catalog.text(other.conditional_context[0].condition.unwrap()),
            "DISABLE_INVENTORY"
        );
    }

    #[test]
    fn committed_parser_fixtures_catalog_without_missing_names() {
        let fixtures = [
            include_str!("../../tools/fixtures/parser/core_types_declarations.c"),
            include_str!("../../tools/fixtures/parser/attributes_rpc_workbench.c"),
            include_str!("../../tools/fixtures/parser/modded_game_mode_members.c"),
            include_str!("../../tools/fixtures/parser/preprocessor_directives.c"),
            include_str!("../../tools/fixtures/parser/game_building_network_component.c"),
            include_str!("../../tools/fixtures/parser/game_building_provider_excerpt.c"),
            include_str!("../../tools/fixtures/parser/game_editable_group_excerpt.c"),
            include_str!("../../tools/fixtures/parser/game_editor_preview_params.c"),
            include_str!("../../tools/fixtures/parser/workbench_basic_code_formatter_excerpt.c"),
            include_str!("../../tools/fixtures/parser/game_optional_semicolon_shapes.c"),
            include_str!("../../tools/fixtures/parser/game_field_initializer_call_shapes.c"),
            include_str!("../../tools/fixtures/parser/local_block_symbols.c"),
        ];

        for fixture in fixtures {
            let catalog = catalog(fixture);
            assert!(!catalog.records().is_empty());
            assert!(
                catalog.records().iter().all(|record| record.name.is_some()),
                "fixture had missing symbol names"
            );
        }
    }

    fn catalog(source: &str) -> SymbolCatalog<'_> {
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ast = AstSourceFile::new(source, &parse);
        SymbolCatalog::from_ast(source, &ast)
    }

    fn count_kind(catalog: &SymbolCatalog<'_>, kind: SymbolKind) -> usize {
        catalog
            .records()
            .iter()
            .filter(|record| record.kind == kind)
            .count()
    }

    fn child_count(catalog: &SymbolCatalog<'_>, parent: SymbolId, kind: SymbolKind) -> usize {
        catalog
            .records()
            .iter()
            .filter(|record| record.parent == Some(parent) && record.kind == kind)
            .count()
    }

    fn find<'a>(catalog: &'a SymbolCatalog<'_>, kind: SymbolKind, name: &str) -> &'a SymbolRecord {
        catalog
            .records()
            .iter()
            .find(|record| {
                record.kind == kind && record.name.is_some_and(|span| catalog.text(span) == name)
            })
            .unwrap_or_else(|| panic!("missing {kind:?} {name}; got {:?}", names(catalog)))
    }

    fn names(catalog: &SymbolCatalog<'_>) -> BTreeMap<String, SymbolKind> {
        catalog
            .records()
            .iter()
            .filter_map(|record| {
                record
                    .name
                    .map(|span| (catalog.text(span).to_string(), record.kind))
            })
            .collect()
    }
}
