use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::lexer::{lex, TextSpan, TokenKind};
use reforger_language_server::lsp::{
    hover_reports_for_source_positions_with_external, position_for_offset, symbol_kind_label,
    LspHoverReport,
};
use reforger_language_server::model::{SourceKind, SOURCE_PRIORITY_GAME_DATA};
use reforger_language_server::resolver::ResolutionReason;
use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const DEFAULT_OUTPUT: &str = "tools/reports/lsp-hover-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const DEFAULT_SAMPLES_PER_FILE: usize = 12;
const MAX_ROWS: usize = 100;
const MAX_MISS_SAMPLES: usize = 75;
const MAX_HIT_SAMPLES: usize = 75;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    if !args.scripts_path.is_dir() {
        return Err(format!(
            "Scripts folder does not exist: {}",
            args.scripts_path.display()
        ));
    }

    let discovery_start = Instant::now();
    let mut files = Vec::new();
    collect_c_files(&args.scripts_path, &mut files)?;
    files.sort();
    let discovery_elapsed = discovery_start.elapsed().as_millis();
    let external_start = Instant::now();
    let external_index = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &args.scripts_path,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })?
    .index;
    let external_elapsed = external_start.elapsed().as_millis();

    let scan_start = Instant::now();
    let mut totals = Totals::default();
    let mut file_rows = Vec::new();
    let mut reason_counts = BTreeMap::<String, usize>::new();
    let mut context_counts = BTreeMap::<String, usize>::new();
    let mut selected_source_counts = BTreeMap::<String, usize>::new();
    let mut kind_counts = BTreeMap::<String, usize>::new();
    let mut miss_samples = Vec::new();
    let mut hit_samples = Vec::new();

    for file in &files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        let source = String::from_utf8_lossy(&bytes);
        let relative_path = relative_display(file, &args.scripts_path);
        let identifiers = identifier_samples(&source, args.samples_per_file);
        let positions = identifiers
            .iter()
            .map(|sample| position_for_offset(&source, sample.span.start))
            .collect::<Vec<_>>();

        let hover_start = Instant::now();
        let reports = hover_reports_for_source_positions_with_external(
            &source,
            &positions,
            Some(&external_index),
        );
        let hover_elapsed = hover_start.elapsed().as_millis();

        let mut row = FileRow {
            path: relative_path.clone(),
            bytes: bytes.len(),
            identifier_samples: identifiers.len(),
            hits: 0,
            misses: 0,
            unresolved_misses: 0,
            parse_diagnostics: reports
                .first()
                .map(|report| report.parse_diagnostics)
                .unwrap_or(0),
            elapsed_ms: hover_elapsed,
        };

        totals.files += 1;
        totals.bytes += bytes.len();
        totals.identifier_samples += identifiers.len();
        totals.parse_diagnostics += row.parse_diagnostics;

        for (sample, report) in identifiers.iter().zip(reports.iter()) {
            let reason = report
                .resolver_reason
                .map(|reason| reason.as_str().to_string())
                .unwrap_or_else(|| "<none>".to_string());
            *reason_counts.entry(reason.clone()).or_default() += 1;
            let context = report
                .identifier_context
                .map(|context| context.as_str().to_string())
                .unwrap_or_else(|| "<none>".to_string());
            *context_counts.entry(context.clone()).or_default() += 1;
            let selected_source = report
                .selected_source
                .map(|source| source.as_str().to_string())
                .unwrap_or_else(|| "<none>".to_string());
            *selected_source_counts
                .entry(selected_source.clone())
                .or_default() += 1;

            if report.is_hit() {
                row.hits += 1;
                totals.hits += 1;
                if selected_source == "file-local" {
                    totals.file_local_hits += 1;
                } else if selected_source == "external" {
                    totals.external_hits += 1;
                }
                if let Some(kind) = report.selected_kind {
                    *kind_counts
                        .entry(symbol_kind_label(kind).to_string())
                        .or_default() += 1;
                }
                hit_samples.push(SampleRow::new(
                    &relative_path,
                    &source,
                    sample,
                    report,
                    reason,
                    context,
                    selected_source,
                ));
            } else {
                row.misses += 1;
                totals.misses += 1;
                if report.resolver_reason == Some(ResolutionReason::Unresolved) {
                    row.unresolved_misses += 1;
                    totals.unresolved_misses += 1;
                }
                miss_samples.push(SampleRow::new(
                    &relative_path,
                    &source,
                    sample,
                    report,
                    reason,
                    context,
                    selected_source,
                ));
            }
        }
        file_rows.push(row);
    }

    let scan_elapsed = scan_start.elapsed().as_millis();
    let render_start = Instant::now();
    let report = render_report(
        &args,
        &totals,
        &file_rows,
        &reason_counts,
        &context_counts,
        &selected_source_counts,
        &kind_counts,
        &sample_evenly_rows(&miss_samples, MAX_MISS_SAMPLES),
        &sample_evenly_rows(&hit_samples, MAX_HIT_SAMPLES),
        discovery_elapsed,
        external_elapsed,
        scan_elapsed,
        render_start,
    );

    if let Some(parent) = args.out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::write(&args.out_path, report)
        .map_err(|error| format!("Failed to write {}: {error}", args.out_path.display()))?;

    println!("Wrote {}", args.out_path.display());
    Ok(())
}

