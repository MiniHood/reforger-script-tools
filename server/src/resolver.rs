use crate::index::{GlobalSymbolId, IndexedFile, IndexedSymbol, SymbolIndex};
use crate::lexer::{lex, TextSpan, TokenKind};
use crate::model::{SourceCategory, SourceKind, SymbolKind};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct ReferenceResolver<'source, 'index> {
    source: &'source str,
    file_index: &'index SymbolIndex,
    external_index: Option<&'index SymbolIndex>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceResolution {
    pub token_text: String,
    pub token_span: TextSpan,
    pub identifier_context: IdentifierContext,
    pub candidates: Vec<ReferenceCandidate>,
    pub selected: Option<ReferenceCandidate>,
    pub reason: ResolutionReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceCandidate {
    pub source: CandidateSource,
    pub id: GlobalSymbolId,
    pub reason: ResolutionReason,
    pub kind: SymbolKind,
    pub name: Option<String>,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub source_kind: SourceKind,
    pub source_category: SourceCategory,
    pub source_priority: u16,
    pub relative_path: Option<PathBuf>,
    pub absolute_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSource {
    FileLocal,
    External,
}

impl CandidateSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FileLocal => "file-local",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionReason {
    DeclarationHit,
    LocalInCallable,
    ParameterInCallable,
    ClassMember,
    TopLevel,
    ExternalPreferred,
    Unresolved,
}

impl ResolutionReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeclarationHit => "declaration-hit",
            Self::LocalInCallable => "local-in-callable",
            Self::ParameterInCallable => "parameter-in-callable",
            Self::ClassMember => "class-member",
            Self::TopLevel => "top-level",
            Self::ExternalPreferred => "external-preferred",
            Self::Unresolved => "unresolved",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierContext {
    DeclarationName,
    TypePosition,
    ValueOrCallable,
}

impl IdentifierContext {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeclarationName => "declaration-name",
            Self::TypePosition => "type-position",
            Self::ValueOrCallable => "value-or-callable",
        }
    }
}

impl<'source, 'index> ReferenceResolver<'source, 'index> {
    pub const fn new(
        source: &'source str,
        file_index: &'index SymbolIndex,
        external_index: Option<&'index SymbolIndex>,
    ) -> Self {
        Self {
            source,
            file_index,
            external_index,
        }
    }

    pub fn resolve_at_offset(&self, offset: usize) -> Option<ReferenceResolution> {
        let token = token_at_offset(self.source, offset)?;
        if token.kind != TokenKind::Identifier {
            return None;
        }

        let token_text = self.source[token.span.start..token.span.end].to_string();
        let identifier_context = self.identifier_context(token.span);
        let mut candidates = Vec::new();
        let mut seen = BTreeSet::new();

        self.push_declaration_hits(&token_text, token.span, &mut candidates, &mut seen);
        if identifier_context == IdentifierContext::TypePosition {
            self.push_type_like_top_level(&token_text, &mut candidates, &mut seen);
            self.push_external_type_like(&token_text, &mut candidates, &mut seen);
            self.push_callable_locals_and_parameters(
                &token_text,
                offset,
                &mut candidates,
                &mut seen,
            );
            self.push_class_members(&token_text, offset, &mut candidates, &mut seen);
            self.push_top_level(&token_text, &mut candidates, &mut seen);
        } else {
            self.push_callable_locals_and_parameters(
                &token_text,
                offset,
                &mut candidates,
                &mut seen,
            );
            self.push_class_members(&token_text, offset, &mut candidates, &mut seen);
            self.push_top_level(&token_text, &mut candidates, &mut seen);
            self.push_external(&token_text, &mut candidates, &mut seen);
        }

        let selected = candidates.first().cloned();
        let reason = selected
            .as_ref()
            .map(|candidate| candidate.reason)
            .unwrap_or(ResolutionReason::Unresolved);

        Some(ReferenceResolution {
            token_text,
            token_span: token.span,
            identifier_context,
            candidates,
            selected,
            reason,
        })
    }

    fn identifier_context(&self, token_span: TextSpan) -> IdentifierContext {
        if self
            .file_index
            .symbols()
            .iter()
            .any(|symbol| symbol.selection_span == token_span)
        {
            return IdentifierContext::DeclarationName;
        }

        if self.file_index.symbols().iter().any(|symbol| {
            [
                symbol.detail.type_text_span,
                symbol.detail.return_type_text_span,
                symbol.detail.base_type_span,
            ]
            .into_iter()
            .flatten()
            .any(|span| span_contains_span(span, token_span))
        }) {
            return IdentifierContext::TypePosition;
        }

        IdentifierContext::ValueOrCallable
    }

