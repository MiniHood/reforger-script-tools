use reforger_language_server::index::{IndexedFile, IndexedSymbol, SymbolIndex};
use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::index_cache::{
    load_or_build_game_data_index, GameDataIndexCacheConfig, GameDataIndexCacheResult,
    IndexCacheStatus,
};
use reforger_language_server::model::{
    CallableForm, SourceCategory, SourceKind, SymbolKind, SOURCE_PRIORITY_GAME_DATA,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/index-cache-composition.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools";

struct Args {
    scripts_path: PathBuf,
    metadata_path: Option<PathBuf>,
    cache_path: PathBuf,
    out_path: PathBuf,
}

#[derive(Default)]
struct Composition {
    files_by_category: BTreeMap<SourceCategory, usize>,
    symbols_by_category: BTreeMap<SourceCategory, usize>,
    bytes_by_category: BTreeMap<SourceCategory, usize>,
    symbols_by_kind: BTreeMap<SymbolKind, usize>,
    bytes_by_kind: BTreeMap<SymbolKind, usize>,
    callable_forms: BTreeMap<String, usize>,
    with_docs: usize,
    with_attributes: usize,
    with_modifiers: usize,
    with_conditional_context: usize,
    editor_files: usize,
    editor_symbols: usize,
    editor_bytes: usize,
    debug_files: usize,
    debug_symbols: usize,
    debug_bytes: usize,
}

#[derive(Serialize)]
struct MeasurementSnapshot<'index> {
    label: &'static str,
    files: Vec<&'index IndexedFile>,
    symbols: Vec<&'index IndexedSymbol>,
}

struct SerializationMeasurement {
    label: &'static str,
    bytes: u64,
    file_count: usize,
    symbol_count: usize,
    deleted_temp_file: bool,
    note: &'static str,
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
            "Failed to write index cache composition report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!(
        "Wrote index cache composition report: {}",
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
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --manifest-path server/Cargo.toml --example index_cache_composition_report -- [--scripts <path>] [--metadata <path|none>] [--cache <path>] [--out <path>]"
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
    })
}

fn render_report(args: &Args) -> Result<String, String> {
    let cache = load_or_build_game_data_index(&GameDataIndexCacheConfig {
        scripts_root: args.scripts_path.clone(),
        cache_path: args.cache_path.clone(),
        metadata_path: args.metadata_path.clone(),
    })?;
    let full = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &args.scripts_path,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })?;
    let composition = compose_index(&cache.index);
    let full_composition = compose_index(&full.index);
    let measurements = temporary_serialization_measurements(&cache.index)?;
    let v2_style_cache_bytes = v2_style_runtime_cache_bytes(&full.index)?;

    let mut report = String::new();
    report.push_str("# Index Cache Composition Report\n\n");
    report.push_str(
        "> Dev-only report generated by `node tools/index-cache-composition-report.mjs`.\n\n",
    );
    report.push_str("This report explains what the current game-data index cache contains and how much of it appears relevant to editor hover/completion. It is measurement data only; it does not define a new cache format.\n\n");
    append_inputs(&mut report, args);
    append_cache_status(&mut report, &cache);
    append_runtime_pruning_summary(&mut report, &full.index, &cache.index);
    append_structural_optimization_summary(
        &mut report,
        &full.index,
        &cache.index,
        cache.cache_file_bytes.unwrap_or(0),
        v2_style_cache_bytes,
    );
    append_slice_summary(&mut report, &cache.index, &composition);
    append_full_kind_comparison(&mut report, &full_composition, &composition);
    append_category_counts(&mut report, &composition);
    append_kind_counts(&mut report, &composition);
    append_feature_counts(&mut report, &composition, cache.index.symbols().len());
    append_temporary_serialization(&mut report, &measurements);
    append_recommendation(&mut report, &composition, &measurements);
    Ok(report)
}

