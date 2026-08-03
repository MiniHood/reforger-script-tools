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
const MAX_CLASSIFICATION_SAMPLES_PER_BUCKET: usize = 3;
const ATTRIBUTE_NAMED_ARGUMENT: &str = "attribute named argument";
const ATTRIBUTE_ENUM_STATIC_VALUE: &str = "attribute enum/static value";
const PREPROCESSOR_DIRECTIVE_TOKEN: &str = "preprocessor directive token";
const PREPROCESSOR_MACRO_TOKEN: &str = "preprocessor macro token";
const CALL_NAMED_ARGUMENT_LABEL: &str = "call named argument label";
const EXCLUDED_SOURCE: &str = "workbench/docs/test excluded source";
const EXTERNAL_NATIVE_UNAVAILABLE: &str = "external/native unavailable";
const UNCONSTRAINED_GENERIC_RECEIVER: &str = "unconstrained generic receiver";

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
    let mut receiver_owner_counts = BTreeMap::<String, usize>::new();
    let mut receiver_failure_counts = BTreeMap::<String, usize>::new();
    let mut selected_source_counts = BTreeMap::<String, usize>::new();
    let mut kind_counts = BTreeMap::<String, usize>::new();
    let mut miss_classification_counts = BTreeMap::<String, usize>::new();
    let mut miss_classification_samples = BTreeMap::<String, Vec<SampleRow>>::new();
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
            non_hover_misses: 0,
            actionable_misses: 0,
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
            let receiver_owner = report
                .receiver_resolution
                .as_ref()
                .and_then(|receiver| receiver.owner_type.clone())
                .unwrap_or_else(|| "<none>".to_string());
            if receiver_owner != "<none>" {
                *receiver_owner_counts
                    .entry(receiver_owner.clone())
                    .or_default() += 1;
            }
            let receiver_failure = report
                .receiver_resolution
                .as_ref()
                .and_then(|receiver| receiver.failure_reason.clone())
                .unwrap_or_else(|| "<none>".to_string());
            if receiver_failure != "<none>" {
                *receiver_failure_counts
                    .entry(receiver_failure.clone())
                    .or_default() += 1;
            }

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
                    receiver_owner,
                    report
                        .receiver_resolution
                        .as_ref()
                        .map(|receiver| receiver.receiver_expression_kind.clone())
                        .unwrap_or_else(|| "<none>".to_string()),
                    receiver_failure,
                ));
            } else {
                row.misses += 1;
                totals.misses += 1;
                if report.resolver_reason == Some(ResolutionReason::Unresolved) {
                    row.unresolved_misses += 1;
                    totals.unresolved_misses += 1;
                }
                let sample_row = SampleRow::new(
                    &relative_path,
                    &source,
                    sample,
                    report,
                    reason,
                    context,
                    selected_source,
                    receiver_owner,
                    report
                        .receiver_resolution
                        .as_ref()
                        .map(|receiver| receiver.receiver_expression_kind.clone())
                        .unwrap_or_else(|| "<none>".to_string()),
                    receiver_failure,
                );
                *miss_classification_counts
                    .entry(sample_row.miss_classification.clone())
                    .or_default() += 1;
                if is_non_hover_miss_classification(&sample_row.miss_classification) {
                    row.non_hover_misses += 1;
                    totals.non_hover_misses += 1;
                } else {
                    row.actionable_misses += 1;
                    totals.actionable_misses += 1;
                }
                let bucket = miss_classification_samples
                    .entry(sample_row.miss_classification.clone())
                    .or_default();
                if bucket.len() < MAX_CLASSIFICATION_SAMPLES_PER_BUCKET {
                    bucket.push(sample_row.clone());
                }
                miss_samples.push(sample_row);
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
        &receiver_owner_counts,
        &receiver_failure_counts,
        &selected_source_counts,
        &kind_counts,
        &miss_classification_counts,
        &miss_classification_samples,
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
    non_hover_misses: usize,
    actionable_misses: usize,
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
    non_hover_misses: usize,
    actionable_misses: usize,
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
    receiver_owner: String,
    receiver_expression_kind: String,
    receiver_failure: String,
    selected_source: String,
    selected: String,
    miss_classification: String,
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
        receiver_owner: String,
        receiver_expression_kind: String,
        receiver_failure: String,
    ) -> Self {
        let position = position_for_offset(source, sample.span.start);
        let selected = match (&report.selected_kind, &report.selected_label) {
            (Some(kind), Some(label)) => format!("{} `{}`", symbol_kind_label(*kind), label),
            _ => "<none>".to_string(),
        };
        let (line_text, token_index) = source_line_at_offset(source, sample.span.start);
        let snippet = display_source_line(&line_text);
        let miss_classification = classify_miss(
            path,
            report,
            &sample.token,
            &reason,
            &context,
            &receiver_owner,
            &receiver_failure,
            &line_text,
            token_index,
        );
        Self {
            path: path.to_string(),
            line: position.line + 1,
            column: position.character + 1,
            token: sample.token.clone(),
            hit: report.is_hit(),
            reason,
            context,
            receiver_owner,
            receiver_expression_kind,
            receiver_failure,
            selected_source,
            selected,
            miss_classification,
            snippet,
        }
    }
}