struct Args {
    scripts_path: PathBuf,
    out_path: PathBuf,
    profile_label: String,
    samples_per_file: usize,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut scripts_path = None;
        let mut out_path = None;
        let mut profile_label = "debug".to_string();
        let mut samples_per_file = DEFAULT_SAMPLES_PER_FILE;
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scripts" => {
                    scripts_path = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--scripts requires a path".to_string())?,
                    ));
                }
                "--out" => {
                    out_path = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--out requires a path".to_string())?,
                    ));
                }
                "--profile-label" => {
                    profile_label = args
                        .next()
                        .ok_or_else(|| "--profile-label requires a value".to_string())?;
                }
                "--samples-per-file" => {
                    samples_per_file = args
                        .next()
                        .ok_or_else(|| "--samples-per-file requires a value".to_string())?
                        .parse::<usize>()
                        .map_err(|error| format!("Invalid --samples-per-file: {error}"))?
                        .max(1);
                }
                "--help" | "-h" => {
                    println!("Usage: cargo run --example lsp_hover_corpus_report -- [--scripts <path>] [--out <path>] [--profile-label <label>] [--samples-per-file <count>]");
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }

        Ok(Self {
            scripts_path: scripts_path.unwrap_or_else(default_scripts_path),
            out_path: resolve_repo_path(out_path, DEFAULT_OUTPUT),
            profile_label,
            samples_per_file,
        })
    }
}

#[derive(Default)]
struct Totals {
    files: usize,
    bytes: usize,
    identifier_samples: usize,
    hits: usize,
    file_local_hits: usize,
    external_hits: usize,
    misses: usize,
    unresolved_misses: usize,
    parse_diagnostics: usize,
}

#[derive(Clone)]
struct FileRow {
    path: String,
    bytes: usize,
    identifier_samples: usize,
    hits: usize,
    misses: usize,
    unresolved_misses: usize,
    parse_diagnostics: usize,
    elapsed_ms: u128,
}

#[derive(Clone)]
struct IdentifierSample {
    token: String,
    span: TextSpan,
}

#[derive(Clone)]
struct SampleRow {
    path: String,
    line: u32,
    column: u32,
    token: String,
    hit: bool,
    reason: String,
    context: String,
    selected_source: String,
    selected: String,
    snippet: String,
}

impl SampleRow {
    fn new(
        path: &str,
        source: &str,
        sample: &IdentifierSample,
        report: &LspHoverReport,
        reason: String,
        context: String,
        selected_source: String,
    ) -> Self {
        let position = position_for_offset(source, sample.span.start);
        let selected = match (&report.selected_kind, &report.selected_label) {
            (Some(kind), Some(label)) => format!("{} `{}`", symbol_kind_label(*kind), label),
            _ => "<none>".to_string(),
        };
        Self {
            path: path.to_string(),
            line: position.line + 1,
            column: position.character + 1,
            token: sample.token.clone(),
            hit: report.is_hit(),
            reason,
            context,
            selected_source,
            selected,
            snippet: source_line(source, position.line),
        }
    }
}

