use reforger_language_server::lexer::{lex, TokenKind};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/lexer-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";

struct Args {
    scripts_path: PathBuf,
    out_path: PathBuf,
}

struct FileError {
    path: PathBuf,
    error_count: usize,
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
            "Failed to write corpus report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!("Wrote lexer corpus report: {}", args.out_path.display());
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
                    "Usage: node tools/lexer-corpus-report.mjs [--scripts <path>] [--out <path>]"
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
    let mut total_error_tokens = 0usize;
    let mut doc_comment_count = 0usize;
    let mut token_counts = BTreeMap::<String, usize>::new();
    let mut keyword_counts = BTreeMap::<String, usize>::new();
    let mut operator_counts = BTreeMap::<String, usize>::new();
    let mut unknown_counts = BTreeMap::<String, usize>::new();
    let mut files_with_errors = Vec::<FileError>::new();
    let mut lossy_files = Vec::<LossyFile>::new();

    for file in &files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        total_bytes += bytes.len();

        let source = String::from_utf8_lossy(&bytes);
        if matches!(source, std::borrow::Cow::Owned(_)) {
            lossy_files.push(LossyFile { path: file.clone() });
        }
        let source = source.into_owned();

        let tokens = lex(&source);
        total_tokens += tokens.len();
        let mut file_error_count = 0usize;

        for token in tokens {
            let kind_name = format!("{:?}", token.kind);
            *token_counts.entry(kind_name).or_default() += 1;

            if token.kind.is_error() {
                total_error_tokens += 1;
                file_error_count += 1;
            }

            match token.kind {
                TokenKind::DocLineComment | TokenKind::DocBlockComment => {
                    doc_comment_count += 1;
                }
                TokenKind::Keyword(keyword) => {
                    *keyword_counts.entry(format!("{keyword:?}")).or_default() += 1;
                }
                TokenKind::Operator(operator) => {
                    *operator_counts.entry(format!("{operator:?}")).or_default() += 1;
                }
                TokenKind::Unknown => {
                    let text = &source[token.span.start..token.span.end];
                    *unknown_counts.entry(escape_inline(text)).or_default() += 1;
                }
                _ => {}
            }
        }

        if file_error_count > 0 {
            files_with_errors.push(FileError {
                path: file.clone(),
                error_count: file_error_count,
            });
        }
    }

    let mut report = String::new();
    report.push_str("# Lexer Corpus Report\n\n");
    report.push_str("> Human-review output generated by `node tools/lexer-corpus-report.mjs`.\n\n");
    report.push_str("This report summarizes real game-data tokenization. It is review data only; Workbench remains compiler truth.\n\n");

    report.push_str("## Summary\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Source path | `{}` |\n", scripts_path.display()));
    report.push_str(&format!("| `.c` files | {} |\n", files.len()));
    report.push_str(&format!("| Bytes | {} |\n", total_bytes));
    report.push_str(&format!("| Tokens including trivia | {} |\n", total_tokens));
    report.push_str(&format!(
        "| Documentation comments | {} |\n",
        doc_comment_count
    ));
    report.push_str(&format!("| Error tokens | {} |\n", total_error_tokens));
    report.push_str(&format!(
        "| Files with errors | {} |\n",
        files_with_errors.len()
    ));
    report.push_str(&format!(
        "| Files decoded lossily | {} |\n\n",
        lossy_files.len()
    ));

    append_counts(&mut report, "Token Counts", &token_counts, 80);
    append_counts(&mut report, "Keyword Frequency", &keyword_counts, 80);
    append_counts(&mut report, "Operator Frequency", &operator_counts, 80);
    append_counts(&mut report, "Unknown Text Frequency", &unknown_counts, 40);
    append_file_errors(&mut report, scripts_path, &files_with_errors);
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

fn append_file_errors(report: &mut String, scripts_path: &Path, files: &[FileError]) {
    report.push_str("## Files With Lexer Errors\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File | Error tokens |\n");
    report.push_str("| --- | ---: |\n");
    for file in files.iter().take(100) {
        let relative = file
            .path
            .strip_prefix(scripts_path)
            .unwrap_or(&file.path)
            .display();
        report.push_str(&format!("| `{}` | {} |\n", relative, file.error_count));
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
        let relative = file
            .path
            .strip_prefix(scripts_path)
            .unwrap_or(&file.path)
            .display();
        report.push_str(&format!("| `{}` |\n", relative));
    }
    report.push('\n');
}

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut values: Vec<_> = counts
        .iter()
        .map(|(item, count)| (item.clone(), *count))
        .collect();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values
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
