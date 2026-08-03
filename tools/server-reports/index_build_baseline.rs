use reforger_language_server::index::{GlobalSymbolId, IndexedFile, IndexedSymbol, SymbolIndex};
use reforger_language_server::index_build::{
    build_index, IndexBuildConfig, IndexBuildCounts, IndexBuildSummary, IndexBuildTimings,
    IndexSourceRoot,
};
use reforger_language_server::model::{
    SourceKind, SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE,
};
use std::env;
use std::fs;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/index-build-baseline.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const CACHE_RECOMMENDATION_THRESHOLD_MS: u128 = 3_000;

struct Args {
    scripts_path: PathBuf,
    workspace_path: Option<PathBuf>,
    out_path: PathBuf,
    profile_label: String,
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
            "Failed to write index build baseline report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!(
        "Wrote index build baseline report: {}",
        args.out_path.display()
    );
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut scripts: Option<PathBuf> = None;
    let mut workspace: Option<PathBuf> = None;
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
            "--workspace" => {
                let Some(value) = args.next() else {
                    return Err("--workspace requires a path".to_string());
                };
                workspace = Some(PathBuf::from(value));
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
                    "Usage: cargo run --manifest-path server/Cargo.toml --example index_build_baseline -- [--scripts <path>] [--workspace <path>] [--out <path>] [--profile-label <label>]"
                );
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }

    Ok(Args {
        scripts_path: scripts.unwrap_or_else(default_scripts_path),
        workspace_path: workspace,
        out_path: resolve_repo_path(out, DEFAULT_REPORT_RELATIVE_PATH),
        profile_label: profile_label.unwrap_or_else(default_profile_label),
    })
}

fn render_report(args: &Args) -> Result<String, String> {
    let mut roots = vec![IndexSourceRoot::new(
        &args.scripts_path,
        SourceKind::GameData,
        SOURCE_PRIORITY_GAME_DATA,
    )];

    if let Some(workspace_path) = &args.workspace_path {
        roots.push(IndexSourceRoot::new(
            workspace_path,
            SourceKind::Workspace,
            SOURCE_PRIORITY_WORKSPACE,
        ));
    }

    let build_result = build_index(&IndexBuildConfig { roots })?;
    let index = build_result.index;
    let summary = build_result.summary;

    let mut report = String::new();
    report.push_str("# Index Build Baseline Report\n\n");
    report.push_str(
        "> Dev-only performance baseline generated without corpus-analysis report rendering.\n\n",
    );
    report.push_str("This report measures index construction only. Timings are wall-clock diagnostics for local trend spotting, not benchmark-grade results.\n\n");
    report.push_str(&format!("## Profile: {}\n\n", args.profile_label));
    append_inputs(&mut report, args);
    append_summary(&mut report, &summary, &index);
    append_phase_timings(&mut report, &summary.timings);
    append_throughput(&mut report, &summary.totals, &summary.timings);
    append_index_shape_estimate(&mut report, &index);
    append_cache_recommendation(&mut report, &args.profile_label, &summary.timings);
    Ok(report)
}

fn append_inputs(report: &mut String, args: &Args) {
    report.push_str("### Inputs\n\n");
    report.push_str("| Input | Path |\n");
    report.push_str("| --- | --- |\n");
    report.push_str(&format!(
        "| Game data scripts | `{}` |\n",
        args.scripts_path.display()
    ));
    if let Some(workspace_path) = &args.workspace_path {
        report.push_str(&format!(
            "| Workspace scripts | `{}` |\n",
            workspace_path.display()
        ));
    }
    report.push_str(&format!(
        "| Scan timestamp unix seconds | `{}` |\n\n",
        timestamp()
    ));
}

fn append_summary(report: &mut String, summary: &IndexBuildSummary, index: &SymbolIndex) {
    let map_counts = index.map_counts();
    report.push_str("### Build Summary\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| `.c` files | {} |\n", summary.totals.files));
    report.push_str(&format!("| Bytes | {} |\n", summary.totals.bytes));
    report.push_str(&format!(
        "| Files decoded lossily | {} |\n",
        summary.totals.lossy_files
    ));
    report.push_str(&format!(
        "| Parse diagnostics | {} |\n",
        summary.totals.parse_diagnostics
    ));
    report.push_str(&format!("| Indexed files | {} |\n", index.files().len()));
    report.push_str(&format!(
        "| Indexed symbols | {} |\n",
        index.symbols().len()
    ));
    report.push_str(&format!("| Unique symbol names | {} |\n", map_counts.names));
    report.push_str(&format!(
        "| Unique top-level names | {} |\n",
        map_counts.top_level_names
    ));
    report.push_str(&format!("| Class names | {} |\n", map_counts.class_names));
    report.push_str(&format!(
        "| Typedef names | {} |\n",
        map_counts.typedef_names
    ));
    report.push_str(&format!(
        "| Method owner/name keys | {} |\n",
        map_counts.method_owner_names
    ));
    report.push_str(&format!(
        "| Parent symbols with children | {} |\n",
        map_counts.parent_symbols
    ));
    report.push_str(&format!(
        "| Non-declaration callable fragments | {} |\n\n",
        summary.totals.non_declaration_callable_fragments
    ));

    report.push_str("### Source Roots\n\n");
    report.push_str("| Source kind | Files | Bytes | Indexed symbols | Parse diagnostics |\n");
    report.push_str("| --- | ---: | ---: | ---: | ---: |\n");
    for (kind, counts) in &summary.by_source_kind {
        report.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            kind.as_str(),
            counts.files,
            counts.bytes,
            counts.indexed_symbols,
            counts.parse_diagnostics
        ));
    }
    report.push('\n');
}

