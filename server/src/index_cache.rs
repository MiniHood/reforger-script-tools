use crate::ast::DocCommentKind;
use crate::index::{
    indexed_callable_form, indexed_conditional_kind, indexed_symbol_kind, semantic_attribute_name,
    GlobalSymbolId, IndexedAttribute, IndexedConditionalBranch, IndexedDocComment, IndexedFile,
    IndexedSymbol, IndexedSymbolDetail, SourceFileId, SymbolIndex,
};
use crate::index_build::{
    build_index_with_control, IndexBuildConfig, IndexBuildControl, IndexBuildResult,
    IndexBuildShape, IndexBuildSummary, IndexBuildTimings, IndexSourceRoot,
};
use crate::lexer::TextSpan;
use crate::model::{
    CallableForm, PreprocessorBranchKind, SourceCategory, SourceFileMetadata, SourceKind, SymbolId,
    SymbolKind, VirtualSourceIdentity, SOURCE_PRIORITY_GAME_DATA,
};
#[cfg(test)]
use crate::semantic_file::SemanticDeclarationId;
use crate::semantic_file::{
    SemanticCallableForm, SemanticConditionalBranchKind, SemanticDeclarationKind,
    SemanticDocCommentKind,
};
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

const CACHE_FORMAT_VERSION: u32 = 16;
const CACHE_SCHEMA: &str = "reforger-symbol-index";
const CACHE_MAGIC: &[u8; 8] = b"RSTIDX16";
const CACHE_CONTAINER_MAGIC: &[u8; 8] = b"RSTCNT17";
const CACHE_CONTAINER_VERSION: u32 = 1;
const CACHE_CONTAINER_HEADER_BYTES: u64 = 8 + 4 + (8 * 4);
const MAX_CACHE_SECTION_BYTES: u64 = 512 * 1024 * 1024;
const CACHE_INDEX_SHAPE: &str = "runtime-pruned:no-local-variables:detail-spans-stripped:layered-external-v1:binary-v9:narrow-u32-integers-v1:packed-symbol-flags-v1:string-table-v1:canonical-public-facts-v1:parser-source-line-map-v1:delta-line-map-v1:source-content-digest-v1:addon-fingerprint-v1:typed-virtual-source-v1:source-category-v2";
const LEGACY_CACHE_MAGIC: &[u8; 8] = b"RSTIDX09";
const V10_CACHE_MAGIC: &[u8; 8] = b"RSTIDX10";
const MAX_CACHE_STRING_TABLE_ENTRIES: usize = 1_000_000;
const MAX_CACHE_RAW_STRING_BYTES: usize = 16 * 1024 * 1024;
const MAX_CACHE_FILE_RECORDS: usize = 1_000_000;
const MAX_CACHE_SYMBOL_RECORDS: usize = 5_000_000;
const MAX_CACHE_SYMBOL_LIST_ITEMS: usize = 1_000_000;
const CACHED_SYMBOL_HAS_PARENT: u16 = 1 << 0;
const CACHED_SYMBOL_HAS_TYPE: u16 = 1 << 1;
const CACHED_SYMBOL_HAS_RETURN_TYPE: u16 = 1 << 2;
const CACHED_SYMBOL_HAS_BASE_TYPE: u16 = 1 << 3;
const CACHED_SYMBOL_HAS_DEFAULT_VALUE: u16 = 1 << 4;
const CACHED_SYMBOL_HAS_ENUM_VALUE: u16 = 1 << 5;
const CACHED_SYMBOL_HAS_ATTRIBUTES: u16 = 1 << 6;
const CACHED_SYMBOL_HAS_MODIFIERS: u16 = 1 << 7;
const CACHED_SYMBOL_HAS_DOC_COMMENTS: u16 = 1 << 8;
const CACHED_SYMBOL_HAS_CONDITIONAL_CONTEXT: u16 = 1 << 9;
const CACHED_SYMBOL_HAS_CALLABLE_FORM: u16 = 1 << 10;
const CACHED_SYMBOL_KNOWN_FLAGS: u16 = (1 << 11) - 1;

