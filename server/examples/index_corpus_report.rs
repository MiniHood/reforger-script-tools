use reforger_language_server::ast::AstSourceFile;
use reforger_language_server::index::{GlobalSymbolId, SymbolIndex};
use reforger_language_server::model::{
    SourceFileMetadata, SourceKind, SymbolCatalog, SymbolKind, SOURCE_PRIORITY_GAME_DATA,
};
use reforger_language_server::parser::parse_source;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/index-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_DUPLICATE_NAMES: usize = 100;
const MAX_DECLARATIONS_PER_DUPLICATE: usize = 8;
const MAX_SAMPLES: usize = 20;

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
    non_declaration_callable_fragments: usize,
}

struct SymbolSample {
    id: GlobalSymbolId,
    kind: SymbolKind,
    name: String,
    detail: String,
}

struct MethodGroupSample {
    owner: String,
    name: String,
    overloads: usize,
    path: String,
    return_types: String,
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
            "Failed to write index corpus report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!("Wrote index corpus report: {}", args.out_path.display());
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
                    "Usage: node tools/index-corpus-report.mjs [--scripts <path>] [--out <path>]"
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
        let catalog = SymbolCatalog::from_ast_with_metadata(
            source,
            &ast,
            game_data_metadata(scripts_path, file),
        );
        totals.non_declaration_callable_fragments += catalog.non_declaration_callable_fragments();
        catalogs.push(catalog);
    }

    let index = SymbolIndex::from_catalogs(catalogs.iter());
    let mut report = String::new();
    report.push_str("# Index Corpus Report\n\n");
    report.push_str("> Human-review output generated by `node tools/index-corpus-report.mjs`.\n\n");
    report.push_str("This report summarizes the first in-memory symbol index over game-data catalogs. It is lookup review data only; Workbench remains compiler truth.\n\n");

    append_summary(&mut report, scripts_path, &totals, &index);
    append_source_kind_counts(&mut report, &index);
    append_kind_counts(&mut report, &index);
    append_duplicate_top_level_names(&mut report, &index);
    append_preferred_duplicate_samples(&mut report, &index);
    append_lookup_samples(&mut report, &index);

    Ok(report)
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

fn append_summary(report: &mut String, scripts_path: &Path, totals: &Totals, index: &SymbolIndex) {
    let map_counts = index.map_counts();
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
    report.push_str(&format!("| Indexed files | {} |\n", index.files().len()));
    report.push_str(&format!(
        "| Indexed symbols | {} |\n",
        index.symbols().len()
    ));
    report.push_str(&format!("| Unique symbol names | {} |\n", map_counts.names));
    report.push_str(&format!(
        "| Unique top-level names | {} |\n",
        map_counts.top_level_names
    ));
    report.push_str(&format!("| Symbol kind maps | {} |\n", map_counts.kinds));
    report.push_str(&format!("| Class names | {} |\n", map_counts.class_names));
    report.push_str(&format!(
        "| Typedef names | {} |\n",
        map_counts.typedef_names
    ));
    report.push_str(&format!(
        "| Method owner/name keys | {} |\n",
        map_counts.method_owner_names
    ));
    report.push_str(&format!(
        "| Parent symbols with children | {} |\n",
        map_counts.parent_symbols
    ));
    report.push_str(&format!(
        "| Non-declaration callable fragments | {} |\n\n",
        totals.non_declaration_callable_fragments
    ));
}

fn append_source_kind_counts(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Source Kind Counts\n\n");
    report.push_str("| Source kind | Files |\n");
    report.push_str("| --- | ---: |\n");
    for (kind, count) in index.source_kind_counts() {
        report.push_str(&format!("| `{}` | {} |\n", kind.as_str(), count));
    }
    report.push('\n');
}

