use reforger_language_server::parser::parse_source;
use reforger_language_server::syntax::{ParseDiagnostic, SyntaxElement, SyntaxKind, SyntaxNode};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/parser-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_DIAGNOSTIC_FILES: usize = 100;
const MAX_SNIPPET_FILES: usize = 25;
const MAX_DIAGNOSTICS_PER_SNIPPET_FILE: usize = 3;
const SNIPPET_CONTEXT_LINES: usize = 2;

struct Args {
    scripts_path: PathBuf,
    out_path: PathBuf,
}

struct FileDiagnostics {
    path: PathBuf,
    diagnostics: Vec<ParseDiagnostic>,
    source: String,
}

struct RecoveryFile {
    path: PathBuf,
    error_nodes: usize,
    expected_error_nodes: usize,
}

struct LossyFile {
    path: PathBuf,
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
            "Failed to write parser corpus report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!("Wrote parser corpus report: {}", args.out_path.display());
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
                    "Usage: node tools/parser-corpus-report.mjs [--scripts <path>] [--out <path>]"
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
    if !scripts_path.is_dir() {
        return Err(format!(
            "Scripts folder does not exist or is not a folder: {}",
            scripts_path.display()
        ));
    }

    let mut files = Vec::new();
    collect_script_files(scripts_path, &mut files)?;
    files.sort();

    let mut total_bytes = 0usize;
    let mut total_tokens = 0usize;
    let mut total_diagnostics = 0usize;
    let mut syntax_counts = BTreeMap::<String, usize>::new();
    let mut diagnostic_counts = BTreeMap::<String, usize>::new();
    let mut files_with_diagnostics = Vec::<FileDiagnostics>::new();
    let mut recovery_files = Vec::<RecoveryFile>::new();
    let mut lossy_files = Vec::<LossyFile>::new();

    for file in &files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        total_bytes += bytes.len();

        let source = String::from_utf8_lossy(&bytes);
        if matches!(source, Cow::Owned(_)) {
            lossy_files.push(LossyFile { path: file.clone() });
        }
        let source = source.into_owned();

        let parse = parse_source(&source);
        total_tokens += parse.root.token_count();
        total_diagnostics += parse.diagnostics.len();
        count_kinds(&parse.root, &mut syntax_counts);
        let error_nodes = count_kind(&parse.root, SyntaxKind::Error);
        if error_nodes > 0 {
            recovery_files.push(RecoveryFile {
                path: file.clone(),
                error_nodes,
                expected_error_nodes: expected_recovery_node_count(&parse.root, &source, file),
            });
        }

        for diagnostic in &parse.diagnostics {
            *diagnostic_counts
                .entry(diagnostic.message.clone())
                .or_default() += 1;
        }

        if !parse.diagnostics.is_empty() {
            files_with_diagnostics.push(FileDiagnostics {
                path: file.clone(),
                diagnostics: parse.diagnostics,
                source,
            });
        }
    }

    files_with_diagnostics.sort_by(|left, right| {
        right
            .diagnostics
            .len()
            .cmp(&left.diagnostics.len())
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut report = String::new();
    report.push_str("# Parser Corpus Report\n\n");
    report
        .push_str("> Human-review output generated by `node tools/parser-corpus-report.mjs`.\n\n");
    report.push_str("This report summarizes declaration-parser behavior across real game-data scripts. It is review data only; Workbench remains compiler truth.\n\n");

    report.push_str("## Summary\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Source path | `{}` |\n", scripts_path.display()));
    report.push_str(&format!(
        "| Scan timestamp unix seconds | {} |\n",
        timestamp()
    ));
    report.push_str(&format!("| `.c` files | {} |\n", files.len()));
    report.push_str(&format!("| Bytes | {} |\n", total_bytes));
    report.push_str(&format!("| Tokens preserved | {} |\n", total_tokens));
    report.push_str(&format!("| Diagnostics | {} |\n", total_diagnostics));
    report.push_str(&format!(
        "| Files with diagnostics | {} |\n",
        files_with_diagnostics.len()
    ));
    report.push_str(&format!(
        "| Files decoded lossily | {} |\n\n",
        lossy_files.len()
    ));

    append_counts(&mut report, "Syntax Kind Frequency", &syntax_counts, 80);
    append_counts(
        &mut report,
        "Diagnostic Message Frequency",
        &diagnostic_counts,
        80,
    );
    append_diagnostic_files(&mut report, scripts_path, &files_with_diagnostics);
    append_diagnostic_snippets(&mut report, scripts_path, &files_with_diagnostics);
    append_expected_recovery_nodes(&mut report, scripts_path, &recovery_files);
    append_lossy_files(&mut report, scripts_path, &lossy_files);

    Ok(report)
}

fn collect_script_files(folder: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Failed to read folder {}: {error}", folder.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to read entry in {}: {error}", folder.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_script_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "c") {
            files.push(path);
        }
    }

    Ok(())
}

