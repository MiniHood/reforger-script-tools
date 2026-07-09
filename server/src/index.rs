use crate::lexer::TextSpan;
use crate::model::{SourceFileMetadata, SourceKind, SymbolCatalog, SymbolId, SymbolKind};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceFileId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalSymbolId {
    pub file_id: SourceFileId,
    pub symbol_id: SymbolId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedFile {
    pub id: SourceFileId,
    pub metadata: SourceFileMetadata,
    pub symbol_start: usize,
    pub symbol_count: usize,
    pub non_declaration_callable_fragments: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSymbolDetail {
    pub type_text: Option<String>,
    pub return_type_text: Option<String>,
    pub base_type: Option<String>,
    pub default_text: Option<String>,
    pub enum_value_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSymbol {
    pub id: GlobalSymbolId,
    pub parent: Option<GlobalSymbolId>,
    pub kind: SymbolKind,
    pub name: Option<String>,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub detail: IndexedSymbolDetail,
}

#[derive(Debug, Clone, Default)]
pub struct SymbolIndex {
    files: Vec<IndexedFile>,
    symbols: Vec<IndexedSymbol>,
    by_name: BTreeMap<String, Vec<GlobalSymbolId>>,
    top_level_by_name: BTreeMap<String, Vec<GlobalSymbolId>>,
    by_kind: BTreeMap<SymbolKind, Vec<GlobalSymbolId>>,
    children: BTreeMap<GlobalSymbolId, Vec<GlobalSymbolId>>,
    classes_by_name: BTreeMap<String, Vec<GlobalSymbolId>>,
    typedefs_by_name: BTreeMap<String, Vec<GlobalSymbolId>>,
    methods_by_owner_name: BTreeMap<(String, String), Vec<GlobalSymbolId>>,
}

impl SymbolIndex {
    pub fn from_catalogs<'source>(
        catalogs: impl IntoIterator<Item = &'source SymbolCatalog<'source>>,
    ) -> Self {
        let mut index = Self::default();
        for catalog in catalogs {
            index.add_catalog(catalog);
        }
        index
    }

    pub fn add_catalog<'source>(&mut self, catalog: &SymbolCatalog<'source>) -> SourceFileId {
        let file_id = SourceFileId(self.files.len());
        let symbol_start = self.symbols.len();

        self.files.push(IndexedFile {
            id: file_id,
            metadata: catalog.metadata().clone(),
            symbol_start,
            symbol_count: catalog.records().len(),
            non_declaration_callable_fragments: catalog.non_declaration_callable_fragments(),
        });

        for record in catalog.records() {
            let id = GlobalSymbolId {
                file_id,
                symbol_id: record.id,
            };
            let parent = record
                .parent
                .map(|symbol_id| GlobalSymbolId { file_id, symbol_id });
            let name = catalog.record_name(record).map(str::to_string);
            let symbol = IndexedSymbol {
                id,
                parent,
                kind: record.kind,
                name,
                span: record.span,
                selection_span: record.selection_span,
                detail: IndexedSymbolDetail {
                    type_text: record
                        .detail
                        .type_text
                        .map(|span| catalog.text(span).to_string()),
                    return_type_text: record
                        .detail
                        .return_type_text
                        .map(|span| catalog.text(span).to_string()),
                    base_type: record
                        .detail
                        .base_type
                        .map(|span| catalog.text(span).to_string()),
                    default_text: record
                        .detail
                        .default_text
                        .map(|span| catalog.text(span).to_string()),
                    enum_value_text: record
                        .detail
                        .enum_value_text
                        .map(|span| catalog.text(span).to_string()),
                },
            };

            self.index_symbol(catalog, &symbol);
            self.symbols.push(symbol);
        }

        file_id
    }

    pub fn files(&self) -> &[IndexedFile] {
        &self.files
    }

    pub fn symbols(&self) -> &[IndexedSymbol] {
        &self.symbols
    }

    pub fn file(&self, id: SourceFileId) -> Option<&IndexedFile> {
        self.files.get(id.0)
    }

    pub fn symbol(&self, id: GlobalSymbolId) -> Option<&IndexedSymbol> {
        let file = self.file(id.file_id)?;
        let local_index = id.symbol_id.0;
        if local_index >= file.symbol_count {
            return None;
        }
        self.symbols.get(file.symbol_start + local_index)
    }

    pub fn symbols_for_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.by_name.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn top_level_symbols_for_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.top_level_by_name
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn preferred_symbols_for_name(&self, name: &str) -> Vec<GlobalSymbolId> {
        self.preferred_from_symbols(self.symbols_for_name(name))
    }

    pub fn preferred_top_level_symbols_for_name(&self, name: &str) -> Vec<GlobalSymbolId> {
        self.preferred_from_symbols(self.top_level_symbols_for_name(name))
    }

    pub fn preferred_from_symbols(&self, symbols: &[GlobalSymbolId]) -> Vec<GlobalSymbolId> {
        let mut symbols = symbols.to_vec();
        symbols.sort_by(|left, right| self.compare_symbol_preference(*left, *right));
        symbols
    }

    pub fn symbols_for_kind(&self, kind: SymbolKind) -> &[GlobalSymbolId] {
        self.by_kind.get(&kind).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn children(&self, parent: GlobalSymbolId) -> &[GlobalSymbolId] {
        self.children.get(&parent).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn classes_by_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.classes_by_name
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn typedefs_by_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.typedefs_by_name
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn methods_by_owner_name(&self, owner: &str, name: &str) -> &[GlobalSymbolId] {
        self.methods_by_owner_name
            .get(&(owner.to_string(), name.to_string()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn method_owner_name_groups(&self) -> &BTreeMap<(String, String), Vec<GlobalSymbolId>> {
        &self.methods_by_owner_name
    }

    pub fn names(&self) -> &BTreeMap<String, Vec<GlobalSymbolId>> {
        &self.by_name
    }

    pub fn duplicate_names(&self) -> Vec<(&str, &[GlobalSymbolId])> {
        self.by_name
            .iter()
            .filter(|(_, symbols)| symbols.len() > 1)
            .map(|(name, symbols)| (name.as_str(), symbols.as_slice()))
            .collect()
    }

    pub fn duplicate_top_level_names(&self) -> Vec<(&str, &[GlobalSymbolId])> {
        self.top_level_by_name
            .iter()
            .filter(|(_, symbols)| symbols.len() > 1)
            .map(|(name, symbols)| (name.as_str(), symbols.as_slice()))
            .collect()
    }

    pub fn map_counts(&self) -> IndexMapCounts {
        IndexMapCounts {
            names: self.by_name.len(),
            top_level_names: self.top_level_by_name.len(),
            kinds: self.by_kind.len(),
            class_names: self.classes_by_name.len(),
            typedef_names: self.typedefs_by_name.len(),
            method_owner_names: self.methods_by_owner_name.len(),
            parent_symbols: self.children.len(),
        }
    }

    pub fn source_kind_counts(&self) -> BTreeMap<SourceKind, usize> {
        let mut counts = BTreeMap::new();
        for file in &self.files {
            *counts.entry(file.metadata.kind).or_default() += 1;
        }
        counts
    }

    fn index_symbol<'source>(&mut self, catalog: &SymbolCatalog<'source>, symbol: &IndexedSymbol) {
        self.by_kind.entry(symbol.kind).or_default().push(symbol.id);

        if let Some(parent) = symbol.parent {
            self.children.entry(parent).or_default().push(symbol.id);
        }

        let Some(name) = &symbol.name else {
            return;
        };

        self.by_name
            .entry(name.clone())
            .or_default()
            .push(symbol.id);

        if symbol.parent.is_none() {
            self.top_level_by_name
                .entry(name.clone())
                .or_default()
                .push(symbol.id);
        }

        match symbol.kind {
            SymbolKind::Class => {
                self.classes_by_name
                    .entry(name.clone())
                    .or_default()
                    .push(symbol.id);
            }
            SymbolKind::Typedef => {
                self.typedefs_by_name
                    .entry(name.clone())
                    .or_default()
                    .push(symbol.id);
            }
            SymbolKind::Method => {
                if let Some(owner) = symbol
                    .parent
                    .and_then(|parent| catalog.record(parent.symbol_id))
                    .and_then(|parent| catalog.record_name(parent))
                {
                    self.methods_by_owner_name
                        .entry((owner.to_string(), name.clone()))
                        .or_default()
                        .push(symbol.id);
                }
            }
            _ => {}
        }
    }

    fn compare_symbol_preference(
        &self,
        left: GlobalSymbolId,
        right: GlobalSymbolId,
    ) -> std::cmp::Ordering {
        let left_file = self.file(left.file_id);
        let right_file = self.file(right.file_id);
        let left_priority = left_file
            .map(|file| file.metadata.priority)
            .unwrap_or_default();
        let right_priority = right_file
            .map(|file| file.metadata.priority)
            .unwrap_or_default();

        right_priority
            .cmp(&left_priority)
            .then_with(|| left.file_id.cmp(&right.file_id))
            .then_with(|| left.symbol_id.cmp(&right.symbol_id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexMapCounts {
    pub names: usize,
    pub top_level_names: usize,
    pub kinds: usize,
    pub class_names: usize,
    pub typedef_names: usize,
    pub method_owner_names: usize,
    pub parent_symbols: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::model::{SourceFileMetadata, SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE};
    use crate::parser::parse_source;
    use std::path::PathBuf;

    #[test]
    fn indexes_names_kinds_children_classes_typedefs_and_methods() {
        let source = r#"typedef string FactionKey;

class Example : Base
{
	int m_Value;
	void Run(int value);
}
"#;
        let catalog = catalog(
            source,
            SourceFileMetadata {
                kind: SourceKind::GameData,
                absolute_path: Some(PathBuf::from("C:/game/Example.c")),
                root_path: Some(PathBuf::from("C:/game")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        assert_eq!(index.files().len(), 1);
        assert_eq!(index.symbols().len(), 5);
        assert_eq!(index.symbols_for_name("Example").len(), 1);
        assert_eq!(index.top_level_symbols_for_name("Example").len(), 1);
        assert_eq!(index.classes_by_name("Example").len(), 1);
        assert_eq!(index.typedefs_by_name("FactionKey").len(), 1);
        assert_eq!(index.methods_by_owner_name("Example", "Run").len(), 1);
        assert_eq!(index.symbols_for_kind(SymbolKind::Parameter).len(), 1);

        let class_id = index.classes_by_name("Example")[0];
        let children = index.children(class_id);
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|id| index
            .symbol(*id)
            .is_some_and(|symbol| symbol.name.as_deref() == Some("Run"))));
    }

    #[test]
    fn global_ids_keep_file_id_and_file_local_symbol_id() {
        let game = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::GameData,
                absolute_path: Some(PathBuf::from("C:/game/Example.c")),
                root_path: Some(PathBuf::from("C:/game")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let workspace = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::Workspace,
                absolute_path: Some(PathBuf::from("C:/workspace/Example.c")),
                root_path: Some(PathBuf::from("C:/workspace")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_WORKSPACE,
            },
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);

        let symbols = index.symbols_for_name("Example");
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].file_id, SourceFileId(0));
        assert_eq!(symbols[0].symbol_id, SymbolId(0));
        assert_eq!(symbols[1].file_id, SourceFileId(1));
        assert_eq!(symbols[1].symbol_id, SymbolId(0));

        let preferred = index.preferred_symbols_for_name("Example");
        assert_eq!(preferred[0].file_id, SourceFileId(1));
        assert_eq!(
            index.file(preferred[0].file_id).unwrap().metadata.kind,
            SourceKind::Workspace
        );
        assert_eq!(index.duplicate_top_level_names().len(), 1);
    }

    #[test]
    fn preferred_top_level_lookup_excludes_non_top_level_symbols() {
        let game = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::GameData,
                absolute_path: Some(PathBuf::from("C:/game/Example.c")),
                root_path: Some(PathBuf::from("C:/game")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let workspace = catalog(
            r#"class Example
{
	void Run(int Example);
}
"#,
            SourceFileMetadata {
                kind: SourceKind::Workspace,
                absolute_path: Some(PathBuf::from("C:/workspace/Example.c")),
                root_path: Some(PathBuf::from("C:/workspace")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_WORKSPACE,
            },
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);

        let all = index.symbols_for_name("Example");
        assert_eq!(all.len(), 3);
        assert!(all.iter().any(|id| index
            .symbol(*id)
            .is_some_and(|symbol| symbol.kind == SymbolKind::Parameter)));

        let top_level = index.top_level_symbols_for_name("Example");
        assert_eq!(top_level.len(), 2);
        assert!(top_level.iter().all(|id| index
            .symbol(*id)
            .is_some_and(|symbol| symbol.parent.is_none())));

        let preferred_all = index.preferred_symbols_for_name("Example");
        assert_eq!(preferred_all.len(), 3);

        let preferred_top_level = index.preferred_top_level_symbols_for_name("Example");
        assert_eq!(preferred_top_level.len(), 2);
        assert_eq!(preferred_top_level[0].file_id, SourceFileId(1));
        assert_eq!(
            index.symbol(preferred_top_level[0]).unwrap().kind,
            SymbolKind::Class
        );
        assert_eq!(
            index
                .file(preferred_top_level[0].file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::Workspace
        );
    }

    #[test]
    fn stores_copied_lookup_details_without_requiring_source_text() {
        let catalog = catalog(
            r#"enum E
{
	One = 1,
}

class Example : Base
{
	void Run(int value = 4);
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let class = index.symbol(index.classes_by_name("Example")[0]).unwrap();
        assert_eq!(class.detail.base_type.as_deref(), Some("Base"));

        let method = index
            .symbol(index.methods_by_owner_name("Example", "Run")[0])
            .unwrap();
        assert_eq!(method.detail.return_type_text.as_deref(), Some("void"));

        let parameter = index.symbols_for_kind(SymbolKind::Parameter)[0];
        let parameter = index.symbol(parameter).unwrap();
        assert_eq!(parameter.detail.type_text.as_deref(), Some("int"));
        assert_eq!(parameter.detail.default_text.as_deref(), Some("4"));

        let enum_member = index.symbols_for_name("One")[0];
        let enum_member = index.symbol(enum_member).unwrap();
        assert_eq!(enum_member.detail.enum_value_text.as_deref(), Some("1"));
    }

    #[test]
    fn exposes_method_owner_name_groups_for_overload_review() {
        let catalog = catalog(
            r#"class SCR_AutotestHarness
{
	void Begin();
	void Begin(int value);
	int Count();
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        assert_eq!(
            index
                .methods_by_owner_name("SCR_AutotestHarness", "Begin")
                .len(),
            2
        );
        assert_eq!(
            index
                .methods_by_owner_name("SCR_AutotestHarness", "Count")
                .len(),
            1
        );

        let begin_key = ("SCR_AutotestHarness".to_string(), "Begin".to_string());
        let count_key = ("SCR_AutotestHarness".to_string(), "Count".to_string());
        assert_eq!(index.method_owner_name_groups()[&begin_key].len(), 2);
        assert_eq!(index.method_owner_name_groups()[&count_key].len(), 1);
    }

    #[test]
    fn preferred_from_symbols_sorts_by_priority_then_stable_ids() {
        let first_game = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::GameData,
                absolute_path: Some(PathBuf::from("C:/game/First.c")),
                root_path: Some(PathBuf::from("C:/game")),
                relative_path: Some(PathBuf::from("First.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let workspace = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::Workspace,
                absolute_path: Some(PathBuf::from("C:/workspace/Example.c")),
                root_path: Some(PathBuf::from("C:/workspace")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_WORKSPACE,
            },
        );
        let second_game = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::GameData,
                absolute_path: Some(PathBuf::from("C:/game/Second.c")),
                root_path: Some(PathBuf::from("C:/game")),
                relative_path: Some(PathBuf::from("Second.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let index = SymbolIndex::from_catalogs([&first_game, &workspace, &second_game]);
        let unsorted = [
            GlobalSymbolId {
                file_id: SourceFileId(2),
                symbol_id: SymbolId(0),
            },
            GlobalSymbolId {
                file_id: SourceFileId(0),
                symbol_id: SymbolId(0),
            },
            GlobalSymbolId {
                file_id: SourceFileId(1),
                symbol_id: SymbolId(0),
            },
        ];

        let preferred = index.preferred_from_symbols(&unsorted);

        assert_eq!(preferred[0].file_id, SourceFileId(1));
        assert_eq!(preferred[1].file_id, SourceFileId(0));
        assert_eq!(preferred[2].file_id, SourceFileId(2));
    }

    fn catalog(source: &str, metadata: SourceFileMetadata) -> SymbolCatalog<'_> {
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ast = AstSourceFile::new(source, &parse);
        SymbolCatalog::from_ast_with_metadata(source, &ast, metadata)
    }
}
