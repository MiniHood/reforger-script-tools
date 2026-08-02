use crate::addon_sources::{read_cached_virtual_source, read_cached_virtual_sources};
use crate::game_data_inspection::{
    inspect, read_source as read_source_evidence, GameDataInspectionError,
    GameDataInspectionOutput, GameDataSourceReadRequest,
};
use crate::game_data_research::{
    list_members, GameDataExamplePage, GameDataExampleSearchRequest, GameDataMemberPage,
    GameDataMemberRequest, GameDataRelationshipPage, GameDataRelationshipRequest,
    GameDataResearchError,
};
use crate::game_data_search::{
    search, GameDataSearchError, GameDataSearchPage, GameDataSearchRequest, SourceLineStarts,
};
use crate::index::{SourceFileId, SymbolIndex};
use crate::index_build::{IndexBuildControl, INDEX_BUILD_CANCELLED};
use crate::index_cache::{
    load_game_data_index_cache_with_control, GameDataIndexCacheResult, IndexCacheStatus,
    IndexCacheTimings, RuntimeIndexSummary, SourceFingerprint,
};
use crate::model::SymbolKind;
use crate::text_search::{
    page as page_text, scan as scan_text, TextSearchCorpus, TextSearchError, TextSearchOptions,
    TextSearchPage, TextSearchRequest, TextSearchResultSet, TextSource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

pub const GAME_DATA_INITIALIZATION_DEADLINE_MS: u64 = 120_000;
pub const MAX_STRUCTURED_RESULT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone)]
pub struct GameDataCatalogueConfig {
    pub cache_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct GameDataCatalogue {
    config: GameDataCatalogueConfig,
    state: Mutex<Option<GameDataCatalogueState>>,
    text_search_cache:
        Mutex<BTreeMap<(String, String, TextSearchOptions), Arc<TextSearchResultSet>>>,
    initialized: AtomicBool,
    #[cfg(all(feature = "test-hooks", debug_assertions))]
    panic_once: std::sync::atomic::AtomicBool,
}

#[derive(Debug, Clone)]
struct GameDataCatalogueState {
    status: GameDataStatus,
    // Ticket #17 adds semantic queries over this exact immutable index.
    index: Option<Arc<SymbolIndex>>,
    source_line_starts: Arc<BTreeMap<SourceFileId, SourceLineStarts>>,
}

impl GameDataCatalogue {
    pub fn new(config: GameDataCatalogueConfig) -> Self {
        Self {
            config,
            state: Mutex::new(None),
            text_search_cache: Mutex::new(BTreeMap::new()),
            initialized: AtomicBool::new(false),
            #[cfg(all(feature = "test-hooks", debug_assertions))]
            panic_once: std::sync::atomic::AtomicBool::new(
                std::env::var("REFORGER_MCP_TEST_PANIC_ONCE").as_deref() == Ok("1"),
            ),
        }
    }