    fn push_declaration_hits(
        &self,
        token_text: &str,
        token_span: TextSpan,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        for id in self.file_index.symbols_for_name(token_text) {
            let Some(symbol) = self.file_index.symbol(*id) else {
                continue;
            };
            if symbol.selection_span == token_span {
                self.push_candidate(
                    candidates,
                    seen,
                    CandidateSource::FileLocal,
                    *id,
                    ResolutionReason::DeclarationHit,
                );
            }
        }
    }

    fn push_callable_locals_and_parameters(
        &self,
        token_text: &str,
        offset: usize,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        let Some(callable) = self.containing_callable(offset) else {
            return;
        };

        for kind in [SymbolKind::LocalVariable, SymbolKind::Parameter] {
            for child in self.file_index.children(callable) {
                let Some(symbol) = self.file_index.symbol(*child) else {
                    continue;
                };
                if symbol.kind != kind || symbol.name.as_deref() != Some(token_text) {
                    continue;
                }
                if kind == SymbolKind::LocalVariable && symbol.selection_span.start > offset {
                    continue;
                }
                let reason = match kind {
                    SymbolKind::LocalVariable => ResolutionReason::LocalInCallable,
                    SymbolKind::Parameter => ResolutionReason::ParameterInCallable,
                    _ => unreachable!(),
                };
                self.push_candidate(candidates, seen, CandidateSource::FileLocal, *child, reason);
            }
        }
    }

    fn push_class_members(
        &self,
        token_text: &str,
        offset: usize,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        let Some(class) = self.containing_class(offset) else {
            return;
        };
        let Some(class_name) = self
            .file_index
            .symbol(class)
            .and_then(|symbol| symbol.name.as_deref())
        else {
            return;
        };

        for member in self.file_index.members_by_owner(class_name) {
            let Some(symbol) = self.file_index.symbol(*member) else {
                continue;
            };
            if symbol.name.as_deref() == Some(token_text) {
                self.push_candidate(
                    candidates,
                    seen,
                    CandidateSource::FileLocal,
                    *member,
                    ResolutionReason::ClassMember,
                );
            }
        }
    }

    fn push_top_level(
        &self,
        token_text: &str,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        for id in self.file_index.top_level_symbols_for_name(token_text) {
            self.push_candidate(
                candidates,
                seen,
                CandidateSource::FileLocal,
                *id,
                ResolutionReason::TopLevel,
            );
        }
    }

    fn push_type_like_top_level(
        &self,
        token_text: &str,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        for id in self.file_index.top_level_symbols_for_name(token_text) {
            let Some(symbol) = self.file_index.symbol(*id) else {
                continue;
            };
            if is_type_like_kind(symbol.kind) {
                self.push_candidate(
                    candidates,
                    seen,
                    CandidateSource::FileLocal,
                    *id,
                    ResolutionReason::TopLevel,
                );
            }
        }
    }

    fn push_external(
        &self,
        token_text: &str,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        let Some(external_index) = self.external_index else {
            return;
        };

        let mut external = Vec::new();
        for id in external_index.preferred_classes_by_name(token_text) {
            push_unique_id(&mut external, id);
        }
        for id in external_index.preferred_typedefs_by_name(token_text) {
            push_unique_id(&mut external, id);
        }
        for id in external_index.preferred_functions_by_name(token_text) {
            push_unique_id(&mut external, id);
        }
        for id in external_index.preferred_top_level_symbols_for_name(token_text) {
            push_unique_id(&mut external, id);
        }

        for id in external {
            push_index_candidate(
                external_index,
                candidates,
                seen,
                CandidateSource::External,
                id,
                ResolutionReason::ExternalPreferred,
            );
        }
    }

    fn push_external_type_like(
        &self,
        token_text: &str,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        let Some(external_index) = self.external_index else {
            return;
        };

        let external = external_index
            .top_level_symbols_for_name(token_text)
            .iter()
            .copied()
            .filter(|id| {
                external_index
                    .symbol(*id)
                    .is_some_and(|symbol| is_type_like_kind(symbol.kind))
            })
            .collect::<Vec<_>>();

        for id in external_index.preferred_from_symbols(&external) {
            push_index_candidate(
                external_index,
                candidates,
                seen,
                CandidateSource::External,
                id,
                ResolutionReason::ExternalPreferred,
            );
        }
    }

