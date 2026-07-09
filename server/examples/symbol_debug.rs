use reforger_language_server::ast::AstSourceFile;
use reforger_language_server::lexer::TextSpan;
use reforger_language_server::model::{
    source_category_for_path, SourceCategory, SourceFileMetadata, SourceKind, SymbolCatalog,
    SymbolId, SymbolKind, SymbolRecord, SOURCE_PRIORITY_FIXTURE, SOURCE_PRIORITY_WORKSPACE,
};
use reforger_language_server::parser::parse_source;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

struct Args {
    file: PathBuf,
    symbol: Option<String>,
    line: Option<usize>,
}

fn main() -> Result<(), String> {
    let args = parse_args()?;
    let file = resolve_file(args.file)?;
    let source = fs::read_to_string(&file)
        .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
    let parse = parse_source(&source);
    let ast = AstSourceFile::new(&source, &parse);
    let metadata = file_metadata(&file);
    let catalog = SymbolCatalog::from_ast_with_metadata(&source, &ast, metadata);

    println!("# Symbol Debug");
    println!();
    println!("File: `{}`", file.display());
    println!("Source kind: `{}`", catalog.metadata().kind.as_str());
    println!(
        "Absolute path: `{}`",
        display_optional_path(&catalog.metadata().absolute_path)
    );
    println!(
        "Root path: `{}`",
        display_optional_path(&catalog.metadata().root_path)
    );
    println!(
        "Relative path: `{}`",
        display_optional_path(&catalog.metadata().relative_path)
    );
    println!("Source priority: {}", catalog.metadata().priority);
    println!("Bytes: {}", source.len());
    println!("Parse diagnostics: {}", parse.diagnostics.len());
    println!("Symbols: {}", catalog.records().len());
    println!(
        "Non-declaration callable fragments: {}",
        catalog.non_declaration_callable_fragments()
    );
    println!();

    if let Some(symbol) = args.symbol {
        print_symbol_filter(&catalog, &symbol);
    } else if let Some(line) = args.line {
        print_line_filter(&catalog, &source, line);
    } else {
        println!("## Symbol Tree");
        println!();
        for record in catalog
            .records()
            .iter()
            .filter(|record| record.parent.is_none())
        {
            print_tree(&catalog, record, 0);
        }
    }

    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut file = None;
    let mut symbol = None;
    let mut line = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--file" => {
                let Some(value) = args.next() else {
                    return Err("--file requires a path".to_string());
                };
                file = Some(PathBuf::from(value));
            }
            "--symbol" => {
                let Some(value) = args.next() else {
                    return Err("--symbol requires a name".to_string());
                };
                symbol = Some(value);
            }
            "--line" => {
                let Some(value) = args.next() else {
                    return Err("--line requires a 1-based line number".to_string());
                };
                line = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid --line value: {value}"))?,
                );
            }
            "--help" | "-h" => {
                println!("Usage: node tools/symbol-debug.mjs --file <path> [--symbol <name>] [--line <line>]");
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }

    let Some(file) = file else {
        return Err("--file is required".to_string());
    };

    Ok(Args { file, symbol, line })
}

fn resolve_file(file: PathBuf) -> Result<PathBuf, String> {
    let path = if file.is_absolute() {
        file
    } else {
        repo_root().join(file)
    };

    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("File does not exist: {}", path.display()))
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("server crate should be inside the repository root")
        .to_path_buf()
}

fn file_metadata(file: &Path) -> SourceFileMetadata {
    let root = repo_root();
    let fixture_root = root.join("tools").join("fixtures");
    if file.starts_with(&fixture_root) {
        return SourceFileMetadata {
            kind: SourceKind::Fixture,
            category: source_category_for_path(SourceKind::Fixture, file.strip_prefix(&root).ok()),
            absolute_path: Some(file.to_path_buf()),
            root_path: Some(root.clone()),
            relative_path: file.strip_prefix(&root).ok().map(Path::to_path_buf),
            priority: SOURCE_PRIORITY_FIXTURE,
        };
    }

    SourceFileMetadata {
        kind: SourceKind::Workspace,
        category: SourceCategory::Workspace,
        absolute_path: Some(file.to_path_buf()),
        root_path: file.starts_with(&root).then_some(root.clone()),
        relative_path: file.strip_prefix(&root).ok().map(Path::to_path_buf),
        priority: SOURCE_PRIORITY_WORKSPACE,
    }
}

