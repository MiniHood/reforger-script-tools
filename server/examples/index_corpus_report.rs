use reforger_language_server::index::{GlobalSymbolId, IndexedFile, IndexedSymbol, SymbolIndex};
use reforger_language_server::index_build::{
    build_index, IndexBuildConfig, IndexBuildCounts, IndexBuildTimings, IndexSourceRoot,
};
use reforger_language_server::model::{
    SourceCategory, SourceKind, SymbolKind, SOURCE_PRIORITY_GAME_DATA,
};
use reforger_language_server::symbol_display::SymbolDisplay;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/index-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_DUPLICATE_NAMES: usize = 100;
const MAX_DECLARATIONS_PER_DUPLICATE: usize = 8;
const MAX_SAMPLES: usize = 20;
const MAX_SIGNATURES_PER_METHOD_GROUP: usize = 3;
const MAX_MEMBER_SHADOW_GROUPS: usize = 100;
const MAX_SHADOWED_PER_GROUP: usize = 5;
const MAX_SUSPICIOUS_DUPLICATE_ROWS: usize = 50;
const MAX_SUSPICIOUS_SHADOW_ROWS: usize = 100;
const MAX_UNKNOWN_CONFLICT_SNIPPETS: usize = 25;
const SNIPPET_CONTEXT_LINES: usize = 2;

struct Args {
    scripts_path: PathBuf,
    out_path: PathBuf,
}

struct SymbolSample {
    id: GlobalSymbolId,
    kind: SymbolKind,
    name: String,
    detail: String,
}

struct MethodGroupSample {
    owner: String,
    name: String,
    overloads: usize,
    path: String,
    signatures: Vec<String>,
}

struct MemberShadowSample {
    owner: String,
    key: String,
    kept: GlobalSymbolId,
    shadowed: Vec<GlobalSymbolId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TopLevelProvenance {
    GeneratedOverlap,
    DocsDoxygenExample,
    TestAutotestOverlap,
    GameGameLibGeneratedSplit,
    MixedKindEngineWrapper,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ShadowProvenance {
    PreprocessorBranchDuplicate,
    PreprocessorPrototypeDuplicate,
    PrototypeDeclarationBlock,
    DocsDoxygenOnlySource,
    GeneratedSourceOverlap,
    InheritedAggregateArtifact,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DuplicateClassification {
    WorkspaceOverlay,
    TypedefFunctionDelegatePair,
    TypedefClassWrapperPattern,
    GeneratedVsSourceDuplicate,
    SuspiciousSameKindDuplicate,
    MixedKindDuplicate,
}

fn main() -> Result<(), String> {
    let args = parse_args()?;
    let report = render_report(&args.scripts_path)?;

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
            "Failed to write index corpus report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!("Wrote index corpus report: {}", args.out_path.display());
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut scripts: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scripts" => {
                let Some(value) = args.next() else {
                    return Err("--scripts requires a path".to_string());
                };
                scripts = Some(PathBuf::from(value));
            }
            "--out" => {
                let Some(value) = args.next() else {
                    return Err("--out requires a path".to_string());
                };
                out = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                println!(
                    "Usage: node tools/index-corpus-report.mjs [--scripts <path>] [--out <path>]"
                );
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }

    Ok(Args {
        scripts_path: scripts.unwrap_or_else(default_scripts_path),
        out_path: resolve_repo_path(out, DEFAULT_REPORT_RELATIVE_PATH),
    })
}

fn default_scripts_path() -> PathBuf {
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

fn render_report(scripts_path: &Path) -> Result<String, String> {
    let build_result = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            scripts_path,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })?;
    let index = build_result.index;
    let summary = build_result.summary;

    let report_render_start = Instant::now();
    let mut report = String::new();
    report.push_str("# Index Corpus Report\n\n");
    report.push_str("> Human-review output generated by `node tools/index-corpus-report.mjs`.\n\n");
    report.push_str("This report summarizes the first in-memory symbol index over game-data catalogs. It is lookup review data only; Workbench remains compiler truth.\n\n");

    append_summary(&mut report, scripts_path, &summary.totals, &index);
    append_lossy_decoded_files(&mut report, scripts_path, &summary.totals);
    append_parse_diagnostic_snippets(&mut report, scripts_path, &summary.totals);
    append_top_level_vs_member_symbols(&mut report, &index);
    append_source_kind_counts(&mut report, &index);
    append_source_category_counts(&mut report, &index);
    append_editor_completion_source_policy(&mut report, &index);
    append_kind_counts(&mut report, &index);
    append_presentation_coverage(&mut report, &index);
    append_duplicate_classification_summary(&mut report, &index);
    append_focused_suspicious_conflict_report(&mut report, &index);
    append_duplicate_top_level_names(&mut report, &index);
    append_preferred_duplicate_samples(&mut report, &index);
    append_lookup_samples(&mut report, &index);
    append_completion_member_shadow_groups(&mut report, &index);
    append_preferred_class_completion_shadow_summary(&mut report, &index);
    let report_render = report_render_start.elapsed();

    append_build_timing(&mut report, &summary.timings, report_render);

    Ok(report)
}

fn append_summary(
    report: &mut String,
    scripts_path: &Path,
    totals: &IndexBuildCounts,
    index: &SymbolIndex,
) {
    let map_counts = index.map_counts();
    report.push_str("## Summary\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Source path | `{}` |\n", scripts_path.display()));
    report.push_str(&format!(
        "| Scan timestamp unix seconds | {} |\n",
        timestamp()
    ));
    report.push_str(&format!("| `.c` files | {} |\n", totals.files));
    report.push_str(&format!("| Bytes | {} |\n", totals.bytes));
    report.push_str(&format!(
        "| Files decoded lossily | {} |\n",
        totals.lossy_files
    ));
    report.push_str(&format!(
        "| Parse diagnostics | {} |\n",
        totals.parse_diagnostics
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
    report.push_str(&format!("| Symbol kind maps | {} |\n", map_counts.kinds));
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
        totals.non_declaration_callable_fragments
    ));
}

fn append_lossy_decoded_files(report: &mut String, scripts_path: &Path, totals: &IndexBuildCounts) {
    report.push_str("## Lossy Decoded Files\n\n");
    if totals.lossy_files == 0 {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("These files required UTF-8 replacement while building the review report. Snippets are bounded around the first replacement character.\n\n");
    report.push_str("| File | First replacement | Byte offset | Replacement chars |\n");
    report.push_str("| --- | ---: | ---: | ---: |\n");
    for detail in &totals.lossy_decode_details {
        report.push_str(&format!(
            "| `{}` | {}:{} | {} | {} |\n",
            escape_table(&display_relative_path(scripts_path, &detail.path)),
            detail.line,
            detail.column,
            detail.first_replacement_offset,
            detail.replacement_count
        ));
    }
    if totals.lossy_files > totals.lossy_decode_details.len() {
        report.push_str(&format!(
            "| ... {} more |\n",
            totals.lossy_files - totals.lossy_decode_details.len()
        ));
    }
    report.push('\n');

    for detail in &totals.lossy_decode_details {
        report.push_str(&format!(
            "### `{}`\n\n",
            escape_inline(&display_relative_path(scripts_path, &detail.path))
        ));
        report.push_str("````enforce\n");
        report.push_str(&detail.snippet);
        report.push_str("````\n\n");
    }
}

fn append_parse_diagnostic_snippets(
    report: &mut String,
    scripts_path: &Path,
    totals: &IndexBuildCounts,
) {
    report.push_str("## Parse Diagnostic Snippets\n\n");
    if totals.parse_diagnostic_details.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    for detail in &totals.parse_diagnostic_details {
        report.push_str(&format!(
            "### `{}`\n\n",
            escape_inline(&display_relative_path(scripts_path, &detail.path))
        ));
        report.push_str(&format!(
            "- `{}` at {}:{} span {}..{}\n\n",
            escape_inline(&detail.message),
            detail.line,
            detail.column,
            detail.span.start,
            detail.span.end
        ));
        report.push_str("````enforce\n");
        report.push_str(&detail.snippet);
        report.push_str("````\n\n");
    }
}

fn append_top_level_vs_member_symbols(report: &mut String, index: &SymbolIndex) {
    let total_symbols = index.symbols().len();
    let top_level_symbols = index
        .symbols()
        .iter()
        .filter(|symbol| symbol.parent.is_none())
        .count();
    let child_member_symbols = total_symbols.saturating_sub(top_level_symbols);
    let parameter_symbols = index.symbols_for_kind(SymbolKind::Parameter).len();
    let non_parameter_child_member_symbols = child_member_symbols.saturating_sub(parameter_symbols);

    report.push_str("## Top-Level vs Member Symbols\n\n");
    report.push_str("Child/member symbols include class members, enum members, callable parameters, and any other symbol with a parent.\n\n");
    report.push_str("| Category | Symbols | Percent of indexed symbols |\n");
    report.push_str("| --- | ---: | ---: |\n");
    report.push_str(&format!(
        "| Top-level symbols | {} | {} |\n",
        top_level_symbols,
        percent(top_level_symbols, total_symbols)
    ));
    report.push_str(&format!(
        "| Child/member symbols | {} | {} |\n",
        child_member_symbols,
        percent(child_member_symbols, total_symbols)
    ));
    report.push_str(&format!(
        "| Parameter symbols | {} | {} |\n",
        parameter_symbols,
        percent(parameter_symbols, total_symbols)
    ));
    report.push_str(&format!(
        "| Non-parameter child/member symbols | {} | {} |\n\n",
        non_parameter_child_member_symbols,
        percent(non_parameter_child_member_symbols, total_symbols)
    ));
}

fn append_build_timing(report: &mut String, timings: &IndexBuildTimings, report_render: Duration) {
    report.push_str("## Build Timing\n\n");
    report.push_str("Wall-clock timings are for human review and trend spotting only; they are not benchmark-grade measurements.\n\n");
    report.push_str("| Phase | Milliseconds |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| File discovery | {} |\n",
        duration_millis(timings.file_discovery)
    ));
    report.push_str(&format!(
        "| Catalog build (read/decode/parse/AST/model) | {} |\n",
        duration_millis(timings.catalog_build)
    ));
    report.push_str(&format!(
        "| Index build | {} |\n",
        duration_millis(timings.index_build)
    ));
    report.push_str(&format!(
        "| Report rendering | {} |\n",
        duration_millis(report_render)
    ));
    report.push_str(&format!(
        "| Total report run | {} |\n\n",
        duration_millis(timings.total + report_render)
    ));
}