fn render_report(
    args: &Args,
    totals: &Totals,
    file_rows: &[FileRow],
    reason_counts: &BTreeMap<String, usize>,
    context_counts: &BTreeMap<String, usize>,
    selected_source_counts: &BTreeMap<String, usize>,
    kind_counts: &BTreeMap<String, usize>,
    miss_samples: &[SampleRow],
    hit_samples: &[SampleRow],
    discovery_elapsed: u128,
    external_elapsed: u128,
    scan_elapsed: u128,
    render_start: Instant,
) -> String {
    let mut report = String::new();
    writeln!(report, "# LSP Hover Corpus Report").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "- Source path: `{}`", args.scripts_path.display()).unwrap();
    writeln!(report, "- Profile: `{}`", args.profile_label).unwrap();
    writeln!(report, "- Samples per file: {}", args.samples_per_file).unwrap();
    writeln!(report, "- `.c` files: {}", totals.files).unwrap();
    writeln!(
        report,
        "- Identifier samples: {}",
        totals.identifier_samples
    )
    .unwrap();
    writeln!(report, "- Hits: {}", totals.hits).unwrap();
    writeln!(report, "- Misses: {}", totals.misses).unwrap();
    writeln!(report).unwrap();
    writeln!(report, "This report samples identifier-token positions across the script corpus and runs the same resolver-first hover projection used by `textDocument/hover`, with the game-data index supplied as external lookup context. It is review tooling only; Workbench remains compiler truth.").unwrap();
    writeln!(report).unwrap();

    append_summary(&mut report, totals);
    append_counts(&mut report, "Resolver Reason Frequency", reason_counts);
    append_counts(&mut report, "Identifier Context Frequency", context_counts);
    append_counts(
        &mut report,
        "Selected Source Frequency",
        selected_source_counts,
    );
    append_counts(&mut report, "Selected Kind Frequency", kind_counts);
    append_top_files(&mut report, file_rows);
    append_samples(&mut report, "Miss Samples", miss_samples);
    append_samples(&mut report, "Hit Samples", hit_samples);
    append_notes(&mut report);
    append_timing(
        &mut report,
        discovery_elapsed,
        external_elapsed,
        scan_elapsed,
        render_start,
        &args.profile_label,
    );
    report
}

fn append_summary(report: &mut String, totals: &Totals) {
    let hit_rate = percentage(totals.hits, totals.identifier_samples);
    writeln!(report, "## Summary").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "| Metric | Value |").unwrap();
    writeln!(report, "| --- | ---: |").unwrap();
    writeln!(report, "| Files | {} |", totals.files).unwrap();
    writeln!(report, "| Bytes | {} |", totals.bytes).unwrap();
    writeln!(
        report,
        "| Identifier samples | {} |",
        totals.identifier_samples
    )
    .unwrap();
    writeln!(report, "| Hover hits | {} |", totals.hits).unwrap();
    writeln!(
        report,
        "| File-local hover hits | {} |",
        totals.file_local_hits
    )
    .unwrap();
    writeln!(report, "| External hover hits | {} |", totals.external_hits).unwrap();
    writeln!(report, "| Hover misses | {} |", totals.misses).unwrap();
    writeln!(report, "| Hit rate | {:.2}% |", hit_rate).unwrap();
    writeln!(
        report,
        "| Unresolved misses | {} |",
        totals.unresolved_misses
    )
    .unwrap();
    writeln!(
        report,
        "| Parse diagnostics | {} |",
        totals.parse_diagnostics
    )
    .unwrap();
    writeln!(report).unwrap();
}

fn append_counts(report: &mut String, heading: &str, counts: &BTreeMap<String, usize>) {
    writeln!(report, "## {heading}").unwrap();
    writeln!(report).unwrap();
    if counts.is_empty() {
        writeln!(report, "None.").unwrap();
        writeln!(report).unwrap();
        return;
    }
    writeln!(report, "| Value | Count |").unwrap();
    writeln!(report, "| --- | ---: |").unwrap();
    for (value, count) in sorted_counts(counts) {
        writeln!(report, "| `{}` | {} |", escape_table(&value), count).unwrap();
    }
    writeln!(report).unwrap();
}

fn append_top_files(report: &mut String, rows: &[FileRow]) {
    let mut by_misses = rows.to_vec();
    by_misses.sort_by(|left, right| {
        right
            .misses
            .cmp(&left.misses)
            .then_with(|| right.unresolved_misses.cmp(&left.unresolved_misses))
            .then_with(|| left.path.cmp(&right.path))
    });
    writeln!(report, "## Top Files By Hover Misses").unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "| File | Bytes | Samples | Hits | Misses | Unresolved | Parse diagnostics | Elapsed ms |"
    )
    .unwrap();
    writeln!(
        report,
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    )
    .unwrap();
    for row in by_misses.into_iter().take(MAX_ROWS) {
        if row.misses == 0 {
            continue;
        }
        writeln!(
            report,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} |",
            escape_table(&row.path),
            row.bytes,
            row.identifier_samples,
            row.hits,
            row.misses,
            row.unresolved_misses,
            row.parse_diagnostics,
            row.elapsed_ms
        )
        .unwrap();
    }
    writeln!(report).unwrap();
}

