use reforger_language_server::index::{GlobalSymbolId, IndexedSymbol, SymbolIndex};
use reforger_language_server::index_build::{
    build_index, IndexBuildConfig, IndexBuildCounts, IndexSourceRoot,
};
use reforger_language_server::model::{
    SourceKind, SymbolKind, SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE,
};
use std::env;
use std::path::{Path, PathBuf};

const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_MATCHES: usize = 100;
const MAX_CHILDREN: usize = 20;

struct Args {
    scripts_path: PathBuf,
    workspace_path: Option<PathBuf>,
    query: Query,
}

enum Query {
    Name(String),
    TopLevel(String),
    Class(String),
    Typedef(String),
    Function(String),
    Method { owner: String, name: String },
}

fn main() -> Result<(), String> {
    let args = parse_args()?;
    let (index, totals) = build_debug_index(&args.scripts_path, args.workspace_path.as_deref())?;

    println!("# Index Debug");
    println!();
    println!("Query: `{}`", query_label(&args.query));
    println!("Scripts: `{}`", args.scripts_path.display());
    if let Some(workspace_path) = &args.workspace_path {
        println!("Workspace: `{}`", workspace_path.display());
    }
    println!("Files: {}", totals.files);
    println!("Bytes: {}", totals.bytes);
    println!("Files decoded lossily: {}", totals.lossy_files);
    println!("Parse diagnostics: {}", totals.parse_diagnostics);
    println!("Indexed files: {}", index.files().len());
    println!("Indexed symbols: {}", index.symbols().len());
    println!();

    match args.query {
        Query::Name(name) => print_query_results(
            &index,
            "All Symbols By Name",
            &name,
            index.symbols_for_name(&name),
            index.preferred_symbols_for_name(&name).first().copied(),
        ),
        Query::TopLevel(name) => {
            print_query_results(
                &index,
                "Top-Level Symbols By Name",
                &name,
                index.top_level_symbols_for_name(&name),
                index
                    .preferred_top_level_symbols_for_name(&name)
                    .first()
                    .copied(),
            );
            print_kind_specific_top_level_preferred(&index, &name);
        }
        Query::Class(name) => {
            let symbols = index.classes_by_name(&name);
            print_query_results(
                &index,
                "Classes By Name",
                &name,
                symbols,
                index.preferred_classes_by_name(&name).first().copied(),
            );
            print_class_member_summary(&index, &name, symbols);
        }
        Query::Typedef(name) => {
            let symbols = index.typedefs_by_name(&name);
            print_query_results(
                &index,
                "Typedefs By Name",
                &name,
                symbols,
                index.preferred_typedefs_by_name(&name).first().copied(),
            );
        }
        Query::Function(name) => {
            let symbols = index.functions_by_name(&name);
            print_query_results(
                &index,
                "Functions By Name",
                &name,
                symbols,
                index.preferred_functions_by_name(&name).first().copied(),
            );
        }
        Query::Method { owner, name } => {
            let symbols = index.methods_by_owner_name(&owner, &name);
            println!(
                "## Method `{}`.`{}`",
                escape_inline(&owner),
                escape_inline(&name)
            );
            println!();
            println!("Overloads: {}", symbols.len());
            println!();
            print_method_signatures(&index, symbols);
            print_query_results(
                &index,
                "Method Matches",
                &format!("{owner}.{name}"),
                symbols,
                index.preferred_from_symbols(symbols).first().copied(),
            );
        }
    }

    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut scripts: Option<PathBuf> = None;
    let mut workspace: Option<PathBuf> = None;
    let mut query: Option<Query> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scripts" => {
                let Some(value) = args.next() else {
                    return Err("--scripts requires a path".to_string());
                };
                scripts = Some(PathBuf::from(value));
            }
            "--workspace" => {
                let Some(value) = args.next() else {
                    return Err("--workspace requires a path".to_string());
                };
                workspace = Some(PathBuf::from(value));
            }
            "--name" => {
                set_query(&mut query, Query::Name(take_value(&mut args, "--name")?))?;
            }
            "--top-level" => {
                set_query(
                    &mut query,
                    Query::TopLevel(take_value(&mut args, "--top-level")?),
                )?;
            }
            "--class" => {
                set_query(&mut query, Query::Class(take_value(&mut args, "--class")?))?;
            }
            "--typedef" => {
                set_query(
                    &mut query,
                    Query::Typedef(take_value(&mut args, "--typedef")?),
                )?;
            }
            "--function" => {
                set_query(
                    &mut query,
                    Query::Function(take_value(&mut args, "--function")?),
                )?;
            }
            "--method" => {
                let owner = take_value(&mut args, "--method owner")?;
                let name = take_value(&mut args, "--method name")?;
                set_query(&mut query, Query::Method { owner, name })?;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }

    let Some(query) = query else {
        return Err("One query mode is required".to_string());
    };

    Ok(Args {
        scripts_path: scripts.unwrap_or_else(default_scripts_path),
        workspace_path: workspace,
        query,
    })
}