fn print_symbol_filter(catalog: &SymbolCatalog<'_>, symbol: &str) {
    println!("## Symbol `{}`", escape_inline(symbol));
    println!();
    let matches = catalog
        .records()
        .iter()
        .filter(|record| catalog.record_name(record) == Some(symbol))
        .collect::<Vec<_>>();

    if matches.is_empty() {
        println!("No matching symbols.");
        return;
    }

    for record in matches {
        print_record_context(catalog, record);
    }
}

fn print_line_filter(catalog: &SymbolCatalog<'_>, source: &str, line: usize) {
    println!("## Line {line}");
    println!();
    let Some((line_start, line_end)) = line_span(source, line) else {
        println!("Line is outside the file.");
        return;
    };

    let matches = catalog
        .records()
        .iter()
        .filter(|record| {
            intersects(record.span, line_start, line_end)
                || intersects(record.selection_span, line_start, line_end)
        })
        .collect::<Vec<_>>();

    if matches.is_empty() {
        println!("No symbols touch this line.");
        return;
    }

    for record in matches {
        print_record_context(catalog, record);
    }
}

fn print_record_context(catalog: &SymbolCatalog<'_>, record: &SymbolRecord) {
    println!(
        "### {} `{}`",
        kind_name(record.kind),
        display_name(catalog, record)
    );
    println!();
    println!("Parent chain:");
    for ancestor in parent_chain(catalog, record) {
        println!("  - {}", summary(catalog, ancestor));
    }
    println!("Record:");
    println!("  - {}", summary(catalog, record));

    let children = catalog
        .records()
        .iter()
        .filter(|child| child.parent == Some(record.id))
        .collect::<Vec<_>>();
    if !children.is_empty() {
        println!("Children:");
        for child in children {
            println!("  - {}", summary(catalog, child));
        }
    }
    println!();
}

fn print_tree(catalog: &SymbolCatalog<'_>, record: &SymbolRecord, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{indent}- {}", summary(catalog, record));
    for child in catalog
        .records()
        .iter()
        .filter(|child| child.parent == Some(record.id))
    {
        print_tree(catalog, child, depth + 1);
    }
}

fn parent_chain<'a>(
    catalog: &'a SymbolCatalog<'_>,
    record: &SymbolRecord,
) -> Vec<&'a SymbolRecord> {
    let mut chain = Vec::new();
    let mut current = record.parent;
    let mut seen = BTreeSet::new();
    while let Some(id) = current {
        if !seen.insert(id.0) {
            break;
        }
        let Some(parent) = catalog.record(id) else {
            break;
        };
        chain.push(parent);
        current = parent.parent;
    }
    chain.reverse();
    chain
}

