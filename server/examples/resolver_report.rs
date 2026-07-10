use reforger_language_server::ast::AstSourceFile;
use reforger_language_server::index::{GlobalSymbolId, SymbolIndex};
use reforger_language_server::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, SymbolCatalog, SymbolKind,
    SOURCE_PRIORITY_FIXTURE, SOURCE_PRIORITY_GAME_DATA,
};
use reforger_language_server::parser::parse_source;
use reforger_language_server::resolver::{ReferenceCandidate, ReferenceResolver};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_OUTPUT: &str = "tools/reports/resolver-fixtures.report.md";

const INLINE_SOURCE: &str = r#"// Fixture truth: game-data-shaped resolver coverage source; not Workbench-confirmed in this repo.
Game g_Game;
typedef string FactionKey;
enum EExample
{
	One = 1
}

class Example
{
	int m_Value;
	void Run(int value)
	{
		int value = 4;
		value = value + 1;
		m_Value = value;
		ExternalType externalValue;
		foreach (int index, auto item : m_aItems)
		{
			Print(index);
			Print(item);
		}
		for (int i = 0; i < 1; i++)
		{
			Print(i);
		}
		FactionKey key;
		EExample flag;
		GlobalFn();
		g_Game = null;
		MissingThing();
	}
}

void GlobalFn();
"#;

const EXTERNAL_SOURCE: &str = r#"// Fixture truth: game-data-shaped external resolver source; not Workbench-confirmed in this repo.
class ExternalType {}
"#;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let out_path = args.out_path;
    let inline_index =
        index_for_source(INLINE_SOURCE, fixture_metadata("inline_resolver_shapes.c"));
    let inline_external =
        index_for_source(EXTERNAL_SOURCE, game_metadata("Game/ExternalType.c", None));
    let mut rows = Vec::new();
    let mut notes = Vec::new();

    if let Some(scripts_root) = args.scripts_root.or_else(default_scripts_root) {
        match load_real_game_data_rows(&scripts_root) {
            Ok(real_rows) if !real_rows.is_empty() => {
                notes.push(format!(
                    "Loaded real game-data checks from `{}`.",
                    scripts_root.display()
                ));
                rows.extend(real_rows);
            }
            Ok(_) => notes.push(format!(
                "No configured real game-data resolver checks were available under `{}`.",
                scripts_root.display()
            )),
            Err(error) => notes.push(format!(
                "Real game-data checks were skipped: {error}. Falling back to committed fixtures."
            )),
        }
    } else {
        notes.push("No downloaded game-data scripts folder was found; using committed fixture checks only.".to_string());
    }

    for check in checks() {
        let offset = offset_for_needle(INLINE_SOURCE, check.needle, check.cursor)?;
        let resolution =
            ReferenceResolver::new(INLINE_SOURCE, &inline_index, Some(&inline_external))
                .resolve_at_offset(offset);
        rows.push(ReportRow::new(
            "inline_resolver_shapes.c".to_string(),
            check.target.to_string(),
            check.needle.to_string(),
            INLINE_SOURCE,
            offset,
            &inline_index,
            Some(&inline_external),
            resolution,
        ));
    }

    let fixture_source = include_str!("../../tools/fixtures/parser/modded_game_mode_members.c");
    let fixture_index = index_for_source(
        fixture_source,
        fixture_metadata("tools/fixtures/parser/modded_game_mode_members.c"),
    );
    let offset = offset_for_needle(fixture_source, "out int foundCount", "foundCount")?;
    let resolution =
        ReferenceResolver::new(fixture_source, &fixture_index, None).resolve_at_offset(offset);
    rows.push(ReportRow::new(
        "tools/fixtures/parser/modded_game_mode_members.c".to_string(),
        "fixture parameter declaration".to_string(),
        "out int foundCount".to_string(),
        fixture_source,
        offset,
        &fixture_index,
        None,
        resolution,
    ));

    let markdown = render_report(&rows, &notes);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::write(&out_path, markdown)
        .map_err(|error| format!("Failed to write {}: {error}", out_path.display()))?;
    println!("Wrote {}", out_path.display());
    Ok(())
}