fn append_source_kind_counts(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Source Kind Counts\n\n");
    report.push_str("| Source kind | Files |\n");
    report.push_str("| --- | ---: |\n");
    for (kind, count) in index.source_kind_counts() {
        report.push_str(&format!("| `{}` | {} |\n", kind.as_str(), count));
    }
    report.push('\n');
}

fn append_source_category_counts(report: &mut String, index: &SymbolIndex) {
    let mut counts = BTreeMap::<String, usize>::new();
    for file in index.files() {
        *counts
            .entry(file.metadata.category.as_str().to_string())
            .or_default() += 1;
    }
    append_counts(report, "Source Category Counts", &counts, 80);
}

fn append_editor_completion_source_policy(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Editor Completion Source Policy\n\n");
    report.push_str("Editor-facing completion includes runtime/workspace categories by default and keeps excluded categories available through raw/debug lookup.\n\n");
    report.push_str("| Source category | Files | Symbols | Default editor completion |\n");
    report.push_str("| --- | ---: | ---: | --- |\n");

    let mut files = BTreeMap::<SourceCategory, usize>::new();
    let mut symbols = BTreeMap::<SourceCategory, usize>::new();
    for file in index.files() {
        *files.entry(file.metadata.category).or_default() += 1;
    }
    for symbol in index.symbols() {
        let category = index
            .file(symbol.id.file_id)
            .map(|file| file.metadata.category)
            .unwrap_or(SourceCategory::Unknown);
        *symbols.entry(category).or_default() += 1;
    }

    for category in [
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
    ] {
        let file_count = files.get(&category).copied().unwrap_or(0);
        let symbol_count = symbols.get(&category).copied().unwrap_or(0);
        if file_count == 0 && symbol_count == 0 {
            continue;
        }
        report.push_str(&format!(
            "| `{}` | {} | {} | `{}` |\n",
            category.as_str(),
            file_count,
            symbol_count,
            if category.is_editor_completion_default() {
                "included"
            } else {
                "excluded"
            }
        ));
    }
    report.push('\n');
}

fn append_kind_counts(report: &mut String, index: &SymbolIndex) {
    let mut counts = BTreeMap::<String, usize>::new();
    for symbol in index.symbols() {
        *counts
            .entry(kind_name(symbol.kind).to_string())
            .or_default() += 1;
    }
    append_counts(report, "Symbol Kind Frequency", &counts, 80);
}

fn append_presentation_coverage(report: &mut String, index: &SymbolIndex) {
    let total = index.symbols().len();
    let mut with_docs = 0usize;
    let mut with_attributes = 0usize;
    let mut with_modifiers = 0usize;
    let mut with_signatures = 0usize;
    let mut missing_labels = 0usize;
    let mut with_detail = 0usize;
    let mut detail_by_kind = BTreeMap::<String, (usize, usize)>::new();
    let mut missing_detail_samples = Vec::<GlobalSymbolId>::new();
    let mut normal_doc_preview_samples = Vec::<GlobalSymbolId>::new();
    let mut doxygen_doc_preview_samples = Vec::<GlobalSymbolId>::new();

    for symbol in index.symbols() {
        let Some(display) = SymbolDisplay::for_symbol(index, symbol.id) else {
            missing_labels += 1;
            continue;
        };
        let kind = kind_name(symbol.kind).to_string();
        let entry = detail_by_kind.entry(kind).or_default();
        entry.1 += 1;
        if display.label == "<unknown>" {
            missing_labels += 1;
        }
        if !display.doc_comments.is_empty() {
            with_docs += 1;
            if doc_comments_have_doxygen_tag(&display.doc_comments) {
                if doxygen_doc_preview_samples.len() < MAX_SAMPLES {
                    doxygen_doc_preview_samples.push(symbol.id);
                }
            } else if normal_doc_preview_samples.len() < MAX_SAMPLES {
                normal_doc_preview_samples.push(symbol.id);
            }
        }
        if !display.attributes.is_empty() {
            with_attributes += 1;
        }
        if !display.modifiers.is_empty() {
            with_modifiers += 1;
        }
        if display.signature.is_some() {
            with_signatures += 1;
        }
        if display.detail.is_some() {
            with_detail += 1;
            entry.0 += 1;
        } else if missing_detail_samples.len() < MAX_SAMPLES {
            missing_detail_samples.push(symbol.id);
        }
    }

    report.push_str("## Presentation Metadata Coverage\n\n");
    report.push_str("Counts show indexed facts available to future hover, completion detail, document symbols, and debug output. Documentation is copied as raw doc-comment text; previews are bounded display helpers. Attribute counts here are symbol-level attribute applications, so a single source attribute can count more than once when one declaration emits multiple symbols.\n\n");
    report.push_str("| Presentation fact | Symbols | Percent of indexed symbols |\n");
    report.push_str("| --- | ---: | ---: |\n");
    report.push_str(&format!(
        "| Optional detail | {} | {} |\n",
        with_detail,
        percent(with_detail, total)
    ));
    report.push_str(&format!(
        "| Callable signatures | {} | {} |\n",
        with_signatures,
        percent(with_signatures, total)
    ));
    report.push_str(&format!(
        "| Doc comments | {} | {} |\n",
        with_docs,
        percent(with_docs, total)
    ));
    report.push_str(&format!(
        "| Attributes | {} | {} |\n",
        with_attributes,
        percent(with_attributes, total)
    ));
    report.push_str(&format!(
        "| Modifiers | {} | {} |\n",
        with_modifiers,
        percent(with_modifiers, total)
    ));
    report.push_str(&format!(
        "| Missing display labels | {} | {} |\n\n",
        missing_labels,
        percent(missing_labels, total)
    ));

    report.push_str("### Optional Detail Coverage By Kind\n\n");
    report.push_str("Optional detail means a compact extra fact such as a base type, field type, callable signature, enum-member value, typedef target, parameter type/default, or global type. Missing optional detail is expected for enums, classes without base types, and enum members without explicit values.\n\n");
    report.push_str("| Kind | With detail | Total | Percent |\n");
    report.push_str("| --- | ---: | ---: | ---: |\n");
    for (kind, (with_detail, total)) in sorted_detail_coverage(&detail_by_kind) {
        report.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            escape_table(&kind),
            with_detail,
            total,
            percent(with_detail, total)
        ));
    }
    report.push('\n');

    report.push_str("### Missing Optional Detail By Kind\n\n");
    let mut missing_by_kind = BTreeMap::<String, usize>::new();
    for (kind, (with_detail, total)) in &detail_by_kind {
        let missing = total.saturating_sub(*with_detail);
        if missing > 0 {
            missing_by_kind.insert(kind.clone(), missing);
        }
    }
    if missing_by_kind.is_empty() {
        report.push_str("None.\n\n");
    } else {
        report.push_str("| Kind | Missing optional detail |\n");
        report.push_str("| --- | ---: |\n");
        for (kind, count) in sorted_counts(&missing_by_kind).into_iter().take(80) {
            report.push_str(&format!("| `{}` | {} |\n", escape_table(&kind), count));
        }
        report.push('\n');
    }

    append_display_sample_table(
        report,
        index,
        "Missing Optional Detail Samples",
        &missing_detail_samples,
        false,
    );
    append_display_sample_table(
        report,
        index,
        "Doc Preview Quality Samples - Normal",
        &normal_doc_preview_samples,
        true,
    );
    append_display_sample_table(
        report,
        index,
        "Doc Preview Quality Samples - Doxygen Tag Source",
        &doxygen_doc_preview_samples,
        true,
    );
}

