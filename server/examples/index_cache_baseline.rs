use reforger_language_server::index::{GlobalSymbolId, IndexedFile, IndexedSymbol, SymbolIndex};
use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::index_cache::{
    load_or_build_game_data_index, GameDataIndexCacheConfig, GameDataIndexCacheResult,
    IndexCacheStatus, IndexCacheTimings,
};
use reforger_language_server::model::{SourceKind, SymbolKind, SOURCE_PRIORITY_GAME_DATA};
use std::env;
use std::fs;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/index-cache-baseline.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools";

struct Args {
    scripts_path: PathBuf,
    metadata_path: Option<PathBuf>,
    cache_path: PathBuf,
    out_path: PathBuf,
    profile_label: String,
}

struct DirectBuildMeasurement {
    index: SymbolIndex,
    total: Duration,
}

fn main() -> Result<(), String> {
    let args = parse_args()?;
    let report = render_report(&args)?;

    if let Some(parent) = args.out_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create report folder {}: {error}",
                parent.display()
            )
        })?;
    }

    fs::write(&args.out_path, report).map_err(|error| {
        format!(
            "Failed to write index cache baseline report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!(
        "Wrote index cache baseline report: {}",
        args.out_path.display()
    );
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut scripts: Option<PathBuf> = None;
    let mut metadata: Option<Option<PathBuf>> = None;
    let mut cache: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut profile_label: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scripts" => {
                let Some(value) = args.next() else {
                    return Err("--scripts requires a path".to_string());
                };
                scripts = Some(PathBuf::from(value));
            }
            "--metadata" => {
                let Some(value) = args.next() else {
                    return Err("--metadata requires a path or 'none'".to_string());
                };
                metadata = Some((value != "none").then(|| PathBuf::from(value)));
            }
            "--cache" => {
                let Some(value) = args.next() else {
                    return Err("--cache requires a path".to_string());
                };
                cache = Some(PathBuf::from(value));
            }
            "--out" => {
                let Some(value) = args.next() else {
                    return Err("--out requires a path".to_string());
                };
                out = Some(PathBuf::from(value));
            }
            "--profile-label" => {
                let Some(value) = args.next() else {
                    return Err("--profile-label requires a value".to_string());
                };
                profile_label = Some(value);
            }
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --manifest-path server/Cargo.toml --example index_cache_baseline -- [--scripts <path>] [--metadata <path|none>] [--cache <path>] [--out <path>] [--profile-label <label>]"
                );
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }

    let scripts_path = scripts.unwrap_or_else(default_scripts_path);
    let metadata_path = metadata.unwrap_or_else(|| default_metadata_path(&scripts_path));

    Ok(Args {
        scripts_path,
        metadata_path,
        cache_path: cache.unwrap_or_else(default_cache_path),
        out_path: resolve_repo_path(out, DEFAULT_REPORT_RELATIVE_PATH),
        profile_label: profile_label.unwrap_or_else(default_profile_label),
    })
}

fn render_report(args: &Args) -> Result<String, String> {
    let existing_cache = load_or_build_game_data_index(&GameDataIndexCacheConfig {
        scripts_root: args.scripts_path.clone(),
        cache_path: args.cache_path.clone(),
        metadata_path: args.metadata_path.clone(),
    })?;

    let temp_cache_path = temp_cache_path(&args.profile_label);
    let temp_cache_result = load_or_build_game_data_index(&GameDataIndexCacheConfig {
        scripts_root: args.scripts_path.clone(),
        cache_path: temp_cache_path.clone(),
        metadata_path: args.metadata_path.clone(),
    });
    let _ = fs::remove_file(&temp_cache_path);
    let temp_cache = temp_cache_result?;

    let direct = direct_build(args)?;

    let mut report = String::new();
    report.push_str("# Index Cache Baseline Report\n\n");
    report.push_str(
        "> Dev-only benchmark comparing binary cache load, cache miss rebuild/write, and direct rebuild.\n\n",
    );
    report.push_str("This report measures cache usefulness only. Timings are local wall-clock diagnostics, not benchmark-grade results.\n\n");
    report.push_str(&format!("## Profile: {}\n\n", args.profile_label));
    append_inputs(&mut report, args);
    append_summary(&mut report, &existing_cache, &temp_cache, &direct);
    append_cache_measurement(&mut report, "Existing cache load/hit path", &existing_cache);
    append_cache_measurement(
        &mut report,
        "Temporary cache miss rebuild/write",
        &temp_cache,
    );
    append_direct_measurement(&mut report, &direct);
    append_count_comparison(&mut report, &existing_cache, &temp_cache, &direct);
    append_structural_optimization(&mut report, &existing_cache, &direct);
    append_memory_and_cache_size(&mut report, &existing_cache, &direct.index);
    append_decision(&mut report, args, &existing_cache, &direct);
    Ok(report)
}