fn render_report(rows: &[ReportRow], notes: &[String]) -> String {
    let mut report = String::new();
    writeln!(report, "# Resolver Fixture Report").unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "This report exercises the first identifier-only reference resolver scaffold. It is review tooling only; it does not perform Workbench validation, full expression parsing, or LSP definition handling."
    )
    .unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "Use this report to review whether cursor identifiers select useful file-local or external symbols before wiring resolver output into hover or definition."
    )
    .unwrap();
    writeln!(report).unwrap();
    if !notes.is_empty() {
        writeln!(report, "## Inputs").unwrap();
        writeln!(report).unwrap();
        for note in notes {
            writeln!(report, "- {}", note).unwrap();
        }
        writeln!(report).unwrap();
    }
    writeln!(
        report,
        "| File | Target | Token | Position | Reason | Selected | Candidates |"
    )
    .unwrap();
    writeln!(report, "| --- | --- | --- | ---: | --- | --- | ---: |").unwrap();
    for row in rows {
        writeln!(
            report,
            "| `{}` | {} | `{}` | {}:{} | `{}` | {} | {} |",
            row.file,
            escape_cell(&row.target),
            escape_cell(&row.token),
            row.line,
            row.column,
            row.reason,
            escape_cell(&row.selected),
            row.candidate_count
        )
        .unwrap();
    }

    writeln!(report).unwrap();
    writeln!(report, "## Candidate Details").unwrap();
    writeln!(report).unwrap();
    for row in rows {
        writeln!(
            report,
            "### `{}` - {} at {}:{}",
            row.file,
            escape_inline(&row.target),
            row.line,
            row.column
        )
        .unwrap();
        writeln!(report).unwrap();
        writeln!(report, "- Needle: `{}`", escape_inline(&row.needle)).unwrap();
        writeln!(report, "- Token: `{}`", escape_inline(&row.token)).unwrap();
        writeln!(report, "- Reason: `{}`", row.reason).unwrap();
        writeln!(report).unwrap();
        writeln!(report, "```enforce").unwrap();
        writeln!(report, "{}", row.snippet).unwrap();
        writeln!(report, "```").unwrap();
        writeln!(report).unwrap();
        if row.candidates.is_empty() {
            writeln!(report, "No candidates.").unwrap();
            writeln!(report).unwrap();
            continue;
        }
        writeln!(
            report,
            "| Rank | Source | Reason | Kind | Name | Path | Position | Span |"
        )
        .unwrap();
        writeln!(
            report,
            "| ---: | --- | --- | --- | --- | --- | ---: | --- |"
        )
        .unwrap();
        for (index, candidate) in row.candidates.iter().enumerate() {
            writeln!(
                report,
                "| {} | `{}` | `{}` | `{}` | `{}` | `{}` | {} | `{}` |",
                index + 1,
                candidate.source,
                candidate.reason,
                candidate.kind,
                escape_cell(&candidate.name),
                escape_cell(&candidate.path),
                candidate.position,
                candidate.span
            )
            .unwrap();
        }
        writeln!(report).unwrap();
    }

    report
}

fn load_real_game_data_rows(scripts_root: &Path) -> Result<Vec<ReportRow>, String> {
    let real_sources = load_real_sources(scripts_root)?;
    if real_sources.is_empty() {
        return Ok(Vec::new());
    }

    let external_sources = load_external_sources(scripts_root)?;
    let external_index = if external_sources.is_empty() {
        None
    } else {
        Some(index_for_owned_sources(&external_sources))
    };

    let mut rows = Vec::new();
    for source_file in &real_sources {
        let file_index = index_for_source(&source_file.source, source_file.metadata.clone());
        for check in &source_file.checks {
            let offset = offset_for_needle(&source_file.source, check.needle, check.cursor)?;
            let resolution =
                ReferenceResolver::new(&source_file.source, &file_index, external_index.as_ref())
                    .resolve_at_offset(offset);
            rows.push(ReportRow::new(
                source_file.relative_path.clone(),
                check.target.to_string(),
                check.needle.to_string(),
                &source_file.source,
                offset,
                &file_index,
                external_index.as_ref(),
                resolution,
            ));
        }
    }
    Ok(rows)
}

