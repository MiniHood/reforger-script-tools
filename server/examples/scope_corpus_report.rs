use reforger_language_server::ast::{AstSourceFile, ClassMember, Declaration, LocalVariableKind};
use reforger_language_server::index::{GlobalSymbolId, SymbolIndex};
use reforger_language_server::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, SymbolCatalog, SymbolKind,
    SOURCE_PRIORITY_GAME_DATA,
};
use reforger_language_server::parser::parse_source;
use reforger_language_server::scope::{LexicalScopeKind, LexicalScopeModel};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/scope-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_ROWS: usize = 50;
const MAX_SAMPLES: usize = 40;
const SNIPPET_CONTEXT_LINES: usize = 2;

struct Args {
    scripts_path: PathBuf,
    out_path: PathBuf,
}

#[derive(Default)]
struct Totals {
    files: usize,
    bytes: usize,
    lossy_files: usize,
    parse_diagnostics: usize,
    indexed_symbols: usize,
    root_scopes: usize,
    callable_scopes: usize,
    block_scopes: usize,
    scoped_parameters: usize,
    scoped_locals: usize,
    unscoped_parameters: usize,
    unscoped_locals: usize,
    local_variables: usize,
    for_initializer_locals: usize,
    foreach_variables: usize,
    local_not_visible_before_declaration: usize,
    local_visible_at_declaration: usize,
    max_scope_depth: usize,
    max_symbols_in_scope: usize,
}

#[derive(Default)]
struct FileStats {
    path: PathBuf,
    source: String,
    diagnostics: usize,
    indexed_symbols: usize,
    scopes: usize,
    callable_scopes: usize,
    block_scopes: usize,
    scoped_parameters: usize,
    scoped_locals: usize,
    unscoped_parameters: usize,
    unscoped_locals: usize,
    local_variables: usize,
    for_initializer_locals: usize,
    foreach_variables: usize,
    max_scope_depth: usize,
    max_symbols_in_scope: usize,
}

struct ShadowSample {
    path: PathBuf,
    source: String,
    symbol: GlobalSymbolId,
    shadowed: GlobalSymbolId,
    classification: String,
    depth: usize,
}

struct VisibilitySample {
    path: PathBuf,
    source: String,
    symbol: GlobalSymbolId,
    name: String,
    visible_before: bool,
    visible_at_declaration: bool,
}

#[derive(Default)]
struct LocalKindCounts {
    local_variables: usize,
    for_initializer_locals: usize,
    foreach_variables: usize,
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
            "Failed to write scope corpus report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!("Wrote scope corpus report: {}", args.out_path.display());
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut scripts = None;
    let mut out = None;
    let mut args = env::args().skip(1);

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
                    "Usage: node tools/scope-corpus-report.mjs [--scripts <path>] [--out <path>]"
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

    let mut totals = Totals::default();
    let mut file_stats = Vec::new();
    let mut scope_depth_frequency = BTreeMap::<usize, usize>::new();
    let mut symbols_per_scope_frequency = BTreeMap::<usize, usize>::new();
    let mut shadow_counts = BTreeMap::<String, usize>::new();
    let mut shadow_samples = Vec::<ShadowSample>::new();
    let mut visibility_samples = Vec::<VisibilitySample>::new();

    for file in &files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        totals.files += 1;
        totals.bytes += bytes.len();

        let source = String::from_utf8_lossy(&bytes);
        if matches!(source, Cow::Owned(_)) {
            totals.lossy_files += 1;
        }
        let source = source.into_owned();
        let parse = parse_source(&source);
        totals.parse_diagnostics += parse.diagnostics.len();

        let ast = AstSourceFile::new(&source, &parse);
        let metadata = game_data_metadata(scripts_path, file);
        let catalog = SymbolCatalog::from_ast_with_metadata(&source, &ast, metadata);
        let mut index = SymbolIndex::default();
        index.add_catalog(&catalog);
        let scope_model = LexicalScopeModel::from_parse_and_index(&parse, &index);
        let local_kinds = ast_local_kind_counts(&ast);

