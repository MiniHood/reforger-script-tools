use crate::ast::DocCommentKind;
use crate::index::{
    GlobalSymbolId, IndexedAttribute, IndexedConditionalBranch, IndexedDocComment, IndexedFile,
    IndexedSymbol, IndexedSymbolDetail, SourceFileId, SymbolIndex,
};
use crate::index_build::{build_index, IndexBuildConfig, IndexBuildResult, IndexSourceRoot};
use crate::lexer::TextSpan;
use crate::model::{
    CallableForm, PreprocessorBranchKind, SourceCategory, SourceFileMetadata, SourceKind, SymbolId,
    SymbolKind, SOURCE_PRIORITY_GAME_DATA,
};
use crate::semantic_file::{
    FileContribution, PublicSymbol, PublicSymbolDetail, SemanticDeclarationKind,
    FILE_CONTRIBUTION_SCHEMA_VERSION, FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

const CACHE_FORMAT_VERSION: u32 = 10;
const CACHE_SCHEMA: &str = "reforger-symbol-index";
const CACHE_MAGIC: &[u8; 8] = b"RSTIDX10";
const CACHE_INDEX_SHAPE: &str =
    "runtime-pruned:no-local-variables:detail-spans-stripped:layered-external-v1:binary-v3:string-table-v1:validated-file-contributions-v1";
const MAX_CACHE_STRING_TABLE_ENTRIES: usize = 1_000_000;
const MAX_CACHE_RAW_STRING_BYTES: usize = 16 * 1024 * 1024;
const MAX_CACHE_FILE_RECORDS: usize = 1_000_000;
const MAX_CACHE_SYMBOL_RECORDS: usize = 5_000_000;
const MAX_CACHE_SYMBOL_LIST_ITEMS: usize = 1_000_000;
const MAX_CACHE_CONTRIBUTION_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug)]
pub struct GameDataIndexCacheConfig {
    pub scripts_root: PathBuf,
    pub cache_path: PathBuf,
    pub metadata_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct GameDataIndexCacheResult {
    pub index: SymbolIndex,
    pub summary: RuntimeIndexSummary,
    pub cache_status: IndexCacheStatus,
    pub fingerprint: SourceFingerprint,
    pub timings: IndexCacheTimings,
    pub cache_file_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexCacheTimings {
    pub fingerprint: Duration,
    pub cache_file_read: Duration,
    pub cache_decode: Duration,
    pub cache_validate: Duration,
    pub map_rebuild: Duration,
    pub cache_read_deserialize_validate: Duration,
    pub rebuild: Duration,
    pub cache_write: Duration,
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
}

#[derive(Debug)]
struct CachedGameDataIndex {
    schema: String,
    format_version: u32,
    index_shape: String,
    crate_version: String,
    fingerprint: SourceFingerprint,
    summary: CachedIndexSummary,
    index: CachedSymbolIndex,
}

#[derive(Debug)]
struct CachedSymbolIndex {
    files: Vec<IndexedFile>,
    symbols: Vec<IndexedSymbol>,
    contributions: Vec<FileContribution>,
}

impl From<&SymbolIndex> for CachedSymbolIndex {
    fn from(index: &SymbolIndex) -> Self {
        let mut snapshot = Self {
            files: index.files().to_vec(),
            symbols: index.symbols().to_vec(),
            contributions: Vec::new(),
        };
        snapshot.contributions = snapshot.public_contributions();
        snapshot
    }
}

impl From<CachedSymbolIndex> for SymbolIndex {
    fn from(snapshot: CachedSymbolIndex) -> Self {
        SymbolIndex::from_indexed_parts(snapshot.files, snapshot.symbols)
    }
}

impl CachedSymbolIndex {
    /// Reconstruct the public records that this legacy query cache exposes and
    /// validate the versioned compiler contract before rebuilding lookup maps.
    /// The index remains a temporary query projection, never a cache-validity
    /// fallback for malformed public semantic data.
    fn validate_public_contributions(&self) -> Result<(), String> {
        for contribution in &self.contributions {
            contribution
                .validate()
                .map_err(|error| format!("invalid public file contribution: {error:?}"))?;
        }
        Ok(())
    }

    fn public_contributions(&self) -> Vec<FileContribution> {
        self.files
            .iter()
            .map(|file| FileContribution {
                schema_version: FILE_CONTRIBUTION_SCHEMA_VERSION,
                source_manifest_version: FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION,
                symbols: self
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.id.file_id == file.id)
                    .filter(|symbol| symbol.name.is_some())
                    .filter_map(|symbol| {
                        let kind = public_semantic_kind(symbol.kind)?;
                        Some(PublicSymbol {
                            kind,
                            name: symbol.name.clone(),
                            container: symbol.parent.and_then(|parent| {
                                self.symbols
                                    .iter()
                                    .find(|candidate| candidate.id == parent)
                                    .and_then(|candidate| candidate.name.clone())
                            }),
                            detail: PublicSymbolDetail {
                                type_text: symbol.detail.type_text.clone(),
                                return_type: symbol.detail.return_type_text.clone(),
                                base_type: symbol.detail.base_type.clone(),
                            },
                        })
                    })
                    .collect(),
            })
            .collect()
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
        SymbolKind::TypeParameter | SymbolKind::Parameter | SymbolKind::LocalVariable => {
            return None
        }
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
    load_or_build_game_data_index_with_progress(config, |_| {})
}

pub fn load_or_build_game_data_index_with_progress(
    config: &GameDataIndexCacheConfig,
    mut progress: impl FnMut(&str),
) -> Result<GameDataIndexCacheResult, String> {
    let total_start = Instant::now();
    let mut timings = IndexCacheTimings::default();
    progress("validate-scripts-root-start");
    if !config.scripts_root.is_dir() {
        return Err(format!(
            "Game-data scripts folder does not exist: {}",
            config.scripts_root.display()
        ));
    }
    progress("validate-scripts-root-end");

    progress("fingerprint-start");
    let fingerprint_start = Instant::now();
    let fingerprint = source_fingerprint(&config.scripts_root, config.metadata_path.as_deref())?;
    timings.fingerprint = fingerprint_start.elapsed();
    progress("fingerprint-end");
    let initial_cache_file_bytes = cache_file_bytes(&config.cache_path);

    progress("cache-load-start");
    let cache_read_start = Instant::now();
    match load_cached_index(&config.cache_path, &fingerprint, &mut timings) {
        Ok(Some(cached)) => {
            timings.cache_read_deserialize_validate = cache_read_start.elapsed();
            progress("cache-load-hit");
            progress("map-rebuild-start");
            let map_rebuild_start = Instant::now();
            let index = cached.index.into();
            timings.map_rebuild = map_rebuild_start.elapsed();
            progress("map-rebuild-end");
            timings.total = total_start.elapsed();
            return Ok(GameDataIndexCacheResult {
                index,
                summary: cached.summary.into(),
                cache_status: IndexCacheStatus::Loaded,
                fingerprint,
                timings,
                cache_file_bytes: initial_cache_file_bytes,
            });
        }
        Ok(None) | Err(_) => {
            timings.cache_read_deserialize_validate = cache_read_start.elapsed();
            progress("cache-load-miss");
        }
    }

    let rebuild_reason = cache_rebuild_reason(&config.cache_path, &fingerprint);
    progress("source-rebuild-start");
    let rebuild_start = Instant::now();
    let built = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &config.scripts_root,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })?;
    timings.rebuild = rebuild_start.elapsed();
    progress("source-rebuild-end");
    let cached_index = built.index.compact_for_runtime_cache();
    let summary = summary_from_build_with_cached_index(&built, &cached_index);