fn classify_miss(
    path: &str,
    report: &LspHoverReport,
    token: &str,
    reason: &str,
    context: &str,
    receiver_owner: &str,
    receiver_failure: &str,
    line_text: &str,
    token_index: usize,
) -> String {
    if report.is_hit() {
        return "<hit>".to_string();
    }

    if reason == "preprocessor-directive" {
        return PREPROCESSOR_DIRECTIVE_TOKEN.to_string();
    }

    if reason == "preprocessor-macro" {
        return PREPROCESSOR_MACRO_TOKEN.to_string();
    }

    if reason == "attribute-named-argument" {
        return ATTRIBUTE_NAMED_ARGUMENT.to_string();
    }

    if reason == "named-argument-label" {
        if is_attribute_named_argument(line_text, token_index, token) {
            return ATTRIBUTE_NAMED_ARGUMENT.to_string();
        }
        return CALL_NAMED_ARGUMENT_LABEL.to_string();
    }

    if is_preprocessor_token(token, line_text) {
        return PREPROCESSOR_DIRECTIVE_TOKEN.to_string();
    }

    if is_preprocessor_macro_token(token) {
        return PREPROCESSOR_MACRO_TOKEN.to_string();
    }

    if is_attribute_named_argument(line_text, token_index, token) {
        return ATTRIBUTE_NAMED_ARGUMENT.to_string();
    }

    if is_call_named_argument_label(line_text, token_index, token) {
        return CALL_NAMED_ARGUMENT_LABEL.to_string();
    }

    if is_attribute_value_expression(line_text, token_index) {
        return ATTRIBUTE_ENUM_STATIC_VALUE.to_string();
    }

    if is_excluded_source_path(path) {
        return EXCLUDED_SOURCE.to_string();
    }

    if is_external_native_unavailable(token, line_text, receiver_failure) {
        return EXTERNAL_NATIVE_UNAVAILABLE.to_string();
    }

    if context == "member-access" {
        if is_unconstrained_generic_receiver(receiver_owner) {
            return UNCONSTRAINED_GENERIC_RECEIVER.to_string();
        }
        if is_invoker_like(receiver_owner, receiver_failure) {
            return "invoker/delegate member".to_string();
        }
        if is_indexed_receiver(line_text, token_index) {
            return "indexed receiver".to_string();
        }
        if is_pseudo_or_primitive_member(receiver_owner, token) {
            return "pseudo/primitive member".to_string();
        }
        if is_field_chain_receiver(line_text, token_index) {
            return "field-chain receiver".to_string();
        }
        return "receiver unresolved".to_string();
    }

    if reason == "unresolved" && looks_like_call(line_text, token_index, token) {
        return "unqualified inherited member".to_string();
    }

    "unknown unresolved".to_string()
}