#[derive(Debug)]
pub struct GameDataIndexCacheConfig {
    pub scripts_root: PathBuf,
    pub cache_path: PathBuf,
    pub metadata_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct GameDataIndexCacheResult {
    pub index: SymbolIndex,
    pub source_line_starts: BTreeMap<SourceFileId, Vec<usize>>,
    pub summary: RuntimeIndexSummary,
    pub cache_status: IndexCacheStatus,
    pub fingerprint: SourceFingerprint,
    pub source_digest: String,
    pub catalogue_digest: String,
    pub timings: IndexCacheTimings,
    pub cache_file_bytes: Option<u64>,
}

/// Reuses the canonical compact semantic-cache format for one archive-backed
/// add-on. The caller owns archive identity and source acquisition; this
/// function never walks a source directory.
pub fn load_or_build_archive_index(
    cache_path: &Path,
    fingerprint: SourceFingerprint,
    source_digest: String,
    build: impl FnOnce() -> Result<IndexBuildResult, String>,
) -> Result<GameDataIndexCacheResult, String> {
    load_or_build_archive_index_with_reuse(
        cache_path,
        fingerprint,
        source_digest,
        true,
        "cache-missing-invalid-or-source-changed",
        build,
    )
}

pub(crate) fn load_or_build_archive_index_with_reuse(
    cache_path: &Path,
    fingerprint: SourceFingerprint,
    source_digest: String,
    allow_reuse: bool,
    rebuild_reason: &str,
    build: impl FnOnce() -> Result<IndexBuildResult, String>,
) -> Result<GameDataIndexCacheResult, String> {
    load_or_build_archive_index_with_reuse_and_locator(
        cache_path,
        fingerprint,
        source_digest,
        allow_reuse,
        rebuild_reason,
        || Ok(None),
        build,
    )
}

pub(crate) fn load_or_build_archive_index_with_reuse_and_locator(
    cache_path: &Path,
    fingerprint: SourceFingerprint,
    source_digest: String,
    allow_reuse: bool,
    rebuild_reason: &str,
    locator_builder: impl FnOnce() -> Result<Option<Vec<u8>>, String>,
    build: impl FnOnce() -> Result<IndexBuildResult, String>,
) -> Result<GameDataIndexCacheResult, String> {
    let total_start = Instant::now();
    let mut timings = IndexCacheTimings::default();
    let load_start = Instant::now();
    if allow_reuse {
        if let Ok(Some(CacheLoad::Current(cached))) =
            load_cached_index(cache_path, &fingerprint, &source_digest, &mut timings)
        {
            timings.cache_read_deserialize_validate = load_start.elapsed();
            let summary: RuntimeIndexSummary = cached.summary.clone().into();
            let map_start = Instant::now();
            let (index, source_line_starts, projection, lookup_maps) =
                cached.into_index_and_line_starts();
            timings.map_projection = projection;
            timings.map_lookup_rebuild = lookup_maps;
            timings.map_rebuild = map_start.elapsed();
            timings.total = total_start.elapsed();
            return Ok(GameDataIndexCacheResult {
                index,
                source_line_starts,
                summary,
                cache_status: IndexCacheStatus::Loaded,
                fingerprint,
                source_digest: source_digest.clone(),
                catalogue_digest: source_digest,
                timings,
                cache_file_bytes: cache_file_bytes(cache_path),
            });
        }
    }
    timings.cache_read_deserialize_validate = load_start.elapsed();
    let build_start = Instant::now();
    let built = build()?;
    timings.rebuild = build_start.elapsed();
    timings.source_build = built.summary.timings;
    let cache_prepare_start = Instant::now();
    let cache_compact_start = Instant::now();
    let cached_index = match built.index_shape {
        IndexBuildShape::Full => built.index.into_runtime_cache()?,
        IndexBuildShape::RuntimeCache => built.index,
    };
    timings.cache_compact = cache_compact_start.elapsed();
    let cache_payload_prepare_start = Instant::now();
    let summary = summary_from_build_with_cached_index(&built.summary, &cached_index);
    let cached_summary = CachedIndexSummary::from(&summary);
    timings.cache_payload_prepare = cache_payload_prepare_start.elapsed();
    timings.cache_prepare = cache_prepare_start.elapsed();
    let locator_payload = locator_builder()?;
    let write_start = Instant::now();
    let write_timings = write_runtime_cache_payload(
        cache_path,
        &cached_index,
        &built.source_line_starts,
        &fingerprint,
        &source_digest,
        &cached_summary,
        locator_payload.as_deref(),
    )?;
    timings.cache_encode = write_timings.encode;
    timings.cache_atomic_write = write_timings.atomic_write;
    timings.cache_write = write_start.elapsed();
    timings.total = total_start.elapsed();
    Ok(GameDataIndexCacheResult {
        index: cached_index,
        source_line_starts: built
            .source_line_starts
            .into_iter()
            .enumerate()
            .map(|(index, starts)| (SourceFileId(index), starts))
            .collect(),
        summary,
        cache_status: IndexCacheStatus::Rebuilt {
            reason: rebuild_reason.to_string(),
        },
        fingerprint,
        source_digest: source_digest.clone(),
        catalogue_digest: source_digest,
        timings,
        cache_file_bytes: cache_file_bytes(cache_path),
    })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexCacheTimings {
    pub fingerprint: Duration,
    /// Time spent reading and decoding the caller-owned cache metadata that
    /// surrounds the binary index payload.
    pub cache_metadata_read: Duration,
    pub cache_file_read: Duration,
    pub cache_decode: Duration,
    pub cache_validate: Duration,
    pub map_rebuild: Duration,
    pub map_projection: Duration,
    pub map_lookup_rebuild: Duration,
    pub cache_read_deserialize_validate: Duration,
    pub rebuild: Duration,
    /// Cold-build stages, retained independently of the enclosing rebuild
    /// duration so diagnostics can distinguish source acquisition, parsing,
    /// semantic modelling, and final aggregation.
    pub source_build: IndexBuildTimings,
    /// Projection of the completed source index into the compact runtime and
    /// serialized cache representations before cache encoding begins.
    pub cache_prepare: Duration,
    pub cache_compact: Duration,
    pub cache_payload_prepare: Duration,
    pub cache_write: Duration,
    pub cache_encode: Duration,
    pub cache_atomic_write: Duration,
    /// Publication of cache metadata owned by the caller, including immutable
    /// manifests and current-revision pointers.
    pub cache_metadata_publish: Duration,
    pub total: Duration,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeIndexSummary {
    pub files: usize,
    pub bytes: usize,
    pub indexed_symbols: usize,
    pub parse_diagnostics: usize,
    pub lossy_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexCacheStatus {
    Loaded,
    Rebuilt { reason: String },
}

impl IndexCacheStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Loaded => "loaded",
            Self::Rebuilt { .. } => "rebuilt",
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Loaded => None,
            Self::Rebuilt { reason } => Some(reason.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SourceFingerprint {
    Downloaded {
        scripts_root: String,
        commit_sha: String,
    },
    Manual {
        scripts_root: String,
        file_count: usize,
        byte_count: u64,
        latest_modified_unix_ms: u128,
    },
    Addon {
        guid: String,
        artifact_digest: String,
        pack_count: usize,
        catalogue_entry_count: usize,
    },
}

#[cfg(test)]
#[derive(Debug, Clone)]
struct CachedGameDataIndex {
    schema: String,
    format_version: u32,
    index_shape: String,
    crate_version: String,
    fingerprint: SourceFingerprint,
    source_digest: String,
    summary: CachedIndexSummary,
    files: Vec<CachedFileContribution>,
}

#[derive(Debug)]
struct DecodedRuntimeCache {
    schema: String,
    format_version: u32,
    index_shape: String,
    crate_version: String,
    fingerprint: SourceFingerprint,
    source_digest: String,
    summary: CachedIndexSummary,
    files: Vec<IndexedFile>,
    symbols: Vec<IndexedSymbol>,
    source_line_starts: BTreeMap<SourceFileId, Vec<usize>>,
}

/// The cache's canonical payload.  This intentionally is neither a
/// `FileContribution` (which carries source-only spans/container text) nor a
/// serialized `SymbolIndex` (which duplicates all derived runtime records).
/// It contains exactly the source metadata and public facts needed to rebuild
/// a runtime contribution and its lookup maps.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedFileContribution {
    metadata: SourceFileMetadata,
    non_declaration_callable_fragments: usize,
    source_line_starts: Vec<usize>,
    symbols: Vec<CachedPublicSymbol>,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedPublicSymbol {
    id: SemanticDeclarationId,
    parent: Option<SemanticDeclarationId>,
    kind: SemanticDeclarationKind,
    name: String,
    span: TextSpan,
    selection_span: TextSpan,
    detail: CachedPublicSymbolDetail,
    attributes: Vec<String>,
    modifiers: Vec<String>,
    doc_comments: Vec<CachedDocComment>,
    conditional_context: Vec<CachedConditionalBranch>,
    callable_form: Option<SemanticCallableForm>,
}

#[cfg(test)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CachedPublicSymbolDetail {
    type_text: Option<String>,
    return_type: Option<String>,
    base_type: Option<String>,
    default_value: Option<String>,
    enum_value: Option<String>,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedDocComment {
    kind: SemanticDocCommentKind,
    text: String,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedConditionalBranch {
    kind: SemanticConditionalBranchKind,
    condition: Option<String>,
}

#[cfg(test)]
impl CachedFileContribution {
    fn validate(&self) -> Result<(), String> {
        for (expected, symbol) in self.symbols.iter().enumerate() {
            let expected = SemanticDeclarationId(expected as u32);
            if symbol.name.is_empty() {
                return Err(format!(
                    "invalid cached public file contribution: symbol {:?} has an empty name",
                    symbol.id
                ));
            }
            if symbol.id != expected {
                return Err(format!(
                    "invalid cached public file contribution: expected dense id {:?}, found {:?}",
                    expected, symbol.id
                ));
            }
            if let Some(parent) = symbol.parent {
                if parent.0 as usize >= self.symbols.len() {
                    return Err(format!(
                        "invalid cached public file contribution: symbol {:?} has missing parent {:?}",
                        symbol.id, parent
                    ));
                }
            }
        }
        Ok(())
    }
}
#[cfg(test)]
impl CachedGameDataIndex {
    fn validate(&self) -> Result<(), String> {
        self.files
            .iter()
            .try_for_each(CachedFileContribution::validate)
    }
}

impl DecodedRuntimeCache {
    fn into_index_and_line_starts(
        self,
    ) -> (
        SymbolIndex,
        BTreeMap<SourceFileId, Vec<usize>>,
        Duration,
        Duration,
    ) {
        let (index, lookup_maps) =
            SymbolIndex::from_indexed_parts_with_map_timing(self.files, self.symbols);
        (index, self.source_line_starts, Duration::ZERO, lookup_maps)
    }
}

fn public_semantic_kind(kind: SymbolKind) -> Option<SemanticDeclarationKind> {
    Some(match kind {
        SymbolKind::Class => SemanticDeclarationKind::Class,
        SymbolKind::Enum => SemanticDeclarationKind::Enum,
        SymbolKind::EnumMember => SemanticDeclarationKind::EnumMember,
        SymbolKind::Typedef => SemanticDeclarationKind::Typedef,
        SymbolKind::Function => SemanticDeclarationKind::Function,
        SymbolKind::GlobalField => SemanticDeclarationKind::GlobalField,
        SymbolKind::Field => SemanticDeclarationKind::Field,
        SymbolKind::Method => SemanticDeclarationKind::Method,
        SymbolKind::Constructor => SemanticDeclarationKind::Constructor,
        SymbolKind::Destructor => SemanticDeclarationKind::Destructor,
        SymbolKind::PreprocessorMacro => SemanticDeclarationKind::PreprocessorMacro,
        SymbolKind::TypeParameter => SemanticDeclarationKind::TypeParameter,
        SymbolKind::Parameter => SemanticDeclarationKind::Parameter,
        SymbolKind::LocalVariable => return None,
    })
}

#[derive(Debug, Clone)]
struct CachedIndexSummary {
    files: usize,
    bytes: usize,
    indexed_symbols: usize,
    parse_diagnostics: usize,
    lossy_files: usize,
}

impl From<&RuntimeIndexSummary> for CachedIndexSummary {
    fn from(summary: &RuntimeIndexSummary) -> Self {
        Self {
            files: summary.files,
            bytes: summary.bytes,
            indexed_symbols: summary.indexed_symbols,
            parse_diagnostics: summary.parse_diagnostics,
            lossy_files: summary.lossy_files,
        }
    }
}

impl From<CachedIndexSummary> for RuntimeIndexSummary {
    fn from(summary: CachedIndexSummary) -> Self {
        Self {
            files: summary.files,
            bytes: summary.bytes,
            indexed_symbols: summary.indexed_symbols,
            parse_diagnostics: summary.parse_diagnostics,
            lossy_files: summary.lossy_files,
        }
    }
}

pub fn load_or_build_game_data_index(
    config: &GameDataIndexCacheConfig,
) -> Result<GameDataIndexCacheResult, String> {
    load_or_build_game_data_index_with_control(config, &IndexBuildControl::default())
}

pub fn load_or_build_game_data_index_with_control(
    config: &GameDataIndexCacheConfig,
    control: &IndexBuildControl,
) -> Result<GameDataIndexCacheResult, String> {
    load_or_build_game_data_index_with_progress_and_control(config, |_| {}, control)
}

/// Loads the parser-owned Game Data cache without inspecting its source tree.
///
/// Consumers of the index produced by the language-engine indexer must not
/// repeat source fingerprinting or decide when that index is rebuilt. A
/// missing, incompatible, or malformed cache is therefore unavailable rather
/// than a reason to parse or write Game Data here. The returned self-described
/// fingerprint lets each consumer enforce its own instance identity.
pub fn load_game_data_index_cache_with_control(
    cache_path: &Path,
    control: &IndexBuildControl,
) -> Result<Option<GameDataIndexCacheResult>, String> {
    let total_start = Instant::now();
    let mut timings = IndexCacheTimings::default();
    control.check()?;
    if !cache_path.is_file() {
        return Ok(None);
    }

    let cache_file_bytes = cache_file_bytes(cache_path);
    let read_start = Instant::now();
    let bytes = read_index_cache_payload(cache_path)?;
    timings.cache_file_read = read_start.elapsed();
    if !bytes.starts_with(CACHE_MAGIC) {
        return Ok(None);
    }

    let decode_start = Instant::now();
    let cached = decode_runtime_cache(&bytes).map_err(|error| {
        format!(
            "Failed to decode index cache {}: {error}",
            cache_path.display()
        )
    })?;
    timings.cache_decode = decode_start.elapsed();

    let validate_start = Instant::now();
    if cached.schema != CACHE_SCHEMA
        || cached.format_version != CACHE_FORMAT_VERSION
        || cached.index_shape != CACHE_INDEX_SHAPE
        || cached.crate_version != env!("CARGO_PKG_VERSION")
    {
        return Ok(None);
    }
    timings.cache_validate = validate_start.elapsed();
    control.check()?;

    let summary: RuntimeIndexSummary = cached.summary.clone().into();
    let fingerprint = cached.fingerprint.clone();
    let source_digest = cached.source_digest.clone();
    let catalogue_digest = source_digest.clone();
    let map_rebuild_start = Instant::now();
    let (index, source_line_starts, projection, lookup_maps) = cached.into_index_and_line_starts();
    timings.map_projection = projection;
    timings.map_lookup_rebuild = lookup_maps;
    timings.map_rebuild = map_rebuild_start.elapsed();
    timings.total = total_start.elapsed();

    Ok(Some(GameDataIndexCacheResult {
        index,
        source_line_starts,
        summary,
        cache_status: IndexCacheStatus::Loaded,
        fingerprint,
        source_digest,
        catalogue_digest,
        timings,
        cache_file_bytes,
    }))
}

pub fn load_or_build_game_data_index_with_progress(
    config: &GameDataIndexCacheConfig,
    progress: impl FnMut(&str),
) -> Result<GameDataIndexCacheResult, String> {
    load_or_build_game_data_index_with_progress_and_control(
        config,
        progress,
        &IndexBuildControl::default(),
    )
}

fn load_or_build_game_data_index_with_progress_and_control(
    config: &GameDataIndexCacheConfig,
    mut progress: impl FnMut(&str),
    control: &IndexBuildControl,
) -> Result<GameDataIndexCacheResult, String> {
    let total_start = Instant::now();
    let mut timings = IndexCacheTimings::default();
    control.check()?;
    progress("validate-scripts-root-start");
    if !config.scripts_root.is_dir() {
        return Err(format!(
            "Game-data scripts folder does not exist: {}",
            config.scripts_root.display()
        ));
    }
    progress("validate-scripts-root-end");

    control.check()?;
    progress("fingerprint-start");
    let fingerprint_start = Instant::now();
    let fingerprint = source_fingerprint_with_control(
        &config.scripts_root,
        config.metadata_path.as_deref(),
        control,
    )?;
    let source_digest = source_content_digest(&config.scripts_root, control)?;
    timings.fingerprint = fingerprint_start.elapsed();
    progress("fingerprint-end");
    let initial_cache_file_bytes = cache_file_bytes(&config.cache_path);

    control.check()?;
    progress("cache-load-start");
    let cache_read_start = Instant::now();
    match load_cached_index(
        &config.cache_path,
        &fingerprint,
        &source_digest,
        &mut timings,
    ) {
        Ok(Some(CacheLoad::Current(cached))) => {
            control.check()?;
            timings.cache_read_deserialize_validate = cache_read_start.elapsed();
            progress("cache-load-hit");
            progress("map-rebuild-start");
            let map_rebuild_start = Instant::now();
            let summary: RuntimeIndexSummary = cached.summary.clone().into();
            let catalogue_digest = source_digest.clone();
            let (index, source_line_starts, projection, lookup_maps) =
                cached.into_index_and_line_starts();
            timings.map_projection = projection;
            timings.map_lookup_rebuild = lookup_maps;
            control.check()?;
            timings.map_rebuild = map_rebuild_start.elapsed();
            progress("map-rebuild-end");
            timings.total = total_start.elapsed();
            return Ok(GameDataIndexCacheResult {
                index,
                source_line_starts,
                summary,
                cache_status: IndexCacheStatus::Loaded,
                fingerprint,
                source_digest,
                catalogue_digest,
                timings,
                cache_file_bytes: initial_cache_file_bytes,
            });
        }
        Ok(None) | Err(_) => {
            timings.cache_read_deserialize_validate = cache_read_start.elapsed();
            progress("cache-load-miss");
        }
    }

    control.check()?;
    let rebuild_reason = cache_rebuild_reason(&config.cache_path, &fingerprint);
    progress("source-rebuild-start");
    let rebuild_start = Instant::now();
    let built = build_index_with_control(
        &IndexBuildConfig {
            roots: vec![IndexSourceRoot::new(
                &config.scripts_root,
                SourceKind::GameData,
                SOURCE_PRIORITY_GAME_DATA,
            )],
        },
        control,
    )?;
    timings.rebuild = rebuild_start.elapsed();
    progress("source-rebuild-end");
    let cache_prepare_start = Instant::now();
    let cache_compact_start = Instant::now();
    let cached_index = match built.index_shape {
        IndexBuildShape::Full => built.index.into_runtime_cache()?,
        IndexBuildShape::RuntimeCache => built.index,
    };
    timings.cache_compact = cache_compact_start.elapsed();
    let cache_payload_prepare_start = Instant::now();
    let summary = summary_from_build_with_cached_index(&built.summary, &cached_index);
    let cached_summary = CachedIndexSummary::from(&summary);
    timings.cache_payload_prepare = cache_payload_prepare_start.elapsed();
    timings.cache_prepare = cache_prepare_start.elapsed();
    let catalogue_digest = source_digest.clone();

    control.check()?;
    progress("cache-write-start");
    let cache_write_start = Instant::now();
    match write_runtime_cache_payload(
        &config.cache_path,
        &cached_index,
        &built.source_line_starts,
        &fingerprint,
        &source_digest,
        &cached_summary,
        None,
    ) {
        Ok(write_timings) => {
            timings.cache_encode = write_timings.encode;
            timings.cache_atomic_write = write_timings.atomic_write;
        }
        Err(write_error) => {
            progress("cache-write-contended");
            if !winner_cache_validates(
                &config.cache_path,
                &fingerprint,
                &source_digest,
                &mut timings,
            ) {
                return Err(write_error);
            }
        }
    }
    timings.cache_write = cache_write_start.elapsed();
    control.check()?;
    progress("cache-write-end");
    timings.total = total_start.elapsed();
    let cache_file_bytes = cache_file_bytes(&config.cache_path);

    Ok(GameDataIndexCacheResult {
        index: cached_index,
        source_line_starts: built
            .source_line_starts
            .into_iter()
            .enumerate()
            .map(|(index, starts)| (SourceFileId(index), starts))
            .collect(),
        summary,
        cache_status: IndexCacheStatus::Rebuilt {
            reason: rebuild_reason,
        },
        fingerprint,
        source_digest,
        catalogue_digest,
        timings,
        cache_file_bytes,
    })
}

fn winner_cache_validates(
    cache_path: &Path,
    fingerprint: &SourceFingerprint,
    source_digest: &str,
    timings: &mut IndexCacheTimings,
) -> bool {
    const VALIDATION_ATTEMPTS: usize = 8;
    const VALIDATION_RETRY_DELAY: Duration = Duration::from_millis(5);

    for attempt in 0..VALIDATION_ATTEMPTS {
        let mut winner_timings = IndexCacheTimings::default();
        if matches!(
            load_cached_index(cache_path, fingerprint, source_digest, &mut winner_timings,),
            Ok(Some(CacheLoad::Current(_)))
        ) {
            timings.cache_file_read += winner_timings.cache_file_read;
            timings.cache_decode += winner_timings.cache_decode;
            timings.cache_validate += winner_timings.cache_validate;
            return true;
        }
        if attempt + 1 < VALIDATION_ATTEMPTS {
            std::thread::sleep(VALIDATION_RETRY_DELAY);
        }
    }
    false
}

fn cache_file_bytes(cache_path: &Path) -> Option<u64> {
    cache_path.metadata().ok().map(|metadata| metadata.len())
}

enum CacheLoad {
    Current(DecodedRuntimeCache),
}

fn load_cached_index(
    cache_path: &Path,
    expected_fingerprint: &SourceFingerprint,
    expected_source_digest: &str,
    timings: &mut IndexCacheTimings,
) -> Result<Option<CacheLoad>, String> {
    if !cache_path.is_file() {
        return Ok(None);
    }

    let read_start = Instant::now();
    let bytes = read_index_cache_payload(cache_path)?;
    let magic = bytes
        .get(..CACHE_MAGIC.len())
        .ok_or_else(|| "Index cache is shorter than its magic".to_string())?;
    if magic == V10_CACHE_MAGIC || magic == LEGACY_CACHE_MAGIC {
        return Ok(None);
    }
    timings.cache_file_read = read_start.elapsed();
    let decode_start = Instant::now();
    let load = if magic == CACHE_MAGIC {
        let cached = decode_runtime_cache(&bytes).map_err(|error| {
            format!(
                "Failed to decode index cache {}: {error}",
                cache_path.display()
            )
        })?;
        drop(bytes);
        CacheLoad::Current(cached)
    } else {
        return Err(format!(
            "Failed to decode index cache {}: binary cache magic mismatch",
            cache_path.display()
        ));
    };
    timings.cache_decode = decode_start.elapsed();

    let validate_start = Instant::now();
    let CacheLoad::Current(cached) = &load;
    if cached.schema != CACHE_SCHEMA
        || cached.format_version != CACHE_FORMAT_VERSION
        || cached.index_shape != CACHE_INDEX_SHAPE
        || cached.crate_version != env!("CARGO_PKG_VERSION")
        || cached.fingerprint != *expected_fingerprint
        || cached.source_digest != expected_source_digest
    {
        timings.cache_validate = validate_start.elapsed();
        return Ok(None);
    }
    timings.cache_validate = validate_start.elapsed();

    Ok(Some(load))
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    let digest = digest.as_ref();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

struct CacheWriteStageTimings {
    encode: Duration,
    atomic_write: Duration,
}

fn write_runtime_cache_payload(
    cache_path: &Path,
    index: &SymbolIndex,
    source_line_starts: &[Vec<usize>],
    fingerprint: &SourceFingerprint,
    source_digest: &str,
    summary: &CachedIndexSummary,
    locator_payload: Option<&[u8]>,
) -> Result<CacheWriteStageTimings, String> {
    let encode_start = Instant::now();
    let bytes = encode_runtime_index(
        index,
        source_line_starts,
        fingerprint,
        source_digest,
        summary,
    )?;
    let bytes = match locator_payload {
        Some(locator_payload) => encode_cache_container(&bytes, locator_payload)?,
        None => bytes,
    };
    let encode = encode_start.elapsed();
    let atomic_write_start = Instant::now();
    write_atomic_bytes(cache_path, &bytes)?;
    Ok(CacheWriteStageTimings {
        encode,
        atomic_write: atomic_write_start.elapsed(),
    })
}

#[derive(Debug, Clone, Copy)]
struct CacheContainerHeader {
    index_offset: u64,
    index_length: u64,
    locator_offset: u64,
    locator_length: u64,
}

fn encode_cache_container(index_payload: &[u8], locator_payload: &[u8]) -> Result<Vec<u8>, String> {
    let index_offset = CACHE_CONTAINER_HEADER_BYTES;
    let index_length = u64::try_from(index_payload.len())
        .map_err(|_| "Index cache payload is too large".to_string())?;
    let locator_offset = index_offset
        .checked_add(index_length)
        .ok_or_else(|| "Index cache payload offset overflowed".to_string())?;
    let locator_length = u64::try_from(locator_payload.len())
        .map_err(|_| "Index cache locator payload is too large".to_string())?;
    let total_length = locator_offset
        .checked_add(locator_length)
        .ok_or_else(|| "Index cache container length overflowed".to_string())?;
    let mut bytes = Vec::with_capacity(
        usize::try_from(total_length)
            .map_err(|_| "Index cache container is too large".to_string())?,
    );
    bytes.extend_from_slice(CACHE_CONTAINER_MAGIC);
    bytes.extend_from_slice(&CACHE_CONTAINER_VERSION.to_le_bytes());
    bytes.extend_from_slice(&index_offset.to_le_bytes());
    bytes.extend_from_slice(&index_length.to_le_bytes());
    bytes.extend_from_slice(&locator_offset.to_le_bytes());
    bytes.extend_from_slice(&locator_length.to_le_bytes());
    bytes.extend_from_slice(index_payload);
    bytes.extend_from_slice(locator_payload);
    Ok(bytes)
}

fn read_cache_container_header(
    file: &mut fs::File,
    file_bytes: u64,
) -> Result<Option<CacheContainerHeader>, String> {
    if file_bytes < CACHE_CONTAINER_HEADER_BYTES {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("Failed to seek index cache container: {error}"))?;
    let mut header = [0_u8; CACHE_CONTAINER_HEADER_BYTES as usize];
    file.read_exact(&mut header)
        .map_err(|error| format!("Failed to read index cache container header: {error}"))?;
    parse_cache_container_header(&header, file_bytes)
}

fn parse_cache_container_header(
    header: &[u8],
    file_bytes: u64,
) -> Result<Option<CacheContainerHeader>, String> {
    if header.len() < CACHE_CONTAINER_HEADER_BYTES as usize
        || header[..CACHE_CONTAINER_MAGIC.len()] != CACHE_CONTAINER_MAGIC[..]
    {
        return Ok(None);
    }
    let version = u32::from_le_bytes(header[8..12].try_into().unwrap());
    if version != CACHE_CONTAINER_VERSION {
        return Err(format!(
            "Unsupported index cache container version {version}"
        ));
    }
    let index_offset = u64::from_le_bytes(header[12..20].try_into().unwrap());
    let index_length = u64::from_le_bytes(header[20..28].try_into().unwrap());
    let locator_offset = u64::from_le_bytes(header[28..36].try_into().unwrap());
    let locator_length = u64::from_le_bytes(header[36..44].try_into().unwrap());
    let index_end = index_offset
        .checked_add(index_length)
        .ok_or_else(|| "Index cache index section overflows".to_string())?;
    let locator_end = locator_offset
        .checked_add(locator_length)
        .ok_or_else(|| "Index cache locator section overflows".to_string())?;
    if index_offset < CACHE_CONTAINER_HEADER_BYTES
        || index_length < CACHE_MAGIC.len() as u64
        || index_length > MAX_CACHE_SECTION_BYTES
        || locator_length > MAX_CACHE_SECTION_BYTES
        || index_end > file_bytes
        || locator_offset != index_end
        || locator_end > file_bytes
    {
        return Err("Index cache container section bounds are invalid".to_string());
    }
    Ok(Some(CacheContainerHeader {
        index_offset,
        index_length,
        locator_offset,
        locator_length,
    }))
}

#[cfg(test)]
fn index_cache_payload_from_bytes(bytes: &[u8]) -> Result<&[u8], String> {
    let Some(header) = parse_cache_container_header(
        bytes,
        u64::try_from(bytes.len()).map_err(|_| "Index cache is too large".to_string())?,
    )?
    else {
        return Ok(bytes);
    };
    let start = usize::try_from(header.index_offset)
        .map_err(|_| "Index cache payload offset is too large".to_string())?;
    let length = usize::try_from(header.index_length)
        .map_err(|_| "Index cache payload length is too large".to_string())?;
    let end = start
        .checked_add(length)
        .ok_or_else(|| "Index cache payload bounds overflowed".to_string())?;
    bytes
        .get(start..end)
        .ok_or_else(|| "Index cache payload is out of bounds".to_string())
}

fn read_index_cache_payload(cache_path: &Path) -> Result<Vec<u8>, String> {
    let mut file = fs::File::open(cache_path).map_err(|error| {
        format!(
            "Failed to open index cache {}: {error}",
            cache_path.display()
        )
    })?;
    let file_bytes = file
        .metadata()
        .map_err(|error| {
            format!(
                "Failed to stat index cache {}: {error}",
                cache_path.display()
            )
        })?
        .len();
    let mut magic = [0_u8; CACHE_MAGIC.len()];
    file.read_exact(&mut magic)
        .map_err(|error| format!("Failed to read index cache magic: {error}"))?;
    if magic == *CACHE_CONTAINER_MAGIC {
        let header = read_cache_container_header(&mut file, file_bytes)?
            .ok_or_else(|| "Invalid index cache container header".to_string())?;
        file.seek(SeekFrom::Start(header.index_offset))
            .map_err(|error| format!("Failed to seek index cache payload: {error}"))?;
        let length = usize::try_from(header.index_length)
            .map_err(|_| "Index cache payload is too large".to_string())?;
        let mut payload = vec![0_u8; length];
        file.read_exact(&mut payload)
            .map_err(|error| format!("Failed to read index cache payload: {error}"))?;
        return Ok(payload);
    }
    if magic == *V10_CACHE_MAGIC || magic == *LEGACY_CACHE_MAGIC {
        return Ok(magic.to_vec());
    }
    let mut payload = Vec::new();
    payload.extend_from_slice(&magic);
    file.read_to_end(&mut payload).map_err(|error| {
        format!(
            "Failed to read index cache {}: {error}",
            cache_path.display()
        )
    })?;
    Ok(payload)
}

/// Reads the optional binary locator section without decoding the semantic
/// index. `None` means an older raw cache or a generic cache without locators.
pub(crate) fn read_index_cache_locator_section(
    cache_path: &Path,
) -> Result<Option<Vec<u8>>, String> {
    if !cache_path.is_file() {
        return Ok(None);
    }
    let mut file = fs::File::open(cache_path).map_err(|error| error.to_string())?;
    let file_bytes = file.metadata().map_err(|error| error.to_string())?.len();
    let Some(header) = read_cache_container_header(&mut file, file_bytes)? else {
        return Ok(None);
    };
    if header.locator_length == 0 {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(header.locator_offset))
        .map_err(|error| error.to_string())?;
    let length = usize::try_from(header.locator_length)
        .map_err(|_| "Index cache locator payload is too large".to_string())?;
    let mut payload = vec![0_u8; length];
    file.read_exact(&mut payload)
        .map_err(|error| error.to_string())?;
    Ok(Some(payload))
}

pub(crate) fn cache_format_identity() -> (&'static str, u32, &'static str) {
    (CACHE_SCHEMA, CACHE_FORMAT_VERSION, CACHE_INDEX_SHAPE)
}

pub(crate) fn write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create index cache folder {}: {error}",
                parent.display()
            )
        })?;
    }

    let temp_path = unique_cache_temp_path(path);
    let result = (|| {
        let file = fs::File::create(&temp_path).map_err(|error| {
            format!(
                "Failed to create temporary index cache {}: {error}",
                temp_path.display()
            )
        })?;
        let mut writer = BufWriter::new(file);
        writer.write_all(bytes).map_err(|error| {
            format!(
                "Failed to write index cache {}: {error}",
                temp_path.display()
            )
        })?;
        writer.flush().map_err(|error| {
            format!(
                "Failed to flush index cache {}: {error}",
                temp_path.display()
            )
        })?;
        // Windows `ReplaceFileW` requires the replacement handle to be closed.
        // Flushing alone leaves the `BufWriter` (and its file) open.
        drop(writer);
        replace_cache_atomically(&temp_path, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(not(windows))]
fn replace_cache_atomically(temp_path: &Path, cache_path: &Path) -> Result<(), String> {
    fs::rename(temp_path, cache_path).map_err(|error| {
        format!(
            "Failed to replace index cache {} with {}: {error}",
            cache_path.display(),
            temp_path.display()
        )
    })
}

#[cfg(windows)]
fn replace_cache_atomically(temp_path: &Path, cache_path: &Path) -> Result<(), String> {
    if !cache_path.exists() {
        return fs::rename(temp_path, cache_path).map_err(|error| {
            format!(
                "Failed to install index cache {} from {}: {error}",
                cache_path.display(),
                temp_path.display()
            )
        });
    }

    use std::os::windows::ffi::OsStrExt;
    let replaced: Vec<u16> = cache_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let replacement: Vec<u16> = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let replaced = unsafe {
        replace_file_w(
            replaced.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        return Err(format!(
            "Failed to atomically replace index cache {} with {}: {}",
            cache_path.display(),
            temp_path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(windows)]
unsafe fn replace_file_w(
    replaced: *const u16,
    replacement: *const u16,
    backup: *const u16,
    flags: u32,
    exclude: *mut std::ffi::c_void,
    reserved: *mut std::ffi::c_void,
) -> i32 {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced: *const u16,
            replacement: *const u16,
            backup: *const u16,
            flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
    }
    unsafe { ReplaceFileW(replaced, replacement, backup, flags, exclude, reserved) }
}

fn unique_cache_temp_path(cache_path: &Path) -> PathBuf {
    let nonce = UNIX_EPOCH
        .elapsed()
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let file_name = cache_path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "game-data-symbol-index".into());
    cache_path.with_file_name(format!("{file_name}.{}.{}.tmp", std::process::id(), nonce))
}

#[cfg(test)]
fn encode_cached_index(cached: &CachedGameDataIndex) -> Result<Vec<u8>, String> {
    let string_table = CacheStringTable::from_cached_index(cached)?;
    let mut writer = BinaryWriter::new(string_table);
    writer.write_bytes(CACHE_MAGIC);
    writer.write_string_table()?;
    writer.write_string(&cached.schema)?;
    writer.write_u32(cached.format_version);
    writer.write_string(&cached.index_shape)?;
    writer.write_string(&cached.crate_version)?;
    writer.write_fingerprint(&cached.fingerprint)?;
    writer.write_string(&cached.source_digest)?;
    writer.write_summary(&cached.summary);
    writer.write_vec_len(cached.files.len())?;
    for file in &cached.files {
        writer.write_cached_file_contribution(file)?;
    }
    Ok(writer.into_bytes())
}

fn encode_runtime_index(
    index: &SymbolIndex,
    source_line_starts: &[Vec<usize>],
    fingerprint: &SourceFingerprint,
    source_digest: &str,
    summary: &CachedIndexSummary,
) -> Result<Vec<u8>, String> {
    let string_table = CacheStringTable::from_runtime_index(index, fingerprint, source_digest)?;
    let line_start_count = source_line_starts.iter().map(Vec::len).sum::<usize>();
    let estimated_capacity = string_table
        .current_encoded_len()
        .saturating_add(index.symbols().len().saturating_mul(40))
        .saturating_add(line_start_count.saturating_mul(size_of::<u32>()))
        .saturating_add(index.files().len().saturating_mul(64))
        .saturating_add(1_024);
    let mut writer = BinaryWriter::new_with_capacity(string_table, estimated_capacity);
    writer.write_bytes(CACHE_MAGIC);
    writer.write_string_table()?;
    writer.write_string(CACHE_SCHEMA)?;
    writer.write_u32(CACHE_FORMAT_VERSION);
    writer.write_string(CACHE_INDEX_SHAPE)?;
    writer.write_string(env!("CARGO_PKG_VERSION"))?;
    writer.write_fingerprint(fingerprint)?;
    writer.write_string(source_digest)?;
    writer.write_summary(summary);
    writer.write_vec_len(index.files().len())?;
    for file in index.files() {
        let end = file
            .symbol_start
            .checked_add(file.symbol_count)
            .ok_or_else(|| "runtime cache symbol range overflow".to_string())?;
        let symbols = index
            .symbols()
            .get(file.symbol_start..end)
            .ok_or_else(|| "runtime cache symbol range is invalid".to_string())?;
        let line_starts = source_line_starts.get(file.id.0).ok_or_else(|| {
            format!(
                "runtime cache is missing line starts for file {}",
                file.id.0
            )
        })?;
        writer.write_runtime_file(file, symbols, line_starts)?;
    }
    Ok(writer.into_bytes())
}

#[cfg(test)]
fn decode_cached_index(bytes: &[u8]) -> Result<CachedGameDataIndex, String> {
    let bytes = index_cache_payload_from_bytes(bytes)?;
    let mut reader = BinaryReader::new(bytes);
    let magic = reader.read_exact(CACHE_MAGIC.len())?;
    if magic != &CACHE_MAGIC[..] {
        return Err("binary cache magic mismatch".to_string());
    }
    reader.read_string_table()?;
    let schema = reader.read_string()?;
    let format_version = reader.read_u32()?;
    let index_shape = reader.read_string()?;
    let crate_version = reader.read_string()?;
    let fingerprint = reader.read_fingerprint()?;
    let source_digest = reader.read_string()?;
    let summary = reader.read_summary()?;
    let file_count = reader.read_bounded_len("file records", MAX_CACHE_FILE_RECORDS)?;
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        files.push(reader.read_cached_file_contribution()?);
    }
    reader.expect_eof()?;
    Ok(CachedGameDataIndex {
        schema,
        format_version,
        index_shape,
        crate_version,
        fingerprint,
        source_digest,
        summary,
        files,
    })
}

fn decode_runtime_cache(bytes: &[u8]) -> Result<DecodedRuntimeCache, String> {
    let mut reader = BinaryReader::new(bytes);
    let magic = reader.read_exact(CACHE_MAGIC.len())?;
    if magic != &CACHE_MAGIC[..] {
        return Err("binary cache magic mismatch".to_string());
    }
    reader.read_string_table()?;
    let schema = reader.read_string()?;
    let format_version = reader.read_u32()?;
    let index_shape = reader.read_string()?;
    let crate_version = reader.read_string()?;
    let fingerprint = reader.read_fingerprint()?;
    let source_digest = reader.read_string()?;
    let summary = reader.read_summary()?;
    let file_count = reader.read_bounded_len("file records", MAX_CACHE_FILE_RECORDS)?;
    let mut files = Vec::with_capacity(file_count);
    let mut symbols = Vec::with_capacity(summary.indexed_symbols);
    let mut source_line_starts = BTreeMap::new();
    for file_index in 0..file_count {
        let file_id = SourceFileId(file_index);
        let metadata = reader.read_metadata()?;
        let non_declaration_callable_fragments = reader.read_usize()?;
        let line_starts = reader.read_source_line_starts()?;
        let symbol_count =
            reader.read_bounded_len("cached public symbols", MAX_CACHE_SYMBOL_RECORDS)?;
        let symbol_start = symbols.len();
        for expected_id in 0..symbol_count {
            symbols.push(reader.read_runtime_public_symbol(
                file_id,
                SymbolId(expected_id),
                symbol_count,
            )?);
        }
        files.push(IndexedFile {
            id: file_id,
            metadata,
            symbol_start,
            symbol_count,
            non_declaration_callable_fragments,
        });
        source_line_starts.insert(file_id, line_starts);
    }
    reader.expect_eof()?;
    Ok(DecodedRuntimeCache {
        schema,
        format_version,
        index_shape,
        crate_version,
        fingerprint,
        source_digest,
        summary,
        files,
        symbols,
        source_line_starts,
    })
}

struct CacheStringTable<'source> {
    ids: AHashMap<Cow<'source, str>, u32>,
    values: Vec<Cow<'source, str>>,
}

impl<'source> CacheStringTable<'source> {
    #[cfg(test)]
    fn from_cached_index(cached: &'source CachedGameDataIndex) -> Result<Self, String> {
        let mut table = Self {
            ids: AHashMap::new(),
            values: Vec::new(),
        };
        table.insert(&cached.schema)?;
        table.insert(&cached.index_shape)?;
        table.insert(&cached.crate_version)?;
        table.insert_fingerprint(&cached.fingerprint)?;
        table.insert(&cached.source_digest)?;
        for file in &cached.files {
            table.insert_cached_file(file)?;
        }
        Ok(table)
    }

    fn from_runtime_index(
        index: &'source SymbolIndex,
        fingerprint: &'source SourceFingerprint,
        source_digest: &'source str,
    ) -> Result<Self, String> {
        let mut table = Self {
            ids: AHashMap::new(),
            values: Vec::with_capacity(index.symbols().len()),
        };
        table.insert(CACHE_SCHEMA)?;
        table.insert(CACHE_INDEX_SHAPE)?;
        table.insert(env!("CARGO_PKG_VERSION"))?;
        table.insert_fingerprint(fingerprint)?;
        table.insert(source_digest)?;
        for file in index.files() {
            table.insert_metadata(&file.metadata)?;
            let end = file
                .symbol_start
                .checked_add(file.symbol_count)
                .ok_or_else(|| "runtime cache symbol range overflow".to_string())?;
            let symbols = index
                .symbols()
                .get(file.symbol_start..end)
                .ok_or_else(|| "runtime cache symbol range is invalid".to_string())?;
            for symbol in symbols {
                table.insert_runtime_symbol(symbol)?;
            }
        }
        Ok(table)
    }

    fn insert(&mut self, value: &'source str) -> Result<(), String> {
        self.insert_value(Cow::Borrowed(value))
    }

    fn insert_value(&mut self, value: Cow<'source, str>) -> Result<(), String> {
        if self.ids.contains_key(value.as_ref()) {
            return Ok(());
        }
        let id = u32::try_from(self.values.len())
            .map_err(|_| "cache string table exceeds u32 entries".to_string())?;
        self.values.push(value.clone());
        self.ids.insert(value, id);
        Ok(())
    }

    fn insert_option(&mut self, value: Option<&'source str>) -> Result<(), String> {
        if let Some(value) = value {
            self.insert(value)?;
        }
        Ok(())
    }

    fn insert_path(&mut self, value: &'source Path) -> Result<(), String> {
        self.insert_value(value.to_string_lossy())
    }

    fn insert_option_path(&mut self, value: Option<&'source Path>) -> Result<(), String> {
        if let Some(value) = value {
            self.insert_path(value)?;
        }
        Ok(())
    }

    fn insert_fingerprint(
        &mut self,
        fingerprint: &'source SourceFingerprint,
    ) -> Result<(), String> {
        match fingerprint {
            SourceFingerprint::Downloaded {
                scripts_root,
                commit_sha,
            } => {
                self.insert(scripts_root)?;
                self.insert(commit_sha)?;
            }
            SourceFingerprint::Manual { scripts_root, .. } => {
                self.insert(scripts_root)?;
            }
            SourceFingerprint::Addon {
                guid,
                artifact_digest,
                ..
            } => {
                self.insert(guid)?;
                self.insert(artifact_digest)?;
            }
        }
        Ok(())
    }

    fn insert_metadata(&mut self, metadata: &'source SourceFileMetadata) -> Result<(), String> {
        self.insert_option_path(metadata.absolute_path.as_deref())?;
        if let Some(source) = &metadata.virtual_source {
            self.insert(&source.uri)?;
            self.insert(&source.addon_guid)?;
            self.insert(&source.revision)?;
            self.insert(&source.logical_path)?;
        }
        self.insert_option_path(metadata.root_path.as_deref())?;
        self.insert_option_path(metadata.relative_path.as_deref())
    }

    #[cfg(test)]
    fn insert_cached_file(&mut self, file: &'source CachedFileContribution) -> Result<(), String> {
        self.insert_metadata(&file.metadata)?;
        for symbol in &file.symbols {
            self.insert(&symbol.name)?;
            self.insert_option(symbol.detail.type_text.as_deref())?;
            self.insert_option(symbol.detail.return_type.as_deref())?;
            self.insert_option(symbol.detail.base_type.as_deref())?;
            self.insert_option(symbol.detail.default_value.as_deref())?;
            self.insert_option(symbol.detail.enum_value.as_deref())?;
            for attribute in &symbol.attributes {
                self.insert(attribute)?;
            }
            for modifier in &symbol.modifiers {
                self.insert(modifier)?;
            }
            for comment in &symbol.doc_comments {
                self.insert(&comment.text)?;
            }
            for branch in &symbol.conditional_context {
                self.insert_option(branch.condition.as_deref())?;
            }
        }
        Ok(())
    }

    fn insert_runtime_symbol(&mut self, symbol: &'source IndexedSymbol) -> Result<(), String> {
        self.insert_option(symbol.name.as_deref())?;
        self.insert_option(symbol.detail.type_text.as_deref())?;
        self.insert_option(symbol.detail.return_type_text.as_deref())?;
        self.insert_option(symbol.detail.base_type.as_deref())?;
        self.insert_option(symbol.detail.default_text.as_deref())?;
        self.insert_option(symbol.detail.enum_value_text.as_deref())?;
        for attribute in &symbol.attributes {
            self.insert(&attribute.text)?;
        }
        for modifier in &symbol.modifiers {
            self.insert(modifier)?;
        }
        for comment in &symbol.doc_comments {
            self.insert(&comment.text)?;
        }
        for branch in &symbol.conditional_context {
            self.insert_option(branch.condition.as_deref())?;
        }
        Ok(())
    }

    fn id(&self, value: &str) -> Result<u32, String> {
        self.ids
            .get(value)
            .copied()
            .ok_or_else(|| format!("cache string was not interned before write: {value:?}"))
    }

    fn current_encoded_len(&self) -> usize {
        size_of::<u32>().saturating_add(
            self.values
                .iter()
                .map(|value| size_of::<u32>().saturating_add(value.len()))
                .sum::<usize>(),
        )
    }
}

struct BinaryWriter<'source> {
    bytes: Vec<u8>,
    string_table: CacheStringTable<'source>,
    narrow_integers: bool,
}

impl<'source> BinaryWriter<'source> {
    #[cfg(test)]
    fn new(string_table: CacheStringTable<'source>) -> Self {
        Self {
            bytes: Vec::new(),
            string_table,
            narrow_integers: true,
        }
    }

    fn new_with_capacity(
        string_table: CacheStringTable<'source>,
        estimated_capacity: usize,
    ) -> Self {
        Self {
            bytes: Vec::with_capacity(estimated_capacity),
            string_table,
            narrow_integers: true,
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_usize(&mut self, value: usize) -> Result<(), String> {
        if self.narrow_integers {
            let value =
                u32::try_from(value).map_err(|_| "cache integer exceeds u32".to_string())?;
            self.write_u32(value);
        } else {
            let value = u64::try_from(value).map_err(|_| "usize value exceeds u64".to_string())?;
            self.write_u64(value);
        }
        Ok(())
    }

    fn write_vec_len(&mut self, len: usize) -> Result<(), String> {
        self.write_usize(len)
    }

    fn write_string_table(&mut self) -> Result<(), String> {
        self.write_vec_len(self.string_table.values.len())?;
        // `values` is already the deterministic insertion order represented
        // by the table IDs. Append directly into the output buffer instead of
        // cloning every interned string for the write pass.
        let bytes = &mut self.bytes;
        for value in &self.string_table.values {
            let value = value.as_bytes();
            if self.narrow_integers {
                let len = u32::try_from(value.len())
                    .map_err(|_| "cache string byte length exceeds u32".to_string())?;
                bytes.extend_from_slice(&len.to_le_bytes());
            } else {
                let len = u64::try_from(value.len())
                    .map_err(|_| "usize value exceeds u64".to_string())?;
                bytes.extend_from_slice(&len.to_le_bytes());
            }
            bytes.extend_from_slice(value);
        }
        Ok(())
    }

    fn write_string(&mut self, value: &str) -> Result<(), String> {
        let id = self.string_table.id(value)?;
        self.write_u32(id);
        Ok(())
    }

    fn write_path(&mut self, value: &Path) -> Result<(), String> {
        self.write_string(&value.to_string_lossy())
    }

    fn write_option_string(&mut self, value: Option<&str>) -> Result<(), String> {
        match value {
            Some(value) => {
                self.write_u8(1);
                self.write_string(value)?;
            }
            None => self.write_u8(0),
        }
        Ok(())
    }

    fn write_option_path(&mut self, value: Option<&Path>) -> Result<(), String> {
        match value {
            Some(value) => {
                self.write_u8(1);
                self.write_path(value)?;
            }
            None => self.write_u8(0),
        }
        Ok(())
    }

    fn write_span(&mut self, span: TextSpan) -> Result<(), String> {
        self.write_usize(span.start)?;
        self.write_usize(span.end)
    }

    fn write_fingerprint(&mut self, fingerprint: &SourceFingerprint) -> Result<(), String> {
        match fingerprint {
            SourceFingerprint::Downloaded {
                scripts_root,
                commit_sha,
            } => {
                self.write_u8(0);
                self.write_string(scripts_root)?;
                self.write_string(commit_sha)?;
            }
            SourceFingerprint::Manual {
                scripts_root,
                file_count,
                byte_count,
                latest_modified_unix_ms,
            } => {
                self.write_u8(1);
                self.write_string(scripts_root)?;
                self.write_usize(*file_count)?;
                self.write_u64(*byte_count);
                self.write_u128(*latest_modified_unix_ms);
            }
            SourceFingerprint::Addon {
                guid,
                artifact_digest,
                pack_count,
                catalogue_entry_count,
            } => {
                self.write_u8(2);
                self.write_string(guid)?;
                self.write_string(artifact_digest)?;
                self.write_usize(*pack_count)?;
                self.write_usize(*catalogue_entry_count)?;
            }
        }
        Ok(())
    }

    fn write_summary(&mut self, summary: &CachedIndexSummary) {
        self.write_u64(summary.files as u64);
        self.write_u64(summary.bytes as u64);
        self.write_u64(summary.indexed_symbols as u64);
        self.write_u64(summary.parse_diagnostics as u64);
        self.write_u64(summary.lossy_files as u64);
    }

    fn write_metadata(&mut self, metadata: &SourceFileMetadata) -> Result<(), String> {
        self.write_u8(source_kind_tag(metadata.kind));
        self.write_u8(source_category_tag(metadata.category));
        self.write_option_path(metadata.absolute_path.as_deref())?;
        self.write_u8(u8::from(metadata.virtual_source.is_some()));
        if let Some(source) = &metadata.virtual_source {
            self.write_string(&source.uri)?;
            self.write_string(&source.addon_guid)?;
            self.write_string(&source.revision)?;
            self.write_string(&source.logical_path)?;
        }
        self.write_option_path(metadata.root_path.as_deref())?;
        self.write_option_path(metadata.relative_path.as_deref())?;
        self.write_u16(metadata.priority);
        Ok(())
    }

    #[cfg(test)]
    fn write_cached_file_contribution(
        &mut self,
        file: &CachedFileContribution,
    ) -> Result<(), String> {
        self.write_metadata(&file.metadata)?;
        self.write_usize(file.non_declaration_callable_fragments)?;
        self.write_source_line_starts(&file.source_line_starts)?;
        self.write_vec_len(file.symbols.len())?;
        for symbol in &file.symbols {
            self.write_cached_public_symbol(symbol)?;
        }
        Ok(())
    }

    fn write_runtime_file(
        &mut self,
        file: &IndexedFile,
        symbols: &[IndexedSymbol],
        source_line_starts: &[usize],
    ) -> Result<(), String> {
        self.write_metadata(&file.metadata)?;
        self.write_usize(file.non_declaration_callable_fragments)?;
        self.write_source_line_starts(source_line_starts)?;
        self.write_vec_len(symbols.len())?;
        for symbol in symbols {
            self.write_runtime_public_symbol(symbol)?;
        }
        Ok(())
    }

    fn write_source_line_starts(&mut self, line_starts: &[usize]) -> Result<(), String> {
        self.write_vec_len(line_starts.len())?;
        if line_starts.first().copied() != Some(0) {
            return Err("invalid cached source line starts".to_string());
        }
        let mut previous = 0_usize;
        for start in line_starts.iter().copied().skip(1) {
            let delta = start
                .checked_sub(previous)
                .filter(|delta| *delta > 0)
                .ok_or_else(|| "invalid cached source line starts".to_string())?;
            self.write_usize(delta)?;
            previous = start;
        }
        Ok(())
    }

    #[cfg(test)]
    fn write_cached_public_symbol(&mut self, symbol: &CachedPublicSymbol) -> Result<(), String> {
        let mut flags = 0_u16;
        if symbol.parent.is_some() {
            flags |= CACHED_SYMBOL_HAS_PARENT;
        }
        flags |= u16::from(symbol.detail.type_text.is_some()) * CACHED_SYMBOL_HAS_TYPE;
        flags |= u16::from(symbol.detail.return_type.is_some()) * CACHED_SYMBOL_HAS_RETURN_TYPE;
        flags |= u16::from(symbol.detail.base_type.is_some()) * CACHED_SYMBOL_HAS_BASE_TYPE;
        flags |= u16::from(symbol.detail.default_value.is_some()) * CACHED_SYMBOL_HAS_DEFAULT_VALUE;
        flags |= u16::from(symbol.detail.enum_value.is_some()) * CACHED_SYMBOL_HAS_ENUM_VALUE;
        flags |= u16::from(!symbol.attributes.is_empty()) * CACHED_SYMBOL_HAS_ATTRIBUTES;
        flags |= u16::from(!symbol.modifiers.is_empty()) * CACHED_SYMBOL_HAS_MODIFIERS;
        flags |= u16::from(!symbol.doc_comments.is_empty()) * CACHED_SYMBOL_HAS_DOC_COMMENTS;
        flags |= u16::from(!symbol.conditional_context.is_empty())
            * CACHED_SYMBOL_HAS_CONDITIONAL_CONTEXT;
        flags |= u16::from(symbol.callable_form.is_some()) * CACHED_SYMBOL_HAS_CALLABLE_FORM;
        self.write_u16(flags);
        if let Some(parent) = symbol.parent {
            self.write_u32(parent.0);
        }
        self.write_u8(semantic_declaration_kind_tag(symbol.kind));
        self.write_string(&symbol.name)?;
        self.write_span(symbol.span)?;
        self.write_span(symbol.selection_span)?;
        if let Some(value) = symbol.detail.type_text.as_deref() {
            self.write_string(value)?;
        }
        if let Some(value) = symbol.detail.return_type.as_deref() {
            self.write_string(value)?;
        }
        if let Some(value) = symbol.detail.base_type.as_deref() {
            self.write_string(value)?;
        }
        if let Some(value) = symbol.detail.default_value.as_deref() {
            self.write_string(value)?;
        }
        if let Some(value) = symbol.detail.enum_value.as_deref() {
            self.write_string(value)?;
        }
        if !symbol.attributes.is_empty() {
            self.write_vec_len(symbol.attributes.len())?;
            for attribute in &symbol.attributes {
                self.write_string(attribute)?;
            }
        }
        if !symbol.modifiers.is_empty() {
            self.write_vec_len(symbol.modifiers.len())?;
            for modifier in &symbol.modifiers {
                self.write_string(modifier)?;
            }
        }
        if !symbol.doc_comments.is_empty() {
            self.write_vec_len(symbol.doc_comments.len())?;
            for comment in &symbol.doc_comments {
                self.write_u8(semantic_doc_comment_kind_tag(comment.kind));
                self.write_string(&comment.text)?;
            }
        }
        if !symbol.conditional_context.is_empty() {
            self.write_vec_len(symbol.conditional_context.len())?;
            for branch in &symbol.conditional_context {
                self.write_u8(semantic_conditional_branch_kind_tag(branch.kind));
                self.write_option_string(branch.condition.as_deref())?;
            }
        }
        if let Some(form) = symbol.callable_form {
            self.write_u8(semantic_callable_form_tag(form));
        }
        Ok(())
    }

    fn write_runtime_public_symbol(&mut self, symbol: &IndexedSymbol) -> Result<(), String> {
        if symbol.kind == SymbolKind::LocalVariable {
            return Err("runtime cache contains a local variable".to_string());
        }
        let name = symbol
            .name
            .as_deref()
            .ok_or_else(|| "runtime cache contains an unnamed public symbol".to_string())?;
        let kind = public_semantic_kind(symbol.kind)
            .ok_or_else(|| "runtime cache contains a non-public symbol kind".to_string())?;
        let mut flags = 0_u16;
        if symbol.parent.is_some() {
            flags |= CACHED_SYMBOL_HAS_PARENT;
        }
        flags |= u16::from(symbol.detail.type_text.is_some()) * CACHED_SYMBOL_HAS_TYPE;
        flags |=
            u16::from(symbol.detail.return_type_text.is_some()) * CACHED_SYMBOL_HAS_RETURN_TYPE;
        flags |= u16::from(symbol.detail.base_type.is_some()) * CACHED_SYMBOL_HAS_BASE_TYPE;
        flags |= u16::from(symbol.detail.default_text.is_some()) * CACHED_SYMBOL_HAS_DEFAULT_VALUE;
        flags |= u16::from(symbol.detail.enum_value_text.is_some()) * CACHED_SYMBOL_HAS_ENUM_VALUE;
        flags |= u16::from(!symbol.attributes.is_empty()) * CACHED_SYMBOL_HAS_ATTRIBUTES;
        flags |= u16::from(!symbol.modifiers.is_empty()) * CACHED_SYMBOL_HAS_MODIFIERS;
        flags |= u16::from(!symbol.doc_comments.is_empty()) * CACHED_SYMBOL_HAS_DOC_COMMENTS;
        flags |= u16::from(!symbol.conditional_context.is_empty())
            * CACHED_SYMBOL_HAS_CONDITIONAL_CONTEXT;
        flags |= u16::from(symbol.callable_form.is_some()) * CACHED_SYMBOL_HAS_CALLABLE_FORM;
        self.write_u16(flags);
        if let Some(parent) = symbol.parent {
            self.write_u32(
                u32::try_from(parent.symbol_id.0)
                    .map_err(|_| "runtime cache parent id exceeds u32".to_string())?,
            );
        }
        self.write_u8(semantic_declaration_kind_tag(kind));
        self.write_string(name)?;
        self.write_span(symbol.span)?;
        self.write_span(symbol.selection_span)?;
        for value in [
            symbol.detail.type_text.as_deref(),
            symbol.detail.return_type_text.as_deref(),
            symbol.detail.base_type.as_deref(),
            symbol.detail.default_text.as_deref(),
            symbol.detail.enum_value_text.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            self.write_string(value)?;
        }
        if !symbol.attributes.is_empty() {
            self.write_vec_len(symbol.attributes.len())?;
            for attribute in &symbol.attributes {
                self.write_string(&attribute.text)?;
            }
        }
        if !symbol.modifiers.is_empty() {
            self.write_vec_len(symbol.modifiers.len())?;
            for modifier in &symbol.modifiers {
                self.write_string(modifier)?;
            }
        }
        if !symbol.doc_comments.is_empty() {
            self.write_vec_len(symbol.doc_comments.len())?;
            for comment in &symbol.doc_comments {
                self.write_u8(semantic_doc_comment_kind_tag(match comment.kind {
                    DocCommentKind::Line => SemanticDocCommentKind::Line,
                    DocCommentKind::Block => SemanticDocCommentKind::Block,
                }));
                self.write_string(&comment.text)?;
            }
        }
        if !symbol.conditional_context.is_empty() {
            self.write_vec_len(symbol.conditional_context.len())?;
            for branch in &symbol.conditional_context {
                self.write_u8(semantic_conditional_branch_kind_tag(match branch.kind {
                    PreprocessorBranchKind::If => SemanticConditionalBranchKind::If,
                    PreprocessorBranchKind::Ifdef => SemanticConditionalBranchKind::Ifdef,
                    PreprocessorBranchKind::Ifndef => SemanticConditionalBranchKind::Ifndef,
                    PreprocessorBranchKind::Elif => SemanticConditionalBranchKind::Elif,
                    PreprocessorBranchKind::Else => SemanticConditionalBranchKind::Else,
                }));
                self.write_option_string(branch.condition.as_deref())?;
            }
        }
        if let Some(form) = symbol.callable_form {
            self.write_u8(semantic_callable_form_tag(match form {
                CallableForm::Implementation => SemanticCallableForm::Implementation,
                CallableForm::Declaration => SemanticCallableForm::Declaration,
                CallableForm::Prototype => SemanticCallableForm::Prototype,
            }));
        }
        Ok(())
    }
}

struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    string_table: Vec<&'a str>,
    narrow_integers: bool,
}

impl<'a> BinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            string_table: Vec::new(),
            narrow_integers: true,
        }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "cache offset overflow".to_string())?;
        if end > self.bytes.len() {
            return Err("unexpected end of cache file".to_string());
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let bytes: [u8; 2] = self
            .read_exact(2)?
            .try_into()
            .map_err(|_| "invalid u16 bytes".to_string())?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes: [u8; 4] = self
            .read_exact(4)?
            .try_into()
            .map_err(|_| "invalid u32 bytes".to_string())?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let bytes: [u8; 8] = self
            .read_exact(8)?
            .try_into()
            .map_err(|_| "invalid u64 bytes".to_string())?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_u128(&mut self) -> Result<u128, String> {
        let bytes: [u8; 16] = self
            .read_exact(16)?
            .try_into()
            .map_err(|_| "invalid u128 bytes".to_string())?;
        Ok(u128::from_le_bytes(bytes))
    }

    fn read_usize(&mut self) -> Result<usize, String> {
        if self.narrow_integers {
            usize::try_from(self.read_u32()?).map_err(|_| "u32 value exceeds usize".to_string())
        } else {
            usize::try_from(self.read_u64()?).map_err(|_| "u64 value exceeds usize".to_string())
        }
    }

    fn read_len(&mut self) -> Result<usize, String> {
        self.read_usize()
    }

    fn read_bounded_len(&mut self, label: &str, max: usize) -> Result<usize, String> {
        let len = self.read_len()?;
        if len > max {
            return Err(format!(
                "cache {label} length {len} exceeds safety limit {max}"
            ));
        }
        Ok(len)
    }

    fn read_raw_string(&mut self) -> Result<&'a str, String> {
        let len = self.read_bounded_len("raw string byte", MAX_CACHE_RAW_STRING_BYTES)?;
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes).map_err(|error| format!("invalid utf-8 string: {error}"))
    }

    fn read_string_table(&mut self) -> Result<(), String> {
        let len = self.read_bounded_len("string table entry", MAX_CACHE_STRING_TABLE_ENTRIES)?;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.read_raw_string()?);
        }
        self.string_table = values;
        Ok(())
    }

    fn read_string(&mut self) -> Result<String, String> {
        let id = usize::try_from(self.read_u32()?)
            .map_err(|_| "string table id exceeds usize".to_string())?;
        self.string_table
            .get(id)
            .map(|value| (*value).to_string())
            .ok_or_else(|| format!("invalid string table id {id}"))
    }

    fn read_option_string(&mut self) -> Result<Option<String>, String> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_string().map(Some),
            tag => Err(format!("invalid option string tag {tag}")),
        }
    }

    fn read_option_path(&mut self) -> Result<Option<PathBuf>, String> {
        Ok(self.read_option_string()?.map(PathBuf::from))
    }

    fn read_span(&mut self) -> Result<TextSpan, String> {
        let start = self.read_usize()?;
        let end = self.read_usize()?;
        Ok(TextSpan { start, end })
    }

    fn read_fingerprint(&mut self) -> Result<SourceFingerprint, String> {
        match self.read_u8()? {
            0 => Ok(SourceFingerprint::Downloaded {
                scripts_root: self.read_string()?,
                commit_sha: self.read_string()?,
            }),
            1 => Ok(SourceFingerprint::Manual {
                scripts_root: self.read_string()?,
                file_count: self.read_usize()?,
                byte_count: self.read_u64()?,
                latest_modified_unix_ms: self.read_u128()?,
            }),
            2 => Ok(SourceFingerprint::Addon {
                guid: self.read_string()?,
                artifact_digest: self.read_string()?,
                pack_count: self.read_usize()?,
                catalogue_entry_count: self.read_usize()?,
            }),
            tag => Err(format!("invalid fingerprint tag {tag}")),
        }
    }

    fn read_summary(&mut self) -> Result<CachedIndexSummary, String> {
        Ok(CachedIndexSummary {
            files: usize::try_from(self.read_u64()?)
                .map_err(|_| "summary files exceeds usize".to_string())?,
            bytes: usize::try_from(self.read_u64()?)
                .map_err(|_| "summary bytes exceeds usize".to_string())?,
            indexed_symbols: usize::try_from(self.read_u64()?)
                .map_err(|_| "summary symbols exceeds usize".to_string())?,
            parse_diagnostics: usize::try_from(self.read_u64()?)
                .map_err(|_| "summary diagnostics exceeds usize".to_string())?,
            lossy_files: usize::try_from(self.read_u64()?)
                .map_err(|_| "summary lossy files exceeds usize".to_string())?,
        })
    }

    fn read_metadata(&mut self) -> Result<SourceFileMetadata, String> {
        Ok(SourceFileMetadata {
            kind: source_kind_from_tag(self.read_u8()?)?,
            category: source_category_from_tag(self.read_u8()?)?,
            absolute_path: self.read_option_path()?,
            virtual_source: if self.read_u8()? != 0 {
                Some(VirtualSourceIdentity {
                    uri: self.read_string()?,
                    addon_guid: self.read_string()?,
                    revision: self.read_string()?,
                    logical_path: self.read_string()?,
                })
            } else {
                None
            },
            root_path: self.read_option_path()?,
            relative_path: self.read_option_path()?,
            priority: self.read_u16()?,
        })
    }

    fn read_list<T>(
        &mut self,
        mut read_item: impl FnMut(&mut Self) -> Result<T, String>,
    ) -> Result<Vec<T>, String> {
        let len = self.read_bounded_len("symbol child list", MAX_CACHE_SYMBOL_LIST_ITEMS)?;
        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            items.push(read_item(self)?);
        }
        Ok(items)
    }

    #[cfg(test)]
    fn read_cached_file_contribution(&mut self) -> Result<CachedFileContribution, String> {
        let metadata = self.read_metadata()?;
        let non_declaration_callable_fragments = self.read_usize()?;
        let source_line_starts = self.read_source_line_starts()?;
        let symbol_count =
            self.read_bounded_len("cached public symbols", MAX_CACHE_SYMBOL_RECORDS)?;
        let mut symbols = Vec::with_capacity(symbol_count);
        for expected_id in 0..symbol_count {
            let expected_id = u32::try_from(expected_id)
                .map_err(|_| "cached public symbol id exceeds u32".to_string())?;
            symbols.push(self.read_cached_public_symbol(SemanticDeclarationId(expected_id))?);
        }
        Ok(CachedFileContribution {
            metadata,
            non_declaration_callable_fragments,
            source_line_starts,
            symbols,
        })
    }

    fn read_source_line_starts(&mut self) -> Result<Vec<usize>, String> {
        let line_count =
            self.read_bounded_len("source line starts", MAX_CACHE_SYMBOL_LIST_ITEMS)?;
        if line_count == 0 {
            return Err("invalid cached source line starts".to_string());
        }
        let mut source_line_starts = Vec::with_capacity(line_count);
        source_line_starts.push(0);
        let mut previous = 0_usize;
        for _ in 1..line_count {
            let delta = self.read_usize()?;
            if delta == 0 {
                return Err("invalid cached source line starts".to_string());
            }
            previous = previous
                .checked_add(delta)
                .ok_or_else(|| "cached source line start overflow".to_string())?;
            source_line_starts.push(previous);
        }
        Ok(source_line_starts)
    }

    #[cfg(test)]
    fn read_cached_public_symbol(
        &mut self,
        id: SemanticDeclarationId,
    ) -> Result<CachedPublicSymbol, String> {
        let flags = self.read_u16()?;
        if flags & !CACHED_SYMBOL_KNOWN_FLAGS != 0 {
            return Err(format!("invalid cached public symbol flags {flags:#06x}"));
        }
        let parent = if flags & CACHED_SYMBOL_HAS_PARENT != 0 {
            Some(SemanticDeclarationId(self.read_u32()?))
        } else {
            None
        };
        let kind = semantic_declaration_kind_from_tag(self.read_u8()?)?;
        let name = self.read_string()?;
        let span = self.read_span()?;
        let selection_span = self.read_span()?;
        let detail = CachedPublicSymbolDetail {
            type_text: self.read_flagged_string(flags, CACHED_SYMBOL_HAS_TYPE)?,
            return_type: self.read_flagged_string(flags, CACHED_SYMBOL_HAS_RETURN_TYPE)?,
            base_type: self.read_flagged_string(flags, CACHED_SYMBOL_HAS_BASE_TYPE)?,
            default_value: self.read_flagged_string(flags, CACHED_SYMBOL_HAS_DEFAULT_VALUE)?,
            enum_value: self.read_flagged_string(flags, CACHED_SYMBOL_HAS_ENUM_VALUE)?,
        };
        let attributes = if flags & CACHED_SYMBOL_HAS_ATTRIBUTES != 0 {
            self.read_list(Self::read_string)?
        } else {
            Vec::new()
        };
        let modifiers = if flags & CACHED_SYMBOL_HAS_MODIFIERS != 0 {
            self.read_list(Self::read_string)?
        } else {
            Vec::new()
        };
        let doc_comments = if flags & CACHED_SYMBOL_HAS_DOC_COMMENTS != 0 {
            self.read_list(|reader| {
                Ok(CachedDocComment {
                    kind: semantic_doc_comment_kind_from_tag(reader.read_u8()?)?,
                    text: reader.read_string()?,
                })
            })?
        } else {
            Vec::new()
        };
        let conditional_context = if flags & CACHED_SYMBOL_HAS_CONDITIONAL_CONTEXT != 0 {
            self.read_list(|reader| {
                Ok(CachedConditionalBranch {
                    kind: semantic_conditional_branch_kind_from_tag(reader.read_u8()?)?,
                    condition: reader.read_option_string()?,
                })
            })?
        } else {
            Vec::new()
        };
        let callable_form = if flags & CACHED_SYMBOL_HAS_CALLABLE_FORM != 0 {
            Some(semantic_callable_form_from_tag(self.read_u8()?)?)
        } else {
            None
        };
        Ok(CachedPublicSymbol {
            id,
            parent,
            kind,
            name,
            span,
            selection_span,
            detail,
            attributes,
            modifiers,
            doc_comments,
            conditional_context,
            callable_form,
        })
    }

    fn read_runtime_public_symbol(
        &mut self,
        file_id: SourceFileId,
        symbol_id: SymbolId,
        file_symbol_count: usize,
    ) -> Result<IndexedSymbol, String> {
        let flags = self.read_u16()?;
        if flags & !CACHED_SYMBOL_KNOWN_FLAGS != 0 {
            return Err(format!("invalid cached public symbol flags {flags:#06x}"));
        }
        let parent = if flags & CACHED_SYMBOL_HAS_PARENT != 0 {
            let parent = self.read_u32()? as usize;
            if parent >= file_symbol_count {
                return Err(format!(
                    "invalid cached public symbol parent {parent} for {file_symbol_count} symbols"
                ));
            }
            Some(GlobalSymbolId {
                file_id,
                symbol_id: SymbolId(parent),
            })
        } else {
            None
        };
        let kind = indexed_symbol_kind(semantic_declaration_kind_from_tag(self.read_u8()?)?);
        let name = self.read_string()?;
        if name.is_empty() {
            return Err(format!(
                "invalid cached public symbol {:?} has an empty name",
                GlobalSymbolId { file_id, symbol_id }
            ));
        }
        let name = Some(name);
        let span = self.read_span()?;
        let selection_span = self.read_span()?;
        let detail = IndexedSymbolDetail {
            type_text: self.read_flagged_string(flags, CACHED_SYMBOL_HAS_TYPE)?,
            type_text_span: None,
            return_type_text: self.read_flagged_string(flags, CACHED_SYMBOL_HAS_RETURN_TYPE)?,
            return_type_text_span: None,
            base_type: self.read_flagged_string(flags, CACHED_SYMBOL_HAS_BASE_TYPE)?,
            base_type_span: None,
            default_text: self.read_flagged_string(flags, CACHED_SYMBOL_HAS_DEFAULT_VALUE)?,
            default_text_span: None,
            enum_value_text: self.read_flagged_string(flags, CACHED_SYMBOL_HAS_ENUM_VALUE)?,
            enum_value_text_span: None,
        };
        let attributes = if flags & CACHED_SYMBOL_HAS_ATTRIBUTES != 0 {
            self.read_list(|reader| {
                let text = reader.read_string()?;
                Ok(IndexedAttribute {
                    name: semantic_attribute_name(&text).map(str::to_owned),
                    text,
                })
            })?
        } else {
            Vec::new()
        };
        let modifiers = if flags & CACHED_SYMBOL_HAS_MODIFIERS != 0 {
            self.read_list(Self::read_string)?
        } else {
            Vec::new()
        };
        let doc_comments = if flags & CACHED_SYMBOL_HAS_DOC_COMMENTS != 0 {
            self.read_list(|reader| {
                Ok(IndexedDocComment {
                    kind: match semantic_doc_comment_kind_from_tag(reader.read_u8()?)? {
                        SemanticDocCommentKind::Line => DocCommentKind::Line,
                        SemanticDocCommentKind::Block => DocCommentKind::Block,
                    },
                    text: reader.read_string()?,
                })
            })?
        } else {
            Vec::new()
        };
        let conditional_context = if flags & CACHED_SYMBOL_HAS_CONDITIONAL_CONTEXT != 0 {
            self.read_list(|reader| {
                Ok(IndexedConditionalBranch {
                    kind: indexed_conditional_kind(semantic_conditional_branch_kind_from_tag(
                        reader.read_u8()?,
                    )?),
                    condition: reader.read_option_string()?,
                })
            })?
        } else {
            Vec::new()
        };
        let callable_form = if flags & CACHED_SYMBOL_HAS_CALLABLE_FORM != 0 {
            Some(indexed_callable_form(semantic_callable_form_from_tag(
                self.read_u8()?,
            )?))
        } else {
            None
        };
        Ok(IndexedSymbol {
            id: GlobalSymbolId { file_id, symbol_id },
            parent,
            kind,
            name,
            span,
            selection_span,
            detail,
            attributes,
            modifiers,
            doc_comments,
            conditional_context,
            callable_form,
        })
    }

    fn read_flagged_string(&mut self, flags: u16, flag: u16) -> Result<Option<String>, String> {
        if flags & flag != 0 {
            self.read_string().map(Some)
        } else {
            Ok(None)
        }
    }

    fn expect_eof(&self) -> Result<(), String> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "cache has {} trailing bytes",
                self.bytes.len().saturating_sub(self.offset)
            ))
        }
    }
}