fn take_value(args: &mut impl Iterator<Item = String>, label: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{label} requires a value"))
}

fn set_query(target: &mut Option<Query>, query: Query) -> Result<(), String> {
    if target.is_some() {
        return Err("Only one query mode can be used at a time".to_string());
    }
    *target = Some(query);
    Ok(())
}

fn print_usage() {
    println!("Usage: node tools/index-debug.mjs [--scripts <path>] [--workspace <path>] <query>");
    println!("Queries:");
    println!("  --name <symbol>");
    println!("  --top-level <symbol>");
    println!("  --class <class>");
    println!("  --typedef <typedef>");
    println!("  --function <function>");
    println!("  --method <owner> <method>");
}

fn default_scripts_path() -> PathBuf {
    if let Some(app_data) = env::var_os("APPDATA") {
        PathBuf::from(app_data).join(DEFAULT_STORAGE_RELATIVE_PATH)
    } else {
        PathBuf::from(DEFAULT_STORAGE_RELATIVE_PATH)
    }
}

fn build_debug_index(
    scripts_path: &Path,
    workspace_path: Option<&Path>,
) -> Result<(SymbolIndex, IndexBuildCounts), String> {
    let mut roots = vec![IndexSourceRoot::new(
        scripts_path,
        SourceKind::GameData,
        SOURCE_PRIORITY_GAME_DATA,
    )];
    if let Some(workspace_path) = workspace_path {
        roots.push(IndexSourceRoot::new(
            workspace_path,
            SourceKind::Workspace,
            SOURCE_PRIORITY_WORKSPACE,
        ));
    }

    let result = build_index(&IndexBuildConfig { roots })?;
    Ok((result.index, result.summary.totals))
}

fn print_query_results(
    index: &SymbolIndex,
    heading: &str,
    query: &str,
    symbols: &[GlobalSymbolId],
    preferred: Option<GlobalSymbolId>,
) {
    println!("## {heading} `{}`", escape_inline(query));
    println!();
    println!("Matches: {}", symbols.len());
    println!();

    if symbols.is_empty() {
        println!("No matches.");
        return;
    }

    if let Some(id) = preferred {
        println!("### Preferred Match");
        println!();
        print_symbol(index, id);
    }

    println!("### All Matches");
    println!();
    for id in symbols.iter().take(MAX_MATCHES) {
        print_symbol(index, *id);
    }
    if symbols.len() > MAX_MATCHES {
        println!("... {} more matches omitted", symbols.len() - MAX_MATCHES);
        println!();
    }
}

fn print_kind_specific_top_level_preferred(index: &SymbolIndex, name: &str) {
    println!(
        "## Kind-Specific Preferred Top-Level `{}`",
        escape_inline(name)
    );
    println!();
    println!("Generic top-level preferred lookup is a cross-kind conflict/debug view. Use these kind-specific rows when the expected declaration kind is known.");
    println!();
    print_kind_specific_row(
        index,
        "Class",
        index.classes_by_name(name),
        index.preferred_classes_by_name(name).first().copied(),
    );
    print_kind_specific_row(
        index,
        "Typedef",
        index.typedefs_by_name(name),
        index.preferred_typedefs_by_name(name).first().copied(),
    );
    print_kind_specific_row(
        index,
        "Function",
        index.functions_by_name(name),
        index.preferred_functions_by_name(name).first().copied(),
    );
    println!();
}

fn print_kind_specific_row(
    index: &SymbolIndex,
    kind: &str,
    symbols: &[GlobalSymbolId],
    preferred: Option<GlobalSymbolId>,
) {
    println!("- {kind}: {} matches", symbols.len());
    if let Some(preferred) = preferred {
        println!("  Preferred:");
        print_indented_member_summary(index, preferred, "  ");
    }
}