        let mut stats = FileStats {
            path: file.clone(),
            source: source.clone(),
            diagnostics: parse.diagnostics.len(),
            indexed_symbols: index.symbols().len(),
            scopes: scope_model.scopes().len(),
            local_variables: local_kinds.local_variables,
            for_initializer_locals: local_kinds.for_initializer_locals,
            foreach_variables: local_kinds.foreach_variables,
            ..FileStats::default()
        };

        totals.indexed_symbols += index.symbols().len();
        totals.local_variables += stats.local_variables;
        totals.for_initializer_locals += stats.for_initializer_locals;
        totals.foreach_variables += stats.foreach_variables;

        for scope in scope_model.scopes() {
            let depth = scope_depth(&scope_model, scope.id);
            *scope_depth_frequency.entry(depth).or_default() += 1;
            *symbols_per_scope_frequency
                .entry(scope.symbols.len())
                .or_default() += 1;
            stats.max_scope_depth = stats.max_scope_depth.max(depth);
            stats.max_symbols_in_scope = stats.max_symbols_in_scope.max(scope.symbols.len());
            totals.max_scope_depth = totals.max_scope_depth.max(depth);
            totals.max_symbols_in_scope = totals.max_symbols_in_scope.max(scope.symbols.len());

            match scope.kind {
                LexicalScopeKind::Root => totals.root_scopes += 1,
                LexicalScopeKind::Callable => {
                    totals.callable_scopes += 1;
                    stats.callable_scopes += 1;
                }
                LexicalScopeKind::Block => {
                    totals.block_scopes += 1;
                    stats.block_scopes += 1;
                }
            }

            for symbol_id in &scope.symbols {
                if let Some(symbol) = index.symbol(*symbol_id) {
                    match symbol.kind {
                        SymbolKind::Parameter => {
                            totals.scoped_parameters += 1;
                            stats.scoped_parameters += 1;
                        }
                        SymbolKind::LocalVariable => {
                            totals.scoped_locals += 1;
                            stats.scoped_locals += 1;
                        }
                        _ => {}
                    }
                }
            }
        }

        for symbol in index.symbols() {
            match symbol.kind {
                SymbolKind::Parameter if scope_model.scope_for_symbol(symbol.id).is_none() => {
                    totals.unscoped_parameters += 1;
                    stats.unscoped_parameters += 1;
                }
                SymbolKind::LocalVariable if scope_model.scope_for_symbol(symbol.id).is_none() => {
                    totals.unscoped_locals += 1;
                    stats.unscoped_locals += 1;
                }
                _ => {}
            }
        }

        collect_shadow_samples(
            file,
            &source,
            &index,
            &scope_model,
            &mut shadow_counts,
            &mut shadow_samples,
        );
        collect_visibility_samples(
            file,
            &source,
            &index,
            &scope_model,
            &mut totals,
            &mut visibility_samples,
        );

        file_stats.push(stats);
    }

    let mut report = String::new();
    report.push_str("# Scope Corpus Report\n\n");
    report.push_str("> Human-review output generated by `node tools/scope-corpus-report.mjs`.\n\n");
    report.push_str("This report checks lexical scope construction over parsed Reforger scripts. It is review data only; Workbench remains compiler truth.\n\n");
    append_summary(&mut report, scripts_path, &totals);
    append_scope_quality(&mut report, &totals);
    append_usize_counts(
        &mut report,
        "Scope Depth Frequency",
        "Depth",
        &scope_depth_frequency,
    );
    append_usize_counts(
        &mut report,
        "Symbols Per Scope Frequency",
        "Symbols",
        &symbols_per_scope_frequency,
    );
    append_shadow_counts(&mut report, &shadow_counts);
    append_top_files(
        &mut report,
        scripts_path,
        "Top Files By Block Scopes",
        &file_stats,
        |stats| stats.block_scopes,
    );
    append_top_files(
        &mut report,
        scripts_path,
        "Top Files By Scope Depth",
        &file_stats,
        |stats| stats.max_scope_depth,
    );
    append_top_files(
        &mut report,
        scripts_path,
        "Top Files By Scoped Locals",
        &file_stats,
        |stats| stats.scoped_locals,
    );
    append_shadow_samples(&mut report, scripts_path, &shadow_samples, &file_stats);
    append_visibility_samples(&mut report, scripts_path, &visibility_samples, &file_stats);
    append_deep_scope_samples(&mut report, scripts_path, &file_stats);

    Ok(report)
}