fn direct_build(args: &Args) -> Result<DirectBuildMeasurement, String> {
    let start = std::time::Instant::now();
    let result = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &args.scripts_path,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })?;
    Ok(DirectBuildMeasurement {
        index: result.index,
        total: start.elapsed(),
    })
}

fn append_inputs(report: &mut String, args: &Args) {
    report.push_str("### Inputs\n\n");
    report.push_str("| Input | Path |\n");
    report.push_str("| --- | --- |\n");
    report.push_str(&format!(
        "| Game data scripts | `{}` |\n",
        args.scripts_path.display()
    ));
    report.push_str(&format!(
        "| Metadata | `{}` |\n",
        args.metadata_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    ));
    report.push_str(&format!("| Cache | `{}` |\n", args.cache_path.display()));
    report.push_str(&format!(
        "| Scan timestamp unix seconds | `{}` |\n\n",
        timestamp()
    ));
}

fn append_summary(
    report: &mut String,
    existing_cache: &GameDataIndexCacheResult,
    temp_cache: &GameDataIndexCacheResult,
    direct: &DirectBuildMeasurement,
) {
    report.push_str("### Summary\n\n");
    report.push_str("| Measurement | Status | Files | Symbols | Total ms |\n");
    report.push_str("| --- | --- | ---: | ---: | ---: |\n");
    report.push_str(&format!(
        "| Existing cache path | {} | {} | {} | {} |\n",
        cache_status_label(&existing_cache.cache_status),
        existing_cache.summary.files,
        existing_cache.summary.indexed_symbols,
        duration_millis(existing_cache.timings.total)
    ));
    report.push_str(&format!(
        "| Temporary cache miss | {} | {} | {} | {} |\n",
        cache_status_label(&temp_cache.cache_status),
        temp_cache.summary.files,
        temp_cache.summary.indexed_symbols,
        duration_millis(temp_cache.timings.total)
    ));
    report.push_str(&format!(
        "| Direct rebuild | no-cache | {} | {} | {} |\n\n",
        direct.index.files().len(),
        direct.index.symbols().len(),
        duration_millis(direct.total)
    ));
}

fn append_cache_measurement(report: &mut String, title: &str, result: &GameDataIndexCacheResult) {
    report.push_str(&format!("### {title}\n\n"));
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Cache status | `{}` |\n",
        cache_status_label(&result.cache_status)
    ));
    if let Some(detail) = result.cache_status.detail() {
        report.push_str(&format!("| Status detail | `{detail}` |\n"));
    }
    report.push_str(&format!(
        "| Fingerprint | `{}` |\n",
        result.fingerprint.summary()
    ));
    report.push_str(&format!(
        "| Cache file bytes | {} |\n",
        result.cache_file_bytes.unwrap_or(0)
    ));
    append_cache_timings(report, &result.timings);
    report.push_str(&format!("| Files | {} |\n", result.summary.files));
    report.push_str(&format!("| Bytes | {} |\n", result.summary.bytes));
    report.push_str(&format!(
        "| Indexed symbols | {} |\n",
        result.summary.indexed_symbols
    ));
    report.push_str(&format!(
        "| Parse diagnostics | {} |\n",
        result.summary.parse_diagnostics
    ));
    report.push('\n');
}