fn append_kind_counts(report: &mut String, index: &SymbolIndex) {
    let mut counts = BTreeMap::<String, usize>::new();
    for symbol in index.symbols() {
        *counts
            .entry(kind_name(symbol.kind).to_string())
            .or_default() += 1;
    }
    append_counts(report, "Symbol Kind Frequency", &counts, 80);
}

fn append_duplicate_top_level_names(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Duplicate Top-Level Name Groups\n\n");
    let mut duplicates = index.duplicate_top_level_names();
    duplicates.sort_by(|left, right| {
        right
            .1
            .len()
            .cmp(&left.1.len())
            .then_with(|| left.0.cmp(right.0))
    });

    if duplicates.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Name | Count | Declarations |\n");
    report.push_str("| --- | ---: | --- |\n");
    for (name, symbols) in duplicates.into_iter().take(MAX_DUPLICATE_NAMES) {
        let mut declarations = symbols
            .iter()
            .take(MAX_DECLARATIONS_PER_DUPLICATE)
            .map(|id| display_symbol_location(index, *id))
            .collect::<Vec<_>>();
        if symbols.len() > MAX_DECLARATIONS_PER_DUPLICATE {
            declarations.push(format!(
                "... {} more",
                symbols.len() - MAX_DECLARATIONS_PER_DUPLICATE
            ));
        }
        report.push_str(&format!(
            "| `{}` | {} | {} |\n",
            escape_table(name),
            symbols.len(),
            declarations.join("<br>")
        ));
    }
    report.push('\n');
}

fn append_preferred_duplicate_samples(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Preferred Duplicate Samples\n\n");
    let mut rows = Vec::new();
    for (name, symbols) in index.duplicate_top_level_names() {
        if symbols.len() < 2 || rows.len() >= MAX_SAMPLES {
            continue;
        }
        if let Some(preferred) = index
            .preferred_top_level_symbols_for_name(name)
            .first()
            .copied()
        {
            rows.push((name.to_string(), preferred));
        }
    }

    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Name | Preferred declaration |\n");
    report.push_str("| --- | --- |\n");
    for (name, id) in rows {
        report.push_str(&format!(
            "| `{}` | {} |\n",
            escape_table(&name),
            display_symbol_location(index, id)
        ));
    }
    report.push('\n');
}

fn append_lookup_samples(report: &mut String, index: &SymbolIndex) {
    report.push_str("## Lookup Samples\n\n");
    append_sample_table(report, index, "Class Lookup Samples", sample_classes(index));
    append_sample_table(
        report,
        index,
        "Typedef Lookup Samples",
        sample_typedefs(index),
    );
    append_method_group_samples(report, sample_method_groups(index));
}

fn sample_classes(index: &SymbolIndex) -> Vec<SymbolSample> {
    sample_by_kind(index, SymbolKind::Class)
}

fn sample_typedefs(index: &SymbolIndex) -> Vec<SymbolSample> {
    sample_by_kind(index, SymbolKind::Typedef)
}

fn sample_by_kind(index: &SymbolIndex, kind: SymbolKind) -> Vec<SymbolSample> {
    index
        .symbols_for_kind(kind)
        .iter()
        .take(MAX_SAMPLES)
        .filter_map(|id| {
            let symbol = index.symbol(*id)?;
            Some(SymbolSample {
                id: *id,
                kind,
                name: symbol
                    .name
                    .clone()
                    .unwrap_or_else(|| "<unknown>".to_string()),
                detail: detail_text(symbol),
            })
        })
        .collect()
}

fn sample_method_groups(index: &SymbolIndex) -> Vec<MethodGroupSample> {
    let mut seen = BTreeSet::new();
    let mut samples = Vec::new();

    index
        .symbols_for_kind(SymbolKind::Method)
        .iter()
        .filter_map(|id| method_group_key(index, *id))
        .for_each(|(owner, name)| {
            if samples.len() >= MAX_SAMPLES || !seen.insert((owner.clone(), name.clone())) {
                return;
            }

            let ids = index.methods_by_owner_name(&owner, &name);
            let Some(first_id) = ids.first().copied() else {
                return;
            };
            samples.push(MethodGroupSample {
                owner: owner.clone(),
                name: name.clone(),
                overloads: ids.len(),
                path: display_sample_location(index, first_id),
                return_types: unique_return_types(index, ids),
            });
        });

    samples
}