fn ast_local_kind_counts(ast: &AstSourceFile<'_, '_>) -> LocalKindCounts {
    let mut counts = LocalKindCounts::default();
    for declaration in ast.declarations() {
        match declaration {
            Declaration::Class(class) => {
                for member in class.members() {
                    if let ClassMember::Method(method) = member {
                        for local in method.local_variables() {
                            count_local_kind(&mut counts, local.kind());
                        }
                    }
                }
            }
            Declaration::Function(function) => {
                for local in function.local_variables() {
                    count_local_kind(&mut counts, local.kind());
                }
            }
            _ => {}
        }
    }
    counts
}

fn count_local_kind(counts: &mut LocalKindCounts, kind: LocalVariableKind) {
    match kind {
        LocalVariableKind::LocalVariable => counts.local_variables += 1,
        LocalVariableKind::ForeachVariable => counts.foreach_variables += 1,
        LocalVariableKind::ForInitializer => counts.for_initializer_locals += 1,
    }
}

fn collect_shadow_samples(
    path: &Path,
    source: &str,
    index: &SymbolIndex,
    scope_model: &LexicalScopeModel,
    shadow_counts: &mut BTreeMap<String, usize>,
    samples: &mut Vec<ShadowSample>,
) {
    for scope in scope_model.scopes() {
        for symbol_id in &scope.symbols {
            let Some(symbol) = index.symbol(*symbol_id) else {
                continue;
            };
            let Some(name) = symbol.name.as_deref() else {
                continue;
            };
            if !matches!(
                symbol.kind,
                SymbolKind::Parameter | SymbolKind::LocalVariable
            ) {
                continue;
            }
            let mut current = scope.parent;
            while let Some(scope_id) = current {
                let Some(parent_scope) = scope_model.scope(scope_id) else {
                    break;
                };
                for parent_symbol_id in &parent_scope.symbols {
                    let Some(parent_symbol) = index.symbol(*parent_symbol_id) else {
                        continue;
                    };
                    if parent_symbol.name.as_deref() == Some(name)
                        && matches!(
                            parent_symbol.kind,
                            SymbolKind::Parameter | SymbolKind::LocalVariable
                        )
                    {
                        let classification = format!(
                            "{} shadows {}",
                            scope_symbol_label(symbol.kind),
                            scope_symbol_label(parent_symbol.kind)
                        );
                        *shadow_counts.entry(classification.clone()).or_default() += 1;
                        if samples.len() < MAX_SAMPLES {
                            samples.push(ShadowSample {
                                path: path.to_path_buf(),
                                source: source.to_string(),
                                symbol: *symbol_id,
                                shadowed: *parent_symbol_id,
                                classification,
                                depth: scope_depth(scope_model, scope.id),
                            });
                        }
                    }
                }
                current = parent_scope.parent;
            }
        }
    }
}