fn is_non_hover_miss_classification(classification: &str) -> bool {
    matches!(
        classification,
        ATTRIBUTE_NAMED_ARGUMENT
            | ATTRIBUTE_ENUM_STATIC_VALUE
            | PREPROCESSOR_DIRECTIVE_TOKEN
            | PREPROCESSOR_MACRO_TOKEN
            | CALL_NAMED_ARGUMENT_LABEL
            | EXCLUDED_SOURCE
            | EXTERNAL_NATIVE_UNAVAILABLE
            | UNCONSTRAINED_GENERIC_RECEIVER
    )
}

fn is_preprocessor_token(token: &str, snippet: &str) -> bool {
    matches!(
        token,
        "if" | "ifdef" | "ifndef" | "elif" | "else" | "endif" | "define" | "undef" | "include"
    ) || snippet.trim_start().starts_with('#')
}

fn is_preprocessor_macro_token(token: &str) -> bool {
    matches!(
        token,
        "__FILE__" | "__LINE__" | "__FUNC__" | "__DATE__" | "__TIME__"
    )
}

fn is_attribute_named_argument(line_text: &str, token_index: usize, token: &str) -> bool {
    if token_index > line_text.len() || token_index + token.len() > line_text.len() {
        return false;
    }
    let before = &line_text[..token_index];
    let after = &line_text[token_index + token.len()..];
    before.rfind('[').is_some_and(|open| {
        before[open..].find(']').is_none() && after.trim_start().starts_with(':')
    })
}

fn is_call_named_argument_label(line_text: &str, token_index: usize, token: &str) -> bool {
    if token_index > line_text.len() || token_index + token.len() > line_text.len() {
        return false;
    }
    let before = &line_text[..token_index];
    let after = &line_text[token_index + token.len()..];
    if !after.trim_start().starts_with(':') {
        return false;
    }
    if before.trim().is_empty() {
        return true;
    }
    has_unclosed_paren_before(before) || before.contains(',')
}

fn has_unclosed_paren_before(text: &str) -> bool {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for ch in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }

    depth > 0
}

fn is_attribute_value_expression(line_text: &str, token_index: usize) -> bool {
    if token_index > line_text.len() {
        return false;
    }
    let before = &line_text[..token_index];
    before
        .rfind('[')
        .is_some_and(|open| before[open..].find(']').is_none())
}

fn is_excluded_source_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    normalized.contains("/workbench")
        || normalized.starts_with("workbench")
        || normalized.contains("/docs/")
        || normalized.contains("/doxygen/")
        || normalized.ends_with("rpldocs.c")
        || normalized.contains("/autotest/")
        || normalized.starts_with("autotest/")
        || normalized.contains("/test/")
        || normalized.starts_with("test/")
}

fn is_external_native_unavailable(token: &str, line_text: &str, receiver_failure: &str) -> bool {
    token == "AddDamage"
        || receiver_failure.contains("`AddDamage`")
        || line_text.contains(".AddDamage(")
}

fn is_unconstrained_generic_receiver(receiver_owner: &str) -> bool {
    matches!(receiver_owner, "OWNER_TYPE" | "T")
}

fn is_invoker_like(receiver_owner: &str, receiver_failure: &str) -> bool {
    receiver_owner.contains("Invoker")
        || receiver_owner.contains("ScriptInvoker")
        || ["Insert", "Remove", "Invoke", "Clear"]
            .iter()
            .any(|member| receiver_failure.contains(&format!("`{member}`")))
}

fn is_indexed_receiver(line_text: &str, token_index: usize) -> bool {
    if token_index > line_text.len() {
        return false;
    }
    line_text[..token_index].contains("].")
}

