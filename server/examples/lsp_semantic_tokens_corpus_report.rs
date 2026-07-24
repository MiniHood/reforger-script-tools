use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::lexer::{lex, TokenKind};
use reforger_language_server::lsp::{
    offset_for_position, semantic_tokens_for_source_with_external,
    semantic_tokens_report_for_source_with_external, LspSemanticTokenReport,
    LspSemanticTokenTimings,
};
use reforger_language_server::model::{SourceKind, SOURCE_PRIORITY_GAME_DATA};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DEFAULT_OUTPUT: &str = "tools/reports/lsp-semantic-tokens-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_ROWS: usize = 50;
const MAX_SAMPLES_PER_FILE: usize = 8;

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
    let files = if let Some(file_path) = args.file_path.clone() {
        if !file_path.is_file() {
            return Err(format!("File does not exist: {}", file_path.display()));
        }
        vec![file_path]
    } else {
        let mut files = Vec::new();
        collect_c_files(&args.scripts_path, &mut files)?;
        files.sort();
        if let Some(max_files) = args.max_files {
            files.truncate(max_files);
        }
        files
    };
    let discovery_elapsed = discovery_start.elapsed();

    let external_start = Instant::now();
    let external_index = if args.external_index {
        Some(
            build_index(&IndexBuildConfig {
                roots: vec![IndexSourceRoot::new(
                    &args.scripts_path,
                    SourceKind::GameData,
                    SOURCE_PRIORITY_GAME_DATA,
                )],
            })
            .map(|result| result.index)?,
        )
    } else {
        None
    };
    let external_elapsed = external_start.elapsed();

    let scan_start = Instant::now();
    let mut rows = Vec::new();
    let mut totals = Totals::default();
    let mut token_type_counts = BTreeMap::<String, usize>::new();
    let mut modifier_counts = BTreeMap::<String, usize>::new();
    let mut uncolored_classification_counts = BTreeMap::<String, usize>::new();

    for file in &files {
        let read_start = Instant::now();
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        let source = String::from_utf8_lossy(&bytes);
        let read_decode_elapsed = read_start.elapsed();

        let projection_start = Instant::now();
        let report = if args.runtime_only {
            let projection =
                semantic_tokens_for_source_with_external(&source, external_index.as_ref());
            RuntimeProjection::TokenOnly {
                semantic_tokens: projection.token_count,
                encoded_integers: projection.tokens.data.len(),
                parse_diagnostics: projection.parse_diagnostics,
                timings: projection.timings,
            }
        } else {
            RuntimeProjection::Report(semantic_tokens_report_for_source_with_external(
                &source,
                external_index.as_ref(),
            ))
        };
        let projection_elapsed = projection_start.elapsed();

        let relative_path = relative_display(file, &args.scripts_path);
        let mut coverage = Coverage::default();
        let parse_diagnostics = report.parse_diagnostics();
        let semantic_tokens = report.semantic_tokens();
        let encoded_integers = report.encoded_integers();
        let semantic_timings = report.timings();
        if let RuntimeProjection::Report(report) = &report {
            coverage = analyze_coverage(&source, report);
            for token in &report.decoded {
                *token_type_counts
                    .entry(token.token_type.to_string())
                    .or_default() += 1;
                for modifier in &token.modifiers {
                    *modifier_counts.entry((*modifier).to_string()).or_default() += 1;
                }
            }
        }

        totals.files += 1;
        totals.bytes += bytes.len();
        totals.parse_diagnostics += parse_diagnostics;
        totals.semantic_tokens += semantic_tokens;
        totals.encoded_integers += encoded_integers;
        totals.identifier_tokens += coverage.identifier_tokens;
        totals.colored_identifiers += coverage.colored_identifiers;
        totals.uncolored_identifiers += coverage.uncolored_identifiers;
        totals.reference_tokens += coverage.reference_tokens;
        totals.declaration_tokens += coverage.declaration_tokens;
        totals.lexical_tokens += coverage.lexical_tokens;
        for (classification, count) in coverage.uncolored_classification_counts {
            *uncolored_classification_counts
                .entry(classification)
                .or_default() += count;
        }

        rows.push(FileRow {
            path: relative_path,
            bytes: bytes.len(),
            parse_diagnostics,
            semantic_tokens,
            encoded_integers,
            identifier_tokens: coverage.identifier_tokens,
            colored_identifiers: coverage.colored_identifiers,
            uncolored_identifiers: coverage.uncolored_identifiers,
            reference_tokens: coverage.reference_tokens,
            declaration_tokens: coverage.declaration_tokens,
            lexical_tokens: coverage.lexical_tokens,
            uncolored_samples: coverage.uncolored_samples,
            read_decode: read_decode_elapsed,
            projection: projection_elapsed,
            semantic_timings,
        });
    }

    let scan_elapsed = scan_start.elapsed();
    let render_start = Instant::now();
    let report = render_report(
        &args,
        &rows,
        &totals,
        &token_type_counts,
        &modifier_counts,
        &uncolored_classification_counts,
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
    file_path: Option<PathBuf>,
    external_index: bool,
    runtime_only: bool,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut scripts_path = None;
        let mut out_path = None;
        let mut profile_label = "debug".to_string();
        let mut max_files = None;
        let mut file_path = None;
        let mut external_index = true;
        let mut runtime_only = false;
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
                "--file" => {
                    file_path = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--file requires a path".to_string())?,
                    ));
                }
                "--no-external-index" => external_index = false,
                "--runtime-only" => runtime_only = true,
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
            file_path,
            external_index,
            runtime_only,
        })
    }
}