fn collect_visibility_samples(
    path: &Path,
    source: &str,
    index: &SymbolIndex,
    scope_model: &LexicalScopeModel,
    totals: &mut Totals,
    samples: &mut Vec<VisibilitySample>,
) {
    for symbol in index
        .symbols()
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::LocalVariable)
    {
        let Some(name) = symbol.name.as_deref() else {
            continue;
        };
        let before_offset = symbol.selection_span.start.saturating_sub(1);
        let visible_before = scope_model
            .visible_symbols_named(index, name, before_offset)
            .contains(&symbol.id);
        let visible_at_declaration = scope_model
            .visible_symbols_named(index, name, symbol.selection_span.start)
            .contains(&symbol.id);

        if !visible_before {
            totals.local_not_visible_before_declaration += 1;
        }
        if visible_at_declaration {
            totals.local_visible_at_declaration += 1;
        }
        if samples.len() < MAX_SAMPLES && (!visible_at_declaration || visible_before) {
            samples.push(VisibilitySample {
                path: path.to_path_buf(),
                source: source.to_string(),
                symbol: symbol.id,
                name: name.to_string(),
                visible_before,
                visible_at_declaration,
            });
        }
    }
}

fn append_summary(report: &mut String, scripts_path: &Path, totals: &Totals) {
    report.push_str("## Summary\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Source path | `{}` |\n", scripts_path.display()));
    report.push_str(&format!(
        "| Scan timestamp unix seconds | {} |\n",
        timestamp()
    ));
    report.push_str(&format!("| `.c` files | {} |\n", totals.files));
    report.push_str(&format!("| Bytes | {} |\n", totals.bytes));
    report.push_str(&format!(
        "| Files decoded lossily | {} |\n",
        totals.lossy_files
    ));
    report.push_str(&format!(
        "| Parse diagnostics | {} |\n",
        totals.parse_diagnostics
    ));
    report.push_str(&format!(
        "| Indexed symbols | {} |\n",
        totals.indexed_symbols
    ));
    report.push_str(&format!("| Root scopes | {} |\n", totals.root_scopes));
    report.push_str(&format!(
        "| Callable scopes | {} |\n",
        totals.callable_scopes
    ));
    report.push_str(&format!("| Block scopes | {} |\n", totals.block_scopes));
    report.push_str(&format!(
        "| Scoped parameters | {} |\n",
        totals.scoped_parameters
    ));
    report.push_str(&format!("| Scoped locals | {} |\n", totals.scoped_locals));
    report.push_str(&format!(
        "| Unscoped parameters | {} |\n",
        totals.unscoped_parameters
    ));
    report.push_str(&format!(
        "| Unscoped locals | {} |\n",
        totals.unscoped_locals
    ));
    report.push_str(&format!(
        "| Regular local declarations | {} |\n",
        totals.local_variables
    ));
    report.push_str(&format!(
        "| `for` initializer locals | {} |\n",
        totals.for_initializer_locals
    ));
    report.push_str(&format!(
        "| `foreach` variables | {} |\n\n",
        totals.foreach_variables
    ));
}

fn append_scope_quality(report: &mut String, totals: &Totals) {
    report.push_str("## Scope Quality\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Max scope depth | {} |\n",
        totals.max_scope_depth
    ));
    report.push_str(&format!(
        "| Max symbols in one scope | {} |\n",
        totals.max_symbols_in_scope
    ));
    report.push_str(&format!(
        "| Locals not visible before declaration | {} |\n",
        totals.local_not_visible_before_declaration
    ));
    report.push_str(&format!(
        "| Locals visible at declaration | {} |\n",
        totals.local_visible_at_declaration
    ));
    report.push_str(&format!(
        "| Local declaration-before-use failures | {} |\n\n",
        totals
            .scoped_locals
            .saturating_sub(totals.local_not_visible_before_declaration)
            + totals
                .scoped_locals
                .saturating_sub(totals.local_visible_at_declaration)
    ));
}

fn append_usize_counts(
    report: &mut String,
    heading: &str,
    item_heading: &str,
    counts: &BTreeMap<usize, usize>,
) {
    report.push_str(&format!("## {heading}\n\n"));
    if counts.is_empty() {
        report.push_str("None.\n\n");
        return;
    }
    report.push_str(&format!("| {item_heading} | Count |\n"));
    report.push_str("| ---: | ---: |\n");
    for (item, count) in counts.iter().take(MAX_ROWS) {
        report.push_str(&format!("| {item} | {count} |\n"));
    }
    report.push('\n');
}

