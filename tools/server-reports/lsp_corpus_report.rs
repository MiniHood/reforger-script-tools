use reforger_language_server::lexer::{lex, TokenKind};
use reforger_language_server::lsp::{
    document_symbol_report_for_source, LspDocumentSymbol, LspPosition, LspRange,
};
use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DEFAULT_OUTPUT: &str = "tools/reports/lsp-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_ROWS: usize = 100;
const MAX_FAILURE_ROWS: usize = 50;
const MAX_UNKNOWN_ZERO_SNIPPETS: usize = 25;
const SNIPPET_CONTEXT_LINES: usize = 2;
const EXPECTED_MAX_DEPTH: usize = 2;

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

    let scan_start = Instant::now();
    let mut rows = Vec::new();
    let mut kind_counts = BTreeMap::<String, usize>::new();
    let mut totals = Totals::default();

    for file in &files {
        let read_start = Instant::now();
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        let source = String::from_utf8_lossy(&bytes);
        let decoded_lossily = matches!(source, std::borrow::Cow::Owned(_));
        let read_decode_elapsed = read_start.elapsed();

        let projection_start = Instant::now();
        let report = document_symbol_report_for_source(&source);
        let projection_elapsed = projection_start.elapsed();

        let top_level_symbols = report.symbols.len();
        let total_symbols = report.total_symbol_count();
        let nested_symbols = total_symbols.saturating_sub(top_level_symbols);
        let max_depth = max_symbol_depth(&report.symbols);
        let unknown_labels = count_unknown_labels(&report.symbols);
        let range_failures = count_range_failures(&report.symbols);
        let relative_path = relative_display(file, &args.scripts_path);
        let zero_symbol_classification = if total_symbols == 0 {
            Some(classify_zero_symbol_file(&relative_path, &source))
        } else {
            None
        };

        totals.files += 1;
        totals.bytes += bytes.len();
        totals.lossy_files += usize::from(decoded_lossily);
        totals.parse_diagnostics += report.parse_diagnostics;
        totals.top_level_symbols += top_level_symbols;
        totals.nested_symbols += nested_symbols;
        totals.total_symbols += total_symbols;
        totals.unknown_labels += unknown_labels;
        totals.range_failures += range_failures;

        record_kind_counts(&report.symbols, &mut kind_counts);

        rows.push(FileRow {
            path: relative_path,
            bytes: bytes.len(),
            decoded_lossily,
            parse_diagnostics: report.parse_diagnostics,
            top_level_symbols,
            nested_symbols,
            total_symbols,
            max_depth,
            unknown_labels,
            range_failures,
            read_decode: read_decode_elapsed,
            projection: projection_elapsed,
            zero_symbol_classification,
            unknown_zero_symbol_snippet: if zero_symbol_classification
                == Some(ZeroSymbolKind::Unknown)
            {
                Some(snippet_around_line(
                    &source,
                    first_non_blank_line(&source),
                    SNIPPET_CONTEXT_LINES,
                ))
            } else {
                None
            },
        });
    }

    let scan_elapsed = scan_start.elapsed().as_millis();
    let render_start = Instant::now();
    let report = render_report(
        &args.scripts_path,
        &rows,
        &totals,
        &kind_counts,
        discovery_elapsed,
        scan_elapsed,
        render_start,
        &args.profile_label,
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
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut scripts_path = None;
        let mut out_path = None;
        let mut profile_label = "debug".to_string();
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
                "--help" | "-h" => {
                    println!(
                        "Usage: cargo run --example lsp_corpus_report -- [--scripts <path>] [--out <path>] [--profile-label <label>]"
                    );
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }

        Ok(Self {
            scripts_path: scripts_path.unwrap_or_else(default_scripts_path),
            out_path: resolve_repo_path(out_path, DEFAULT_OUTPUT),
            profile_label,
        })
    }
}

#[derive(Default)]
struct Totals {
    files: usize,
    bytes: usize,
    lossy_files: usize,
    parse_diagnostics: usize,
    top_level_symbols: usize,
    nested_symbols: usize,
    total_symbols: usize,
    unknown_labels: usize,
    range_failures: usize,
}

