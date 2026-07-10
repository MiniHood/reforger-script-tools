use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::lsp::{
    hover_report_for_source_position_with_external, position_for_offset, symbol_kind_label,
    LspPosition,
};
use reforger_language_server::model::{SourceKind, SOURCE_PRIORITY_GAME_DATA};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const DEFAULT_FIXTURE_ROOT: &str = "tools/fixtures/parser";
const DEFAULT_OUTPUT: &str = "tools/reports/lsp-hover-fixtures.report.md";

const INLINE_HOVER_SOURCE: &str = r#"// Fixture truth: game-data-shaped LSP hover coverage source; not Workbench-confirmed in this repo.
//! Global game instance.
Game g_Game;

[EnumBitFlag()]
enum ExampleFlags
{
	None = 0,
	Enabled = 1
}
"#;

const INLINE_USAGE_HOVER_SOURCE: &str = r#"// Fixture truth: game-data-shaped hover usage coverage source; not Workbench-confirmed in this repo.
Game g_Game;
typedef string FactionKey;

class Example
{
	protected int m_Value;

	void Run(string name)
	{
		int index = 4;
		index = index + 1;
		Print(name);
		m_Value = index;
		FactionKey key;
		g_Game = null;
		MissingThing();
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

    let mut checks = hover_checks(&fixture_root);
    if let Some(scripts_root) = scripts_root.as_ref() {
        checks.extend(game_data_hover_checks(scripts_root));
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
    checks.push(HoverCheck::inline(
        "inline_hover_shapes.c",
        INLINE_HOVER_SOURCE,
        "global field",
        "g_Game",
        "g_Game",
    ));
    checks.push(HoverCheck::inline(
        "inline_hover_shapes.c",
        INLINE_HOVER_SOURCE,
        "enum member",
        "Enabled = 1",
        "Enabled",
    ));
    checks.push(HoverCheck::inline(
        "inline_hover_shapes.c",
        INLINE_HOVER_SOURCE,
        "whitespace miss",
        "\n\n[EnumBitFlag",
        "\n",
    ));
    checks.push(HoverCheck::inline(
        "inline_usage_hover_shapes.c",
        INLINE_USAGE_HOVER_SOURCE,
        "local variable use",
        "index + 1",
        "index",
    ));
    checks.push(HoverCheck::inline(
        "inline_usage_hover_shapes.c",
        INLINE_USAGE_HOVER_SOURCE,
        "parameter use",
        "Print(name)",
        "name",
    ));
    checks.push(HoverCheck::inline(
        "inline_usage_hover_shapes.c",
        INLINE_USAGE_HOVER_SOURCE,
        "field use",
        "m_Value = index",
        "m_Value",
    ));
    checks.push(HoverCheck::inline(
        "inline_usage_hover_shapes.c",
        INLINE_USAGE_HOVER_SOURCE,
        "typedef reference",
        "FactionKey key",
        "FactionKey",
    ));
    checks.push(HoverCheck::inline(
        "inline_usage_hover_shapes.c",
        INLINE_USAGE_HOVER_SOURCE,
        "global field use",
        "g_Game = null",
        "g_Game",
    ));
    checks.push(HoverCheck::inline(
        "inline_usage_hover_shapes.c",
        INLINE_USAGE_HOVER_SOURCE,
        "unresolved identifier miss",
        "MissingThing();",
        "MissingThing",
    ));

    let mut rows = Vec::new();
    for check in checks {
        let source = check.source()?;
        let position = position_for_needle(&source, check.needle, check.cursor)?;
        let start = Instant::now();
        let report = hover_report_for_source_position_with_external(
            &source,
            position,
            external_index.as_ref(),
        );
        let elapsed_ms = start.elapsed().as_millis();
        let hover_preview = report
            .hover
            .as_ref()
            .map(|hover| single_line_preview(&hover.contents.value))
            .unwrap_or_else(|| "<none>".to_string());

        rows.push(HoverRow {
            file: check.file_label(),
            target: check.target.to_string(),
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
            parse_diagnostics: report.parse_diagnostics,
            selection_source: report.selection_source.as_str().to_string(),
            resolver_reason: report
                .resolver_reason
                .map(|reason| reason.as_str().to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            identifier_context: report
                .identifier_context
                .map(|context| context.as_str().to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            resolver_candidate_count: report.resolver_candidate_count,
            selected_source: report
                .selected_source
                .map(|source| source.as_str().to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            elapsed_ms,
            hover_preview,
        });
    }

    let mut markdown = String::new();
    writeln!(markdown, "# LSP Hover Fixture Report").unwrap();
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
    writeln!(markdown, "This report exercises the same resolver-first hover path used by `textDocument/hover`, with game-data supplied as optional external index context. It is review tooling only; it does not perform semantic lookup or Workbench validation.").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "| File | Target | Position | Hit | Selection | Selected source | Resolver reason | Identifier context | Resolver candidates | Selected | Parse diagnostics | Elapsed ms | Hover preview |").unwrap();
    writeln!(
        markdown,
        "| --- | --- | ---: | --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | --- |"
    )
    .unwrap();
    for row in &rows {
        writeln!(
            markdown,
            "| `{}` | {} | {}:{} | {} | `{}` | `{}` | `{}` | `{}` | {} | {} `{}` | {} | {} | {} |",
            row.file,
            escape_markdown_cell(&row.target),
            row.line,
            row.column,
            if row.hit { "yes" } else { "no" },
            row.selection_source,
            row.selected_source,
            row.resolver_reason,
            row.identifier_context,
            row.resolver_candidate_count,
            row.selected_kind,
            escape_markdown_cell(&row.selected_label),
            row.parse_diagnostics,
            row.elapsed_ms,
            escape_markdown_cell(&row.hover_preview)
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
                    println!("Usage: cargo run --example lsp_hover_report -- [--fixtures <path>] [--scripts <path>] [--out <path>]");
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

struct HoverCheck<'source> {
    file: HoverFile<'source>,
    target: &'static str,
    needle: &'static str,
    cursor: &'static str,
}

enum HoverFile<'source> {
    Fixture(PathBuf),
    Inline {
        label: &'static str,
        source: &'source str,
    },
}

impl<'source> HoverCheck<'source> {
    fn fixture(
        fixture_root: &Path,
        file: &'static str,
        target: &'static str,
        needle: &'static str,
        cursor: &'static str,
    ) -> Self {
        Self {
            file: HoverFile::Fixture(fixture_root.join(file)),
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
            file: HoverFile::Fixture(path),
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
            file: HoverFile::Inline { label, source },
            target,
            needle,
            cursor,
        }
    }

    fn source(&self) -> Result<String, String> {
        match &self.file {
            HoverFile::Fixture(path) => fs::read_to_string(path)
                .map_err(|error| format!("Failed to read {}: {error}", path.display())),
            HoverFile::Inline { source, .. } => Ok((*source).to_string()),
        }
    }

    fn file_label(&self) -> String {
        match &self.file {
            HoverFile::Fixture(path) => path.display().to_string().replace('\\', "/"),
            HoverFile::Inline { label, .. } => format!("<inline>/{label}"),
        }
    }
}

struct HoverRow {
    file: String,
    target: String,
    line: u32,
    column: u32,
    hit: bool,
    selected_kind: String,
    selected_label: String,
    parse_diagnostics: usize,
    selection_source: String,
    selected_source: String,
    resolver_reason: String,
    identifier_context: String,
    resolver_candidate_count: usize,
    elapsed_ms: u128,
    hover_preview: String,
}

fn hover_checks(fixture_root: &Path) -> Vec<HoverCheck<'static>> {
    vec![
        HoverCheck::fixture(
            fixture_root,
            "core_types_declarations.c",
            "class",
            "class array<Class T>",
            "array",
        ),
        HoverCheck::fixture(
            fixture_root,
            "modded_game_mode_members.c",
            "field",
            "m_bAllowFactionChange",
            "m_bAllowFactionChange",
        ),
        HoverCheck::fixture(
            fixture_root,
            "modded_game_mode_members.c",
            "method",
            "GetComponentsByType(typename",
            "GetComponentsByType",
        ),
        HoverCheck::fixture(
            fixture_root,
            "modded_game_mode_members.c",
            "parameter",
            "out int foundCount",
            "foundCount",
        ),
        HoverCheck::fixture(
            fixture_root,
            "core_types_declarations.c",
            "typedef",
            "typedef array<string> TStringArray",
            "TStringArray",
        ),
        HoverCheck::fixture(
            fixture_root,
            "local_block_symbols.c",
            "local variable",
            "outfitDataArray = {}",
            "outfitDataArray",
        ),
        HoverCheck::fixture(
            fixture_root,
            "local_block_symbols.c",
            "foreach variable",
            "SCR_OutfitFactionData data :",
            "data",
        ),
        HoverCheck::fixture(
            fixture_root,
            "local_block_symbols.c",
            "foreach auto variable",
            "auto quickslot :",
            "quickslot",
        ),
        HoverCheck::fixture(
            fixture_root,
            "local_block_symbols.c",
            "for initializer",
            "int i = 0",
            "i =",
        ),
        HoverCheck::fixture(
            fixture_root,
            "local_block_symbols.c",
            "for initializer comma declarator",
            "count = outfitDataArray.Count()",
            "count",
        ),
    ]
}

fn game_data_hover_checks(scripts_root: &Path) -> Vec<HoverCheck<'static>> {
    let mut checks = Vec::new();
    let scr_base_game_mode = scripts_root
        .join("Game")
        .join("GameMode")
        .join("SCR_BaseGameMode.c");
    if scr_base_game_mode.exists() {
        checks.extend([
            HoverCheck::path(
                scr_base_game_mode.clone(),
                "SCR_BaseGameMode class declaration",
                "class SCR_BaseGameMode : BaseGameMode",
                "SCR_BaseGameMode",
            ),
            HoverCheck::path(
                scr_base_game_mode.clone(),
                "SCR_BaseGameMode base type external hit",
                "class SCR_BaseGameMode : BaseGameMode",
                " BaseGameMode",
            ),
            HoverCheck::path(
                scr_base_game_mode.clone(),
                "SCR_BaseGameMode OnGameStart method",
                "protected override void OnGameStart()",
                "OnGameStart",
            ),
            HoverCheck::path(
                scr_base_game_mode.clone(),
                "SCR_BaseGameMode field use",
                "Event_OnGameStart.Invoke();",
                "Event_OnGameStart",
            ),
            HoverCheck::path(
                scr_base_game_mode.clone(),
                "SCR_BaseGameMode foreach local use",
                "comp.OnGameEnd();",
                "comp",
            ),
            HoverCheck::path(
                scr_base_game_mode,
                "SCR_BaseGameMode member collection use",
                "foreach (SCR_BaseGameModeComponent comp : m_aAdditionalGamemodeComponents)",
                "m_aAdditionalGamemodeComponents",
            ),
        ]);
    }

    let scr_autotest_result = scripts_root
        .join("Autotest")
        .join("Game")
        .join("TestFramework")
        .join("SCR_AutotestResult.c");
    if scr_autotest_result.exists() {
        checks.extend([
            HoverCheck::path(
                scr_autotest_result.clone(),
                "SCR_AutotestResult return type collision",
                "static SCR_AutotestResult AsFailure",
                "SCR_AutotestResult",
            ),
            HoverCheck::path(
                scr_autotest_result,
                "SCR_AutotestResult local type collision",
                "SCR_AutotestResult result = new SCR_AutotestResult(false",
                "SCR_AutotestResult",
            ),
        ]);
    }

    let widget_use = scripts_root
        .join("Game")
        .join("Components")
        .join("CleanSweep")
        .join("SCR_CleanSweepAreaSelectionButtonComponent.c");
    if widget_use.exists() {
        checks.push(HoverCheck::path(
            widget_use,
            "Widget external type hit",
            "Widget parent = w.GetParent();",
            "Widget",
        ));
    }

    let ientity_use = scripts_root
        .join("Game")
        .join("Sandbox")
        .join("Resources")
        .join("UserActions")
        .join("SCR_ResourceStorageOpenAction.c");
    if ientity_use.exists() {
        checks.push(HoverCheck::path(
            ientity_use,
            "IEntity external type hit",
            "PerformActionInternal(SCR_InventoryStorageManagerComponent manager, IEntity pOwnerEntity, IEntity pUserEntity)",
            "IEntity",
        ));
    }

    let scenario_get_use = scripts_root
        .join("Game")
        .join("ScenarioFramework")
        .join("Actions")
        .join("ResourceComponentActions")
        .join("SCR_ScenarioFrameworkActionOnResourceChanged.c");
    if scenario_get_use.exists() {
        checks.push(HoverCheck::path(
            scenario_get_use,
            "SCR_ScenarioFrameworkGet external type hit",
            "ref SCR_ScenarioFrameworkGet m_Getter;",
            "SCR_ScenarioFrameworkGet",
        ));
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

fn single_line_preview(value: &str) -> String {
    value
        .replace('\r', "")
        .replace('\n', " / ")
        .chars()
        .take(220)
        .collect()
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