fn sorted_detail_coverage(
    counts: &BTreeMap<String, (usize, usize)>,
) -> Vec<(String, (usize, usize))> {
    let mut rows = counts
        .iter()
        .map(|(kind, counts)| (kind.clone(), *counts))
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let left_missing = left.1 .1.saturating_sub(left.1 .0);
        let right_missing = right.1 .1.saturating_sub(right.1 .0);
        right_missing
            .cmp(&left_missing)
            .then_with(|| right.1 .1.cmp(&left.1 .1))
            .then_with(|| left.0.cmp(&right.0))
    });
    rows
}

fn append_display_sample_table(
    report: &mut String,
    index: &SymbolIndex,
    heading: &str,
    samples: &[GlobalSymbolId],
    include_preview: bool,
) {
    report.push_str(&format!("### {heading}\n\n"));
    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    let include_raw_doc = include_preview && heading.contains("Doxygen Tag Source");
    if include_raw_doc {
        report
            .push_str("| Kind | Name | Path | Line | Category | Raw tag line | Clean preview |\n");
        report.push_str("| --- | --- | --- | ---: | --- | --- | --- |\n");
    } else if include_preview {
        report.push_str("| Kind | Name | Path | Line | Category | Preview |\n");
        report.push_str("| --- | --- | --- | ---: | --- | --- |\n");
    } else {
        report.push_str("| Kind | Name | Path | Line | Category |\n");
        report.push_str("| --- | --- | --- | ---: | --- |\n");
    }

    for id in samples {
        let Some(symbol) = index.symbol(*id) else {
            continue;
        };
        let Some(file) = index.file(id.file_id) else {
            continue;
        };
        let display = SymbolDisplay::for_symbol(index, *id);
        let name = symbol.name.as_deref().unwrap_or("<unknown>");
        let path = file
            .metadata
            .relative_path
            .as_ref()
            .or(file.metadata.absolute_path.as_ref())
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| format!("file {}", id.file_id.0));
        let line = symbol_line(index, *id)
            .map(|line| line.to_string())
            .unwrap_or_else(|| "?".to_string());
        if include_raw_doc {
            let raw_tag_line = display
                .as_ref()
                .and_then(|display| first_doxygen_tag_line(&display.doc_comments))
                .unwrap_or_default();
            let preview = display
                .and_then(|display| display.documentation_preview)
                .unwrap_or_default();
            report.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} | `{}` | `{}` | `{}` |\n",
                kind_name(symbol.kind),
                escape_table(name),
                escape_table(&path),
                line,
                file.metadata.category.as_str(),
                escape_table(&raw_tag_line),
                escape_table(&preview)
            ));
        } else if include_preview {
            let preview = display
                .and_then(|display| display.documentation_preview)
                .unwrap_or_default();
            report.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} | `{}` | `{}` |\n",
                kind_name(symbol.kind),
                escape_table(name),
                escape_table(&path),
                line,
                file.metadata.category.as_str(),
                escape_table(&preview)
            ));
        } else {
            report.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} | `{}` |\n",
                kind_name(symbol.kind),
                escape_table(name),
                escape_table(&path),
                line,
                file.metadata.category.as_str()
            ));
        }
    }
    report.push('\n');
}

fn doc_comments_have_doxygen_tag(
    comments: &[reforger_language_server::index::IndexedDocComment],
) -> bool {
    first_doxygen_tag_line(comments).is_some()
}

fn first_doxygen_tag_line(
    comments: &[reforger_language_server::index::IndexedDocComment],
) -> Option<String> {
    comments
        .iter()
        .flat_map(|comment| comment.text.lines())
        .map(raw_doc_line_without_markers)
        .find(|line| is_doxygen_tag_line(line))
}

fn raw_doc_line_without_markers(line: &str) -> String {
    let mut value = line.trim();
    value = value.strip_prefix("//!").unwrap_or(value).trim_start();
    value = value.strip_prefix("/*!").unwrap_or(value).trim_start();
    value = value.strip_prefix('*').unwrap_or(value).trim_start();
    value = value.strip_suffix("*/").unwrap_or(value).trim_end();
    value.trim().to_string()
}

fn is_doxygen_tag_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    [
        "\\param",
        "\\return",
        "\\returns",
        "\\code",
        "\\brief",
        "\\warning",
        "\\note",
    ]
    .iter()
    .any(|tag| trimmed.starts_with(tag))
}

fn append_duplicate_classification_summary(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Duplicate Classification Summary\n\n");
    report.push_str("Duplicate classifications are review buckets only. They help separate expected source patterns from conflicts that may need semantic handling later.\n\n");

    let mut counts = BTreeMap::<String, usize>::new();
    for (_, symbols) in index.duplicate_top_level_names() {
        *counts
            .entry(duplicate_classification_label(classify_duplicate(index, symbols)).to_string())
            .or_default() += 1;
    }

    if counts.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Classification | Groups |\n");
    report.push_str("| --- | ---: |\n");
    for classification in [
        DuplicateClassification::TypedefFunctionDelegatePair,
        DuplicateClassification::TypedefClassWrapperPattern,
        DuplicateClassification::GeneratedVsSourceDuplicate,
        DuplicateClassification::SuspiciousSameKindDuplicate,
        DuplicateClassification::MixedKindDuplicate,
        DuplicateClassification::WorkspaceOverlay,
    ] {
        let label = duplicate_classification_label(classification);
        report.push_str(&format!(
            "| `{}` | {} |\n",
            label,
            counts.get(label).copied().unwrap_or(0)
        ));
    }
    report.push('\n');
}

fn append_focused_suspicious_conflict_report(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Focused Suspicious Conflict Report\n\n");
    report.push_str("These sections isolate the highest-risk index facts from the broader review tables. They are review targets, not semantic errors by themselves.\n\n");
    append_suspicious_top_level_duplicate_provenance(report, index);
    append_suspicious_top_level_duplicates(report, index);
    append_same_owner_shadow_conflict_classification(report, index);
    append_completion_filtering_decision(report, index);
    append_suspicious_same_owner_shadow_conflicts(report, index);
    append_unknown_conflict_snippets(report, index);
}

fn suspicious_top_level_rows(
    index: &SymbolIndex,
) -> Vec<(
    String,
    DuplicateClassification,
    TopLevelProvenance,
    Vec<GlobalSymbolId>,
)> {
    let mut rows = index
        .duplicate_top_level_names()
        .into_iter()
        .filter_map(|(name, symbols)| {
            let classification = classify_duplicate(index, symbols);
            if !matches!(
                classification,
                DuplicateClassification::SuspiciousSameKindDuplicate
                    | DuplicateClassification::MixedKindDuplicate
            ) {
                return None;
            }
            Some((
                name.to_string(),
                classification,
                classify_top_level_provenance(index, symbols),
                symbols.to_vec(),
            ))
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| {
        left.2
            .cmp(&right.2)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| right.3.len().cmp(&left.3.len()))
    });
    rows
}

fn append_suspicious_top_level_duplicate_provenance(report: &mut String, index: &SymbolIndex) {
    report.push_str("### Suspicious Top-Level Duplicate Provenance\n\n");
    report.push_str("This re-buckets suspicious duplicate declarations by source shape so generated/docs/test patterns do not look the same as unknown conflicts.\n\n");

    let rows = suspicious_top_level_rows(index);
    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    let mut counts = BTreeMap::<String, usize>::new();
    for (_, _, provenance, _) in &rows {
        *counts
            .entry(top_level_provenance_label(*provenance).to_string())
            .or_default() += 1;
    }
    append_counts_with_level(report, "Top-Level Provenance Buckets", &counts, 80, 4);

    report.push_str("| Name | Provenance | Classification | Declarations |\n");
    report.push_str("| --- | --- | --- | --- |\n");
    for (name, classification, provenance, symbols) in
        rows.into_iter().take(MAX_SUSPICIOUS_DUPLICATE_ROWS)
    {
        let mut declarations = symbols
            .iter()
            .take(MAX_DECLARATIONS_PER_DUPLICATE)
            .map(|id| display_symbol_location_with_category(index, *id))
            .collect::<Vec<_>>();
        if symbols.len() > MAX_DECLARATIONS_PER_DUPLICATE {
            declarations.push(format!(
                "... {} more",
                symbols.len() - MAX_DECLARATIONS_PER_DUPLICATE
            ));
        }

        report.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} |\n",
            escape_table(&name),
            top_level_provenance_label(provenance),
            duplicate_classification_label(classification),
            declarations.join("<br>")
        ));
    }
    report.push('\n');
}

