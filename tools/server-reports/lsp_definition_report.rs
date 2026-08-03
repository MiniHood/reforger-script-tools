use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::lsp::{
    definition_report_for_source_position_with_external, position_for_offset, symbol_kind_label,
    LspDefinitionReport, LspPosition,
};
use reforger_language_server::model::{SourceKind, SOURCE_PRIORITY_GAME_DATA};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const DEFAULT_FIXTURE_ROOT: &str = "tools/fixtures/parser";
const DEFAULT_OUTPUT: &str = "tools/reports/lsp-definition-fixtures.report.md";

const INLINE_DEFINITION_SOURCE: &str = r#"// Fixture truth: game-data-shaped LSP definition coverage source; not Workbench-confirmed in this repo.
Game g_Game;
typedef string FactionKey;

[EnumBitFlag()]
enum ExampleFlags
{
	None = 0,
	Enabled = 1
}

class Entity
{
	vector GetOrigin();
}

class Example
{
	protected int m_Value;

	void Run(string name, Entity ent)
	{
		int index = 4;
		index = index + 1;
		Print(name);
		m_Value = index;
		ent.GetOrigin();
		FactionKey key;
		ExampleFlags flag = ExampleFlags.Enabled;
		g_Game = null;
		MissingThing();
		PrintFormat("Value: %1", index, level: LogLevel.DEBUG);
	}
}
"#;

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
    let scripts_root = args.scripts_root.or_else(default_scripts_root);

    if !fixture_root.is_dir() {
        return Err(format!(
            "Fixture root does not exist: {}",
            fixture_root.display()
        ));
    }

    let mut checks = definition_checks(&fixture_root);
    if let Some(scripts_root) = scripts_root.as_ref() {
        checks.extend(game_data_definition_checks(scripts_root));
    }
    let external_index = scripts_root
        .as_ref()
        .filter(|scripts_root| scripts_root.is_dir())
        .map(|scripts_root| {
            build_index(&IndexBuildConfig {
                roots: vec![IndexSourceRoot::new(
                    scripts_root,
                    SourceKind::GameData,
                    SOURCE_PRIORITY_GAME_DATA,
                )],
            })
            .map(|result| result.index)
        })
        .transpose()?;

    for (target, needle, cursor) in [
        ("class declaration", "class Example", "Example"),
        ("method declaration", "void Run(string", "Run"),
        ("field use", "m_Value = index", "m_Value"),
        ("parameter use", "Print(name)", "name"),
        ("local use", "index + 1", "index"),
        ("typedef use", "FactionKey key", "FactionKey"),
        ("enum member use", "ExampleFlags.Enabled", "Enabled"),
        ("global field use", "g_Game = null", "g_Game"),
        ("receiver member use", "ent.GetOrigin", "GetOrigin"),
        ("named argument label miss", "level: LogLevel", "level"),
        ("unresolved miss", "MissingThing();", "MissingThing"),
    ] {
        checks.push(DefinitionCheck::inline(
            "inline_definition_shapes.c",
            INLINE_DEFINITION_SOURCE,
            target,
            needle,
            cursor,
        ));
    }

    let mut rows = Vec::new();
    for check in checks {
        let source = check.source()?;
        let uri = check.uri();
        let position = position_for_needle(&source, check.needle, check.cursor)?;
        let start = Instant::now();
        let report = definition_report_for_source_position_with_external(
            &source,
            &uri,
            position,
            external_index.as_ref(),
        );
        let elapsed_ms = start.elapsed().as_millis();

        rows.push(DefinitionRow::from_report(
            check.file_label(),
            check.target.to_string(),
            position,
            report,
            elapsed_ms,
        ));
    }

    let mut markdown = String::new();
    writeln!(markdown, "# LSP Definition Fixture Report").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "- Fixture root: `{}`", fixture_root.display()).unwrap();
    writeln!(
        markdown,
        "- Game-data scripts: `{}`",
        scripts_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<not found>".to_string())
    )
    .unwrap();
    writeln!(markdown, "- Checks: {}", rows.len()).unwrap();
    writeln!(
        markdown,
        "- External index: `{}`",
        external_index
            .as_ref()
            .map(|index| format!("ready, {} symbols", index.symbols().len()))
            .unwrap_or_else(|| "unavailable".to_string())
    )
    .unwrap();
    writeln!(
        markdown,
        "- Hits: {}",
        rows.iter().filter(|row| row.hit).count()
    )
    .unwrap();
    writeln!(
        markdown,
        "- Misses: {}",
        rows.iter().filter(|row| !row.hit).count()
    )
    .unwrap();
    writeln!(
        markdown,
        "- Parse diagnostics: {}",
        rows.iter().map(|row| row.parse_diagnostics).sum::<usize>()
    )
    .unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "This report exercises the same resolver-first definition path used by `textDocument/definition`, with game-data supplied as optional external index context. It is review tooling only; it does not perform semantic lookup or Workbench validation.").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "| File | Target | Position | Hit | Selected source | Resolver reason | Identifier context | Candidates | Selected | Target URI | Target range | Parse diagnostics | Elapsed ms |").unwrap();
    writeln!(
        markdown,
        "| --- | --- | ---: | --- | --- | --- | --- | ---: | --- | --- | --- | ---: | ---: |"
    )
    .unwrap();
    for row in &rows {
        writeln!(
            markdown,
            "| `{}` | {} | {}:{} | {} | `{}` | `{}` | `{}` | {} | {} `{}` | `{}` | `{}` | {} | {} |",
            row.file,
            escape_markdown_cell(&row.target),
            row.line,
            row.column,
            if row.hit { "yes" } else { "no" },
            row.selected_source,
            row.resolver_reason,
            row.identifier_context,
            row.resolver_candidate_count,
            row.selected_kind,
            escape_markdown_cell(&row.selected_label),
            escape_markdown_cell(&row.target_uri),
            row.target_range,
            row.parse_diagnostics,
            row.elapsed_ms
        )
        .unwrap();
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

struct Args {
    fixture_root: Option<PathBuf>,
    scripts_root: Option<PathBuf>,
    out_path: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut fixture_root = None;
        let mut scripts_root = None;
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
                "--scripts" => {
                    scripts_root = Some(PathBuf::from(
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
                "--help" | "-h" => {
                    println!("Usage: cargo run --example lsp_definition_report -- [--fixtures <path>] [--scripts <path>] [--out <path>]");
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        Ok(Self {
            fixture_root,
            scripts_root,
            out_path,
        })
    }
}

struct DefinitionCheck<'source> {
    file: DefinitionFile<'source>,
    target: &'static str,
    needle: &'static str,
    cursor: &'static str,
}

enum DefinitionFile<'source> {
    Fixture(PathBuf),
    Inline {
        label: &'static str,
        source: &'source str,
    },
}

impl<'source> DefinitionCheck<'source> {
    fn fixture(
        fixture_root: &Path,
        file: &'static str,
        target: &'static str,
        needle: &'static str,
        cursor: &'static str,
    ) -> Self {
        Self {
            file: DefinitionFile::Fixture(fixture_root.join(file)),
            target,
            needle,
            cursor,
        }
    }

    fn path(
        path: PathBuf,
        target: &'static str,
        needle: &'static str,
        cursor: &'static str,
    ) -> Self {
        Self {
            file: DefinitionFile::Fixture(path),
            target,
            needle,
            cursor,
        }
    }

    fn inline(
        label: &'static str,
        source: &'source str,
        target: &'static str,
        needle: &'static str,
        cursor: &'static str,
    ) -> Self {
        Self {
            file: DefinitionFile::Inline { label, source },
            target,
            needle,
            cursor,
        }
    }

    fn source(&self) -> Result<String, String> {
        match &self.file {
            DefinitionFile::Fixture(path) => fs::read_to_string(path)
                .map_err(|error| format!("Failed to read {}: {error}", path.display())),
            DefinitionFile::Inline { source, .. } => Ok((*source).to_string()),
        }
    }

    fn file_label(&self) -> String {
        match &self.file {
            DefinitionFile::Fixture(path) => path.display().to_string().replace('\\', "/"),
            DefinitionFile::Inline { label, .. } => format!("<inline>/{label}"),
        }
    }

    fn uri(&self) -> String {
        match &self.file {
            DefinitionFile::Fixture(path) => {
                format!("file:///{}", path.display().to_string().replace('\\', "/"))
            }
            DefinitionFile::Inline { label, .. } => format!("file:///{label}"),
        }
    }
}

struct DefinitionRow {
    file: String,
    target: String,
    line: u32,
    column: u32,
    hit: bool,
    selected_kind: String,
    selected_label: String,
    selected_source: String,
    resolver_reason: String,
    identifier_context: String,
    resolver_candidate_count: usize,
    target_uri: String,
    target_range: String,
    parse_diagnostics: usize,
    elapsed_ms: u128,
}

impl DefinitionRow {
    fn from_report(
        file: String,
        target: String,
        position: LspPosition,
        report: LspDefinitionReport,
        elapsed_ms: u128,
    ) -> Self {
        let location = report.locations.first();
        Self {
            file,
            target,
            line: position.line + 1,
            column: position.character + 1,
            hit: report.is_hit(),
            selected_kind: report
                .selected_kind
                .map(symbol_kind_label)
                .unwrap_or("None")
                .to_string(),
            selected_label: report
                .selected_label
                .unwrap_or_else(|| "<none>".to_string()),
            selected_source: report
                .selected_source
                .map(|source| source.as_str().to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            resolver_reason: report
                .resolver_reason
                .map(|reason| reason.as_str().to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            identifier_context: report
                .identifier_context
                .map(|context| context.as_str().to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            resolver_candidate_count: report.resolver_candidate_count,
            target_uri: location
                .map(|location| location.uri.clone())
                .unwrap_or_else(|| "<none>".to_string()),
            target_range: location
                .map(|location| format_range(location.range))
                .unwrap_or_else(|| "<none>".to_string()),
            parse_diagnostics: report.parse_diagnostics,
            elapsed_ms,
        }
    }
}

fn definition_checks(fixture_root: &Path) -> Vec<DefinitionCheck<'static>> {
    vec![
        DefinitionCheck::fixture(
            fixture_root,
            "core_types_declarations.c",
            "class declaration",
            "class array<Class T>",
            "array",
        ),
        DefinitionCheck::fixture(
            fixture_root,
            "modded_game_mode_members.c",
            "field declaration",
            "m_bAllowFactionChange",
            "m_bAllowFactionChange",
        ),
        DefinitionCheck::fixture(
            fixture_root,
            "modded_game_mode_members.c",
            "method declaration",
            "GetComponentsByType(typename",
            "GetComponentsByType",
        ),
        DefinitionCheck::fixture(
            fixture_root,
            "modded_game_mode_members.c",
            "parameter declaration",
            "out int foundCount",
            "foundCount",
        ),
        DefinitionCheck::fixture(
            fixture_root,
            "local_block_symbols.c",
            "local variable declaration",
            "outfitDataArray = {}",
            "outfitDataArray",
        ),
    ]
}

fn game_data_definition_checks(scripts_root: &Path) -> Vec<DefinitionCheck<'static>> {
    let mut checks = Vec::new();
    let scr_base_game_mode = scripts_root
        .join("Game")
        .join("GameMode")
        .join("SCR_BaseGameMode.c");
    if scr_base_game_mode.exists() {
        checks.extend([
            DefinitionCheck::path(
                scr_base_game_mode.clone(),
                "SCR_BaseGameMode class declaration",
                "class SCR_BaseGameMode : BaseGameMode",
                "SCR_BaseGameMode",
            ),
            DefinitionCheck::path(
                scr_base_game_mode.clone(),
                "SCR_BaseGameMode external base type",
                "class SCR_BaseGameMode : BaseGameMode",
                " BaseGameMode",
            ),
            DefinitionCheck::path(
                scr_base_game_mode.clone(),
                "SCR_BaseGameMode method declaration",
                "protected override void OnGameStart()",
                "OnGameStart",
            ),
            DefinitionCheck::path(
                scr_base_game_mode,
                "SCR_BaseGameMode field use",
                "Event_OnGameStart.Invoke();",
                "Event_OnGameStart",
            ),
        ]);
    }
    checks
}

fn position_for_needle(source: &str, needle: &str, cursor: &str) -> Result<LspPosition, String> {
    let start = source
        .find(needle)
        .ok_or_else(|| format!("Missing needle `{needle}`"))?;
    let cursor_start = needle
        .find(cursor)
        .ok_or_else(|| format!("Missing cursor `{cursor}` in needle `{needle}`"))?;
    let leading_whitespace = cursor.len() - cursor.trim_start().len();
    Ok(position_for_offset(
        source,
        start + cursor_start + leading_whitespace,
    ))
}

fn format_range(range: reforger_language_server::lsp::LspRange) -> String {
    format!(
        "{}:{}-{}:{}",
        range.start.line + 1,
        range.start.character + 1,
        range.end.line + 1,
        range.end.character + 1
    )
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn repo_path(relative: &str) -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(relative)
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