#[derive(Clone)]
struct FileRow {
    path: String,
    bytes: usize,
    decoded_lossily: bool,
    parse_diagnostics: usize,
    top_level_symbols: usize,
    nested_symbols: usize,
    total_symbols: usize,
    max_depth: usize,
    unknown_labels: usize,
    range_failures: usize,
    read_decode: Duration,
    projection: Duration,
    zero_symbol_classification: Option<ZeroSymbolKind>,
    unknown_zero_symbol_snippet: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ZeroSymbolKind {
    Empty,
    DocsOnly,
    CommentOnly,
    Unknown,
}

fn render_report(
    scripts_path: &Path,
    rows: &[FileRow],
    totals: &Totals,
    kind_counts: &BTreeMap<String, usize>,
    discovery_elapsed: u128,
    scan_elapsed: u128,
    render_start: Instant,
    profile_label: &str,
) -> String {
    let mut report = String::new();
    writeln!(report, "# LSP Corpus Report").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "- Source path: `{}`", scripts_path.display()).unwrap();
    writeln!(report, "- Profile: `{profile_label}`").unwrap();
    writeln!(report, "- `.c` files: {}", totals.files).unwrap();
    writeln!(report, "- Bytes: {}", totals.bytes).unwrap();
    writeln!(report, "- Total document symbols: {}", totals.total_symbols).unwrap();
    writeln!(report, "- Parse diagnostics: {}", totals.parse_diagnostics).unwrap();
    writeln!(report, "- Unknown labels: {}", totals.unknown_labels).unwrap();
    writeln!(report, "- Range sanity failures: {}", totals.range_failures).unwrap();
    writeln!(report).unwrap();
    writeln!(report, "This report runs the same document-symbol projection used by `textDocument/documentSymbol` across the script corpus. It validates LSP-facing labels, kinds, details, ranges, and tree shape; Workbench remains compiler truth.").unwrap();
    writeln!(report).unwrap();

    append_summary(&mut report, totals);
    append_kind_counts(&mut report, kind_counts);
    append_lsp_kind_mapping_notes(&mut report);
    append_lossy_files(&mut report, rows);
    append_zero_symbol_classification(&mut report, rows);
    append_zero_symbol_files(&mut report, rows);
    append_failure_sections(&mut report, rows);
    append_top_files(&mut report, rows);
    append_tree_depth_summary(&mut report, rows);
    append_projection_timing_stats(&mut report, rows, profile_label);
    append_timing(
        &mut report,
        discovery_elapsed,
        scan_elapsed,
        render_start,
        profile_label,
    );
    report
}

fn append_summary(report: &mut String, totals: &Totals) {
    writeln!(report, "## Summary").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "| Metric | Value |").unwrap();
    writeln!(report, "| --- | ---: |").unwrap();
    writeln!(report, "| `.c` files | {} |", totals.files).unwrap();
    writeln!(report, "| Bytes | {} |", totals.bytes).unwrap();
    writeln!(report, "| Files decoded lossily | {} |", totals.lossy_files).unwrap();
    writeln!(
        report,
        "| Top-level document symbols | {} |",
        totals.top_level_symbols
    )
    .unwrap();
    writeln!(
        report,
        "| Nested document symbols | {} |",
        totals.nested_symbols
    )
    .unwrap();
    writeln!(
        report,
        "| Total document symbols | {} |",
        totals.total_symbols
    )
    .unwrap();
    writeln!(
        report,
        "| Parse diagnostics | {} |",
        totals.parse_diagnostics
    )
    .unwrap();
    writeln!(report, "| Unknown labels | {} |", totals.unknown_labels).unwrap();
    writeln!(
        report,
        "| Range sanity failures | {} |",
        totals.range_failures
    )
    .unwrap();
    writeln!(report).unwrap();
}

fn append_kind_counts(report: &mut String, kind_counts: &BTreeMap<String, usize>) {
    writeln!(report, "## Document Symbol Kind Frequency").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "| Kind | Count |").unwrap();
    writeln!(report, "| --- | ---: |").unwrap();
    for (kind, count) in sorted_counts(kind_counts) {
        writeln!(report, "| `{}` | {} |", escape_table(&kind), count).unwrap();
    }
    writeln!(report).unwrap();
}

fn append_lsp_kind_mapping_notes(report: &mut String) {
    writeln!(report, "## LSP Kind Mapping Notes").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "These are current document-symbol display choices, not compiler or semantic claims. VS Code/LSP does not provide exact Enfusion-specific kinds for every declaration shape.").unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "| Report label | LSP kind number | Protocol-facing meaning | Note |"
    )
    .unwrap();
    writeln!(report, "| --- | ---: | --- | --- |").unwrap();
    writeln!(report, "| `Typedef` | 26 | TypeParameter | LSP has no typedef kind; report label keeps the Enfusion meaning readable. |").unwrap();
    writeln!(report, "| `Destructor` | 6 | Method | LSP has no destructor kind; destructor labels remain source-backed through symbol display. |").unwrap();
    writeln!(
        report,
        "| `Variable` | 13 | Variable | Used for top-level global fields. |"
    )
    .unwrap();
    writeln!(report).unwrap();
}

