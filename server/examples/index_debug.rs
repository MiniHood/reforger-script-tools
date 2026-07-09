use reforger_language_server::index::{GlobalSymbolId, IndexedSymbol, SymbolIndex};
use reforger_language_server::index_build::{
    build_index, IndexBuildConfig, IndexBuildCounts, IndexSourceRoot,
};
use reforger_language_server::index_query::{
    EditorCompletionCandidate, EditorCompletionMembers, IndexQuery,
};
use reforger_language_server::model::{
    SourceCategory, SourceKind, SymbolKind, SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE,
};
use reforger_language_server::symbol_display::{SymbolDisplay, SymbolDisplayInfo};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_MATCHES: usize = 100;
const MAX_CHILDREN: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShadowProvenance {
    PreprocessorBranchDuplicate,
    PreprocessorPrototypeDuplicate,
    PrototypeDeclarationBlock,
    DocsDoxygenOnlySource,
    GeneratedSourceOverlap,
    ExpectedInheritedBaseShadow,
    Unknown,
}

struct Args {
    scripts_path: PathBuf,
    workspace_path: Option<PathBuf>,
    query: Query,
    limit: usize,
    member_filter: Option<String>,
    symbol_filter: Option<String>,
    show_docs: bool,
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

    match &args.query {
        Query::Name(name) => print_query_results(
            &index,
            &args,
            "All Symbols By Name",
            &name,
            index.symbols_for_name(&name),
            index.preferred_symbols_for_name(&name).first().copied(),
        ),
        Query::TopLevel(name) => {
            print_query_results(
                &index,
                &args,
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
                &args,
                "Classes By Name",
                &name,
                symbols,
                index.preferred_classes_by_name(&name).first().copied(),
            );
            print_class_member_summary(&index, &args, &name, symbols);
        }
        Query::Typedef(name) => {
            let symbols = index.typedefs_by_name(&name);
            print_query_results(
                &index,
                &args,
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
                &args,
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
                &args,
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
    let mut limit = MAX_CHILDREN;
    let mut member_filter: Option<String> = None;
    let mut symbol_filter: Option<String> = None;
    let mut show_docs = false;

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
            "--limit" => {
                let value = take_value(&mut args, "--limit")?;
                limit = value.parse::<usize>().map_err(|error| {
                    format!("--limit requires a positive integer, got `{value}`: {error}")
                })?;
                if limit == 0 {
                    return Err("--limit requires a positive integer".to_string());
                }
            }
            "--member" => {
                member_filter = Some(take_value(&mut args, "--member")?);
            }
            "--symbol" => {
                symbol_filter = Some(take_value(&mut args, "--symbol")?);
            }
            "--show-docs" => {
                show_docs = true;
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
        limit,
        member_filter,
        symbol_filter,
        show_docs,
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
    println!("Filters:");
    println!("  --limit <n>");
    println!("  --member <name>    only with --class; filters member-heavy sections");
    println!("  --symbol <name>    filters printed symbols/candidates by exact label");
    println!("  --show-docs        prints raw doc-comment text for matched symbols");
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
    args: &Args,
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
        if symbol_matches_filters(index, args, id) || matches!(&args.query, Query::Class(_)) {
            if !symbol_matches_filters(index, args, id) {
                println!("Preferred class anchor shown despite `--symbol` filter.");
                println!();
            }
            print_symbol(index, args, id);
        } else {
            println!("Preferred match hidden by `--symbol` filter.");
            println!();
        }
    }

    println!("### All Matches");
    println!();
    let filtered = symbols
        .iter()
        .copied()
        .filter(|id| symbol_matches_filters(index, args, *id))
        .collect::<Vec<_>>();
    let limit = args.limit.min(MAX_MATCHES);
    for id in filtered.iter().take(limit) {
        print_symbol(index, args, *id);
    }
    if filtered.len() > limit {
        println!("... {} more matches omitted", filtered.len() - limit);
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
        print_indented_member_summary(index, preferred, "  ", false);
    }
}

fn print_class_member_summary(
    index: &SymbolIndex,
    args: &Args,
    owner: &str,
    symbols: &[GlobalSymbolId],
) {
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
    let direct_members_all = index.direct_members_by_owner(owner);
    let direct_members = filter_member_ids(index, args, direct_members_all);
    println!(
        "Members: {} shown / {} total",
        direct_members.len(),
        direct_members_all.len()
    );
    for id in direct_members.iter().take(args.limit) {
        print_member_summary(index, *id, args.show_docs);
    }
    if direct_members.len() > args.limit {
        println!(
            "... {} more members omitted",
            direct_members.len() - args.limit
        );
    }
    println!();

    let all_members = index.members_for_class_including_bases(owner);
    let inherited_members = all_members
        .iter()
        .skip(index.direct_members_by_owner(owner).len())
        .copied()
        .collect::<Vec<_>>();
    let inherited_total = inherited_members.len();
    let inherited_members = filter_member_ids(index, args, &inherited_members);
    println!(
        "## Owner-Name Aggregate Members Including Bases `{}`",
        escape_inline(owner)
    );
    println!();
    println!("Raw all-candidates view for debugging. This uses exact owner/base names and can include members from multiple source kinds when an overlay is indexed.");
    println!();
    println!(
        "Members: {} direct shown / {} direct total, {} inherited/base-chain shown / {} inherited/base-chain total, {} raw total",
        direct_members.len(),
        direct_members_all.len(),
        inherited_members.len(),
        inherited_total,
        all_members.len()
    );
    for id in inherited_members.iter().take(args.limit) {
        print_member_summary(index, *id, args.show_docs);
    }
    if inherited_members.len() > args.limit {
        println!(
            "... {} more inherited/base-chain members omitted",
            inherited_members.len() - args.limit
        );
    }
    println!();

    let completion = index.completion_members_for_class(owner);
    print_completion_lookup(
        index,
        args,
        owner,
        "Completion Owner-Name Aggregate Members",
        "Completion-ready view over the same owner-name aggregate candidates; it de-duplicates by member key but does not semantically merge modded classes. This remains a raw debug view, not the future editor completion truth.",
        &completion,
    );

    let preferred_completion = index.completion_members_for_preferred_class(owner);
    print_completion_lookup(
        index,
        args,
        owner,
        "Raw Preferred-Class Overlay Completion Members",
        "Raw index view over preferred-class overlay candidates. It does not apply the editor-facing source-category policy; use the IndexQuery section below for future editor behavior.",
        &preferred_completion,
    );

    let editor_completion = IndexQuery::new(index).completion_members_for_class(owner);
    print_editor_completion_lookup(index, args, owner, &editor_completion);

    for class_id in symbols.iter().take(args.limit.min(MAX_MATCHES)) {
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
        let direct_member_count = index.direct_members_by_owner(class_name).len();
        let raw_members = index.members_for_class_including_bases(class_name);
        let inherited_member_count = raw_members.len().saturating_sub(direct_member_count);
        let raw_completion = index.completion_members_for_class(class_name);
        let editor_completion = IndexQuery::new(index).completion_members_for_class(class_name);
        println!(
            "- Class `{}` direct fields {} direct members {} inherited/base-chain members {} raw total {} raw aggregate completion {} raw aggregate shadow groups {} editor completion {} editor shadow groups {}",
            escape_inline(class_name),
            fields,
            direct_member_count,
            inherited_member_count,
            raw_members.len(),
            raw_completion.members.len(),
            raw_completion.shadowed_groups.len(),
            editor_completion.candidates.len(),
            editor_completion.shadowed_groups.len()
        );
    }
    println!();
}

fn print_editor_completion_lookup(
    index: &SymbolIndex,
    args: &Args,
    owner: &str,
    completion: &EditorCompletionMembers,
) {
    println!(
        "## IndexQuery Editor Completion Members `{}`",
        escape_inline(owner)
    );
    println!();
    println!("Future editor-facing completion facade. It applies source-category policy, preferred class selection, duplicate member de-duplication, source priority, and callable-form preference.");
    println!();
    let candidates = completion
        .candidates
        .iter()
        .filter(|candidate| candidate_matches_filters(args, candidate))
        .collect::<Vec<_>>();
    println!(
        "Members: {} shown / {} visible, {} raw candidates, {} shadow groups",
        candidates.len(),
        completion.candidates.len(),
        completion.raw_candidates.len(),
        completion.shadowed_groups.len()
    );
    for candidate in candidates.iter().take(args.limit) {
        print_editor_candidate(candidate);
    }
    if candidates.len() > args.limit {
        println!(
            "... {} more editor completion members omitted",
            candidates.len() - args.limit
        );
    }
    println!();

    if !completion.shadowed_groups.is_empty() {
        println!("### Editor Shadowed Member Groups");
        println!();
        let groups = completion
            .shadowed_groups
            .iter()
            .filter(|group| shadow_group_matches_filters(index, args, group.kept, &group.shadowed))
            .collect::<Vec<_>>();
        println!(
            "Shadow groups: {} shown / {} total",
            groups.len(),
            completion.shadowed_groups.len()
        );
        for group in groups.iter().take(args.limit) {
            println!("- `{}`", escape_inline(&group.key));
            println!("  Kept:");
            print_indented_member_summary(index, group.kept, "  ", args.show_docs);
            println!("  Hidden:");
            for hidden in &group.shadowed {
                print_indented_member_summary(index, *hidden, "  ", args.show_docs);
            }
        }
        if groups.len() > args.limit {
            println!(
                "... {} more editor shadow groups omitted",
                groups.len() - args.limit
            );
        }
        println!();
    }
}

fn print_editor_candidate(candidate: &EditorCompletionCandidate) {
    let path = candidate
        .relative_path
        .as_ref()
        .or(candidate.absolute_path.as_ref())
        .map(|path| escape_inline(&path.display().to_string()))
        .unwrap_or_else(|| "<unknown-path>".to_string());
    println!(
        "- {} `{}` origin `{:?}` category `{}` source `{}` priority {} path `{}`{}{}{}{}",
        kind_name(candidate.kind),
        candidate
            .name
            .as_deref()
            .map(escape_inline)
            .unwrap_or_else(|| "<unknown>".to_string()),
        candidate.origin,
        candidate.source_category.as_str(),
        candidate.source_kind.as_str(),
        candidate.source_priority,
        path,
        editor_callable_form_suffix(candidate),
        editor_conditional_context_suffix(candidate),
        editor_detail_suffix(candidate),
        editor_presentation_suffix(&candidate.display)
    );
}

fn editor_callable_form_suffix(candidate: &EditorCompletionCandidate) -> String {
    candidate
        .callable_form
        .map(|form| format!(" form `{}`", form.as_str()))
        .unwrap_or_default()
}

fn editor_conditional_context_suffix(candidate: &EditorCompletionCandidate) -> String {
    if candidate.conditional_context.is_empty() {
        return " condition `unconditional`".to_string();
    }

    let context = candidate
        .conditional_context
        .iter()
        .map(|branch| {
            branch
                .condition
                .as_deref()
                .map(|condition| format!("{} {}", branch.kind.as_str(), condition))
                .unwrap_or_else(|| branch.kind.as_str().to_string())
        })
        .collect::<Vec<_>>()
        .join(" > ");
    format!(" condition `{}`", escape_inline(&context))
}

fn editor_detail_suffix(candidate: &EditorCompletionCandidate) -> String {
    if let Some(signature) = &candidate.signature {
        return format!(" detail signature: `{}`", escape_inline(signature));
    }
    if let Some(detail) = &candidate.detail {
        return format!(" detail `{}`", escape_inline(detail));
    }
    String::new()
}

fn editor_presentation_suffix(display: &SymbolDisplayInfo) -> String {
    let mut parts = Vec::new();
    if !display.modifiers.is_empty() {
        parts.push(format!(
            " modifiers `{}`",
            escape_inline(&display.modifiers.join(" "))
        ));
    }
    let attribute_names = display
        .attributes
        .iter()
        .filter_map(|attribute| attribute.name.as_deref())
        .collect::<Vec<_>>();
    if !attribute_names.is_empty() {
        parts.push(format!(
            " attributes `{}`",
            escape_inline(&attribute_names.join(", "))
        ));
    }
    if let Some(preview) = &display.documentation_preview {
        parts.push(format!(" docs `{}`", escape_inline(preview)));
    }
    parts.join("")
}

fn print_completion_lookup(
    index: &SymbolIndex,
    args: &Args,
    owner: &str,
    heading: &str,
    description: &str,
    completion: &reforger_language_server::index::CompletionMemberLookup,
) {
    println!("## {heading} `{}`", escape_inline(owner));
    println!();
    println!("{description}");
    println!();
    let members = filter_member_ids(index, args, &completion.members);
    println!(
        "Members: {} shown / {} visible, {} raw candidates, {} shadow groups",
        members.len(),
        completion.members.len(),
        completion.raw_candidates.len(),
        completion.shadowed_groups.len()
    );
    for id in members.iter().take(args.limit) {
        print_member_summary(index, *id, args.show_docs);
    }
    if members.len() > args.limit {
        println!(
            "... {} more completion members omitted",
            members.len() - args.limit
        );
    }
    println!();

    if !completion.shadowed_groups.is_empty() {
        println!("### Shadowed Member Groups");
        println!();
        let groups = completion
            .shadowed_groups
            .iter()
            .filter(|group| shadow_group_matches_filters(index, args, group.kept, &group.shadowed))
            .collect::<Vec<_>>();
        println!(
            "Shadow groups: {} shown / {} total",
            groups.len(),
            completion.shadowed_groups.len()
        );
        for group in groups.iter().take(args.limit) {
            println!(
                "- `{}` cause `{}`",
                escape_inline(&group.key),
                shadow_provenance_label(classify_shadow_group(index, group))
            );
            println!("  Kept:");
            print_indented_member_summary(index, group.kept, "  ", args.show_docs);
            println!("  Hidden:");
            for hidden in &group.shadowed {
                print_indented_member_summary(index, *hidden, "  ", args.show_docs);
            }
        }
        if groups.len() > args.limit {
            println!(
                "... {} more shadow groups omitted",
                groups.len() - args.limit
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

fn print_symbol(index: &SymbolIndex, args: &Args, id: GlobalSymbolId) {
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
    println!(
        "  Source category: `{}` editor completion `{}`{}{}",
        file.metadata.category.as_str(),
        if file.metadata.category.is_editor_completion_default() {
            "included"
        } else {
            "excluded"
        },
        callable_form_suffix(symbol),
        conditional_context_suffix(symbol)
    );
    if let Some(display) = SymbolDisplay::for_symbol(index, id) {
        print_display_metadata(&display, "  ", args.show_docs);
    }

    let children = index.children(id);
    if !children.is_empty() {
        let children = filter_member_ids(index, args, children);
        println!("  Children: {}", children.len());
        for child_id in children.iter().take(args.limit) {
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
        if children.len() > args.limit {
            println!(
                "  - ... {} more children omitted",
                children.len() - args.limit
            );
        }
    }
    println!();
}

fn print_member_summary(index: &SymbolIndex, id: GlobalSymbolId, show_docs: bool) {
    let Some(symbol) = index.symbol(id) else {
        return;
    };
    println!(
        "- {} `{}`{}{}",
        kind_name(symbol.kind),
        display_symbol_name(symbol),
        detail_text(index, symbol),
        SymbolDisplay::for_symbol(index, id)
            .map(|display| editor_presentation_suffix(&display))
            .unwrap_or_default()
    );
    print_raw_doc_comments(index, id, "  ", show_docs);
}

fn print_indented_member_summary(
    index: &SymbolIndex,
    id: GlobalSymbolId,
    indent: &str,
    show_docs: bool,
) {
    let Some(symbol) = index.symbol(id) else {
        return;
    };
    let path = id.file_id.0.to_string() + ":" + &id.symbol_id.0.to_string();
    println!(
        "{}- {} `{}` {} category `{}` editor `{}`{}{}{}",
        indent,
        kind_name(symbol.kind),
        display_symbol_name(symbol),
        path,
        source_category_for_symbol(index, id).as_str(),
        if source_category_for_symbol(index, id).is_editor_completion_default() {
            "included"
        } else {
            "excluded"
        },
        callable_form_suffix(symbol),
        conditional_context_suffix(symbol),
        detail_text(index, symbol)
    );
    print_raw_doc_comments(index, id, indent, show_docs);
}

fn print_raw_doc_comments(index: &SymbolIndex, id: GlobalSymbolId, indent: &str, show_docs: bool) {
    if !show_docs {
        return;
    }
    let Some(display) = SymbolDisplay::for_symbol(index, id) else {
        return;
    };
    for (index, comment) in display.doc_comments.iter().enumerate() {
        println!(
            "{}Doc comment {}: `{}`",
            indent,
            index + 1,
            escape_inline(&comment.text)
        );
    }
}

fn print_display_metadata(display: &SymbolDisplayInfo, indent: &str, show_docs: bool) {
    if !display.modifiers.is_empty() {
        println!(
            "{}Modifiers: `{}`",
            indent,
            escape_inline(&display.modifiers.join(" "))
        );
    }
    if !display.attributes.is_empty() {
        let attributes = display
            .attributes
            .iter()
            .map(|attribute| {
                attribute
                    .name
                    .as_deref()
                    .unwrap_or(attribute.text.as_str())
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(", ");
        println!("{}Attributes: `{}`", indent, escape_inline(&attributes));
    }
    if let Some(preview) = &display.documentation_preview {
        println!("{}Docs preview: `{}`", indent, escape_inline(preview));
    }
    if !display.doc_comments.is_empty() {
        println!("{}Doc comments: {}", indent, display.doc_comments.len());
        if show_docs {
            for (index, comment) in display.doc_comments.iter().enumerate() {
                println!(
                    "{}Doc comment {}: `{}`",
                    indent,
                    index + 1,
                    escape_inline(&comment.text)
                );
            }
        }
    }
}

fn filter_member_ids(
    index: &SymbolIndex,
    args: &Args,
    ids: &[GlobalSymbolId],
) -> Vec<GlobalSymbolId> {
    ids.iter()
        .copied()
        .filter(|id| member_matches_filters(index, args, *id))
        .collect()
}

fn member_matches_filters(index: &SymbolIndex, args: &Args, id: GlobalSymbolId) -> bool {
    if !symbol_matches_filters(index, args, id) {
        return false;
    }
    let name = index.symbol(id).and_then(|symbol| symbol.name.as_deref());
    if args
        .member_filter
        .as_deref()
        .is_some_and(|filter| name != Some(filter))
    {
        return false;
    }
    true
}

fn symbol_matches_filters(index: &SymbolIndex, args: &Args, id: GlobalSymbolId) -> bool {
    let Some(symbol) = index.symbol(id) else {
        return false;
    };
    let name = symbol.name.as_deref();
    if args
        .symbol_filter
        .as_deref()
        .is_some_and(|filter| name != Some(filter))
    {
        return false;
    }
    true
}

fn candidate_matches_filters(args: &Args, candidate: &EditorCompletionCandidate) -> bool {
    let name = candidate.name.as_deref();
    if args
        .symbol_filter
        .as_deref()
        .is_some_and(|filter| name != Some(filter))
    {
        return false;
    }
    if args
        .member_filter
        .as_deref()
        .is_some_and(|filter| name != Some(filter))
    {
        return false;
    }
    true
}

fn shadow_group_matches_filters(
    index: &SymbolIndex,
    args: &Args,
    kept: GlobalSymbolId,
    shadowed: &[GlobalSymbolId],
) -> bool {
    member_matches_filters(index, args, kept)
        || shadowed
            .iter()
            .any(|id| member_matches_filters(index, args, *id))
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
    SymbolDisplay::for_symbol(index, symbol.id)
        .and_then(|display| display.detail)
        .map(|detail| format!(" detail `{}`", escape_inline(&detail)))
        .unwrap_or_default()
}

fn callable_form_suffix(symbol: &IndexedSymbol) -> String {
    symbol
        .callable_form
        .map(|form| format!(" form `{}`", form.as_str()))
        .unwrap_or_default()
}

fn conditional_context_suffix(symbol: &IndexedSymbol) -> String {
    if symbol.conditional_context.is_empty() {
        return " condition `unconditional`".to_string();
    }

    let context = symbol
        .conditional_context
        .iter()
        .map(|branch| {
            branch
                .condition
                .as_deref()
                .map(|condition| format!("{} {}", branch.kind.as_str(), condition))
                .unwrap_or_else(|| branch.kind.as_str().to_string())
        })
        .collect::<Vec<_>>()
        .join(" > ");
    format!(" condition `{}`", escape_inline(&context))
}

fn classify_shadow_group(
    index: &SymbolIndex,
    group: &reforger_language_server::index::MemberShadowGroup,
) -> ShadowProvenance {
    let ids = std::iter::once(group.kept)
        .chain(group.shadowed.iter().copied())
        .collect::<Vec<_>>();

    if ids
        .iter()
        .any(|id| source_category_for_symbol(index, *id) == SourceCategory::DocsDoxygen)
        || ids
            .iter()
            .any(|id| file_contains(index, *id, "#ifdef DOXYGEN"))
    {
        return ShadowProvenance::DocsDoxygenOnlySource;
    }

    let kept_owner = owner_name(index, group.kept);
    if group
        .shadowed
        .iter()
        .all(|hidden| owner_name(index, *hidden) != kept_owner)
    {
        return ShadowProvenance::ExpectedInheritedBaseShadow;
    }

    let has_preprocessor = ids
        .iter()
        .any(|id| symbol_has_preprocessor_context(index, *id));
    let has_prototype = ids.iter().any(|id| symbol_looks_like_prototype(index, *id));
    if has_preprocessor && has_prototype {
        return ShadowProvenance::PreprocessorPrototypeDuplicate;
    }
    if has_preprocessor {
        return ShadowProvenance::PreprocessorBranchDuplicate;
    }
    if has_prototype {
        return ShadowProvenance::PrototypeDeclarationBlock;
    }

    let has_generated = ids
        .iter()
        .any(|id| source_category_for_symbol(index, *id) == SourceCategory::Generated);
    let has_non_generated = ids
        .iter()
        .any(|id| source_category_for_symbol(index, *id) != SourceCategory::Generated);
    if has_generated && has_non_generated {
        return ShadowProvenance::GeneratedSourceOverlap;
    }

    ShadowProvenance::Unknown
}

fn owner_name(index: &SymbolIndex, id: GlobalSymbolId) -> Option<&str> {
    index
        .symbol(id)
        .and_then(|symbol| symbol.parent)
        .and_then(|parent| index.symbol(parent))
        .and_then(|parent| parent.name.as_deref())
}

fn source_category_for_symbol(index: &SymbolIndex, id: GlobalSymbolId) -> SourceCategory {
    index
        .file(id.file_id)
        .map(|file| file.metadata.category)
        .unwrap_or(SourceCategory::Unknown)
}

fn file_contains(index: &SymbolIndex, id: GlobalSymbolId, needle: &str) -> bool {
    source_text(index, id).is_some_and(|source| source.contains(needle))
}

fn symbol_has_preprocessor_context(index: &SymbolIndex, id: GlobalSymbolId) -> bool {
    index
        .symbol(id)
        .is_some_and(|symbol| !symbol.conditional_context.is_empty())
}

fn symbol_looks_like_prototype(index: &SymbolIndex, id: GlobalSymbolId) -> bool {
    let Some(symbol) = index.symbol(id) else {
        return false;
    };
    symbol
        .callable_form
        .is_some_and(|form| form.as_str() != "implementation")
}

fn source_text(index: &SymbolIndex, id: GlobalSymbolId) -> Option<String> {
    let path = index
        .file(id.file_id)
        .and_then(|file| file.metadata.absolute_path.as_ref())?;
    fs::read_to_string(path).ok()
}

fn shadow_provenance_label(provenance: ShadowProvenance) -> &'static str {
    match provenance {
        ShadowProvenance::PreprocessorBranchDuplicate => "preprocessor branch duplicate",
        ShadowProvenance::PreprocessorPrototypeDuplicate => {
            "preprocessor branch/prototype duplicate"
        }
        ShadowProvenance::PrototypeDeclarationBlock => "prototype/declaration block",
        ShadowProvenance::DocsDoxygenOnlySource => "docs/Doxygen-only source",
        ShadowProvenance::GeneratedSourceOverlap => "generated/source overlap",
        ShadowProvenance::ExpectedInheritedBaseShadow => "expected inherited/base shadow",
        ShadowProvenance::Unknown => "unknown",
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
