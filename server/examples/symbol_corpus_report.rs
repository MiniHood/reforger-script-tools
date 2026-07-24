use reforger_language_server::ast::{AstSourceFile, ClassMember, Declaration};
use reforger_language_server::lexer::TextSpan;
use reforger_language_server::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, SymbolCatalog, SymbolKind,
    SymbolRecord, SOURCE_PRIORITY_GAME_DATA,
};
use reforger_language_server::parser::parse_source;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/symbol-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_DUPLICATE_NAMES: usize = 100;
const MAX_OVERLOAD_GROUPS: usize = 100;
const MAX_SAMPLES_PER_KIND: usize = 8;
const MAX_FRAGMENT_FILES: usize = 25;
const MAX_FRAGMENTS_PER_FILE: usize = 3;
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
    symbols: usize,
    missing_names: usize,
    child_symbols: usize,
    child_symbols_with_parent: usize,
    non_declaration_callable_fragments: usize,
    records_with_attributes: usize,
    attributes: usize,
    records_with_doc_comments: usize,
    doc_comments: usize,
}

#[derive(Default)]
struct Frequencies {
    symbol_kinds: BTreeMap<String, usize>,
    missing_names_by_kind: BTreeMap<String, usize>,
    base_types: BTreeMap<String, usize>,
    type_texts: BTreeMap<String, usize>,
    return_type_texts: BTreeMap<String, usize>,
    modifiers: BTreeMap<String, usize>,
    attribute_names: BTreeMap<String, usize>,
    type_shape_base_names: BTreeMap<String, usize>,
    type_shape_qualifiers: BTreeMap<String, usize>,
    type_shape_generic_arities: BTreeMap<String, usize>,
    type_shape_array_suffixes: BTreeMap<String, usize>,
}

#[derive(Default)]
struct DocCoverage {
    records: usize,
    records_with_docs: usize,
    comments: usize,
}

#[derive(Clone)]
struct Occurrence {
    kind: &'static str,
    path: PathBuf,
    line: usize,
}

struct SymbolSample {
    kind: &'static str,
    path: PathBuf,
    line: usize,
    name: String,
    detail: String,
}

struct FileFragments {
    path: PathBuf,
    spans: Vec<TextSpan>,
    source: String,
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
            "Failed to write symbol corpus report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!("Wrote symbol corpus report: {}", args.out_path.display());
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut scripts: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;

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
                    "Usage: node tools/symbol-corpus-report.mjs [--scripts <path>] [--out <path>]"
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
    let mut frequencies = Frequencies::default();
    let mut doc_coverage = BTreeMap::<String, DocCoverage>::new();
    let mut top_level_names = BTreeMap::<String, Vec<Occurrence>>::new();
    let mut method_groups = BTreeMap::<String, Vec<Occurrence>>::new();
    let mut constructor_groups = BTreeMap::<String, Vec<Occurrence>>::new();
    let mut destructor_groups = BTreeMap::<String, Vec<Occurrence>>::new();
    let mut samples_by_kind = BTreeMap::<String, Vec<SymbolSample>>::new();
    let mut fragment_files = Vec::<FileFragments>::new();

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
        let catalog = SymbolCatalog::from_ast_with_metadata(
            &source,
            &ast,
            game_data_metadata(scripts_path, file),
        );
        scan_catalog(
            scripts_path,
            file,
            &source,
            &catalog,
            &mut totals,
            &mut frequencies,
            &mut doc_coverage,
            &mut top_level_names,
            &mut method_groups,
            &mut constructor_groups,
            &mut destructor_groups,
            &mut samples_by_kind,
        );

