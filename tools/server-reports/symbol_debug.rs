use reforger_language_server::lexer::TextSpan;
use reforger_language_server::model::{
    source_category_for_path, SourceCategory, SourceFileMetadata, SourceKind,
    SOURCE_PRIORITY_FIXTURE, SOURCE_PRIORITY_WORKSPACE,
};
use reforger_language_server::parser::parse_source;
use reforger_language_server::semantic_file::{
    SemanticDeclaration, SemanticDeclarationId, SemanticDeclarationKind, SemanticFile, SemanticText,
};
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
    let metadata = file_metadata(&file);
    let semantic_file = SemanticFile::build(&source, &parse);

    println!("# Symbol Debug");
    println!();
    println!("File: `{}`", file.display());
    println!("Source kind: `{}`", metadata.kind.as_str());
    println!(
        "Absolute path: `{}`",
        display_optional_path(&metadata.absolute_path)
    );
    println!(
        "Root path: `{}`",
        display_optional_path(&metadata.root_path)
    );
    println!(
        "Relative path: `{}`",
        display_optional_path(&metadata.relative_path)
    );
    println!("Source priority: {}", metadata.priority);
    println!("Bytes: {}", source.len());
    println!("Parse diagnostics: {}", parse.diagnostics.len());
    println!("Symbols: {}", semantic_file.declarations().len());
    println!(
        "Non-declaration callable fragments: {}",
        semantic_file.non_declaration_callable_fragments()
    );
    println!();

    if let Some(symbol) = args.symbol {
        print_symbol_filter(&semantic_file, &source, &symbol);
    } else if let Some(line) = args.line {
        print_line_filter(&semantic_file, &source, line);
    } else {
        println!("## Symbol Tree");
        println!();
        for record in semantic_file
            .declarations()
            .iter()
            .filter(|record| record.parent.is_none())
        {
            print_tree(&semantic_file, &source, record, 0);
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
            virtual_source: None,
            root_path: Some(root.clone()),
            relative_path: file.strip_prefix(&root).ok().map(Path::to_path_buf),
            priority: SOURCE_PRIORITY_FIXTURE,
        };
    }

    SourceFileMetadata {
        kind: SourceKind::Workspace,
        category: SourceCategory::Workspace,
        absolute_path: Some(file.to_path_buf()),
        virtual_source: None,
        root_path: file.starts_with(&root).then_some(root.clone()),
        relative_path: file.strip_prefix(&root).ok().map(Path::to_path_buf),
        priority: SOURCE_PRIORITY_WORKSPACE,
    }
}

fn print_symbol_filter(semantic_file: &SemanticFile, source: &str, symbol: &str) {
    println!("## Symbol `{}`", escape_inline(symbol));
    println!();
    let matches = semantic_file
        .declarations()
        .iter()
        .filter(|record| record.name.as_ref().is_some_and(|name| name.text == symbol))
        .collect::<Vec<_>>();

    if matches.is_empty() {
        println!("No matching symbols.");
        return;
    }

    for record in matches {
        print_record_context(semantic_file, source, record);
    }
}

fn print_line_filter(semantic_file: &SemanticFile, source: &str, line: usize) {
    println!("## Line {line}");
    println!();
    let Some((line_start, line_end)) = line_span(source, line) else {
        println!("Line is outside the file.");
        return;
    };

    let matches = semantic_file
        .declarations()
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
        print_record_context(semantic_file, source, record);
    }
}

fn print_record_context(semantic_file: &SemanticFile, source: &str, record: &SemanticDeclaration) {
    println!("### {} `{}`", kind_name(record.kind), display_name(record));
    println!();
    println!("Parent chain:");
    for ancestor in parent_chain(semantic_file, record) {
        println!("  - {}", summary(source, ancestor));
    }
    println!("Record:");
    println!("  - {}", summary(source, record));

    let children = semantic_file
        .declarations()
        .iter()
        .filter(|child| child.parent == Some(record.id))
        .collect::<Vec<_>>();
    if !children.is_empty() {
        println!("Children:");
        for child in children {
            println!("  - {}", summary(source, child));
        }
    }
    println!();
}

fn print_tree(
    semantic_file: &SemanticFile,
    source: &str,
    record: &SemanticDeclaration,
    depth: usize,
) {
    let indent = "  ".repeat(depth);
    println!("{indent}- {}", summary(source, record));
    for child in semantic_file
        .declarations()
        .iter()
        .filter(|child| child.parent == Some(record.id))
    {
        print_tree(semantic_file, source, child, depth + 1);
    }
}