fn is_pseudo_or_primitive_member(receiver_owner: &str, token: &str) -> bool {
    matches!(
        receiver_owner,
        "T" | "int" | "float" | "bool" | "string" | "vector" | "typename" | "Class"
    ) || matches!(token, "ToString" | "Type" | "ClassName" | "IsInherited")
}

fn is_field_chain_receiver(line_text: &str, token_index: usize) -> bool {
    if token_index > line_text.len() {
        return false;
    }
    line_text[..token_index].matches('.').count() >= 2
}

fn looks_like_call(line_text: &str, token_index: usize, token: &str) -> bool {
    if token_index > line_text.len() || token_index + token.len() > line_text.len() {
        return false;
    }
    line_text[token_index + token.len()..]
        .trim_start()
        .starts_with('(')
}

fn render_report(
    args: &Args,
    totals: &Totals,
    file_rows: &[FileRow],
    reason_counts: &BTreeMap<String, usize>,
    context_counts: &BTreeMap<String, usize>,
    receiver_owner_counts: &BTreeMap<String, usize>,
    receiver_failure_counts: &BTreeMap<String, usize>,
    selected_source_counts: &BTreeMap<String, usize>,
    kind_counts: &BTreeMap<String, usize>,
    miss_classification_counts: &BTreeMap<String, usize>,
    miss_classification_samples: &BTreeMap<String, Vec<SampleRow>>,
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
        "Receiver Owner Frequency",
        receiver_owner_counts,
    );
    append_counts(
        &mut report,
        "Receiver Failure Frequency",
        receiver_failure_counts,
    );
    append_miss_classification(
        &mut report,
        miss_classification_counts,
        miss_classification_samples,
    );
    append_counts(
        &mut report,
        "Selected Source Frequency",
        selected_source_counts,
    );
    append_counts(&mut report, "Selected Kind Frequency", kind_counts);
    append_top_actionable_files(&mut report, file_rows);
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
    let actionable_samples = totals
        .identifier_samples
        .saturating_sub(totals.non_hover_misses);
    let actionable_hit_rate = percentage(totals.hits, actionable_samples);
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
    writeln!(report, "| Raw hit rate | {:.2}% |", hit_rate).unwrap();
    writeln!(
        report,
        "| Non-hover/source-noise misses | {} |",
        totals.non_hover_misses
    )
    .unwrap();
    writeln!(
        report,
        "| Actionable hover samples | {} |",
        actionable_samples
    )
    .unwrap();
    writeln!(
        report,
        "| Actionable hover misses | {} |",
        totals.actionable_misses
    )
    .unwrap();
    writeln!(
        report,
        "| Actionable hover hit rate | {:.2}% |",
        actionable_hit_rate
    )
    .unwrap();
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