fn count_kinds(node: &SyntaxNode, counts: &mut BTreeMap<String, usize>) {
    *counts.entry(format!("{:?}", node.kind)).or_default() += 1;
    for child in &node.children {
        if let SyntaxElement::Node(child_node) = child {
            count_kinds(child_node, counts);
        }
    }
}

fn count_kind(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    let own = usize::from(node.kind == kind);
    own + node
        .children
        .iter()
        .map(|child| match child {
            SyntaxElement::Node(child) => count_kind(child, kind),
            SyntaxElement::Token(_) => 0,
        })
        .sum::<usize>()
}

fn expected_recovery_node_count(node: &SyntaxNode, source: &str, path: &Path) -> usize {
    let relative = path.display().to_string().replace('/', "\\");
    if !relative.ends_with("Game\\game.c") || !source.contains("#ifdef BREAK_COMPILATION") {
        return 0;
    }

    count_kind(node, SyntaxKind::Error)
}

fn append_counts(
    report: &mut String,
    heading: &str,
    counts: &BTreeMap<String, usize>,
    limit: usize,
) {
    report.push_str(&format!("## {heading}\n\n"));

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

fn append_diagnostic_files(report: &mut String, scripts_path: &Path, files: &[FileDiagnostics]) {
    report.push_str("## Top Files With Parser Diagnostics\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File | Diagnostics |\n");
    report.push_str("| --- | ---: |\n");
    for file in files.iter().take(MAX_DIAGNOSTIC_FILES) {
        report.push_str(&format!(
            "| `{}` | {} |\n",
            relative_path(scripts_path, &file.path),
            file.diagnostics.len()
        ));
    }
    report.push('\n');
}

fn append_diagnostic_snippets(report: &mut String, scripts_path: &Path, files: &[FileDiagnostics]) {
    report.push_str("## Diagnostic Snippets\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    for file in files.iter().take(MAX_SNIPPET_FILES) {
        report.push_str(&format!(
            "### `{}`\n\n",
            relative_path(scripts_path, &file.path)
        ));
        for diagnostic in file
            .diagnostics
            .iter()
            .take(MAX_DIAGNOSTICS_PER_SNIPPET_FILE)
        {
            let (line, column) = line_column(&file.source, diagnostic.span.start);
            report.push_str(&format!(
                "- `{}` at {}:{} span {}..{}\n\n",
                escape_inline(&diagnostic.message),
                line,
                column,
                diagnostic.span.start,
                diagnostic.span.end
            ));
            append_source_snippet(report, &file.source, line);
        }
    }
}

fn append_source_snippet(report: &mut String, source: &str, line: usize) {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        report.push_str("````text\n<empty file>\n````\n\n");
        return;
    }

    let start = line.saturating_sub(SNIPPET_CONTEXT_LINES + 1);
    let end = (line + SNIPPET_CONTEXT_LINES).min(lines.len());

    report.push_str("````enforce\n");
    for index in start..end {
        let marker = if index + 1 == line { ">" } else { " " };
        report.push_str(&format!(
            "{marker} {:>5} | {}\n",
            index + 1,
            lines[index].replace('\t', "    ")
        ));
    }
    report.push_str("````\n\n");
}

fn append_expected_recovery_nodes(
    report: &mut String,
    scripts_path: &Path,
    files: &[RecoveryFile],
) {
    report.push_str("## Expected Recovery Nodes\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    let total = files.iter().map(|file| file.error_nodes).sum::<usize>();
    let expected = files
        .iter()
        .map(|file| file.expected_error_nodes)
        .sum::<usize>();

    report.push_str("| Metric | Count |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Error nodes | {total} |\n"));
    report.push_str(&format!(
        "| Expected preprocessor-test recovery | {expected} |\n"
    ));
    report.push_str(&format!(
        "| Unexplained recovery nodes | {} |\n\n",
        total.saturating_sub(expected)
    ));

    report.push_str("| File | Error nodes | Classification |\n");
    report.push_str("| --- | ---: | --- |\n");
    for file in files.iter().take(MAX_DIAGNOSTIC_FILES) {
        let classification = if file.expected_error_nodes == file.error_nodes {
            "expected `#ifdef BREAK_COMPILATION` preprocessor-test text"
        } else if file.expected_error_nodes > 0 {
            "mixed expected and unexplained recovery"
        } else {
            "unexplained recovery"
        };
        report.push_str(&format!(
            "| `{}` | {} | {classification} |\n",
            relative_path(scripts_path, &file.path),
            file.error_nodes
        ));
    }
    report.push('\n');
}

fn append_lossy_files(report: &mut String, scripts_path: &Path, files: &[LossyFile]) {
    report.push_str("## Files Decoded Lossily\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File |\n");
    report.push_str("| --- |\n");
    for file in files.iter().take(100) {
        report.push_str(&format!(
            "| `{}` |\n",
            relative_path(scripts_path, &file.path)
        ));
    }
    report.push('\n');
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;

    for (index, value) in source.char_indices() {
        if index >= offset {
            break;
        }

        if value == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut values: Vec<_> = counts
        .iter()
        .map(|(item, count)| (item.clone(), *count))
        .collect();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
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