    progress("cache-write-start");
    let cache_write_start = Instant::now();
    write_cached_index(&config.cache_path, &fingerprint, &summary, &cached_index)?;
    timings.cache_write = cache_write_start.elapsed();
    progress("cache-write-end");
    timings.total = total_start.elapsed();
    let cache_file_bytes = cache_file_bytes(&config.cache_path);

    Ok(GameDataIndexCacheResult {
        index: cached_index,
        summary,
        cache_status: IndexCacheStatus::Rebuilt {
            reason: rebuild_reason,
        },
        fingerprint,
        timings,
        cache_file_bytes,
    })
}

fn cache_file_bytes(cache_path: &Path) -> Option<u64> {
    cache_path.metadata().ok().map(|metadata| metadata.len())
}

fn load_cached_index(
    cache_path: &Path,
    expected_fingerprint: &SourceFingerprint,
    timings: &mut IndexCacheTimings,
) -> Result<Option<CachedGameDataIndex>, String> {
    if !cache_path.is_file() {
        return Ok(None);
    }

    let read_start = Instant::now();
    let mut file = fs::File::open(cache_path).map_err(|error| {
        format!(
            "Failed to open index cache {}: {error}",
            cache_path.display()
        )
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| {
        format!(
            "Failed to read index cache {}: {error}",
            cache_path.display()
        )
    })?;
    timings.cache_file_read = read_start.elapsed();
    let decode_start = Instant::now();
    let cached = decode_cached_index(&bytes).map_err(|error| {
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
        || cached.fingerprint != *expected_fingerprint
    {
        timings.cache_validate = validate_start.elapsed();
        return Ok(None);
    }
    if cached.index.validate_public_contributions().is_err() {
        timings.cache_validate = validate_start.elapsed();
        return Ok(None);
    }
    timings.cache_validate = validate_start.elapsed();

    Ok(Some(cached))
}

fn write_cached_index(
    cache_path: &Path,
    fingerprint: &SourceFingerprint,
    summary: &RuntimeIndexSummary,
    index: &SymbolIndex,
) -> Result<(), String> {
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create index cache folder {}: {error}",
                parent.display()
            )
        })?;
    }

    let temp_path = unique_cache_temp_path(cache_path);
    let file = fs::File::create(&temp_path).map_err(|error| {
        format!(
            "Failed to create temporary index cache {}: {error}",
            temp_path.display()
        )
    })?;
    let cached = CachedGameDataIndex {
        schema: CACHE_SCHEMA.to_string(),
        format_version: CACHE_FORMAT_VERSION,
        index_shape: CACHE_INDEX_SHAPE.to_string(),
        crate_version: env!("CARGO_PKG_VERSION").to_string(),
        fingerprint: fingerprint.clone(),
        summary: CachedIndexSummary::from(summary),
        index: CachedSymbolIndex::from(index),
    };
    let bytes = encode_cached_index(&cached)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(&bytes).map_err(|error| {
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
    if cache_path.exists() {
        fs::remove_file(cache_path).map_err(|error| {
            format!(
                "Failed to remove stale index cache {}: {error}",
                cache_path.display()
            )
        })?;
    }
    fs::rename(&temp_path, cache_path).map_err(|error| {
        format!(
            "Failed to replace index cache {} with {}: {error}",
            cache_path.display(),
            temp_path.display()
        )
    })
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
    writer.write_summary(&cached.summary);
    writer.write_vec_len(cached.index.files.len())?;
    for file in &cached.index.files {
        writer.write_indexed_file(file)?;
    }
    writer.write_vec_len(cached.index.symbols.len())?;
    for symbol in &cached.index.symbols {
        writer.write_indexed_symbol(symbol)?;
    }
    let contribution_bytes = serde_json::to_vec(&cached.index.contributions)
        .map_err(|error| format!("Failed to encode file contributions: {error}"))?;
    writer.write_vec_len(contribution_bytes.len())?;
    writer.write_bytes(&contribution_bytes);
    Ok(writer.into_bytes())
}

fn decode_cached_index(bytes: &[u8]) -> Result<CachedGameDataIndex, String> {
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
    let summary = reader.read_summary()?;
    let file_count = reader.read_bounded_len("file records", MAX_CACHE_FILE_RECORDS)?;
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        files.push(reader.read_indexed_file()?);
    }
    let symbol_count = reader.read_bounded_len("symbol records", MAX_CACHE_SYMBOL_RECORDS)?;
    let mut symbols = Vec::with_capacity(symbol_count);
    for _ in 0..symbol_count {
        symbols.push(reader.read_indexed_symbol()?);
    }
    let contribution_len =
        reader.read_bounded_len("public contribution bytes", MAX_CACHE_CONTRIBUTION_BYTES)?;
    let contributions = serde_json::from_slice(reader.read_exact(contribution_len)?)
        .map_err(|error| format!("invalid public file contributions: {error}"))?;
    reader.expect_eof()?;
    Ok(CachedGameDataIndex {
        schema,
        format_version,
        index_shape,
        crate_version,
        fingerprint,
        summary,
        index: CachedSymbolIndex {
            files,
            symbols,
            contributions,
        },
    })
}

