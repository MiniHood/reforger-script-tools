use reforger_language_server::index::SymbolIndex;
use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::lexer::{lex, TokenKind};
use reforger_language_server::lsp::{
    completion_report_for_cached_analysis_with_external, file_index_for_source,
    position_for_offset, LspCompletionReport,
};
use reforger_language_server::model::{SourceKind, SymbolKind, SOURCE_PRIORITY_GAME_DATA};
use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DEFAULT_OUTPUT: &str = "tools/reports/lsp-completion-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_ROWS: usize = 75;
const SAMPLE_ITEM_LIMIT: usize = 5;

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
    if let Some(max_files) = args.max_files {
        files.truncate(max_files);
    }
    let discovery_elapsed = discovery_start.elapsed();

    let external_start = Instant::now();
    let external_index = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &args.scripts_path,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .map(|result| result.index)?;
    let external_elapsed = external_start.elapsed();

    let scan_start = Instant::now();
    let mut rows = Vec::new();
    let mut totals = Totals::default();
    let mut context_counts = BTreeMap::<String, usize>::new();
    let mut failure_counts = BTreeMap::<String, usize>::new();
    let mut owner_counts = BTreeMap::<String, usize>::new();
    let mut candidate_buckets = BTreeMap::<String, usize>::new();
    let mut empty_classification_counts = BTreeMap::<String, usize>::new();

    let sampled_files = sample_evenly_paths(&files, args.max_checks.max(1));
    let per_file_limit = per_file_limit(args.max_checks, sampled_files.len());
    'files: for file in sampled_files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        let source = String::from_utf8_lossy(&bytes);
        let relative_path = relative_display(file, &args.scripts_path);
        let analysis = file_index_for_source(&source);
        let completion_offsets = completion_sample_offsets(&source);
        let sampled_offsets = sample_evenly_samples(&completion_offsets, per_file_limit);
        totals.files += 1;
        totals.bytes += bytes.len();
        for sample in &completion_offsets {
            match sample.kind {
                CompletionSampleKind::Member => totals.member_positions_seen += 1,
                CompletionSampleKind::Prefix => totals.prefix_positions_seen += 1,
            }
        }

        for sample in sampled_offsets {
            if rows.len() >= args.max_checks {
                break 'files;
            }
            let start = Instant::now();
            let position = position_for_offset(&source, sample.offset);
            let report = completion_report_for_cached_analysis_with_external(
                &source,
                &analysis,
                position,
                Some(&external_index),
            );
            let elapsed = start.elapsed();
            let source_line = line_text_at_offset(&source, sample.offset);
            let empty_classification = (report.candidate_count == 0).then(|| {
                classify_empty_result(&report, &source_line, &relative_path, &external_index)
                    .to_string()
            });
            record_counts(
                &report,
                &mut context_counts,
                &mut failure_counts,
                &mut owner_counts,
                &mut candidate_buckets,
            );
            totals.checks += 1;
            totals.parse_diagnostics += report.parse_diagnostics;
            totals.total_candidates += report.candidate_count;
            totals.context_detection += report.timings.context_detection;
            totals.receiver_inference += report.timings.receiver_inference;
            totals.candidate_lookup += report.timings.candidate_lookup;
            totals.item_rendering += report.timings.item_rendering;
            totals.completion_total += report.timings.total;
            if report.candidate_count == 0 {
                totals.empty_results += 1;
                if let Some(classification) = &empty_classification {
                    *empty_classification_counts
                        .entry(classification.clone())
                        .or_default() += 1;
                }
            }
            if report.failure_reason.is_some() {
                totals.failures += 1;
            }
            rows.push(CheckRow {
                path: relative_path.clone(),
                sample_kind: sample.kind,
                line: position.line + 1,
                character: position.character,
                source_line,
                empty_classification,
                elapsed,
                report,
            });
        }
    }
    let scan_elapsed = scan_start.elapsed();
    let render_start = Instant::now();
    let report = render_report(
        &args,
        &rows,
        &totals,
        &context_counts,
        &failure_counts,
        &owner_counts,
        &candidate_buckets,
        &empty_classification_counts,
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

#[derive(Debug)]
struct Args {
    scripts_path: PathBuf,
    out_path: PathBuf,
    profile_label: String,
    max_files: Option<usize>,
    max_checks: usize,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut scripts_path = None;
        let mut out_path = None;
        let mut profile_label = "debug".to_string();
        let mut max_files = None;
        let mut max_checks = 1000usize;
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
                "--max-files" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--max-files requires a number".to_string())?;
                    max_files = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| format!("Invalid --max-files value: {value}"))?,
                    );
                }
                "--max-checks" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--max-checks requires a number".to_string())?;
                    max_checks = value
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid --max-checks value: {value}"))?;
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }

        Ok(Self {
            scripts_path: scripts_path.unwrap_or_else(default_scripts_path),
            out_path: resolve_repo_path(out_path, DEFAULT_OUTPUT),
            profile_label,
            max_files,
            max_checks,
        })
    }
}

