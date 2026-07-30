use reforger_language_server::index::SymbolIndex;
use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::lexer::TextSpan;
use reforger_language_server::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, SymbolCatalog, SymbolKind,
    SOURCE_PRIORITY_GAME_DATA,
};
use reforger_language_server::parser::parse_source;
use reforger_language_server::reference_finder::{
    scan_file_local_references_with_external, UnresolvedReferenceToken,
};
use reforger_language_server::scope::LexicalScopeModel;
use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const DEFAULT_OUTPUT: &str = "tools/reports/reference-finder-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_ROWS: usize = 75;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    if !args.scripts_path.is_dir() {
        return Err(format!(
            "Scripts folder does not exist: {}",
            args.scripts_path.display()
        ));
    }

    let started = Instant::now();
    let mut files = Vec::new();
    collect_c_files(&args.scripts_path, &mut files)?;
    files.sort();
    if let Some(max_files) = args.max_files {
        files.truncate(max_files);
    }
    let files = sample_evenly_paths(&files, args.max_files.unwrap_or(files.len()).max(1));
    let external_index = if args.external_index {
        Some(
            build_index(&IndexBuildConfig {
                roots: vec![IndexSourceRoot::new(
                    &args.scripts_path,
                    SourceKind::GameData,
                    SOURCE_PRIORITY_GAME_DATA,
                )],
            })
            .map(|result| result.index)?,
        )
    } else {
        None
    };

    let mut totals = Totals::default();
    let mut reference_kind_counts = BTreeMap::<String, ReferenceKindCount>::new();
    let mut unresolved_counts = BTreeMap::<String, usize>::new();
    let mut unresolved_review_counts = BTreeMap::<String, usize>::new();
    let mut target_rows = Vec::<TargetRow>::new();
    let mut unresolved_samples = Vec::<UnresolvedRow>::new();

    for file in files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        let source = String::from_utf8_lossy(&bytes);
        let relative_path = relative_display(file, &args.scripts_path);
        let relative_path_buf = PathBuf::from(&relative_path);
        let parse = parse_source(&source);
        let ast = reforger_language_server::ast::AstSourceFile::new(&source, &parse);
        let metadata = SourceFileMetadata {
            kind: SourceKind::GameData,
            category: source_category_for_path(SourceKind::GameData, Some(&relative_path_buf)),
            absolute_path: file.canonicalize().ok(),
            virtual_source: None,
            root_path: args.scripts_path.canonicalize().ok(),
            relative_path: Some(relative_path_buf),
            priority: SOURCE_PRIORITY_GAME_DATA,
        };
        let catalog = SymbolCatalog::from_ast_with_metadata(&source, &ast, metadata);
        let index = SymbolIndex::from_catalogs([&catalog]);
        let scope = LexicalScopeModel::from_parse_and_index(&parse, &index);
        let scan = scan_file_local_references_with_external(
            &source,
            &index,
            &parse,
            &scope,
            external_index.as_ref(),
        );

        totals.files += 1;
        totals.bytes += bytes.len();
        totals.parse_diagnostics += parse.diagnostics.len();
        totals.identifiers_scanned += scan.identifiers_scanned;
        totals.external_references += scan.external_references;
        totals.unresolved += scan.unresolved.len();

        for unresolved in &scan.unresolved {
            let bucket = unresolved_bucket(&source, unresolved);
            *unresolved_counts.entry(bucket.to_string()).or_default() += 1;
            *unresolved_review_counts
                .entry(unresolved_review_bucket(bucket).to_string())
                .or_default() += 1;
            if unresolved_samples.len() < MAX_ROWS
                || unresolved_review_bucket(bucket) == "actionable unresolved"
            {
                push_bounded_unresolved_sample(
                    &mut unresolved_samples,
                    UnresolvedRow {
                        path: relative_path.clone(),
                        line: line_number(&source, unresolved.span),
                        token: unresolved.token_text.clone(),
                        bucket: bucket.to_string(),
                        reason: unresolved.reason.as_str().to_string(),
                        context: unresolved.identifier_context.as_str().to_string(),
                        source_line: line_text_at_offset(&source, unresolved.span.start),
                    },
                );
            }
        }

        for symbol in index.symbols() {
            if !is_reference_target_kind(symbol.kind) {
                continue;
            }
            totals.targets += 1;
            let references = scan
                .references_by_target
                .get(&symbol.id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let declaration_refs = references
                .iter()
                .filter(|reference| reference.is_declaration)
                .count();
            let usage_refs = references.len().saturating_sub(declaration_refs);
            let kind = symbol_kind_label(symbol.kind).to_string();
            let count = reference_kind_counts.entry(kind.clone()).or_default();
            count.targets += 1;
            count.references += references.len();
            count.declarations += declaration_refs;
            count.usages += usage_refs;
            if usage_refs == 0 {
                count.declaration_only += 1;
            }
            if references.is_empty() {
                count.missing_declaration_reference += 1;
            }
            if target_rows.len() < MAX_ROWS || references.is_empty() {
                push_bounded_target_row(
                    &mut target_rows,
                    TargetRow {
                        path: relative_path.clone(),
                        line: line_number(&source, symbol.selection_span),
                        kind,
                        name: symbol
                            .name
                            .clone()
                            .unwrap_or_else(|| "<missing>".to_string()),
                        references: references.len(),
                        declaration_refs,
                        usage_refs,
                        source_line: line_text_at_offset(&source, symbol.selection_span.start),
                    },
                );
            }
        }
    }

    let report = render_report(
        &args,
        &totals,
        &reference_kind_counts,
        &unresolved_counts,
        &unresolved_review_counts,
        &target_rows,
        &unresolved_samples,
        started.elapsed().as_millis(),
    );
    if let Some(parent) = args.out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::write(&args.out_path, report)
        .map_err(|error| format!("Failed to write {}: {error}", args.out_path.display()))?;
    println!("Wrote {}", args.out_path.display());
    Ok(())
}