fn append_inputs(report: &mut String, args: &Args) {
    report.push_str("## Inputs\n\n");
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

fn append_cache_status(report: &mut String, cache: &GameDataIndexCacheResult) {
    report.push_str("## Cache Status\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Status | `{}` |\n",
        cache_status_label(&cache.cache_status)
    ));
    report.push_str(&format!(
        "| Fingerprint | `{}` |\n",
        cache.fingerprint.summary()
    ));
    report.push_str(&format!(
        "| Cache file bytes | {} |\n",
        cache.cache_file_bytes.unwrap_or(0)
    ));
    report.push_str(&format!(
        "| Cache file MiB | {:.2} |\n",
        cache.cache_file_bytes.unwrap_or(0) as f64 / (1024.0 * 1024.0)
    ));
    report.push_str(&format!(
        "| Load-or-build ms | {} |\n",
        cache.timings.total.as_millis()
    ));
    report.push_str(&format!("| Files | {} |\n", cache.summary.files));
    report.push_str(&format!(
        "| Indexed symbols | {} |\n",
        cache.summary.indexed_symbols
    ));
    report.push_str(&format!(
        "| Parse diagnostics | {} |\n\n",
        cache.summary.parse_diagnostics
    ));
}

fn append_runtime_pruning_summary(report: &mut String, full: &SymbolIndex, runtime: &SymbolIndex) {
    let full_locals = count_kind(full, SymbolKind::LocalVariable);
    let runtime_locals = count_kind(runtime, SymbolKind::LocalVariable);
    let full_parameters = count_kind(full, SymbolKind::Parameter);
    let runtime_parameters = count_kind(runtime, SymbolKind::Parameter);
    let removed_symbols = full.symbols().len().saturating_sub(runtime.symbols().len());

    report.push_str("## Runtime-Pruned Cache Summary\n\n");
    report.push_str("The runtime game-data cache intentionally removes `LocalVariable` symbols only. Open-document analysis still keeps locals, while external game-data cache keeps parameters for callable signatures and future signature help.\n\n");
    report.push_str("| Metric | Full direct index | Runtime cache | Delta |\n");
    report.push_str("| --- | ---: | ---: | ---: |\n");
    report.push_str(&format!(
        "| Files | {} | {} | {} |\n",
        full.files().len(),
        runtime.files().len(),
        full.files().len() as isize - runtime.files().len() as isize
    ));
    report.push_str(&format!(
        "| Symbols | {} | {} | {} |\n",
        full.symbols().len(),
        runtime.symbols().len(),
        removed_symbols
    ));
    report.push_str(&format!(
        "| Local variables | {} | {} | {} |\n",
        full_locals,
        runtime_locals,
        full_locals.saturating_sub(runtime_locals)
    ));
    report.push_str(&format!(
        "| Parameters | {} | {} | {} |\n\n",
        full_parameters,
        runtime_parameters,
        full_parameters as isize - runtime_parameters as isize
    ));
}

fn append_structural_optimization_summary(
    report: &mut String,
    full: &SymbolIndex,
    runtime: &SymbolIndex,
    cache_file_bytes: u64,
    v2_style_cache_bytes: u64,
) {
    let full_detail_spans = detail_span_count(full);
    let runtime_detail_spans = detail_span_count(runtime);
    let removed_maps = [
        "by_name",
        "top_level_by_name",
        "by_kind",
        "children",
        "classes_by_name",
        "typedefs_by_name",
        "functions_by_name",
        "methods_by_owner_name",
        "fields_by_owner_name",
        "members_by_owner",
    ];
    let bytes_saved = v2_style_cache_bytes.saturating_sub(cache_file_bytes);

    report.push_str("## V6 Structural Optimization Summary\n\n");
    report.push_str("The v6 runtime cache persists files and symbols only, strips source-only detail spans, removes external local variables, preserves parameters/type parameters, and rebuilds derived lookup maps after deserialization. This keeps editor-visible facts while avoiding duplicate serialized lookup data.\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| V2-style full-map runtime JSON estimate | {} |\n",
        v2_style_cache_bytes
    ));
    report.push_str(&format!(
        "| V6 actual cache file bytes | {} |\n",
        cache_file_bytes
    ));
    report.push_str(&format!("| Estimated bytes saved | {} |\n", bytes_saved));
    report.push_str(&format!(
        "| Estimated size reduction | {} |\n",
        percent(bytes_saved as usize, v2_style_cache_bytes as usize)
    ));
    report.push_str(&format!(
        "| Derived lookup maps omitted | {} |\n",
        removed_maps.len()
    ));
    report.push_str(&format!(
        "| Full direct detail span fields | {} |\n",
        full_detail_spans
    ));
    report.push_str(&format!(
        "| V6 runtime detail span fields | {} |\n",
        runtime_detail_spans
    ));
    report.push_str(&format!(
        "| Runtime symbols still carrying copied detail text | {} |\n\n",
        runtime
            .symbols()
            .iter()
            .filter(|symbol| symbol.detail.type_text.is_some()
                || symbol.detail.return_type_text.is_some()
                || symbol.detail.base_type.is_some()
                || symbol.detail.default_text.is_some()
                || symbol.detail.enum_value_text.is_some())
            .count()
    ));
    report.push_str("Omitted maps: ");
    report.push_str(&removed_maps.join(", "));
    report.push_str(".\n\n");
}