fn source_kind_tag(kind: SourceKind) -> u8 {
    match kind {
        SourceKind::Unknown => 0,
        SourceKind::GameData => 1,
        SourceKind::Workspace => 2,
        SourceKind::Fixture => 3,
    }
}

fn source_kind_from_tag(tag: u8) -> Result<SourceKind, String> {
    match tag {
        0 => Ok(SourceKind::Unknown),
        1 => Ok(SourceKind::GameData),
        2 => Ok(SourceKind::Workspace),
        3 => Ok(SourceKind::Fixture),
        _ => Err(format!("invalid source kind tag {tag}")),
    }
}

fn source_category_tag(category: SourceCategory) -> u8 {
    match category {
        SourceCategory::Workspace => 0,
        SourceCategory::Game => 1,
        SourceCategory::GameCode => 2,
        SourceCategory::GameLib => 3,
        SourceCategory::Core => 4,
        SourceCategory::Generated => 5,
        SourceCategory::Workbench => 6,
        SourceCategory::DocsDoxygen => 7,
        SourceCategory::TestAutotest => 8,
        SourceCategory::Unknown => 9,
    }
}

fn source_category_from_tag(tag: u8) -> Result<SourceCategory, String> {
    match tag {
        0 => Ok(SourceCategory::Workspace),
        1 => Ok(SourceCategory::Game),
        2 => Ok(SourceCategory::GameCode),
        3 => Ok(SourceCategory::GameLib),
        4 => Ok(SourceCategory::Core),
        5 => Ok(SourceCategory::Generated),
        6 => Ok(SourceCategory::Workbench),
        7 => Ok(SourceCategory::DocsDoxygen),
        8 => Ok(SourceCategory::TestAutotest),
        9 => Ok(SourceCategory::Unknown),
        _ => Err(format!("invalid source category tag {tag}")),
    }
}