fn append_cache_timings(report: &mut String, timings: &IndexCacheTimings) {
    report.push_str(&format!(
        "| Fingerprint ms | {} |\n",
        duration_millis(timings.fingerprint)
    ));
    report.push_str(&format!(
        "| Cache read/deserialize/validate ms | {} |\n",
        duration_millis(timings.cache_read_deserialize_validate)
    ));
    report.push_str(&format!(
        "| Cache file read ms | {} |\n",
        duration_millis(timings.cache_file_read)
    ));
    report.push_str(&format!(
        "| Binary decode ms | {} |\n",
        duration_millis(timings.cache_decode)
    ));
    report.push_str(&format!(
        "| Cache validate ms | {} |\n",
        duration_millis(timings.cache_validate)
    ));
    report.push_str(&format!(
        "| Lookup map rebuild ms | {} |\n",
        duration_millis(timings.map_rebuild)
    ));
    report.push_str(&format!(
        "| Rebuild ms | {} |\n",
        duration_millis(timings.rebuild)
    ));
    report.push_str(&format!(
        "| Cache serialize/write ms | {} |\n",
        duration_millis(timings.cache_write)
    ));
    report.push_str(&format!(
        "| Total load-or-build ms | {} |\n",
        duration_millis(timings.total)
    ));
}

fn append_direct_measurement(report: &mut String, direct: &DirectBuildMeasurement) {
    report.push_str("### Direct Rebuild Without Cache\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Total rebuild ms | {} |\n",
        duration_millis(direct.total)
    ));
    report.push_str(&format!(
        "| Indexed files | {} |\n",
        direct.index.files().len()
    ));
    report.push_str(&format!(
        "| Indexed symbols | {} |\n\n",
        direct.index.symbols().len()
    ));
}

fn append_count_comparison(
    report: &mut String,
    existing_cache: &GameDataIndexCacheResult,
    temp_cache: &GameDataIndexCacheResult,
    direct: &DirectBuildMeasurement,
) {
    report.push_str("### Count Consistency\n\n");
    report.push_str("| Comparison | Result |\n");
    report.push_str("| --- | --- |\n");
    let direct_locals = count_kind(&direct.index, SymbolKind::LocalVariable);
    let direct_parameters = count_kind(&direct.index, SymbolKind::Parameter);
    let expected_pruned_symbols = direct.index.symbols().len().saturating_sub(direct_locals);
    report.push_str(&format!(
        "| Existing cache symbols match direct rebuild minus locals | {} |\n",
        yes_no(existing_cache.index.symbols().len() == expected_pruned_symbols)
    ));
    report.push_str(&format!(
        "| Temp cache symbols match direct rebuild minus locals | {} |\n",
        yes_no(temp_cache.index.symbols().len() == expected_pruned_symbols)
    ));
    report.push_str(&format!(
        "| Existing cache local variables removed | {} |\n",
        yes_no(count_kind(&existing_cache.index, SymbolKind::LocalVariable) == 0)
    ));
    report.push_str(&format!(
        "| Temp cache local variables removed | {} |\n",
        yes_no(count_kind(&temp_cache.index, SymbolKind::LocalVariable) == 0)
    ));
    report.push_str(&format!(
        "| Existing cache parameters preserved | {} |\n",
        yes_no(count_kind(&existing_cache.index, SymbolKind::Parameter) == direct_parameters)
    ));
    report.push_str(&format!(
        "| Temp cache parameters preserved | {} |\n",
        yes_no(count_kind(&temp_cache.index, SymbolKind::Parameter) == direct_parameters)
    ));
    report.push_str(&format!(
        "| Existing cache files match direct rebuild | {} |\n",
        yes_no(existing_cache.index.files().len() == direct.index.files().len())
    ));
    report.push_str(&format!(
        "| Temp cache files match direct rebuild | {} |\n\n",
        yes_no(temp_cache.index.files().len() == direct.index.files().len())
    ));
    report.push_str(&format!(
        "Runtime cache pruning removed `{direct_locals}` local-variable symbols and preserved `{direct_parameters}` parameter symbols from the full direct index.\n\n"
    ));
}

