use reforger_language_server::lsp::{parser_diagnostics_for_source, LspDiagnostic};
use reforger_language_server::parser::parse_source;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

const DEFAULT_OUTPUT: &str = "tools/reports/lsp-diagnostics-fixtures.report.md";

struct DiagnosticCase {
    name: &'static str,
    description: &'static str,
    source: &'static str,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let out_path = args.out_path.unwrap_or_else(|| repo_path(DEFAULT_OUTPUT));
    let cases = diagnostic_cases();

    let mut rows = Vec::new();
    for case in cases {
        let parse = parse_source(case.source);
        let diagnostics = parser_diagnostics_for_source(case.source, &parse.diagnostics);
        rows.push(DiagnosticRow {
            name: case.name,
            description: case.description,
            source: case.source,
            diagnostics,
        });
    }

    let mut markdown = String::new();
    writeln!(markdown, "# LSP Diagnostics Fixture Report").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "- Cases: {}", rows.len()).unwrap();
    writeln!(
        markdown,
        "- Diagnostics: {}",
        rows.iter().map(|row| row.diagnostics.len()).sum::<usize>()
    )
    .unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "This report exercises the parser-diagnostic projection used by `textDocument/publishDiagnostics`. Diagnostics are extension parser facts only; Workbench remains compiler truth.").unwrap();
    writeln!(markdown).unwrap();

    writeln!(
        markdown,
        "| Case | Description | Diagnostics | First message | First range |"
    )
    .unwrap();
    writeln!(markdown, "| --- | --- | ---: | --- | --- |").unwrap();
    for row in &rows {
        let first = row.diagnostics.first();
        writeln!(
            markdown,
            "| `{}` | {} | {} | {} | {} |",
            row.name,
            escape_markdown_cell(row.description),
            row.diagnostics.len(),
            first
                .map(|diagnostic| escape_markdown_cell(&diagnostic.message))
                .unwrap_or_else(|| "None.".to_string()),
            first
                .map(format_diagnostic_range)
                .unwrap_or_else(|| "<none>".to_string())
        )
        .unwrap();
    }

    writeln!(markdown).unwrap();
    writeln!(markdown, "## Diagnostic Details").unwrap();
    for row in &rows {
        writeln!(markdown).unwrap();
        writeln!(markdown, "### {}", row.name).unwrap();
        writeln!(markdown).unwrap();
        writeln!(markdown, "{}", row.description).unwrap();
        writeln!(markdown).unwrap();
        if row.diagnostics.is_empty() {
            writeln!(markdown, "No diagnostics.").unwrap();
            continue;
        }
        for (index, diagnostic) in row.diagnostics.iter().enumerate() {
            writeln!(
                markdown,
                "{}. `{}` `{}` severity {} at {}",
                index + 1,
                diagnostic.source,
                diagnostic.code,
                diagnostic.severity,
                format_diagnostic_range(diagnostic)
            )
            .unwrap();
            writeln!(
                markdown,
                "   Message: {}",
                escape_markdown_cell(&diagnostic.message)
            )
            .unwrap();
            writeln!(markdown).unwrap();
            writeln!(markdown, "~~~enforce").unwrap();
            writeln!(markdown, "{}", snippet_for_range(row.source, diagnostic)).unwrap();
            writeln!(markdown, "~~~").unwrap();
        }
    }

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::write(&out_path, markdown)
        .map_err(|error| format!("Failed to write {}: {error}", out_path.display()))?;
    println!("Wrote {}", out_path.display());
    Ok(())
}

fn diagnostic_cases() -> Vec<DiagnosticCase> {
    vec![
        DiagnosticCase {
            name: "valid_declaration",
            description: "Valid declaration source should publish no parser diagnostics.",
            source: "class Valid\n{\n\tvoid Run();\n}\n",
        },
        DiagnosticCase {
            name: "missing_parameter_close",
            description: "Malformed method parameter list should mark the parser recovery point.",
            source: "class Broken\n{\n\tvoid Run(\n}\n",
        },
        DiagnosticCase {
            name: "unterminated_string",
            description: "Lexer error tokens should flow through the parser diagnostic channel.",
            source: "class Broken\n{\n\tstring value = \"missing close;\n}\n",
        },
        DiagnosticCase {
            name: "malformed_body_statement",
            description: "Malformed body expressions should produce bounded parser diagnostics.",
            source:
                "class Broken\n{\n\tvoid Run()\n\t{\n\t\tif (true\n\t\t\tPrint(\"x\");\n\t}\n}\n",
        },
    ]
}

struct DiagnosticRow {
    name: &'static str,
    description: &'static str,
    source: &'static str,
    diagnostics: Vec<LspDiagnostic>,
}

struct Args {
    out_path: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut out_path = None;
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--out" => {
                    out_path = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--out requires a path".to_string())?,
                    ));
                }
                "--help" | "-h" => {
                    println!("Usage: cargo run --example lsp_diagnostics_report -- [--out <path>]");
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        Ok(Self { out_path })
    }
}

fn snippet_for_range(source: &str, diagnostic: &LspDiagnostic) -> String {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return "<empty source>".to_string();
    }
    let target_line = diagnostic.range.start.line as usize;
    let start = target_line.saturating_sub(1);
    let end = (target_line + 2).min(lines.len());
    let mut snippet = String::new();
    for line_index in start..end {
        let marker = if line_index == target_line { ">" } else { " " };
        let line = lines[line_index].replace('\t', "\\t");
        writeln!(snippet, "{marker} {:>4} | {}", line_index + 1, line).unwrap();
        if line_index == target_line {
            let start_col = diagnostic.range.start.character as usize;
            let end_col = diagnostic.range.end.character as usize;
            let caret_count = end_col.saturating_sub(start_col).max(1);
            writeln!(
                snippet,
                "       | {}{}",
                " ".repeat(start_col),
                "^".repeat(caret_count)
            )
            .unwrap();
        }
    }
    snippet.trim_end().to_string()
}

fn format_diagnostic_range(diagnostic: &LspDiagnostic) -> String {
    format!(
        "{}:{}-{}:{}",
        diagnostic.range.start.line + 1,
        diagnostic.range.start.character + 1,
        diagnostic.range.end.line + 1,
        diagnostic.range.end.character + 1
    )
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn repo_path(relative: &str) -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(relative)
}