fn parent_chain<'a>(
    semantic_file: &'a SemanticFile,
    record: &SemanticDeclaration,
) -> Vec<&'a SemanticDeclaration> {
    let mut chain = Vec::new();
    let mut current = record.parent;
    let mut seen = BTreeSet::new();
    while let Some(id) = current {
        if !seen.insert(id.0) {
            break;
        }
        let Some(parent) = semantic_file.declaration(id) else {
            break;
        };
        chain.push(parent);
        current = parent.parent;
    }
    chain.reverse();
    chain
}

fn summary(source: &str, record: &SemanticDeclaration) -> String {
    format!(
        "{} `{}` id {} parent `{}` decl {} selection {}{} attrs {} modifiers `{}` docs {} preview `{}`",
        kind_name(record.kind),
        display_name(record),
        record.id.0,
        display_parent(record.parent),
        display_location(source, record.span),
        display_location(source, record.selection_span),
        display_detail(record),
        display_attributes(&record.attributes),
        display_texts(&record.modifiers),
        record.doc_comments.len(),
        doc_preview(record),
    )
}

fn kind_name(kind: SemanticDeclarationKind) -> &'static str {
    match kind {
        SemanticDeclarationKind::Class => "Class",
        SemanticDeclarationKind::TypeParameter => "TypeParameter",
        SemanticDeclarationKind::Enum => "Enum",
        SemanticDeclarationKind::EnumMember => "EnumMember",
        SemanticDeclarationKind::Typedef => "Typedef",
        SemanticDeclarationKind::Function => "Function",
        SemanticDeclarationKind::GlobalField => "GlobalField",
        SemanticDeclarationKind::Field => "Field",
        SemanticDeclarationKind::Method => "Method",
        SemanticDeclarationKind::Constructor => "Constructor",
        SemanticDeclarationKind::Destructor => "Destructor",
        SemanticDeclarationKind::Parameter => "Parameter",
        SemanticDeclarationKind::LocalVariable => "LocalVariable",
        SemanticDeclarationKind::PreprocessorMacro => "PreprocessorMacro",
    }
}

fn display_name(record: &SemanticDeclaration) -> String {
    record
        .name
        .as_ref()
        .map(|name| escape_inline(&name.text))
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn display_parent(parent: Option<SemanticDeclarationId>) -> String {
    parent
        .map(|id| id.0.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn display_detail(record: &SemanticDeclaration) -> String {
    let mut values = Vec::new();
    push_detail(&mut values, "type", record.detail.type_text.as_ref());
    push_detail(&mut values, "return", record.detail.return_type.as_ref());
    push_detail(&mut values, "base", record.detail.base_type.as_ref());
    push_detail(&mut values, "default", record.detail.default_value.as_ref());
    push_detail(&mut values, "enum_value", record.detail.enum_value.as_ref());

    if values.is_empty() {
        String::new()
    } else {
        format!(" {}", values.join(" "))
    }
}

fn push_detail(values: &mut Vec<String>, label: &str, text: Option<&SemanticText>) {
    if let Some(text) = text {
        values.push(format!("{label} `{}`", escape_inline(&text.text)));
    }
}

fn display_texts(values: &[SemanticText]) -> String {
    let text = values
        .iter()
        .map(|value| escape_inline(&value.text))
        .collect::<Vec<_>>();
    if text.is_empty() {
        "<none>".to_string()
    } else {
        text.join(" ")
    }
}

fn display_attributes(attributes: &[SemanticText]) -> String {
    let names = attributes
        .iter()
        .map(|attribute| escape_inline(attribute_name(&attribute.text)))
        .collect::<Vec<_>>();
    if names.is_empty() {
        "<none>".to_string()
    } else {
        format!("[{}]", names.join(", "))
    }
}

fn attribute_name(text: &str) -> &str {
    text.trim()
        .trim_start_matches('[')
        .split(['(', ']'])
        .next()
        .unwrap_or(text)
        .trim()
}

fn doc_preview(record: &SemanticDeclaration) -> String {
    record
        .doc_comments
        .first()
        .map(|comment| doc_text_preview(&comment.text))
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

fn display_location(source: &str, span: TextSpan) -> String {
    let (line, column) = line_column(source, span.start);
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