fn append_slice_summary(report: &mut String, index: &SymbolIndex, composition: &Composition) {
    let total_files = index.files().len();
    let total_symbols = index.symbols().len();
    let total_bytes = composition.editor_bytes + composition.debug_bytes;

    report.push_str("## Runtime Cache vs Editor Runtime Slice\n\n");
    report.push_str("This section analyzes the runtime-pruned cache. Editor runtime uses the current `SourceCategory::is_editor_completion_default()` policy. Debug/review-only categories remain indexed today for broad lookup and report/debug tools.\n\n");
    report
        .push_str("| Slice | Files | Symbols | Lower-bound bytes | File % | Symbol % | Byte % |\n");
    report.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    report.push_str(&format!(
        "| Runtime cache index | {} | {} | {} | 100.0% | 100.0% | 100.0% |\n",
        total_files, total_symbols, total_bytes
    ));
    report.push_str(&format!(
        "| Editor runtime slice | {} | {} | {} | {} | {} | {} |\n",
        composition.editor_files,
        composition.editor_symbols,
        composition.editor_bytes,
        percent(composition.editor_files, total_files),
        percent(composition.editor_symbols, total_symbols),
        percent(composition.editor_bytes, total_bytes)
    ));
    report.push_str(&format!(
        "| Debug/review-only slice | {} | {} | {} | {} | {} | {} |\n\n",
        composition.debug_files,
        composition.debug_symbols,
        composition.debug_bytes,
        percent(composition.debug_files, total_files),
        percent(composition.debug_symbols, total_symbols),
        percent(composition.debug_bytes, total_bytes)
    ));
}

fn append_full_kind_comparison(report: &mut String, full: &Composition, runtime: &Composition) {
    report.push_str("## Full Direct vs Runtime Cache By Symbol Kind\n\n");
    report.push_str("| Symbol kind | Full direct | Runtime cache | Removed |\n");
    report.push_str("| --- | ---: | ---: | ---: |\n");
    for kind in symbol_kinds() {
        let full_count = full.symbols_by_kind.get(&kind).copied().unwrap_or(0);
        let runtime_count = runtime.symbols_by_kind.get(&kind).copied().unwrap_or(0);
        if full_count == 0 && runtime_count == 0 {
            continue;
        }
        report.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            kind_name(kind),
            full_count,
            runtime_count,
            full_count.saturating_sub(runtime_count)
        ));
    }
    report.push('\n');
}

fn append_category_counts(report: &mut String, composition: &Composition) {
    report.push_str("## Source Category Composition\n\n");
    report.push_str("| Category | Editor default | Files | Symbols | Lower-bound bytes |\n");
    report.push_str("| --- | --- | ---: | ---: | ---: |\n");
    for category in source_categories() {
        let files = composition
            .files_by_category
            .get(&category)
            .copied()
            .unwrap_or(0);
        let symbols = composition
            .symbols_by_category
            .get(&category)
            .copied()
            .unwrap_or(0);
        let bytes = composition
            .bytes_by_category
            .get(&category)
            .copied()
            .unwrap_or(0);
        if files == 0 && symbols == 0 && bytes == 0 {
            continue;
        }
        report.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} |\n",
            category.as_str(),
            if category.is_editor_completion_default() {
                "included"
            } else {
                "excluded"
            },
            files,
            symbols,
            bytes
        ));
    }
    report.push('\n');
}

fn append_kind_counts(report: &mut String, composition: &Composition) {
    report.push_str("## Symbol Kind Composition\n\n");
    report.push_str("| Symbol kind | Symbols | Lower-bound bytes |\n");
    report.push_str("| --- | ---: | ---: |\n");
    for kind in symbol_kinds() {
        let symbols = composition.symbols_by_kind.get(&kind).copied().unwrap_or(0);
        let bytes = composition.bytes_by_kind.get(&kind).copied().unwrap_or(0);
        if symbols == 0 && bytes == 0 {
            continue;
        }
        report.push_str(&format!(
            "| `{}` | {} | {} |\n",
            kind_name(kind),
            symbols,
            bytes
        ));
    }
    report.push('\n');
}