fn append_shadow_counts(report: &mut String, counts: &BTreeMap<String, usize>) {
    report.push_str("## Shadow Classification\n\n");
    if counts.is_empty() {
        report.push_str("None.\n\n");
        return;
    }
    report.push_str("| Classification | Count |\n");
    report.push_str("| --- | ---: |\n");
    for (classification, count) in sorted_counts(counts).into_iter().take(MAX_ROWS) {
        report.push_str(&format!("| `{}` | {} |\n", classification, count));
    }
    report.push('\n');
}

fn append_top_files<F>(
    report: &mut String,
    root: &Path,
    title: &str,
    file_stats: &[FileStats],
    metric: F,
) where
    F: Fn(&FileStats) -> usize,
{
    let mut rows = file_stats
        .iter()
        .filter(|stats| metric(stats) > 0)
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        metric(right)
            .cmp(&metric(left))
            .then_with(|| left.path.cmp(&right.path))
    });

    report.push_str(&format!("## {title}\n\n"));
    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str(
        "| File | Value | Scopes | Symbols | Parameters | Locals | Max depth | Diagnostics |\n",
    );
    report.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for stats in rows.into_iter().take(25) {
        report.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} | {} | {} |\n",
            relative_path(root, &stats.path),
            metric(stats),
            stats.scopes,
            stats.indexed_symbols,
            stats.scoped_parameters,
            stats.scoped_locals,
            stats.max_scope_depth,
            stats.diagnostics
        ));
    }
    report.push('\n');
}

fn append_shadow_samples(
    report: &mut String,
    root: &Path,
    samples: &[ShadowSample],
    file_stats: &[FileStats],
) {
    report.push_str("## Shadow Samples\n\n");
    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }
    report.push_str("| File | Line | Symbol | Shadows | Classification | Scope depth |\n");
    report.push_str("| --- | ---: | --- | --- | --- | ---: |\n");
    for sample in samples.iter().take(MAX_SAMPLES) {
        let Some(index) = index_for_file(file_stats, &sample.path) else {
            continue;
        };
        let Some(symbol) = index.symbol(sample.symbol) else {
            continue;
        };
        let Some(shadowed) = index.symbol(sample.shadowed) else {
            continue;
        };
        let (line, _) = line_column(&sample.source, symbol.selection_span.start);
        report.push_str(&format!(
            "| `{}` | {} | `{}` | `{}` | `{}` | {} |\n",
            relative_path(root, &sample.path),
            line,
            symbol_label(symbol.kind, symbol.name.as_deref()),
            symbol_label(shadowed.kind, shadowed.name.as_deref()),
            sample.classification,
            sample.depth
        ));
    }
    report.push('\n');

    report.push_str("### Shadow Source Snippets\n\n");
    for sample in samples.iter().take(10) {
        let Some(index) = index_for_file(file_stats, &sample.path) else {
            continue;
        };
        let Some(symbol) = index.symbol(sample.symbol) else {
            continue;
        };
        let (line, column) = line_column(&sample.source, symbol.selection_span.start);
        report.push_str(&format!(
            "#### `{}` {}:{} `{}`\n\n",
            relative_path(root, &sample.path),
            line,
            column,
            symbol_label(symbol.kind, symbol.name.as_deref())
        ));
        append_source_snippet(report, &sample.source, line);
    }
}

fn append_visibility_samples(
    report: &mut String,
    root: &Path,
    samples: &[VisibilitySample],
    file_stats: &[FileStats],
) {
    report.push_str("## Declaration-Before-Use Anomaly Samples\n\n");
    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }
    report.push_str("| File | Line | Local | Visible before | Visible at declaration |\n");
    report.push_str("| --- | ---: | --- | --- | --- |\n");
    for sample in samples.iter().take(MAX_SAMPLES) {
        let Some(index) = index_for_file(file_stats, &sample.path) else {
            continue;
        };
        let Some(symbol) = index.symbol(sample.symbol) else {
            continue;
        };
        let (line, _) = line_column(&sample.source, symbol.selection_span.start);
        report.push_str(&format!(
            "| `{}` | {} | `{}` | {} | {} |\n",
            relative_path(root, &sample.path),
            line,
            escape_table(&sample.name),
            sample.visible_before,
            sample.visible_at_declaration
        ));
    }
    report.push('\n');
}

