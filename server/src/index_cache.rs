use crate::index::{IndexedFile, IndexedSymbol, SymbolIndex};
use crate::index_build::{build_index, IndexBuildConfig, IndexBuildResult, IndexSourceRoot};
use crate::model::{SourceKind, SOURCE_PRIORITY_GAME_DATA};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

const CACHE_FORMAT_VERSION: u32 = 6;
const CACHE_SCHEMA: &str = "reforger-symbol-index";

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

#[derive(Debug, Serialize, Deserialize)]
struct CachedGameDataIndex {
    schema: String,
    format_version: u32,
    crate_version: String,
    fingerprint: SourceFingerprint,
    summary: CachedIndexSummary,
    index: CachedSymbolIndex,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedSymbolIndex {
    files: Vec<IndexedFile>,
    symbols: Vec<IndexedSymbol>,
}

impl From<&SymbolIndex> for CachedSymbolIndex {
    fn from(index: &SymbolIndex) -> Self {
        Self {
            files: index.files().to_vec(),
            symbols: index.symbols().to_vec(),
        }
    }
}

impl From<CachedSymbolIndex> for SymbolIndex {
    fn from(snapshot: CachedSymbolIndex) -> Self {
        SymbolIndex::from_indexed_parts(snapshot.files, snapshot.symbols)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    let total_start = Instant::now();
    let mut timings = IndexCacheTimings::default();
    if !config.scripts_root.is_dir() {
        return Err(format!(
            "Game-data scripts folder does not exist: {}",
            config.scripts_root.display()
        ));
    }

    let fingerprint_start = Instant::now();
    let fingerprint = source_fingerprint(&config.scripts_root, config.metadata_path.as_deref())?;
    timings.fingerprint = fingerprint_start.elapsed();
    let initial_cache_file_bytes = cache_file_bytes(&config.cache_path);

    let cache_read_start = Instant::now();
    match load_cached_index(&config.cache_path, &fingerprint) {
        Ok(Some(cached)) => {
            timings.cache_read_deserialize_validate = cache_read_start.elapsed();
            timings.total = total_start.elapsed();
            return Ok(GameDataIndexCacheResult {
                index: cached.index.into(),
                summary: cached.summary.into(),
                cache_status: IndexCacheStatus::Loaded,
                fingerprint,
                timings,
                cache_file_bytes: initial_cache_file_bytes,
            });
        }
        Ok(None) | Err(_) => {
            timings.cache_read_deserialize_validate = cache_read_start.elapsed();
        }
    }

    let rebuild_reason = cache_rebuild_reason(&config.cache_path, &fingerprint);
    let rebuild_start = Instant::now();
    let built = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &config.scripts_root,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })?;
    timings.rebuild = rebuild_start.elapsed();
    let cached_index = built.index.compact_for_runtime_cache();
    let summary = summary_from_build_with_cached_index(&built, &cached_index);

    let cache_write_start = Instant::now();
    write_cached_index(&config.cache_path, &fingerprint, &summary, &cached_index)?;
    timings.cache_write = cache_write_start.elapsed();
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
) -> Result<Option<CachedGameDataIndex>, String> {
    if !cache_path.is_file() {
        return Ok(None);
    }

    let file = fs::File::open(cache_path).map_err(|error| {
        format!(
            "Failed to open index cache {}: {error}",
            cache_path.display()
        )
    })?;
    let reader = BufReader::new(file);
    let cached = serde_json::from_reader::<_, CachedGameDataIndex>(reader).map_err(|error| {
        format!(
            "Failed to deserialize index cache {}: {error}",
            cache_path.display()
        )
    })?;

    if cached.schema != CACHE_SCHEMA
        || cached.format_version != CACHE_FORMAT_VERSION
        || cached.crate_version != env!("CARGO_PKG_VERSION")
        || cached.fingerprint != *expected_fingerprint
    {
        return Ok(None);
    }

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

    let temp_path = cache_path.with_extension("tmp");
    let file = fs::File::create(&temp_path).map_err(|error| {
        format!(
            "Failed to create temporary index cache {}: {error}",
            temp_path.display()
        )
    })?;
    let writer = BufWriter::new(file);
    let cached = CachedGameDataIndex {
        schema: CACHE_SCHEMA.to_string(),
        format_version: CACHE_FORMAT_VERSION,
        crate_version: env!("CARGO_PKG_VERSION").to_string(),
        fingerprint: fingerprint.clone(),
        summary: CachedIndexSummary::from(summary),
        index: CachedSymbolIndex::from(index),
    };
    serde_json::to_writer(writer, &cached).map_err(|error| {
        format!(
            "Failed to serialize index cache {}: {error}",
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
        let cache_json = fs::read_to_string(&cache).unwrap();
        assert!(cache_json.contains("\"format_version\":6"));
        assert!(!cache_json.contains("\"by_name\""));
        assert!(!cache_json.contains("\"methods_by_owner_name\""));

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
    fn v6_cache_load_rebuilds_lookup_maps_from_files_and_symbols() {
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

        let stale = fs::read_to_string(&cache)
            .unwrap()
            .replace("\"format_version\":6", "\"format_version\":5");
        write_file(&cache, &stale);

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
        write_file(&cache, "{ bad json");

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