struct CacheStringTable {
    ids: BTreeMap<String, u32>,
    values: Vec<String>,
}

impl CacheStringTable {
    fn from_cached_index(cached: &CachedGameDataIndex) -> Result<Self, String> {
        let mut table = Self {
            ids: BTreeMap::new(),
            values: Vec::new(),
        };
        table.insert(&cached.schema)?;
        table.insert(&cached.index_shape)?;
        table.insert(&cached.crate_version)?;
        table.insert_fingerprint(&cached.fingerprint)?;
        for file in &cached.index.files {
            table.insert_metadata(&file.metadata)?;
        }
        for symbol in &cached.index.symbols {
            table.insert_symbol(symbol)?;
        }
        Ok(table)
    }

    fn insert(&mut self, value: &str) -> Result<(), String> {
        if self.ids.contains_key(value) {
            return Ok(());
        }
        let id = u32::try_from(self.values.len())
            .map_err(|_| "cache string table exceeds u32 entries".to_string())?;
        self.values.push(value.to_string());
        self.ids.insert(value.to_string(), id);
        Ok(())
    }

    fn insert_option(&mut self, value: Option<&str>) -> Result<(), String> {
        if let Some(value) = value {
            self.insert(value)?;
        }
        Ok(())
    }

    fn insert_path(&mut self, value: &Path) -> Result<(), String> {
        self.insert(&value.to_string_lossy())
    }

