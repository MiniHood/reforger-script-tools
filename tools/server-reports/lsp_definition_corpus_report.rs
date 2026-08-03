use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::lexer::{lex, TokenKind};
use reforger_language_server::lsp::{
    definition_report_for_source_position_with_external, position_for_offset, symbol_kind_label,
    LspDefinitionReport,
};
use reforger_language_server::model::{SourceKind, SOURCE_PRIORITY_GAME_DATA};
use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DEFAULT_OUTPUT: &str = "tools/reports/lsp-definition-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_ROWS: usize = 75;

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
    let mut reason_counts = BTreeMap::<String, usize>::new();
    let mut context_counts = BTreeMap::<String, usize>::new();
    let mut source_counts = BTreeMap::<String, usize>::new();
    let mut kind_counts = BTreeMap::<String, usize>::new();
    let mut miss_counts = BTreeMap::<String, usize>::new();
    let mut miss_review_counts = BTreeMap::<String, usize>::new();
    let mut target_review_counts = BTreeMap::<String, usize>::new();

    let sampled_files = sample_evenly_paths(&files, args.max_checks.max(1));
    let per_file_limit = per_file_limit(args.max_checks, sampled_files.len());
    'files: for file in sampled_files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        let source = String::from_utf8_lossy(&bytes);
        let relative_path = relative_display(file, &args.scripts_path);
        let uri = format!("file:///{}", file.display().to_string().replace('\\', "/"));
        totals.files += 1;
        totals.bytes += bytes.len();

        let identifiers = sample_evenly_identifiers(&identifier_offsets(&source), per_file_limit);
        for (offset, token_text) in identifiers {
            if rows.len() >= args.max_checks {
                break 'files;
            }
            let start = Instant::now();
            let position = position_for_offset(&source, offset);
            let report = definition_report_for_source_position_with_external(
                &source,
                &uri,
                position,
                Some(&external_index),
            );
            let source_line = line_text_at_offset(&source, offset);
            let miss_bucket = (!report.is_hit())
                .then(|| miss_bucket(&source, offset, &token_text, &report).to_string());
            let elapsed = start.elapsed();
            record_counts(
                &report,
                &mut reason_counts,
                &mut context_counts,
                &mut source_counts,
                &mut kind_counts,
            );
            totals.checks += 1;
            totals.parse_diagnostics += report.parse_diagnostics;
            totals.resolver_candidates += report.resolver_candidate_count;
            if report.is_hit() {
                totals.hits += 1;
                *target_review_counts
                    .entry(target_review_bucket(&report).to_string())
                    .or_default() += 1;
            } else {
                totals.misses += 1;
                if let Some(bucket) = &miss_bucket {
                    *miss_counts.entry(bucket.clone()).or_default() += 1;
                    *miss_review_counts
                        .entry(miss_review_bucket(bucket).to_string())
                        .or_default() += 1;
                }
            }
            rows.push(CheckRow {
                path: relative_path.clone(),
                token_text,
                line: position.line + 1,
                character: position.character,
                source_line,
                miss_bucket,
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
        &reason_counts,
        &context_counts,
        &source_counts,
        &kind_counts,
        &miss_counts,
        &miss_review_counts,
        &target_review_counts,
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
    checks: usize,
    hits: usize,
    misses: usize,
    parse_diagnostics: usize,
    resolver_candidates: usize,
}

struct CheckRow {
    path: String,
    token_text: String,
    line: u32,
    character: u32,
    source_line: String,
    miss_bucket: Option<String>,
    elapsed: Duration,
    report: LspDefinitionReport,
}

fn identifier_offsets(source: &str) -> Vec<(usize, String)> {
    lex(source)
        .into_iter()
        .filter(|token| token.kind == TokenKind::Identifier)
        .map(|token| {
            (
                token.span.start,
                source[token.span.start..token.span.end].to_string(),
            )
        })
        .collect()
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

fn sample_evenly_identifiers(values: &[(usize, String)], limit: usize) -> Vec<(usize, String)> {
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

fn per_file_limit(max_checks: usize, file_count: usize) -> usize {
    if file_count == 0 {
        return 0;
    }
    max_checks.div_ceil(file_count).max(1)
}

fn record_counts(
    report: &LspDefinitionReport,
    reason_counts: &mut BTreeMap<String, usize>,
    context_counts: &mut BTreeMap<String, usize>,
    source_counts: &mut BTreeMap<String, usize>,
    kind_counts: &mut BTreeMap<String, usize>,
) {
    let reason = report
        .resolver_reason
        .map(|reason| reason.as_str())
        .unwrap_or("<none>");
    *reason_counts.entry(reason.to_string()).or_default() += 1;
    let context = report
        .identifier_context
        .map(|context| context.as_str())
        .unwrap_or("<none>");
    *context_counts.entry(context.to_string()).or_default() += 1;
    let source = report
        .selected_source
        .map(|source| source.as_str())
        .unwrap_or("<none>");
    *source_counts.entry(source.to_string()).or_default() += 1;
    let kind = report
        .selected_kind
        .map(symbol_kind_label)
        .unwrap_or("<none>");
    *kind_counts.entry(kind.to_string()).or_default() += 1;
}

fn render_report(
    args: &Args,
    rows: &[CheckRow],
    totals: &Totals,
    reason_counts: &BTreeMap<String, usize>,
    context_counts: &BTreeMap<String, usize>,
    source_counts: &BTreeMap<String, usize>,
    kind_counts: &BTreeMap<String, usize>,
    miss_counts: &BTreeMap<String, usize>,
    miss_review_counts: &BTreeMap<String, usize>,
    target_review_counts: &BTreeMap<String, usize>,
    discovery_elapsed: Duration,
    external_elapsed: Duration,
    scan_elapsed: Duration,
    render_start: Instant,
) -> String {
    let mut report = String::new();
    writeln!(report, "# LSP Definition Corpus Report").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "- Source path: `{}`", args.scripts_path.display()).unwrap();
    writeln!(report, "- Profile: `{}`", args.profile_label).unwrap();
    writeln!(report, "- Files scanned: {}", totals.files).unwrap();
    writeln!(report, "- Definition checks: {}", totals.checks).unwrap();
    writeln!(report, "- Hits: {}", totals.hits).unwrap();
    writeln!(report, "- Misses: {}", totals.misses).unwrap();
    writeln!(
        report,
        "- Hit rate: {:.2}%",
        percent(totals.hits, totals.checks)
    )
    .unwrap();
    writeln!(report).unwrap();
    writeln!(report, "This report samples real identifier tokens and runs the same resolver-first location projection used by `textDocument/definition`. It is review tooling only; Workbench remains compiler truth.").unwrap();

    append_summary(&mut report, totals);
    append_counts(&mut report, "Resolver Reason Frequency", reason_counts);
    append_counts(&mut report, "Identifier Context Frequency", context_counts);
    append_counts(&mut report, "Selected Source Frequency", source_counts);
    append_counts(&mut report, "Selected Kind Frequency", kind_counts);
    append_counts(&mut report, "Miss Classification", miss_counts);
    append_definition_review(&mut report, rows, miss_review_counts, target_review_counts);
    append_miss_samples(&mut report, rows);
    append_timing(
        &mut report,
        rows,
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
    writeln!(report, "| Checks | {} |", totals.checks).unwrap();
    writeln!(report, "| Hits | {} |", totals.hits).unwrap();
    writeln!(report, "| Misses | {} |", totals.misses).unwrap();
    writeln!(
        report,
        "| Hit rate | {:.2}% |",
        percent(totals.hits, totals.checks)
    )
    .unwrap();
    writeln!(
        report,
        "| Parse diagnostics | {} |",
        totals.parse_diagnostics
    )
    .unwrap();
    writeln!(
        report,
        "| Average resolver candidates | {:.2} |",
        if totals.checks == 0 {
            0.0
        } else {
            totals.resolver_candidates as f64 / totals.checks as f64
        }
    )
    .unwrap();
}

fn append_counts(report: &mut String, title: &str, counts: &BTreeMap<String, usize>) {
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

fn append_definition_review(
    report: &mut String,
    rows: &[CheckRow],
    miss_review_counts: &BTreeMap<String, usize>,
    target_review_counts: &BTreeMap<String, usize>,
) {
    append_counts(report, "Definition Target Review", target_review_counts);

    report.push_str("\nThe corpus report uses one game-data external index, so `external game-data` means a Ctrl+click target outside the sampled open file but still inside the indexed scripts corpus. Workspace overlay targets are covered by LSP workspace overlay tests, not this corpus report.\n");

    append_counts(report, "Miss Review Buckets", miss_review_counts);

    append_receiver_member_miss_samples(report, rows);
    append_actionable_miss_samples(report, rows);
}

fn append_receiver_member_miss_samples(report: &mut String, rows: &[CheckRow]) {
    report.push_str("\n## Receiver / Member Definition Misses\n\n");
    let samples = rows
        .iter()
        .filter(|row| {
            !row.report.is_hit()
                && (row
                    .report
                    .identifier_context
                    .is_some_and(|context| context.as_str() == "member-access")
                    || row
                        .miss_bucket
                        .as_deref()
                        .is_some_and(|bucket| bucket == "unresolved member access"))
        })
        .take(MAX_ROWS)
        .collect::<Vec<_>>();
    if samples.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Path | Line | Token | Bucket | Reason | Source |\n");
    report.push_str("| --- | ---: | --- | --- | --- | --- |\n");
    for row in samples {
        writeln!(
            report,
            "| `{}` | {}:{} | `{}` | `{}` | `{}` | `{}` |",
            escape_table(&row.path),
            row.line,
            row.character,
            escape_table(&row.token_text),
            escape_table(row.miss_bucket.as_deref().unwrap_or("<none>")),
            row.report
                .resolver_reason
                .map(|reason| reason.as_str())
                .unwrap_or("<none>"),
            escape_table(&row.source_line),
        )
        .unwrap();
    }
}

fn append_actionable_miss_samples(report: &mut String, rows: &[CheckRow]) {
    report.push_str("\n## Actionable Definition Miss Samples\n\n");
    let samples = rows
        .iter()
        .filter(|row| {
            !row.report.is_hit()
                && row
                    .miss_bucket
                    .as_deref()
                    .is_some_and(|bucket| miss_review_bucket(bucket) == "actionable unresolved")
        })
        .take(MAX_ROWS)
        .collect::<Vec<_>>();
    if samples.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Path | Line | Token | Bucket | Reason | Context | Source |\n");
    report.push_str("| --- | ---: | --- | --- | --- | --- | --- |\n");
    for row in samples {
        writeln!(
            report,
            "| `{}` | {}:{} | `{}` | `{}` | `{}` | `{}` | `{}` |",
            escape_table(&row.path),
            row.line,
            row.character,
            escape_table(&row.token_text),
            escape_table(row.miss_bucket.as_deref().unwrap_or("<none>")),
            row.report
                .resolver_reason
                .map(|reason| reason.as_str())
                .unwrap_or("<none>"),
            row.report
                .identifier_context
                .map(|context| context.as_str())
                .unwrap_or("<none>"),
            escape_table(&row.source_line),
        )
        .unwrap();
    }
}

fn append_miss_samples(report: &mut String, rows: &[CheckRow]) {
    report.push_str("\n## Miss Samples\n\n");
    let samples = rows
        .iter()
        .filter(|row| !row.report.is_hit())
        .take(MAX_ROWS)
        .collect::<Vec<_>>();
    if samples.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Path | Line | Token | Bucket | Reason | Context | Candidates | Source |\n");
    report.push_str("| --- | ---: | --- | --- | --- | --- | ---: | --- |\n");
    for row in samples {
        writeln!(
            report,
            "| `{}` | {}:{} | `{}` | `{}` | `{}` | `{}` | {} | `{}` |",
            escape_table(&row.path),
            row.line,
            row.character,
            escape_table(&row.token_text),
            escape_table(row.miss_bucket.as_deref().unwrap_or("<none>")),
            row.report
                .resolver_reason
                .map(|reason| reason.as_str())
                .unwrap_or("<none>"),
            row.report
                .identifier_context
                .map(|context| context.as_str())
                .unwrap_or("<none>"),
            row.report.resolver_candidate_count,
            escape_table(&row.source_line),
        )
        .unwrap();
    }
}

fn append_timing(
    report: &mut String,
    rows: &[CheckRow],
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
        "| Definition projection | {} |",
        scan_elapsed.as_millis()
    )
    .unwrap();
    writeln!(
        report,
        "| Report rendering | {} |",
        render_start.elapsed().as_millis()
    )
    .unwrap();

    report.push_str("\n## Slowest Definition Checks\n\n");
    let mut sorted = rows.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        right
            .elapsed
            .cmp(&left.elapsed)
            .then_with(|| left.path.cmp(&right.path))
    });
    report.push_str("| Path | Line | Milliseconds | Token | Hit | Source |\n");
    report.push_str("| --- | ---: | ---: | --- | --- | --- |\n");
    for row in sorted.into_iter().take(MAX_ROWS) {
        writeln!(
            report,
            "| `{}` | {}:{} | {} | `{}` | {} | `{}` |",
            escape_table(&row.path),
            row.line,
            row.character,
            row.elapsed.as_millis(),
            escape_table(&row.token_text),
            if row.report.is_hit() { "yes" } else { "no" },
            escape_table(&row.source_line),
        )
        .unwrap();
    }
}

fn target_review_bucket(report: &LspDefinitionReport) -> &'static str {
    match report.selected_source.map(|source| source.as_str()) {
        Some("file-local") => "file-local target",
        Some("external") => "external game-data target",
        Some(_) => "other target",
        None => "no target",
    }
}

fn miss_review_bucket(bucket: &str) -> &'static str {
    match bucket {
        "preprocessor directive"
        | "preprocessor macro name"
        | "attribute named argument"
        | "attribute enum/static value"
        | "named call argument label" => "source-noise / non-definition target",
        "unresolved member access" => "actionable receiver/member unresolved",
        "unresolved type" | "unresolved value/callable" | "other unresolved" => {
            "actionable unresolved"
        }
        _ => "other",
    }
}

