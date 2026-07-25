use crate::game_data_search::{
    search, GameDataSearchError, GameDataSearchPage, GameDataSearchRequest, SourceLineStarts,
};
use crate::index::{SourceFileId, SymbolIndex};
use crate::index_build::{IndexBuildControl, INDEX_BUILD_CANCELLED};
use crate::index_cache::{
    load_or_build_game_data_index_with_control, GameDataIndexCacheConfig, GameDataIndexCacheResult,
    IndexCacheStatus, IndexCacheTimings, RuntimeIndexSummary, SourceFingerprint,
};
use crate::model::SymbolKind;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

pub const GAME_DATA_INITIALIZATION_DEADLINE_MS: u64 = 120_000;
pub const MAX_STRUCTURED_RESULT_BYTES: usize = 256 * 1024;
const MAX_METADATA_TEXT_CHARS: usize = 512;

#[derive(Debug, Clone)]
pub struct GameDataCatalogueConfig {
    pub scripts_root: Option<PathBuf>,
    pub metadata_path: Option<PathBuf>,
    pub cache_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct GameDataCatalogue {
    config: GameDataCatalogueConfig,
    state: Mutex<Option<GameDataCatalogueState>>,
    #[cfg(debug_assertions)]
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
            #[cfg(debug_assertions)]
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
        Ok(status)
    }

    pub fn search(
        &self,
        control: &IndexBuildControl,
        request: GameDataSearchRequest,
    ) -> Result<GameDataSearchPage, GameDataCatalogueSearchError> {
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

    fn lock_state(&self, control: &IndexBuildControl) -> Result<MutexGuard<'_, Option<GameDataCatalogueState>>, String> {
        loop {
            control.check()?;
            match self.state.try_lock() {
                Ok(state) => return Ok(state),
                Err(TryLockError::Poisoned(error)) => return Ok(error.into_inner()),
                Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(2)),
            }
        }
    }

    #[cfg(debug_assertions)]
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

    #[cfg(not(debug_assertions))]
    fn before_initialization(&self, control: &IndexBuildControl) -> Result<(), String> {
        control.check()
    }
}

#[derive(Debug)]
pub enum GameDataCatalogueSearchError {
    Initialization(String),
    Unavailable,
    Search(GameDataSearchError),
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
    Downloaded,
    Manual,
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
    pub fingerprint: u64,
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
    let source = source_status(config, None);
    let Some(scripts_root) = config.scripts_root.clone() else {
        return Ok(unavailable_state(
            source,
            started.elapsed(),
            "game_data_not_configured",
            "Game Data is not configured for this MCP process.",
            "Regenerate the MCP configuration from the extension after configuring Game Data.",
        ));
    };
    let Some(cache_path) = config.cache_path.clone() else {
        return Ok(unavailable_state(
            source,
            started.elapsed(),
            "cache_not_configured",
            "The validated Game Data cache location is not configured.",
            "Regenerate the MCP configuration from the extension.",
        ));
    };

    let result = load_or_build_game_data_index_with_control(
        &GameDataIndexCacheConfig {
            scripts_root,
            metadata_path: config.metadata_path.clone(),
            cache_path,
        },
        control,
    );

    match result {
        Ok(result) => Ok(ready_state(config, result)),
        Err(error) if error == INDEX_BUILD_CANCELLED => Err(error),
        Err(_) => Ok(unavailable_state(
            source,
            started.elapsed(),
            "game_data_initialization_failed",
            "Game Data could not be validated, loaded, or rebuilt.",
            "Verify the configured Game Data source, then restart the MCP process.",
        )),
    }
}

fn ready_state(
    config: &GameDataCatalogueConfig,
    result: GameDataIndexCacheResult,
) -> GameDataCatalogueState {
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

    let source = source_status(config, Some(&result.fingerprint));
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
        recovery: vec!["Restart the MCP process after changing or updating Game Data.".to_string()],
    };

    let mut source_line_starts = BTreeMap::new();
    for file in result.index.files() {
        let Some(path) = file.metadata.absolute_path.as_ref() else {
            return unavailable_state(
                source_status(config, Some(&result.fingerprint)),
                Duration::ZERO,
                "game_data_source_snapshot_failed",
                "Game Data source lines could not be captured for the catalogue snapshot.",
                "Verify the configured Game Data source, then restart the MCP process.",
            );
        };
        let Ok(source) = fs::read_to_string(path) else {
            return unavailable_state(
                source_status(config, Some(&result.fingerprint)),
                Duration::ZERO,
                "game_data_source_snapshot_failed",
                "Game Data source lines could not be captured for the catalogue snapshot.",
                "Verify the configured Game Data source, then restart the MCP process.",
            );
        };
        source_line_starts.insert(file.id, SourceLineStarts::from_source(&source));
    }
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

fn source_status(
    config: &GameDataCatalogueConfig,
    fingerprint: Option<&SourceFingerprint>,
) -> GameDataSourceStatus {
    let acquisition = match fingerprint {
        Some(SourceFingerprint::Downloaded { .. }) => GameDataAcquisition::Downloaded,
        Some(SourceFingerprint::Manual { .. }) => GameDataAcquisition::Manual,
        None if config.metadata_path.is_some() => GameDataAcquisition::Downloaded,
        None => GameDataAcquisition::Manual,
    };
    let metadata = config
        .metadata_path
        .as_deref()
        .and_then(read_download_metadata);
    let fingerprint_commit = match fingerprint {
        Some(SourceFingerprint::Downloaded { commit_sha, .. }) => Some(commit_sha.clone()),
        _ => None,
    };

    GameDataSourceStatus {
        acquisition,
        branch: metadata
            .as_ref()
            .and_then(|value| metadata_text(value, "branch")),
        commit_sha: fingerprint_commit.or_else(|| {
            metadata
                .as_ref()
                .and_then(|value| metadata_text(value, "commitSha"))
        }),
        commit_date: metadata
            .as_ref()
            .and_then(|value| metadata_text(value, "commitDate")),
        downloaded_at: metadata
            .as_ref()
            .and_then(|value| metadata_text(value, "downloadedAt")),
    }
}

fn read_download_metadata(path: &std::path::Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn metadata_text(metadata: &Value, field: &str) -> Option<String> {
    let value = metadata.get(field)?.as_str()?.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.chars().take(MAX_METADATA_TEXT_CHARS).collect())
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
        fingerprint: duration_ms(timings.fingerprint),
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