fn load_real_sources(scripts_root: &Path) -> Result<Vec<SourceFile>, String> {
    let mut sources = Vec::new();
    push_real_source(
        &mut sources,
        scripts_root,
        "Game/GameMode/SCR_BaseGameMode.c",
        vec![
            Check {
                target: "real class declaration",
                needle: "class SCR_BaseGameMode : BaseGameMode",
                cursor: "SCR_BaseGameMode",
            },
            Check {
                target: "real external base type",
                needle: "SCR_BaseGameMode : BaseGameMode",
                cursor: " BaseGameMode",
            },
            Check {
                target: "real method declaration",
                needle: "protected override void OnGameStart()",
                cursor: "OnGameStart",
            },
            Check {
                target: "real field use",
                needle: "Event_OnGameStart.Invoke();",
                cursor: "Event_OnGameStart",
            },
            Check {
                target: "real foreach variable declaration",
                needle:
                    "foreach (SCR_BaseGameModeComponent comp : m_aAdditionalGamemodeComponents)",
                cursor: "comp",
            },
            Check {
                target: "real foreach variable use",
                needle: "comp.OnGameEnd();",
                cursor: "comp",
            },
            Check {
                target: "real member collection use",
                needle:
                    "foreach (SCR_BaseGameModeComponent comp : m_aAdditionalGamemodeComponents)",
                cursor: "m_aAdditionalGamemodeComponents",
            },
        ],
    )?;
    push_real_source(
        &mut sources,
        scripts_root,
        "Game/Sandbox/Resources/SCR_ResourceComponent.c",
        vec![
            Check {
                target: "real method declaration",
                needle: "void SetResourceTypeEnabled(bool enable, EResourceType resourceType = EResourceType.SUPPLIES)",
                cursor: "SetResourceTypeEnabled",
            },
            Check {
                target: "real parameter declaration",
                needle: "bool enable, EResourceType resourceType",
                cursor: "enable",
            },
            Check {
                target: "real external enum type",
                needle: "void SetResourceTypeEnabled(bool enable, EResourceType resourceType",
                cursor: "EResourceType",
            },
            Check {
                target: "real parameter use",
                needle: "m_aDisabledResourceTypes.Find(resourceType)",
                cursor: "resourceType",
            },
            Check {
                target: "real local declaration",
                needle: "int index = m_aDisabledResourceTypes.Find(resourceType);",
                cursor: "index",
            },
            Check {
                target: "real local use",
                needle: "m_aDisabledResourceTypes.Remove(index);",
                cursor: "index",
            },
            Check {
                target: "real field use",
                needle: "m_aDisabledResourceTypes.Find(resourceType)",
                cursor: "m_aDisabledResourceTypes",
            },
        ],
    )?;
    Ok(sources)
}

fn push_real_source(
    sources: &mut Vec<SourceFile>,
    scripts_root: &Path,
    relative_path: &str,
    checks: Vec<Check>,
) -> Result<(), String> {
    let path = scripts_root.join(relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));
    if !path.exists() {
        return Ok(());
    }
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    sources.push(SourceFile {
        relative_path: relative_path.to_string(),
        metadata: game_metadata(relative_path, Some(scripts_root)),
        source,
        checks,
    });
    Ok(())
}