fn semantic_declaration_kind_tag(kind: SemanticDeclarationKind) -> u8 {
    match kind {
        SemanticDeclarationKind::Class => 0,
        SemanticDeclarationKind::TypeParameter => 1,
        SemanticDeclarationKind::Enum => 2,
        SemanticDeclarationKind::EnumMember => 3,
        SemanticDeclarationKind::Typedef => 4,
        SemanticDeclarationKind::Function => 5,
        SemanticDeclarationKind::GlobalField => 6,
        SemanticDeclarationKind::Field => 7,
        SemanticDeclarationKind::Method => 8,
        SemanticDeclarationKind::Constructor => 9,
        SemanticDeclarationKind::Destructor => 10,
        SemanticDeclarationKind::Parameter => 11,
        SemanticDeclarationKind::LocalVariable => 12,
        SemanticDeclarationKind::PreprocessorMacro => 13,
    }
}

fn semantic_declaration_kind_from_tag(tag: u8) -> Result<SemanticDeclarationKind, String> {
    Ok(match tag {
        0 => SemanticDeclarationKind::Class,
        1 => SemanticDeclarationKind::TypeParameter,
        2 => SemanticDeclarationKind::Enum,
        3 => SemanticDeclarationKind::EnumMember,
        4 => SemanticDeclarationKind::Typedef,
        5 => SemanticDeclarationKind::Function,
        6 => SemanticDeclarationKind::GlobalField,
        7 => SemanticDeclarationKind::Field,
        8 => SemanticDeclarationKind::Method,
        9 => SemanticDeclarationKind::Constructor,
        10 => SemanticDeclarationKind::Destructor,
        11 => SemanticDeclarationKind::Parameter,
        12 => SemanticDeclarationKind::LocalVariable,
        13 => SemanticDeclarationKind::PreprocessorMacro,
        _ => return Err(format!("invalid semantic declaration kind tag {tag}")),
    })
}