fn append_suspicious_top_level_duplicates(report: &mut String, index: &SymbolIndex) {
    report.push_str("### Suspicious Top-Level Duplicates\n\n");
    report.push_str("Includes same-kind duplicate declarations and mixed-kind duplicate declarations that do not match the known typedef/function, typedef/class, generated/non-generated, or workspace-overlay buckets.\n\n");

    let rows = suspicious_top_level_rows(index);

    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str(&format!(
        "Total suspicious top-level groups: {}\n\n",
        rows.len()
    ));
    report.push_str("| Name | Classification | Count | Declarations |\n");
    report.push_str("| --- | --- | ---: | --- |\n");
    for (name, classification, _, symbols) in rows.into_iter().take(MAX_SUSPICIOUS_DUPLICATE_ROWS) {
        let mut declarations = symbols
            .iter()
            .take(MAX_DECLARATIONS_PER_DUPLICATE)
            .map(|id| display_symbol_location(index, *id))
            .collect::<Vec<_>>();
        if symbols.len() > MAX_DECLARATIONS_PER_DUPLICATE {
            declarations.push(format!(
                "... {} more",
                symbols.len() - MAX_DECLARATIONS_PER_DUPLICATE
            ));
        }
        report.push_str(&format!(
            "| `{}` | `{}` | {} | {} |\n",
            escape_table(&name),
            duplicate_classification_label(classification),
            symbols.len(),
            declarations.join("<br>")
        ));
    }
    report.push('\n');
}

fn append_same_owner_shadow_conflict_classification(report: &mut String, index: &SymbolIndex) {
    report.push_str("### Same-Owner Shadow Conflict Classification\n\n");
    report.push_str("This classifies the preferred-class completion shadow groups where the kept member hides another member from the same owner name.\n\n");

    let mut rows = same_owner_shadow_conflict_rows(index);
    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    let mut counts = BTreeMap::<String, usize>::new();
    for (_, provenance, _) in &rows {
        *counts
            .entry(shadow_provenance_label(*provenance).to_string())
            .or_default() += 1;
    }
    append_counts_with_level(report, "Same-Owner Conflict Buckets", &counts, 80, 4);

    rows.sort_by(|left, right| {
        left.1
            .cmp(&right.1)
            .then_with(|| right.2.shadowed.len().cmp(&left.2.shadowed.len()))
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.2.key.cmp(&right.2.key))
    });

    report
        .push_str("| Class | Member key | Likely cause | Kept | Hidden same-owner candidates |\n");
    report.push_str("| --- | --- | --- | --- | --- |\n");
    for (_, provenance, sample) in rows.into_iter().take(MAX_SUSPICIOUS_SHADOW_ROWS) {
        let same_owner_hidden = same_owner_shadowed_ids(index, &sample);
        let mut hidden = same_owner_hidden
            .iter()
            .take(MAX_SHADOWED_PER_GROUP)
            .map(|id| display_symbol_location_with_category(index, *id))
            .collect::<Vec<_>>();
        if same_owner_hidden.len() > MAX_SHADOWED_PER_GROUP {
            hidden.push(format!(
                "... {} more",
                same_owner_hidden.len() - MAX_SHADOWED_PER_GROUP
            ));
        }

        report.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} | {} |\n",
            escape_table(&sample.owner),
            escape_table(&sample.key),
            shadow_provenance_label(provenance),
            display_symbol_location_with_category(index, sample.kept),
            hidden.join("<br>")
        ));
    }
    report.push('\n');
}

fn append_completion_filtering_decision(report: &mut String, index: &SymbolIndex) {
    report.push_str("### Editor Completion Filtering Decision\n\n");
    report.push_str("This section translates same-owner conflict classifications into the current editor-completion policy. The policy is intentionally conservative: keep every source symbol indexed for debug/navigation, collapse duplicate completion items only in the editor-facing completion view, and require a later semantic/preprocessor slice before deleting or suppressing source facts globally.\n\n");

    let rows = same_owner_shadow_conflict_rows(index);
    if rows.is_empty() {
        report.push_str(
            "No same-owner completion conflicts were found. No filtering decision is needed.\n\n",
        );
        return;
    }

    let mut counts = BTreeMap::<ShadowProvenance, usize>::new();
    for (_, provenance, _) in &rows {
        *counts.entry(*provenance).or_default() += 1;
    }

    report.push_str("| Likely cause | Groups | Editor completion decision | Why | Future requirement before stronger filtering |\n");
    report.push_str("| --- | ---: | --- | --- | --- |\n");
    for provenance in [
        ShadowProvenance::PreprocessorPrototypeDuplicate,
        ShadowProvenance::PreprocessorBranchDuplicate,
        ShadowProvenance::PrototypeDeclarationBlock,
        ShadowProvenance::DocsDoxygenOnlySource,
        ShadowProvenance::GeneratedSourceOverlap,
        ShadowProvenance::InheritedAggregateArtifact,
        ShadowProvenance::Unknown,
    ] {
        let count = counts.get(&provenance).copied().unwrap_or(0);
        if count == 0 {
            continue;
        }
        report.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            shadow_provenance_label(provenance),
            count,
            completion_filtering_decision(provenance),
            completion_filtering_rationale(provenance),
            completion_filtering_future_requirement(provenance)
        ));
    }
    report.push('\n');
    report.push_str("Decision summary: the current preferred-class completion path may collapse all listed duplicate completion keys into one visible item, but raw aggregate/debug lookup must continue exposing every candidate. Conditional context is descriptive only; preprocessor branch duplicates are not active-branch truth without Workbench-backed validation or future macro evaluation.\n\n");
}

fn append_suspicious_same_owner_shadow_conflicts(report: &mut String, index: &SymbolIndex) {
    report.push_str("### Classified Same-Owner Completion Shadows\n\n");
    report.push_str("These are completion de-duplication groups where the kept member hides another member from the same owner class name. Classified rows have an understood source-shape cause; they remain debug-visible but are not automatically high-risk.\n\n");

    let raw_samples = collect_completion_shadow_samples(index, false);
    let preferred_samples = collect_completion_shadow_samples(index, true);
    let raw_conflict_count = raw_samples
        .iter()
        .filter(|sample| is_same_owner_shadow_conflict(index, sample))
        .count();
    let preferred_conflict_count = preferred_samples
        .iter()
        .filter(|sample| is_same_owner_shadow_conflict(index, sample))
        .count();

    report.push_str("| Completion view | Classified same-owner shadow groups |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Raw owner-name aggregate | {} |\n",
        raw_conflict_count
    ));
    report.push_str(&format!(
        "| Preferred-class editor path | {} |\n\n",
        preferred_conflict_count
    ));

    let mut rows = preferred_samples
        .into_iter()
        .filter(|sample| is_same_owner_shadow_conflict(index, sample))
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| {
        right
            .shadowed
            .len()
            .cmp(&left.shadowed.len())
            .then_with(|| left.owner.cmp(&right.owner))
            .then_with(|| left.key.cmp(&right.key))
    });

    if rows.is_empty() {
        report.push_str("No preferred-class same-owner shadows.\n\n");
        return;
    }

    report.push_str("Preferred-class classified same-owner shadow details:\n\n");
    report.push_str(
        "| Class | Member key | Same-owner hidden | Kept | Hidden same-owner candidates |\n",
    );
    report.push_str("| --- | --- | ---: | --- | --- |\n");
    for sample in rows.into_iter().take(MAX_SUSPICIOUS_SHADOW_ROWS) {
        let same_owner_hidden = same_owner_shadowed_ids(index, &sample);
        let mut hidden = same_owner_hidden
            .iter()
            .take(MAX_SHADOWED_PER_GROUP)
            .map(|id| display_symbol_location(index, *id))
            .collect::<Vec<_>>();
        if same_owner_hidden.len() > MAX_SHADOWED_PER_GROUP {
            hidden.push(format!(
                "... {} more",
                same_owner_hidden.len() - MAX_SHADOWED_PER_GROUP
            ));
        }

        report.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} |\n",
            escape_table(&sample.owner),
            escape_table(&sample.key),
            same_owner_hidden.len(),
            display_symbol_location(index, sample.kept),
            hidden.join("<br>")
        ));
    }
    report.push('\n');

    report.push_str("### Unknown / High-Risk Same-Owner Shadows\n\n");
    let unknown_rows = same_owner_shadow_conflict_rows(index)
        .into_iter()
        .filter(|(_, provenance, _)| *provenance == ShadowProvenance::Unknown)
        .collect::<Vec<_>>();
    if unknown_rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Class | Member key | Kept | Hidden same-owner candidates |\n");
    report.push_str("| --- | --- | --- | --- |\n");
    for (_, _, sample) in unknown_rows.into_iter().take(MAX_SUSPICIOUS_SHADOW_ROWS) {
        let same_owner_hidden = same_owner_shadowed_ids(index, &sample);
        let hidden = same_owner_hidden
            .iter()
            .take(MAX_SHADOWED_PER_GROUP)
            .map(|id| display_symbol_location(index, *id))
            .collect::<Vec<_>>();
        report.push_str(&format!(
            "| `{}` | `{}` | {} | {} |\n",
            escape_table(&sample.owner),
            escape_table(&sample.key),
            display_symbol_location(index, sample.kept),
            hidden.join("<br>")
        ));
    }
    report.push('\n');
}