fn append_feature_counts(report: &mut String, composition: &Composition, total_symbols: usize) {
    report.push_str("## Presentation And Policy Facts\n\n");
    report.push_str("| Fact | Symbols | Percent |\n");
    report.push_str("| --- | ---: | ---: |\n");
    report.push_str(&format!(
        "| Doc comments | {} | {} |\n",
        composition.with_docs,
        percent(composition.with_docs, total_symbols)
    ));
    report.push_str(&format!(
        "| Attributes | {} | {} |\n",
        composition.with_attributes,
        percent(composition.with_attributes, total_symbols)
    ));
    report.push_str(&format!(
        "| Modifiers | {} | {} |\n",
        composition.with_modifiers,
        percent(composition.with_modifiers, total_symbols)
    ));
    report.push_str(&format!(
        "| Conditional context | {} | {} |\n\n",
        composition.with_conditional_context,
        percent(composition.with_conditional_context, total_symbols)
    ));

    report.push_str("### Callable Forms\n\n");
    if composition.callable_forms.is_empty() {
        report.push_str("None.\n\n");
        return;
    }
    report.push_str("| Callable form | Symbols |\n");
    report.push_str("| --- | ---: |\n");
    for (form, count) in &composition.callable_forms {
        report.push_str(&format!("| `{}` | {} |\n", form, count));
    }
    report.push('\n');
}

fn append_temporary_serialization(report: &mut String, measurements: &[SerializationMeasurement]) {
    report.push_str("## Temporary Serialized Size Measurements\n\n");
    report.push_str("These are temporary measurement snapshots written outside tracked source paths and deleted immediately. They are not reusable cache files and do not define cache schema.\n\n");
    report.push_str("| Snapshot | Files | Symbols | JSON bytes | Temp deleted | Note |\n");
    report.push_str("| --- | ---: | ---: | ---: | --- | --- |\n");
    for measurement in measurements {
        report.push_str(&format!(
            "| `{}` | {} | {} | {} | `{}` | {} |\n",
            measurement.label,
            measurement.file_count,
            measurement.symbol_count,
            measurement.bytes,
            if measurement.deleted_temp_file {
                "yes"
            } else {
                "no"
            },
            measurement.note
        ));
    }
    report.push('\n');
}

fn append_recommendation(
    report: &mut String,
    composition: &Composition,
    measurements: &[SerializationMeasurement],
) {
    let total_symbols = composition.editor_symbols + composition.debug_symbols;
    let editor_symbol_percent = ratio(composition.editor_symbols, total_symbols);
    let full_json = measurements
        .iter()
        .find(|measurement| measurement.label == "runtime-cache-measurement")
        .map(|measurement| measurement.bytes)
        .unwrap_or(0);
    let editor_json = measurements
        .iter()
        .find(|measurement| measurement.label == "editor-runtime-categories")
        .map(|measurement| measurement.bytes)
        .unwrap_or(0);
    let editor_json_percent = if full_json == 0 {
        1.0
    } else {
        editor_json as f64 / full_json as f64
    };

    report.push_str("## Recommendation\n\n");
    if editor_symbol_percent >= 0.85 || editor_json_percent >= 0.85 {
        report.push_str("Keep the current single JSON cache for now. The editor-runtime slice is close to the full cache, so splitting the cache is unlikely to repay the extra complexity yet.\n\n");
    } else if editor_json_percent <= 0.65 {
        report.push_str("A future split cache may be worth designing: an editor-runtime cache plus optional debug/review cache could materially reduce startup load and disk size. Do not change runtime behavior in this slice; use this report as planning evidence.\n\n");
    } else {
        report.push_str("The editor-runtime slice is smaller but not decisively enough to justify cache redesign yet. Revisit after completion/definition decide exactly which facts must be loaded at startup.\n\n");
    }

    report.push_str(&format!(
        "- Editor-runtime symbols: {} of {} ({})\n",
        composition.editor_symbols,
        total_symbols,
        percent(composition.editor_symbols, total_symbols)
    ));
    report.push_str(&format!(
        "- Editor-runtime measurement JSON: {} of {} ({})\n",
        editor_json,
        full_json,
        percent(editor_json as usize, full_json as usize)
    ));
    report.push_str(
        "- Source files and parser/model/index output remain truth; cache is disposable.\n\n",
    );
}