fn append_lossy_files(report: &mut String, rows: &[FileRow]) {
    let lossy_rows = rows
        .iter()
        .filter(|row| row.decoded_lossily)
        .take(MAX_FAILURE_ROWS)
        .collect::<Vec<_>>();

    writeln!(report, "## Lossy Decoded Files").unwrap();
    writeln!(report).unwrap();
    if lossy_rows.is_empty() {
        writeln!(report, "None.").unwrap();
        writeln!(report).unwrap();
        return;
    }

    writeln!(report, "| File | Bytes | Symbols |").unwrap();
    writeln!(report, "| --- | ---: | ---: |").unwrap();
    for row in lossy_rows {
        writeln!(
            report,
            "| `{}` | {} | {} |",
            escape_table(&row.path),
            row.bytes,
            row.total_symbols
        )
        .unwrap();
    }
    writeln!(report).unwrap();
}

fn append_zero_symbol_files(report: &mut String, rows: &[FileRow]) {
    let zero_symbol_rows = rows
        .iter()
        .filter(|row| row.total_symbols == 0)
        .take(MAX_FAILURE_ROWS)
        .collect::<Vec<_>>();

    writeln!(report, "## Files With Zero Document Symbols").unwrap();
    writeln!(report).unwrap();
    if zero_symbol_rows.is_empty() {
        writeln!(report, "None.").unwrap();
        writeln!(report).unwrap();
        return;
    }
    writeln!(report, "| File | Bytes | Parse diagnostics |").unwrap();
    writeln!(report, "| --- | ---: | ---: |").unwrap();
    for row in zero_symbol_rows {
        writeln!(
            report,
            "| `{}` | {} | {} |",
            escape_table(&row.path),
            row.bytes,
            row.parse_diagnostics
        )
        .unwrap();
    }
    writeln!(report).unwrap();
}

fn append_zero_symbol_classification(report: &mut String, rows: &[FileRow]) {
    let zero_rows = rows
        .iter()
        .filter(|row| row.total_symbols == 0)
        .collect::<Vec<_>>();
    let mut counts = BTreeMap::<ZeroSymbolKind, usize>::new();
    for row in &zero_rows {
        if let Some(classification) = row.zero_symbol_classification {
            *counts.entry(classification).or_default() += 1;
        }
    }

    writeln!(report, "## Zero Symbol File Classification").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "| Classification | Files |").unwrap();
    writeln!(report, "| --- | ---: |").unwrap();
    writeln!(report, "| `total` | {} |", zero_rows.len()).unwrap();
    for classification in [
        ZeroSymbolKind::Empty,
        ZeroSymbolKind::DocsOnly,
        ZeroSymbolKind::CommentOnly,
        ZeroSymbolKind::Unknown,
    ] {
        writeln!(
            report,
            "| `{}` | {} |",
            zero_symbol_kind_label(classification),
            counts.get(&classification).copied().unwrap_or(0)
        )
        .unwrap();
    }
    writeln!(report).unwrap();

    let unknown_rows = zero_rows
        .iter()
        .filter(|row| row.zero_symbol_classification == Some(ZeroSymbolKind::Unknown))
        .take(MAX_UNKNOWN_ZERO_SNIPPETS)
        .collect::<Vec<_>>();
    writeln!(report, "### Unknown Non-Empty Zero-Symbol Snippets").unwrap();
    writeln!(report).unwrap();
    if unknown_rows.is_empty() {
        writeln!(report, "None.").unwrap();
        writeln!(report).unwrap();
        return;
    }

    for row in unknown_rows {
        writeln!(report, "#### `{}`", escape_inline(&row.path)).unwrap();
        writeln!(report).unwrap();
        writeln!(report, "````enforce").unwrap();
        if let Some(snippet) = &row.unknown_zero_symbol_snippet {
            report.push_str(snippet);
        }
        writeln!(report, "````").unwrap();
        writeln!(report).unwrap();
    }
}