#[derive(Debug)]
struct Args {
    scripts_path: PathBuf,
    out_path: PathBuf,
    max_files: Option<usize>,
    external_index: bool,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut scripts_path = None;
        let mut out_path = None;
        let mut max_files = None;
        let mut external_index = true;
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scripts" => {
                    scripts_path = Some(PathBuf::from(
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
                "--max-files" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--max-files requires a number".to_string())?;
                    max_files = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| format!("Invalid --max-files value: {value}"))?,
                    );
                }
                "--no-external-index" => {
                    external_index = false;
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        Ok(Self {
            scripts_path: scripts_path.unwrap_or_else(default_scripts_path),
            out_path: resolve_repo_path(out_path, DEFAULT_OUTPUT),
            max_files,
            external_index,
        })
    }
}

#[derive(Default)]
struct Totals {
    files: usize,
    bytes: usize,
    parse_diagnostics: usize,
    identifiers_scanned: usize,
    targets: usize,
    unresolved: usize,
    external_references: usize,
}

#[derive(Default)]
struct ReferenceKindCount {
    targets: usize,
    references: usize,
    declarations: usize,
    usages: usize,
    declaration_only: usize,
    missing_declaration_reference: usize,
}

struct TargetRow {
    path: String,
    line: usize,
    kind: String,
    name: String,
    references: usize,
    declaration_refs: usize,
    usage_refs: usize,
    source_line: String,
}

struct UnresolvedRow {
    path: String,
    line: usize,
    token: String,
    bucket: String,
    reason: String,
    context: String,
    source_line: String,
}