fn semantic_doc_comment_kind_tag(kind: SemanticDocCommentKind) -> u8 {
    match kind {
        SemanticDocCommentKind::Line => 0,
        SemanticDocCommentKind::Block => 1,
    }
}

fn semantic_doc_comment_kind_from_tag(tag: u8) -> Result<SemanticDocCommentKind, String> {
    match tag {
        0 => Ok(SemanticDocCommentKind::Line),
        1 => Ok(SemanticDocCommentKind::Block),
        _ => Err(format!("invalid semantic doc comment kind tag {tag}")),
    }
}

fn semantic_conditional_branch_kind_tag(kind: SemanticConditionalBranchKind) -> u8 {
    match kind {
        SemanticConditionalBranchKind::If => 0,
        SemanticConditionalBranchKind::Ifdef => 1,
        SemanticConditionalBranchKind::Ifndef => 2,
        SemanticConditionalBranchKind::Elif => 3,
        SemanticConditionalBranchKind::Else => 4,
    }
}

fn semantic_conditional_branch_kind_from_tag(
    tag: u8,
) -> Result<SemanticConditionalBranchKind, String> {
    match tag {
        0 => Ok(SemanticConditionalBranchKind::If),
        1 => Ok(SemanticConditionalBranchKind::Ifdef),
        2 => Ok(SemanticConditionalBranchKind::Ifndef),
        3 => Ok(SemanticConditionalBranchKind::Elif),
        4 => Ok(SemanticConditionalBranchKind::Else),
        _ => Err(format!(
            "invalid semantic conditional branch kind tag {tag}"
        )),
    }
}