fn append_unknown_conflict_snippets(report: &mut String, index: &SymbolIndex) {
    report.push_str("### Unknown Conflict Snippets\n\n");
    report.push_str("Only conflicts that could not be classified by path/source-shape heuristics are shown here. Snippets are bounded and are for review only.\n\n");

    let mut snippets = Vec::new();

    for (name, _, provenance, symbols) in suspicious_top_level_rows(index) {
        if provenance != TopLevelProvenance::Unknown {
            continue;
        }
        for id in symbols.into_iter().take(MAX_DECLARATIONS_PER_DUPLICATE) {
            if let Some(snippet) = symbol_snippet(index, id) {
                snippets.push((
                    format!("top-level duplicate `{name}`"),
                    display_symbol_location_with_category(index, id),
                    snippet,
                ));
            }
            if snippets.len() >= MAX_UNKNOWN_CONFLICT_SNIPPETS {
                break;
            }
        }
        if snippets.len() >= MAX_UNKNOWN_CONFLICT_SNIPPETS {
            break;
        }
    }

    if snippets.len() < MAX_UNKNOWN_CONFLICT_SNIPPETS {
        for (_, provenance, sample) in same_owner_shadow_conflict_rows(index) {
            if provenance != ShadowProvenance::Unknown {
                continue;
            }
            let ids = std::iter::once(sample.kept)
                .chain(same_owner_shadowed_ids(index, &sample).into_iter())
                .collect::<Vec<_>>();
            for id in ids {
                if let Some(snippet) = symbol_snippet(index, id) {
                    snippets.push((
                        format!("same-owner shadow `{}`.`{}`", sample.owner, sample.key),
                        display_symbol_location_with_category(index, id),
                        snippet,
                    ));
                }
                if snippets.len() >= MAX_UNKNOWN_CONFLICT_SNIPPETS {
                    break;
                }
            }
            if snippets.len() >= MAX_UNKNOWN_CONFLICT_SNIPPETS {
                break;
            }
        }
    }

    if snippets.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    for (heading, location, snippet) in snippets {
        report.push_str(&format!(
            "#### {} - {}\n\n",
            escape_inline(&heading),
            location
        ));
        report.push_str("````enforce\n");
        report.push_str(&snippet);
        report.push_str("````\n\n");
    }
}

fn same_owner_shadow_conflict_rows(
    index: &SymbolIndex,
) -> Vec<(String, ShadowProvenance, MemberShadowSample)> {
    collect_completion_shadow_samples(index, true)
        .into_iter()
        .filter(|sample| is_same_owner_shadow_conflict(index, sample))
        .map(|sample| {
            (
                sample.owner.clone(),
                classify_same_owner_shadow_provenance(index, &sample),
                sample,
            )
        })
        .collect()
}

fn append_duplicate_top_level_names(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Duplicate Top-Level Name Groups\n\n");
    let mut duplicates = index.duplicate_top_level_names();
    duplicates.sort_by(|left, right| {
        right
            .1
            .len()
            .cmp(&left.1.len())
            .then_with(|| left.0.cmp(right.0))
    });

    if duplicates.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Name | Classification | Count | Declarations |\n");
    report.push_str("| --- | --- | ---: | --- |\n");
    for (name, symbols) in duplicates.into_iter().take(MAX_DUPLICATE_NAMES) {
        let classification = duplicate_classification_label(classify_duplicate(index, symbols));
        let mut declarations = symbols
            .iter()
            .take(MAX_DECLARATIONS_PER_DUPLICATE)
            .map(|id| display_symbol_location(index, *id))
            .collect::<Vec<_>>();
        if symbols.len() > MAX_DECLARATIONS_PER_DUPLICATE {
            declarations.push(format!(
                "... {} more",
                symbols.len() - MAX_DECLARATIONS_PER_DUPLICATE
            ));
        }
        report.push_str(&format!(
            "| `{}` | `{}` | {} | {} |\n",
            escape_table(name),
            classification,
            symbols.len(),
            declarations.join("<br>")
        ));
    }
    report.push('\n');
}

fn append_preferred_duplicate_samples(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Preferred Duplicate Samples\n\n");
    let mut rows = Vec::new();
    for (name, symbols) in index.duplicate_top_level_names() {
        if symbols.len() < 2 || rows.len() >= MAX_SAMPLES {
            continue;
        }
        if let Some(preferred) = index
            .preferred_top_level_symbols_for_name(name)
            .first()
            .copied()
        {
            rows.push((name.to_string(), preferred));
        }
    }

    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Name | Classification | Preferred declaration |\n");
    report.push_str("| --- | --- | --- |\n");
    for (name, id) in rows {
        let classification = duplicate_classification_label(classify_duplicate(
            index,
            index.top_level_symbols_for_name(&name),
        ));
        report.push_str(&format!(
            "| `{}` | `{}` | {} |\n",
            escape_table(&name),
            classification,
            display_symbol_location(index, id)
        ));
    }
    report.push('\n');
}

fn append_lookup_samples(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Lookup Samples\n\n");
    append_sample_table(report, index, "Class Lookup Samples", sample_classes(index));
    append_sample_table(
        report,
        index,
        "Typedef Lookup Samples",
        sample_typedefs(index),
    );
    append_method_group_samples(report, sample_method_groups(index));
}

fn append_completion_member_shadow_groups(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Raw Aggregate Completion Shadows\n\n");
    report.push_str("These groups show raw inherited class-member candidates that are hidden from the completion-ready member view by kind/name/signature de-duplication.\n\n");

    let mut samples = collect_completion_shadow_samples(index, false);
    let total_groups = samples.len();
    let mut kind_counts = BTreeMap::<String, usize>::new();
    for sample in &samples {
        if let Some(symbol) = index.symbol(sample.kept) {
            *kind_counts
                .entry(kind_name(symbol.kind).to_string())
                .or_default() += 1;
        }
    }

    report.push_str(&format!("Total shadow groups: {total_groups}\n\n"));
    append_shadow_kind_counts(report, &kind_counts);

    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    append_shadow_review_summary(report, index, &samples);

    samples.sort_by(|left, right| {
        right
            .shadowed
            .len()
            .cmp(&left.shadowed.len())
            .then_with(|| left.owner.cmp(&right.owner))
            .then_with(|| left.key.cmp(&right.key))
    });

    report.push_str("| Class | Member key | Classification | Kept | Hidden candidates |\n");
    report.push_str("| --- | --- | --- | --- | --- |\n");
    for sample in samples.into_iter().take(MAX_MEMBER_SHADOW_GROUPS) {
        let mut hidden = sample
            .shadowed
            .iter()
            .take(MAX_SHADOWED_PER_GROUP)
            .map(|id| display_symbol_location(index, *id))
            .collect::<Vec<_>>();
        if sample.shadowed.len() > MAX_SHADOWED_PER_GROUP {
            hidden.push(format!(
                "... {} more",
                sample.shadowed.len() - MAX_SHADOWED_PER_GROUP
            ));
        }

        report.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} | {} |\n",
            escape_table(&sample.owner),
            escape_table(&sample.key),
            shadow_classification(index, &sample),
            display_symbol_location(index, sample.kept),
            hidden.join("<br>")
        ));
    }
    report.push('\n');
}

fn append_preferred_class_completion_shadow_summary(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Preferred-Class Completion Shadows\n\n");
    report.push_str("This summarizes the future editor-facing completion path: preferred class declarations first, lower-priority same-owner overlays next, then exact-name base-chain members.\n\n");

    let mut samples = collect_completion_shadow_samples(index, true);
    let total_groups = samples.len();
    let mut kind_counts = BTreeMap::<String, usize>::new();
    for sample in &samples {
        if let Some(symbol) = index.symbol(sample.kept) {
            *kind_counts
                .entry(kind_name(symbol.kind).to_string())
                .or_default() += 1;
        }
    }

    report.push_str(&format!("Total shadow groups: {total_groups}\n\n"));
    append_shadow_kind_counts(report, &kind_counts);

    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    append_shadow_review_summary(report, index, &samples);

    samples.sort_by(|left, right| {
        right
            .shadowed
            .len()
            .cmp(&left.shadowed.len())
            .then_with(|| left.owner.cmp(&right.owner))
            .then_with(|| left.key.cmp(&right.key))
    });

    report.push_str("| Class | Member key | Classification | Kept | Hidden candidates |\n");
    report.push_str("| --- | --- | --- | --- | --- |\n");
    for sample in samples.into_iter().take(MAX_SAMPLES) {
        let mut hidden = sample
            .shadowed
            .iter()
            .take(MAX_SHADOWED_PER_GROUP)
            .map(|id| display_symbol_location(index, *id))
            .collect::<Vec<_>>();
        if sample.shadowed.len() > MAX_SHADOWED_PER_GROUP {
            hidden.push(format!(
                "... {} more",
                sample.shadowed.len() - MAX_SHADOWED_PER_GROUP
            ));
        }

        report.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} | {} |\n",
            escape_table(&sample.owner),
            escape_table(&sample.key),
            shadow_classification(index, &sample),
            display_symbol_location(index, sample.kept),
            hidden.join("<br>")
        ));
    }
    report.push('\n');
}