        let fragment_spans = collect_fragment_spans(&ast);
        if !fragment_spans.is_empty() {
            fragment_files.push(FileFragments {
                path: file.clone(),
                spans: fragment_spans,
                source,
            });
        }
    }

    let mut report = String::new();
    report.push_str("# Symbol Corpus Report\n\n");
    report
        .push_str("> Human-review output generated by `node tools/symbol-corpus-report.mjs`.\n\n");
    report.push_str("This report summarizes file-local declaration model symbols across real game-data scripts. It is review data only; Workbench remains compiler truth.\n\n");

    append_summary(&mut report, scripts_path, &totals);
    append_counts(
        &mut report,
        "Symbol Kind Frequency",
        &frequencies.symbol_kinds,
        80,
    );
    append_counts(
        &mut report,
        "Missing Names By Symbol Kind",
        &frequencies.missing_names_by_kind,
        80,
    );
    append_counts(
        &mut report,
        "Modifier Frequency",
        &frequencies.modifiers,
        80,
    );
    append_counts(
        &mut report,
        "Attribute Name Frequency",
        &frequencies.attribute_names,
        80,
    );
    append_doc_coverage(&mut report, &doc_coverage);
    append_counts(
        &mut report,
        "Class Base Type Frequency",
        &frequencies.base_types,
        80,
    );
    append_counts(
        &mut report,
        "Type Text Frequency",
        &frequencies.type_texts,
        80,
    );
    append_counts(
        &mut report,
        "Type Shape Base Name Frequency",
        &frequencies.type_shape_base_names,
        80,
    );
    append_counts(
        &mut report,
        "Type Shape Qualifier Frequency",
        &frequencies.type_shape_qualifiers,
        80,
    );
    append_counts(
        &mut report,
        "Type Shape Generic Arity Frequency",
        &frequencies.type_shape_generic_arities,
        16,
    );
    append_counts(
        &mut report,
        "Type Shape Array Suffix Frequency",
        &frequencies.type_shape_array_suffixes,
        32,
    );
    append_counts(
        &mut report,
        "Regular Callable Return Type Frequency",
        &frequencies.return_type_texts,
        80,
    );
    append_duplicate_top_level_names(&mut report, scripts_path, &top_level_names);
    append_overload_groups(
        &mut report,
        scripts_path,
        "Regular Method Overload Groups",
        "Constructors and destructors are excluded here and reported in their own sections.",
        &method_groups,
        MAX_OVERLOAD_GROUPS,
    );
    append_overload_groups(
        &mut report,
        scripts_path,
        "Constructor Overload Groups",
        "Constructor overloads are grouped separately because constructor names intentionally match the owning class.",
        &constructor_groups,
        MAX_OVERLOAD_GROUPS,
    );
    append_overload_groups(
        &mut report,
        scripts_path,
        "Destructor Overload Groups",
        "Destructor overloads are grouped separately from regular methods and constructors for review clarity.",
        &destructor_groups,
        MAX_OVERLOAD_GROUPS,
    );
    append_samples(&mut report, scripts_path, &samples_by_kind);
    append_fragment_snippets(&mut report, scripts_path, &fragment_files);

    Ok(report)
}

