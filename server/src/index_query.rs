use crate::index::{CompletionMemberLookup, GlobalSymbolId, MemberShadowGroup, SymbolIndex};
use crate::lexer::TextSpan;
use crate::model::{SourceKind, SymbolKind};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct IndexQuery<'index> {
    index: &'index SymbolIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCompletionMembers {
    pub raw_candidates: Vec<GlobalSymbolId>,
    pub candidates: Vec<EditorCompletionCandidate>,
    pub shadowed_groups: Vec<EditorMemberShadowGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCompletionCandidate {
    pub id: GlobalSymbolId,
    pub name: Option<String>,
    pub kind: SymbolKind,
    pub detail: Option<String>,
    pub signature: Option<String>,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub source_kind: SourceKind,
    pub source_priority: u16,
    pub relative_path: Option<PathBuf>,
    pub absolute_path: Option<PathBuf>,
    pub origin: EditorCompletionOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorCompletionOrigin {
    Direct,
    Overlay,
    Inherited,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorMemberShadowGroup {
    pub key: String,
    pub kept: GlobalSymbolId,
    pub shadowed: Vec<GlobalSymbolId>,
}

impl<'index> IndexQuery<'index> {
    pub const fn new(index: &'index SymbolIndex) -> Self {
        Self { index }
    }

    pub fn preferred_class(&self, name: &str) -> Option<GlobalSymbolId> {
        self.index.preferred_classes_by_name(name).first().copied()
    }

    pub fn preferred_typedef(&self, name: &str) -> Option<GlobalSymbolId> {
        self.index.preferred_typedefs_by_name(name).first().copied()
    }

    pub fn preferred_function(&self, name: &str) -> Option<GlobalSymbolId> {
        self.index
            .preferred_functions_by_name(name)
            .first()
            .copied()
    }

    pub fn top_level_conflicts(&self, name: &str) -> Vec<GlobalSymbolId> {
        self.index.top_level_symbols_for_name(name).to_vec()
    }

    pub fn callable_signature(&self, id: GlobalSymbolId) -> Option<String> {
        self.index.callable_signature(id)
    }

    pub fn completion_members_for_class(&self, name: &str) -> EditorCompletionMembers {
        let completion = self.index.completion_members_for_preferred_class(name);
        self.editor_completion_members(name, completion)
    }

    pub fn raw_symbols_for_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.index.symbols_for_name(name)
    }

    pub fn raw_top_level_symbols_for_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.index.top_level_symbols_for_name(name)
    }

    pub fn raw_completion_members_for_owner_name(&self, owner: &str) -> CompletionMemberLookup {
        self.index.completion_members_for_class(owner)
    }

    fn editor_completion_members(
        &self,
        owner: &str,
        completion: CompletionMemberLookup,
    ) -> EditorCompletionMembers {
        let preferred_class = self.preferred_class(owner);
        let candidates = completion
            .members
            .iter()
            .filter_map(|id| self.editor_completion_candidate(owner, preferred_class, *id))
            .collect();
        let shadowed_groups = completion
            .shadowed_groups
            .into_iter()
            .map(editor_shadow_group)
            .collect();

        EditorCompletionMembers {
            raw_candidates: completion.raw_candidates,
            candidates,
            shadowed_groups,
        }
    }

    fn editor_completion_candidate(
        &self,
        owner: &str,
        preferred_class: Option<GlobalSymbolId>,
        id: GlobalSymbolId,
    ) -> Option<EditorCompletionCandidate> {
        let symbol = self.index.symbol(id)?;
        let file = self.index.file(id.file_id)?;
        let origin = self.completion_origin(owner, preferred_class, symbol.parent);
        let detail = symbol_detail_text(self.index, id);

        Some(EditorCompletionCandidate {
            id,
            name: symbol.name.clone(),
            kind: symbol.kind,
            detail,
            signature: self.index.callable_signature(id),
            span: symbol.span,
            selection_span: symbol.selection_span,
            source_kind: file.metadata.kind,
            source_priority: file.metadata.priority,
            relative_path: file.metadata.relative_path.clone(),
            absolute_path: file.metadata.absolute_path.clone(),
            origin,
        })
    }

    fn completion_origin(
        &self,
        owner: &str,
        preferred_class: Option<GlobalSymbolId>,
        parent: Option<GlobalSymbolId>,
    ) -> EditorCompletionOrigin {
        let Some(parent) = parent else {
            return EditorCompletionOrigin::Unknown;
        };
        let Some(parent_symbol) = self.index.symbol(parent) else {
            return EditorCompletionOrigin::Unknown;
        };
        let Some(parent_name) = parent_symbol.name.as_deref() else {
            return EditorCompletionOrigin::Unknown;
        };

        if parent_name != owner {
            return EditorCompletionOrigin::Inherited;
        }

        if Some(parent) == preferred_class {
            EditorCompletionOrigin::Direct
        } else {
            EditorCompletionOrigin::Overlay
        }
    }
}