    pub fn status(&self, control: &IndexBuildControl) -> Result<GameDataStatus, String> {
        control.check()?;
        let mut state = self.lock_state(control)?;
        if let Some(state) = state.as_ref() {
            return Ok(state.status.clone());
        }

        self.before_initialization(control)?;
        let initialized = initialize_catalogue(&self.config, control)?;
        let status = initialized.status.clone();
        *state = Some(initialized);
        self.initialized.store(true, Ordering::Release);
        Ok(status)
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    pub fn search(
        &self,
        control: &IndexBuildControl,
        request: GameDataSearchRequest,
    ) -> Result<GameDataSearchPage, GameDataCatalogueSearchError> {
        self.before_operation(control)
            .map_err(GameDataCatalogueSearchError::Initialization)?;
        let status = self
            .status(control)
            .map_err(GameDataCatalogueSearchError::Initialization)?;
        if !status.available {
            return Err(GameDataCatalogueSearchError::Unavailable);
        }
        let state = self
            .lock_state(control)
            .map_err(GameDataCatalogueSearchError::Initialization)?;
        let snapshot = state
            .as_ref()
            .ok_or(GameDataCatalogueSearchError::Unavailable)?;
        let index = snapshot
            .index
            .clone()
            .ok_or(GameDataCatalogueSearchError::Unavailable)?;
        let source_line_starts = snapshot.source_line_starts.clone();
        drop(state);
        search(
            &index,
            &source_line_starts,
            control,
            status
                .catalogue_revision
                .as_deref()
                .ok_or(GameDataCatalogueSearchError::Unavailable)?,
            request,
        )
        .map_err(GameDataCatalogueSearchError::Search)
    }

    pub fn search_text(
        &self,
        control: &IndexBuildControl,
        request: TextSearchRequest,
    ) -> Result<TextSearchPage, GameDataCatalogueTextSearchError> {
        self.before_operation(control)
            .map_err(GameDataCatalogueTextSearchError::Initialization)?;
        let status = self
            .status(control)
            .map_err(GameDataCatalogueTextSearchError::Initialization)?;
        if !status.available {
            return Err(GameDataCatalogueTextSearchError::Unavailable);
        }
        let revision = status
            .catalogue_revision
            .clone()
            .ok_or(GameDataCatalogueTextSearchError::Unavailable)?;
        let cache_key = (revision.clone(), request.query.clone(), request.options);
        if let Some(result_set) = self
            .text_search_cache
            .lock()
            .unwrap()
            .get(&cache_key)
            .cloned()
        {
            return page_text(&result_set, control, request)
                .map_err(GameDataCatalogueTextSearchError::TextSearch);
        }
        let state = self
            .lock_state(control)
            .map_err(GameDataCatalogueTextSearchError::Initialization)?;
        let snapshot = state
            .as_ref()
            .ok_or(GameDataCatalogueTextSearchError::Unavailable)?;
        let index = snapshot
            .index
            .clone()
            .ok_or(GameDataCatalogueTextSearchError::Unavailable)?;
        let cache_path = if index
            .files()
            .iter()
            .any(|file| file.metadata.virtual_source.is_some())
        {
            Some(
                resolve_current_index_pointer(
                    self.config
                        .cache_path
                        .as_ref()
                        .ok_or(GameDataCatalogueTextSearchError::Unavailable)?,
                )
                .map_err(GameDataCatalogueTextSearchError::Initialization)?,
            )
        } else {
            None
        };
        drop(state);

        let mut sources = Vec::new();
        let mut virtual_sources = Vec::new();
        let mut source_read_failures = 0;
        for file in index.files() {
            control.check().map_err(|_| {
                GameDataCatalogueTextSearchError::TextSearch(TextSearchError::Cancelled)
            })?;
            let Some(relative_path) = file
                .metadata
                .relative_path
                .as_ref()
                .map(|path| path.to_string_lossy().replace('\\', "/"))
            else {
                source_read_failures += 1;
                continue;
            };
            if let Some(virtual_source) = &file.metadata.virtual_source {
                virtual_sources.push((relative_path, virtual_source.uri.clone()));
                continue;
            }
            let source = if let Some(path) = &file.metadata.absolute_path {
                fs::read(path)
                    .ok()
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            } else {
                None
            };
            let Some(source) = source else {
                source_read_failures += 1;
                continue;
            };
            sources.push(TextSource {
                relative_path,
                content: Arc::<str>::from(source),
            });
        }
        if !virtual_sources.is_empty() {
            let cache_path = cache_path
                .as_ref()
                .ok_or(GameDataCatalogueTextSearchError::Unavailable)?;
            let uris = virtual_sources
                .iter()
                .map(|(_, uri)| uri.clone())
                .collect::<Vec<_>>();
            let batch =
                read_cached_virtual_sources(&uris, cache_path, control).map_err(|error| {
                    if error == INDEX_BUILD_CANCELLED {
                        GameDataCatalogueTextSearchError::TextSearch(TextSearchError::Cancelled)
                    } else {
                        GameDataCatalogueTextSearchError::Initialization(error)
                    }
                })?;
            for ((relative_path, _), source) in
                virtual_sources.into_iter().zip(batch.sources.into_iter())
            {
                match source {
                    Ok(source) => sources.push(TextSource {
                        relative_path,
                        content: Arc::<str>::from(source),
                    }),
                    Err(_) => source_read_failures += 1,
                }
            }
        }
        let result_set = scan_text(
            TextSearchCorpus {
                files_considered: index.files().len(),
                sources,
                source_read_failures,
            },
            control,
            &revision,
            &request,
        )
        .map_err(GameDataCatalogueTextSearchError::TextSearch)
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
        page_text(&result_set, control, request)
            .map_err(GameDataCatalogueTextSearchError::TextSearch)
    }

    pub fn inspect(
        &self,
        control: &IndexBuildControl,
        symbol_ref: String,
    ) -> Result<GameDataInspectionOutput, GameDataInspectionError> {
        self.before_operation(control)
            .map_err(GameDataInspectionError::Initialization)?;
        let status = self
            .status(control)
            .map_err(GameDataInspectionError::Initialization)?;
        let revision = status
            .catalogue_revision
            .as_deref()
            .ok_or(GameDataInspectionError::Unavailable)?;
        let state = self
            .lock_state(control)
            .map_err(GameDataInspectionError::Initialization)?;
        let snapshot = state.as_ref().ok_or(GameDataInspectionError::Unavailable)?;
        let index = snapshot
            .index
            .clone()
            .ok_or(GameDataInspectionError::Unavailable)?;
        let starts = snapshot.source_line_starts.clone();
        drop(state);
        inspect(&index, &starts, control, revision, &symbol_ref)
    }

    pub fn search_examples(
        &self,
        control: &IndexBuildControl,
        request: GameDataExampleSearchRequest,
    ) -> Result<GameDataExamplePage, GameDataCatalogueResearchError> {
        let _ = request;
        self.research_snapshot(control)?;
        Err(GameDataCatalogueResearchError::SourceEvidenceUnavailable)
    }

    pub fn list_members(
        &self,
        control: &IndexBuildControl,
        request: GameDataMemberRequest,
    ) -> Result<GameDataMemberPage, GameDataCatalogueResearchError> {
        let (revision, index, starts) = self.research_snapshot(control)?;
        list_members(&index, &starts, control, &revision, request)
            .map_err(GameDataCatalogueResearchError::Research)
    }

    pub fn query_relationships(
        &self,
        control: &IndexBuildControl,
        request: GameDataRelationshipRequest,
    ) -> Result<GameDataRelationshipPage, GameDataCatalogueResearchError> {
        let _ = request;
        self.research_snapshot(control)?;
        Err(GameDataCatalogueResearchError::SourceEvidenceUnavailable)
    }

    pub fn read_source(
        &self,
        control: &IndexBuildControl,
        request: GameDataSourceReadRequest,
    ) -> Result<serde_json::Value, GameDataInspectionError> {
        self.before_operation(control)
            .map_err(GameDataInspectionError::Initialization)?;
        let status = self
            .status(control)
            .map_err(GameDataInspectionError::Initialization)?;
        let revision = status
            .catalogue_revision
            .as_deref()
            .ok_or(GameDataInspectionError::Unavailable)?;
        let state = self
            .lock_state(control)
            .map_err(GameDataInspectionError::Initialization)?;
        let snapshot = state.as_ref().ok_or(GameDataInspectionError::Unavailable)?;
        let index = snapshot
            .index
            .clone()
            .ok_or(GameDataInspectionError::Unavailable)?;
        let (source_file_id, virtual_source, absolute_path) = index
            .files()
            .iter()
            .find(|file| {
                file.metadata.relative_path.as_ref().is_some_and(|path| {
                    path.to_string_lossy().replace('\\', "/") == request.relative_path
                })
            })
            .map(|file| {
                (
                    file.id,
                    file.metadata.virtual_source.clone(),
                    file.metadata.absolute_path.clone(),
                )
            })
            .ok_or_else(|| {
                GameDataInspectionError::InvalidSource(
                    "relativePath is not in the catalogue".to_string(),
                )
            })?;
        drop(state);
        let source = if let Some(virtual_source) = &virtual_source {
            let cache_path = resolve_current_index_pointer(
                self.config
                    .cache_path
                    .as_ref()
                    .ok_or(GameDataInspectionError::Unavailable)?,
            )
            .map_err(GameDataInspectionError::Initialization)?;
            read_cached_virtual_source(&virtual_source.uri, &cache_path).map_err(|error| {
                GameDataInspectionError::SourceReadFailed(format!(
                    "Failed to read Game Data source {}: {error}",
                    virtual_source.uri
                ))
            })?
        } else if let Some(path) = absolute_path {
            String::from_utf8_lossy(&fs::read(&path).map_err(|error| {
                GameDataInspectionError::SourceReadFailed(format!(
                    "Failed to read Game Data source {}: {error}",
                    path.display()
                ))
            })?)
            .into_owned()
        } else {
            return Err(GameDataInspectionError::SourceEvidenceUnavailable);
        };
        let mut source_texts = BTreeMap::new();
        source_texts.insert(source_file_id, Arc::<str>::from(source));
        read_source_evidence(&index, control, revision, &source_texts, request)
    }

    fn research_snapshot(
        &self,
        control: &IndexBuildControl,
    ) -> Result<
        (
            String,
            Arc<SymbolIndex>,
            Arc<BTreeMap<SourceFileId, SourceLineStarts>>,
        ),
        GameDataCatalogueResearchError,
    > {
        self.before_operation(control)
            .map_err(GameDataCatalogueResearchError::Initialization)?;
        let status = self
            .status(control)
            .map_err(GameDataCatalogueResearchError::Initialization)?;
        let revision = status
            .catalogue_revision
            .ok_or(GameDataCatalogueResearchError::Unavailable)?;
        let state = self
            .lock_state(control)
            .map_err(GameDataCatalogueResearchError::Initialization)?;
        let snapshot = state
            .as_ref()
            .ok_or(GameDataCatalogueResearchError::Unavailable)?;
        Ok((
            revision,
            snapshot
                .index
                .clone()
                .ok_or(GameDataCatalogueResearchError::Unavailable)?,
            snapshot.source_line_starts.clone(),
        ))
    }

    fn lock_state(
        &self,
        control: &IndexBuildControl,
    ) -> Result<MutexGuard<'_, Option<GameDataCatalogueState>>, String> {
        loop {
            control.check()?;
            match self.state.try_lock() {
                Ok(state) => return Ok(state),
                Err(TryLockError::Poisoned(error)) => return Ok(error.into_inner()),
                Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(2)),
            }
        }
    }