fn render_report(
    args: &Args,
    totals: &Totals,
    reference_kind_counts: &BTreeMap<String, ReferenceKindCount>,
    unresolved_counts: &BTreeMap<String, usize>,
    unresolved_review_counts: &BTreeMap<String, usize>,
    target_rows: &[TargetRow],
    unresolved_samples: &[UnresolvedRow],
    elapsed_ms: u128,
) -> String {
    let mut report = String::new();
    writeln!(report, "# Reference Finder Corpus Report\n").unwrap();
    writeln!(report, "- Source path: `{}`", args.scripts_path.display()).unwrap();
    writeln!(
        report,
        "- Max files: `{}`",
        args.max_files
            .map(|value| value.to_string())
            .unwrap_or_else(|| "all".to_string())
    )
    .unwrap();
    writeln!(
        report,
        "- External index: `{}`",
        if args.external_index {
            "enabled"
        } else {
            "disabled"
        }
    )
    .unwrap();
    writeln!(report).unwrap();
    writeln!(report, "This report scans real game-data files once, resolves every identifier through the resolver, and groups file-local references by exact selected symbol id. It is review tooling for future references/rename; it does not perform workspace-wide search or text-only matching.").unwrap();

    report.push_str("\n## Summary\n\n");
    report.push_str("| Metric | Count |\n| --- | ---: |\n");
    writeln!(report, "| Files | {} |", totals.files).unwrap();
    writeln!(report, "| Bytes | {} |", totals.bytes).unwrap();
    writeln!(
        report,
        "| Parse diagnostics | {} |",
        totals.parse_diagnostics
    )
    .unwrap();
    writeln!(
        report,
        "| Identifier tokens scanned | {} |",
        totals.identifiers_scanned
    )
    .unwrap();
    writeln!(report, "| Reference targets | {} |", totals.targets).unwrap();
    writeln!(report, "| Unresolved identifiers | {} |", totals.unresolved).unwrap();
    writeln!(
        report,
        "| External selections | {} |",
        totals.external_references
    )
    .unwrap();
    writeln!(report, "| Elapsed ms | {} |", elapsed_ms).unwrap();

    append_kind_counts(&mut report, reference_kind_counts);
    append_counts(
        &mut report,
        "Unresolved Identifier Classification",
        unresolved_counts,
    );
    append_counts(
        &mut report,
        "Unresolved Review Buckets",
        unresolved_review_counts,
    );
    append_target_rows(&mut report, target_rows);
    append_unresolved_rows(&mut report, unresolved_samples);
    report
}

fn append_kind_counts(report: &mut String, counts: &BTreeMap<String, ReferenceKindCount>) {
    report.push_str("\n## Reference Coverage By Kind\n\n");
    if counts.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Kind | Targets | References | Declaration refs | Usage refs | Declaration-only targets | Missing declaration ref |\n");
    report.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for (kind, count) in counts {
        writeln!(
            report,
            "| `{}` | {} | {} | {} | {} | {} | {} |",
            escape_table(kind),
            count.targets,
            count.references,
            count.declarations,
            count.usages,
            count.declaration_only,
            count.missing_declaration_reference,
        )
        .unwrap();
    }
}

fn append_counts(report: &mut String, title: &str, counts: &BTreeMap<String, usize>) {
    writeln!(report, "\n## {title}\n").unwrap();
    if counts.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Value | Count |\n| --- | ---: |\n");
    for (value, count) in sorted_counts(counts).into_iter().take(MAX_ROWS) {
        writeln!(report, "| `{}` | {} |", escape_table(&value), count).unwrap();
    }
}

fn append_target_rows(report: &mut String, rows: &[TargetRow]) {
    report.push_str("\n## Target Reference Samples\n\n");
    if rows.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Path | Line | Kind | Name | Refs | Decl refs | Usage refs | Source |\n");
    report.push_str("| --- | ---: | --- | --- | ---: | ---: | ---: | --- |\n");
    for row in rows.iter().take(MAX_ROWS) {
        writeln!(
            report,
            "| `{}` | {} | `{}` | `{}` | {} | {} | {} | `{}` |",
            escape_table(&row.path),
            row.line,
            escape_table(&row.kind),
            escape_table(&row.name),
            row.references,
            row.declaration_refs,
            row.usage_refs,
            escape_table(&row.source_line),
        )
        .unwrap();
    }
}

fn append_unresolved_rows(report: &mut String, rows: &[UnresolvedRow]) {
    report.push_str("\n## Unresolved Identifier Samples\n\n");
    if rows.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Path | Line | Token | Bucket | Reason | Context | Source |\n");
    report.push_str("| --- | ---: | --- | --- | --- | --- | --- |\n");
    for row in rows.iter().take(MAX_ROWS) {
        writeln!(
            report,
            "| `{}` | {} | `{}` | `{}` | `{}` | `{}` | `{}` |",
            escape_table(&row.path),
            row.line,
            escape_table(&row.token),
            escape_table(&row.bucket),
            escape_table(&row.reason),
            escape_table(&row.context),
            escape_table(&row.source_line),
        )
        .unwrap();
    }
}