fn append_failure_sections(report: &mut String, rows: &[FileRow]) {
    append_filtered_rows(
        report,
        "Files With Parse Diagnostics",
        rows,
        |row| row.parse_diagnostics > 0,
        "Parse diagnostics",
        |row| row.parse_diagnostics,
    );
    append_filtered_rows(
        report,
        "Files With Unknown Labels",
        rows,
        |row| row.unknown_labels > 0,
        "Unknown labels",
        |row| row.unknown_labels,
    );
    append_filtered_rows(
        report,
        "Files With Range Sanity Failures",
        rows,
        |row| row.range_failures > 0,
        "Range failures",
        |row| row.range_failures,
    );
}

fn append_filtered_rows(
    report: &mut String,
    heading: &str,
    rows: &[FileRow],
    predicate: impl Fn(&FileRow) -> bool,
    count_label: &str,
    count: impl Fn(&FileRow) -> usize,
) {
    let mut matches = rows.iter().filter(|row| predicate(row)).collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        count(right)
            .cmp(&count(left))
            .then_with(|| left.path.cmp(&right.path))
    });

    writeln!(report, "## {heading}").unwrap();
    writeln!(report).unwrap();
    if matches.is_empty() {
        writeln!(report, "None.").unwrap();
        writeln!(report).unwrap();
        return;
    }
    writeln!(report, "| File | {count_label} | Symbols |").unwrap();
    writeln!(report, "| --- | ---: | ---: |").unwrap();
    for row in matches.into_iter().take(MAX_FAILURE_ROWS) {
        writeln!(
            report,
            "| `{}` | {} | {} |",
            escape_table(&row.path),
            count(row),
            row.total_symbols
        )
        .unwrap();
    }
    writeln!(report).unwrap();
}

fn append_top_files(report: &mut String, rows: &[FileRow]) {
    append_top_rows(report, "Top Files By Document Symbols", rows, |row| {
        row.total_symbols
    });
    append_top_rows(
        report,
        "Slowest Document-Symbol Projection Files",
        rows,
        |row| duration_micros(row.projection) as usize,
    );
    append_slowest_per_symbol(report, rows);
}

fn append_top_rows(
    report: &mut String,
    heading: &str,
    rows: &[FileRow],
    key: impl Fn(&FileRow) -> usize,
) {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|left, right| {
        key(right)
            .cmp(&key(left))
            .then_with(|| left.path.cmp(&right.path))
    });

    writeln!(report, "## {heading}").unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "| File | Symbols | Top-level | Nested | Max depth | Projection ms |"
    )
    .unwrap();
    writeln!(report, "| --- | ---: | ---: | ---: | ---: | ---: |").unwrap();
    for row in sorted.into_iter().take(MAX_ROWS) {
        writeln!(
            report,
            "| `{}` | {} | {} | {} | {} | {} |",
            escape_table(&row.path),
            row.total_symbols,
            row.top_level_symbols,
            row.nested_symbols,
            row.max_depth,
            duration_millis(row.projection)
        )
        .unwrap();
    }
    writeln!(report).unwrap();
}

fn append_slowest_per_symbol(report: &mut String, rows: &[FileRow]) {
    let mut sorted = rows
        .iter()
        .filter(|row| row.total_symbols > 0)
        .collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        per_symbol_micros(right)
            .cmp(&per_symbol_micros(left))
            .then_with(|| left.path.cmp(&right.path))
    });

    writeln!(report, "## Top Slow Files Per Symbol").unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "| File | Symbols | Projection ms | Microseconds per symbol |"
    )
    .unwrap();
    writeln!(report, "| --- | ---: | ---: | ---: |").unwrap();
    for row in sorted.into_iter().take(MAX_ROWS) {
        writeln!(
            report,
            "| `{}` | {} | {} | {} |",
            escape_table(&row.path),
            row.total_symbols,
            duration_millis(row.projection),
            per_symbol_micros(row)
        )
        .unwrap();
    }
    writeln!(report).unwrap();
}