    fn push_candidate(
        &self,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
        source: CandidateSource,
        id: GlobalSymbolId,
        reason: ResolutionReason,
    ) {
        push_index_candidate(self.file_index, candidates, seen, source, id, reason);
    }

    fn containing_callable(&self, offset: usize) -> Option<GlobalSymbolId> {
        self.file_index
            .symbols()
            .iter()
            .filter(|symbol| is_callable_kind(symbol.kind) && span_contains(symbol.span, offset))
            .min_by_key(|symbol| symbol.span.len())
            .map(|symbol| symbol.id)
    }

    fn containing_class(&self, offset: usize) -> Option<GlobalSymbolId> {
        self.file_index
            .symbols()
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Class && span_contains(symbol.span, offset))
            .min_by_key(|symbol| symbol.span.len())
            .map(|symbol| symbol.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CandidateKey {
    source: CandidateSource,
    id: GlobalSymbolId,
}

impl PartialOrd for CandidateSource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CandidateSource {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

fn push_index_candidate(
    index: &SymbolIndex,
    candidates: &mut Vec<ReferenceCandidate>,
    seen: &mut BTreeSet<CandidateKey>,
    source: CandidateSource,
    id: GlobalSymbolId,
    reason: ResolutionReason,
) {
    let key = CandidateKey { source, id };
    if !seen.insert(key) {
        return;
    }
    let Some(symbol) = index.symbol(id) else {
        return;
    };
    let Some(file) = index.file(id.file_id) else {
        return;
    };
    candidates.push(candidate_from_symbol(source, id, reason, symbol, file));
}

fn candidate_from_symbol(
    source: CandidateSource,
    id: GlobalSymbolId,
    reason: ResolutionReason,
    symbol: &IndexedSymbol,
    file: &IndexedFile,
) -> ReferenceCandidate {
    ReferenceCandidate {
        source,
        id,
        reason,
        kind: symbol.kind,
        name: symbol.name.clone(),
        span: symbol.span,
        selection_span: symbol.selection_span,
        source_kind: file.metadata.kind,
        source_category: file.metadata.category,
        source_priority: file.metadata.priority,
        relative_path: file.metadata.relative_path.clone(),
        absolute_path: file.metadata.absolute_path.clone(),
    }
}

fn token_at_offset(source: &str, offset: usize) -> Option<crate::lexer::Token> {
    lex(source)
        .into_iter()
        .find(|token| token.span.start <= offset && offset < token.span.end)
}

fn span_contains(span: TextSpan, offset: usize) -> bool {
    span.start <= offset && offset < span.end
}

fn span_contains_span(outer: TextSpan, inner: TextSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn is_callable_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Destructor
    )
}

fn is_type_like_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Class | SymbolKind::Enum | SymbolKind::Typedef
    )
}