fn append_samples(report: &mut String, heading: &str, samples: &[SampleRow]) {
    writeln!(report, "## {heading}").unwrap();
    writeln!(report).unwrap();
    if samples.is_empty() {
        writeln!(report, "None.").unwrap();
        writeln!(report).unwrap();
        return;
    }
    writeln!(
        report,
        "| File | Position | Token | Hit | Reason | Context | Selected source | Selected | Source line |"
    )
    .unwrap();
    writeln!(
        report,
        "| --- | ---: | --- | --- | --- | --- | --- | --- | --- |"
    )
    .unwrap();
    for sample in samples {
        writeln!(
            report,
            "| `{}` | {}:{} | `{}` | {} | `{}` | `{}` | `{}` | {} | `{}` |",
            escape_table(&sample.path),
            sample.line,
            sample.column,
            escape_table(&sample.token),
            if sample.hit { "yes" } else { "no" },
            escape_table(&sample.reason),
            escape_table(&sample.context),
            escape_table(&sample.selected_source),
            escape_table(&sample.selected),
            escape_table(&sample.snippet)
        )
        .unwrap();
    }
    writeln!(report).unwrap();
}

fn append_notes(report: &mut String) {
    writeln!(report, "## Review Notes").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "- Samples are deterministic, bounded identifier-token positions per file; hit and miss sample tables are evenly selected from the full corpus result set.").unwrap();
    writeln!(report, "- The report supplies the full game-data index as external hover context. Remaining misses are unresolved after file-local and external top-level lookup.").unwrap();
    writeln!(report, "- Miss samples are planning evidence for resolver/index work, not Workbench compiler truth.").unwrap();
    writeln!(report).unwrap();
}

fn append_timing(
    report: &mut String,
    discovery_elapsed: u128,
    external_elapsed: u128,
    scan_elapsed: u128,
    render_start: Instant,
    profile_label: &str,
) {
    let render_elapsed = render_start.elapsed().as_millis();
    writeln!(report, "## Timing").unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "Wall-clock timings are for review only. Current profile: `{profile_label}`."
    )
    .unwrap();
    writeln!(report).unwrap();
    writeln!(report, "| Phase | Milliseconds |").unwrap();
    writeln!(report, "| --- | ---: |").unwrap();
    writeln!(report, "| File discovery | {discovery_elapsed} |").unwrap();
    writeln!(report, "| External index build | {external_elapsed} |").unwrap();
    writeln!(report, "| Read/decode/sample/hover | {scan_elapsed} |").unwrap();
    writeln!(report, "| Report rendering | {render_elapsed} |").unwrap();
    writeln!(
        report,
        "| Total report run | {} |",
        discovery_elapsed + external_elapsed + scan_elapsed + render_elapsed
    )
    .unwrap();
    writeln!(report).unwrap();
}

fn identifier_samples(source: &str, limit: usize) -> Vec<IdentifierSample> {
    let identifiers = lex(source)
        .into_iter()
        .filter(|token| token.kind == TokenKind::Identifier)
        .map(|token| IdentifierSample {
            token: source[token.span.start..token.span.end].to_string(),
            span: token.span,
        })
        .collect::<Vec<_>>();
    sample_evenly(&identifiers, limit)
}

fn sample_evenly(values: &[IdentifierSample], limit: usize) -> Vec<IdentifierSample> {
    if values.len() <= limit {
        return values.to_vec();
    }
    if limit <= 1 {
        return values.first().cloned().into_iter().collect();
    }

    let mut sampled = Vec::new();
    let mut last_index = None;
    for index in 0..limit {
        let value_index = index * (values.len() - 1) / (limit - 1);
        if last_index == Some(value_index) {
            continue;
        }
        sampled.push(values[value_index].clone());
        last_index = Some(value_index);
    }
    sampled
}

fn sample_evenly_rows(values: &[SampleRow], limit: usize) -> Vec<SampleRow> {
    if values.len() <= limit {
        return values.to_vec();
    }
    if limit <= 1 {
        return values.first().cloned().into_iter().collect();
    }

    let mut sampled = Vec::new();
    let mut last_index = None;
    for index in 0..limit {
        let value_index = index * (values.len() - 1) / (limit - 1);
        if last_index == Some(value_index) {
            continue;
        }
        sampled.push(values[value_index].clone());
        last_index = Some(value_index);
    }
    sampled
}

fn collect_c_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(path).map_err(|error| format!("Failed to read {}: {error}", path.display()))?
    {
        let entry = entry.map_err(|error| format!("Failed to read directory entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_c_files(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("c") {
            files.push(path);
        }
    }
    Ok(())
}

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut values = counts
        .iter()
        .map(|(value, count)| (value.clone(), *count))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values
}

fn percentage(value: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 * 100.0 / total as f64
    }
}

fn source_line(source: &str, line: u32) -> String {
    source
        .lines()
        .nth(line as usize)
        .unwrap_or_default()
        .trim()
        .chars()
        .take(180)
        .collect()
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

fn relative_display(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn escape_table(value: &str) -> String {
    value
        .replace('|', "\\|")
        .replace('\r', "")
        .replace('\n', "\\n")
}
