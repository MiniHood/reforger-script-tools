use reforger_language_server::lsp::{semantic_tokens_report_for_source, LspSemanticTokenReport};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

const DEFAULT_OUTPUT: &str = "tools/reports/lsp-semantic-tokens-fixtures.report.md";

const INLINE_SOURCE: &str = r#"// Fixture truth: game-data-shaped semantic-token coverage source; not Workbench-confirmed in this repo.
[Attribute()]
modded class SCR_SemanticExample
{
	static const int COUNT = 4;
	ref array<string> m_Names;

	void Run(notnull IEntity entity, out int foundCount)
	{
		string name = "alpha";
		SCR_SemanticExample other;
		other.Run(entity, foundCount);
		m_Names.Insert(name);
	}
}

#ifdef DEBUG
#endif
"#;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let out_path = args.out_path.unwrap_or_else(|| repo_path(DEFAULT_OUTPUT));
    let report = semantic_tokens_report_for_source(INLINE_SOURCE);

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create report directory {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(&out_path, render_report(&report)).map_err(|error| {
        format!(
            "Failed to write semantic-token report {}: {error}",
            out_path.display()
        )
    })?;
    println!("Wrote {}", out_path.display());
    Ok(())
}

fn render_report(report: &LspSemanticTokenReport) -> String {
    let mut output = String::new();
    output.push_str("# LSP Semantic Tokens Fixture Report\n\n");
    output.push_str("This report decodes the Rust LSP semantic-token output used for Enforce coloring. TextMate scopes are intentionally not involved.\n\n");
    output.push_str("## Summary\n\n");
    output.push_str("| Metric | Count |\n");
    output.push_str("| --- | ---: |\n");
    let _ = writeln!(
        output,
        "| Parse diagnostics | {} |",
        report.parse_diagnostics
    );
    let _ = writeln!(
        output,
        "| Encoded integers | {} |",
        report.tokens.data.len()
    );
    let _ = writeln!(output, "| Semantic tokens | {} |", report.decoded.len());

    output.push_str("\n## Decoded Tokens\n\n");
    output.push_str("| Text | Range | Type | Modifiers |\n");
    output.push_str("| --- | --- | --- | --- |\n");
    for token in &report.decoded {
        let _ = writeln!(
            output,
            "| `{}` | `L{}:C{}-L{}:C{}` | `{}` | `{}` |",
            escape_table_text(&token.text),
            token.range.start.line,
            token.range.start.character,
            token.range.end.line,
            token.range.end.character,
            token.token_type,
            token.modifiers.join(", "),
        );
    }

    output
}

fn escape_table_text(value: &str) -> String {
    value
        .chars()
        .flat_map(char::escape_default)
        .collect::<String>()
        .replace('|', "\\|")
}

#[derive(Debug, Default)]
struct Args {
    out_path: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut args = env::args().skip(1);
        let mut parsed = Args::default();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--out" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--out requires a path".to_string())?;
                    parsed.out_path = Some(PathBuf::from(value));
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        Ok(parsed)
    }
}

fn print_help() {
    println!("Usage: cargo run --manifest-path server/Cargo.toml --example lsp_semantic_tokens_report -- [--out <path>]");
}

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("server manifest should have a repository parent")
        .join(relative)
}
