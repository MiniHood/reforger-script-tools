use reforger_language_server::index::{GlobalSymbolId, SymbolIndex};
use reforger_language_server::lexer::TextSpan;
use reforger_language_server::model::{SourceFileMetadata, SymbolKind};
use reforger_language_server::parser::parse_source;
use reforger_language_server::reference_finder::find_file_local_references;
use reforger_language_server::scope::LexicalScopeModel;
use reforger_language_server::semantic_file::SemanticFile;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_OUTPUT: &str = "tools/reports/reference-finder-fixtures.report.md";

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let source = fixture_source();
    let parse = parse_source(source);
    let semantic_file = SemanticFile::build(source, &parse);
    let index = SymbolIndex::from_semantic_files([(&semantic_file, SourceFileMetadata::unknown())]);
    let scope = LexicalScopeModel::from_parse_and_index(&parse, &index);

    let cases = [
        Case {
            label: "parameter value",
            name: "value",
            kind: SymbolKind::Parameter,
            owner: Some("Run"),
        },
        Case {
            label: "local result",
            name: "result",
            kind: SymbolKind::LocalVariable,
            owner: Some("Run"),
        },
        Case {
            label: "field m_Value",
            name: "m_Value",
            kind: SymbolKind::Field,
            owner: Some("Example"),
        },
        Case {
            label: "method Run",
            name: "Run",
            kind: SymbolKind::Method,
            owner: Some("Example"),
        },
        Case {
            label: "typedef ExampleId",
            name: "ExampleId",
            kind: SymbolKind::Typedef,
            owner: None,
        },
        Case {
            label: "global field s_Global",
            name: "s_Global",
            kind: SymbolKind::GlobalField,
            owner: None,
        },
        Case {
            label: "enum member Ready",
            name: "Ready",
            kind: SymbolKind::EnumMember,
            owner: Some("ExampleState"),
        },
    ];

    let mut report = String::new();
    writeln!(report, "# Reference Finder Fixture Report\n").unwrap();
    writeln!(report, "This report exercises file-local reference search through the resolver. It is review tooling only and does not perform workspace-wide search or rename edits.\n").unwrap();
    writeln!(report, "| Metric | Count |").unwrap();
    writeln!(report, "| --- | ---: |").unwrap();
    writeln!(
        report,
        "| Parse diagnostics | {} |",
        parse.diagnostics.len()
    )
    .unwrap();
    writeln!(report, "| Indexed symbols | {} |", index.symbols().len()).unwrap();

    report.push_str("\n## Reference Cases\n\n");
    report.push_str("| Case | Target | References | Declaration refs | Usage refs | Identifier tokens scanned |\n");
    report.push_str("| --- | --- | ---: | ---: | ---: | ---: |\n");
    let mut case_rows = Vec::new();
    for case in cases {
        let target = find_target(&index, case)
            .ok_or_else(|| format!("Missing target {} {:?}", case.name, case.kind))?;
        let result = find_file_local_references(source, &index, &parse, &scope, target);
        let declaration_refs = result
            .references
            .iter()
            .filter(|reference| reference.is_declaration)
            .count();
        let usage_refs = result.references.len().saturating_sub(declaration_refs);
        writeln!(
            report,
            "| {} | `{}` `{}` | {} | {} | {} | {} |",
            case.label,
            symbol_kind_label(case.kind),
            case.name,
            result.references.len(),
            declaration_refs,
            usage_refs,
            result.identifiers_scanned,
        )
        .unwrap();
        case_rows.push((case, result.references));
    }

    for (case, references) in case_rows {
        writeln!(report, "\n## {}\n", case.label).unwrap();
        if references.is_empty() {
            report.push_str("None.\n");
            continue;
        }
        report.push_str("| Line | Token | Declaration | Reason | Candidates | Source |\n");
        report.push_str("| ---: | --- | --- | --- | ---: | --- |\n");
        for reference in references {
            let (line, character) = line_character(source, reference.span);
            writeln!(
                report,
                "| {}:{} | `{}` | {} | `{}` | {} | `{}` |",
                line,
                character,
                escape_table(&reference.token_text),
                if reference.is_declaration {
                    "yes"
                } else {
                    "no"
                },
                reference.reason.as_str(),
                reference.candidate_count,
                escape_table(&line_text_at_offset(source, reference.span.start)),
            )
            .unwrap();
        }
    }

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
    out_path: PathBuf,
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
                    print_help();
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        Ok(Self {
            out_path: resolve_repo_path(out_path, DEFAULT_OUTPUT),
        })
    }
}

#[derive(Clone, Copy)]
struct Case {
    label: &'static str,
    name: &'static str,
    kind: SymbolKind,
    owner: Option<&'static str>,
}

fn find_target(index: &SymbolIndex, case: Case) -> Option<GlobalSymbolId> {
    index
        .symbols()
        .iter()
        .find(|symbol| {
            symbol.name.as_deref() == Some(case.name)
                && symbol.kind == case.kind
                && owner_name(index, symbol.parent).as_deref() == case.owner
        })
        .map(|symbol| symbol.id)
}

fn owner_name(index: &SymbolIndex, parent: Option<GlobalSymbolId>) -> Option<String> {
    let parent = parent?;
    index.symbol(parent)?.name.clone()
}

fn fixture_source() -> &'static str {
    r#"
typedef int ExampleId;
ExampleId s_Global;

enum ExampleState
{
    Ready = 1,
    Done = 2
}

class Example
{
    protected int m_Value;

    int Run(ExampleId value)
    {
        int result = value + m_Value + s_Global + ExampleState.Ready;
        this.m_Value = result;
        return result;
    }

    void Call()
    {
        int next = Run(s_Global);
    }
}
"#
}

fn line_character(source: &str, span: TextSpan) -> (usize, usize) {
    let mut line = 1usize;
    let mut line_start = 0usize;
    for (index, character) in source.char_indices() {
        if index >= span.start {
            break;
        }
        if character == '\n' {
            line += 1;
            line_start = index + 1;
        }
    }
    (line, span.start.saturating_sub(line_start))
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

fn symbol_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "Class",
        SymbolKind::Enum => "Enum",
        SymbolKind::Typedef => "Typedef",
        SymbolKind::Function => "Function",
        SymbolKind::GlobalField => "GlobalField",
        SymbolKind::Field => "Field",
        SymbolKind::Method => "Method",
        SymbolKind::Constructor => "Constructor",
        SymbolKind::Destructor => "Destructor",
        SymbolKind::Parameter => "Parameter",
        SymbolKind::EnumMember => "EnumMember",
        SymbolKind::TypeParameter => "TypeParameter",
        SymbolKind::LocalVariable => "LocalVariable",
        SymbolKind::PreprocessorMacro => "PreprocessorMacro",
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

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|")
}

fn print_help() {
    println!(
        "Usage: cargo run --manifest-path server/Cargo.toml --example reference_finder_report -- [--out <path>]"
    );
}
