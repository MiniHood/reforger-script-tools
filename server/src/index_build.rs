use crate::index::SymbolIndex;
use crate::lexer::TextSpan;
use crate::model::{source_category_for_path, SourceFileMetadata, SourceKind};
use crate::parser::parse_source;
use crate::semantic_file::{FileContribution, SemanticFile};
use crate::syntax::ParseDiagnostic;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

pub const INDEX_BUILD_CANCELLED: &str = "index build cancelled";
const MAX_RECORDED_LOSSY_FILES: usize = 50;
const MAX_RECORDED_DIAGNOSTIC_FILES: usize = 100;
const MAX_RECORDED_DIAGNOSTICS_PER_FILE: usize = 3;
const SNIPPET_CONTEXT_LINES: usize = 2;
const REPLACEMENT_CHARACTER: char = '\u{FFFD}';
const REPLACEMENT_CHARACTER_LABEL: &str = "<U+FFFD>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexSourceRoot {
    pub root_path: PathBuf,
    pub kind: SourceKind,
    pub priority: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexBuildConfig {
    pub roots: Vec<IndexSourceRoot>,
}

#[derive(Debug, Clone, Default)]
pub struct IndexBuildControl {
    cancelled: Arc<AtomicBool>,
}

#[derive(Debug)]
pub struct IndexBuildResult {
    pub index: SymbolIndex,
    pub summary: IndexBuildSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexBuildSummary {
    pub totals: IndexBuildCounts,
    pub by_source_kind: BTreeMap<SourceKind, IndexBuildCounts>,
    pub timings: IndexBuildTimings,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexBuildCounts {
    pub files: usize,
    pub bytes: usize,
    pub lossy_files: usize,
    pub lossy_decode_details: Vec<LossyDecodeDetail>,
    pub parse_diagnostics: usize,
    pub parse_diagnostic_files: usize,
    pub parse_diagnostic_details: Vec<ParseDiagnosticDetail>,
    pub indexed_files: usize,
    pub indexed_symbols: usize,
    pub non_declaration_callable_fragments: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossyDecodeDetail {
    pub path: PathBuf,
    pub first_replacement_offset: usize,
    pub line: usize,
    pub column: usize,
    pub replacement_count: usize,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnosticDetail {
    pub path: PathBuf,
    pub message: String,
    pub span: TextSpan,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexBuildTimings {
    pub file_discovery: Duration,
    pub read_decode: Duration,
    pub parse: Duration,
    pub ast_model_catalog: Duration,
    pub catalog_build: Duration,
    pub index_build: Duration,
    pub total: Duration,
}

struct PendingFileContribution {
    contribution: FileContribution,
    metadata: SourceFileMetadata,
}

impl IndexSourceRoot {
    pub fn new(root_path: impl Into<PathBuf>, kind: SourceKind, priority: u16) -> Self {
        Self {
            root_path: root_path.into(),
            kind,
            priority,
        }
    }
}

impl IndexBuildControl {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn check(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err(INDEX_BUILD_CANCELLED.to_string())
        } else {
            Ok(())
        }
    }
}

pub fn build_index(config: &IndexBuildConfig) -> Result<IndexBuildResult, String> {
    build_index_with_control(config, &IndexBuildControl::default())
}

pub fn build_index_with_control(
    config: &IndexBuildConfig,
    control: &IndexBuildControl,
) -> Result<IndexBuildResult, String> {
    let total_start = Instant::now();
    let mut summary = IndexBuildSummary {
        totals: IndexBuildCounts::default(),
        by_source_kind: BTreeMap::new(),
        timings: IndexBuildTimings::default(),
    };
    let mut pending_contributions = Vec::new();

    for root in &config.roots {
        control.check()?;
        if !root.root_path.is_dir() {
            return Err(format!(
                "Index source root does not exist or is not a folder: {}",
                root.root_path.display()
            ));
        }

        let file_discovery_start = Instant::now();
        let mut files = Vec::new();
        collect_script_files(&root.root_path, &mut files, control)?;
        files.sort();
        summary.timings.file_discovery += file_discovery_start.elapsed();

        for file in files {
            control.check()?;
            pending_contributions.push(build_file(root, &file, &mut summary, control)?);
        }
    }

    control.check()?;
    let index_build_start = Instant::now();
    let mut index = SymbolIndex::default();
    index
        .add_file_contributions(
            pending_contributions
                .iter()
                .map(|pending| (&pending.contribution, pending.metadata.clone())),
        )
        .map_err(|error| {
            format!("Invalid semantic contribution during index aggregation: {error:?}")
        })?;
    control.check()?;
    summary.timings.index_build += index_build_start.elapsed();

    summary.timings.total = total_start.elapsed();
    Ok(IndexBuildResult { index, summary })
}

fn build_file(
    root: &IndexSourceRoot,
    file: &Path,
    summary: &mut IndexBuildSummary,
    control: &IndexBuildControl,
) -> Result<PendingFileContribution, String> {
    control.check()?;
    let semantic_file_build_start = Instant::now();
    let read_decode_start = Instant::now();
    let bytes =
        fs::read(file).map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
    let byte_count = bytes.len();
    let source = String::from_utf8_lossy(&bytes);
    let lossy = matches!(source, Cow::Owned(_));
    let source = source.into_owned();
    summary.timings.read_decode += read_decode_start.elapsed();

    control.check()?;
    let parse_start = Instant::now();
    let parse = parse_source(&source);
    let parse_diagnostics = parse.diagnostics.len();
    summary.timings.parse += parse_start.elapsed();

    control.check()?;
    let ast_model_catalog_start = Instant::now();
    let semantic_file = SemanticFile::build(&source, &parse);
    semantic_file.contribution().validate().map_err(|error| {
        format!(
            "Invalid semantic contribution for {}: {error:?}",
            file.display()
        )
    })?;
    let indexed_symbols = semantic_file.declarations().len();
    let non_declaration_callable_fragments = semantic_file.non_declaration_callable_fragments();
    summary.timings.ast_model_catalog += ast_model_catalog_start.elapsed();
    summary.timings.catalog_build += semantic_file_build_start.elapsed();

    control.check()?;
    record_file_counts(
        summary,
        root.kind,
        file,
        byte_count,
        lossy,
        &bytes,
        &source,
        &parse.diagnostics,
        parse_diagnostics,
        indexed_symbols,
        non_declaration_callable_fragments,
    );

    Ok(PendingFileContribution {
        contribution: semantic_file.contribution().clone(),
        metadata: source_metadata(&root.root_path, file, root.kind, root.priority),
    })
}

fn record_file_counts(
    summary: &mut IndexBuildSummary,
    kind: SourceKind,
    file: &Path,
    byte_count: usize,
    lossy: bool,
    bytes: &[u8],
    source: &str,
    diagnostics: &[ParseDiagnostic],
    parse_diagnostics: usize,
    indexed_symbols: usize,
    non_declaration_callable_fragments: usize,
) {
    let lossy_detail = if lossy {
        lossy_decode_detail(file, bytes, source)
    } else {
        None
    };
    let diagnostic_details = parse_diagnostic_details(file, source, diagnostics);
    let source_counts = summary.by_source_kind.entry(kind).or_default();
    for counts in [&mut summary.totals, source_counts] {
        counts.files += 1;
        counts.bytes += byte_count;
        counts.indexed_files += 1;
        counts.indexed_symbols += indexed_symbols;
        counts.parse_diagnostics += parse_diagnostics;
        counts.non_declaration_callable_fragments += non_declaration_callable_fragments;
        if parse_diagnostics > 0 {
            counts.parse_diagnostic_files += 1;
            record_parse_diagnostic_details(counts, &diagnostic_details);
        }
        if lossy {
            counts.lossy_files += 1;
            if let Some(detail) = &lossy_detail {
                if counts.lossy_decode_details.len() < MAX_RECORDED_LOSSY_FILES {
                    counts.lossy_decode_details.push(detail.clone());
                }
            }
        }
    }
}

fn record_parse_diagnostic_details(
    counts: &mut IndexBuildCounts,
    details: &[ParseDiagnosticDetail],
) {
    if details.is_empty()
        || counts.parse_diagnostic_details.len() >= diagnostic_detail_capacity()
        || diagnostic_detail_file_count(counts) >= MAX_RECORDED_DIAGNOSTIC_FILES
    {
        return;
    }

    let remaining = diagnostic_detail_capacity() - counts.parse_diagnostic_details.len();
    counts
        .parse_diagnostic_details
        .extend(details.iter().take(remaining).cloned());
}

fn diagnostic_detail_capacity() -> usize {
    MAX_RECORDED_DIAGNOSTIC_FILES * MAX_RECORDED_DIAGNOSTICS_PER_FILE
}

fn diagnostic_detail_file_count(counts: &IndexBuildCounts) -> usize {
    let mut paths = Vec::<&PathBuf>::new();
    for detail in &counts.parse_diagnostic_details {
        if !paths.contains(&&detail.path) {
            paths.push(&detail.path);
        }
    }
    paths.len()
}

fn lossy_decode_detail(file: &Path, bytes: &[u8], source: &str) -> Option<LossyDecodeDetail> {
    let first_replacement_source_offset = source
        .char_indices()
        .find_map(|(offset, value)| (value == REPLACEMENT_CHARACTER).then_some(offset))?;
    let first_replacement_offset =
        first_invalid_utf8_offset(bytes).unwrap_or(first_replacement_source_offset);
    let replacement_count = source
        .chars()
        .filter(|value| *value == REPLACEMENT_CHARACTER)
        .count();
    let (line, column) = line_column(source, first_replacement_source_offset);

    Some(LossyDecodeDetail {
        path: file.to_path_buf(),
        first_replacement_offset,
        line,
        column,
        replacement_count,
        snippet: source_snippet(source, line),
    })
}

fn first_invalid_utf8_offset(bytes: &[u8]) -> Option<usize> {
    std::str::from_utf8(bytes)
        .err()
        .map(|error| error.valid_up_to())
}

fn parse_diagnostic_details(
    file: &Path,
    source: &str,
    diagnostics: &[ParseDiagnostic],
) -> Vec<ParseDiagnosticDetail> {
    diagnostics
        .iter()
        .take(MAX_RECORDED_DIAGNOSTICS_PER_FILE)
        .map(|diagnostic| {
            let (line, column) = line_column(source, diagnostic.span.start);
            ParseDiagnosticDetail {
                path: file.to_path_buf(),
                message: diagnostic.message.clone(),
                span: diagnostic.span,
                line,
                column,
                snippet: source_snippet(source, line),
            }
        })
        .collect()
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

fn source_snippet(source: &str, line: usize) -> String {
    let lines = source.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return "<empty file>\n".to_string();
    }

    let start = line.saturating_sub(SNIPPET_CONTEXT_LINES + 1);
    let end = (line + SNIPPET_CONTEXT_LINES).min(lines.len());
    let mut snippet = String::new();
    for index in start..end {
        let marker = if index + 1 == line { ">" } else { " " };
        snippet.push_str(&format!(
            "{marker} {:>5} | {}\n",
            index + 1,
            render_snippet_line(lines[index])
        ));
    }
    snippet
}

fn render_snippet_line(line: &str) -> String {
    line.replace('\t', "    ")
        .replace(REPLACEMENT_CHARACTER, REPLACEMENT_CHARACTER_LABEL)
}

fn source_metadata(
    root: &Path,
    file: &Path,
    kind: SourceKind,
    priority: u16,
) -> SourceFileMetadata {
    let relative_path = file.strip_prefix(root).unwrap_or(file).to_path_buf();
    SourceFileMetadata {
        kind,
        category: source_category_for_path(kind, Some(&relative_path)),
        absolute_path: Some(file.to_path_buf()),
        root_path: Some(root.to_path_buf()),
        relative_path: Some(relative_path),
        priority,
    }
}

fn collect_script_files(
    folder: &Path,
    files: &mut Vec<PathBuf>,
    control: &IndexBuildControl,
) -> Result<(), String> {
    control.check()?;
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Failed to read folder {}: {error}", folder.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to read entry in {}: {error}", folder.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_script_files(&path, files, control)?;
        } else if path.extension().is_some_and(|extension| extension == "c") {
            files.push(path);
        }
        control.check()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        SourceCategory, SymbolKind, SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE,
    };
    use crate::semantic_file::SemanticFile;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn builds_single_game_data_root_with_metadata() {
        let root = test_root("single_game_data");
        write_file(&root.join("Game.c"), "class GameOnly {}");

        let result = build_index(&IndexBuildConfig {
            roots: vec![IndexSourceRoot::new(
                &root,
                SourceKind::GameData,
                SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap();

        assert_eq!(result.summary.totals.files, 1);
        assert_eq!(result.summary.totals.indexed_files, 1);
        assert_eq!(result.summary.totals.indexed_symbols, 1);
        assert!(result.summary.timings.catalog_build >= result.summary.timings.read_decode);
        assert!(result.summary.timings.catalog_build >= result.summary.timings.parse);
        assert!(result.summary.timings.catalog_build >= result.summary.timings.ast_model_catalog);
        let class = result.index.classes_by_name("GameOnly")[0];
        let file = result.index.file(class.file_id).unwrap();
        assert_eq!(file.metadata.kind, SourceKind::GameData);
        assert_eq!(file.metadata.category.as_str(), "unknown");
        assert_eq!(file.metadata.priority, SOURCE_PRIORITY_GAME_DATA);
        assert_eq!(
            file.metadata.relative_path.as_deref(),
            Some(Path::new("Game.c"))
        );

        cleanup(&root);
    }

    #[test]
    fn preserves_workspace_and_game_data_source_kinds_and_priorities() {
        let game_root = test_root("overlay_game");
        let workspace_root = test_root("overlay_workspace");
        write_file(&game_root.join("Example.c"), "class Example {}");
        write_file(&workspace_root.join("Example.c"), "modded class Example {}");

        let result = build_index(&IndexBuildConfig {
            roots: vec![
                IndexSourceRoot::new(&game_root, SourceKind::GameData, SOURCE_PRIORITY_GAME_DATA),
                IndexSourceRoot::new(
                    &workspace_root,
                    SourceKind::Workspace,
                    SOURCE_PRIORITY_WORKSPACE,
                ),
            ],
        })
        .unwrap();

        let preferred = result.index.preferred_classes_by_name("Example")[0];
        assert_eq!(
            result.index.file(preferred.file_id).unwrap().metadata.kind,
            SourceKind::Workspace
        );
        assert_eq!(
            result
                .summary
                .by_source_kind
                .get(&SourceKind::GameData)
                .unwrap()
                .files,
            1
        );
        assert_eq!(
            result
                .summary
                .by_source_kind
                .get(&SourceKind::Workspace)
                .unwrap()
                .files,
            1
        );

        cleanup(&game_root);
        cleanup(&workspace_root);
    }

    #[test]
    fn assigns_source_categories_from_relative_paths() {
        let root = test_root("source_categories");
        for path in [
            "Game/Runtime.c",
            "GameCode/Faction/FactionKey.c",
            "GameLib/Runtime.c",
            "Core/proto/Types.c",
            "Game/generated/Generated.c",
            "WorkbenchGame/Plugin.c",
            "GameLib/WorldSystemsDocs.c",
            "Autotest/Game/Test.c",
        ] {
            let class_name = path
                .chars()
                .filter(|value| value.is_ascii_alphanumeric())
                .collect::<String>();
            write_file(&root.join(path), &format!("class {class_name} {{}}"));
        }

        let result = build_index(&IndexBuildConfig {
            roots: vec![IndexSourceRoot::new(
                &root,
                SourceKind::GameData,
                SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap();

        let categories = result
            .index
            .files()
            .iter()
            .map(|file| file.metadata.category)
            .collect::<Vec<_>>();

        assert!(categories.contains(&SourceCategory::Game));
        assert!(categories.contains(&SourceCategory::GameCode));
        assert!(categories.contains(&SourceCategory::GameLib));
        assert!(categories.contains(&SourceCategory::Core));
        assert!(categories.contains(&SourceCategory::Generated));
        assert!(categories.contains(&SourceCategory::Workbench));
        assert!(categories.contains(&SourceCategory::DocsDoxygen));
        assert!(categories.contains(&SourceCategory::TestAutotest));

        cleanup(&root);
    }

    #[test]
    fn ignores_non_script_files_and_indexes_deterministically() {
        let root = test_root("deterministic");
        write_file(&root.join("B.c"), "class B {}");
        write_file(&root.join("A.c"), "class A {}");
        write_file(&root.join("Ignored.txt"), "class Ignored {}");

        let result = build_index(&IndexBuildConfig {
            roots: vec![IndexSourceRoot::new(
                &root,
                SourceKind::GameData,
                SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap();

        assert_eq!(result.summary.totals.files, 2);
        assert!(result.index.classes_by_name("Ignored").is_empty());
        assert_eq!(result.index.symbols()[0].name.as_deref(), Some("A"));
        assert_eq!(result.index.symbols()[1].name.as_deref(), Some("B"));

        cleanup(&root);
    }

    #[test]
    fn rebuilds_lookup_maps_once_after_batching_multiple_files() {
        let root = test_root("batched_lookup_rebuild");
        write_file(&root.join("A.c"), "class A {}");
        write_file(&root.join("B.c"), "class B {}");
        write_file(&root.join("C.c"), "class C {}");

        let result = build_index(&IndexBuildConfig {
            roots: vec![IndexSourceRoot::new(
                &root,
                SourceKind::GameData,
                SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap();

        assert_eq!(result.index.lookup_map_rebuild_count(), 1);
        assert_eq!(result.index.classes_by_name("A").len(), 1);
        assert_eq!(result.index.classes_by_name("B").len(), 1);
        assert_eq!(result.index.classes_by_name("C").len(), 1);

        cleanup(&root);
    }

    #[test]
    fn missing_root_returns_clear_error() {
        let root = test_root("missing");
        cleanup(&root);

        let error = build_index(&IndexBuildConfig {
            roots: vec![IndexSourceRoot::new(
                &root,
                SourceKind::GameData,
                SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap_err();

        assert!(error.contains("does not exist or is not a folder"));
    }

    #[test]
    fn lossy_utf8_and_parse_diagnostics_are_counted_without_abort() {
        let root = test_root("lossy_and_diagnostics");
        fs::write(root.join("Lossy.c"), [0xff, b'c', b'l', b'a', b's', b's']).unwrap();
        write_file(&root.join("Malformed.c"), "class Broken {");

        let result = build_index(&IndexBuildConfig {
            roots: vec![IndexSourceRoot::new(
                &root,
                SourceKind::GameData,
                SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap();

        assert_eq!(result.summary.totals.files, 2);
        assert_eq!(result.summary.totals.lossy_files, 1);
        assert_eq!(result.summary.totals.lossy_decode_details.len(), 1);
        let lossy_detail = &result.summary.totals.lossy_decode_details[0];
        assert_eq!(lossy_detail.first_replacement_offset, 0);
        assert_eq!(lossy_detail.line, 1);
        assert_eq!(lossy_detail.column, 1);
        assert_eq!(lossy_detail.replacement_count, 1);
        assert!(lossy_detail.snippet.contains("<U+FFFD>"));
        assert!(!lossy_detail.snippet.contains('\u{FFFD}'));
        assert!(result.summary.totals.parse_diagnostics > 0);
        assert!(result.summary.totals.parse_diagnostic_files > 0);
        assert!(!result.summary.totals.parse_diagnostic_details.is_empty());
        let diagnostic_detail = &result.summary.totals.parse_diagnostic_details[0];
        assert!(!diagnostic_detail.message.is_empty());
        assert!(diagnostic_detail.line > 0);
        assert!(diagnostic_detail.column > 0);
        assert!(result
            .summary
            .totals
            .parse_diagnostic_details
            .iter()
            .any(|detail| detail.snippet.contains("class Broken")));

        cleanup(&root);
    }

    #[test]
    fn matches_direct_semantic_pipeline_for_small_fixture() {
        let root = test_root("matches_direct");
        let file = root.join("Example.c");
        let source = r#"class Example
{
	int m_Value;
	void Run(int value);
}
"#;
        write_file(&file, source);

        let result = build_index(&IndexBuildConfig {
            roots: vec![IndexSourceRoot::new(
                &root,
                SourceKind::GameData,
                SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap();

        let parse = parse_source(source);
        let semantic_file = SemanticFile::build(source, &parse);
        let mut expected = SymbolIndex::default();
        expected
            .add_file_contribution(
                &semantic_file.contribution(),
                source_metadata(
                    &root,
                    &file,
                    SourceKind::GameData,
                    SOURCE_PRIORITY_GAME_DATA,
                ),
            )
            .unwrap();

        assert_eq!(result.index.files(), expected.files());
        assert_eq!(result.index.symbols(), expected.symbols());
        assert_eq!(result.index.symbols_for_kind(SymbolKind::Class).len(), 1);
        assert_eq!(result.index.symbols_for_kind(SymbolKind::Field).len(), 1);
        assert_eq!(result.index.symbols_for_kind(SymbolKind::Method).len(), 1);
        assert_eq!(
            result.index.symbols_for_kind(SymbolKind::Parameter).len(),
            1
        );
        assert_eq!(
            result
                .index
                .callable_signature(result.index.methods_by_owner_name("Example", "Run")[0])
                .as_deref(),
            Some("Example.Run(int value) -> void")
        );

        cleanup(&root);
    }

    fn test_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "reforger_index_build_{name}_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_file(path: &Path, source: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, source).unwrap();
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }
}