fn summary(catalog: &SymbolCatalog<'_>, record: &SymbolRecord) -> String {
    format!(
        "{} `{}` id {} parent `{}` decl {} selection {}{} attrs {} modifiers `{}` docs {} preview `{}`",
        kind_name(record.kind),
        display_name(catalog, record),
        record.id.0,
        display_parent(record.parent),
        display_location(catalog, record.span),
        display_location(catalog, record.selection_span),
        display_detail(catalog, record),
        display_attributes(catalog, &record.attributes),
        display_spans(catalog, &record.modifiers),
        record.doc_comments.len(),
        doc_preview(catalog, record),
    )
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

fn display_name(catalog: &SymbolCatalog<'_>, record: &SymbolRecord) -> String {
    catalog
        .record_name(record)
        .map(escape_inline)
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn display_parent(parent: Option<SymbolId>) -> String {
    parent
        .map(|id| id.0.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn display_detail(catalog: &SymbolCatalog<'_>, record: &SymbolRecord) -> String {
    let mut values = Vec::new();
    push_detail(catalog, &mut values, "type", record.detail.type_text);
    push_detail(
        catalog,
        &mut values,
        "return",
        record.detail.return_type_text,
    );
    push_detail(catalog, &mut values, "base", record.detail.base_type);
    push_detail(catalog, &mut values, "default", record.detail.default_text);
    push_detail(
        catalog,
        &mut values,
        "enum_value",
        record.detail.enum_value_text,
    );

    if values.is_empty() {
        String::new()
    } else {
        format!(" {}", values.join(" "))
    }
}

fn push_detail(
    catalog: &SymbolCatalog<'_>,
    values: &mut Vec<String>,
    label: &str,
    span: Option<TextSpan>,
) {
    if let Some(span) = span {
        values.push(format!("{label} `{}`", escape_inline(catalog.text(span))));
    }
}

fn display_spans(catalog: &SymbolCatalog<'_>, spans: &[TextSpan]) -> String {
    let text = spans
        .iter()
        .map(|span| escape_inline(catalog.text(*span)))
        .collect::<Vec<_>>();
    if text.is_empty() {
        "<none>".to_string()
    } else {
        text.join(" ")
    }
}

fn display_attributes(catalog: &SymbolCatalog<'_>, spans: &[TextSpan]) -> String {
    let names = spans
        .iter()
        .map(|span| {
            catalog
                .attribute_name(*span)
                .map(escape_inline)
                .unwrap_or_else(|| escape_inline(catalog.text(*span)))
        })
        .collect::<Vec<_>>();
    if names.is_empty() {
        "<none>".to_string()
    } else {
        format!("[{}]", names.join(", "))
    }
}

fn doc_preview(catalog: &SymbolCatalog<'_>, record: &SymbolRecord) -> String {
    record
        .doc_comments
        .first()
        .map(|comment| doc_text_preview(catalog.text(comment.span)))
        .unwrap_or_else(|| "<none>".to_string())
}

fn doc_text_preview(value: &str) -> String {
    let cleaned = value
        .lines()
        .map(clean_doc_line)
        .find(|line| !line.is_empty())
        .unwrap_or_else(|| value.trim().to_string());
    escape_inline(&cleaned)
}

fn clean_doc_line(line: &str) -> String {
    line.trim()
        .trim_start_matches("//!")
        .trim_start_matches("///<")
        .trim_start_matches("//")
        .trim_start_matches("/*!")
        .trim_start_matches("/**")
        .trim_start_matches("/*")
        .trim_start_matches('*')
        .trim_end_matches("*/")
        .trim()
        .to_string()
}

fn display_location(catalog: &SymbolCatalog<'_>, span: TextSpan) -> String {
    let (line, column) = line_column(catalog.source(), span.start);
    format!("{}:{} span {}..{}", line, column, span.start, span.end)
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

fn line_span(source: &str, line: usize) -> Option<(usize, usize)> {
    if line == 0 {
        return None;
    }

    let mut current_line = 1usize;
    let mut line_start = 0usize;
    for (index, value) in source.char_indices() {
        if current_line == line && value == '\n' {
            return Some((line_start, index));
        }
        if value == '\n' {
            current_line += 1;
            line_start = index + 1;
        }
    }

    if current_line == line {
        Some((line_start, source.len()))
    } else {
        None
    }
}

fn intersects(span: TextSpan, start: usize, end: usize) -> bool {
    span.start <= end && span.end >= start
}

fn escape_inline(value: &str) -> String {
    value.replace('`', "\\`").replace('\n', "\\n")
}

fn display_optional_path(path: &Option<PathBuf>) -> String {
    path.as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<none>".to_string())
}