#[derive(Default)]
struct Totals {
    files: usize,
    bytes: usize,
    member_positions_seen: usize,
    prefix_positions_seen: usize,
    checks: usize,
    parse_diagnostics: usize,
    total_candidates: usize,
    empty_results: usize,
    failures: usize,
    context_detection: Duration,
    receiver_inference: Duration,
    candidate_lookup: Duration,
    item_rendering: Duration,
    completion_total: Duration,
}

struct CheckRow {
    path: String,
    sample_kind: CompletionSampleKind,
    line: u32,
    character: u32,
    source_line: String,
    empty_classification: Option<String>,
    elapsed: Duration,
    report: LspCompletionReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionSampleKind {
    Member,
    Prefix,
}

impl CompletionSampleKind {
    fn as_str(self) -> &'static str {
        match self {
            CompletionSampleKind::Member => "member-dot",
            CompletionSampleKind::Prefix => "identifier-prefix",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CompletionSample {
    offset: usize,
    kind: CompletionSampleKind,
}

fn completion_sample_offsets(source: &str) -> Vec<CompletionSample> {
    let mut samples = Vec::new();
    for token in lex(source) {
        if token.kind == TokenKind::Dot {
            samples.push(CompletionSample {
                offset: token.span.end,
                kind: CompletionSampleKind::Member,
            });
            continue;
        }

        if token.kind != TokenKind::Identifier {
            continue;
        }

        let text = &source[token.span.start..token.span.end];
        for prefix in ["SCR_", "GetG", "Widget", "Base"] {
            if text.starts_with(prefix)
                && text.len() > prefix.len()
                && !is_after_member_dot(source, token.span.start)
            {
                samples.push(CompletionSample {
                    offset: token.span.start + prefix.len(),
                    kind: CompletionSampleKind::Prefix,
                });
                break;
            }
        }
    }
    samples.sort_by_key(|sample| sample.offset);
    samples
}

fn is_after_member_dot(source: &str, offset: usize) -> bool {
    source[..offset]
        .chars()
        .rev()
        .find(|character| !character.is_whitespace())
        == Some('.')
}

fn record_counts(
    report: &LspCompletionReport,
    context_counts: &mut BTreeMap<String, usize>,
    failure_counts: &mut BTreeMap<String, usize>,
    owner_counts: &mut BTreeMap<String, usize>,
    candidate_buckets: &mut BTreeMap<String, usize>,
) {
    *context_counts
        .entry(report.completion_context.clone())
        .or_default() += 1;
    if let Some(failure) = &report.failure_reason {
        *failure_counts.entry(failure.clone()).or_default() += 1;
    }
    if let Some(owner) = &report.owner_type {
        *owner_counts.entry(owner.clone()).or_default() += 1;
    }
    *candidate_buckets
        .entry(candidate_bucket(report.candidate_count).to_string())
        .or_default() += 1;
}

fn render_report(
    args: &Args,
    rows: &[CheckRow],
    totals: &Totals,
    context_counts: &BTreeMap<String, usize>,
    failure_counts: &BTreeMap<String, usize>,
    owner_counts: &BTreeMap<String, usize>,
    candidate_buckets: &BTreeMap<String, usize>,
    empty_classification_counts: &BTreeMap<String, usize>,
    discovery_elapsed: Duration,
    external_elapsed: Duration,
    scan_elapsed: Duration,
    render_start: Instant,
) -> String {
    let mut report = String::new();
    writeln!(report, "# LSP Completion Corpus Report").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "- Source path: `{}`", args.scripts_path.display()).unwrap();
    writeln!(report, "- Profile: `{}`", args.profile_label).unwrap();
    writeln!(report, "- Files scanned: {}", totals.files).unwrap();
    writeln!(
        report,
        "- Member positions seen: {}",
        totals.member_positions_seen
    )
    .unwrap();
    writeln!(
        report,
        "- Prefix positions seen: {}",
        totals.prefix_positions_seen
    )
    .unwrap();
    writeln!(report, "- Completion checks: {}", totals.checks).unwrap();
    writeln!(report, "- Empty results: {}", totals.empty_results).unwrap();
    writeln!(report, "- Failure results: {}", totals.failures).unwrap();
    writeln!(report).unwrap();
    writeln!(report, "This report samples real member-access `receiver.` positions and real identifier-prefix positions such as `SCR_`, `GetG`, and generic type prefixes, then runs the same completion path used by `textDocument/completion`. It is review tooling only; Workbench remains compiler truth.").unwrap();

    append_summary(&mut report, totals);
    append_counts(&mut report, "Completion Context Frequency", context_counts);
    append_counts(&mut report, "Failure Reason Frequency", failure_counts);
    append_counts(&mut report, "Candidate Count Buckets", candidate_buckets);
    append_counts(
        &mut report,
        "Empty Result Classification",
        empty_classification_counts,
    );
    append_counts_limited(&mut report, "Top Inferred Owner Types", owner_counts);
    append_empty_or_failure_samples(&mut report, rows);
    append_large_candidate_samples(&mut report, rows);
    append_timing(
        &mut report,
        rows,
        totals,
        discovery_elapsed,
        external_elapsed,
        scan_elapsed,
        render_start,
    );
    report
}

fn append_summary(report: &mut String, totals: &Totals) {
    report.push_str("\n## Summary\n\n");
    report.push_str("| Metric | Count |\n");
    report.push_str("| --- | ---: |\n");
    writeln!(report, "| Files | {} |", totals.files).unwrap();
    writeln!(report, "| Bytes | {} |", totals.bytes).unwrap();
    writeln!(
        report,
        "| Member positions seen | {} |",
        totals.member_positions_seen
    )
    .unwrap();
    writeln!(
        report,
        "| Prefix positions seen | {} |",
        totals.prefix_positions_seen
    )
    .unwrap();
    writeln!(report, "| Checks | {} |", totals.checks).unwrap();
    writeln!(
        report,
        "| Parse diagnostics | {} |",
        totals.parse_diagnostics
    )
    .unwrap();
    writeln!(report, "| Total candidates | {} |", totals.total_candidates).unwrap();
    writeln!(
        report,
        "| Average candidates | {:.2} |",
        if totals.checks == 0 {
            0.0
        } else {
            totals.total_candidates as f64 / totals.checks as f64
        }
    )
    .unwrap();
    writeln!(report, "| Empty results | {} |", totals.empty_results).unwrap();
    writeln!(report, "| Failure results | {} |", totals.failures).unwrap();
}

fn append_counts(report: &mut String, title: &str, counts: &BTreeMap<String, usize>) {
    writeln!(report, "\n## {title}\n").unwrap();
    if counts.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Value | Count |\n");
    report.push_str("| --- | ---: |\n");
    for (value, count) in sorted_counts(counts) {
        writeln!(report, "| `{}` | {} |", escape_table(&value), count).unwrap();
    }
}

fn append_counts_limited(report: &mut String, title: &str, counts: &BTreeMap<String, usize>) {
    writeln!(report, "\n## {title}\n").unwrap();
    if counts.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Value | Count |\n");
    report.push_str("| --- | ---: |\n");
    for (value, count) in sorted_counts(counts).into_iter().take(MAX_ROWS) {
        writeln!(report, "| `{}` | {} |", escape_table(&value), count).unwrap();
    }
}

fn append_empty_or_failure_samples(report: &mut String, rows: &[CheckRow]) {
    report.push_str("\n## Empty Or Failure Samples\n\n");
    let samples = rows
        .iter()
        .filter(|row| row.report.candidate_count == 0 || row.report.failure_reason.is_some())
        .take(MAX_ROWS)
        .collect::<Vec<_>>();
    if samples.is_empty() {
        report.push_str("None.\n");
        return;
    }
    append_sample_table(report, &samples);
}

fn append_large_candidate_samples(report: &mut String, rows: &[CheckRow]) {
    report.push_str("\n## Large Candidate Samples\n\n");
    let mut samples = rows.iter().collect::<Vec<_>>();
    samples.sort_by(|left, right| {
        right
            .report
            .candidate_count
            .cmp(&left.report.candidate_count)
            .then_with(|| left.path.cmp(&right.path))
    });
    let samples = samples.into_iter().take(MAX_ROWS).collect::<Vec<_>>();
    append_sample_table(report, &samples);
}

fn append_sample_table(report: &mut String, rows: &[&CheckRow]) {
    report.push_str("| Path | Line | Sample | Context | Receiver | Owner | Prefix | Candidates | Empty class | Failure | Samples | Source |\n");
    report
        .push_str("| --- | ---: | --- | --- | --- | --- | --- | ---: | --- | --- | --- | --- |\n");
    for row in rows {
        writeln!(
            report,
            "| `{}` | {}:{} | `{}` | `{}` | `{}` | `{}` | `{}` | {} | `{}` | `{}` | `{}` | `{}` |",
            escape_table(&row.path),
            row.line,
            row.character,
            row.sample_kind.as_str(),
            row.report.completion_context,
            escape_table(row.report.receiver_text.as_deref().unwrap_or("<none>")),
            escape_table(row.report.owner_type.as_deref().unwrap_or("<none>")),
            escape_table(&row.report.prefix),
            row.report.candidate_count,
            escape_table(row.empty_classification.as_deref().unwrap_or("<none>")),
            escape_table(row.report.failure_reason.as_deref().unwrap_or("<none>")),
            escape_table(&sample_items(&row.report)),
            escape_table(&row.source_line),
        )
        .unwrap();
    }
}

fn classify_empty_result(
    report: &LspCompletionReport,
    source_line: &str,
    relative_path: &str,
    external_index: &SymbolIndex,
) -> &'static str {
    if report.completion_context == "none" {
        return "source-noise / non-completion-worthy";
    }
    if is_excluded_source_path(relative_path) {
        return "excluded source / non-completion-worthy";
    }
    if report.owner_type.is_none() || report.failure_reason.is_some() {
        return "unresolved receiver";
    }
    let owner = report.owner_type.as_deref().unwrap_or_default();
    if owner_is_enum(owner, external_index) {
        return "enum/static owner";
    }
    if owner_is_class(owner, external_index) && looks_static_owner(report, source_line) {
        return "static class owner";
    }
    if owner_has_any_indexed_symbol(owner, external_index) {
        return "true completion defect";
    }
    "no members indexed for owner"
}

fn is_excluded_source_path(relative_path: &str) -> bool {
    let path = relative_path.replace('\\', "/").to_ascii_lowercase();
    path.starts_with("autotest/")
        || path.contains("/autotest/")
        || path.contains("/workbench/")
        || path.contains("/workbenchgame/")
        || path.contains("/docs/")
        || path.contains("/doxygen/")
}

fn owner_is_enum(owner: &str, index: &SymbolIndex) -> bool {
    index
        .top_level_symbols_for_name(owner)
        .iter()
        .filter_map(|id| index.symbol(*id))
        .any(|symbol| symbol.kind == SymbolKind::Enum)
}

fn owner_is_class(owner: &str, index: &SymbolIndex) -> bool {
    index
        .classes_by_name(owner)
        .iter()
        .filter_map(|id| index.symbol(*id))
        .any(|symbol| symbol.kind == SymbolKind::Class)
}

fn owner_has_any_indexed_symbol(owner: &str, index: &SymbolIndex) -> bool {
    !index.symbols_for_name(owner).is_empty()
}

fn looks_static_owner(report: &LspCompletionReport, source_line: &str) -> bool {
    let receiver = report.receiver_text.as_deref().unwrap_or_default();
    receiver
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
        || source_line.contains(&format!("{receiver}."))
}

fn append_timing(
    report: &mut String,
    rows: &[CheckRow],
    totals: &Totals,
    discovery_elapsed: Duration,
    external_elapsed: Duration,
    scan_elapsed: Duration,
    render_start: Instant,
) {
    report.push_str("\n## Timing\n\n");
    report.push_str("| Phase | Milliseconds |\n");
    report.push_str("| --- | ---: |\n");
    writeln!(
        report,
        "| File discovery | {} |",
        discovery_elapsed.as_millis()
    )
    .unwrap();
    writeln!(
        report,
        "| External index build | {} |",
        external_elapsed.as_millis()
    )
    .unwrap();
    writeln!(
        report,
        "| Completion projection | {} |",
        scan_elapsed.as_millis()
    )
    .unwrap();
    writeln!(
        report,
        "|   Context detection | {} |",
        totals.context_detection.as_millis()
    )
    .unwrap();
    writeln!(
        report,
        "|   Receiver inference | {} |",
        totals.receiver_inference.as_millis()
    )
    .unwrap();
    writeln!(
        report,
        "|   Candidate lookup | {} |",
        totals.candidate_lookup.as_millis()
    )
    .unwrap();
    writeln!(
        report,
        "|   Item rendering | {} |",
        totals.item_rendering.as_millis()
    )
    .unwrap();
    writeln!(
        report,
        "|   Reported completion total | {} |",
        totals.completion_total.as_millis()
    )
    .unwrap();
    writeln!(
        report,
        "| Report rendering | {} |",
        render_start.elapsed().as_millis()
    )
    .unwrap();

    report.push_str("\n## Slowest Completion Checks\n\n");
    let mut sorted = rows.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        right
            .elapsed
            .cmp(&left.elapsed)
            .then_with(|| left.path.cmp(&right.path))
    });
    report.push_str("| Path | Line | Milliseconds | Context | Lookup | Render | Owner | Candidates | Source |\n");
    report.push_str("| --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | --- |\n");
    for row in sorted.into_iter().take(MAX_ROWS) {
        writeln!(
            report,
            "| `{}` | {}:{} | {} | {} | {} | {} | `{}` | {} | `{}` |",
            escape_table(&row.path),
            row.line,
            row.character,
            row.elapsed.as_millis(),
            row.report.timings.context_detection.as_millis(),
            row.report.timings.candidate_lookup.as_millis(),
            row.report.timings.item_rendering.as_millis(),
            escape_table(row.report.owner_type.as_deref().unwrap_or("<none>")),
            row.report.candidate_count,
            escape_table(&row.source_line),
        )
        .unwrap();
    }
}