fn append_miss_classification(
    report: &mut String,
    counts: &BTreeMap<String, usize>,
    samples: &BTreeMap<String, Vec<SampleRow>>,
) {
    writeln!(report, "## Remaining Miss Classification").unwrap();
    writeln!(report).unwrap();
    if counts.is_empty() {
        writeln!(report, "None.").unwrap();
        writeln!(report).unwrap();
        return;
    }

    writeln!(report, "| Classification | Count |").unwrap();
    writeln!(report, "| --- | ---: |").unwrap();
    for (classification, count) in sorted_counts(counts) {
        writeln!(
            report,
            "| `{}` | {} |",
            escape_table(&classification),
            count
        )
        .unwrap();
    }
    writeln!(report).unwrap();

    writeln!(report, "### Classification Samples").unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "| Classification | File | Position | Token | Reason | Context | Receiver owner | Receiver expression | Receiver failure | Source line |"
    )
    .unwrap();
    writeln!(
        report,
        "| --- | --- | ---: | --- | --- | --- | --- | --- | --- | --- |"
    )
    .unwrap();
    for (classification, rows) in samples {
        for sample in rows {
            writeln!(
                report,
                "| `{}` | `{}` | {}:{} | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |",
                escape_table(classification),
                escape_table(&sample.path),
                sample.line,
                sample.column,
                escape_table(&sample.token),
                escape_table(&sample.reason),
                escape_table(&sample.context),
                escape_table(&sample.receiver_owner),
                escape_table(&sample.receiver_expression_kind),
                escape_table(&sample.receiver_failure),
                escape_table(&sample.snippet)
            )
            .unwrap();
        }
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
        "| File | Bytes | Samples | Hits | Misses | Non-hover/source-noise | Actionable misses | Unresolved | Parse diagnostics | Elapsed ms |"
    )
    .unwrap();
    writeln!(
        report,
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    )
    .unwrap();
    for row in by_misses.into_iter().take(MAX_ROWS) {
        if row.misses == 0 {
            continue;
        }
        writeln!(
            report,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            escape_table(&row.path),
            row.bytes,
            row.identifier_samples,
            row.hits,
            row.misses,
            row.non_hover_misses,
            row.actionable_misses,
            row.unresolved_misses,
            row.parse_diagnostics,
            row.elapsed_ms
        )
        .unwrap();
    }
    writeln!(report).unwrap();
}

fn append_top_actionable_files(report: &mut String, rows: &[FileRow]) {
    let mut by_actionable = rows.to_vec();
    by_actionable.sort_by(|left, right| {
        right
            .actionable_misses
            .cmp(&left.actionable_misses)
            .then_with(|| right.misses.cmp(&left.misses))
            .then_with(|| left.path.cmp(&right.path))
    });
    writeln!(report, "## Top Files By Actionable Hover Misses").unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "This excludes sampled misses classified as attribute labels/values, preprocessor tokens, named call labels, Workbench/docs/test source noise, unavailable native/API declarations, or unconstrained generic receivers."
    )
    .unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "| File | Bytes | Samples | Hits | Actionable misses | Non-hover/source-noise | Raw misses | Parse diagnostics | Elapsed ms |"
    )
    .unwrap();
    writeln!(
        report,
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    )
    .unwrap();
    for row in by_actionable.into_iter().take(MAX_ROWS) {
        if row.actionable_misses == 0 {
            continue;
        }
        writeln!(
            report,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} |",
            escape_table(&row.path),
            row.bytes,
            row.identifier_samples,
            row.hits,
            row.actionable_misses,
            row.non_hover_misses,
            row.misses,
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
        "| File | Position | Token | Hit | Reason | Context | Receiver owner | Receiver expression | Receiver failure | Selected source | Selected | Source line |"
    )
    .unwrap();
    writeln!(
        report,
        "| --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    )
    .unwrap();
    for sample in samples {
        writeln!(
            report,
            "| `{}` | {}:{} | `{}` | {} | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | {} | `{}` |",
            escape_table(&sample.path),
            sample.line,
            sample.column,
            escape_table(&sample.token),
            if sample.hit { "yes" } else { "no" },
            escape_table(&sample.reason),
            escape_table(&sample.context),
            escape_table(&sample.receiver_owner),
            escape_table(&sample.receiver_expression_kind),
            escape_table(&sample.receiver_failure),
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
    writeln!(report, "- The report supplies the full game-data index as external hover context. Remaining misses are unresolved after file-local and external top-level/member lookup.").unwrap();
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

fn source_line_at_offset(source: &str, offset: usize) -> (String, usize) {
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line_end = source[offset..]
        .find('\n')
        .map_or(source.len(), |index| offset + index);
    let line = source[line_start..line_end]
        .trim_end_matches('\r')
        .to_string();
    let token_index = offset.saturating_sub(line_start).min(line.len());
    (line, token_index)
}

fn display_source_line(line: &str) -> String {
    line.trim().chars().take(180).collect()
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