fn append_structural_optimization(
    report: &mut String,
    existing_cache: &GameDataIndexCacheResult,
    direct: &DirectBuildMeasurement,
) {
    let v2_style = direct.index.without_local_variables();
    let v2_style_bytes = serde_json::to_vec(&v2_style)
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    let v9_bytes = existing_cache.cache_file_bytes.unwrap_or(0);
    let v9_detail_spans = detail_span_count(&existing_cache.index);
    let full_detail_spans = detail_span_count(&direct.index);
    let saved = v2_style_bytes.saturating_sub(v9_bytes);

    report.push_str("## Runtime Cache Structural Optimization\n\n");
    report.push_str("V9 persists cache metadata plus files/symbols only in a dependency-free binary format, stores repeated strings through an interned string table, stores an explicit index-shape marker, strips source-only detail spans, removes external local variables, and rebuilds lookup maps after load. The v2-style estimate serializes the runtime-pruned index with derived maps still present.\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| V2-style full-map runtime JSON estimate | {} |\n",
        v2_style_bytes
    ));
    report.push_str(&format!("| V9 actual cache file bytes | {} |\n", v9_bytes));
    report.push_str(&format!("| Estimated bytes saved | {} |\n", saved));
    report.push_str(&format!(
        "| Estimated size reduction | {} |\n",
        percent(saved as usize, v2_style_bytes as usize)
    ));
    report.push_str(&format!(
        "| Full direct detail span fields | {} |\n",
        full_detail_spans
    ));
    report.push_str(&format!(
        "| V9 cached detail span fields | {} |\n",
        v9_detail_spans
    ));
    report.push_str(&format!("| Lookup maps persisted | {} |\n", yes_no(false)));
    report.push_str(&format!(
        "| Lookup maps rebuilt on load | {} |\n\n",
        yes_no(!existing_cache.index.symbols().is_empty())
    ));
    append_lookup_map_shape(report, existing_cache);
}

fn append_lookup_map_shape(report: &mut String, existing_cache: &GameDataIndexCacheResult) {
    let maps = existing_cache.index.map_counts();
    let total_entries = maps.name_entries
        + maps.top_level_name_entries
        + maps.kind_entries
        + maps.class_name_entries
        + maps.typedef_name_entries
        + maps.function_name_entries
        + maps.method_owner_name_entries
        + maps.field_owner_name_entries
        + maps.member_owner_entries
        + maps.child_entries;

    report.push_str("### Lookup Map Rebuild Shape\n\n");
    report.push_str("The binary cache persists files and symbols only. These lookup maps are rebuilt after decode so the runtime can answer name, kind, class, typedef, function, member, owner/name, and parent/child queries without scanning all symbols per request.\n\n");
    report.push_str("| Map | Keys | Symbol ID entries |\n");
    report.push_str("| --- | ---: | ---: |\n");
    report.push_str(&format!(
        "| All names | {} | {} |\n",
        maps.names, maps.name_entries
    ));
    report.push_str(&format!(
        "| Top-level names | {} | {} |\n",
        maps.top_level_names, maps.top_level_name_entries
    ));
    report.push_str(&format!(
        "| Kinds | {} | {} |\n",
        maps.kinds, maps.kind_entries
    ));
    report.push_str(&format!(
        "| Classes by name | {} | {} |\n",
        maps.class_names, maps.class_name_entries
    ));
    report.push_str(&format!(
        "| Typedefs by name | {} | {} |\n",
        maps.typedef_names, maps.typedef_name_entries
    ));
    report.push_str(&format!(
        "| Functions by name | {} | {} |\n",
        maps.function_names, maps.function_name_entries
    ));
    report.push_str(&format!(
        "| Methods by owner/name | {} | {} |\n",
        maps.method_owner_names, maps.method_owner_name_entries
    ));
    report.push_str(&format!(
        "| Fields by owner/name | {} | {} |\n",
        maps.field_owner_names, maps.field_owner_name_entries
    ));
    report.push_str(&format!(
        "| Members by owner | {} | {} |\n",
        maps.member_owners, maps.member_owner_entries
    ));
    report.push_str(&format!(
        "| Children by parent | {} | {} |\n",
        maps.parent_symbols, maps.child_entries
    ));
    report.push_str(&format!(
        "| Total rebuilt symbol ID entries |  | {} |\n\n",
        total_entries
    ));
}