enum RuntimeProjection {
    Report(LspSemanticTokenReport),
    TokenOnly {
        semantic_tokens: usize,
        encoded_integers: usize,
        parse_diagnostics: usize,
        timings: LspSemanticTokenTimings,
    },
}

impl RuntimeProjection {
    fn parse_diagnostics(&self) -> usize {
        match self {
            RuntimeProjection::Report(report) => report.parse_diagnostics,
            RuntimeProjection::TokenOnly {
                parse_diagnostics, ..
            } => *parse_diagnostics,
        }
    }

    fn semantic_tokens(&self) -> usize {
        match self {
            RuntimeProjection::Report(report) => report.decoded.len(),
            RuntimeProjection::TokenOnly {
                semantic_tokens, ..
            } => *semantic_tokens,
        }
    }

    fn encoded_integers(&self) -> usize {
        match self {
            RuntimeProjection::Report(report) => report.tokens.data.len(),
            RuntimeProjection::TokenOnly {
                encoded_integers, ..
            } => *encoded_integers,
        }
    }

    fn timings(&self) -> LspSemanticTokenTimings {
        match self {
            RuntimeProjection::Report(report) => report.timings,
            RuntimeProjection::TokenOnly { timings, .. } => *timings,
        }
    }
}

#[derive(Default)]
struct Totals {
    files: usize,
    bytes: usize,
    parse_diagnostics: usize,
    semantic_tokens: usize,
    encoded_integers: usize,
    identifier_tokens: usize,
    colored_identifiers: usize,
    uncolored_identifiers: usize,
    reference_tokens: usize,
    declaration_tokens: usize,
    lexical_tokens: usize,
}

struct FileRow {
    path: String,
    bytes: usize,
    parse_diagnostics: usize,
    semantic_tokens: usize,
    encoded_integers: usize,
    identifier_tokens: usize,
    colored_identifiers: usize,
    uncolored_identifiers: usize,
    reference_tokens: usize,
    declaration_tokens: usize,
    lexical_tokens: usize,
    uncolored_samples: Vec<UncoloredSample>,
    read_decode: Duration,
    projection: Duration,
    semantic_timings: LspSemanticTokenTimings,
}

#[derive(Default)]
struct Coverage {
    identifier_tokens: usize,
    colored_identifiers: usize,
    uncolored_identifiers: usize,
    reference_tokens: usize,
    declaration_tokens: usize,
    lexical_tokens: usize,
    uncolored_samples: Vec<UncoloredSample>,
    uncolored_classification_counts: BTreeMap<String, usize>,
}

struct UncoloredSample {
    token: String,
    classification: String,
    line: usize,
    column: usize,
    snippet: String,
}