    fn insert_option_path(&mut self, value: Option<&Path>) -> Result<(), String> {
        if let Some(value) = value {
            self.insert_path(value)?;
        }
        Ok(())
    }

    fn insert_fingerprint(&mut self, fingerprint: &SourceFingerprint) -> Result<(), String> {
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
        }
        Ok(())
    }

    fn insert_metadata(&mut self, metadata: &SourceFileMetadata) -> Result<(), String> {
        self.insert_option_path(metadata.absolute_path.as_deref())?;
        self.insert_option_path(metadata.root_path.as_deref())?;
        self.insert_option_path(metadata.relative_path.as_deref())
    }

    fn insert_symbol(&mut self, symbol: &IndexedSymbol) -> Result<(), String> {
        self.insert_option(symbol.name.as_deref())?;
        self.insert_detail(&symbol.detail)?;
        for attribute in &symbol.attributes {
            self.insert_option(attribute.name.as_deref())?;
            self.insert(&attribute.text)?;
        }
        for modifier in &symbol.modifiers {
            self.insert(modifier)?;
        }
        for doc_comment in &symbol.doc_comments {
            self.insert(&doc_comment.text)?;
        }
        for branch in &symbol.conditional_context {
            self.insert_option(branch.condition.as_deref())?;
        }
        Ok(())
    }

    fn insert_detail(&mut self, detail: &IndexedSymbolDetail) -> Result<(), String> {
        self.insert_option(detail.type_text.as_deref())?;
        self.insert_option(detail.return_type_text.as_deref())?;
        self.insert_option(detail.base_type.as_deref())?;
        self.insert_option(detail.default_text.as_deref())?;
        self.insert_option(detail.enum_value_text.as_deref())
    }

    fn id(&self, value: &str) -> Result<u32, String> {
        self.ids
            .get(value)
            .copied()
            .ok_or_else(|| format!("cache string was not interned before write: {value:?}"))
    }
}