fn collect_completion_shadow_samples(
    index: &SymbolIndex,
    preferred_class_view: bool,
) -> Vec<MemberShadowSample> {
    let mut samples = Vec::new();
    let mut class_names = BTreeSet::new();
    for class_id in index.symbols_for_kind(SymbolKind::Class) {
        if let Some(name) = index
            .symbol(*class_id)
            .and_then(|symbol| symbol.name.as_deref())
        {
            class_names.insert(name.to_string());
        }
    }

    for class_name in class_names {
        let completion = if preferred_class_view {
            index.completion_members_for_preferred_class(&class_name)
        } else {
            index.raw_completion_members_for_owner_name(&class_name)
        };
        for group in completion.shadowed_groups {
            samples.push(MemberShadowSample {
                owner: class_name.clone(),
                key: group.key,
                kept: group.kept,
                shadowed: group.shadowed,
            });
        }
    }

    samples
}

fn append_shadow_kind_counts(report: &mut String, counts: &BTreeMap<String, usize>) {
    report.push_str("| Shadow kind | Groups |\n");
    report.push_str("| --- | ---: |\n");
    for kind in ["Method", "Field", "Constructor", "Destructor"] {
        report.push_str(&format!(
            "| `{kind}` | {} |\n",
            counts.get(kind).copied().unwrap_or(0)
        ));
    }
    report.push('\n');
}

fn append_shadow_review_summary(
    report: &mut String,
    index: &SymbolIndex,
    samples: &[MemberShadowSample],
) {
    let mut method_counts = BTreeMap::<String, usize>::new();
    let mut class_counts = BTreeMap::<String, usize>::new();
    let mut matrix_counts = BTreeMap::<String, usize>::new();
    let mut classification_counts = BTreeMap::<String, usize>::new();

    for sample in samples {
        *class_counts.entry(sample.owner.clone()).or_default() += sample.shadowed.len();
        *classification_counts
            .entry(shadow_classification(index, sample).to_string())
            .or_default() += 1;

        if index
            .symbol(sample.kept)
            .is_some_and(|symbol| symbol.kind == SymbolKind::Method)
        {
            let name = index
                .symbol(sample.kept)
                .and_then(|symbol| symbol.name.as_deref())
                .unwrap_or("<unknown>")
                .to_string();
            *method_counts.entry(name).or_default() += sample.shadowed.len();
        }

        let kept_source = source_kind_name(index, sample.kept);
        for hidden in &sample.shadowed {
            let hidden_source = source_kind_name(index, *hidden);
            *matrix_counts
                .entry(format!("{kept_source} kept / {hidden_source} hidden"))
                .or_default() += 1;
        }
    }

    append_counts_with_level(
        report,
        "Top Shadowed Method Names",
        &method_counts,
        MAX_SAMPLES,
        3,
    );
    append_counts_with_level(
        report,
        "Top Classes With Shadow Groups",
        &class_counts,
        MAX_SAMPLES,
        3,
    );
    append_counts_with_level(
        report,
        "Shadow Source Kind Matrix",
        &matrix_counts,
        MAX_SAMPLES,
        3,
    );
    append_counts_with_level(
        report,
        "Shadow Review Classification",
        &classification_counts,
        MAX_SAMPLES,
        3,
    );
}

fn shadow_classification(index: &SymbolIndex, sample: &MemberShadowSample) -> &'static str {
    let kept_owner = owner_name(index, sample.kept);
    if sample
        .shadowed
        .iter()
        .any(|hidden| owner_name(index, *hidden) == kept_owner)
    {
        "suspicious same-owner conflict"
    } else {
        "expected inherited/base shadow"
    }
}

fn is_same_owner_shadow_conflict(index: &SymbolIndex, sample: &MemberShadowSample) -> bool {
    !same_owner_shadowed_ids(index, sample).is_empty()
}

fn same_owner_shadowed_ids(
    index: &SymbolIndex,
    sample: &MemberShadowSample,
) -> Vec<GlobalSymbolId> {
    let kept_owner = owner_name(index, sample.kept);
    sample
        .shadowed
        .iter()
        .copied()
        .filter(|hidden| owner_name(index, *hidden) == kept_owner)
        .collect()
}

fn owner_name(index: &SymbolIndex, id: GlobalSymbolId) -> Option<&str> {
    index
        .symbol(id)
        .and_then(|symbol| symbol.parent)
        .and_then(|parent| index.symbol(parent))
        .and_then(|parent| parent.name.as_deref())
}

fn source_kind_name(index: &SymbolIndex, id: GlobalSymbolId) -> &'static str {
    index
        .file(id.file_id)
        .map(|file| file.metadata.kind.as_str())
        .unwrap_or("Unknown")
}

fn sample_classes(index: &SymbolIndex) -> Vec<SymbolSample> {
    sample_by_kind(index, SymbolKind::Class)
}

fn sample_typedefs(index: &SymbolIndex) -> Vec<SymbolSample> {
    sample_by_kind(index, SymbolKind::Typedef)
}

fn sample_by_kind(index: &SymbolIndex, kind: SymbolKind) -> Vec<SymbolSample> {
    index
        .symbols_for_kind(kind)
        .iter()
        .take(MAX_SAMPLES)
        .filter_map(|id| {
            let symbol = index.symbol(*id)?;
            Some(SymbolSample {
                id: *id,
                kind,
                name: symbol
                    .name
                    .clone()
                    .unwrap_or_else(|| "<unknown>".to_string()),
                detail: detail_text(index, *id),
            })
        })
        .collect()
}

fn sample_method_groups(index: &SymbolIndex) -> Vec<MethodGroupSample> {
    let mut seen = BTreeSet::new();
    let mut samples = Vec::new();

    index
        .symbols_for_kind(SymbolKind::Method)
        .iter()
        .filter_map(|id| method_group_key(index, *id))
        .for_each(|(owner, name)| {
            if samples.len() >= MAX_SAMPLES || !seen.insert((owner.clone(), name.clone())) {
                return;
            }

            let ids = index.methods_by_owner_name(&owner, &name);
            let Some(first_id) = ids.first().copied() else {
                return;
            };
            samples.push(MethodGroupSample {
                owner: owner.clone(),
                name: name.clone(),
                overloads: ids.len(),
                path: display_sample_location(index, first_id),
                signatures: method_signatures(index, ids),
            });
        });

    samples
}

fn method_group_key(index: &SymbolIndex, id: GlobalSymbolId) -> Option<(String, String)> {
    let symbol = index.symbol(id)?;
    let owner = symbol
        .parent
        .and_then(|parent| index.symbol(parent))
        .and_then(|parent| parent.name.as_ref())?;
    let name = symbol.name.as_ref()?;
    Some((owner.clone(), name.clone()))
}