fn compose_index(index: &SymbolIndex) -> Composition {
    let mut composition = Composition::default();

    for file in index.files() {
        *composition
            .files_by_category
            .entry(file.metadata.category)
            .or_default() += 1;
        let file_bytes = estimated_file_bytes(file);
        *composition
            .bytes_by_category
            .entry(file.metadata.category)
            .or_default() += file_bytes;
        if file.metadata.category.is_editor_completion_default() {
            composition.editor_files += 1;
            composition.editor_bytes += file_bytes;
        } else {
            composition.debug_files += 1;
            composition.debug_bytes += file_bytes;
        }
    }

    for symbol in index.symbols() {
        let category = index
            .file(symbol.id.file_id)
            .map(|file| file.metadata.category)
            .unwrap_or(SourceCategory::Unknown);
        let bytes = estimated_symbol_bytes(symbol);

        *composition.symbols_by_category.entry(category).or_default() += 1;
        *composition.bytes_by_category.entry(category).or_default() += bytes;
        *composition.symbols_by_kind.entry(symbol.kind).or_default() += 1;
        *composition.bytes_by_kind.entry(symbol.kind).or_default() += bytes;

        if category.is_editor_completion_default() {
            composition.editor_symbols += 1;
            composition.editor_bytes += bytes;
        } else {
            composition.debug_symbols += 1;
            composition.debug_bytes += bytes;
        }

        if let Some(form) = symbol.callable_form {
            *composition
                .callable_forms
                .entry(callable_form_name(form).to_string())
                .or_default() += 1;
        }
        if !symbol.doc_comments.is_empty() {
            composition.with_docs += 1;
        }
        if !symbol.attributes.is_empty() {
            composition.with_attributes += 1;
        }
        if !symbol.modifiers.is_empty() {
            composition.with_modifiers += 1;
        }
        if !symbol.conditional_context.is_empty() {
            composition.with_conditional_context += 1;
        }
    }

    composition
}

fn temporary_serialization_measurements(
    index: &SymbolIndex,
) -> Result<Vec<SerializationMeasurement>, String> {
    let full_files = index.files().iter().collect::<Vec<_>>();
    let full_symbols = index.symbols().iter().collect::<Vec<_>>();
    let editor_files = index
        .files()
        .iter()
        .filter(|file| file.metadata.category.is_editor_completion_default())
        .collect::<Vec<_>>();
    let editor_symbols = index
        .symbols()
        .iter()
        .filter(|symbol| {
            index
                .file(symbol.id.file_id)
                .is_some_and(|file| file.metadata.category.is_editor_completion_default())
        })
        .collect::<Vec<_>>();
    let editor_runtime_core_symbols = index
        .symbols()
        .iter()
        .filter(|symbol| {
            index
                .file(symbol.id.file_id)
                .is_some_and(|file| file.metadata.category.is_editor_completion_default())
                && !matches!(
                    symbol.kind,
                    SymbolKind::Parameter | SymbolKind::LocalVariable
                )
        })
        .collect::<Vec<_>>();

    Ok(vec![
        write_temp_snapshot(
            "runtime-cache-measurement",
            full_files,
            full_symbols,
            "full runtime-pruned public file/symbol data",
        )?,
        write_temp_snapshot(
            "editor-runtime-categories",
            editor_files.clone(),
            editor_symbols,
            "editor-included source categories only",
        )?,
        write_temp_snapshot(
            "editor-runtime-no-locals-params",
            editor_files,
            editor_runtime_core_symbols,
            "estimate only; removing parameters/locals can break callable/local parent-child detail",
        )?,
    ])
}

fn v2_style_runtime_cache_bytes(index: &SymbolIndex) -> Result<u64, String> {
    let v2_style = index.without_local_variables();
    serde_json::to_vec(&v2_style)
        .map(|bytes| bytes.len() as u64)
        .map_err(|error| format!("Failed to serialize v2-style full-map estimate: {error}"))
}