struct BinaryWriter {
    bytes: Vec<u8>,
    string_table: CacheStringTable,
}

impl BinaryWriter {
    fn new(string_table: CacheStringTable) -> Self {
        Self {
            bytes: Vec::new(),
            string_table,
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
        let value = u64::try_from(value).map_err(|_| "usize value exceeds u64".to_string())?;
        self.write_u64(value);
        Ok(())
    }

    fn write_vec_len(&mut self, len: usize) -> Result<(), String> {
        self.write_usize(len)
    }

    fn write_raw_string(&mut self, value: &str) -> Result<(), String> {
        self.write_vec_len(value.len())?;
        self.write_bytes(value.as_bytes());
        Ok(())
    }

    fn write_string_table(&mut self) -> Result<(), String> {
        let values = self.string_table.values.clone();
        self.write_vec_len(values.len())?;
        for value in &values {
            self.write_raw_string(value)?;
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

    fn write_option_span(&mut self, value: Option<TextSpan>) -> Result<(), String> {
        match value {
            Some(value) => {
                self.write_u8(1);
                self.write_span(value)?;
            }
            None => self.write_u8(0),
        }
        Ok(())
    }

    fn write_span(&mut self, span: TextSpan) -> Result<(), String> {
        self.write_usize(span.start)?;
        self.write_usize(span.end)
    }

    fn write_global_id(&mut self, id: GlobalSymbolId) -> Result<(), String> {
        self.write_usize(id.file_id.0)?;
        self.write_usize(id.symbol_id.0)
    }

    fn write_option_global_id(&mut self, id: Option<GlobalSymbolId>) -> Result<(), String> {
        match id {
            Some(id) => {
                self.write_u8(1);
                self.write_global_id(id)?;
            }
            None => self.write_u8(0),
        }
        Ok(())
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
        self.write_option_path(metadata.root_path.as_deref())?;
        self.write_option_path(metadata.relative_path.as_deref())?;
        self.write_u16(metadata.priority);
        Ok(())
    }

    fn write_indexed_file(&mut self, file: &IndexedFile) -> Result<(), String> {
        self.write_usize(file.id.0)?;
        self.write_metadata(&file.metadata)?;
        self.write_usize(file.symbol_start)?;
        self.write_usize(file.symbol_count)?;
        self.write_usize(file.non_declaration_callable_fragments)
    }

    fn write_detail(&mut self, detail: &IndexedSymbolDetail) -> Result<(), String> {
        self.write_option_string(detail.type_text.as_deref())?;
        self.write_option_span(detail.type_text_span)?;
        self.write_option_string(detail.return_type_text.as_deref())?;
        self.write_option_span(detail.return_type_text_span)?;
        self.write_option_string(detail.base_type.as_deref())?;
        self.write_option_span(detail.base_type_span)?;
        self.write_option_string(detail.default_text.as_deref())?;
        self.write_option_span(detail.default_text_span)?;
        self.write_option_string(detail.enum_value_text.as_deref())?;
        self.write_option_span(detail.enum_value_text_span)
    }

    fn write_indexed_symbol(&mut self, symbol: &IndexedSymbol) -> Result<(), String> {
        self.write_global_id(symbol.id)?;
        self.write_option_global_id(symbol.parent)?;
        self.write_u8(symbol_kind_tag(symbol.kind));
        self.write_option_string(symbol.name.as_deref())?;
        self.write_span(symbol.span)?;
        self.write_span(symbol.selection_span)?;
        self.write_detail(&symbol.detail)?;
        self.write_vec_len(symbol.attributes.len())?;
        for attribute in &symbol.attributes {
            self.write_attribute(attribute)?;
        }
        self.write_vec_len(symbol.modifiers.len())?;
        for modifier in &symbol.modifiers {
            self.write_string(modifier)?;
        }
        self.write_vec_len(symbol.doc_comments.len())?;
        for doc_comment in &symbol.doc_comments {
            self.write_doc_comment(doc_comment)?;
        }
        self.write_vec_len(symbol.conditional_context.len())?;
        for branch in &symbol.conditional_context {
            self.write_conditional_branch(branch)?;
        }
        match symbol.callable_form {
            Some(form) => {
                self.write_u8(1);
                self.write_u8(callable_form_tag(form));
            }
            None => self.write_u8(0),
        }
        Ok(())
    }

    fn write_attribute(&mut self, attribute: &IndexedAttribute) -> Result<(), String> {
        self.write_option_string(attribute.name.as_deref())?;
        self.write_string(&attribute.text)
    }

    fn write_doc_comment(&mut self, comment: &IndexedDocComment) -> Result<(), String> {
        self.write_u8(doc_comment_kind_tag(comment.kind));
        self.write_string(&comment.text)
    }

    fn write_conditional_branch(
        &mut self,
        branch: &IndexedConditionalBranch,
    ) -> Result<(), String> {
        self.write_u8(preprocessor_branch_kind_tag(branch.kind));
        self.write_option_string(branch.condition.as_deref())
    }
}

struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    string_table: Vec<String>,
}

impl<'a> BinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            string_table: Vec::new(),
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
        usize::try_from(self.read_u64()?).map_err(|_| "u64 value exceeds usize".to_string())
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

    fn read_raw_string(&mut self) -> Result<String, String> {
        let len = self.read_bounded_len("raw string byte", MAX_CACHE_RAW_STRING_BYTES)?;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|error| format!("invalid utf-8 string: {error}"))
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
            .cloned()
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

    fn read_option_span(&mut self) -> Result<Option<TextSpan>, String> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_span().map(Some),
            tag => Err(format!("invalid option span tag {tag}")),
        }
    }

    fn read_global_id(&mut self) -> Result<GlobalSymbolId, String> {
        Ok(GlobalSymbolId {
            file_id: SourceFileId(self.read_usize()?),
            symbol_id: SymbolId(self.read_usize()?),
        })
    }

    fn read_option_global_id(&mut self) -> Result<Option<GlobalSymbolId>, String> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_global_id().map(Some),
            tag => Err(format!("invalid option global id tag {tag}")),
        }
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
            root_path: self.read_option_path()?,
            relative_path: self.read_option_path()?,
            priority: self.read_u16()?,
        })
    }

    fn read_indexed_file(&mut self) -> Result<IndexedFile, String> {
        Ok(IndexedFile {
            id: SourceFileId(self.read_usize()?),
            metadata: self.read_metadata()?,
            symbol_start: self.read_usize()?,
            symbol_count: self.read_usize()?,
            non_declaration_callable_fragments: self.read_usize()?,
        })
    }

    fn read_detail(&mut self) -> Result<IndexedSymbolDetail, String> {
        Ok(IndexedSymbolDetail {
            type_text: self.read_option_string()?,
            type_text_span: self.read_option_span()?,
            return_type_text: self.read_option_string()?,
            return_type_text_span: self.read_option_span()?,
            base_type: self.read_option_string()?,
            base_type_span: self.read_option_span()?,
            default_text: self.read_option_string()?,
            default_text_span: self.read_option_span()?,
            enum_value_text: self.read_option_string()?,
            enum_value_text_span: self.read_option_span()?,
        })
    }

    fn read_indexed_symbol(&mut self) -> Result<IndexedSymbol, String> {
        let id = self.read_global_id()?;
        let parent = self.read_option_global_id()?;
        let kind = symbol_kind_from_tag(self.read_u8()?)?;
        let name = self.read_option_string()?;
        let span = self.read_span()?;
        let selection_span = self.read_span()?;
        let detail = self.read_detail()?;
        let attributes = self.read_list(Self::read_attribute)?;
        let modifiers = self.read_list(Self::read_string)?;
        let doc_comments = self.read_list(Self::read_doc_comment)?;
        let conditional_context = self.read_list(Self::read_conditional_branch)?;
        let callable_form = match self.read_u8()? {
            0 => None,
            1 => Some(callable_form_from_tag(self.read_u8()?)?),
            tag => return Err(format!("invalid callable form option tag {tag}")),
        };
        Ok(IndexedSymbol {
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

    fn read_attribute(&mut self) -> Result<IndexedAttribute, String> {
        Ok(IndexedAttribute {
            name: self.read_option_string()?,
            text: self.read_string()?,
        })
    }

    fn read_doc_comment(&mut self) -> Result<IndexedDocComment, String> {
        Ok(IndexedDocComment {
            kind: doc_comment_kind_from_tag(self.read_u8()?)?,
            text: self.read_string()?,
        })
    }

    fn read_conditional_branch(&mut self) -> Result<IndexedConditionalBranch, String> {
        Ok(IndexedConditionalBranch {
            kind: preprocessor_branch_kind_from_tag(self.read_u8()?)?,
            condition: self.read_option_string()?,
        })
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

fn symbol_kind_tag(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Class => 0,
        SymbolKind::TypeParameter => 1,
        SymbolKind::Enum => 2,
        SymbolKind::EnumMember => 3,
        SymbolKind::Typedef => 4,
        SymbolKind::Function => 5,
        SymbolKind::GlobalField => 6,
        SymbolKind::Field => 7,
        SymbolKind::Method => 8,
        SymbolKind::Constructor => 9,
        SymbolKind::Destructor => 10,
        SymbolKind::Parameter => 11,
        SymbolKind::LocalVariable => 12,
        SymbolKind::PreprocessorMacro => 13,
    }
}

fn symbol_kind_from_tag(tag: u8) -> Result<SymbolKind, String> {
    match tag {
        0 => Ok(SymbolKind::Class),
        1 => Ok(SymbolKind::TypeParameter),
        2 => Ok(SymbolKind::Enum),
        3 => Ok(SymbolKind::EnumMember),
        4 => Ok(SymbolKind::Typedef),
        5 => Ok(SymbolKind::Function),
        6 => Ok(SymbolKind::GlobalField),
        7 => Ok(SymbolKind::Field),
        8 => Ok(SymbolKind::Method),
        9 => Ok(SymbolKind::Constructor),
        10 => Ok(SymbolKind::Destructor),
        11 => Ok(SymbolKind::Parameter),
        12 => Ok(SymbolKind::LocalVariable),
        13 => Ok(SymbolKind::PreprocessorMacro),
        _ => Err(format!("invalid symbol kind tag {tag}")),
    }
}

fn doc_comment_kind_tag(kind: DocCommentKind) -> u8 {
    match kind {
        DocCommentKind::Line => 0,
        DocCommentKind::Block => 1,
    }
}

fn doc_comment_kind_from_tag(tag: u8) -> Result<DocCommentKind, String> {
    match tag {
        0 => Ok(DocCommentKind::Line),
        1 => Ok(DocCommentKind::Block),
        _ => Err(format!("invalid doc comment kind tag {tag}")),
    }
}

fn preprocessor_branch_kind_tag(kind: PreprocessorBranchKind) -> u8 {
    match kind {
        PreprocessorBranchKind::If => 0,
        PreprocessorBranchKind::Ifdef => 1,
        PreprocessorBranchKind::Ifndef => 2,
        PreprocessorBranchKind::Elif => 3,
        PreprocessorBranchKind::Else => 4,
    }
}

fn preprocessor_branch_kind_from_tag(tag: u8) -> Result<PreprocessorBranchKind, String> {
    match tag {
        0 => Ok(PreprocessorBranchKind::If),
        1 => Ok(PreprocessorBranchKind::Ifdef),
        2 => Ok(PreprocessorBranchKind::Ifndef),
        3 => Ok(PreprocessorBranchKind::Elif),
        4 => Ok(PreprocessorBranchKind::Else),
        _ => Err(format!("invalid preprocessor branch kind tag {tag}")),
    }
}

fn callable_form_tag(form: CallableForm) -> u8 {
    match form {
        CallableForm::Implementation => 0,
        CallableForm::Declaration => 1,
        CallableForm::Prototype => 2,
    }
}

fn callable_form_from_tag(tag: u8) -> Result<CallableForm, String> {
    match tag {
        0 => Ok(CallableForm::Implementation),
        1 => Ok(CallableForm::Declaration),
        2 => Ok(CallableForm::Prototype),
        _ => Err(format!("invalid callable form tag {tag}")),
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

fn source_fingerprint(
    scripts_root: &Path,
    metadata_path: Option<&Path>,
) -> Result<SourceFingerprint, String> {
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

    let manual = manual_folder_fingerprint(Path::new(&scripts_root))?;
    Ok(SourceFingerprint::Manual {
        scripts_root,
        file_count: manual.file_count,
        byte_count: manual.byte_count,
        latest_modified_unix_ms: manual.latest_modified_unix_ms,
    })
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

fn manual_folder_fingerprint(root: &Path) -> Result<ManualFolderFingerprint, String> {
    let mut fingerprint = ManualFolderFingerprint {
        file_count: 0,
        byte_count: 0,
        latest_modified_unix_ms: 0,
    };
    collect_manual_fingerprint(root, &mut fingerprint)?;
    Ok(fingerprint)
}

fn collect_manual_fingerprint(
    folder: &Path,
    fingerprint: &mut ManualFolderFingerprint,
) -> Result<(), String> {
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
            collect_manual_fingerprint(&path, fingerprint)?;
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
    }
    Ok(())
}

fn summary_from_build(result: &IndexBuildResult) -> RuntimeIndexSummary {
    RuntimeIndexSummary {
        files: result.summary.totals.files,
        bytes: result.summary.totals.bytes,
        indexed_symbols: result.summary.totals.indexed_symbols,
        parse_diagnostics: result.summary.totals.parse_diagnostics,
        lossy_files: result.summary.totals.lossy_files,
    }
}

fn summary_from_build_with_cached_index(
    result: &IndexBuildResult,
    cached_index: &SymbolIndex,
) -> RuntimeIndexSummary {
    RuntimeIndexSummary {
        indexed_symbols: cached_index.symbols().len(),
        ..summary_from_build(result)
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SymbolKind;
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
        assert_eq!(decoded.index.contributions.len(), 1);
        decoded.index.validate_public_contributions().unwrap();

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
    fn v10_binary_cache_load_rebuilds_lookup_maps_from_files_and_symbols() {
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
        assert!(loaded
            .index
            .members_by_owner("Example")
            .iter()
            .any(|id| loaded
                .index
                .symbol(*id)
                .is_some_and(|symbol| symbol.name.as_deref() == Some("m_Value"))));

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
        decoded.index.contributions[0].schema_version += 1;
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
        bytes.extend_from_slice(&(MAX_CACHE_STRING_TABLE_ENTRIES as u64 + 1).to_le_bytes());

        let error = decode_cached_index(&bytes).unwrap_err();
        assert!(error.contains("string table entry length"));
        assert!(error.contains("exceeds safety limit"));
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