fn method_group_key(index: &SymbolIndex, id: GlobalSymbolId) -> Option<(String, String)> {
    let symbol = index.symbol(id)?;
    let owner = symbol
        .parent
        .and_then(|parent| index.symbol(parent))
        .and_then(|parent| parent.name.as_ref())?;
    let name = symbol.name.as_ref()?;
    Some((owner.clone(), name.clone()))
}

fn append_sample_table(
    report: &mut String,
    index: &SymbolIndex,
    heading: &str,
    samples: Vec<SymbolSample>,
) {
    report.push_str(&format!("### {heading}\n\n"));
    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Kind | Name | Location | Detail |\n");
    report.push_str("| --- | --- | --- | --- |\n");
    for sample in samples {
        report.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` |\n",
            kind_name(sample.kind),
            escape_table(&sample.name),
            display_sample_location(index, sample.id),
            escape_table(&sample.detail)
        ));
    }
    report.push('\n');
}

fn append_method_group_samples(report: &mut String, samples: Vec<MethodGroupSample>) {
    report.push_str("### Method Owner/Name Samples\n\n");
    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Owner.Method | Overloads | Path | Return types |\n");
    report.push_str("| --- | ---: | --- | --- |\n");
    for sample in samples {
        report.push_str(&format!(
            "| `{}` | {} | `{}` | `{}` |\n",
            escape_table(&format!("{}.{}", sample.owner, sample.name)),
            sample.overloads,
            escape_table(&sample.path),
            escape_table(&sample.return_types)
        ));
    }
    report.push('\n');
}

fn display_symbol_location(index: &SymbolIndex, id: GlobalSymbolId) -> String {
    let Some(symbol) = index.symbol(id) else {
        return format!("missing symbol {:?}", id);
    };
    let Some(file) = index.file(id.file_id) else {
        return format!("missing file {:?}", id.file_id);
    };
    let path = file
        .metadata
        .relative_path
        .as_ref()
        .or(file.metadata.absolute_path.as_ref())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unknown-path>".to_string());
    let name = symbol.name.as_deref().unwrap_or("<unknown>");
    format!(
        "{} `{}` in `{}` priority {}",
        kind_name(symbol.kind),
        escape_table(name),
        path,
        file.metadata.priority
    )
}

fn display_sample_location(index: &SymbolIndex, id: GlobalSymbolId) -> String {
    let Some(file) = index.file(id.file_id) else {
        return format!("file {} symbol {}", id.file_id.0, id.symbol_id.0);
    };
    let path = file
        .metadata
        .relative_path
        .as_ref()
        .or(file.metadata.absolute_path.as_ref())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| format!("file {}", id.file_id.0));
    format!("{path} #{}", id.symbol_id.0)
}

fn unique_return_types(index: &SymbolIndex, ids: &[GlobalSymbolId]) -> String {
    let mut return_types = ids
        .iter()
        .filter_map(|id| index.symbol(*id))
        .map(|symbol| {
            symbol
                .detail
                .return_type_text
                .as_deref()
                .unwrap_or("<unknown>")
                .to_string()
        })
        .collect::<Vec<_>>();
    return_types.sort();
    return_types.dedup();
    return_types.join(", ")
}

fn detail_text(symbol: &reforger_language_server::index::IndexedSymbol) -> String {
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
    values.join(" ")
}

fn push_detail(values: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        values.push(format!("{label}: {value}"));
    }
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

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut values = counts
        .iter()
        .map(|(item, count)| (item.clone(), *count))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values
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

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|")
}