fn analyze_coverage(source: &str, report: &LspSemanticTokenReport) -> Coverage {
    let mut coverage = Coverage::default();
    let mut colored_spans = BTreeSet::<(usize, usize)>::new();

    for token in &report.decoded {
        let start = offset_for_position(source, token.range.start).unwrap_or(0);
        let end = offset_for_position(source, token.range.end).unwrap_or(start);
        colored_spans.insert((start, end));
        if is_identifier_like_semantic_type(token.token_type) {
            if token.modifiers.contains(&"declaration") {
                coverage.declaration_tokens += 1;
            } else {
                coverage.reference_tokens += 1;
            }
        } else {
            coverage.lexical_tokens += 1;
        }
    }

    for token in lex(source) {
        if token.kind != TokenKind::Identifier {
            continue;
        }
        coverage.identifier_tokens += 1;
        let key = (token.span.start, token.span.end);
        if colored_spans.contains(&key) {
            coverage.colored_identifiers += 1;
        } else {
            coverage.uncolored_identifiers += 1;
            if coverage.uncolored_samples.len() < MAX_SAMPLES_PER_FILE {
                let token_text = source[token.span.start..token.span.end].to_string();
                let classification =
                    classify_uncolored_identifier(source, token.span.start, &token_text);
                let (line, column) = line_column(source, token.span.start);
                coverage.uncolored_samples.push(UncoloredSample {
                    token: token_text,
                    classification: classification.to_string(),
                    line,
                    column,
                    snippet: raw_line_at_offset(source, token.span.start)
                        .trim()
                        .to_string(),
                });
            }
            let token_text = &source[token.span.start..token.span.end];
            let classification =
                classify_uncolored_identifier(source, token.span.start, token_text);
            *coverage
                .uncolored_classification_counts
                .entry(classification.to_string())
                .or_default() += 1;
        }
    }

    coverage
}