    #[cfg(all(feature = "test-hooks", debug_assertions))]
    fn before_initialization(&self, control: &IndexBuildControl) -> Result<(), String> {
        use std::io::Write;
        use std::sync::atomic::Ordering;

        if let Ok(marker) = std::env::var("REFORGER_MCP_TEST_INITIALIZATION_STARTED_MARKER") {
            if let Ok(mut file) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(marker)
            {
                let _ = writeln!(file, "started");
            }
        }
        let uninterruptible_delay_ms = std::env::var("REFORGER_MCP_TEST_UNINTERRUPTIBLE_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        std::thread::sleep(Duration::from_millis(uninterruptible_delay_ms));
        if self.panic_once.swap(false, Ordering::AcqRel) {
            panic!("intentional MCP initialization test panic");
        }
        let delay_ms = std::env::var("REFORGER_MCP_TEST_INITIALIZATION_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(delay_ms) {
            control.check()?;
            std::thread::sleep(Duration::from_millis(5));
        }
        control.check()
    }

    #[cfg(not(all(feature = "test-hooks", debug_assertions)))]
    fn before_initialization(&self, control: &IndexBuildControl) -> Result<(), String> {
        control.check()
    }

    #[cfg(all(feature = "test-hooks", debug_assertions))]
    fn before_operation(&self, control: &IndexBuildControl) -> Result<(), String> {
        let delay_ms = std::env::var("REFORGER_MCP_TEST_GAME_DATA_OPERATION_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(delay_ms) {
            control.check()?;
            std::thread::sleep(Duration::from_millis(5));
        }
        control.check()
    }

    #[cfg(not(all(feature = "test-hooks", debug_assertions)))]
    fn before_operation(&self, control: &IndexBuildControl) -> Result<(), String> {
        control.check()
    }
}

#[derive(Debug)]
pub enum GameDataCatalogueSearchError {
    Initialization(String),
    Unavailable,
    Search(GameDataSearchError),
}

#[derive(Debug)]
pub enum GameDataCatalogueTextSearchError {
    Initialization(String),
    Unavailable,
    TextSearch(TextSearchError),
}

#[derive(Debug)]
pub enum GameDataCatalogueResearchError {
    Initialization(String),
    Unavailable,
    SourceEvidenceUnavailable,
    Research(GameDataResearchError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameDataStatus {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalogue_revision: Option<String>,
    pub authorities: GameDataAuthorities,
    pub source: GameDataSourceStatus,
    pub coverage: GameDataCoverage,
    pub counts: GameDataCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<GameDataCacheStatus>,
    pub timings_ms: GameDataTimingsMs,
    pub limits: GameDataLimits,
    pub warnings: Vec<GameDataNotice>,
    pub recovery: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameDataAuthorities {
    pub source_evidence: FactAuthority,
    pub source_metadata: FactAuthority,
    pub semantic_catalogue: FactAuthority,
    pub cache: FactAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FactAuthority {
    Filesystem,
    LanguageEngine,
    EvidenceCatalogue,
    Workbench,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameDataSourceStatus {
    pub acquisition: GameDataAcquisition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloaded_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GameDataAcquisition {
    LocalPack,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameDataCoverage {
    pub files: usize,
    pub bytes: usize,
    pub indexed_symbols: usize,
    pub parse_diagnostics: usize,
    pub lossy_files: usize,
    pub lossless_files: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameDataCounts {
    pub symbols_by_kind: BTreeMap<String, usize>,
    pub files_by_source_category: BTreeMap<String, usize>,
    pub symbols_by_source_category: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameDataCacheStatus {
    pub outcome: GameDataCacheOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rebuild_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GameDataCacheOutcome {
    Loaded,
    Rebuilt,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameDataTimingsMs {
    pub cache_file_read: u64,
    pub cache_decode: u64,
    pub cache_validate: u64,
    pub map_rebuild: u64,
    pub rebuild: u64,
    pub cache_write: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameDataLimits {
    pub initialization_deadline_ms: u64,
    pub max_structured_result_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameDataNotice {
    pub code: String,
    pub message: String,
}

fn initialize_catalogue(
    config: &GameDataCatalogueConfig,
    control: &IndexBuildControl,
) -> Result<GameDataCatalogueState, String> {
    control.check()?;
    let started = Instant::now();
    let source = source_status(None);
    let Some(cache_path) = config.cache_path.clone() else {
        return Ok(unavailable_state(
            source,
            started.elapsed(),
            "game_data_index_not_configured",
            "The language-engine Game Data index location is not configured.",
            "Regenerate the MCP configuration from the extension.",
        ));
    };

    let cache_path = resolve_current_index_pointer(&cache_path)?;
    let result = load_game_data_index_cache_with_control(&cache_path, control);

    match result {
        Ok(Some(result)) => Ok(ready_state(result)),
        Ok(None) => Ok(unavailable_state(
            source,
            started.elapsed(),
            "game_data_index_unavailable",
            "The language-engine Game Data index is missing or incompatible.",
            "Activate the language server so it builds the Game Data index, then restart MCP.",
        )),
        Err(error) if error == INDEX_BUILD_CANCELLED => Err(error),
        Err(_) => Ok(unavailable_state(
            source,
            started.elapsed(),
            "game_data_initialization_failed",
            "The language-engine Game Data index could not be loaded.",
            "Activate the language server to rebuild the index, then restart MCP.",
        )),
    }
}

fn resolve_current_index_pointer(path: &Path) -> Result<PathBuf, String> {
    if path.extension().and_then(|value| value.to_str()) != Some("json") {
        return Ok(path.to_path_buf());
    }
    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read add-on index pointer {}: {error}",
            path.display()
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Invalid add-on index pointer {}: {error}", path.display()))?;
    if value.get("schema").and_then(serde_json::Value::as_str)
        != Some("reforger-addon-current-revision-v1")
    {
        return Err(format!(
            "Unsupported add-on index pointer {}",
            path.display()
        ));
    }
    let relative = value
        .get("index")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("Add-on index pointer has no index: {}", path.display()))?;
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!("Unsafe add-on index pointer: {}", path.display()));
    }
    Ok(path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(relative))
}

fn ready_state(result: GameDataIndexCacheResult) -> GameDataCatalogueState {
    let mut warnings = Vec::new();
    if result.summary.parse_diagnostics > 0 {
        warnings.push(GameDataNotice {
            code: "parse_diagnostics_present".to_string(),
            message: format!(
                "{} parser diagnostics were recorded while building the catalogue.",
                result.summary.parse_diagnostics
            ),
        });
    }
    if result.summary.lossy_files > 0 {
        warnings.push(GameDataNotice {
            code: "lossy_files_present".to_string(),
            message: format!(
                "{} source files required lossy UTF-8 decoding.",
                result.summary.lossy_files
            ),
        });
    }

    let source = source_status(Some(&result.fingerprint));
    let status = GameDataStatus {
        available: true,
        catalogue_revision: Some(format!("gd1:{}", result.catalogue_digest)),
        authorities: authorities(),
        source,
        coverage: coverage(&result.summary),
        counts: counts(&result.index),
        cache: Some(cache_status(&result)),
        timings_ms: timings_ms(result.timings),
        limits: limits(),
        warnings,
        recovery: vec![
            "Activate the language server to refresh the Game Data index, then restart MCP."
                .to_string(),
        ],
    };

    let source_line_starts = result
        .source_line_starts
        .into_iter()
        .map(|(file, starts)| (file, SourceLineStarts::from_cached_starts(starts)))
        .collect();
    GameDataCatalogueState {
        status,
        index: Some(Arc::new(result.index)),
        source_line_starts: Arc::new(source_line_starts),
    }
}

fn unavailable_state(
    source: GameDataSourceStatus,
    elapsed: Duration,
    code: &str,
    message: &str,
    recovery: &str,
) -> GameDataCatalogueState {
    GameDataCatalogueState {
        status: GameDataStatus {
            available: false,
            catalogue_revision: None,
            authorities: authorities(),
            source,
            coverage: GameDataCoverage::default(),
            counts: GameDataCounts::default(),
            cache: None,
            timings_ms: GameDataTimingsMs {
                total: duration_ms(elapsed),
                ..GameDataTimingsMs::default()
            },
            limits: limits(),
            warnings: vec![GameDataNotice {
                code: code.to_string(),
                message: message.to_string(),
            }],
            recovery: vec![recovery.to_string()],
        },
        index: None,
        source_line_starts: Arc::new(BTreeMap::new()),
    }
}

fn source_status(fingerprint: Option<&SourceFingerprint>) -> GameDataSourceStatus {
    let _ = fingerprint;

    GameDataSourceStatus {
        acquisition: GameDataAcquisition::LocalPack,
        branch: None,
        commit_sha: None,
        commit_date: None,
        downloaded_at: None,
    }
}

fn coverage(summary: &RuntimeIndexSummary) -> GameDataCoverage {
    GameDataCoverage {
        files: summary.files,
        bytes: summary.bytes,
        indexed_symbols: summary.indexed_symbols,
        parse_diagnostics: summary.parse_diagnostics,
        lossy_files: summary.lossy_files,
        lossless_files: summary.files.saturating_sub(summary.lossy_files),
    }
}

fn counts(index: &SymbolIndex) -> GameDataCounts {
    let mut symbols_by_kind = BTreeMap::new();
    let mut files_by_source_category = BTreeMap::new();
    let mut symbols_by_source_category = BTreeMap::new();

    for file in index.files() {
        *files_by_source_category
            .entry(file.metadata.category.as_str().to_string())
            .or_insert(0) += 1;
    }
    for symbol in index.symbols() {
        *symbols_by_kind
            .entry(symbol_kind_name(symbol.kind).to_string())
            .or_insert(0) += 1;
        if let Some(file) = index.file(symbol.id.file_id) {
            *symbols_by_source_category
                .entry(file.metadata.category.as_str().to_string())
                .or_insert(0) += 1;
        }
    }

    GameDataCounts {
        symbols_by_kind,
        files_by_source_category,
        symbols_by_source_category,
    }
}

fn symbol_kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "class",
        SymbolKind::TypeParameter => "typeParameter",
        SymbolKind::Enum => "enum",
        SymbolKind::EnumMember => "enumMember",
        SymbolKind::Typedef => "typedef",
        SymbolKind::Function => "function",
        SymbolKind::GlobalField => "globalField",
        SymbolKind::Field => "field",
        SymbolKind::Method => "method",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Destructor => "destructor",
        SymbolKind::Parameter => "parameter",
        SymbolKind::LocalVariable => "localVariable",
        SymbolKind::PreprocessorMacro => "preprocessorMacro",
    }
}

fn cache_status(result: &GameDataIndexCacheResult) -> GameDataCacheStatus {
    match &result.cache_status {
        IndexCacheStatus::Loaded => GameDataCacheStatus {
            outcome: GameDataCacheOutcome::Loaded,
            rebuild_reason: None,
            file_bytes: result.cache_file_bytes,
        },
        IndexCacheStatus::Rebuilt { .. } => GameDataCacheStatus {
            outcome: GameDataCacheOutcome::Rebuilt,
            rebuild_reason: Some("cache_miss_or_invalid".to_string()),
            file_bytes: result.cache_file_bytes,
        },
    }
}

fn timings_ms(timings: IndexCacheTimings) -> GameDataTimingsMs {
    GameDataTimingsMs {
        cache_file_read: duration_ms(timings.cache_file_read),
        cache_decode: duration_ms(timings.cache_decode),
        cache_validate: duration_ms(timings.cache_validate),
        map_rebuild: duration_ms(timings.map_rebuild),
        rebuild: duration_ms(timings.rebuild),
        cache_write: duration_ms(timings.cache_write),
        total: duration_ms(timings.total),
    }
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn limits() -> GameDataLimits {
    GameDataLimits {
        initialization_deadline_ms: GAME_DATA_INITIALIZATION_DEADLINE_MS,
        max_structured_result_bytes: MAX_STRUCTURED_RESULT_BYTES,
    }
}

fn authorities() -> GameDataAuthorities {
    GameDataAuthorities {
        source_evidence: FactAuthority::EvidenceCatalogue,
        source_metadata: FactAuthority::Filesystem,
        semantic_catalogue: FactAuthority::LanguageEngine,
        cache: FactAuthority::Filesystem,
    }
}