fn print_class_member_summary(index: &SymbolIndex, owner: &str, symbols: &[GlobalSymbolId]) {
    if symbols.is_empty() {
        return;
    }

    println!(
        "## Direct Owner-Name Aggregate Members `{}`",
        escape_inline(owner)
    );
    println!();
    println!("This is `direct_members_by_owner()` output for the owner name across all indexed source files. In overlay mode it is not limited to the preferred class declaration.");
    println!();
    let direct_members = index.direct_members_by_owner(owner);
    println!("Members: {}", direct_members.len());
    for id in direct_members.iter().take(MAX_CHILDREN) {
        print_member_summary(index, *id);
    }
    if direct_members.len() > MAX_CHILDREN {
        println!(
            "... {} more members omitted",
            direct_members.len() - MAX_CHILDREN
        );
    }
    println!();

    let all_members = index.members_for_class_including_bases(owner);
    let inherited_members = all_members
        .iter()
        .skip(direct_members.len())
        .copied()
        .collect::<Vec<_>>();
    println!(
        "## Owner-Name Aggregate Members Including Bases `{}`",
        escape_inline(owner)
    );
    println!();
    println!("Raw all-candidates view for debugging. This uses exact owner/base names and can include members from multiple source kinds when an overlay is indexed.");
    println!();
    println!(
        "Members: {} direct, {} inherited/base-chain, {} total",
        direct_members.len(),
        inherited_members.len(),
        all_members.len()
    );
    for id in inherited_members.iter().take(MAX_CHILDREN) {
        print_member_summary(index, *id);
    }
    if inherited_members.len() > MAX_CHILDREN {
        println!(
            "... {} more inherited/base-chain members omitted",
            inherited_members.len() - MAX_CHILDREN
        );
    }
    println!();

    let completion = index.completion_members_for_class(owner);
    print_completion_lookup(
        index,
        owner,
        "Completion Owner-Name Aggregate Members",
        "Completion-ready view over the same owner-name aggregate candidates; it de-duplicates by member key but does not semantically merge modded classes. This remains a raw debug view, not the future editor completion truth.",
        &completion,
    );

    let preferred_completion = index.completion_members_for_preferred_class(owner);
    print_completion_lookup(
        index,
        owner,
        "Preferred-Class Overlay Completion Members",
        "Future editor-facing completion path. It starts from preferred class declarations, intentionally includes lower-priority same-owner overlay members, then appends exact-name base-chain members.",
        &preferred_completion,
    );

    for class_id in symbols.iter().take(MAX_MATCHES) {
        let Some(class_symbol) = index.symbol(*class_id) else {
            continue;
        };
        let Some(class_name) = class_symbol.name.as_deref() else {
            continue;
        };
        let fields = index
            .members_by_owner(class_name)
            .iter()
            .filter_map(|id| index.symbol(*id))
            .filter(|symbol| symbol.kind == SymbolKind::Field)
            .count();
        println!(
            "- Class `{}` direct fields {} direct members {} inherited/base-chain members {} raw total {} completion visible {} shadow groups {}",
            escape_inline(class_name),
            fields,
            index.direct_members_by_owner(class_name).len(),
            index
                .members_for_class_including_bases(class_name)
                .len()
                .saturating_sub(index.direct_members_by_owner(class_name).len()),
            index.members_for_class_including_bases(class_name).len(),
            index.completion_members_for_class(class_name).members.len(),
            index
                .completion_members_for_class(class_name)
                .shadowed_groups
                .len()
        );
    }
    println!();
}

fn print_completion_lookup(
    index: &SymbolIndex,
    owner: &str,
    heading: &str,
    description: &str,
    completion: &reforger_language_server::index::CompletionMemberLookup,
) {
    println!("## {heading} `{}`", escape_inline(owner));
    println!();
    println!("{description}");
    println!();
    println!(
        "Members: {} visible, {} raw candidates, {} shadow groups",
        completion.members.len(),
        completion.raw_candidates.len(),
        completion.shadowed_groups.len()
    );
    for id in completion.members.iter().take(MAX_CHILDREN) {
        print_member_summary(index, *id);
    }
    if completion.members.len() > MAX_CHILDREN {
        println!(
            "... {} more completion members omitted",
            completion.members.len() - MAX_CHILDREN
        );
    }
    println!();

    if !completion.shadowed_groups.is_empty() {
        println!("### Shadowed Member Groups");
        println!();
        for group in completion.shadowed_groups.iter().take(MAX_CHILDREN) {
            println!("- `{}`", escape_inline(&group.key));
            println!("  Kept:");
            print_indented_member_summary(index, group.kept, "  ");
            println!("  Hidden:");
            for hidden in &group.shadowed {
                print_indented_member_summary(index, *hidden, "  ");
            }
        }
        if completion.shadowed_groups.len() > MAX_CHILDREN {
            println!(
                "... {} more shadow groups omitted",
                completion.shadowed_groups.len() - MAX_CHILDREN
            );
        }
        println!();
    }
}