fn unresolved_bucket(source: &str, token: &UnresolvedReferenceToken) -> &'static str {
    let reason = token.reason.as_str();
    let line = raw_line_at_offset(source, token.span.start);
    let trimmed = line.trim_start();
    if reason == "preprocessor-directive" {
        "preprocessor directive"
    } else if reason == "preprocessor-macro" {
        "preprocessor macro name"
    } else if reason == "attribute-named-argument" {
        "attribute named argument"
    } else if reason == "named-argument-label" {
        if is_attribute_line(&line) {
            "attribute named argument"
        } else {
            "named call argument label"
        }
    } else if trimmed.starts_with('#') {
        "preprocessor token"
    } else if is_attribute_line(&line) {
        "attribute value/noise"
    } else if token.identifier_context.as_str() == "member-access" {
        "unresolved member access"
    } else if token.identifier_context.as_str() == "type-position" {
        "unresolved type"
    } else {
        "unresolved value/callable"
    }
}

fn unresolved_review_bucket(bucket: &str) -> &'static str {
    match bucket {
        "preprocessor directive"
        | "preprocessor macro name"
        | "preprocessor token"
        | "attribute named argument"
        | "named call argument label"
        | "attribute value/noise" => "source-noise / non-reference target",
        "unresolved member access" => "actionable receiver/member unresolved",
        "unresolved type" | "unresolved value/callable" => "actionable unresolved",
        _ => "other",
    }
}

fn is_reference_target_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Class
            | SymbolKind::TypeParameter
            | SymbolKind::Enum
            | SymbolKind::EnumMember
            | SymbolKind::Typedef
            | SymbolKind::Function
            | SymbolKind::GlobalField
            | SymbolKind::Field
            | SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Destructor
            | SymbolKind::Parameter
            | SymbolKind::LocalVariable
    )
}

fn push_bounded_target_row(rows: &mut Vec<TargetRow>, row: TargetRow) {
    if rows.len() < MAX_ROWS || row.references == 0 {
        rows.push(row);
        rows.sort_by(|left, right| {
            left.references
                .cmp(&right.references)
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.name.cmp(&right.name))
        });
        rows.truncate(MAX_ROWS);
    }
}

fn push_bounded_unresolved_sample(rows: &mut Vec<UnresolvedRow>, row: UnresolvedRow) {
    rows.push(row);
    rows.sort_by(|left, right| {
        unresolved_review_bucket(&left.bucket)
            .cmp(unresolved_review_bucket(&right.bucket))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });
    rows.truncate(MAX_ROWS);
}

fn collect_c_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(root).map_err(|error| format!("Failed to read {}: {error}", root.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to read entry in {}: {error}", root.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_c_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "c") {
            files.push(path);
        }
    }
    Ok(())
}

fn sample_evenly_paths(paths: &[PathBuf], limit: usize) -> Vec<&PathBuf> {
    if paths.len() <= limit {
        return paths.iter().collect();
    }
    if limit <= 1 {
        return paths.first().into_iter().collect();
    }
    let mut sampled = Vec::new();
    let mut last_index = None;
    for index in 0..limit {
        let value_index = index * (paths.len() - 1) / (limit - 1);
        if last_index == Some(value_index) {
            continue;
        }
        sampled.push(&paths[value_index]);
        last_index = Some(value_index);
    }
    sampled
}

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut values = counts
        .iter()
        .map(|(value, count)| (value.clone(), *count))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values
}

fn raw_line_at_offset(source: &str, offset: usize) -> String {
    let start = source[..offset]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let end = source[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(source.len());
    source[start..end].to_string()
}

fn line_text_at_offset(source: &str, offset: usize) -> String {
    raw_line_at_offset(source, offset).trim().to_string()
}

fn line_number(source: &str, span: TextSpan) -> usize {
    source[..span.start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn is_attribute_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('[') && trimmed.contains(']')
}

fn symbol_kind_label(kind: SymbolKind) -> &'static str {
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

fn relative_display(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|")
}

fn print_help() {
    println!("Usage: cargo run --manifest-path server/Cargo.toml --example reference_finder_corpus_report -- [--scripts <path>] [--out <path>] [--max-files <n>] [--no-external-index]");
}