fn append_phase_timings(report: &mut String, timings: &IndexBuildTimings) {
    report.push_str("### Phase Timings\n\n");
    report.push_str("| Phase | Milliseconds |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| File discovery | {} |\n",
        duration_millis(timings.file_discovery)
    ));
    report.push_str(&format!(
        "| Read/decode | {} |\n",
        duration_millis(timings.read_decode)
    ));
    report.push_str(&format!("| Parse | {} |\n", duration_millis(timings.parse)));
    report.push_str(&format!(
        "| AST/model catalog | {} |\n",
        duration_millis(timings.ast_model_catalog)
    ));
    report.push_str(&format!(
        "| Catalog build aggregate | {} |\n",
        duration_millis(timings.catalog_build)
    ));
    report.push_str(&format!(
        "| Index aggregation | {} |\n",
        duration_millis(timings.index_build)
    ));
    report.push_str(&format!(
        "| Total build | {} |\n\n",
        duration_millis(timings.total)
    ));
}

fn append_throughput(report: &mut String, totals: &IndexBuildCounts, timings: &IndexBuildTimings) {
    report.push_str("### Throughput\n\n");
    report.push_str("| Measurement | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Read/decode throughput | {} MiB/s |\n",
        mib_per_second(totals.bytes, timings.read_decode)
    ));
    report.push_str(&format!(
        "| Parse throughput | {} MiB/s |\n",
        mib_per_second(totals.bytes, timings.parse)
    ));
    report.push_str(&format!(
        "| Full build throughput | {} MiB/s |\n\n",
        mib_per_second(totals.bytes, timings.total)
    ));
}

fn append_index_shape_estimate(report: &mut String, index: &SymbolIndex) {
    let estimate = index_shape_estimate(index);
    report.push_str("### Index Shape / Lower-Bound Memory Estimate\n\n");
    report.push_str("This is a lower-bound estimate from record sizes and copied text bytes visible through public index data. It is not process RSS and excludes allocator, map, vector capacity, and other heap overhead.\n\n");
    report.push_str("| Item | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Indexed files | {} |\n", index.files().len()));
    report.push_str(&format!(
        "| Indexed symbols | {} |\n",
        index.symbols().len()
    ));
    report.push_str(&format!(
        "| Name-map symbol id entries | {} |\n",
        estimate.name_map_id_entries
    ));
    report.push_str(&format!(
        "| Copied text bytes visible in index | {} |\n",
        estimate.copied_text_bytes
    ));
    report.push_str(&format!(
        "| Record lower-bound bytes | {} |\n",
        estimate.record_lower_bound_bytes
    ));
    report.push_str(&format!(
        "| Total lower-bound bytes | {} |\n",
        estimate.total_lower_bound_bytes
    ));
    report.push_str(&format!(
        "| Total lower-bound MiB | {:.2} |\n\n",
        estimate.total_lower_bound_bytes as f64 / (1024.0 * 1024.0)
    ));
}

fn append_cache_recommendation(
    report: &mut String,
    profile_label: &str,
    timings: &IndexBuildTimings,
) {
    report.push_str("### Cache Recommendation\n\n");
    let is_release = profile_label.to_ascii_lowercase().contains("release");
    if !is_release {
        report.push_str(
            "Debug timing is informational only. Use the release section for cache decisions.\n\n",
        );
        return;
    }

    let total_ms = duration_millis(timings.total);
    if total_ms >= CACHE_RECOMMENDATION_THRESHOLD_MS {
        report.push_str(&format!(
            "Release build time is `{total_ms}` ms, which is at or above the `{CACHE_RECOMMENDATION_THRESHOLD_MS}` ms threshold. Runtime cache work is recommended before using full game-data indexing on extension/LSP startup.\n\n"
        ));
        report.push_str("Recommended next architecture step: cache the game-data index in global storage, invalidate it by game-data commit SHA, index workspace files separately/incrementally, and treat the cache as disposable rather than source truth.\n\n");
    } else {
        report.push_str(&format!(
            "Release build time is `{total_ms}` ms, below the `{CACHE_RECOMMENDATION_THRESHOLD_MS}` ms threshold. A persisted cache is not yet proven necessary; re-evaluate during actual LSP/runtime startup integration.\n\n"
        ));
    }
}

struct IndexShapeEstimate {
    name_map_id_entries: usize,
    copied_text_bytes: usize,
    record_lower_bound_bytes: usize,
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
        name_map_id_entries,
        copied_text_bytes,
        record_lower_bound_bytes,
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

fn default_scripts_path() -> PathBuf {
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

fn mib_per_second(bytes: usize, duration: Duration) -> String {
    let seconds = duration.as_secs_f64();
    if seconds <= f64::EPSILON {
        return "n/a".to_string();
    }

    format!("{:.2}", bytes as f64 / (1024.0 * 1024.0) / seconds)
}