fn push_unique_id(ids: &mut Vec<GlobalSymbolId>, id: GlobalSymbolId) {
    if !ids.contains(&id) {
        ids.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::model::{
        source_category_for_path, SourceFileMetadata, SymbolCatalog, SOURCE_PRIORITY_GAME_DATA,
        SOURCE_PRIORITY_WORKSPACE,
    };
    use crate::parser::parse_source;

    #[test]
    fn declaration_identifier_resolves_to_itself() {
        let source = "class Example {}";
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve(&index, source, "Example");

        assert_eq!(resolution.token_text, "Example");
        assert_eq!(resolution.reason, ResolutionReason::DeclarationHit);
        assert_eq!(resolution.selected.unwrap().kind, SymbolKind::Class);
    }

    #[test]
    fn local_variable_use_resolves_before_parameter_member_and_top_level() {
        let source = r#"int value;
class Example
{
	int value;
	void Run(int value)
	{
		int value = 4;
		value = value + 1;
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "value = value + 1", "value");

        assert_eq!(resolution.reason, ResolutionReason::LocalInCallable);
        let candidates = candidate_kinds_reasons(&resolution);
        assert_eq!(
            candidates[..4],
            [
                (SymbolKind::LocalVariable, ResolutionReason::LocalInCallable),
                (SymbolKind::Parameter, ResolutionReason::ParameterInCallable),
                (SymbolKind::Field, ResolutionReason::ClassMember),
                (SymbolKind::GlobalField, ResolutionReason::TopLevel),
            ]
        );
    }

    #[test]
    fn parameter_use_resolves_before_member_and_top_level_when_no_local_exists() {
        let source = r#"int value;
class Example
{
	int value;
	void Run(int value)
	{
		Print(value);
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "Print(value)", "value");

        assert_eq!(resolution.reason, ResolutionReason::ParameterInCallable);
        assert_eq!(resolution.selected.unwrap().kind, SymbolKind::Parameter);
    }

    #[test]
    fn class_member_use_resolves_from_containing_class() {
        let source = r#"class Example
{
	int m_Value;
	void Run()
	{
		m_Value = 4;
		Run();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let field = resolve_at_needle(&index, source, "m_Value = 4", "m_Value");
        let method = resolve_at_needle(&index, source, "Run();", "Run");

        assert_eq!(field.reason, ResolutionReason::ClassMember);
        assert_eq!(field.selected.unwrap().kind, SymbolKind::Field);
        assert_eq!(method.reason, ResolutionReason::ClassMember);
        assert_eq!(method.selected.unwrap().kind, SymbolKind::Method);
    }

    #[test]
    fn file_local_top_level_declarations_resolve_by_name() {
        let source = r#"Game g_Game;
typedef string FactionKey;
enum EExample { One = 1 }
void GlobalFn();
class Example
{
	void Run()
	{
		Example local;
		FactionKey key;
		EExample flag;
		GlobalFn();
		g_Game = null;
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        assert_eq!(
            resolve_at_needle(&index, source, "Example local", "Example").reason,
            ResolutionReason::TopLevel
        );
        assert_eq!(
            resolve_at_needle(&index, source, "FactionKey key", "FactionKey")
                .selected
                .unwrap()
                .kind,
            SymbolKind::Typedef
        );
        assert_eq!(
            resolve_at_needle(&index, source, "EExample flag", "EExample")
                .selected
                .unwrap()
                .kind,
            SymbolKind::Enum
        );
        assert_eq!(
            resolve_at_needle(&index, source, "GlobalFn();", "GlobalFn")
                .selected
                .unwrap()
                .kind,
            SymbolKind::Function
        );
        assert_eq!(
            resolve_at_needle(&index, source, "g_Game = null", "g_Game")
                .selected
                .unwrap()
                .kind,
            SymbolKind::GlobalField
        );
    }

    #[test]
    fn type_position_prefers_type_declarations_over_constructor_members() {
        let source = r#"class Example
{
	void Example();
	static Example Make()
	{
		Example value;
		array<Example> values;
		Example constructed = new Example();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let return_type = resolve_at_needle(&index, source, "static Example Make", "Example");
        let local_type = resolve_at_needle(&index, source, "Example value", "Example");
        let generic_arg = resolve_at_needle(&index, source, "array<Example> values", "Example");
        let constructor_call = resolve_at_needle(&index, source, "new Example()", "Example");

        assert_eq!(
            return_type.identifier_context,
            IdentifierContext::TypePosition
        );
        assert_eq!(return_type.reason, ResolutionReason::TopLevel);
        assert_eq!(return_type.selected.unwrap().kind, SymbolKind::Class);

        assert_eq!(
            local_type.identifier_context,
            IdentifierContext::TypePosition
        );
        assert_eq!(local_type.selected.unwrap().kind, SymbolKind::Class);

        assert_eq!(
            generic_arg.identifier_context,
            IdentifierContext::TypePosition
        );
        assert_eq!(generic_arg.selected.unwrap().kind, SymbolKind::Class);

        assert_eq!(
            constructor_call.identifier_context,
            IdentifierContext::ValueOrCallable
        );
        assert_eq!(constructor_call.reason, ResolutionReason::ClassMember);
        assert_eq!(
            constructor_call.selected.unwrap().kind,
            SymbolKind::Constructor
        );
    }

    #[test]
    fn base_type_position_prefers_file_local_class() {
        let source = r#"class Base {}
class Child : Base {}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "class Child : Base", "Base");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::TypePosition
        );
        assert_eq!(resolution.reason, ResolutionReason::TopLevel);
        assert_eq!(resolution.selected.unwrap().kind, SymbolKind::Class);
    }

    #[test]
    fn external_index_resolves_missing_type_name() {
        let source = r#"class Example
{
	void Run()
	{
		ExternalType value;
	}
}
"#;
        let file_index = index_for_source(source, workspace_metadata("Example.c"));
        let external_source = "class ExternalType {}";
        let external_index =
            index_for_source(external_source, game_metadata("Game/ExternalType.c"));

        let offset = offset_for_needle(source, "ExternalType value", "ExternalType");
        let resolution = ReferenceResolver::new(source, &file_index, Some(&external_index))
            .resolve_at_offset(offset)
            .unwrap();

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::TypePosition
        );
        assert_eq!(resolution.reason, ResolutionReason::ExternalPreferred);
        let selected = resolution.selected.unwrap();
        assert_eq!(selected.source, CandidateSource::External);
        assert_eq!(selected.kind, SymbolKind::Class);
        assert_eq!(selected.source_kind, SourceKind::GameData);
    }

    #[test]
    fn non_identifier_tokens_do_not_resolve() {
        let source = r#"class Example
{
	void Run()
	{
		// comment value
		string text = "value";
		if (true) {}
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        assert_no_resolution(&index, source, "class Example", "class");
        assert_no_resolution(&index, source, "// comment value", "value");
        assert_no_resolution(&index, source, "\"value\"", "value");
        assert_no_resolution(&index, source, "(true)", "(");
    }

    #[test]
    fn ambiguous_same_name_candidates_are_preserved_with_selected_first() {
        let source = r#"class Shared {}
typedef string Shared;
void Shared();
class Example
{
	void Run()
	{
		Shared value;
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Shared.c"));

        let resolution = resolve_at_needle(&index, source, "Shared value", "Shared");

        assert_eq!(resolution.reason, ResolutionReason::TopLevel);
        assert_eq!(resolution.candidates.len(), 3);
        assert_eq!(resolution.selected.unwrap().kind, SymbolKind::Class);
        assert!(resolution
            .candidates
            .iter()
            .any(|candidate| candidate.kind == SymbolKind::Typedef));
        assert!(resolution
            .candidates
            .iter()
            .any(|candidate| candidate.kind == SymbolKind::Function));
    }

    fn resolve(index: &SymbolIndex, source: &str, needle: &str) -> ReferenceResolution {
        let offset = source.find(needle).unwrap();
        ReferenceResolver::new(source, index, None)
            .resolve_at_offset(offset)
            .unwrap()
    }

    fn resolve_at_needle(
        index: &SymbolIndex,
        source: &str,
        needle: &str,
        cursor: &str,
    ) -> ReferenceResolution {
        let offset = offset_for_needle(source, needle, cursor);
        ReferenceResolver::new(source, index, None)
            .resolve_at_offset(offset)
            .unwrap()
    }

    fn assert_no_resolution(index: &SymbolIndex, source: &str, needle: &str, cursor: &str) {
        let offset = offset_for_needle(source, needle, cursor);
        assert_eq!(
            ReferenceResolver::new(source, index, None).resolve_at_offset(offset),
            None
        );
    }

    fn candidate_kinds_reasons(
        resolution: &ReferenceResolution,
    ) -> Vec<(SymbolKind, ResolutionReason)> {
        resolution
            .candidates
            .iter()
            .map(|candidate| (candidate.kind, candidate.reason))
            .collect()
    }

    fn offset_for_needle(source: &str, needle: &str, cursor: &str) -> usize {
        let start = source.find(needle).unwrap();
        let cursor_start = needle.find(cursor).unwrap();
        start + cursor_start
    }

    fn index_for_source(source: &str, metadata: SourceFileMetadata) -> SymbolIndex {
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ast = AstSourceFile::new(source, &parse);
        let catalog = SymbolCatalog::from_ast_with_metadata(source, &ast, metadata);
        SymbolIndex::from_catalogs([&catalog])
    }

    fn workspace_metadata(path: &str) -> SourceFileMetadata {
        SourceFileMetadata {
            kind: SourceKind::Workspace,
            category: SourceCategory::Workspace,
            absolute_path: Some(PathBuf::from("C:/workspace").join(path)),
            root_path: Some(PathBuf::from("C:/workspace")),
            relative_path: Some(PathBuf::from(path)),
            priority: SOURCE_PRIORITY_WORKSPACE,
        }
    }

    fn game_metadata(path: &str) -> SourceFileMetadata {
        let relative_path = PathBuf::from(path);
        SourceFileMetadata {
            kind: SourceKind::GameData,
            category: source_category_for_path(SourceKind::GameData, Some(&relative_path)),
            absolute_path: Some(PathBuf::from("C:/game").join(path)),
            root_path: Some(PathBuf::from("C:/game")),
            relative_path: Some(relative_path),
            priority: SOURCE_PRIORITY_GAME_DATA,
        }
    }
}
