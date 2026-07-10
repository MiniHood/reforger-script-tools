use crate::index::{CompletionMemberLookup, GlobalSymbolId, IndexedConditionalBranch, SymbolIndex};
use crate::lexer::TextSpan;
use crate::model::{CallableForm, SourceCategory, SourceKind, SymbolKind};
use crate::symbol_display::{SymbolDisplay, SymbolDisplayInfo};
use std::collections::BTreeMap;
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
    pub source_category: SourceCategory,
    pub source_priority: u16,
    pub relative_path: Option<PathBuf>,
    pub absolute_path: Option<PathBuf>,
    pub origin: EditorCompletionOrigin,
    pub conditional_context: Vec<IndexedConditionalBranch>,
    pub callable_form: Option<CallableForm>,
    pub display: SymbolDisplayInfo,
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

    pub fn symbol_display(&self, id: GlobalSymbolId) -> Option<SymbolDisplayInfo> {
        SymbolDisplay::for_symbol(self.index, id)
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
        self.index.raw_completion_members_for_owner_name(owner)
    }

    fn editor_completion_members(
        &self,
        owner: &str,
        completion: CompletionMemberLookup,
    ) -> EditorCompletionMembers {
        let preferred_class = self.preferred_editor_class(owner);
        if preferred_class.is_none() {
            return EditorCompletionMembers {
                raw_candidates: completion.raw_candidates,
                candidates: Vec::new(),
                shadowed_groups: Vec::new(),
            };
        }

        let (candidates, shadowed_groups) =
            self.filtered_editor_completion_candidates(owner, preferred_class, &completion);

        EditorCompletionMembers {
            raw_candidates: completion.raw_candidates,
            candidates,
            shadowed_groups,
        }
    }

    fn preferred_editor_class(&self, name: &str) -> Option<GlobalSymbolId> {
        self.index
            .preferred_classes_by_name(name)
            .into_iter()
            .find(|id| self.is_editor_completion_source(*id))
    }

    fn filtered_editor_completion_candidates(
        &self,
        owner: &str,
        preferred_class: Option<GlobalSymbolId>,
        completion: &CompletionMemberLookup,
    ) -> (Vec<EditorCompletionCandidate>, Vec<EditorMemberShadowGroup>) {
        let mut ids_by_key = BTreeMap::<String, Vec<GlobalSymbolId>>::new();
        let mut key_order = Vec::<String>::new();

        for id in &completion.raw_candidates {
            if !self.is_editor_completion_source(*id) {
                continue;
            }
            let key = self.index.completion_member_key(*id);
            if !ids_by_key.contains_key(&key) {
                key_order.push(key.clone());
            }
            ids_by_key.entry(key).or_default().push(*id);
        }

        let mut candidates = Vec::new();
        let mut shadowed_groups = Vec::new();
        for key in key_order {
            let mut ids = ids_by_key.remove(&key).unwrap_or_default();
            ids.sort_by(|left, right| {
                self.compare_editor_completion_preference(owner, preferred_class, *left, *right)
            });
            let Some(kept) = ids.first().copied() else {
                continue;
            };
            if let Some(candidate) = self.editor_completion_candidate(owner, preferred_class, kept)
            {
                candidates.push(candidate);
            }
            let shadowed = ids.into_iter().filter(|id| *id != kept).collect::<Vec<_>>();
            if !shadowed.is_empty() {
                shadowed_groups.push(EditorMemberShadowGroup {
                    key,
                    kept,
                    shadowed,
                });
            }
        }

        (candidates, shadowed_groups)
    }

    fn is_editor_completion_source(&self, id: GlobalSymbolId) -> bool {
        self.index
            .file(id.file_id)
            .is_some_and(|file| file.metadata.category.is_editor_completion_default())
    }

    fn compare_editor_completion_preference(
        &self,
        owner: &str,
        preferred_class: Option<GlobalSymbolId>,
        left: GlobalSymbolId,
        right: GlobalSymbolId,
    ) -> std::cmp::Ordering {
        let left_origin = self.completion_origin(
            owner,
            preferred_class,
            self.index.symbol(left).and_then(|s| s.parent),
        );
        let right_origin = self.completion_origin(
            owner,
            preferred_class,
            self.index.symbol(right).and_then(|s| s.parent),
        );
        editor_origin_rank(left_origin)
            .cmp(&editor_origin_rank(right_origin))
            .then_with(|| {
                let left_priority = self
                    .index
                    .file(left.file_id)
                    .map(|file| file.metadata.priority)
                    .unwrap_or_default();
                let right_priority = self
                    .index
                    .file(right.file_id)
                    .map(|file| file.metadata.priority)
                    .unwrap_or_default();
                right_priority.cmp(&left_priority)
            })
            .then_with(|| {
                let left_form = self
                    .index
                    .symbol(left)
                    .and_then(|symbol| symbol.callable_form);
                let right_form = self
                    .index
                    .symbol(right)
                    .and_then(|symbol| symbol.callable_form);
                callable_form_rank(left_form).cmp(&callable_form_rank(right_form))
            })
            .then_with(|| left.file_id.cmp(&right.file_id))
            .then_with(|| left.symbol_id.cmp(&right.symbol_id))
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
        let display = self.symbol_display(id)?;
        let detail = display.detail.clone();

        Some(EditorCompletionCandidate {
            id,
            name: symbol.name.clone(),
            kind: symbol.kind,
            detail,
            signature: display.signature.clone(),
            span: symbol.span,
            selection_span: symbol.selection_span,
            source_kind: file.metadata.kind,
            source_category: file.metadata.category,
            source_priority: file.metadata.priority,
            relative_path: file.metadata.relative_path.clone(),
            absolute_path: file.metadata.absolute_path.clone(),
            origin,
            conditional_context: symbol.conditional_context.clone(),
            callable_form: symbol.callable_form,
            display,
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

const fn editor_origin_rank(origin: EditorCompletionOrigin) -> u8 {
    match origin {
        EditorCompletionOrigin::Direct => 0,
        EditorCompletionOrigin::Overlay => 1,
        EditorCompletionOrigin::Inherited => 2,
        EditorCompletionOrigin::Unknown => 3,
    }
}

const fn callable_form_rank(form: Option<CallableForm>) -> u8 {
    match form {
        Some(CallableForm::Implementation) => 0,
        Some(CallableForm::Declaration) => 1,
        Some(CallableForm::Prototype) => 2,
        None => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::index::SymbolIndex;
    use crate::model::{
        source_category_for_path, SourceCategory, SourceFileMetadata, SymbolCatalog,
        SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE,
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
    fn editor_completion_excludes_non_runtime_source_categories_by_default() {
        let docs = catalog(
            r#"class Example
{
	void DocsOnly();
}
"#,
            game_metadata("GameLib/WorldSystemsDocs.c"),
        );
        let workbench = catalog(
            r#"class Example
{
	void WorkbenchOnly();
}
"#,
            game_metadata("WorkbenchGame/Example.c"),
        );
        let runtime = catalog(
            r#"class Example
{
	void RuntimeOnly();
}
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&docs, &workbench, &runtime]);
        let query = IndexQuery::new(&index);

        let raw = query.raw_completion_members_for_owner_name("Example");
        let editor = query.completion_members_for_class("Example");

        assert_eq!(raw.members.len(), 3);
        assert_eq!(
            editor
                .candidates
                .iter()
                .map(|candidate| candidate.name.as_deref().unwrap())
                .collect::<Vec<_>>(),
            vec!["RuntimeOnly"]
        );
        assert_eq!(editor.candidates[0].source_category, SourceCategory::Game);
    }

    #[test]
    fn editor_completion_returns_no_candidates_when_class_is_docs_only() {
        let docs = catalog(
            r#"class HelloWorldSystem : WorldSystem
{
	void DocsOnly();
}

class WorldSystem
{
	void GeneratedBase();
}
"#,
            game_metadata("GameLib/WorldSystemsDocs.c"),
        );
        let index = SymbolIndex::from_catalogs([&docs]);
        let query = IndexQuery::new(&index);

        let editor = query.completion_members_for_class("HelloWorldSystem");

        assert!(!editor.raw_candidates.is_empty());
        assert!(editor.candidates.is_empty());
        assert!(editor.shadowed_groups.is_empty());
    }

    #[test]
    fn editor_completion_uses_lower_priority_runtime_class_when_higher_priority_class_is_excluded()
    {
        let docs = catalog(
            r#"class Example
{
	void DocsOnly();
}
"#,
            game_metadata("GameLib/WorldSystemsDocs.c"),
        );
        let runtime = catalog(
            r#"class Example
{
	void RuntimeOnly();
}
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&docs, &runtime]);
        let query = IndexQuery::new(&index);

        let editor = query.completion_members_for_class("Example");

        assert_eq!(
            editor
                .candidates
                .iter()
                .map(|candidate| (candidate.name.as_deref().unwrap(), candidate.origin))
                .collect::<Vec<_>>(),
            vec![("RuntimeOnly", EditorCompletionOrigin::Direct)]
        );
    }

    #[test]
    fn editor_completion_exposes_conditional_context_and_callable_form() {
        let catalog = catalog(
            r#"#ifndef DISABLE_INVENTORY
class Example
{
	void Run() {}
}
#endif
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_members_for_class("Example");
        let run = completion
            .candidates
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("Run"))
            .unwrap();

        assert_eq!(run.callable_form, Some(CallableForm::Implementation));
        assert_eq!(run.conditional_context.len(), 1);
        assert_eq!(run.conditional_context[0].kind.as_str(), "#ifndef");
        assert_eq!(
            run.conditional_context[0].condition.as_deref(),
            Some("DISABLE_INVENTORY")
        );
    }

    #[test]
    fn editor_completion_prefers_implementation_over_declaration_for_same_key() {
        let catalog = catalog(
            r#"class Example
{
	void Run();
	void Run() {}
}
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_members_for_class("Example");
        let run = completion
            .candidates
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("Run"))
            .unwrap();

        assert_eq!(run.callable_form, Some(CallableForm::Implementation));
        let shadow_group = completion
            .shadowed_groups
            .iter()
            .find(|group| group.key == "Method Run() -> void")
            .unwrap();
        assert_eq!(shadow_group.kept, run.id);
        assert_eq!(shadow_group.shadowed.len(), 1);
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
    fn symbol_display_returns_editor_ready_metadata() {
        let catalog = catalog(
            r#"//! Run documentation.
class Example
{
	[Attribute()]
	protected void Run(int value = 4);
}
"#,
            workspace_metadata("Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);
        let method = find(&index, SymbolKind::Method, "Run");

        let display = query.symbol_display(method).unwrap();

        assert_eq!(display.label, "Run");
        assert_eq!(display.kind, SymbolKind::Method);
        assert_eq!(
            display.signature.as_deref(),
            Some("Example.Run(int value = 4) -> void")
        );
        assert_eq!(
            display.detail.as_deref(),
            Some("Example.Run(int value = 4) -> void")
        );
        assert_eq!(display.modifiers, vec!["protected"]);
        assert_eq!(display.attributes[0].name.as_deref(), Some("Attribute"));
        assert_eq!(display.source_kind, SourceKind::Workspace);
    }

    #[test]
    fn completion_candidates_include_shared_symbol_display() {
        let catalog = catalog(
            r#"class Example
{
	//! Value docs.
	int m_Value;
}
"#,
            workspace_metadata("Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_members_for_class("Example");
        let candidate = completion
            .candidates
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("m_Value"))
            .unwrap();

        assert_eq!(candidate.detail.as_deref(), Some("type int"));
        assert_eq!(candidate.display.label, "m_Value");
        assert_eq!(candidate.display.detail.as_deref(), Some("type int"));
        assert_eq!(
            candidate.display.documentation_preview.as_deref(),
            Some("Value docs.")
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
        let relative_path = PathBuf::from(path);
        let mut category = source_category_for_path(SourceKind::GameData, Some(&relative_path));
        if category == SourceCategory::Unknown {
            category = SourceCategory::Game;
        }
        SourceFileMetadata {
            kind: SourceKind::GameData,
            category,
            absolute_path: Some(PathBuf::from("C:/game").join(path)),
            root_path: Some(PathBuf::from("C:/game")),
            relative_path: Some(relative_path),
            priority: SOURCE_PRIORITY_GAME_DATA,
        }
    }

    fn workspace_metadata(path: &str) -> SourceFileMetadata {
        let relative_path = PathBuf::from(path);
        SourceFileMetadata {
            kind: SourceKind::Workspace,
            category: SourceCategory::Workspace,
            absolute_path: Some(PathBuf::from("C:/workspace").join(path)),
            root_path: Some(PathBuf::from("C:/workspace")),
            relative_path: Some(relative_path),
            priority: SOURCE_PRIORITY_WORKSPACE,
        }
    }
}