fn print_method_signatures(index: &SymbolIndex, symbols: &[GlobalSymbolId]) {
    if symbols.is_empty() {
        return;
    }

    println!("### Signatures");
    println!();
    for id in symbols.iter().take(MAX_MATCHES) {
        if let Some(signature) = index.callable_signature(*id) {
            println!("- `{}`", escape_inline(&signature));
        }
    }
    if symbols.len() > MAX_MATCHES {
        println!(
            "... {} more signatures omitted",
            symbols.len() - MAX_MATCHES
        );
    }
    println!();
}

fn print_symbol(index: &SymbolIndex, id: GlobalSymbolId) {
    let Some(symbol) = index.symbol(id) else {
        println!("- Missing symbol {:?}", id);
        return;
    };
    let Some(file) = index.file(id.file_id) else {
        println!("- Missing file {:?}", id.file_id);
        return;
    };

    println!(
        "- {} `{}` file {} symbol {} source `{}` priority {} path `{}` span {}..{} selection {}..{}{}",
        kind_name(symbol.kind),
        display_symbol_name(symbol),
        id.file_id.0,
        id.symbol_id.0,
        file.metadata.kind.as_str(),
        file.metadata.priority,
        display_path(file),
        symbol.span.start,
        symbol.span.end,
        symbol.selection_span.start,
        symbol.selection_span.end,
        detail_text(index, symbol),
    );

    let children = index.children(id);
    if !children.is_empty() {
        println!("  Children: {}", children.len());
        for child_id in children.iter().take(MAX_CHILDREN) {
            if let Some(child) = index.symbol(*child_id) {
                println!(
                    "  - {} `{}` file {} symbol {}{}",
                    kind_name(child.kind),
                    display_symbol_name(child),
                    child_id.file_id.0,
                    child_id.symbol_id.0,
                    detail_text(index, child)
                );
            }
        }
        if children.len() > MAX_CHILDREN {
            println!(
                "  - ... {} more children omitted",
                children.len() - MAX_CHILDREN
            );
        }
    }
    println!();
}

fn print_member_summary(index: &SymbolIndex, id: GlobalSymbolId) {
    let Some(symbol) = index.symbol(id) else {
        return;
    };
    println!(
        "- {} `{}`{}",
        kind_name(symbol.kind),
        display_symbol_name(symbol),
        detail_text(index, symbol)
    );
}

fn print_indented_member_summary(index: &SymbolIndex, id: GlobalSymbolId, indent: &str) {
    let Some(symbol) = index.symbol(id) else {
        return;
    };
    let path = id.file_id.0.to_string() + ":" + &id.symbol_id.0.to_string();
    println!(
        "{}- {} `{}` {}{}",
        indent,
        kind_name(symbol.kind),
        display_symbol_name(symbol),
        path,
        detail_text(index, symbol)
    );
}

fn query_label(query: &Query) -> String {
    match query {
        Query::Name(name) => format!("--name {name}"),
        Query::TopLevel(name) => format!("--top-level {name}"),
        Query::Class(name) => format!("--class {name}"),
        Query::Typedef(name) => format!("--typedef {name}"),
        Query::Function(name) => format!("--function {name}"),
        Query::Method { owner, name } => format!("--method {owner} {name}"),
    }
}

fn display_symbol_name(symbol: &IndexedSymbol) -> String {
    symbol
        .name
        .as_deref()
        .map(escape_inline)
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn display_path(file: &reforger_language_server::index::IndexedFile) -> String {
    file.metadata
        .relative_path
        .as_ref()
        .or(file.metadata.absolute_path.as_ref())
        .map(|path| escape_inline(&path.display().to_string()))
        .unwrap_or_else(|| "<unknown-path>".to_string())
}

fn detail_text(index: &SymbolIndex, symbol: &IndexedSymbol) -> String {
    let mut values = Vec::new();
    if let Some(signature) = index.callable_signature(symbol.id) {
        push_detail(&mut values, "signature", Some(&signature));
    }
    push_detail(&mut values, "type", symbol.detail.type_text.as_deref());
    push_detail(
        &mut values,
        "return",
        symbol.detail.return_type_text.as_deref(),
    );
    push_detail(&mut values, "base", symbol.detail.base_type.as_deref());
    push_detail(
        &mut values,
        "default",
        symbol.detail.default_text.as_deref(),
    );
    push_detail(
        &mut values,
        "enum_value",
        symbol.detail.enum_value_text.as_deref(),
    );

    if values.is_empty() {
        String::new()
    } else {
        format!(" detail {}", values.join(" "))
    }
}

fn push_detail(values: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        values.push(format!("{label}: `{}`", escape_inline(value)));
    }
}

fn kind_name(kind: SymbolKind) -> &'static str {
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
    }
}

fn escape_inline(value: &str) -> String {
    value.replace('`', "\\`").replace('\n', "\\n")
}