fn semantic_callable_form_tag(form: SemanticCallableForm) -> u8 {
    match form {
        SemanticCallableForm::Implementation => 0,
        SemanticCallableForm::Declaration => 1,
        SemanticCallableForm::Prototype => 2,
    }
}

fn semantic_callable_form_from_tag(tag: u8) -> Result<SemanticCallableForm, String> {
    match tag {
        0 => Ok(SemanticCallableForm::Implementation),
        1 => Ok(SemanticCallableForm::Declaration),
        2 => Ok(SemanticCallableForm::Prototype),
        _ => Err(format!("invalid semantic callable form tag {tag}")),
    }
}

fn cache_rebuild_reason(cache_path: &Path, fingerprint: &SourceFingerprint) -> String {
    if !cache_path.is_file() {
        return "cache-missing".to_string();
    }
    format!(
        "cache-stale-or-incompatible fingerprint={}",
        fingerprint.summary()
    )
}

#[cfg(test)]
fn source_fingerprint(
    scripts_root: &Path,
    metadata_path: Option<&Path>,
) -> Result<SourceFingerprint, String> {
    source_fingerprint_with_control(scripts_root, metadata_path, &IndexBuildControl::default())
}

fn source_fingerprint_with_control(
    scripts_root: &Path,
    metadata_path: Option<&Path>,
    control: &IndexBuildControl,
) -> Result<SourceFingerprint, String> {
    control.check()?;
    let scripts_root = scripts_root
        .canonicalize()
        .unwrap_or_else(|_| scripts_root.to_path_buf())
        .to_string_lossy()
        .to_string();

    if let Some(metadata_path) = metadata_path {
        if let Some(commit_sha) = read_commit_sha(metadata_path)? {
            return Ok(SourceFingerprint::Downloaded {
                scripts_root,
                commit_sha,
            });
        }
    }

    let manual = manual_folder_fingerprint_with_control(Path::new(&scripts_root), control)?;
    Ok(SourceFingerprint::Manual {
        scripts_root,
        file_count: manual.file_count,
        byte_count: manual.byte_count,
        latest_modified_unix_ms: manual.latest_modified_unix_ms,
    })
}