fn append_deep_scope_samples(report: &mut String, root: &Path, file_stats: &[FileStats]) {
    let mut rows = file_stats
        .iter()
        .filter(|stats| stats.max_scope_depth >= 4)
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .max_scope_depth
            .cmp(&left.max_scope_depth)
            .then_with(|| left.path.cmp(&right.path))
    });

    report.push_str("## Deep Scope Samples\n\n");
    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }
    for stats in rows.into_iter().take(10) {
        report.push_str(&format!(
            "### `{}` depth {}\n\n",
            relative_path(root, &stats.path),
            stats.max_scope_depth
        ));
        if let Some(line) = line_for_first_deep_construct(&stats.source) {
            append_source_snippet(report, &stats.source, line);
        }
    }
}

fn index_for_file(stats: &[FileStats], path: &Path) -> Option<SymbolIndex> {
    let stats = stats.iter().find(|stats| stats.path == path)?;
    let parse = parse_source(&stats.source);
    let ast = AstSourceFile::new(&stats.source, &parse);
    let catalog =
        SymbolCatalog::from_ast_with_metadata(&stats.source, &ast, SourceFileMetadata::unknown());
    Some(SymbolIndex::from_catalogs([&catalog]))
}

fn game_data_metadata(root: &Path, file: &Path) -> SourceFileMetadata {
    let relative_path = file.strip_prefix(root).unwrap_or(file).to_path_buf();
    SourceFileMetadata {
        kind: SourceKind::GameData,
        category: source_category_for_path(SourceKind::GameData, Some(&relative_path)),
        absolute_path: Some(file.to_path_buf()),
        root_path: Some(root.to_path_buf()),
        relative_path: Some(relative_path),
        priority: SOURCE_PRIORITY_GAME_DATA,
    }
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

fn scope_depth(
    scope_model: &LexicalScopeModel,
    id: reforger_language_server::scope::LexicalScopeId,
) -> usize {
    let mut depth = 0usize;
    let mut current = Some(id);
    while let Some(scope_id) = current {
        let Some(scope) = scope_model.scope(scope_id) else {
            break;
        };
        current = scope.parent;
        if current.is_some() {
            depth += 1;
        }
    }
    depth
}

fn scope_symbol_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Parameter => "parameter",
        SymbolKind::LocalVariable => "local",
        _ => "symbol",
    }
}

fn symbol_label(kind: SymbolKind, name: Option<&str>) -> String {
    format!("{kind:?} {}", name.unwrap_or("<missing>"))
}

fn line_for_first_deep_construct(source: &str) -> Option<usize> {
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("if ")
            || trimmed.starts_with("if(")
            || trimmed.starts_with("for ")
            || trimmed.starts_with("for(")
            || trimmed.starts_with("foreach ")
            || trimmed.starts_with("foreach(")
            || trimmed == "{"
        {
            return Some(index + 1);
        }
    }
    None
}

fn append_source_snippet(report: &mut String, source: &str, line: usize) {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        report.push_str("````text\n<empty file>\n````\n\n");
        return;
    }
    let start = line.saturating_sub(SNIPPET_CONTEXT_LINES + 1);
    let end = (line + SNIPPET_CONTEXT_LINES).min(lines.len());
    report.push_str("````enforce\n");
    for index in start..end {
        let marker = if index + 1 == line { ">" } else { " " };
        report.push_str(&format!(
            "{marker} {:>5} | {}\n",
            index + 1,
            lines[index].replace('\t', "    ")
        ));
    }
    report.push_str("````\n\n");
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
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

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut rows = counts
        .iter()
        .map(|(key, count)| (key.clone(), *count))
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    rows
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|")
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
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