fn append_tree_depth_summary(report: &mut String, rows: &[FileRow]) {
    let mut depth_counts = BTreeMap::<usize, usize>::new();
    for row in rows {
        *depth_counts.entry(row.max_depth).or_default() += 1;
    }

    writeln!(report, "## Tree Depth Summary").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "Current expected max depth is `{EXPECTED_MAX_DEPTH}` because the LSP path exposes declarations and members, not method locals or statements.").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "| Max depth | Files |").unwrap();
    writeln!(report, "| ---: | ---: |").unwrap();
    for (depth, count) in depth_counts {
        writeln!(report, "| {} | {} |", depth, count).unwrap();
    }
    writeln!(report).unwrap();

    let deeper = rows
        .iter()
        .filter(|row| row.max_depth > EXPECTED_MAX_DEPTH)
        .take(MAX_FAILURE_ROWS)
        .collect::<Vec<_>>();
    writeln!(report, "### Files Deeper Than Expected").unwrap();
    writeln!(report).unwrap();
    if deeper.is_empty() {
        writeln!(report, "None.").unwrap();
        writeln!(report).unwrap();
        return;
    }
    writeln!(report, "| File | Max depth | Symbols |").unwrap();
    writeln!(report, "| --- | ---: | ---: |").unwrap();
    for row in deeper {
        writeln!(
            report,
            "| `{}` | {} | {} |",
            escape_table(&row.path),
            row.max_depth,
            row.total_symbols
        )
        .unwrap();
    }
    writeln!(report).unwrap();
}

fn append_projection_timing_stats(report: &mut String, rows: &[FileRow], profile_label: &str) {
    let mut projection_micros = rows
        .iter()
        .map(|row| duration_micros(row.projection))
        .collect::<Vec<_>>();
    projection_micros.sort_unstable();
    let total_projection_micros = projection_micros.iter().sum::<u128>();
    let total_read_decode_micros = rows
        .iter()
        .map(|row| duration_micros(row.read_decode))
        .sum::<u128>();
    let max_projection_micros = projection_micros.last().copied().unwrap_or(0);
    let average_projection_micros = if rows.is_empty() {
        0
    } else {
        total_projection_micros / rows.len() as u128
    };
    let p95_projection_micros = percentile(&projection_micros, 95);
    let total_symbols = rows.iter().map(|row| row.total_symbols).sum::<usize>();
    let files_per_second = rate_per_second(rows.len(), total_projection_micros);
    let symbols_per_second = rate_per_second(total_symbols, total_projection_micros);

    writeln!(report, "## Projection Timing Stats").unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "Profile `{profile_label}` timing is local wall-clock report timing. Use release mode for closer runtime signal."
    )
    .unwrap();
    writeln!(report).unwrap();
    writeln!(report, "| Metric | Value |").unwrap();
    writeln!(report, "| --- | ---: |").unwrap();
    writeln!(
        report,
        "| Total read/decode milliseconds | {} |",
        total_read_decode_micros / 1000
    )
    .unwrap();
    writeln!(
        report,
        "| Total projection milliseconds | {} |",
        total_projection_micros / 1000
    )
    .unwrap();
    writeln!(
        report,
        "| Average projection milliseconds/file | {:.3} |",
        average_projection_micros as f64 / 1000.0
    )
    .unwrap();
    writeln!(
        report,
        "| P95 projection milliseconds/file | {:.3} |",
        p95_projection_micros as f64 / 1000.0
    )
    .unwrap();
    writeln!(
        report,
        "| Max projection milliseconds/file | {:.3} |",
        max_projection_micros as f64 / 1000.0
    )
    .unwrap();
    writeln!(report, "| Files per second | {:.1} |", files_per_second).unwrap();
    writeln!(
        report,
        "| Document symbols per second | {:.1} |",
        symbols_per_second
    )
    .unwrap();
    writeln!(report).unwrap();
}

fn append_timing(
    report: &mut String,
    discovery_elapsed: u128,
    scan_elapsed: u128,
    render_start: Instant,
    profile_label: &str,
) {
    let render_elapsed = render_start.elapsed().as_millis();
    writeln!(report, "## Timing").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "Wall-clock timings are for review and trend spotting only; they are not benchmark-grade measurements. Current profile: `{profile_label}`.").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "| Phase | Milliseconds |").unwrap();
    writeln!(report, "| --- | ---: |").unwrap();
    writeln!(report, "| File discovery | {discovery_elapsed} |").unwrap();
    writeln!(
        report,
        "| Read/decode/document-symbol projection | {scan_elapsed} |"
    )
    .unwrap();
    writeln!(report, "| Report rendering | {render_elapsed} |").unwrap();
    writeln!(
        report,
        "| Total report run | {} |",
        discovery_elapsed + scan_elapsed + render_elapsed
    )
    .unwrap();
    writeln!(report).unwrap();
}