pub(crate) fn source_content_digest(
    root: &Path,
    control: &IndexBuildControl,
) -> Result<String, String> {
    let mut files = Vec::new();
    collect_source_digest_files(root, &mut files, control)?;
    files.sort();

    let mut hasher = Sha256::new();
    hasher.update(b"reforger-game-data-source-v1\0");
    for file in files {
        control.check()?;
        let relative = file.strip_prefix(root).unwrap_or(&file);
        let logical_path = relative.to_string_lossy().replace('\\', "/");
        hasher.update((logical_path.len() as u64).to_le_bytes());
        hasher.update(logical_path.as_bytes());
        let bytes = fs::read(&file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        hasher.update((bytes.len() as u64).to_le_bytes());
        for chunk in bytes.chunks(64 * 1024) {
            control.check()?;
            hasher.update(chunk);
        }
    }
    Ok(hex_digest(hasher.finalize()))
}

fn collect_source_digest_files(
    folder: &Path,
    files: &mut Vec<PathBuf>,
    control: &IndexBuildControl,
) -> Result<(), String> {
    control.check()?;
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Failed to read folder {}: {error}", folder.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to read entry in {}: {error}", folder.display()))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Failed to read metadata for {}: {error}", path.display()))?;
        if metadata.is_dir() {
            collect_source_digest_files(&path, files, control)?;
        } else if metadata.is_file() && path.extension().is_some_and(|extension| extension == "c") {
            files.push(path);
        }
        control.check()?;
    }
    Ok(())
}

fn read_commit_sha(metadata_path: &Path) -> Result<Option<String>, String> {
    if !metadata_path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(metadata_path).map_err(|error| {
        format!(
            "Failed to read game-data metadata {}: {error}",
            metadata_path.display()
        )
    })?;
    let json = serde_json::from_str::<Value>(&raw).map_err(|error| {
        format!(
            "Failed to parse game-data metadata {}: {error}",
            metadata_path.display()
        )
    })?;
    Ok(json
        .get("commitSha")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManualFolderFingerprint {
    file_count: usize,
    byte_count: u64,
    latest_modified_unix_ms: u128,
}

fn manual_folder_fingerprint_with_control(
    root: &Path,
    control: &IndexBuildControl,
) -> Result<ManualFolderFingerprint, String> {
    let mut fingerprint = ManualFolderFingerprint {
        file_count: 0,
        byte_count: 0,
        latest_modified_unix_ms: 0,
    };
    collect_manual_fingerprint(root, &mut fingerprint, control)?;
    Ok(fingerprint)
}

fn collect_manual_fingerprint(
    folder: &Path,
    fingerprint: &mut ManualFolderFingerprint,
    control: &IndexBuildControl,
) -> Result<(), String> {
    control.check()?;
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Failed to read folder {}: {error}", folder.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to read entry in {}: {error}", folder.display()))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Failed to read metadata for {}: {error}", path.display()))?;
        if metadata.is_dir() {
            collect_manual_fingerprint(&path, fingerprint, control)?;
        } else if metadata.is_file() && path.extension().is_some_and(|extension| extension == "c") {
            fingerprint.file_count += 1;
            fingerprint.byte_count += metadata.len();
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis())
                .unwrap_or(0);
            fingerprint.latest_modified_unix_ms = fingerprint.latest_modified_unix_ms.max(modified);
        }
        control.check()?;
    }
    Ok(())
}

fn summary_from_build(summary: &IndexBuildSummary) -> RuntimeIndexSummary {
    RuntimeIndexSummary {
        files: summary.totals.files,
        bytes: summary.totals.bytes,
        indexed_symbols: summary.totals.indexed_symbols,
        parse_diagnostics: summary.totals.parse_diagnostics,
        lossy_files: summary.totals.lossy_files,
    }
}

fn summary_from_build_with_cached_index(
    summary: &IndexBuildSummary,
    cached_index: &SymbolIndex,
) -> RuntimeIndexSummary {
    RuntimeIndexSummary {
        indexed_symbols: cached_index.symbols().len(),
        ..summary_from_build(summary)
    }
}

impl SourceFingerprint {
    pub fn summary(&self) -> String {
        match self {
            Self::Downloaded { commit_sha, .. } => format!("downloaded:{commit_sha}"),
            Self::Manual {
                file_count,
                byte_count,
                latest_modified_unix_ms,
                ..
            } => format!(
                "manual:files={file_count}:bytes={byte_count}:modified={latest_modified_unix_ms}"
            ),
            Self::Addon {
                guid,
                artifact_digest,
                pack_count,
                catalogue_entry_count,
            } => format!(
                "addon:{guid}:packs={pack_count}:entries={catalogue_entry_count}:artifact={artifact_digest}"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SymbolKind;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cache_loads_when_fingerprint_matches() {
        let root = test_root("loads");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        write_file(
            &scripts.join("Game/Example.c"),
            "class Example { int m_Value; void Run(int value); }",
        );
        write_file(
            &root.join("metadata.json"),
            r#"{"commitSha":"abc123","fileCount":1}"#,
        );

        let first = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(root.join("metadata.json")),
        })
        .unwrap();
        assert!(matches!(
            first.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        let second = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: Some(root.join("metadata.json")),
        })
        .unwrap();
        assert_eq!(second.cache_status, IndexCacheStatus::Loaded);
        assert_eq!(second.summary.indexed_symbols, 4);
        assert_eq!(second.index.classes_by_name("Example").len(), 1);
        assert_eq!(
            second.index.methods_by_owner_name("Example", "Run").len(),
            1
        );
        assert!(second.timings.cache_read_deserialize_validate > std::time::Duration::ZERO);
        assert!(second.timings.total > std::time::Duration::ZERO);
        assert!(second.cache_file_bytes.unwrap_or_default() > 0);
        let cache_bytes = fs::read(&cache).unwrap();
        assert!(cache_bytes.starts_with(CACHE_MAGIC));
        let decoded = decode_cached_index(&cache_bytes).unwrap();
        assert_eq!(decoded.format_version, CACHE_FORMAT_VERSION);
        assert_eq!(decoded.index_shape, CACHE_INDEX_SHAPE);
        assert_eq!(decoded.files.len(), 1);
        decoded.validate().unwrap();
        assert_eq!(
            encode_cached_index(&decoded).unwrap(),
            index_cache_payload_from_bytes(&cache_bytes).unwrap()
        );

        cleanup(&root);
    }

    #[test]
    fn cache_consumer_loads_the_parser_owned_snapshot_without_source_validation() {
        let root = test_root("consumer-load");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        let source = scripts.join("Game/Example.c");
        write_file(&source, "class CachedExample {}\n");

        load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: None,
        })
        .unwrap();
        write_file(&source, "class ChangedAfterIndexBuild {}\n");

        let loaded = load_game_data_index_cache_with_control(&cache, &IndexBuildControl::default())
            .unwrap()
            .expect("parser-owned cache is available");