fn load_external_sources(scripts_root: &Path) -> Result<Vec<OwnedSource>, String> {
    let mut sources = Vec::new();
    for relative_path in [
        "Game/generated/GameMode/BaseGameMode.c",
        "Game/Sandbox/Resources/Container/SCR_ResourceContainer.c",
    ] {
        let path = scripts_root.join(relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !path.exists() {
            continue;
        }
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
        sources.push(OwnedSource {
            metadata: game_metadata(relative_path, Some(scripts_root)),
            source,
        });
    }
    Ok(sources)
}

fn index_for_owned_sources(sources: &[OwnedSource]) -> SymbolIndex {
    let catalogs = sources
        .iter()
        .map(|source| {
            let parse = parse_source(&source.source);
            let ast = AstSourceFile::new(&source.source, &parse);
            SymbolCatalog::from_ast_with_metadata(&source.source, &ast, source.metadata.clone())
        })
        .collect::<Vec<_>>();
    SymbolIndex::from_catalogs(catalogs.iter())
}

struct Check {
    target: &'static str,
    needle: &'static str,
    cursor: &'static str,
}

struct SourceFile {
    relative_path: String,
    metadata: SourceFileMetadata,
    source: String,
    checks: Vec<Check>,
}

struct OwnedSource {
    metadata: SourceFileMetadata,
    source: String,
}

fn checks() -> Vec<Check> {
    vec![
        Check {
            target: "class declaration",
            needle: "class Example",
            cursor: "Example",
        },
        Check {
            target: "local variable use",
            needle: "value = value + 1",
            cursor: "value",
        },
        Check {
            target: "class field use",
            needle: "m_Value = value",
            cursor: "m_Value",
        },
        Check {
            target: "external type",
            needle: "ExternalType externalValue",
            cursor: "ExternalType",
        },
        Check {
            target: "foreach index variable",
            needle: "Print(index)",
            cursor: "index",
        },
        Check {
            target: "foreach item variable",
            needle: "Print(item)",
            cursor: "item",
        },
        Check {
            target: "for initializer variable",
            needle: "Print(i)",
            cursor: "i)",
        },
        Check {
            target: "typedef reference",
            needle: "FactionKey key",
            cursor: "FactionKey",
        },
        Check {
            target: "enum reference",
            needle: "EExample flag",
            cursor: "EExample",
        },
        Check {
            target: "global function reference",
            needle: "GlobalFn();",
            cursor: "GlobalFn",
        },
        Check {
            target: "global field reference",
            needle: "g_Game = null",
            cursor: "g_Game",
        },
        Check {
            target: "unresolved identifier",
            needle: "MissingThing();",
            cursor: "MissingThing",
        },
    ]
}

struct ReportRow {
    file: String,
    target: String,
    needle: String,
    token: String,
    line: usize,
    column: usize,
    snippet: String,
    reason: String,
    selected: String,
    candidate_count: usize,
    candidates: Vec<CandidateRow>,
}

impl ReportRow {
    fn new(
        file: String,
        target: String,
        needle: String,
        source: &str,
        offset: usize,
        file_index: &SymbolIndex,
        external_index: Option<&SymbolIndex>,
        resolution: Option<reforger_language_server::resolver::ReferenceResolution>,
    ) -> Self {
        let (line, column) = line_column(source, offset);
        let snippet = source_snippet(source, offset);
        let Some(resolution) = resolution else {
            return Self {
                file,
                target,
                needle,
                token: "<none>".to_string(),
                line,
                column,
                snippet,
                reason: "no-token".to_string(),
                selected: "<none>".to_string(),
                candidate_count: 0,
                candidates: Vec::new(),
            };
        };

        let selected = resolution
            .selected
            .as_ref()
            .map(|candidate| display_candidate(file_index, external_index, source, candidate))
            .unwrap_or_else(|| "<none>".to_string());
        let candidates = resolution
            .candidates
            .iter()
            .map(|candidate| CandidateRow::new(file_index, external_index, source, candidate))
            .collect::<Vec<_>>();
        Self {
            file,
            target,
            needle,
            token: resolution.token_text,
            line,
            column,
            snippet,
            reason: resolution.reason.as_str().to_string(),
            selected,
            candidate_count: candidates.len(),
            candidates,
        }
    }
}

struct CandidateRow {
    source: String,
    reason: String,
    kind: String,
    name: String,
    path: String,
    position: String,
    span: String,
}

impl CandidateRow {
    fn new(
        file_index: &SymbolIndex,
        external_index: Option<&SymbolIndex>,
        source: &str,
        candidate: &ReferenceCandidate,
    ) -> Self {
        let index = candidate_index(file_index, external_index, candidate);
        let path = index
            .and_then(|index| index.file(candidate.id.file_id))
            .and_then(|file| {
                file.metadata
                    .relative_path
                    .as_ref()
                    .or(file.metadata.absolute_path.as_ref())
            })
            .map(|path| path.display().to_string().replace('\\', "/"))
            .unwrap_or_else(|| "<unknown>".to_string());
        let position = match candidate.source {
            reforger_language_server::resolver::CandidateSource::FileLocal => {
                let (line, column) = line_column(source, candidate.selection_span.start);
                format!("{line}:{column}")
            }
            reforger_language_server::resolver::CandidateSource::External => "-".to_string(),
        };

        Self {
            source: candidate.source.as_str().to_string(),
            reason: candidate.reason.as_str().to_string(),
            kind: kind_label(candidate.kind).to_string(),
            name: candidate
                .name
                .clone()
                .unwrap_or_else(|| "<unknown>".to_string()),
            path,
            position,
            span: format!(
                "{}..{}",
                candidate.selection_span.start, candidate.selection_span.end
            ),
        }
    }
}

fn display_candidate(
    file_index: &SymbolIndex,
    external_index: Option<&SymbolIndex>,
    source: &str,
    candidate: &ReferenceCandidate,
) -> String {
    let row = CandidateRow::new(file_index, external_index, source, candidate);
    format!("{} `{}` via {}", row.kind, row.name, row.reason)
}

fn candidate_index<'a>(
    file_index: &'a SymbolIndex,
    external_index: Option<&'a SymbolIndex>,
    candidate: &ReferenceCandidate,
) -> Option<&'a SymbolIndex> {
    match candidate.source {
        reforger_language_server::resolver::CandidateSource::FileLocal => Some(file_index),
        reforger_language_server::resolver::CandidateSource::External => external_index,
    }
}