fn append_sample_table(
    report: &mut String,
    index: &SymbolIndex,
    heading: &str,
    samples: Vec<SymbolSample>,
) {
    report.push_str(&format!("### {heading}\n\n"));
    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Kind | Name | Location | Detail |\n");
    report.push_str("| --- | --- | --- | --- |\n");
    for sample in samples {
        report.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` |\n",
            kind_name(sample.kind),
            escape_table(&sample.name),
            display_sample_location(index, sample.id),
            escape_table(&sample.detail)
        ));
    }
    report.push('\n');
}

fn append_method_group_samples(report: &mut String, samples: Vec<MethodGroupSample>) {
    report.push_str("### Method Owner/Name Samples\n\n");
    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Owner.Method | Overloads | Path | Signatures |\n");
    report.push_str("| --- | ---: | --- | --- |\n");
    for sample in samples {
        report.push_str(&format!(
            "| `{}` | {} | `{}` | `{}` |\n",
            escape_table(&format!("{}.{}", sample.owner, sample.name)),
            sample.overloads,
            escape_table(&sample.path),
            escape_table(&sample.signatures.join("<br>"))
        ));
    }
    report.push('\n');
}

fn display_symbol_location(index: &SymbolIndex, id: GlobalSymbolId) -> String {
    let Some(symbol) = index.symbol(id) else {
        return format!("missing symbol {:?}", id);
    };
    let Some(file) = index.file(id.file_id) else {
        return format!("missing file {:?}", id.file_id);
    };
    let path = file
        .metadata
        .relative_path
        .as_ref()
        .or(file.metadata.absolute_path.as_ref())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unknown-path>".to_string());
    let name = symbol.name.as_deref().unwrap_or("<unknown>");
    let detail = detail_text(index, id);
    let detail_suffix = if detail.is_empty() {
        String::new()
    } else {
        format!(" {}", escape_table(&detail))
    };
    let symbol_suffix = symbol_metadata_suffix(symbol);
    format!(
        "{} `{}` in `{}` priority {}{}{}",
        kind_name(symbol.kind),
        escape_table(name),
        path,
        file.metadata.priority,
        symbol_suffix,
        detail_suffix
    )
}

fn display_symbol_location_with_category(index: &SymbolIndex, id: GlobalSymbolId) -> String {
    let base = display_symbol_location(index, id);
    let category = source_category_for_symbol(index, id).as_str();
    let line = symbol_line(index, id)
        .map(|line| line.to_string())
        .unwrap_or_else(|| "?".to_string());
    format!("{base} category `{category}` line {line}")
}

fn display_sample_location(index: &SymbolIndex, id: GlobalSymbolId) -> String {
    let Some(file) = index.file(id.file_id) else {
        return format!("file {} symbol {}", id.file_id.0, id.symbol_id.0);
    };
    let path = file
        .metadata
        .relative_path
        .as_ref()
        .or(file.metadata.absolute_path.as_ref())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| format!("file {}", id.file_id.0));
    format!("{path} #{}", id.symbol_id.0)
}

fn symbol_metadata_suffix(symbol: &IndexedSymbol) -> String {
    let mut values = Vec::new();
    if let Some(form) = symbol.callable_form {
        values.push(format!("form: {}", form.as_str()));
    }
    if !symbol.conditional_context.is_empty() {
        let context = symbol
            .conditional_context
            .iter()
            .map(|branch| {
                branch
                    .condition
                    .as_deref()
                    .map(|condition| format!("{} {}", branch.kind.as_str(), condition))
                    .unwrap_or_else(|| branch.kind.as_str().to_string())
            })
            .collect::<Vec<_>>()
            .join(" > ");
        values.push(format!("condition: {context}"));
    }

    if values.is_empty() {
        String::new()
    } else {
        format!(" {}", values.join(" "))
    }
}

fn source_category_for_symbol(index: &SymbolIndex, id: GlobalSymbolId) -> SourceCategory {
    index
        .file(id.file_id)
        .map(|file| file.metadata.category)
        .unwrap_or(SourceCategory::Unknown)
}

fn normalized_file_path(file: &IndexedFile) -> String {
    file.metadata
        .relative_path
        .as_ref()
        .or(file.metadata.absolute_path.as_ref())
        .map(|path| path.to_string_lossy().replace('\\', "/").to_lowercase())
        .unwrap_or_default()
}

fn source_path(index: &SymbolIndex, id: GlobalSymbolId) -> Option<PathBuf> {
    index
        .file(id.file_id)
        .and_then(|file| file.metadata.absolute_path.clone())
}

fn source_text(index: &SymbolIndex, id: GlobalSymbolId) -> Option<String> {
    fs::read_to_string(source_path(index, id)?).ok()
}

fn symbol_line(index: &SymbolIndex, id: GlobalSymbolId) -> Option<usize> {
    let symbol = index.symbol(id)?;
    let source = source_text(index, id)?;
    Some(line_col(&source, symbol.selection_span.start).0)
}

fn symbol_snippet(index: &SymbolIndex, id: GlobalSymbolId) -> Option<String> {
    let symbol = index.symbol(id)?;
    let source = source_text(index, id)?;
    let (line, _) = line_col(&source, symbol.selection_span.start);
    Some(snippet_around_line(&source, line, SNIPPET_CONTEXT_LINES))
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for (index, ch) in source.char_indices() {
        if index >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn snippet_around_line(source: &str, line: usize, context: usize) -> String {
    let lines = source.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return String::new();
    }

    let start = line.saturating_sub(context + 1);
    let end = (line + context).min(lines.len());
    let mut snippet = String::new();
    for number in start..end {
        let marker = if number + 1 == line { ">" } else { " " };
        snippet.push_str(&format!(
            "{marker} {:>5} | {}\n",
            number + 1,
            escape_snippet_line(lines[number])
        ));
    }
    snippet
}

fn escape_snippet_line(value: &str) -> String {
    value.replace('\t', "\\t")
}

fn method_signatures(index: &SymbolIndex, ids: &[GlobalSymbolId]) -> Vec<String> {
    let mut signatures = ids
        .iter()
        .take(MAX_SIGNATURES_PER_METHOD_GROUP)
        .filter_map(|id| index.callable_signature(*id))
        .collect::<Vec<_>>();
    if ids.len() > MAX_SIGNATURES_PER_METHOD_GROUP {
        signatures.push(format!(
            "... {} more",
            ids.len() - MAX_SIGNATURES_PER_METHOD_GROUP
        ));
    }
    signatures
}

fn detail_text(index: &SymbolIndex, id: GlobalSymbolId) -> String {
    SymbolDisplay::for_symbol(index, id)
        .and_then(|display| display.detail)
        .unwrap_or_default()
}

fn classify_top_level_provenance(
    index: &SymbolIndex,
    symbols: &[GlobalSymbolId],
) -> TopLevelProvenance {
    let categories = symbols
        .iter()
        .map(|id| source_category_for_symbol(index, *id))
        .collect::<BTreeSet<_>>();

    if categories.contains(&SourceCategory::DocsDoxygen)
        || symbols
            .iter()
            .any(|id| file_contains(index, *id, "#ifdef DOXYGEN"))
    {
        return TopLevelProvenance::DocsDoxygenExample;
    }

    if symbols.iter().any(|id| {
        index
            .file(id.file_id)
            .is_some_and(|file| is_game_generated(file))
    }) && symbols.iter().any(|id| {
        index
            .file(id.file_id)
            .is_some_and(|file| is_gamelib_generated(file))
    }) {
        return TopLevelProvenance::GameGameLibGeneratedSplit;
    }

    if categories.contains(&SourceCategory::Generated) {
        return TopLevelProvenance::GeneratedOverlap;
    }

    if categories.contains(&SourceCategory::TestAutotest) {
        return TopLevelProvenance::TestAutotestOverlap;
    }

    let kinds = duplicate_kind_counts(index, symbols);
    if kinds.len() > 1 {
        return TopLevelProvenance::MixedKindEngineWrapper;
    }

    TopLevelProvenance::Unknown
}

fn classify_same_owner_shadow_provenance(
    index: &SymbolIndex,
    sample: &MemberShadowSample,
) -> ShadowProvenance {
    let same_owner_hidden = same_owner_shadowed_ids(index, sample);
    let ids = std::iter::once(sample.kept)
        .chain(same_owner_hidden.iter().copied())
        .collect::<Vec<_>>();

    if ids
        .iter()
        .any(|id| source_category_for_symbol(index, *id) == SourceCategory::DocsDoxygen)
        || ids
            .iter()
            .any(|id| file_contains(index, *id, "#ifdef DOXYGEN"))
    {
        return ShadowProvenance::DocsDoxygenOnlySource;
    }

    let has_preprocessor = ids
        .iter()
        .any(|id| symbol_has_preprocessor_context(index, *id));
    let has_prototype = ids.iter().any(|id| symbol_looks_like_prototype(index, *id));
    if has_preprocessor && has_prototype {
        return ShadowProvenance::PreprocessorPrototypeDuplicate;
    }
    if has_preprocessor {
        return ShadowProvenance::PreprocessorBranchDuplicate;
    }
    if has_prototype {
        return ShadowProvenance::PrototypeDeclarationBlock;
    }

    let categories = ids
        .iter()
        .map(|id| source_category_for_symbol(index, *id))
        .collect::<BTreeSet<_>>();
    if categories.contains(&SourceCategory::Generated) && categories.len() > 1 {
        return ShadowProvenance::GeneratedSourceOverlap;
    }

    if same_owner_hidden
        .iter()
        .any(|id| owner_name(index, *id) != owner_name(index, sample.kept))
    {
        return ShadowProvenance::InheritedAggregateArtifact;
    }

    ShadowProvenance::Unknown
}

fn classify_duplicate(index: &SymbolIndex, symbols: &[GlobalSymbolId]) -> DuplicateClassification {
    if symbols.iter().any(|id| {
        index
            .file(id.file_id)
            .is_some_and(|file| file.metadata.kind == SourceKind::Workspace)
    }) {
        return DuplicateClassification::WorkspaceOverlay;
    }

    let kinds = duplicate_kind_counts(index, symbols);
    let has_typedef = kinds.contains_key(&SymbolKind::Typedef);
    let has_function = kinds.contains_key(&SymbolKind::Function);
    let has_class = kinds.contains_key(&SymbolKind::Class);

    if has_typedef
        && has_function
        && symbols.iter().all(|id| {
            index.symbol(*id).is_some_and(|symbol| {
                matches!(symbol.kind, SymbolKind::Typedef | SymbolKind::Function)
            })
        })
    {
        return DuplicateClassification::TypedefFunctionDelegatePair;
    }

    if has_typedef
        && has_class
        && symbols.iter().all(|id| {
            index.symbol(*id).is_some_and(|symbol| {
                matches!(symbol.kind, SymbolKind::Typedef | SymbolKind::Class)
            })
        })
    {
        return DuplicateClassification::TypedefClassWrapperPattern;
    }

    if has_generated_and_source_paths(index, symbols) {
        return DuplicateClassification::GeneratedVsSourceDuplicate;
    }

    if kinds.values().any(|count| *count > 1) {
        return DuplicateClassification::SuspiciousSameKindDuplicate;
    }

    DuplicateClassification::MixedKindDuplicate
}

fn file_contains(index: &SymbolIndex, id: GlobalSymbolId, needle: &str) -> bool {
    source_text(index, id).is_some_and(|source| source.contains(needle))
}

fn symbol_has_preprocessor_context(index: &SymbolIndex, id: GlobalSymbolId) -> bool {
    index
        .symbol(id)
        .is_some_and(|symbol| !symbol.conditional_context.is_empty())
}

fn symbol_looks_like_prototype(index: &SymbolIndex, id: GlobalSymbolId) -> bool {
    let Some(symbol) = index.symbol(id) else {
        return false;
    };
    symbol
        .callable_form
        .is_some_and(|form| form.as_str() != "implementation")
}

fn is_game_generated(file: &IndexedFile) -> bool {
    let path = normalized_file_path(file);
    path.starts_with("game/generated/")
}

fn is_gamelib_generated(file: &IndexedFile) -> bool {
    let path = normalized_file_path(file);
    path.starts_with("gamelib/generated/")
}

fn duplicate_kind_counts(
    index: &SymbolIndex,
    symbols: &[GlobalSymbolId],
) -> BTreeMap<SymbolKind, usize> {
    let mut counts = BTreeMap::new();
    for id in symbols {
        if let Some(symbol) = index.symbol(*id) {
            *counts.entry(symbol.kind).or_default() += 1;
        }
    }
    counts
}

fn has_generated_and_source_paths(index: &SymbolIndex, symbols: &[GlobalSymbolId]) -> bool {
    let mut has_generated = false;
    let mut has_non_generated = false;

    for id in symbols {
        let Some(file) = index.file(id.file_id) else {
            continue;
        };
        let path = file
            .metadata
            .relative_path
            .as_ref()
            .or(file.metadata.absolute_path.as_ref())
            .map(|path| path.to_string_lossy().replace('\\', "/").to_lowercase())
            .unwrap_or_default();
        if path.contains("/generated/") || path.starts_with("generated/") {
            has_generated = true;
        } else {
            has_non_generated = true;
        }
    }

    has_generated && has_non_generated
}

fn duplicate_classification_label(classification: DuplicateClassification) -> &'static str {
    match classification {
        DuplicateClassification::WorkspaceOverlay => "workspace overlay duplicate",
        DuplicateClassification::TypedefFunctionDelegatePair => "typedef + function delegate pair",
        DuplicateClassification::TypedefClassWrapperPattern => "typedef + class wrapper pattern",
        DuplicateClassification::GeneratedVsSourceDuplicate => {
            "generated vs non-generated duplicate"
        }
        DuplicateClassification::SuspiciousSameKindDuplicate => "suspicious same-kind duplicate",
        DuplicateClassification::MixedKindDuplicate => "mixed-kind duplicate",
    }
}

fn top_level_provenance_label(provenance: TopLevelProvenance) -> &'static str {
    match provenance {
        TopLevelProvenance::GeneratedOverlap => "generated overlap",
        TopLevelProvenance::DocsDoxygenExample => "docs/Doxygen example",
        TopLevelProvenance::TestAutotestOverlap => "test/autotest overlap",
        TopLevelProvenance::GameGameLibGeneratedSplit => "Game/GameLib generated split",
        TopLevelProvenance::MixedKindEngineWrapper => "mixed-kind engine wrapper",
        TopLevelProvenance::Unknown => "unknown",
    }
}

fn shadow_provenance_label(provenance: ShadowProvenance) -> &'static str {
    match provenance {
        ShadowProvenance::PreprocessorBranchDuplicate => "preprocessor branch duplicate",
        ShadowProvenance::PreprocessorPrototypeDuplicate => {
            "preprocessor branch/prototype duplicate"
        }
        ShadowProvenance::PrototypeDeclarationBlock => "prototype/declaration block",
        ShadowProvenance::DocsDoxygenOnlySource => "docs/Doxygen-only source",
        ShadowProvenance::GeneratedSourceOverlap => "generated/source overlap",
        ShadowProvenance::InheritedAggregateArtifact => "inherited aggregate artifact",
        ShadowProvenance::Unknown => "unknown",
    }
}

fn completion_filtering_decision(provenance: ShadowProvenance) -> &'static str {
    match provenance {
        ShadowProvenance::PreprocessorBranchDuplicate => {
            "`Collapse in completion; keep all symbols indexed`"
        }
        ShadowProvenance::PreprocessorPrototypeDuplicate => {
            "`Collapse in completion; prefer implementation-quality symbol when available later`"
        }
        ShadowProvenance::PrototypeDeclarationBlock => {
            "`Collapse in completion; keep prototype visible through debug/navigation`"
        }
        ShadowProvenance::DocsDoxygenOnlySource => {
            "`Do not use docs-only source for normal editor completion later`"
        }
        ShadowProvenance::GeneratedSourceOverlap => {
            "`Collapse only when source priority/provenance policy says they are equivalent`"
        }
        ShadowProvenance::InheritedAggregateArtifact => {
            "`Collapse matching inherited keys; keep raw inherited candidates for debug`"
        }
        ShadowProvenance::Unknown => "`Do not add stronger filtering; investigate first`",
    }
}

fn completion_filtering_rationale(provenance: ShadowProvenance) -> &'static str {
    match provenance {
        ShadowProvenance::PreprocessorBranchDuplicate => {
            "Both branches are preserved because the index does not evaluate preprocessor conditions; duplicate completion labels would be noise."
        }
        ShadowProvenance::PreprocessorPrototypeDuplicate => {
            "The same callable shape appears in active-looking code and preserved declaration/prototype branches; one completion item is enough, but source facts remain useful."
        }
        ShadowProvenance::PrototypeDeclarationBlock => {
            "Prototype-only duplicates are declaration facts, not separate callable choices for completion."
        }
        ShadowProvenance::DocsDoxygenOnlySource => {
            "Docs examples are useful for documentation review but should not compete with real game/workspace symbols in editor features."
        }
        ShadowProvenance::GeneratedSourceOverlap => {
            "Generated/source overlaps may be valid engine surfaces; filtering needs provenance priority rather than a blanket rule."
        }
        ShadowProvenance::InheritedAggregateArtifact => {
            "Base-chain candidates are expected to shadow when the derived class exposes the same completion key."
        }
        ShadowProvenance::Unknown => {
            "Unknown conflicts could indicate parser/model/index gaps, so hiding them would mask defects."
        }
    }
}

fn completion_filtering_future_requirement(provenance: ShadowProvenance) -> &'static str {
    match provenance {
        ShadowProvenance::PreprocessorBranchDuplicate
        | ShadowProvenance::PreprocessorPrototypeDuplicate => {
            "Workbench-backed active-branch validation before claiming compiler-active truth."
        }
        ShadowProvenance::PrototypeDeclarationBlock => {
            "Editor-facing policy validation for when implementations should hide declarations/prototypes."
        }
        ShadowProvenance::DocsDoxygenOnlySource => {
            "Keep source-category exclusion wired into future runtime/LSP query paths."
        }
        ShadowProvenance::GeneratedSourceOverlap => {
            "Source provenance priority for generated, Game, GameLib, and workspace roots."
        }
        ShadowProvenance::InheritedAggregateArtifact => {
            "Semantic inheritance/override validation before claiming compiler-accurate override behavior."
        }
        ShadowProvenance::Unknown => "Bounded source review and a new classifier or bug fix.",
    }
}

fn display_relative_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string()
}

fn append_counts(
    report: &mut String,
    heading: &str,
    counts: &BTreeMap<String, usize>,
    limit: usize,
) {
    append_counts_with_level(report, heading, counts, limit, 2);
}

fn append_counts_with_level(
    report: &mut String,
    heading: &str,
    counts: &BTreeMap<String, usize>,
    limit: usize,
    level: usize,
) {
    let level = level.max(1);
    report.push_str(&format!("{} {heading}\n\n", "#".repeat(level)));

    if counts.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Item | Count |\n");
    report.push_str("| --- | ---: |\n");
    for (item, count) in sorted_counts(counts).into_iter().take(limit) {
        report.push_str(&format!("| `{}` | {} |\n", escape_table(&item), count));
    }
    report.push('\n');
}

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut values = counts
        .iter()
        .map(|(item, count)| (item.clone(), *count))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values
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

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn percent(part: usize, total: usize) -> String {
    if total == 0 {
        return "0.0%".to_string();
    }
    format!("{:.1}%", (part as f64 / total as f64) * 100.0)
}

fn duration_millis(duration: Duration) -> u128 {
    duration.as_millis()
}

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|")
}

fn escape_inline(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}