fn editor_shadow_group(group: MemberShadowGroup) -> EditorMemberShadowGroup {
    EditorMemberShadowGroup {
        key: group.key,
        kept: group.kept,
        shadowed: group.shadowed,
    }
}

fn symbol_detail_text(index: &SymbolIndex, id: GlobalSymbolId) -> Option<String> {
    let symbol = index.symbol(id)?;

    if let Some(signature) = index.callable_signature(id) {
        return Some(signature);
    }

    symbol
        .detail
        .type_text
        .as_deref()
        .or(symbol.detail.return_type_text.as_deref())
        .or(symbol.detail.base_type.as_deref())
        .or(symbol.detail.default_text.as_deref())
        .or(symbol.detail.enum_value_text.as_deref())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::index::SymbolIndex;
    use crate::model::{
        SourceFileMetadata, SymbolCatalog, SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE,
    };
    use crate::parser::parse_source;
    use std::path::PathBuf;

    #[test]
    fn preferred_kind_lookup_returns_workspace_symbols_first() {
        let game = catalog(
            r#"class Example {}
typedef int ExampleAlias;
void ExampleFn();
"#,
            game_metadata("Game.c"),
        );
        let workspace = catalog(
            r#"modded class Example {}
typedef float ExampleAlias;
void ExampleFn(int value);
"#,
            workspace_metadata("Workspace.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);
        let query = IndexQuery::new(&index);

        for id in [
            query.preferred_class("Example").unwrap(),
            query.preferred_typedef("ExampleAlias").unwrap(),
            query.preferred_function("ExampleFn").unwrap(),
        ] {
            assert_eq!(
                index.file(id.file_id).unwrap().metadata.kind,
                SourceKind::Workspace
            );
        }
    }

    #[test]
    fn top_level_conflicts_returns_mixed_kinds_without_authoritative_preference() {
        let catalog = catalog(
            r#"typedef string FactionKey;
class FactionKey : string {}
void FactionKey(int value);
"#,
            workspace_metadata("FactionKey.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let conflicts = query.top_level_conflicts("FactionKey");

        assert_eq!(conflicts.len(), 3);
        assert!(conflicts
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Class));
        assert!(conflicts
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Typedef));
        assert!(conflicts
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Function));
    }

    #[test]
    fn editor_completion_uses_preferred_class_overlay_path() {
        let game = catalog(
            r#"class BaseMode
{
	void BaseOnly();
	void OnGameStart();
}

class SCR_BaseGameMode : BaseMode
{
	void OnGameStart();
	void GameOnly();
}
"#,
            game_metadata("Game.c"),
        );
        let workspace = catalog(
            r#"modded class SCR_BaseGameMode
{
	override void OnGameStart();
	void WorkspaceOnly();
}
"#,
            workspace_metadata("Workspace.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_members_for_class("SCR_BaseGameMode");

        assert_eq!(
            completion
                .candidates
                .iter()
                .map(|candidate| (
                    candidate.name.as_deref().unwrap_or("<missing>"),
                    candidate.origin,
                    candidate.source_kind
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "OnGameStart",
                    EditorCompletionOrigin::Direct,
                    SourceKind::Workspace
                ),
                (
                    "WorkspaceOnly",
                    EditorCompletionOrigin::Direct,
                    SourceKind::Workspace
                ),
                (
                    "GameOnly",
                    EditorCompletionOrigin::Overlay,
                    SourceKind::GameData
                ),
                (
                    "BaseOnly",
                    EditorCompletionOrigin::Inherited,
                    SourceKind::GameData
                )
            ]
        );

        let on_game_start = completion
            .candidates
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("OnGameStart"))
            .unwrap();
        assert_eq!(on_game_start.source_kind, SourceKind::Workspace);
        assert_eq!(
            on_game_start.signature.as_deref(),
            Some("SCR_BaseGameMode.OnGameStart() -> void")
        );

        let shadow_group = completion
            .shadowed_groups
            .iter()
            .find(|group| group.key == "Method OnGameStart() -> void")
            .unwrap();
        assert_eq!(shadow_group.kept, on_game_start.id);
        assert!(shadow_group.shadowed.iter().any(|id| index
            .file(id.file_id)
            .is_some_and(|file| file.metadata.kind == SourceKind::GameData)));
    }

    #[test]
    fn raw_owner_name_completion_stays_separate_from_editor_completion() {
        let game = catalog(
            r#"class SCR_BaseGameMode
{
	void GameOnly();
}
"#,
            game_metadata("Game.c"),
        );
        let workspace = catalog(
            r#"modded class SCR_BaseGameMode
{
	void WorkspaceOnly();
}
"#,
            workspace_metadata("Workspace.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);
        let query = IndexQuery::new(&index);

        let raw = query.raw_completion_members_for_owner_name("SCR_BaseGameMode");
        let editor = query.completion_members_for_class("SCR_BaseGameMode");

        assert_eq!(raw.members.len(), 2);
        assert_eq!(editor.candidates.len(), 2);
        assert!(raw.members.iter().all(|id| editor
            .candidates
            .iter()
            .any(|candidate| candidate.id == *id)));
    }

    #[test]
    fn callable_signature_delegates_to_index_for_all_callable_kinds() {
        let catalog = catalog(
            r#"void GlobalFn(int value = 4);

class Example
{
	void Run(string name);
	void Example(int value);
	void ~Example();
	int m_Value;
}
"#,
            workspace_metadata("Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        assert_eq!(
            query
                .callable_signature(find(&index, SymbolKind::Function, "GlobalFn"))
                .as_deref(),
            Some("GlobalFn(int value = 4) -> void")
        );
        assert_eq!(
            query
                .callable_signature(find(&index, SymbolKind::Method, "Run"))
                .as_deref(),
            Some("Example.Run(string name) -> void")
        );
        assert_eq!(
            query
                .callable_signature(find(&index, SymbolKind::Constructor, "Example"))
                .as_deref(),
            Some("Example(int value)")
        );
        assert_eq!(
            query
                .callable_signature(find(&index, SymbolKind::Destructor, "Example"))
                .as_deref(),
            Some("~Example()")
        );
        assert_eq!(
            query.callable_signature(find(&index, SymbolKind::Field, "m_Value")),
            None
        );
    }

    #[test]
    fn missing_names_are_empty_and_do_not_panic() {
        let catalog = catalog("class Example {}", workspace_metadata("Example.c"));
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        assert_eq!(query.preferred_class("Missing"), None);
        assert_eq!(query.preferred_typedef("Missing"), None);
        assert_eq!(query.preferred_function("Missing"), None);
        assert!(query.top_level_conflicts("Missing").is_empty());
        assert!(query.raw_symbols_for_name("Missing").is_empty());
        assert!(query.raw_top_level_symbols_for_name("Missing").is_empty());

        let completion = query.completion_members_for_class("Missing");
        assert!(completion.raw_candidates.is_empty());
        assert!(completion.candidates.is_empty());
        assert!(completion.shadowed_groups.is_empty());
    }

    fn find(index: &SymbolIndex, kind: SymbolKind, name: &str) -> GlobalSymbolId {
        index
            .symbols()
            .iter()
            .find(|symbol| symbol.kind == kind && symbol.name.as_deref() == Some(name))
            .map(|symbol| symbol.id)
            .unwrap_or_else(|| panic!("missing {kind:?} {name}"))
    }

    fn catalog(source: &str, metadata: SourceFileMetadata) -> SymbolCatalog<'_> {
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ast = AstSourceFile::new(source, &parse);
        SymbolCatalog::from_ast_with_metadata(source, &ast, metadata)
    }

    fn game_metadata(path: &str) -> SourceFileMetadata {
        SourceFileMetadata {
            kind: SourceKind::GameData,
            absolute_path: Some(PathBuf::from("C:/game").join(path)),
            root_path: Some(PathBuf::from("C:/game")),
            relative_path: Some(PathBuf::from(path)),
            priority: SOURCE_PRIORITY_GAME_DATA,
        }
    }

    fn workspace_metadata(path: &str) -> SourceFileMetadata {
        SourceFileMetadata {
            kind: SourceKind::Workspace,
            absolute_path: Some(PathBuf::from("C:/workspace").join(path)),
            root_path: Some(PathBuf::from("C:/workspace")),
            relative_path: Some(PathBuf::from(path)),
            priority: SOURCE_PRIORITY_WORKSPACE,
        }
    }
}