fn game_data_metadata(scripts_path: &Path, file: &Path) -> SourceFileMetadata {
    let relative_path = file
        .strip_prefix(scripts_path)
        .unwrap_or(file)
        .to_path_buf();
    SourceFileMetadata {
        kind: SourceKind::GameData,
        category: source_category_for_path(SourceKind::GameData, Some(&relative_path)),
        absolute_path: Some(file.to_path_buf()),
        root_path: Some(scripts_path.to_path_buf()),
        relative_path: Some(relative_path),
        priority: SOURCE_PRIORITY_GAME_DATA,
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_catalog(
    scripts_path: &Path,
    file: &Path,
    source: &str,
    catalog: &SymbolCatalog<'_>,
    totals: &mut Totals,
    frequencies: &mut Frequencies,
    doc_coverage: &mut BTreeMap<String, DocCoverage>,
    top_level_names: &mut BTreeMap<String, Vec<Occurrence>>,
    method_groups: &mut BTreeMap<String, Vec<Occurrence>>,
    constructor_groups: &mut BTreeMap<String, Vec<Occurrence>>,
    destructor_groups: &mut BTreeMap<String, Vec<Occurrence>>,
    samples_by_kind: &mut BTreeMap<String, Vec<SymbolSample>>,
) {
    totals.symbols += catalog.records().len();
    totals.non_declaration_callable_fragments += catalog.non_declaration_callable_fragments();

    for record in catalog.records() {
        let kind = kind_name(record.kind);
        count(&mut frequencies.symbol_kinds, kind);

        let coverage = doc_coverage.entry(kind.to_string()).or_default();
        coverage.records += 1;
        if !record.doc_comments.is_empty() {
            coverage.records_with_docs += 1;
            coverage.comments += record.doc_comments.len();
        }

        if record.name.is_none() {
            totals.missing_names += 1;
            count(&mut frequencies.missing_names_by_kind, kind);
        }

        if is_child_kind(record.kind) {
            totals.child_symbols += 1;
            if record.parent.is_some() {
                totals.child_symbols_with_parent += 1;
            }
        }

        if record.parent.is_none() {
            if let Some(name) = catalog.record_name(record) {
                top_level_names
                    .entry(name.to_string())
                    .or_default()
                    .push(occurrence(source, file, record));
            }
        }

        if !record.attributes.is_empty() {
            totals.records_with_attributes += 1;
            totals.attributes += record.attributes.len();
            for attribute in &record.attributes {
                if let Some(name) = catalog.attribute_name(*attribute) {
                    count(&mut frequencies.attribute_names, name);
                }
            }
        }

        if !record.doc_comments.is_empty() {
            totals.records_with_doc_comments += 1;
            totals.doc_comments += record.doc_comments.len();
        }

        for modifier in &record.modifiers {
            count(&mut frequencies.modifiers, catalog.text(*modifier));
        }

        if let Some(base_type) = record.detail.base_type {
            count(&mut frequencies.base_types, catalog.text(base_type));
        }

        if let Some(type_text) = record.detail.type_text {
            count(&mut frequencies.type_texts, catalog.text(type_text));
        }

        if let Some(type_shape) = catalog.record_type_shape(record) {
            if let Some(base_name) = type_shape.base_name_text() {
                count(&mut frequencies.type_shape_base_names, base_name);
            }
            for qualifier in type_shape.qualifier_texts() {
                count(&mut frequencies.type_shape_qualifiers, qualifier);
            }
            count(
                &mut frequencies.type_shape_generic_arities,
                &type_shape.generic_args().len().to_string(),
            );
            for suffix in type_shape.array_suffix_texts() {
                count(&mut frequencies.type_shape_array_suffixes, suffix);
            }
        }

        if let Some(return_type_text) = record.detail.return_type_text {
            if matches!(record.kind, SymbolKind::Function | SymbolKind::Method) {
                count(
                    &mut frequencies.return_type_texts,
                    catalog.text(return_type_text),
                );
            }
        }

        if matches!(
            record.kind,
            SymbolKind::Method | SymbolKind::Constructor | SymbolKind::Destructor
        ) {
            if let Some(name) = catalog.record_name(record) {
                let owner = owner_name(catalog, record);
                let group = format!("{owner}.{name}");
                let occurrence = occurrence(source, file, record);
                match record.kind {
                    SymbolKind::Method => method_groups.entry(group).or_default().push(occurrence),
                    SymbolKind::Constructor => {
                        constructor_groups
                            .entry(group)
                            .or_default()
                            .push(occurrence);
                    }
                    SymbolKind::Destructor => {
                        destructor_groups.entry(group).or_default().push(occurrence);
                    }
                    _ => {}
                }
            }
        }

        let samples = samples_by_kind.entry(kind.to_string()).or_default();
        if samples.len() < MAX_SAMPLES_PER_KIND {
            samples.push(SymbolSample {
                kind,
                path: file
                    .strip_prefix(scripts_path)
                    .unwrap_or(file)
                    .to_path_buf(),
                line: line_column(source, record.selection_span.start).0,
                name: catalog
                    .record_name(record)
                    .map(str::to_string)
                    .unwrap_or_else(|| "<unknown>".to_string()),
                detail: detail_text(catalog, record),
            });
        }
    }
}

fn collect_fragment_spans(ast: &AstSourceFile<'_, '_>) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    for declaration in ast.declarations() {
        match declaration {
            Declaration::Class(class) => {
                for member in class.members() {
                    if let ClassMember::Method(method) = member {
                        spans.extend(method.parameter_fragments().into_iter().map(|p| p.span()));
                    }
                }
            }
            Declaration::Function(function) => {
                spans.extend(function.parameter_fragments().into_iter().map(|p| p.span()));
            }
            _ => {}
        }
    }
    spans
}

fn occurrence(source: &str, file: &Path, record: &SymbolRecord) -> Occurrence {
    Occurrence {
        kind: kind_name(record.kind),
        path: file.to_path_buf(),
        line: line_column(source, record.selection_span.start).0,
    }
}

fn owner_name(catalog: &SymbolCatalog<'_>, record: &SymbolRecord) -> String {
    record
        .parent
        .and_then(|parent| catalog.record(parent))
        .and_then(|parent| catalog.record_name(parent))
        .unwrap_or("<unknown>")
        .to_string()
}

fn detail_text(catalog: &SymbolCatalog<'_>, record: &SymbolRecord) -> String {
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
    values.join(" ")
}

fn push_detail(
    catalog: &SymbolCatalog<'_>,
    values: &mut Vec<String>,
    label: &str,
    span: Option<TextSpan>,
) {
    if let Some(span) = span {
        values.push(format!("{label}: {}", catalog.text(span)));
    }
}

fn is_child_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::EnumMember
            | SymbolKind::Field
            | SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Destructor
            | SymbolKind::Parameter
    )
}

