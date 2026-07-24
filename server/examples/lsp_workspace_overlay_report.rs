use reforger_language_server::ast::AstSourceFile;
use reforger_language_server::index::SymbolIndex;
use reforger_language_server::lsp::{
    definition_report_for_source_position_with_external,
    hover_report_for_source_position_with_external, LspPosition,
};
use reforger_language_server::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, SymbolCatalog,
    SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE,
};
use reforger_language_server::parser::parse_source;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const DEFAULT_STRESS_FILES: usize = 200;
const DEFAULT_STRESS_MEMBERS_PER_FILE: usize = 8;
const DEFAULT_STRESS_UPDATES: usize = 20;

struct Args {
    out: PathBuf,
    stress_files: usize,
    stress_members_per_file: usize,
    stress_updates: usize,
}

fn main() -> Result<(), String> {
    let args = parse_args()?;

    let game_source = "class OverlayType\n{\n\tvoid GameOnly();\n\tvoid Shared();\n}\n";
    let workspace_source = "class OverlayType\n{\n\tvoid WorkspaceOnly();\n\tvoid Shared();\n}\n";
    let updated_workspace_source = "class OverlayType\n{\n\tvoid UpdatedOnly();\n}\n";
    let user_source = "class User\n{\n\tvoid Run()\n\t{\n\t\tOverlayType value;\n\t\tvalue.WorkspaceOnly();\n\t\tvalue.UpdatedOnly();\n\t}\n}\n";

    let temp = env::temp_dir().join("reforger_lsp_workspace_overlay_report");
    let game_root = temp.join("game-data").join("scripts");
    let workspace_root = temp.join("workspace").join("Scripts");
    fs::create_dir_all(&game_root).map_err(|error| error.to_string())?;
    fs::create_dir_all(&workspace_root).map_err(|error| error.to_string())?;
    let game_file = game_root.join("OverlayType.c");
    let workspace_file = workspace_root.join("OverlayType.c");
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
    let updated_workspace_index = index_for_source(
        updated_workspace_source,
        &workspace_root,
        &workspace_file,
        SourceKind::Workspace,
        SOURCE_PRIORITY_WORKSPACE,
    );
    let updated_overlay = SymbolIndex::merged([&updated_workspace_index, &game_index]);
    let deleted_overlay = SymbolIndex::merged([&game_index]);

    let workspace_hover = hover_report_for_source_position_with_external(
        user_source,
        position_for_needle(user_source, "value.WorkspaceOnly", "WorkspaceOnly"),
        Some(&overlay),
    );
    let updated_hover = hover_report_for_source_position_with_external(
        user_source,
        position_for_needle(user_source, "value.UpdatedOnly", "UpdatedOnly"),
        Some(&updated_overlay),
    );
    let deleted_hover = hover_report_for_source_position_with_external(
        user_source,
        position_for_needle(user_source, "value.WorkspaceOnly", "WorkspaceOnly"),
        Some(&deleted_overlay),
    );
    let definition = definition_report_for_source_position_with_external(
        user_source,
        "file:///workspace/Scripts/User.c",
        position_for_needle(user_source, "OverlayType value", "OverlayType"),
        Some(&overlay),
    );

    let mut report = String::new();
    report.push_str("# LSP Workspace Overlay Report\n\n");
    report.push_str(
        "Dev-only proof that workspace symbols overlay game-data symbols for hover/definition.\n\n",
    );
    report.push_str("## Summary\n\n");
    report.push_str(&format!(
        "- Game-data symbols: `{}`\n",
        game_index.symbols().len()
    ));
    report.push_str(&format!(
        "- Workspace symbols: `{}`\n",
        workspace_index.symbols().len()
    ));
    report.push_str(&format!(
        "- Overlay symbols: `{}`\n",
        overlay.symbols().len()
    ));
    report.push_str("\n## Checks\n\n");
    append_hover_check(&mut report, "Workspace member hover", &workspace_hover);
    append_hover_check(
        &mut report,
        "Updated workspace member hover",
        &updated_hover,
    );
    append_hover_check(
        &mut report,
        "Deleted workspace member hover",
        &deleted_hover,
    );
    report.push_str(&format!(
        "- Definition target count: `{}`\n",
        definition.locations.len()
    ));
    if let Some(location) = definition.locations.first() {
        report.push_str(&format!("- Definition URI: `{}`\n", location.uri));
    }

    append_stress_report(
        &mut report,
        &game_index,
        &workspace_root,
        args.stress_files,
        args.stress_members_per_file,
        args.stress_updates,
    );

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&args.out, report).map_err(|error| error.to_string())?;
    println!("Wrote {}", args.out.display());
    let _ = fs::remove_dir_all(temp);
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut out = PathBuf::from("tools/reports/lsp-workspace-overlay.report.md");
    let mut stress_files = DEFAULT_STRESS_FILES;
    let mut stress_members_per_file = DEFAULT_STRESS_MEMBERS_PER_FILE;
    let mut stress_updates = DEFAULT_STRESS_UPDATES;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                let Some(value) = args.next() else {
                    return Err("--out requires a path".to_string());
                };
                out = PathBuf::from(value);
            }
            "--stress-files" => {
                let Some(value) = args.next() else {
                    return Err("--stress-files requires a number".to_string());
                };
                stress_files = value
                    .parse()
                    .map_err(|error| format!("Invalid --stress-files value `{value}`: {error}"))?;
            }
            "--stress-members" => {
                let Some(value) = args.next() else {
                    return Err("--stress-members requires a number".to_string());
                };
                stress_members_per_file = value.parse().map_err(|error| {
                    format!("Invalid --stress-members value `{value}`: {error}")
                })?;
            }
            "--stress-updates" => {
                let Some(value) = args.next() else {
                    return Err("--stress-updates requires a number".to_string());
                };
                stress_updates = value.parse().map_err(|error| {
                    format!("Invalid --stress-updates value `{value}`: {error}")
                })?;
            }
            "--help" | "-h" => {
                println!("Usage: cargo run --manifest-path server/Cargo.toml --example lsp_workspace_overlay_report -- [--out <path>] [--stress-files <n>] [--stress-members <n>] [--stress-updates <n>]");
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }

    Ok(Args {
        out,
        stress_files,
        stress_members_per_file,
        stress_updates,
    })
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