        assert!(matches!(loaded.cache_status, IndexCacheStatus::Loaded));
        assert!(loaded
            .index
            .symbols()
            .iter()
            .any(|symbol| symbol.name.as_deref() == Some("CachedExample")));
        assert_eq!(
            loaded.source_line_starts.get(&SourceFileId(0)),
            Some(&vec![0, 23])
        );
        assert!(!loaded
            .index
            .symbols()
            .iter()
            .any(|symbol| symbol.name.as_deref() == Some("ChangedAfterIndexBuild")));
        cleanup(&root);
    }

    #[test]
    fn concurrent_cold_builders_publish_one_valid_cache_without_failing_losers() {
        let root = test_root("concurrent_cold_publish");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        for index in 0..32 {
            write_file(
                &scripts.join(format!("Game/Fixture{index}.c")),
                &format!("class ConcurrentFixture{index} {{ int m_Value; }}"),
            );
        }
        let workers = 12;
        let barrier = Arc::new(std::sync::Barrier::new(workers));
        let handles = (0..workers)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let config = GameDataIndexCacheConfig {
                    scripts_root: scripts.clone(),
                    cache_path: cache.clone(),
                    metadata_path: None,
                };
                std::thread::spawn(move || {
                    barrier.wait();
                    load_or_build_game_data_index(&config)
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            let result = handle.join().expect("cache builder did not panic");
            assert!(
                result.is_ok(),
                "a losing cache publisher must keep its valid in-memory index: {result:?}"
            );
        }
        let loaded = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: None,
        })
        .expect("winning cache remains valid");
        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert_eq!(loaded.summary.files, 32);

        cleanup(&root);
    }

    #[test]
    fn cache_rebuilds_when_commit_changes() {
        let root = test_root("commit_changes");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(&scripts.join("Example.c"), "class Example {}");
        write_file(&metadata, r#"{"commitSha":"one"}"#);

        let _ = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(metadata.clone()),
        })
        .unwrap();
        write_file(&metadata, r#"{"commitSha":"two"}"#);

        let rebuilt = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: Some(metadata),
        })
        .unwrap();

        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        cleanup(&root);
    }

    #[test]
    fn cache_rebuild_writes_runtime_pruned_index() {
        let root = test_root("pruned");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(
            &scripts.join("Game/Example.c"),
            r#"class Example
{
	ref array<int> m_Values;

	int Run(string name = "ok")
	{
		int localValue = 1;
		return localValue;
	}
}
"#,
        );
        write_file(&metadata, r#"{"commitSha":"abc123"}"#);

        let result = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(metadata.clone()),
        })
        .unwrap();

        assert!(matches!(
            result.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert!(result
            .index
            .symbols_for_kind(SymbolKind::LocalVariable)
            .is_empty());
        assert_eq!(
            result.index.symbols_for_kind(SymbolKind::Parameter).len(),
            1
        );
        assert_eq!(result.index.symbols_for_name("localValue").len(), 0);
        let field = result
            .index
            .symbol(result.index.fields_by_owner_name("Example", "m_Values")[0])
            .unwrap();
        assert_eq!(field.detail.type_text.as_deref(), Some("ref array<int>"));
        assert!(field.detail.type_text_span.is_none());
        assert_eq!(
            result
                .index
                .callable_signature(result.index.methods_by_owner_name("Example", "Run")[0])
                .as_deref(),
            Some("Example.Run(string name = \"ok\") -> int")
        );
        let parameter = result.index.symbols_for_name("name")[0];
        let parameter = result.index.symbol(parameter).unwrap();
        assert_eq!(parameter.detail.type_text.as_deref(), Some("string"));
        assert_eq!(parameter.detail.default_text.as_deref(), Some("\"ok\""));
        assert!(parameter.detail.type_text_span.is_none());
        assert!(parameter.detail.default_text_span.is_none());

        let loaded = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: Some(metadata),
        })
        .unwrap();
        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert!(loaded
            .index
            .symbols_for_kind(SymbolKind::LocalVariable)
            .is_empty());
        assert_eq!(
            loaded.index.symbols_for_kind(SymbolKind::Parameter).len(),
            1
        );
        assert_eq!(loaded.index.classes_by_name("Example").len(), 1);
        assert_eq!(
            loaded
                .index
                .fields_by_owner_name("Example", "m_Values")
                .len(),
            1
        );
        assert_eq!(
            loaded.index.methods_by_owner_name("Example", "Run").len(),
            1
        );
        assert_eq!(
            loaded
                .index
                .callable_signature(loaded.index.methods_by_owner_name("Example", "Run")[0])
                .as_deref(),
            Some("Example.Run(string name = \"ok\") -> int")
        );

        cleanup(&root);
    }

    #[test]
    fn cache_preserves_public_symbols_after_pruned_local_ids() {
        const SOURCE: &str =
            include_str!("../../tools/fixtures/index/contribution_public_ids_after_local.c");
        let root = test_root("public-ids-after-local");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Game/Contribution.c"), SOURCE);

        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: None,
        };
        let rebuilt = load_or_build_game_data_index(&config).unwrap();
        let later = rebuilt
            .index
            .classes_by_name("ContributionIdsAfterPublicFixture")[0];
        assert_eq!(later.symbol_id.0, 2);
        assert_eq!(
            rebuilt
                .index
                .symbol(later)
                .and_then(|symbol| symbol.name.as_deref()),
            Some("ContributionIdsAfterPublicFixture")
        );

        let loaded = load_or_build_game_data_index(&config).unwrap();
        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        let later = loaded
            .index
            .classes_by_name("ContributionIdsAfterPublicFixture")[0];
        assert_eq!(later.symbol_id.0, 2);
        assert_eq!(
            loaded
                .index
                .symbol(later)
                .and_then(|symbol| symbol.name.as_deref()),
            Some("ContributionIdsAfterPublicFixture")
        );

        cleanup(&root);
    }

    #[test]
    fn cache_codec_derives_dense_public_ids_from_record_order() {
        const SOURCE: &str =
            include_str!("../../tools/fixtures/index/contribution_public_ids_after_local.c");
        let root = test_root("sparse-public-ids");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Game/Contribution.c"), SOURCE);

        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: None,
        };
        load_or_build_game_data_index(&config).unwrap();

        let mut decoded = decode_cached_index(&fs::read(&cache).unwrap()).unwrap();
        decoded.files[0].symbols[2].id = SemanticDeclarationId(3);
        fs::write(&cache, encode_cached_index(&decoded).unwrap()).unwrap();

        let loaded = load_or_build_game_data_index(&config).unwrap();
        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert_eq!(
            loaded
                .index
                .classes_by_name("ContributionIdsAfterPublicFixture")[0]
                .symbol_id
                .0,
            2
        );
        cleanup(&root);
    }

    #[test]
    fn retired_cache_formats_trigger_rebuild_without_decoding_their_payloads() {
        for (name, magic) in [("v9", LEGACY_CACHE_MAGIC), ("v10", V10_CACHE_MAGIC)] {
            let root = test_root(&format!("{name}_identity_rebuild"));
            let cache = root.join("cache.bin");
            let scripts = root.join("scripts");
            write_file(&scripts.join("Game/Example.c"), "class Example {}");
            fs::write(
                &cache,
                [magic.as_slice(), b"arbitrary retired payload"].concat(),
            )
            .unwrap();

            let rebuilt = load_or_build_game_data_index(&GameDataIndexCacheConfig {
                scripts_root: scripts,
                cache_path: cache.clone(),
                metadata_path: None,
            })
            .unwrap();

            assert!(matches!(
                rebuilt.cache_status,
                IndexCacheStatus::Rebuilt { .. }
            ));
            assert_eq!(rebuilt.index.classes_by_name("Example").len(), 1);
            assert!(fs::read(&cache).unwrap().starts_with(CACHE_MAGIC));
            cleanup(&root);
        }
    }
    #[test]
    fn current_cache_load_preserves_complete_lookup_projection() {
        let root = test_root("rebuild_maps");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(
            &scripts.join("Game/Example.c"),
            r#"typedef string FactionKey;

void GlobalFn(int value);

class Example : BaseExample
{
	int m_Value;
	void Run(string name);
}
"#,
        );
        write_file(&metadata, r#"{"commitSha":"abc123"}"#);

        let rebuilt = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(metadata.clone()),
        })
        .unwrap();
        let rebuilt_files = rebuilt.index.files().to_vec();
        let rebuilt_symbols = rebuilt.index.symbols().to_vec();
        let rebuilt_map_counts = rebuilt.index.map_counts();
        let rebuilt_prefix = rebuilt
            .index
            .top_level_symbols_with_ascii_case_insensitive_prefix("ex")
            .collect::<Vec<_>>();
        let rebuilt_method_groups = rebuilt.index.method_owner_name_groups().clone();

        let loaded = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: Some(metadata),
        })
        .unwrap();

        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert_eq!(loaded.index.files(), rebuilt_files);
        assert_eq!(loaded.index.symbols(), rebuilt_symbols);
        assert_eq!(loaded.index.map_counts(), rebuilt_map_counts);
        assert_eq!(
            loaded
                .index
                .top_level_symbols_with_ascii_case_insensitive_prefix("ex")
                .collect::<Vec<_>>(),
            rebuilt_prefix
        );
        assert_eq!(
            loaded.index.method_owner_name_groups(),
            &rebuilt_method_groups
        );
        assert_eq!(loaded.index.classes_by_name("Example").len(), 1);
        assert_eq!(loaded.index.typedefs_by_name("FactionKey").len(), 1);
        assert_eq!(loaded.index.functions_by_name("GlobalFn").len(), 1);
        assert_eq!(loaded.index.symbols_for_kind(SymbolKind::Class).len(), 1);
        assert_eq!(loaded.index.symbols_for_name("m_Value").len(), 1);
        assert_eq!(
            loaded
                .index
                .fields_by_owner_name("Example", "m_Value")
                .len(),
            1
        );
        assert_eq!(
            loaded.index.methods_by_owner_name("Example", "Run").len(),
            1
        );
        assert!(loaded.index.members_by_owner("Example").iter().any(|id| {
            loaded
                .index
                .symbol(*id)
                .is_some_and(|symbol| symbol.name.as_deref() == Some("m_Value"))
        }));

        let class = loaded.index.classes_by_name("Example")[0];
        assert!(!loaded.index.children(class).is_empty());
        assert_eq!(
            loaded.index.preferred_classes_by_name("Example"),
            vec![class]
        );
        assert_eq!(
            loaded
                .index
                .callable_signature(loaded.index.methods_by_owner_name("Example", "Run")[0])
                .as_deref(),
            Some("Example.Run(string name) -> void")
        );

        cleanup(&root);
    }

    #[test]
    fn cache_load_preserves_multi_file_symbol_ranges() {
        let root = test_root("multi_file_ranges");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(
            &scripts.join("Game/First.c"),
            r#"class First
{
	void Run()
	{
		int localValue;
	}
}
"#,
        );
        write_file(
            &scripts.join("Game/generated/GameMode/BaseGameMode.c"),
            r#"class GenericEntityClass {}

class BaseGameModeClass : GenericEntityClass
{
}
"#,
        );
        write_file(&metadata, r#"{"commitSha":"abc123"}"#);

        let _ = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(metadata.clone()),
        })
        .unwrap();

        let loaded = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: Some(metadata),
        })
        .unwrap();

        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert!(loaded
            .index
            .symbols_for_kind(SymbolKind::LocalVariable)
            .is_empty());

        let base_class_ids = loaded.index.classes_by_name("BaseGameModeClass");
        assert_eq!(base_class_ids.len(), 1);
        let base_class = loaded.index.symbol(base_class_ids[0]).unwrap();
        assert_eq!(base_class.name.as_deref(), Some("BaseGameModeClass"));
        assert_eq!(base_class.kind, SymbolKind::Class);
        assert_eq!(
            base_class.detail.base_type.as_deref(),
            Some("GenericEntityClass")
        );

        let generic_class_ids = loaded.index.classes_by_name("GenericEntityClass");
        assert_eq!(generic_class_ids.len(), 1);
        let generic_class = loaded.index.symbol(generic_class_ids[0]).unwrap();
        assert_eq!(generic_class.name.as_deref(), Some("GenericEntityClass"));

        cleanup(&root);
    }

    #[test]
    fn cache_rebuilds_when_format_version_is_stale() {
        let root = test_root("format_version");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Example.c"), "class Example {}");

        let _ = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: None,
        })
        .unwrap();

        let mut decoded = decode_cached_index(&fs::read(&cache).unwrap()).unwrap();
        decoded.format_version = CACHE_FORMAT_VERSION - 1;
        fs::write(&cache, encode_cached_index(&decoded).unwrap()).unwrap();

        let rebuilt = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: None,
        })
        .unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        cleanup(&root);
    }

    #[test]
    fn cache_rebuilds_when_a_public_contribution_is_not_current() {
        let root = test_root("stale-contribution");
        let scripts = root.join("scripts");
        let cache = root.join("index.bin");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(scripts.join("Example.c"), "class Example {}\n").unwrap();

        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: None,
        };
        load_or_build_game_data_index(&config).unwrap();

        let mut decoded = decode_cached_index(&fs::read(&cache).unwrap()).unwrap();
        decoded.files[0].symbols[0].name.clear();
        fs::write(&cache, encode_cached_index(&decoded).unwrap()).unwrap();

        let rebuilt = load_or_build_game_data_index(&config).unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        cleanup(&root);
    }

    #[test]
    fn cache_rebuilds_when_index_shape_is_stale() {
        let root = test_root("index_shape");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Example.c"), "class Example {}");

        let _ = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: None,
        })
        .unwrap();

        let mut decoded = decode_cached_index(&fs::read(&cache).unwrap()).unwrap();
        decoded.index_shape = "old-index-shape".to_string();
        fs::write(&cache, encode_cached_index(&decoded).unwrap()).unwrap();

        let rebuilt = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: None,
        })
        .unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        cleanup(&root);
    }

    #[test]
    fn corrupt_cache_rebuilds_without_failing() {
        let root = test_root("corrupt");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Example.c"), "class Example {}");
        fs::write(&cache, b"bad binary cache").unwrap();

        let result = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: None,
        })
        .unwrap();

        assert!(matches!(
            result.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert_eq!(result.index.classes_by_name("Example").len(), 1);
        assert!(result.timings.rebuild > std::time::Duration::ZERO);
        assert!(result.timings.cache_write > std::time::Duration::ZERO);
        assert!(result.timings.total > std::time::Duration::ZERO);

        cleanup(&root);
    }

    #[test]
    fn corrupt_binary_cache_lengths_do_not_allocate_unbounded_memory() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(CACHE_MAGIC);
        bytes.extend_from_slice(
            &(u32::try_from(MAX_CACHE_STRING_TABLE_ENTRIES).unwrap() + 1).to_le_bytes(),
        );

        let error = decode_cached_index(&bytes).unwrap_err();
        assert!(error.contains("string table entry length"));
        assert!(error.contains("exceeds safety limit"));
    }

    #[test]
    fn current_codec_bounds_cache_sized_integers_to_u32() {
        let table = CacheStringTable {
            ids: AHashMap::new(),
            values: Vec::new(),
        };
        let mut writer = BinaryWriter::new(table);
        assert_eq!(
            writer.write_usize(u32::MAX as usize + 1).unwrap_err(),
            "cache integer exceeds u32"
        );
    }

    #[test]
    fn current_codec_rejects_unknown_symbol_flags() {
        let bytes = (CACHED_SYMBOL_KNOWN_FLAGS + 1).to_le_bytes();
        let mut reader = BinaryReader::new(&bytes);
        assert_eq!(
            reader
                .read_cached_public_symbol(SemanticDeclarationId(0))
                .unwrap_err(),
            "invalid cached public symbol flags 0x0800"
        );
    }

    #[test]
    fn manual_fingerprint_changes_when_files_change() {
        let root = test_root("manual_fingerprint");
        let scripts = root.join("scripts");
        write_file(&scripts.join("A.c"), "class A {}");
        let first = source_fingerprint(&scripts, None).unwrap();
        write_file(&scripts.join("B.c"), "class B {}");
        let second = source_fingerprint(&scripts, None).unwrap();

        assert_ne!(first, second);

        cleanup(&root);
    }

    #[test]
    fn catalogue_digest_tracks_semantic_facts_not_commit_or_physical_root_alone() {
        let root = test_root("catalogue_digest");
        let left_scripts = root.join("left/scripts");
        let right_scripts = root.join("right/scripts");
        for scripts in [&left_scripts, &right_scripts] {
            write_file(
                &scripts.join("Game/Same.c"),
                "class SameAA { void Run() { int value = 1; } }",
            );
        }
        let metadata = root.join("download.json");
        write_file(&metadata, r#"{"commitSha":"same-commit"}"#);

        let build = |scripts: &Path, cache_name: &str| {
            load_or_build_game_data_index(&GameDataIndexCacheConfig {
                scripts_root: scripts.to_path_buf(),
                cache_path: root.join(cache_name),
                metadata_path: Some(metadata.clone()),
            })
            .unwrap()
            .catalogue_digest
        };
        let left = build(&left_scripts, "left.bin");
        let right = build(&right_scripts, "right.bin");
        assert_eq!(left, right, "installed location is not catalogue identity");

        write_file(
            &left_scripts.join("Game/Same.c"),
            "class SameAA { void Run() { int value = 2; } }",
        );
        let changed_result = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: left_scripts,
            cache_path: root.join("left.bin"),
            metadata_path: Some(metadata),
        })
        .unwrap();
        assert!(matches!(
            changed_result.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert_ne!(
            left, changed_result.catalogue_digest,
            "same-size body-only changes under one commit identity must change the catalogue"
        );
        cleanup(&root);
    }

    fn test_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "reforger_index_cache_{name}_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_file(path: &Path, source: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, source).unwrap();
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }
}
