use reforger_language_server::ast::AstSourceFile;
use reforger_language_server::index::SymbolIndex;
use reforger_language_server::lsp::{
    completion_report_for_source_position_with_external, LspCompletionReport, LspPosition,
};
use reforger_language_server::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, SymbolCatalog,
    SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE,
};
use reforger_language_server::parser::parse_source;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    let mut out = PathBuf::from("tools/reports/lsp-completion-fixtures.report.md");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--out" {
            if let Some(value) = args.next() {
                out = PathBuf::from(value);
            }
        }
    }

    let temp = env::temp_dir().join("reforger_lsp_completion_report");
    let game_root = temp.join("game-data").join("scripts");
    let workspace_root = temp.join("workspace").join("Scripts");
    fs::create_dir_all(&game_root).map_err(|error| error.to_string())?;
    fs::create_dir_all(&workspace_root).map_err(|error| error.to_string())?;

    let game_dir = game_root.join("Game");
    fs::create_dir_all(&game_dir).map_err(|error| error.to_string())?;
    let game_file = game_dir.join("RuntimeTypes.c");
    let workspace_file = workspace_root.join("WorkspaceTypes.c");
    let game_source = r#"class array
{
	void Insert(int value);
	void Remove(int index);
}

class Game
{
	World GetWorld();
}

class World
{
	void Trace();
}

class SCR_ReportType {}
enum SCR_ReportEnum
{
	SCR_ReportValue
}
typedef int SCR_ReportAlias;
int SCR_GlobalValue;
Game GetGame();

class OverlayType
{
	void GameOnly();
	void Shared();
}

class OverloadType
{
	void Run();
	void Run(int value);
}

class ParentType
{
	protected void OnPostInit(IEntity owner);
	private void OnHidden();
	static void OnStatic();
}
"#;
    let workspace_source = r#"class OverlayType
{
	void WorkspaceOnly();
	void Shared();
}
"#;
    fs::write(&game_file, game_source).map_err(|error| error.to_string())?;
    fs::write(&workspace_file, workspace_source).map_err(|error| error.to_string())?;

    let game_index = index_for_source(
        game_source,
        &game_root,
        &game_file,
        SourceKind::GameData,
        SOURCE_PRIORITY_GAME_DATA,
    );
    let workspace_index = index_for_source(
        workspace_source,
        &workspace_root,
        &workspace_file,
        SourceKind::Workspace,
        SOURCE_PRIORITY_WORKSPACE,
    );
    let overlay = SymbolIndex::merged([&workspace_index, &game_index]);
    let deleted_overlay = SymbolIndex::merged([&game_index]);
    let core_fixture_path = PathBuf::from("tools/fixtures/parser/core_types_declarations.c");
    let core_fixture_source = fs::read_to_string(&core_fixture_path).unwrap_or_default();
    let core_fixture_index = if core_fixture_source.is_empty() {
        SymbolIndex::default()
    } else {
        index_for_source(
            &core_fixture_source,
            Path::new("tools/fixtures/parser/game-data"),
            Path::new("tools/fixtures/parser/game-data/Core/proto/Types.c"),
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )
    };

    let checks = vec![
        (
            "array member",
            completion_check(
                "array<int> m_aDisabledResourceTypes;\n\t\tm_aDisabledResourceTypes.",
                "m_aDisabledResourceTypes.",
                &overlay,
            ),
        ),
        (
            "GetGame member",
            completion_check("GetGame().", "GetGame().", &overlay),
        ),
        (
            "callable prefix snippet",
            completion_check("GetG", "GetG", &overlay),
        ),
        (
            "type prefix",
            completion_source_check(
                "class Example { void Run(SCR_ value) {} }",
                "SCR_",
                &overlay,
            ),
        ),
        (
            "top-level value prefix",
            completion_check("SCR_", "SCR_", &overlay),
        ),
        (
            "function prefix",
            completion_check("GetG", "GetG", &overlay),
        ),
        (
            "game-derived fixture member",
            completion_source_check(
                "class Example { void Run() { array<string> values; values.C } }",
                "values.C",
                &core_fixture_index,
            ),
        ),
        (
            "game-derived fixture type",
            completion_source_check(
                "class Example { void Run(TString value) {} }",
                "TString",
                &core_fixture_index,
            ),
        ),
        (
            "workspace overlay member",
            completion_check("OverlayType overlay;\n\t\toverlay.", "overlay.", &overlay),
        ),
        (
            "workspace deleted member",
            completion_check(
                "OverlayType overlay;\n\t\toverlay.",
                "overlay.",
                &deleted_overlay,
            ),
        ),
        (
            "overload labels",
            completion_check(
                "OverloadType overload;\n\t\toverload.R",
                "overload.R",
                &overlay,
            ),
        ),
        (
            "inherited override skeleton",
            completion_source_check(
                "class ChildType : ParentType\n{\n\tOnPostIn\n}\n",
                "OnPostIn",
                &overlay,
            ),
        ),
        (
            "unresolved receiver",
            completion_check("missing.", "missing.", &overlay),
        ),
        (
            "non-member position",
            completion_report_for_source_position_with_external(
                "class Example {}",
                LspPosition {
                    line: 0,
                    character: 5,
                },
                Some(&overlay),
            ),
        ),
    ];

    let mut report = String::new();
    report.push_str("# LSP Completion Fixture Report\n\n");
    report.push_str("Dev-only proof for member, type-prefix, and top-level-prefix `textDocument/completion`.\n\n");
    report.push_str("| Check | Context | Receiver | Owner | Prefix | Items | Failure | Samples | First Edit | First Sort |\n");
    report.push_str("| --- | --- | --- | --- | --- | ---: | --- | --- | --- | --- |\n");
    for (label, check) in checks {
        append_check(&mut report, label, &check);
    }

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&out, report).map_err(|error| error.to_string())?;
    println!("Wrote {}", out.display());
    let _ = fs::remove_dir_all(temp);
    Ok(())
}