fn append_hover_check(
    report: &mut String,
    label: &str,
    hover: &reforger_language_server::lsp::LspHoverReport,
) {
    report.push_str(&format!(
        "- {}: hit=`{}` selected=`{}` kind=`{}` source=`{}` reason=`{}`\n",
        label,
        hover.is_hit(),
        hover.selected_label.as_deref().unwrap_or("<none>"),
        hover
            .selected_kind
            .map(|kind| format!("{kind:?}"))
            .unwrap_or_else(|| "<none>".to_string()),
        hover
            .selected_source
            .map(|source| source.as_str())
            .unwrap_or("<none>"),
        hover
            .resolver_reason
            .map(|reason| reason.as_str())
            .unwrap_or("<none>")
    ));
}

fn append_stress_report(
    report: &mut String,
    game_index: &SymbolIndex,
    workspace_root: &Path,
    stress_files: usize,
    stress_members_per_file: usize,
    stress_updates: usize,
) {
    report.push_str("\n## Workspace Overlay Stress\n\n");
    report.push_str("Synthetic stress data measures the current full overlay recompute shape after workspace file updates. Timings are dev-machine wall-clock diagnostics, not benchmarks.\n\n");

    let build_start = Instant::now();
    let mut workspace_indexes = Vec::new();
    for file_index in 0..stress_files {
        let source = stress_source(file_index, stress_members_per_file, 0);
        let file = workspace_root.join(format!("StressType{file_index}.c"));
        workspace_indexes.push(index_for_source(
            &source,
            workspace_root,
            &file,
            SourceKind::Workspace,
            SOURCE_PRIORITY_WORKSPACE,
        ));
    }
    let build_ms = build_start.elapsed().as_millis();

    let initial_merge_start = Instant::now();
    let initial_overlay =
        SymbolIndex::merged(workspace_indexes.iter().chain(std::iter::once(game_index)));
    let initial_merge_ms = initial_merge_start.elapsed().as_millis();
    let initial_symbols = initial_overlay.symbols().len();

    let mut update_reindex_ms = Vec::new();
    let mut update_merge_ms = Vec::new();
    for update in 0..stress_updates {
        if workspace_indexes.is_empty() {
            break;
        }
        let file_index = update % workspace_indexes.len();
        let source = stress_source(file_index, stress_members_per_file, update + 1);
        let file = workspace_root.join(format!("StressType{file_index}.c"));
        let reindex_start = Instant::now();
        workspace_indexes[file_index] = index_for_source(
            &source,
            workspace_root,
            &file,
            SourceKind::Workspace,
            SOURCE_PRIORITY_WORKSPACE,
        );
        update_reindex_ms.push(reindex_start.elapsed().as_millis());

        let merge_start = Instant::now();
        let overlay =
            SymbolIndex::merged(workspace_indexes.iter().chain(std::iter::once(game_index)));
        update_merge_ms.push(merge_start.elapsed().as_millis());
        std::hint::black_box(overlay.symbols().len());
    }

    let delete_merge_ms = if workspace_indexes.is_empty() {
        0
    } else {
        workspace_indexes.pop();
        let delete_start = Instant::now();
        let overlay =
            SymbolIndex::merged(workspace_indexes.iter().chain(std::iter::once(game_index)));
        let elapsed = delete_start.elapsed().as_millis();
        std::hint::black_box(overlay.symbols().len());
        elapsed
    };

    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Synthetic workspace files | {stress_files} |\n"));
    report.push_str(&format!(
        "| Members per synthetic class | {stress_members_per_file} |\n"
    ));
    report.push_str(&format!("| Update iterations | {stress_updates} |\n"));
    report.push_str(&format!("| Initial workspace build ms | {build_ms} |\n"));
    report.push_str(&format!(
        "| Initial overlay merge ms | {initial_merge_ms} |\n"
    ));
    report.push_str(&format!(
        "| Initial overlay symbols | {initial_symbols} |\n"
    ));
    append_timing_rows(report, "Changed-file reindex", &update_reindex_ms);
    append_timing_rows(report, "Overlay recompute after update", &update_merge_ms);
    report.push_str(&format!(
        "| Overlay recompute after delete ms | {delete_merge_ms} |\n\n"
    ));

    report.push_str("### Stress Interpretation\n\n");
    if percentile(&update_merge_ms, 95) > 50 {
        report.push_str("- Overlay recompute p95 is above 50 ms in this synthetic run. Consider an incremental overlay map update slice before much larger workspace indexing.\n\n");
    } else {
        report.push_str("- Overlay recompute p95 is at or below 50 ms in this synthetic run. Full recompute remains acceptable for the current live-overlay slice.\n\n");
    }
}

