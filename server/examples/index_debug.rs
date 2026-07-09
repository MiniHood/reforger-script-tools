use reforger_language_server::ast::AstSourceFile;
use reforger_language_server::index::{GlobalSymbolId, IndexedSymbol, SymbolIndex};
use reforger_language_server::model::{
    SourceFileMetadata, SourceKind, SymbolCatalog, SymbolKind, SOURCE_PRIORITY_GAME_DATA,
};
use reforger_language_server::parser::parse_source;
use std::borrow::Cow;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_MATCHES: usize = 100;
const MAX_CHILDREN: usize = 20;

struct Args {
    scripts_path: PathBuf,
    query: Query,
}

enum Query {
    Name(String),
    TopLevel(String),
    Class(String),
    Typedef(String),
    Method { owner: String, name: String },
}

#[derive(Default)]
struct Totals {
    files: usize,
    bytes: usize,
    lossy_files: usize,
    parse_diagnostics: usize,
}

fn main() -> Result<(), String> {
    let args = parse_args()?;
    let (index, totals) = build_index(&args.scripts_path)?;

    println!("# Index Debug");
    println!();
    println!("Query: `{}`", query_label(&args.query));
    println!("Scripts: `{}`", args.scripts_path.display());
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
        Query::TopLevel(name) => print_query_results(
            &index,
            "Top-Level Symbols By Name",
            &name,
            index.top_level_symbols_for_name(&name),
            index
                .preferred_top_level_symbols_for_name(&name)
                .first()
                .copied(),
        ),
        Query::Class(name) => {
            let symbols = index.classes_by_name(&name);
            print_query_results(
                &index,
                "Classes By Name",
                &name,
                symbols,
                index.preferred_from_symbols(symbols).first().copied(),
            );
        }
        Query::Typedef(name) => {
            let symbols = index.typedefs_by_name(&name);
            print_query_results(
                &index,
                "Typedefs By Name",
                &name,
                symbols,
                index.preferred_from_symbols(symbols).first().copied(),
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
    let mut query: Option<Query> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scripts" => {
                let Some(value) = args.next() else {
                    return Err("--scripts requires a path".to_string());
                };
                scripts = Some(PathBuf::from(value));
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
    println!("Usage: node tools/index-debug.mjs [--scripts <path>] <query>");
    println!("Queries:");
    println!("  --name <symbol>");
    println!("  --top-level <symbol>");
    println!("  --class <class>");
    println!("  --typedef <typedef>");
    println!("  --method <owner> <method>");
}

fn default_scripts_path() -> PathBuf {
    if let Some(app_data) = env::var_os("APPDATA") {
        PathBuf::from(app_data).join(DEFAULT_STORAGE_RELATIVE_PATH)
    } else {
        PathBuf::from(DEFAULT_STORAGE_RELATIVE_PATH)
    }
}

fn build_index(scripts_path: &Path) -> Result<(SymbolIndex, Totals), String> {
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
    let mut catalogs = Vec::new();

    for file in &files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        totals.files += 1;
        totals.bytes += bytes.len();

        let source = String::from_utf8_lossy(&bytes);
        if matches!(source, Cow::Owned(_)) {
            totals.lossy_files += 1;
        }
        let source: &'static str = Box::leak(source.into_owned().into_boxed_str());

        let parse = parse_source(source);
        totals.parse_diagnostics += parse.diagnostics.len();
        let ast = AstSourceFile::new(source, &parse);
        catalogs.push(SymbolCatalog::from_ast_with_metadata(
            source,
            &ast,
            game_data_metadata(scripts_path, file),
        ));
    }

    Ok((SymbolIndex::from_catalogs(catalogs.iter()), totals))
}

fn game_data_metadata(scripts_path: &Path, file: &Path) -> SourceFileMetadata {
    SourceFileMetadata {
        kind: SourceKind::GameData,
        absolute_path: Some(file.to_path_buf()),
        root_path: Some(scripts_path.to_path_buf()),
        relative_path: Some(
            file.strip_prefix(scripts_path)
                .unwrap_or(file)
                .to_path_buf(),
        ),
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
        detail_text(symbol),
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
                    detail_text(child)
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

fn query_label(query: &Query) -> String {
    match query {
        Query::Name(name) => format!("--name {name}"),
        Query::TopLevel(name) => format!("--top-level {name}"),
        Query::Class(name) => format!("--class {name}"),
        Query::Typedef(name) => format!("--typedef {name}"),
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

fn detail_text(symbol: &IndexedSymbol) -> String {
    let mut values = Vec::new();
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