fn write_temp_snapshot(
    label: &'static str,
    files: Vec<&IndexedFile>,
    symbols: Vec<&IndexedSymbol>,
    note: &'static str,
) -> Result<SerializationMeasurement, String> {
    let path = temp_snapshot_path(label);
    let snapshot = MeasurementSnapshot {
        label,
        files,
        symbols,
    };
    let raw = serde_json::to_vec(&snapshot)
        .map_err(|error| format!("Failed to serialize temporary {label} snapshot: {error}"))?;
    fs::write(&path, &raw).map_err(|error| {
        format!(
            "Failed to write temporary snapshot {}: {error}",
            path.display()
        )
    })?;
    let bytes = path
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(raw.len() as u64);
    let file_count = snapshot.files.len();
    let symbol_count = snapshot.symbols.len();
    let _ = fs::remove_file(&path);
    let deleted_temp_file = !path.exists();

    Ok(SerializationMeasurement {
        label,
        bytes,
        file_count,
        symbol_count,
        deleted_temp_file,
        note,
    })
}

fn estimated_file_bytes(file: &IndexedFile) -> usize {
    size_of::<IndexedFile>()
        + path_bytes(file.metadata.absolute_path.as_deref())
        + path_bytes(file.metadata.root_path.as_deref())
        + path_bytes(file.metadata.relative_path.as_deref())
}

fn estimated_symbol_bytes(symbol: &IndexedSymbol) -> usize {
    size_of::<IndexedSymbol>()
        + option_string_bytes(symbol.name.as_deref())
        + option_string_bytes(symbol.detail.type_text.as_deref())
        + option_string_bytes(symbol.detail.return_type_text.as_deref())
        + option_string_bytes(symbol.detail.base_type.as_deref())
        + option_string_bytes(symbol.detail.default_text.as_deref())
        + option_string_bytes(symbol.detail.enum_value_text.as_deref())
        + symbol.modifiers.iter().map(String::len).sum::<usize>()
        + symbol
            .attributes
            .iter()
            .map(|attribute| attribute.text.len() + option_string_bytes(attribute.name.as_deref()))
            .sum::<usize>()
        + symbol
            .doc_comments
            .iter()
            .map(|comment| comment.text.len())
            .sum::<usize>()
        + symbol
            .conditional_context
            .iter()
            .map(|branch| option_string_bytes(branch.condition.as_deref()))
            .sum::<usize>()
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

fn temp_snapshot_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!(
        "reforger_index_cache_composition_{label}_{}_{}.json",
        std::process::id(),
        nonce
    ))
}

fn cache_status_label(status: &IndexCacheStatus) -> String {
    match status {
        IndexCacheStatus::Loaded => "loaded".to_string(),
        IndexCacheStatus::Rebuilt { reason } => format!("rebuilt ({reason})"),
    }
}

fn source_categories() -> [SourceCategory; 10] {
    [
        SourceCategory::Workspace,
        SourceCategory::Game,
        SourceCategory::GameCode,
        SourceCategory::GameLib,
        SourceCategory::Core,
        SourceCategory::Generated,
        SourceCategory::Workbench,
        SourceCategory::DocsDoxygen,
        SourceCategory::TestAutotest,
        SourceCategory::Unknown,
    ]
}

fn symbol_kinds() -> [SymbolKind; 12] {
    [
        SymbolKind::Class,
        SymbolKind::Enum,
        SymbolKind::EnumMember,
        SymbolKind::Typedef,
        SymbolKind::Function,
        SymbolKind::GlobalField,
        SymbolKind::Field,
        SymbolKind::Method,
        SymbolKind::Constructor,
        SymbolKind::Destructor,
        SymbolKind::Parameter,
        SymbolKind::LocalVariable,
    ]
}

fn kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "Class",
        SymbolKind::TypeParameter => "TypeParameter",
        SymbolKind::Enum => "Enum",
        SymbolKind::EnumMember => "EnumMember",
        SymbolKind::Typedef => "Typedef",
        SymbolKind::Function => "Function",
        SymbolKind::GlobalField => "GlobalField",
        SymbolKind::Field => "Field",
        SymbolKind::Method => "Method",
        SymbolKind::Constructor => "Constructor",
        SymbolKind::Destructor => "Destructor",
        SymbolKind::Parameter => "Parameter",
        SymbolKind::LocalVariable => "LocalVariable",
        SymbolKind::PreprocessorMacro => "PreprocessorMacro",
    }
}

fn count_kind(index: &SymbolIndex, kind: SymbolKind) -> usize {
    index.symbols_for_kind(kind).len()
}

fn callable_form_name(form: CallableForm) -> &'static str {
    match form {
        CallableForm::Implementation => "implementation",
        CallableForm::Declaration => "declaration",
        CallableForm::Prototype => "prototype",
    }
}