fn kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "Class",
        SymbolKind::TypeParameter => "TypeParameter",
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
        SymbolKind::LocalVariable => "LocalVariable",
        SymbolKind::PreprocessorMacro => "PreprocessorMacro",
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

fn append_summary(report: &mut String, scripts_path: &Path, totals: &Totals) {
    report.push_str("## Summary\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Source path | `{}` |\n", scripts_path.display()));
    report.push_str(&format!(
        "| Source kind | `{}` |\n",
        SourceKind::GameData.as_str()
    ));
    report.push_str(&format!(
        "| Source priority | {} |\n",
        SOURCE_PRIORITY_GAME_DATA
    ));
    report.push_str(&format!("| Source root | `{}` |\n", scripts_path.display()));
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
    report.push_str(&format!("| Symbols | {} |\n", totals.symbols));
    report.push_str(&format!(
        "| Missing symbol names | {} |\n",
        totals.missing_names
    ));
    report.push_str(&format!("| Child symbols | {} |\n", totals.child_symbols));
    report.push_str(&format!(
        "| Child symbols with parent | {} |\n",
        totals.child_symbols_with_parent
    ));
    report.push_str(&format!(
        "| Non-declaration callable fragments | {} |\n",
        totals.non_declaration_callable_fragments
    ));
    report.push_str(&format!(
        "| Records with attributes | {} |\n",
        totals.records_with_attributes
    ));
    report.push_str(&format!("| Attributes | {} |\n", totals.attributes));
    report.push_str(&format!(
        "| Records with doc comments | {} |\n",
        totals.records_with_doc_comments
    ));
    report.push_str(&format!("| Doc comments | {} |\n\n", totals.doc_comments));
}

fn append_counts(
    report: &mut String,
    heading: &str,
    counts: &BTreeMap<String, usize>,
    limit: usize,
) {
    report.push_str(&format!("## {heading}\n\n"));

    if counts.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Item | Count |\n");
    report.push_str("| --- | ---: |\n");
    for (item, count) in sorted_counts(counts).into_iter().take(limit) {
        report.push_str(&format!("| `{}` | {} |\n", escape_table(&item), count));
    }
    report.push('\n');
}

fn append_doc_coverage(report: &mut String, coverage: &BTreeMap<String, DocCoverage>) {
    report.push_str("## Doc Comment Coverage By Symbol Kind\n\n");
    if coverage.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Kind | Records | Records with docs | Doc comments |\n");
    report.push_str("| --- | ---: | ---: | ---: |\n");
    for (kind, coverage) in coverage {
        report.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            kind, coverage.records, coverage.records_with_docs, coverage.comments
        ));
    }
    report.push('\n');
}

