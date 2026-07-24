use reforger_language_server::lsp::{
    document_symbol_report_for_source, LspDocumentSymbol, LspPosition, LspRange,
};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_FIXTURE_ROOT: &str = "tools/fixtures/parser";
const DEFAULT_OUTPUT: &str = "tools/reports/lsp-fixtures.report.md";
const MAX_SYMBOL_LINES_PER_FILE: usize = 140;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let fixture_root = args
        .fixture_root
        .unwrap_or_else(|| repo_path(DEFAULT_FIXTURE_ROOT));
    let out_path = args.out_path.unwrap_or_else(|| repo_path(DEFAULT_OUTPUT));

    if !fixture_root.is_dir() {
        return Err(format!(
            "Fixture root does not exist: {}",
            fixture_root.display()
        ));
    }

    let mut files = Vec::new();
    collect_c_files(&fixture_root, &mut files)?;
    files.sort();

    let mut rows = Vec::new();
    let mut details = String::new();
    let mut total_parse_diagnostics = 0usize;
    let mut total_unknown_labels = 0usize;
    let mut total_range_failures = 0usize;
    let mut total_symbols = 0usize;

    for file in &files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        let source = String::from_utf8_lossy(&bytes);
        let report = document_symbol_report_for_source(&source);
        let top_level_symbols = report.symbols.len();
        let symbol_count = report.total_symbol_count();
        let nested_symbols = symbol_count.saturating_sub(top_level_symbols);
        let max_depth = max_symbol_depth(&report.symbols);
        let unknown_labels = count_unknown_labels(&report.symbols);
        let range_failures = count_range_failures(&report.symbols);
        let relative_path = relative_display(file, &fixture_root);

        total_parse_diagnostics += report.parse_diagnostics;
        total_unknown_labels += unknown_labels;
        total_range_failures += range_failures;
        total_symbols += symbol_count;

        rows.push(FileRow {
            path: relative_path.clone(),
            parse_diagnostics: report.parse_diagnostics,
            top_level_symbols,
            nested_symbols,
            max_depth,
            unknown_labels,
            range_failures,
            byte_count: bytes.len(),
        });

        writeln!(details, "## {relative_path}").unwrap();
        writeln!(details).unwrap();
        writeln!(details, "- Bytes: {}", bytes.len()).unwrap();
        writeln!(details, "- Parse diagnostics: {}", report.parse_diagnostics).unwrap();
        writeln!(details, "- Top-level symbols: {top_level_symbols}").unwrap();
        writeln!(details, "- Nested symbols: {nested_symbols}").unwrap();
        writeln!(details, "- Max tree depth: {max_depth}").unwrap();
        writeln!(details, "- Unknown labels: {unknown_labels}").unwrap();
        writeln!(details, "- Range sanity failures: {range_failures}").unwrap();
        writeln!(details).unwrap();
        writeln!(details, "```text").unwrap();
        let mut rendered_lines = 0usize;
        render_symbol_tree(&report.symbols, 0, &mut rendered_lines, &mut details);
        if rendered_lines == 0 {
            writeln!(details, "<no symbols>").unwrap();
        } else if rendered_lines >= MAX_SYMBOL_LINES_PER_FILE {
            writeln!(
                details,
                "... truncated after {MAX_SYMBOL_LINES_PER_FILE} symbol lines"
            )
            .unwrap();
        }
        writeln!(details, "```").unwrap();
        writeln!(details).unwrap();
    }

    let mut markdown = String::new();
    writeln!(markdown, "# LSP Fixture Report").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "- Fixture root: `{}`", fixture_root.display()).unwrap();
    writeln!(markdown, "- Files: {}", rows.len()).unwrap();
    writeln!(markdown, "- Total document symbols: {total_symbols}").unwrap();
    writeln!(markdown, "- Parse diagnostics: {total_parse_diagnostics}").unwrap();
    writeln!(markdown, "- Unknown labels: {total_unknown_labels}").unwrap();
    writeln!(markdown, "- Range sanity failures: {total_range_failures}").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "This report exercises the same document-symbol conversion path used by `textDocument/documentSymbol`. It is review tooling only; it does not validate compiler truth.").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "## Summary").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "| File | Bytes | Parse diagnostics | Top-level | Nested | Max depth | Unknown labels | Range failures |").unwrap();
    writeln!(
        markdown,
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    )
    .unwrap();
    for row in &rows {
        writeln!(
            markdown,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} |",
            row.path,
            row.byte_count,
            row.parse_diagnostics,
            row.top_level_symbols,
            row.nested_symbols,
            row.max_depth,
            row.unknown_labels,
            row.range_failures
        )
        .unwrap();
    }
    writeln!(markdown).unwrap();
    writeln!(markdown, "# Symbol Trees").unwrap();
    writeln!(markdown).unwrap();
    markdown.push_str(&details);

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::write(&out_path, markdown)
        .map_err(|error| format!("Failed to write {}: {error}", out_path.display()))?;
    println!("Wrote {}", out_path.display());
    Ok(())
}

struct Args {
    fixture_root: Option<PathBuf>,
    out_path: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut fixture_root = None;
        let mut out_path = None;
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--fixtures" => {
                    fixture_root = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--fixtures requires a path".to_string())?,
                    ));
                }
                "--out" => {
                    out_path = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--out requires a path".to_string())?,
                    ));
                }
                "--help" | "-h" => {
                    println!("Usage: cargo run --example lsp_report -- [--fixtures <path>] [--out <path>]");
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        Ok(Self {
            fixture_root,
            out_path,
        })
    }
}

struct FileRow {
    path: String,
    byte_count: usize,
    parse_diagnostics: usize,
    top_level_symbols: usize,
    nested_symbols: usize,
    max_depth: usize,
    unknown_labels: usize,
    range_failures: usize,
}

fn repo_path(relative: &str) -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(relative)
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

fn render_symbol_tree(
    symbols: &[LspDocumentSymbol],
    depth: usize,
    rendered_lines: &mut usize,
    output: &mut String,
) {
    for symbol in symbols {
        if *rendered_lines >= MAX_SYMBOL_LINES_PER_FILE {
            return;
        }
        *rendered_lines += 1;
        let indent = "  ".repeat(depth);
        let detail = symbol
            .detail
            .as_ref()
            .map(|value| format!(" detail={}", value.replace('\n', "\\n")))
            .unwrap_or_default();
        writeln!(
            output,
            "{}- {} {}{} range={}:{}-{}:{} selection={}:{}-{}:{}",
            indent,
            kind_label(symbol.kind),
            symbol.name,
            detail,
            symbol.range.start.line + 1,
            symbol.range.start.character + 1,
            symbol.range.end.line + 1,
            symbol.range.end.character + 1,
            symbol.selection_range.start.line + 1,
            symbol.selection_range.start.character + 1,
            symbol.selection_range.end.line + 1,
            symbol.selection_range.end.character + 1
        )
        .unwrap();
        render_symbol_tree(&symbol.children, depth + 1, rendered_lines, output);
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

fn relative_display(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}