fn append_timing_rows(report: &mut String, label: &str, values: &[u128]) {
    report.push_str(&format!("| {label} avg ms | {} |\n", average(values)));
    report.push_str(&format!(
        "| {label} p95 ms | {} |\n",
        percentile(values, 95)
    ));
    report.push_str(&format!(
        "| {label} max ms | {} |\n",
        values.iter().copied().max().unwrap_or(0)
    ));
}

fn average(values: &[u128]) -> u128 {
    if values.is_empty() {
        return 0;
    }
    values.iter().sum::<u128>() / values.len() as u128
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let mut values = values.to_vec();
    values.sort_unstable();
    let index = ((values.len() - 1) * percentile) / 100;
    values[index]
}

fn stress_source(file_index: usize, members: usize, generation: usize) -> String {
    let mut source = format!("class StressType{file_index}\n{{\n");
    source.push_str(&format!("\tint m_Value{generation};\n"));
    for member in 0..members {
        source.push_str(&format!("\tvoid Method{member}_{generation}() {{}}\n"));
    }
    source.push_str("}\n");
    source
}

fn position_for_needle(source: &str, needle: &str, cursor: &str) -> LspPosition {
    let start = source.find(needle).expect("needle not found");
    let cursor_offset = source[start..]
        .find(cursor)
        .map(|offset| start + offset)
        .expect("cursor not found");
    let before = &source[..cursor_offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let line_start = before.rfind('\n').map(|offset| offset + 1).unwrap_or(0);
    LspPosition {
        line,
        character: before[line_start..].encode_utf16().count() as u32,
    }
}