fn miss_bucket(
    source: &str,
    offset: usize,
    token_text: &str,
    report: &LspDefinitionReport,
) -> &'static str {
    let reason = report
        .resolver_reason
        .map(|reason| reason.as_str())
        .unwrap_or("<none>");
    let context = report
        .identifier_context
        .map(|context| context.as_str())
        .unwrap_or("<none>");
    let line = raw_line_at_offset(source, offset);
    let trimmed = line.trim_start();
    if reason == "preprocessor-directive" {
        "preprocessor directive"
    } else if reason == "preprocessor-macro" {
        "preprocessor macro name"
    } else if reason == "attribute-named-argument" {
        "attribute named argument"
    } else if reason == "named-argument-label" {
        if is_attribute_line(&line) {
            "attribute named argument"
        } else {
            "named call argument label"
        }
    } else if trimmed.starts_with('#') {
        if matches!(
            token_text,
            "ifdef" | "ifndef" | "endif" | "else" | "elif" | "define"
        ) {
            "preprocessor directive"
        } else {
            "preprocessor macro name"
        }
    } else if is_attribute_line(&line) {
        if token_followed_by_colon(source, offset, token_text) {
            "attribute named argument"
        } else {
            "attribute enum/static value"
        }
    } else if token_followed_by_colon(source, offset, token_text) {
        "named call argument label"
    } else if context == "value-or-callable" && reason == "unresolved" {
        "unresolved value/callable"
    } else if context == "type-position" && reason == "unresolved" {
        "unresolved type"
    } else if context == "member-access" {
        "unresolved member access"
    } else {
        "other unresolved"
    }
}

fn is_attribute_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('[') && trimmed.contains(']')
}

fn token_followed_by_colon(source: &str, offset: usize, token_text: &str) -> bool {
    let mut cursor = offset.saturating_add(token_text.len());
    while cursor < source.len() {
        let Some(character) = source[cursor..].chars().next() else {
            break;
        };
        if character == ':' {
            return true;
        }
        if character == '\n' || character == '\r' || !character.is_whitespace() {
            return false;
        }
        cursor += character.len_utf8();
    }
    false
}

fn raw_line_at_offset(source: &str, offset: usize) -> String {
    let start = source[..offset]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let end = source[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(source.len());
    source[start..end].to_string()
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

fn percent(part: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / total as f64
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
    println!("Usage: cargo run --manifest-path server/Cargo.toml --example lsp_definition_corpus_report -- [--scripts <path>] [--out <path>] [--profile-label <label>] [--max-files <n>] [--max-checks <n>]");
}