fn sample_items(report: &LspCompletionReport) -> String {
    report
        .list
        .items
        .iter()
        .take(SAMPLE_ITEM_LIMIT)
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn candidate_bucket(count: usize) -> &'static str {
    match count {
        0 => "0",
        1..=5 => "1-5",
        6..=20 => "6-20",
        21..=50 => "21-50",
        51..=100 => "51-100",
        _ => "100+",
    }
}

fn line_text_at_offset(source: &str, offset: usize) -> String {
    let start = source[..offset]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let end = source[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(source.len());
    source[start..end].trim().to_string()
}

fn sample_evenly_paths(paths: &[PathBuf], limit: usize) -> Vec<&PathBuf> {
    if paths.len() <= limit {
        return paths.iter().collect();
    }
    if limit <= 1 {
        return paths.first().into_iter().collect();
    }
    let mut sampled = Vec::new();
    let mut last_index = None;
    for index in 0..limit {
        let value_index = index * (paths.len() - 1) / (limit - 1);
        if last_index == Some(value_index) {
            continue;
        }
        sampled.push(&paths[value_index]);
        last_index = Some(value_index);
    }
    sampled
}

fn sample_evenly_samples(values: &[CompletionSample], limit: usize) -> Vec<CompletionSample> {
    if values.len() <= limit {
        return values.to_vec();
    }
    if limit <= 1 {
        return values.first().copied().into_iter().collect();
    }
    let mut sampled = Vec::new();
    let mut last_index = None;
    for index in 0..limit {
        let value_index = index * (values.len() - 1) / (limit - 1);
        if last_index == Some(value_index) {
            continue;
        }
        sampled.push(values[value_index]);
        last_index = Some(value_index);
    }
    sampled
}

fn per_file_limit(max_checks: usize, file_count: usize) -> usize {
    if file_count == 0 {
        return 0;
    }
    max_checks.div_ceil(file_count).max(1)
}

fn collect_c_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(root).map_err(|error| format!("Failed to read {}: {error}", root.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to read entry in {}: {error}", root.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_c_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "c") {
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

fn print_help() {
    println!("Usage: cargo run --manifest-path server/Cargo.toml --example lsp_completion_corpus_report -- [--scripts <path>] [--out <path>] [--profile-label <label>] [--max-files <n>] [--max-checks <n>]");
}