fn classify_uncolored_identifier(source: &str, offset: usize, token_text: &str) -> &'static str {
    let line = raw_line_at_offset(source, offset);
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        if matches!(
            token_text,
            "ifdef" | "ifndef" | "endif" | "else" | "elif" | "define" | "undef" | "include"
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
    } else if previous_significant_char(source, offset) == Some('.') {
        if looks_like_constant_name(token_text) || looks_like_uppercase_value(token_text) {
            "enum/static member unresolved"
        } else if next_significant_char(source, offset.saturating_add(token_text.len()))
            == Some('(')
        {
            "member method unresolved"
        } else {
            "member field unresolved"
        }
    } else if token_text.starts_with("m_") {
        "field/member reference unresolved"
    } else if looks_like_constant_name(token_text) {
        "enum/static value unresolved"
    } else if next_significant_char(source, offset.saturating_add(token_text.len())) == Some('(') {
        "function/method unresolved"
    } else {
        "unresolved identifier"
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

fn previous_significant_char(source: &str, offset: usize) -> Option<char> {
    source[..offset]
        .chars()
        .rev()
        .find(|character| !character.is_whitespace())
}

fn next_significant_char(source: &str, offset: usize) -> Option<char> {
    source[offset..]
        .chars()
        .find(|character| !character.is_whitespace())
}

fn looks_like_constant_name(token_text: &str) -> bool {
    token_text
        .chars()
        .any(|character| character == '_' || character.is_ascii_digit())
        && token_text
            .chars()
            .filter(|character| character.is_ascii_alphabetic())
            .all(|character| character.is_ascii_uppercase())
}

fn looks_like_uppercase_value(token_text: &str) -> bool {
    token_text
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .all(|character| character.is_ascii_uppercase())
        && token_text
            .chars()
            .any(|character| character.is_ascii_alphabetic())
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

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    for character in source[..offset].chars() {
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn is_identifier_like_semantic_type(token_type: &str) -> bool {
    matches!(
        token_type,
        "class"
            | "enum"
            | "type"
            | "function"
            | "reforgerField"
            | "variable"
            | "parameter"
            | "enumMember"
            | "typeParameter"
    )
}

fn render_report(
    args: &Args,
    rows: &[FileRow],
    totals: &Totals,
    token_type_counts: &BTreeMap<String, usize>,
    modifier_counts: &BTreeMap<String, usize>,
    uncolored_classification_counts: &BTreeMap<String, usize>,
    discovery_elapsed: Duration,
    external_elapsed: Duration,
    scan_elapsed: Duration,
    render_start: Instant,
) -> String {
    let mut report = String::new();
    writeln!(report, "# LSP Semantic Tokens Corpus Report").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "- Source path: `{}`", args.scripts_path.display()).unwrap();
    writeln!(report, "- Profile: `{}`", args.profile_label).unwrap();
    writeln!(
        report,
        "- Projection mode: `{}`",
        if args.runtime_only {
            "runtime token-only"
        } else {
            "report with decoded debug rows"
        }
    )
    .unwrap();
    writeln!(
        report,
        "- External index: `{}`",
        if args.external_index {
            "enabled"
        } else {
            "disabled"
        }
    )
    .unwrap();
    if let Some(max_files) = args.max_files {
        writeln!(report, "- Max files: `{max_files}`").unwrap();
    }
    if let Some(file_path) = &args.file_path {
        writeln!(report, "- Target file: `{}`", file_path.display()).unwrap();
    }
    writeln!(report, "- Files scanned: {}", totals.files).unwrap();
    writeln!(report, "- Parse diagnostics: {}", totals.parse_diagnostics).unwrap();
    writeln!(report, "- Semantic tokens: {}", totals.semantic_tokens).unwrap();
    writeln!(
        report,
        "- Identifier coloring coverage: {:.2}%",
        percent(totals.colored_identifiers, totals.identifier_tokens)
    )
    .unwrap();
    writeln!(report).unwrap();
    if args.runtime_only {
        writeln!(report, "This report runs the token-only semantic-token projection used by `textDocument/semanticTokens/full`. Coverage and classification tables are intentionally empty in this mode because decoded debug rows are not built.").unwrap();
    } else {
        writeln!(report, "This report runs semantic-token projection with decoded debug rows across the script corpus. It validates Enforce coloring coverage after the TextMate grammar removal; Workbench remains compiler truth.").unwrap();
    }

    append_summary(&mut report, totals);
    append_counts(
        &mut report,
        "Semantic Token Type Frequency",
        token_type_counts,
    );
    append_counts(
        &mut report,
        "Semantic Token Modifier Frequency",
        modifier_counts,
    );
    append_counts(
        &mut report,
        "Uncolored Identifier Classification",
        uncolored_classification_counts,
    );
    append_weak_files(&mut report, rows);
    append_uncolored_samples_by_classification(&mut report, rows);
    append_uncolored_samples(&mut report, rows);
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

fn append_uncolored_samples_by_classification(report: &mut String, rows: &[FileRow]) {
    report.push_str("\n## Uncolored Identifier Samples By Classification\n\n");
    let mut samples = BTreeMap::<String, Vec<(&FileRow, &UncoloredSample)>>::new();
    for row in rows {
        for sample in &row.uncolored_samples {
            samples
                .entry(sample.classification.clone())
                .or_default()
                .push((row, sample));
        }
    }
    if samples.is_empty() {
        report.push_str("None.\n");
        return;
    }

    for (classification, mut entries) in samples {
        entries.sort_by(|left, right| {
            left.0
                .path
                .cmp(&right.0.path)
                .then_with(|| left.1.line.cmp(&right.1.line))
                .then_with(|| left.1.column.cmp(&right.1.column))
        });
        writeln!(report, "\n### {classification}\n").unwrap();
        report.push_str("| Path | Line | Column | Token | Snippet |\n");
        report.push_str("| --- | ---: | ---: | --- | --- |\n");
        for (row, sample) in entries.into_iter().take(MAX_ROWS) {
            writeln!(
                report,
                "| `{}` | {} | {} | `{}` | `{}` |",
                escape_table(&row.path),
                sample.line,
                sample.column,
                escape_table(&sample.token),
                escape_table(&sample.snippet),
            )
            .unwrap();
        }
    }
}

fn append_summary(report: &mut String, totals: &Totals) {
    report.push_str("\n## Summary\n\n");
    report.push_str("| Metric | Count |\n");
    report.push_str("| --- | ---: |\n");
    writeln!(report, "| Files | {} |", totals.files).unwrap();
    writeln!(report, "| Bytes | {} |", totals.bytes).unwrap();
    writeln!(
        report,
        "| Parse diagnostics | {} |",
        totals.parse_diagnostics
    )
    .unwrap();
    writeln!(report, "| Semantic tokens | {} |", totals.semantic_tokens).unwrap();
    writeln!(report, "| Encoded integers | {} |", totals.encoded_integers).unwrap();
    writeln!(
        report,
        "| Identifier tokens | {} |",
        totals.identifier_tokens
    )
    .unwrap();
    writeln!(
        report,
        "| Colored identifiers | {} |",
        totals.colored_identifiers
    )
    .unwrap();
    writeln!(
        report,
        "| Uncolored identifiers | {} |",
        totals.uncolored_identifiers
    )
    .unwrap();
    writeln!(
        report,
        "| Identifier coloring coverage | {:.2}% |",
        percent(totals.colored_identifiers, totals.identifier_tokens)
    )
    .unwrap();
    writeln!(
        report,
        "| Declaration semantic tokens | {} |",
        totals.declaration_tokens
    )
    .unwrap();
    writeln!(
        report,
        "| Reference semantic tokens | {} |",
        totals.reference_tokens
    )
    .unwrap();
    writeln!(
        report,
        "| Lexical semantic tokens | {} |",
        totals.lexical_tokens
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

fn append_weak_files(report: &mut String, rows: &[FileRow]) {
    report.push_str("\n## Weakest Identifier Coloring Coverage\n\n");
    let mut sorted = rows
        .iter()
        .filter(|row| row.identifier_tokens > 0)
        .collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        coverage_basis_points(left)
            .cmp(&coverage_basis_points(right))
            .then_with(|| right.uncolored_identifiers.cmp(&left.uncolored_identifiers))
            .then_with(|| left.path.cmp(&right.path))
    });
    if sorted.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Path | Identifier coverage | Identifiers | Uncolored | Semantic tokens | Declaration | Reference | Lexical | Encoded ints | Bytes | Parse diagnostics |\n");
    report.push_str(
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for row in sorted.into_iter().take(MAX_ROWS) {
        writeln!(
            report,
            "| `{}` | {:.2}% | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            escape_table(&row.path),
            percent(row.colored_identifiers, row.identifier_tokens),
            row.identifier_tokens,
            row.uncolored_identifiers,
            row.semantic_tokens,
            row.declaration_tokens,
            row.reference_tokens,
            row.lexical_tokens,
            row.encoded_integers,
            row.bytes,
            row.parse_diagnostics,
        )
        .unwrap();
    }
}

fn append_uncolored_samples(report: &mut String, rows: &[FileRow]) {
    report.push_str("\n## Uncolored Identifier Samples\n\n");
    let mut sorted = rows
        .iter()
        .filter(|row| row.uncolored_identifiers > 0)
        .collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        right
            .uncolored_identifiers
            .cmp(&left.uncolored_identifiers)
            .then_with(|| left.path.cmp(&right.path))
    });
    if sorted.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Path | Uncolored | Samples |\n");
    report.push_str("| --- | ---: | --- |\n");
    for row in sorted.into_iter().take(MAX_ROWS) {
        writeln!(
            report,
            "| `{}` | {} | `{}` |",
            escape_table(&row.path),
            row.uncolored_identifiers,
            escape_table(
                &row.uncolored_samples
                    .iter()
                    .map(|sample| {
                        format!(
                            "{}:{} {} ({})",
                            sample.line, sample.column, sample.token, sample.classification
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("`, `"),
            ),
        )
        .unwrap();
    }
}

fn append_timing(
    report: &mut String,
    rows: &[FileRow],
    discovery_elapsed: Duration,
    external_elapsed: Duration,
    scan_elapsed: Duration,
    render_start: Instant,
) {
    report.push_str("\n## Timing\n\n");
    report.push_str(
        "Wall-clock timings are dev-report diagnostics, not benchmark-grade measurements.\n\n",
    );
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
        "| Read/decode + semantic projection | {} |",
        scan_elapsed.as_millis()
    )
    .unwrap();
    writeln!(
        report,
        "| Report rendering | {} |",
        render_start.elapsed().as_millis()
    )
    .unwrap();

    let semantic_lex_ms: u128 = rows.iter().map(|row| row.semantic_timings.lex_ms).sum();
    let semantic_token_loop_ms: u128 = rows
        .iter()
        .map(|row| row.semantic_timings.token_loop_ms)
        .sum();
    let semantic_resolver_ms: u128 = rows
        .iter()
        .map(|row| row.semantic_timings.resolver_ms)
        .sum();
    let semantic_declaration_overlay_ms: u128 = rows
        .iter()
        .map(|row| row.semantic_timings.declaration_overlay_ms)
        .sum();
    let semantic_type_detail_overlay_ms: u128 = rows
        .iter()
        .map(|row| row.semantic_timings.type_detail_overlay_ms)
        .sum();
    let semantic_symbol_declaration_overlay_ms: u128 = rows
        .iter()
        .map(|row| row.semantic_timings.symbol_declaration_overlay_ms)
        .sum();
    let semantic_delimiter_overlay_ms: u128 = rows
        .iter()
        .map(|row| row.semantic_timings.delimiter_overlay_ms)
        .sum();
    let semantic_sort_filter_split_ms: u128 = rows
        .iter()
        .map(|row| row.semantic_timings.sort_filter_split_ms)
        .sum();
    let semantic_encode_ms: u128 = rows.iter().map(|row| row.semantic_timings.encode_ms).sum();
    let semantic_decode_debug_ms: u128 = rows
        .iter()
        .map(|row| row.semantic_timings.decode_debug_ms)
        .sum();
    let resolver_calls: usize = rows
        .iter()
        .map(|row| row.semantic_timings.identifier_resolver_calls)
        .sum();
    let delimiter_resolver_calls: usize = rows
        .iter()
        .map(|row| row.semantic_timings.delimiter_resolver_calls)
        .sum();
    let delimiter_owners_reused: usize = rows
        .iter()
        .map(|row| row.semantic_timings.delimiter_owners_reused)
        .sum();
    let delimiter_owners_invalidated: usize = rows
        .iter()
        .map(|row| row.semantic_timings.delimiter_owners_invalidated)
        .sum();
    let delimiter_owners_recomputed: usize = rows
        .iter()
        .map(|row| row.semantic_timings.delimiter_owners_recomputed)
        .sum();
    report.push_str("\n## Semantic Projection Internal Timing\n\n");
    report.push_str("| Phase | Milliseconds |\n");
    report.push_str("| --- | ---: |\n");
    writeln!(report, "| Lex | {semantic_lex_ms} |").unwrap();
    writeln!(report, "| Token loop | {semantic_token_loop_ms} |").unwrap();
    writeln!(report, "| Resolver calls | {semantic_resolver_ms} |").unwrap();
    writeln!(
        report,
        "| Declaration overlay | {semantic_declaration_overlay_ms} |"
    )
    .unwrap();
    writeln!(
        report,
        "|   Type details | {semantic_type_detail_overlay_ms} |"
    )
    .unwrap();
    writeln!(
        report,
        "|   Declaration symbols | {semantic_symbol_declaration_overlay_ms} |"
    )
    .unwrap();
    writeln!(
        report,
        "|   Scope delimiters | {semantic_delimiter_overlay_ms} |"
    )
    .unwrap();
    writeln!(
        report,
        "|   Scope delimiter resolver calls | {delimiter_resolver_calls} |"
    )
    .unwrap();
    writeln!(
        report,
        "|   Scope delimiter owners reused | {delimiter_owners_reused} |"
    )
    .unwrap();
    writeln!(
        report,
        "|   Scope delimiter owners invalidated | {delimiter_owners_invalidated} |"
    )
    .unwrap();
    writeln!(
        report,
        "|   Scope delimiter owners recomputed | {delimiter_owners_recomputed} |"
    )
    .unwrap();
    writeln!(
        report,
        "| Sort/filter/split | {semantic_sort_filter_split_ms} |"
    )
    .unwrap();
    writeln!(report, "| Encode | {semantic_encode_ms} |").unwrap();
    writeln!(report, "| Decode debug rows | {semantic_decode_debug_ms} |").unwrap();
    writeln!(report, "| Identifier resolver calls | {resolver_calls} |").unwrap();

    let mut projection = rows
        .iter()
        .map(|row| row.projection.as_micros())
        .collect::<Vec<_>>();
    projection.sort_unstable();
    report.push_str("\n## Slowest Semantic Token Files\n\n");
    let mut sorted = rows.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        right
            .projection
            .cmp(&left.projection)
            .then_with(|| left.path.cmp(&right.path))
    });
    report.push_str(
        "| Path | Projection ms | Read/decode ms | Semantic tokens | Identifier coverage |\n",
    );
    report.push_str("| --- | ---: | ---: | ---: | ---: |\n");
    for row in sorted.into_iter().take(MAX_ROWS) {
        writeln!(
            report,
            "| `{}` | {} | {} | {} | {:.2}% |",
            escape_table(&row.path),
            row.projection.as_millis(),
            row.read_decode.as_millis(),
            row.semantic_tokens,
            percent(row.colored_identifiers, row.identifier_tokens),
        )
        .unwrap();
    }
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

fn coverage_basis_points(row: &FileRow) -> usize {
    if row.identifier_tokens == 0 {
        return 10_000;
    }
    (row.colored_identifiers * 10_000) / row.identifier_tokens
}

fn percent(part: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / total as f64
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
    println!("Usage: cargo run --manifest-path server/Cargo.toml --example lsp_semantic_tokens_corpus_report -- [--scripts <path>] [--file <path>] [--out <path>] [--profile-label <label>] [--max-files <n>] [--no-external-index] [--runtime-only]");
}
