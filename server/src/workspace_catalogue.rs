use crate::game_data_inspection::{
    inspect, read_source, GameDataInspectionError, GameDataInspectionOutput,
    GameDataSourceReadRequest,
};
use crate::game_data_research::{
    list_members, query_relationships, GameDataMemberPage, GameDataMemberRequest,
    GameDataRelationshipPage, GameDataRelationshipRequest, GameDataResearchError,
};
use crate::game_data_search::{
    search_workspace, GameDataAddonMap, GameDataSearchError, GameDataSearchPage,
    GameDataSearchRequest, SourceLineStarts,
};
use crate::index::{SourceFileId, SymbolIndex};
use crate::index_build::{
    build_index_with_control, IndexBuildConfig, IndexBuildControl, IndexSourceRoot,
};
use crate::model::{SourceKind, SOURCE_PRIORITY_WORKSPACE};
use crate::text_search::{
    page as page_text, physical_source_uri, scan as scan_text, TextSearchCorpus, TextSearchError,
    TextSearchOptions, TextSearchPage, TextSearchRequest, TextSearchResultSet, TextSource,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct WorkspaceCatalogueConfig {
    pub roots: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct WorkspaceCatalogue {
    config: WorkspaceCatalogueConfig,
    snapshot: Mutex<Option<Arc<WorkspaceSnapshot>>>,
    text_search_cache:
        Mutex<BTreeMap<(String, String, TextSearchOptions), Arc<TextSearchResultSet>>>,
}

#[derive(Debug)]
struct WorkspaceSnapshot {
    revision: String,
    index: Arc<SymbolIndex>,
    starts: Arc<BTreeMap<SourceFileId, SourceLineStarts>>,
    sources: Arc<BTreeMap<SourceFileId, Arc<str>>>,
}

#[derive(Debug)]
pub enum WorkspaceCatalogueError {
    Unavailable,
    Initialization(String),
    Search(GameDataSearchError),
    TextSearch(TextSearchError),
    Inspection(GameDataInspectionError),
    Research(GameDataResearchError),
}

impl WorkspaceCatalogue {
    pub fn new(config: WorkspaceCatalogueConfig) -> Self {
        Self {
            config,
            snapshot: Mutex::new(None),
            text_search_cache: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn has_configured_roots(&self) -> bool {
        !self.config.roots.is_empty()
    }

    pub fn search(
        &self,
        control: &IndexBuildControl,
        request: GameDataSearchRequest,
    ) -> Result<GameDataSearchPage, WorkspaceCatalogueError> {
        let snapshot = self.snapshot(control)?;
        search_workspace(
            &snapshot.index,
            &snapshot.starts,
            control,
            &snapshot.revision,
            request,
        )
        .map_err(WorkspaceCatalogueError::Search)
    }

    pub fn search_text(
        &self,
        control: &IndexBuildControl,
        request: TextSearchRequest,
    ) -> Result<TextSearchPage, WorkspaceCatalogueError> {
        let snapshot = self.snapshot(control)?;
        let cache_key = (
            snapshot.revision.clone(),
            request.query.clone(),
            request.options,
        );
        if let Some(result_set) = self
            .text_search_cache
            .lock()
            .unwrap()
            .get(&cache_key)
            .cloned()
        {
            return page_text(&result_set, control, request)
                .map_err(WorkspaceCatalogueError::TextSearch);
        }
        let sources = snapshot
            .index
            .files()
            .iter()
            .filter_map(|file| {
                let relative_path = file
                    .metadata
                    .relative_path
                    .as_ref()?
                    .to_string_lossy()
                    .replace('\\', "/");
                let content = snapshot.sources.get(&file.id)?.clone();
                Some(TextSource {
                    relative_path,
                    addon_guid: None,
                    addon_label: None,
                    source_uri: file
                        .metadata
                        .absolute_path
                        .as_deref()
                        .and_then(physical_source_uri),
                    content,
                })
            })
            .collect();
        let result_set = scan_text(
            TextSearchCorpus {
                files_considered: snapshot.index.files().len(),
                sources,
                source_read_failures: 0,
                ..TextSearchCorpus::default()
            },
            control,
            &snapshot.revision,
            &request,
        )
        .map_err(WorkspaceCatalogueError::TextSearch)
        .map(Arc::new)?;
        let mut cache = self.text_search_cache.lock().unwrap();
        cache.insert(cache_key, result_set.clone());
        while cache.len() > 8 {
            let oldest = cache.keys().next().cloned();
            if let Some(oldest) = oldest {
                cache.remove(&oldest);
            }
        }
        drop(cache);
        page_text(&result_set, control, request).map_err(WorkspaceCatalogueError::TextSearch)
    }

    pub fn inspect(
        &self,
        control: &IndexBuildControl,
        symbol_ref: &str,
    ) -> Result<GameDataInspectionOutput, WorkspaceCatalogueError> {
        let snapshot = self.snapshot(control)?;
        inspect(
            &snapshot.index,
            &snapshot.starts,
            &GameDataAddonMap::new(),
            control,
            &snapshot.revision,
            symbol_ref,
        )
        .map_err(WorkspaceCatalogueError::Inspection)
    }

    pub fn read_source(
        &self,
        control: &IndexBuildControl,
        request: GameDataSourceReadRequest,
    ) -> Result<serde_json::Value, WorkspaceCatalogueError> {
        let snapshot = self.snapshot(control)?;
        read_source(
            &snapshot.index,
            &GameDataAddonMap::new(),
            control,
            &snapshot.revision,
            &snapshot.sources,
            request,
        )
        .map_err(WorkspaceCatalogueError::Inspection)
    }

    pub fn list_members(
        &self,
        control: &IndexBuildControl,
        request: GameDataMemberRequest,
    ) -> Result<GameDataMemberPage, WorkspaceCatalogueError> {
        let snapshot = self.snapshot(control)?;
        list_members(
            &snapshot.index,
            &snapshot.starts,
            &GameDataAddonMap::new(),
            control,
            &snapshot.revision,
            request,
        )
        .map_err(WorkspaceCatalogueError::Research)
    }

    pub fn query_relationships(
        &self,
        control: &IndexBuildControl,
        request: GameDataRelationshipRequest,
    ) -> Result<GameDataRelationshipPage, WorkspaceCatalogueError> {
        let snapshot = self.snapshot(control)?;
        query_relationships(
            &snapshot.index,
            &snapshot.sources,
            &snapshot.starts,
            control,
            &snapshot.revision,
            request,
        )
        .map_err(WorkspaceCatalogueError::Research)
    }

    fn snapshot(
        &self,
        control: &IndexBuildControl,
    ) -> Result<Arc<WorkspaceSnapshot>, WorkspaceCatalogueError> {
        if !self.has_configured_roots() {
            return Err(WorkspaceCatalogueError::Unavailable);
        }
        if let Some(snapshot) = self.snapshot.lock().unwrap().clone() {
            return Ok(snapshot);
        }
        control
            .check()
            .map_err(WorkspaceCatalogueError::Initialization)?;
        let built = build_index_with_control(
            &IndexBuildConfig {
                roots: self
                    .config
                    .roots
                    .iter()
                    .map(|root| {
                        IndexSourceRoot::new(root, SourceKind::Workspace, SOURCE_PRIORITY_WORKSPACE)
                    })
                    .collect(),
            },
            control,
        )
        .map_err(WorkspaceCatalogueError::Initialization)?;
        let mut starts = BTreeMap::new();
        let mut sources = BTreeMap::new();
        let mut hasher = Sha256::new();
        for root in &self.config.roots {
            hasher.update(root.to_string_lossy().as_bytes());
        }
        for (file, cached_starts) in built.index.files().iter().zip(&built.source_line_starts) {
            control
                .check()
                .map_err(WorkspaceCatalogueError::Initialization)?;
            let path = file.metadata.absolute_path.as_ref().ok_or_else(|| {
                WorkspaceCatalogueError::Initialization(
                    "workspace index file has no physical path".to_string(),
                )
            })?;
            let bytes = fs::read(path).map_err(|error| {
                WorkspaceCatalogueError::Initialization(format!(
                    "failed to read indexed workspace file {}: {error}",
                    path.display()
                ))
            })?;
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(&bytes);
            let source = String::from_utf8_lossy(&bytes).into_owned();
            starts.insert(
                file.id,
                SourceLineStarts::from_cached_starts(cached_starts.clone()),
            );
            sources.insert(file.id, Arc::<str>::from(source));
        }
        let snapshot = Arc::new(WorkspaceSnapshot {
            revision: format!("ws1:{:x}", hasher.finalize()),
            index: Arc::new(built.index),
            starts: Arc::new(starts),
            sources: Arc::new(sources),
        });
        *self.snapshot.lock().unwrap() = Some(snapshot.clone());
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_data_search::GameDataSearchRequest;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn indexes_workspace_symbols_and_keeps_revision_bound_inspection() {
        let root = std::env::temp_dir().join(format!(
            "reforger-workspace-catalogue-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Example.c"), "class Example { void Run() {} }\n").unwrap();
        let catalogue = WorkspaceCatalogue::new(WorkspaceCatalogueConfig {
            roots: vec![root.clone()],
        });
        let control = IndexBuildControl::default();
        let page = catalogue
            .search(&control, GameDataSearchRequest::new("Run"))
            .unwrap();
        assert_eq!(page.returned, 1);
        assert_eq!(page.results[0].source_category, "workspace");
        let inspection = catalogue
            .inspect(&control, &page.results[0].symbol_ref)
            .unwrap();
        assert_eq!(inspection.name.as_deref(), Some("Run"));
        assert_eq!(inspection.source_category, "workspace");
        assert!(catalogue
            .search(&control, GameDataSearchRequest::new("Run"))
            .unwrap()
            .next_cursor
            .is_none());
        let text = catalogue
            .search_text(
                &control,
                TextSearchRequest {
                    query: "void Run".to_string(),
                    addon_guids: None,
                    options: TextSearchOptions::default(),
                    limit: Some(10),
                    cursor: None,
                },
            )
            .unwrap();
        assert_eq!(text.total, 1);
        assert_eq!(text.results[0].relative_path, "Example.c");
        assert_eq!(text.results[0].match_range.start_line, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reports_unavailable_without_explicit_workspace_roots() {
        let catalogue = WorkspaceCatalogue::new(WorkspaceCatalogueConfig { roots: Vec::new() });
        let result = catalogue.search(
            &IndexBuildControl::default(),
            GameDataSearchRequest::new("Example"),
        );
        assert!(matches!(result, Err(WorkspaceCatalogueError::Unavailable)));
    }
}