fn append_duplicate_top_level_names(
    report: &mut String,
    scripts_path: &Path,
    names: &BTreeMap<String, Vec<Occurrence>>,
) {
    report.push_str("## Duplicate Top-Level Names\n\n");
    let duplicates = duplicate_entries(names);
    if duplicates.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Name | Count | Declarations |\n");
    report.push_str("| --- | ---: | --- |\n");
    for (name, occurrences) in duplicates.into_iter().take(MAX_DUPLICATE_NAMES) {
        let declarations = occurrences
            .iter()
            .map(|occurrence| {
                format!(
                    "{} `{}` in `{}:{}`",
                    occurrence.kind,
                    escape_table(&name),
                    relative_path(scripts_path, &occurrence.path),
                    occurrence.line
                )
            })
            .collect::<Vec<_>>()
            .join("<br>");
        report.push_str(&format!(
            "| `{}` | {} | {} |\n",
            escape_table(&name),
            occurrences.len(),
            declarations
        ));
    }
    report.push('\n');
}

fn duplicate_entries(names: &BTreeMap<String, Vec<Occurrence>>) -> Vec<(String, Vec<Occurrence>)> {
    let mut entries = names
        .iter()
        .filter(|(_, occurrences)| occurrences.len() > 1)
        .map(|(name, occurrences)| (name.clone(), occurrences.clone()))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .1
            .len()
            .cmp(&left.1.len())
            .then_with(|| left.0.cmp(&right.0))
    });
    entries
}

fn append_overload_groups(
    report: &mut String,
    scripts_path: &Path,
    heading: &str,
    note: &str,
    groups: &BTreeMap<String, Vec<Occurrence>>,
    limit: usize,
) {
    report.push_str(&format!("## {heading}\n\n"));
    report.push_str(note);
    report.push_str("\n\n");
    let groups = duplicate_entries(groups);
    if groups.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Group | Count | Locations |\n");
    report.push_str("| --- | ---: | --- |\n");
    for (group, occurrences) in groups.into_iter().take(limit) {
        let locations = occurrences
            .iter()
            .map(|occurrence| {
                format!(
                    "{}:{}",
                    relative_path(scripts_path, &occurrence.path),
                    occurrence.line
                )
            })
            .collect::<Vec<_>>()
            .join("<br>");
        report.push_str(&format!(
            "| `{}` | {} | {} |\n",
            escape_table(&group),
            occurrences.len(),
            locations
        ));
    }
    report.push('\n');
}

fn append_samples(
    report: &mut String,
    scripts_path: &Path,
    samples: &BTreeMap<String, Vec<SymbolSample>>,
) {
    report.push_str("## Sample Symbols By Kind\n\n");
    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Kind | Name | Location | Detail |\n");
    report.push_str("| --- | --- | --- | --- |\n");
    for samples in samples.values() {
        for sample in samples {
            report.push_str(&format!(
                "| `{}` | `{}` | `{}` | `{}` |\n",
                sample.kind,
                escape_table(&sample.name),
                format!(
                    "{}:{}",
                    relative_path(scripts_path, &sample.path),
                    sample.line
                ),
                escape_table(&sample.detail)
            ));
        }
    }
    report.push('\n');
}

fn append_fragment_snippets(report: &mut String, scripts_path: &Path, files: &[FileFragments]) {
    report.push_str("## Non-Declaration Callable Fragment Snippets\n\n");
    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    for file in files.iter().take(MAX_FRAGMENT_FILES) {
        report.push_str(&format!(
            "### `{}`\n\n",
            relative_path(scripts_path, &file.path)
        ));
        for span in file.spans.iter().take(MAX_FRAGMENTS_PER_FILE) {
            let (line, column) = line_column(&file.source, span.start);
            report.push_str(&format!(
                "- `non-declaration callable fragment` at {}:{} span {}..{}\n\n",
                line, column, span.start, span.end
            ));
            append_source_snippet(report, &file.source, line);
        }
    }
}

fn append_source_snippet(report: &mut String, source: &str, line: usize) {
    let lines = source.lines().collect::<Vec<_>>();
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

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut values = counts
        .iter()
        .map(|(item, count)| (item.clone(), *count))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values
}

fn count(counts: &mut BTreeMap<String, usize>, value: &str) {
    *counts.entry(value.to_string()).or_default() += 1;
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
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

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|")
}