struct Args {
    out_path: PathBuf,
    scripts_root: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut out_path = None;
    let mut scripts_root = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                out_path = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--out requires a path".to_string())?,
                ));
            }
            "--scripts" => {
                scripts_root = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--scripts requires a path".to_string())?,
                ));
            }
            "--help" | "-h" => {
                println!("Usage: cargo run --manifest-path server/Cargo.toml --example resolver_report -- [--scripts <path>] [--out <path>]");
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }
    Ok(Args {
        out_path: out_path.unwrap_or_else(|| repo_root().join(DEFAULT_OUTPUT)),
        scripts_root,
    })
}

fn index_for_source(source: &str, metadata: SourceFileMetadata) -> SymbolIndex {
    let parse = parse_source(source);
    let ast = AstSourceFile::new(source, &parse);
    let catalog = SymbolCatalog::from_ast_with_metadata(source, &ast, metadata);
    SymbolIndex::from_catalogs([&catalog])
}

fn fixture_metadata(path: &str) -> SourceFileMetadata {
    let relative_path = PathBuf::from(path);
    SourceFileMetadata {
        kind: SourceKind::Fixture,
        category: source_category_for_path(SourceKind::Fixture, Some(&relative_path)),
        absolute_path: Some(repo_root().join(path)),
        root_path: Some(repo_root()),
        relative_path: Some(relative_path),
        priority: SOURCE_PRIORITY_FIXTURE,
    }
}

fn game_metadata(path: &str, scripts_root: Option<&Path>) -> SourceFileMetadata {
    let relative_path = PathBuf::from(path);
    let root_path = scripts_root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("C:/game"));
    SourceFileMetadata {
        kind: SourceKind::GameData,
        category: source_category_for_path(SourceKind::GameData, Some(&relative_path)),
        absolute_path: Some(root_path.join(&relative_path)),
        root_path: Some(root_path),
        relative_path: Some(relative_path),
        priority: SOURCE_PRIORITY_GAME_DATA,
    }
}

fn default_scripts_root() -> Option<PathBuf> {
    let app_data = env::var_os("APPDATA")?;
    let path = PathBuf::from(app_data)
        .join("Code")
        .join("User")
        .join("globalStorage")
        .join("undefined_publisher.reforger-sript-tools")
        .join("game-data")
        .join("scripts");
    path.exists().then_some(path)
}

fn offset_for_needle(source: &str, needle: &str, cursor: &str) -> Result<usize, String> {
    let start = source
        .find(needle)
        .ok_or_else(|| format!("Missing needle `{needle}`"))?;
    let cursor_start = needle
        .find(cursor)
        .ok_or_else(|| format!("Missing cursor `{cursor}` in `{needle}`"))?;
    let leading_whitespace = cursor.len() - cursor.trim_start().len();
    Ok(start + cursor_start + leading_whitespace)
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
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

fn source_snippet(source: &str, offset: usize) -> String {
    let (target_line, target_column) = line_column(source, offset);
    let lines = source.lines().collect::<Vec<_>>();
    let start = target_line.saturating_sub(2).max(1);
    let end = (target_line + 1).min(lines.len());
    let mut snippet = String::new();
    for line_number in start..=end {
        let marker = if line_number == target_line { ">" } else { " " };
        let text = lines.get(line_number - 1).copied().unwrap_or_default();
        writeln!(
            snippet,
            "{marker} {:>5} | {}",
            line_number,
            text.replace('\t', "    ")
        )
        .unwrap();
        if line_number == target_line {
            writeln!(
                snippet,
                "        | {}^",
                " ".repeat(target_column.saturating_sub(1))
            )
            .unwrap();
        }
    }
    snippet.trim_end().to_string()
}

fn kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "Class",
        SymbolKind::Enum => "Enum",
        SymbolKind::EnumMember => "EnumMember",
        SymbolKind::Typedef => "Typedef",
        SymbolKind::Function => "Function",
        SymbolKind::GlobalField => "GlobalField",
        SymbolKind::Field => "Field",
        SymbolKind::Method => "Method",
        SymbolKind::Constructor => "Constructor",
        SymbolKind::Destructor => "Destructor",
        SymbolKind::Parameter => "Parameter",
        SymbolKind::LocalVariable => "LocalVariable",
    }
}

fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn escape_inline(value: &str) -> String {
    value.replace('`', "\\`")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("server crate should be inside the repository root")
        .to_path_buf()
}

#[allow(dead_code)]
fn _display_id(id: GlobalSymbolId) -> String {
    format!("{}:{}", id.file_id.0, id.symbol_id.0)
}