fn option_string_bytes(value: Option<&str>) -> usize {
    value.map(str::len).unwrap_or(0)
}

fn path_bytes(path: Option<&Path>) -> usize {
    path.map(|path| path.to_string_lossy().len()).unwrap_or(0)
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
    default_storage_root().join("index-cache/game-data-symbol-index.v6.json")
}

fn default_storage_root() -> PathBuf {
    if let Some(app_data) = env::var_os("APPDATA") {
        PathBuf::from(app_data).join(DEFAULT_STORAGE_RELATIVE_PATH)
    } else {
        PathBuf::from(DEFAULT_STORAGE_RELATIVE_PATH)
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

fn ratio(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 / total as f64
    }
}

fn percent(part: usize, total: usize) -> String {
    if total == 0 {
        return "0.0%".to_string();
    }
    format!("{:.1}%", ratio(part, total) * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reforger_language_server::ast::AstSourceFile;
    use reforger_language_server::index::SymbolIndex;
    use reforger_language_server::model::{
        SourceFileMetadata, SourceKind, SymbolCatalog, SOURCE_PRIORITY_GAME_DATA,
    };
    use reforger_language_server::parser::parse_source;

    #[test]
    fn composition_counts_categories_and_kinds() {
        let index = fixture_index();
        let composition = compose_index(&index);

        assert_eq!(
            composition
                .files_by_category
                .get(&SourceCategory::Game)
                .copied(),
            Some(1)
        );
        assert_eq!(
            composition
                .files_by_category
                .get(&SourceCategory::DocsDoxygen)
                .copied(),
            Some(1)
        );
        assert_eq!(
            composition.symbols_by_kind.get(&SymbolKind::Class).copied(),
            Some(2)
        );
        assert_eq!(
            composition
                .symbols_by_kind
                .get(&SymbolKind::Method)
                .copied(),
            Some(2)
        );
    }

    #[test]
    fn composition_uses_editor_category_policy() {
        let index = fixture_index();
        let composition = compose_index(&index);

        assert_eq!(composition.editor_files, 1);
        assert_eq!(composition.debug_files, 1);
        assert!(composition.editor_symbols > 0);
        assert!(composition.debug_symbols > 0);
    }

    #[test]
    fn lower_bound_byte_estimates_include_copied_text() {
        let index = fixture_index();
        let composition = compose_index(&index);

        assert!(composition.editor_bytes > 0);
        assert!(composition.debug_bytes > 0);
        assert!(
            composition
                .bytes_by_kind
                .get(&SymbolKind::Method)
                .copied()
                .unwrap_or_default()
                > 0
        );
    }

    #[test]
    fn temporary_serialization_deletes_temp_files() {
        let index = fixture_index();
        let measurements = temporary_serialization_measurements(&index).unwrap();

        assert_eq!(measurements.len(), 3);
        assert!(measurements.iter().all(|measurement| measurement.bytes > 0));
        assert!(measurements
            .iter()
            .all(|measurement| measurement.deleted_temp_file));
    }

    fn fixture_index() -> SymbolIndex {
        let runtime = catalog(
            r#"//! Runtime docs.
class RuntimeClass
{
	[Attribute()]
	protected void Run(int value);
}
"#,
            metadata(SourceCategory::Game, "Game/RuntimeClass.c"),
        );
        let docs = catalog(
            r#"class DocsClass
{
	void Example();
}
"#,
            metadata(SourceCategory::DocsDoxygen, "GameLib/WorldSystemsDocs.c"),
        );
        SymbolIndex::from_catalogs([&runtime, &docs])
    }

    fn catalog(source: &str, metadata: SourceFileMetadata) -> SymbolCatalog<'_> {
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ast = AstSourceFile::new(source, &parse);
        SymbolCatalog::from_ast_with_metadata(source, &ast, metadata)
    }

    fn metadata(category: SourceCategory, relative_path: &str) -> SourceFileMetadata {
        SourceFileMetadata {
            kind: SourceKind::GameData,
            category,
            absolute_path: Some(PathBuf::from("C:/game").join(relative_path)),
            root_path: Some(PathBuf::from("C:/game")),
            relative_path: Some(PathBuf::from(relative_path)),
            priority: SOURCE_PRIORITY_GAME_DATA,
        }
    }
}