fn append_memory_and_cache_size(
    report: &mut String,
    cache_result: &GameDataIndexCacheResult,
    index: &SymbolIndex,
) {
    let estimate = index_shape_estimate(index);
    report.push_str("### Cache Size And Lower-Bound Memory\n\n");
    report.push_str("The memory estimate is a lower bound from public index record sizes and copied text bytes. It is not process RSS and excludes allocator/map overhead.\n\n");
    report.push_str("| Item | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Binary cache file bytes | {} |\n",
        cache_result.cache_file_bytes.unwrap_or(0)
    ));
    report.push_str(&format!(
        "| Binary cache file MiB | {:.2} |\n",
        cache_result.cache_file_bytes.unwrap_or(0) as f64 / (1024.0 * 1024.0)
    ));
    report.push_str(&format!(
        "| Index lower-bound bytes | {} |\n",
        estimate.total_lower_bound_bytes
    ));
    report.push_str(&format!(
        "| Index lower-bound MiB | {:.2} |\n\n",
        estimate.total_lower_bound_bytes as f64 / (1024.0 * 1024.0)
    ));
}

fn append_decision(
    report: &mut String,
    args: &Args,
    existing_cache: &GameDataIndexCacheResult,
    direct: &DirectBuildMeasurement,
) {
    report.push_str("### Cache Usefulness Decision\n\n");
    if !args.profile_label.to_ascii_lowercase().contains("release") {
        report.push_str(
            "Debug timing is informational only. Use the release section for cache decisions.\n\n",
        );
        return;
    }

    if existing_cache.cache_status != IndexCacheStatus::Loaded {
        report.push_str("The existing cache path did not produce a cache hit in this release run, so binary cache-hit usefulness is not proven. Re-run after a valid cache exists before expanding runtime cache policy.\n\n");
        return;
    }

    let cache_hit_ms = duration_millis(existing_cache.timings.total);
    let direct_ms = duration_millis(direct.total);
    if cache_hit_ms == 0 || direct_ms == 0 {
        report.push_str("Timing was too small to compare reliably.\n\n");
        return;
    }

    let cache_hit = cache_hit_ms as f64;
    let direct = direct_ms as f64;
    if cache_hit <= direct * 0.75 {
        report.push_str(&format!(
            "Binary cache is useful in this run: release cache hit `{cache_hit_ms}` ms is at least 25% faster than direct rebuild `{direct_ms}` ms.\n\n"
        ));
    } else if cache_hit <= direct * 1.25 {
        report.push_str(&format!(
            "Binary cache is not proven useful in this run: release cache hit `{cache_hit_ms}` ms is within ±25% of direct rebuild `{direct_ms}` ms.\n\n"
        ));
    } else {
        report.push_str(&format!(
            "Binary cache appears harmful in this run: release cache hit `{cache_hit_ms}` ms is slower than direct rebuild `{direct_ms}` ms. Consider disabling or replacing the cache before expanding runtime indexing.\n\n"
        ));
    }
}