fn completion_check(body: &str, needle: &str, overlay: &SymbolIndex) -> LspCompletionReport {
    let source = format!(
        "class Example\n{{\n\tvoid Run()\n\t{{\n\t\t{}\n\t}}\n}}\n",
        body
    );
    completion_report_for_source_position_with_external(
        &source,
        position_after_needle(&source, needle),
        Some(overlay),
    )
}

fn completion_source_check(
    source: &str,
    needle: &str,
    overlay: &SymbolIndex,
) -> LspCompletionReport {
    completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, needle),
        Some(overlay),
    )
}

fn append_check(report: &mut String, label: &str, check: &LspCompletionReport) {
    let samples = check
        .list
        .items
        .iter()
        .take(5)
        .map(|item| {
            let detail = item
                .label_details
                .as_ref()
                .and_then(|details| details.detail.as_deref())
                .unwrap_or("");
            format!("{}{}", item.label, detail)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let first_edit = check
        .list
        .items
        .first()
        .map(|item| item.text_edit.new_text.as_str())
        .unwrap_or("");
    let first_sort = check
        .list
        .items
        .first()
        .and_then(|item| item.sort_text.as_deref())
        .unwrap_or("");
    report.push_str(&format!(
        "| {} | `{}` | `{}` | `{}` | `{}` | {} | `{}` | `{}` | `{}` | `{}` |\n",
        label,
        check.completion_context,
        check.receiver_text.as_deref().unwrap_or("<none>"),
        check.owner_type.as_deref().unwrap_or("<none>"),
        check.prefix,
        check.candidate_count,
        check.failure_reason.as_deref().unwrap_or("<none>"),
        samples,
        first_edit,
        first_sort
    ));
}

fn index_for_source(
    source: &str,
    root: &Path,
    file: &Path,
    kind: SourceKind,
    priority: u16,
) -> SymbolIndex {
    let parse = parse_source(source);
    let ast = AstSourceFile::new(source, &parse);
    let relative_path = file.strip_prefix(root).unwrap_or(file).to_path_buf();
    let catalog = SymbolCatalog::from_ast_with_metadata(
        source,
        &ast,
        SourceFileMetadata {
            kind,
            category: source_category_for_path(kind, Some(&relative_path)),
            absolute_path: Some(file.to_path_buf()),
            root_path: Some(root.to_path_buf()),
            relative_path: Some(relative_path),
            priority,
        },
    );
    SymbolIndex::from_catalogs([&catalog])
}

fn position_after_needle(source: &str, needle: &str) -> LspPosition {
    let offset = source
        .find(needle)
        .map(|start| start + needle.len())
        .expect("needle not found");
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let line_start = before.rfind('\n').map(|offset| offset + 1).unwrap_or(0);
    LspPosition {
        line,
        character: before[line_start..].encode_utf16().count() as u32,
    }
}