fn classify_zero_symbol_file(relative_path: &str, source: &str) -> ZeroSymbolKind {
    if source.trim().is_empty() {
        return ZeroSymbolKind::Empty;
    }

    if path_has_docs_segment(relative_path) {
        return ZeroSymbolKind::DocsOnly;
    }

    let tokens = lex(source);
    let meaningful_tokens = tokens
        .iter()
        .filter(|token| token.kind != TokenKind::Eof)
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace))
        .collect::<Vec<_>>();
    if meaningful_tokens.is_empty() {
        return ZeroSymbolKind::Empty;
    }

    if meaningful_tokens.iter().all(|token| {
        matches!(
            token.kind,
            TokenKind::DocLineComment | TokenKind::DocBlockComment
        )
    }) {
        return ZeroSymbolKind::DocsOnly;
    }

    if meaningful_tokens.iter().all(|token| {
        matches!(
            token.kind,
            TokenKind::LineComment
                | TokenKind::BlockComment
                | TokenKind::DocLineComment
                | TokenKind::DocBlockComment
        )
    }) {
        return ZeroSymbolKind::CommentOnly;
    }

    ZeroSymbolKind::Unknown
}

fn path_has_docs_segment(relative_path: &str) -> bool {
    relative_path
        .replace('\\', "/")
        .split('/')
        .any(|segment| segment.eq_ignore_ascii_case("docs") || segment.eq_ignore_ascii_case("doc"))
}

fn zero_symbol_kind_label(kind: ZeroSymbolKind) -> &'static str {
    match kind {
        ZeroSymbolKind::Empty => "empty",
        ZeroSymbolKind::DocsOnly => "docs-only",
        ZeroSymbolKind::CommentOnly => "comment-only",
        ZeroSymbolKind::Unknown => "unknown",
    }
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

fn record_kind_counts(symbols: &[LspDocumentSymbol], counts: &mut BTreeMap<String, usize>) {
    for symbol in symbols {
        *counts
            .entry(kind_label(symbol.kind).to_string())
            .or_default() += 1;
        record_kind_counts(&symbol.children, counts);
    }
}

fn max_symbol_depth(symbols: &[LspDocumentSymbol]) -> usize {
    symbols
        .iter()
        .map(|symbol| 1 + max_symbol_depth(&symbol.children))
        .max()
        .unwrap_or(0)
}

fn count_unknown_labels(symbols: &[LspDocumentSymbol]) -> usize {
    symbols
        .iter()
        .map(|symbol| {
            usize::from(symbol.name.trim().is_empty() || symbol.name == "<unknown>")
                + count_unknown_labels(&symbol.children)
        })
        .sum()
}

fn count_range_failures(symbols: &[LspDocumentSymbol]) -> usize {
    symbols
        .iter()
        .map(|symbol| {
            usize::from(!range_contains(symbol.range, symbol.selection_range))
                + count_range_failures(&symbol.children)
        })
        .sum()
}

fn range_contains(outer: LspRange, inner: LspRange) -> bool {
    position_le(outer.start, inner.start) && position_le(inner.end, outer.end)
}

fn position_le(left: LspPosition, right: LspPosition) -> bool {
    (left.line, left.character) <= (right.line, right.character)
}

fn first_non_blank_line(source: &str) -> usize {
    source
        .lines()
        .position(|line| !line.trim().is_empty())
        .map(|index| index + 1)
        .unwrap_or(1)
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

fn kind_label(kind: u32) -> &'static str {
    match kind {
        5 => "Class",
        6 => "Method",
        8 => "Field",
        9 => "Constructor",
        10 => "Enum",
        12 => "Function",
        13 => "Variable",
        22 => "EnumMember",
        26 => "Typedef",
        _ => "Symbol",
    }
}

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut values = counts
        .iter()
        .map(|(kind, count)| (kind.clone(), *count))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values
}

fn percentile(sorted_values: &[u128], percentile: usize) -> u128 {
    if sorted_values.is_empty() {
        return 0;
    }
    let index = ((sorted_values.len() - 1) * percentile) / 100;
    sorted_values[index]
}

fn rate_per_second(count: usize, elapsed_micros: u128) -> f64 {
    if elapsed_micros == 0 {
        return 0.0;
    }
    count as f64 / (elapsed_micros as f64 / 1_000_000.0)
}

fn duration_millis(duration: Duration) -> u128 {
    duration.as_millis()
}

fn duration_micros(duration: Duration) -> u128 {
    duration.as_micros()
}

fn per_symbol_micros(row: &FileRow) -> u128 {
    if row.total_symbols == 0 {
        return 0;
    }
    duration_micros(row.projection) / row.total_symbols as u128
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

fn escape_snippet_line(value: &str) -> String {
    value.replace('\t', "\\t")
}