fn cache_status_label(status: &IndexCacheStatus) -> String {
    match status {
        IndexCacheStatus::Loaded => "loaded".to_string(),
        IndexCacheStatus::Rebuilt { reason } => format!("rebuilt ({reason})"),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn percent(part: usize, total: usize) -> String {
    if total == 0 {
        return "0.0%".to_string();
    }
    format!("{:.1}%", part as f64 * 100.0 / total as f64)
}

fn count_kind(index: &SymbolIndex, kind: SymbolKind) -> usize {
    index.symbols_for_kind(kind).len()
}

fn detail_span_count(index: &SymbolIndex) -> usize {
    index
        .symbols()
        .iter()
        .map(|symbol| {
            usize::from(symbol.detail.type_text_span.is_some())
                + usize::from(symbol.detail.return_type_text_span.is_some())
                + usize::from(symbol.detail.base_type_span.is_some())
                + usize::from(symbol.detail.default_text_span.is_some())
                + usize::from(symbol.detail.enum_value_text_span.is_some())
        })
        .sum()
}

struct IndexShapeEstimate {
    total_lower_bound_bytes: usize,
}

fn index_shape_estimate(index: &SymbolIndex) -> IndexShapeEstimate {
    let name_map_id_entries = index.names().values().map(Vec::len).sum::<usize>();
    let copied_text_bytes = copied_text_bytes(index);
    let record_lower_bound_bytes = size_of::<SymbolIndex>()
        + index.files().len() * size_of::<IndexedFile>()
        + index.symbols().len() * size_of::<IndexedSymbol>()
        + name_map_id_entries * size_of::<GlobalSymbolId>();
    let total_lower_bound_bytes = record_lower_bound_bytes + copied_text_bytes;

    IndexShapeEstimate {
        total_lower_bound_bytes,
    }
}

fn copied_text_bytes(index: &SymbolIndex) -> usize {
    let mut bytes = 0usize;
    for file in index.files() {
        bytes += path_bytes(file.metadata.absolute_path.as_deref());
        bytes += path_bytes(file.metadata.root_path.as_deref());
        bytes += path_bytes(file.metadata.relative_path.as_deref());
    }

    for symbol in index.symbols() {
        bytes += option_string_bytes(symbol.name.as_deref());
        bytes += option_string_bytes(symbol.detail.type_text.as_deref());
        bytes += option_string_bytes(symbol.detail.return_type_text.as_deref());
        bytes += option_string_bytes(symbol.detail.base_type.as_deref());
        bytes += option_string_bytes(symbol.detail.default_text.as_deref());
        bytes += option_string_bytes(symbol.detail.enum_value_text.as_deref());
        bytes += symbol.modifiers.iter().map(String::len).sum::<usize>();
        bytes += symbol
            .attributes
            .iter()
            .map(|attribute| attribute.text.len() + option_string_bytes(attribute.name.as_deref()))
            .sum::<usize>();
        bytes += symbol
            .doc_comments
            .iter()
            .map(|comment| comment.text.len())
            .sum::<usize>();
        bytes += symbol
            .conditional_context
            .iter()
            .map(|branch| option_string_bytes(branch.condition.as_deref()))
            .sum::<usize>();
    }

    bytes
}

fn option_string_bytes(value: Option<&str>) -> usize {
    value.map(str::len).unwrap_or(0)
}

fn path_bytes(path: Option<&Path>) -> usize {
    path.map(|path| path.to_string_lossy().len()).unwrap_or(0)
}

fn temp_cache_path(profile_label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!(
        "reforger_index_cache_baseline_{}_{}_{}.bin",
        profile_label,
        std::process::id(),
        nonce
    ))
}

fn default_scripts_path() -> PathBuf {
    default_storage_root().join("game-data/scripts")
}

fn default_metadata_path(scripts_path: &Path) -> Option<PathBuf> {
    scripts_path
        .parent()
        .map(|parent| parent.join("metadata.json"))
        .filter(|path| path.is_file())
}

fn default_cache_path() -> PathBuf {
    default_storage_root().join("index-cache/game-data-symbol-index.v9.bin")
}

fn default_storage_root() -> PathBuf {
    if let Some(app_data) = env::var_os("APPDATA") {
        PathBuf::from(app_data).join(DEFAULT_STORAGE_RELATIVE_PATH)
    } else {
        PathBuf::from(DEFAULT_STORAGE_RELATIVE_PATH)
    }
}

fn default_profile_label() -> String {
    if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "release".to_string()
    }
}

fn resolve_repo_path(path: Option<PathBuf>, default_relative_path: &str) -> PathBuf {
    match path {
        Some(path) if path.is_absolute() => path,
        Some(path) => repo_root().join(path),
        None => repo_root().join(default_relative_path),
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("server crate should be inside the repository root")
        .to_path_buf()
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn duration_millis(duration: Duration) -> u128 {
    duration.as_millis()
}
