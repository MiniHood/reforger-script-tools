use super::{format_paths, LspLogger, LspServerOptions};
use crate::ast::AstSourceFile;
use crate::index::SymbolIndex;
use crate::index_cache::{
    load_or_build_game_data_index_with_progress, GameDataIndexCacheConfig, RuntimeIndexSummary,
};
use crate::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, SymbolCatalog,
    SOURCE_PRIORITY_WORKSPACE,
};
use crate::parser::parse_source;
use std::collections::BTreeMap;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Clone)]
pub(crate) struct ExternalIndexHandle {
    state: Arc<Mutex<ExternalIndexState>>,
}

#[derive(Debug)]
struct ExternalIndexState {
    status: ExternalIndexStatus,
    generation: u64,
    workspace_index: Option<Arc<SymbolIndex>>,
    game_data_index: Option<Arc<SymbolIndex>>,
    workspace_files: BTreeMap<PathBuf, Arc<WorkspaceIndexedFile>>,
    workspace_live_changes: BTreeMap<PathBuf, Option<Arc<WorkspaceIndexedFile>>>,
    workspace_generation: u64,
    workspace_startup_pending: bool,
    workspace_roots: Vec<PathBuf>,
    summary: Option<RuntimeIndexSummary>,
    workspace_summary: RuntimeIndexSummary,
    game_data_summary: Option<RuntimeIndexSummary>,
    cache_status: Option<String>,
    cache_detail: Option<String>,
    fingerprint: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct WorkspaceIndexedFile {
    index: SymbolIndex,
    bytes: usize,
    parse_diagnostics: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExternalIndexStatus {
    Missing,
    Building,
    Updating,
    Ready,
    Failed,
}

impl ExternalIndexStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Building => "building",
            Self::Updating => "updating",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExternalIndexStatusSummary {
    pub(crate) status: &'static str,
    pub(crate) generation: u64,
    pub(crate) files: usize,
    pub(crate) symbols: usize,
    pub(crate) parse_diagnostics: usize,
    pub(crate) workspace_files: usize,
    pub(crate) workspace_symbols: usize,
    pub(crate) workspace_parse_diagnostics: usize,
    pub(crate) game_data_files: usize,
    pub(crate) game_data_symbols: usize,
    pub(crate) game_data_parse_diagnostics: usize,
    pub(crate) cache_status: Option<String>,
    pub(crate) cache_detail: Option<String>,
    pub(crate) fingerprint: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ExternalIndexSnapshot {
    pub(crate) status: &'static str,
    pub(crate) workspace: Option<Arc<SymbolIndex>>,
    pub(crate) game_data: Option<Arc<SymbolIndex>>,
}

impl ExternalIndexSnapshot {
    pub(crate) fn available_layers(&self) -> &'static str {
        match (self.workspace.is_some(), self.game_data.is_some()) {
            (true, true) => "workspace,game-data",
            (true, false) => "workspace",
            (false, true) => "game-data",
            (false, false) => "none",
        }
    }
}

impl ExternalIndexHandle {
    fn missing() -> Self {
        Self {
            state: Arc::new(Mutex::new(ExternalIndexState {
                status: ExternalIndexStatus::Missing,
                generation: 0,
                workspace_index: None,
                game_data_index: None,
                workspace_files: BTreeMap::new(),
                workspace_live_changes: BTreeMap::new(),
                workspace_generation: 0,
                workspace_startup_pending: false,
                workspace_roots: Vec::new(),
                summary: None,
                workspace_summary: RuntimeIndexSummary::default(),
                game_data_summary: None,
                cache_status: None,
                cache_detail: None,
                fingerprint: None,
                error: None,
            })),
        }
    }

    pub(crate) fn status_summary(&self) -> ExternalIndexStatusSummary {
        let state = self.state.lock().unwrap();
        let summary = state.summary.as_ref();
        let game_data_summary = state.game_data_summary.as_ref();
        ExternalIndexStatusSummary {
            status: state.status.as_str(),
            generation: state.generation,
            files: summary.map(|summary| summary.files).unwrap_or(0),
            symbols: summary.map(|summary| summary.indexed_symbols).unwrap_or(0),
            parse_diagnostics: summary
                .map(|summary| summary.parse_diagnostics)
                .unwrap_or(0),
            workspace_files: state.workspace_summary.files,
            workspace_symbols: state.workspace_summary.indexed_symbols,
            workspace_parse_diagnostics: state.workspace_summary.parse_diagnostics,
            game_data_files: game_data_summary.map(|summary| summary.files).unwrap_or(0),
            game_data_symbols: game_data_summary
                .map(|summary| summary.indexed_symbols)
                .unwrap_or(0),
            game_data_parse_diagnostics: game_data_summary
                .map(|summary| summary.parse_diagnostics)
                .unwrap_or(0),
            cache_status: state.cache_status.clone(),
            cache_detail: state.cache_detail.clone(),
            fingerprint: state.fingerprint.clone(),
            error: state.error.clone(),
        }
    }

    pub(crate) fn snapshot(&self) -> ExternalIndexSnapshot {
        let state = self.state.lock().unwrap();
        ExternalIndexSnapshot {
            status: state.status.as_str(),
            workspace: state.workspace_index.clone(),
            game_data: state.game_data_index.clone(),
        }
    }

    pub(crate) fn update_workspace_file(
        &self,
        path: PathBuf,
        text: String,
    ) -> Result<(usize, usize), String> {
        let normalized_path = normalize_workspace_path(&path);
        let root = {
            let state = self.state.lock().unwrap();
            workspace_root_for_file(&state.workspace_roots, &path)
                .or_else(|| workspace_root_for_file(&state.workspace_roots, &normalized_path))
        }
        .unwrap_or_else(|| {
            normalized_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        });

        let indexed = Arc::new(build_workspace_file_index(&root, &normalized_path, &text));
        let symbol_count = indexed.index.symbols().len();
        let parse_diagnostics = indexed.parse_diagnostics;
        self.publish_workspace_change(normalized_path, Some(indexed));
        Ok((symbol_count, parse_diagnostics))
    }

    pub(crate) fn delete_workspace_file(&self, path: &Path) -> bool {
        let normalized_path = normalize_workspace_path(path);
        self.publish_workspace_change(normalized_path, None)
    }

    fn publish_workspace_change(
        &self,
        path: PathBuf,
        replacement: Option<Arc<WorkspaceIndexedFile>>,
    ) -> bool {
        loop {
            let (mut files, workspace_generation, startup_pending) = {
                let state = self.state.lock().unwrap();
                (
                    state.workspace_files.clone(),
                    state.workspace_generation,
                    state.workspace_startup_pending,
                )
            };
            let removed = replacement.is_none() && files.remove(&path).is_some();
            if let Some(indexed) = replacement.clone() {
                files.insert(path.clone(), indexed);
            }
            let (workspace_index, workspace_summary) = workspace_aggregate(&files);
            let mut state = self.state.lock().unwrap();
            if state.workspace_generation != workspace_generation {
                continue;
            }
            state.status = ExternalIndexStatus::Updating;
            if startup_pending {
                state
                    .workspace_live_changes
                    .insert(path.clone(), replacement.clone());
            }
            state.workspace_files = files;
            state.workspace_index = workspace_index;
            state.workspace_summary = workspace_summary;
            let game_data_summary = state.game_data_summary.clone();
            recompute_summary(&mut state, game_data_summary);
            if replacement.is_some() || removed {
                state.workspace_generation += 1;
                state.generation += 1;
            }
            state.status = if state.workspace_index.is_some() || state.game_data_index.is_some() {
                ExternalIndexStatus::Ready
            } else {
                ExternalIndexStatus::Missing
            };
            return replacement.is_some() || removed;
        }
    }
}

pub(crate) fn start_external_index(
    options: &LspServerOptions,
    logger: LspLogger,
) -> ExternalIndexHandle {
    if (options.game_data_scripts.is_none() || options.index_cache.is_none())
        && options.workspace_scripts.is_empty()
    {
        return ExternalIndexHandle::missing();
    }

    let handle = ExternalIndexHandle {
        state: Arc::new(Mutex::new(ExternalIndexState {
            status: ExternalIndexStatus::Building,
            generation: 0,
            workspace_index: None,
            game_data_index: None,
            workspace_files: BTreeMap::new(),
            workspace_live_changes: BTreeMap::new(),
            workspace_generation: 0,
            workspace_startup_pending: true,
            workspace_roots: options.workspace_scripts.clone(),
            summary: None,
            workspace_summary: RuntimeIndexSummary::default(),
            game_data_summary: None,
            cache_status: None,
            cache_detail: None,
            fingerprint: None,
            error: None,
        })),
    };

    let state = handle.state.clone();
    let scripts_root = options.game_data_scripts.clone();
    let cache_path = options.index_cache.clone();
    let metadata_path = options.game_data_metadata.clone();
    let workspace_roots = options.workspace_scripts.clone();
    thread::spawn(move || {
        let thread_logger = logger.clone();
        let panic_state = state.clone();
        if let Err(payload) = catch_unwind(AssertUnwindSafe(|| {
            run_external_index_thread(
                state,
                scripts_root,
                cache_path,
                metadata_path,
                workspace_roots,
                logger,
            );
        })) {
            let panic_message = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("<non-string panic>");
            thread_logger.log(&format!(
                "externalIndex thread panic error={}",
                panic_message
            ));
            if let Ok(mut state) = panic_state.lock() {
                state.status = ExternalIndexStatus::Failed;
                state.error = Some(format!("external-index startup panicked: {panic_message}"));
                state.generation += 1;
            }
        }
    });

    handle
}

fn run_external_index_thread(
    state: Arc<Mutex<ExternalIndexState>>,
    scripts_root: Option<PathBuf>,
    cache_path: Option<PathBuf>,
    metadata_path: Option<PathBuf>,
    workspace_roots: Vec<PathBuf>,
    logger: LspLogger,
) {
    let start = Instant::now();
    logger.log(&format!(
        "externalIndex start game_data_scripts={} workspace_roots={}",
        scripts_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<unset>".to_string()),
        format_paths(&workspace_roots)
    ));

    let game_data_start = Instant::now();
    let game_data_result = match (scripts_root, cache_path) {
        (Some(scripts_root), Some(cache_path)) => {
            logger.log(&format!(
                "externalIndex gameData start scripts={} cache={}",
                scripts_root.display(),
                cache_path.display()
            ));
            let phase_logger = logger.clone();
            let phase_start = Instant::now();
            Some(load_or_build_game_data_index_with_progress(
                &GameDataIndexCacheConfig {
                    scripts_root,
                    cache_path,
                    metadata_path,
                },
                |phase| {
                    phase_logger.log(&format!(
                        "externalIndex gameData phase={} elapsed_ms={}",
                        phase,
                        phase_start.elapsed().as_millis()
                    ));
                },
            ))
        }
        _ => None,
    };
    let game_data_ready_ms = game_data_start.elapsed().as_millis();
    logger.log(&format!(
        "externalIndex gameData load returned has_result={} elapsed_ms={}",
        game_data_result.is_some(),
        start.elapsed().as_millis()
    ));

    let workspace_start = Instant::now();
    logger.log(&format!(
        "externalIndex workspace start roots={} elapsed_ms={}",
        format_paths(&workspace_roots),
        start.elapsed().as_millis()
    ));
    let workspace_result = build_workspace_indexes(&workspace_roots, &logger, start);
    let workspace_ready_ms = workspace_start.elapsed().as_millis();
    logger.log(&format!(
        "externalIndex workspace load returned success={} elapsed_ms={}",
        workspace_result.is_ok(),
        start.elapsed().as_millis()
    ));

    logger.log(&format!(
        "externalIndex publish start elapsed_ms={}",
        start.elapsed().as_millis()
    ));
    let mut error_messages = Vec::new();
    let (game_data_index, game_data_summary, cache_status, cache_detail, fingerprint) =
        match game_data_result {
            Some(Ok(result)) => {
                let cache_status = result.cache_status.as_str().to_string();
                let cache_detail = result.cache_status.detail().map(str::to_string);
                let fingerprint = result.fingerprint.summary();
                logger.log(&format!(
                "externalIndex gameData ready cache_status={} cache_detail={} files={} symbols={} parse_diagnostics={} cache_file_read_ms={} cache_decode_ms={} cache_validate_ms={} cache_map_rebuild_ms={} cache_total_ms={} elapsed_ms={}",
                cache_status,
                cache_detail.as_deref().unwrap_or("<none>"),
                result.summary.files,
                result.summary.indexed_symbols,
                result.summary.parse_diagnostics,
                result.timings.cache_file_read.as_millis(),
                result.timings.cache_decode.as_millis(),
                result.timings.cache_validate.as_millis(),
                result.timings.map_rebuild.as_millis(),
                result.timings.total.as_millis(),
                start.elapsed().as_millis()
            ));
                (
                    Some(Arc::new(result.index)),
                    Some(result.summary),
                    Some(cache_status),
                    cache_detail,
                    Some(fingerprint),
                )
            }
            Some(Err(error)) => {
                logger.log(&format!(
                    "externalIndex gameData failed error={} elapsed_ms={}",
                    error,
                    start.elapsed().as_millis()
                ));
                error_messages.push(error);
                (None, None, None, None, None)
            }
            None => (None, None, None, None, None),
        };

    let baseline_workspace_files = match workspace_result {
        Ok((workspace_files, workspace_summary)) => {
            logger.log(&format!(
                "externalIndex workspace ready roots={} files={} symbols={} parse_diagnostics={} elapsed_ms={}",
                format_paths(&workspace_roots),
                workspace_summary.files,
                workspace_summary.indexed_symbols,
                workspace_summary.parse_diagnostics,
                start.elapsed().as_millis()
            ));
            workspace_files
        }
        Err(error) => {
            logger.log(&format!(
                "externalIndex workspace failed roots={} error={} elapsed_ms={}",
                format_paths(&workspace_roots),
                error,
                start.elapsed().as_millis()
            ));
            error_messages.push(error);
            BTreeMap::new()
        }
    };

    // Merge startup and live workspace files before aggregation, then publish only if no live
    // edit raced the snapshot. SymbolIndex::merged never runs while the overlay lock is held.
    let (status, summary, workspace_summary, published_game_data_summary) = loop {
        let (live_changes, workspace_generation) = {
            let state = state.lock().unwrap();
            (
                state.workspace_live_changes.clone(),
                state.workspace_generation,
            )
        };
        let mut workspace_files = baseline_workspace_files.clone();
        for (path, replacement) in live_changes {
            match replacement {
                Some(indexed) => {
                    workspace_files.insert(path, indexed);
                }
                None => {
                    workspace_files.remove(&path);
                }
            }
        }
        let (workspace_index, workspace_summary) = workspace_aggregate(&workspace_files);

        let mut state = state.lock().unwrap();
        if state.workspace_generation != workspace_generation {
            continue;
        }
        state.game_data_index = game_data_index.clone();
        state.game_data_summary = game_data_summary.clone();
        state.cache_status = cache_status.clone();
        state.cache_detail = cache_detail.clone();
        state.fingerprint = fingerprint.clone();
        state.workspace_files = workspace_files;
        state.workspace_index = workspace_index;
        state.workspace_summary = workspace_summary;
        state.workspace_live_changes.clear();
        state.workspace_startup_pending = false;
        let current_game_data_summary = state.game_data_summary.clone();
        recompute_summary(&mut state, current_game_data_summary);
        state.generation += 1;
        state.status = if state.workspace_index.is_some() || state.game_data_index.is_some() {
            ExternalIndexStatus::Ready
        } else if error_messages.is_empty() {
            ExternalIndexStatus::Missing
        } else {
            ExternalIndexStatus::Failed
        };
        state.error = (!error_messages.is_empty()).then(|| error_messages.join("; "));
        break (
            state.status,
            state.summary.clone(),
            state.workspace_summary.clone(),
            state.game_data_summary.clone(),
        );
    };

    logger.log(&format!(
        "externalIndex layered status={} files={} symbols={} workspace_files={} workspace_symbols={} game_data_files={} game_data_symbols={} game_data_ready_ms={} workspace_ready_ms={} layered_ready_ms={} elapsed_ms={}",
        status.as_str(),
        summary.as_ref().map(|summary| summary.files).unwrap_or(0),
        summary.as_ref().map(|summary| summary.indexed_symbols).unwrap_or(0),
        workspace_summary.files,
        workspace_summary.indexed_symbols,
        published_game_data_summary.as_ref().map(|summary| summary.files).unwrap_or(0),
        published_game_data_summary.as_ref().map(|summary| summary.indexed_symbols).unwrap_or(0),
        game_data_ready_ms,
        workspace_ready_ms,
        start.elapsed().as_millis(),
        start.elapsed().as_millis()
    ));
}

fn build_workspace_indexes(
    roots: &[PathBuf],
    logger: &LspLogger,
    external_start: Instant,
) -> Result<
    (
        BTreeMap<PathBuf, Arc<WorkspaceIndexedFile>>,
        RuntimeIndexSummary,
    ),
    String,
> {
    let roots = normalize_workspace_roots(roots);
    logger.log(&format!(
        "externalIndex workspace roots normalized requested={} unique={} roots={} elapsed_ms={}",
        roots.requested_count,
        roots.paths.len(),
        format_paths(&roots.paths),
        external_start.elapsed().as_millis()
    ));

    let mut files = Vec::new();
    for root in &roots.paths {
        if !root.is_dir() {
            return Err(format!(
                "Workspace scripts root does not exist or is not a folder: {}",
                root.display()
            ));
        }
        collect_workspace_script_files(root, &mut files)?;
    }
    files.sort();
    files.dedup();
    logger.log(&format!(
        "externalIndex workspace discovered files={} elapsed_ms={}",
        files.len(),
        external_start.elapsed().as_millis()
    ));

    let mut indexed_files = BTreeMap::new();
    for file in files {
        let file_start = Instant::now();
        logger.log(&format!(
            "externalIndex workspace file start path={} total_elapsed_ms={}",
            file.display(),
            external_start.elapsed().as_millis()
        ));
        let bytes = fs::read(&file).map_err(|error| {
            format!("Failed to read workspace file {}: {error}", file.display())
        })?;
        let source = String::from_utf8_lossy(&bytes).into_owned();
        let root = workspace_root_for_file(&roots.paths, &file).unwrap_or_else(|| {
            file.parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        });
        let indexed = build_workspace_file_index(&root, &file, &source);
        logger.log(&format!(
            "externalIndex workspace file indexed path={} bytes={} symbols={} parse_diagnostics={} elapsed_ms={} total_elapsed_ms={}",
            file.display(),
            indexed.bytes,
            indexed.index.symbols().len(),
            indexed.parse_diagnostics,
            file_start.elapsed().as_millis(),
            external_start.elapsed().as_millis()
        ));
        indexed_files.insert(file, Arc::new(indexed));
    }

    let summary = workspace_summary_from_files(&indexed_files);
    Ok((indexed_files, summary))
}

struct NormalizedWorkspaceRoots {
    requested_count: usize,
    paths: Vec<PathBuf>,
}

fn normalize_workspace_roots(roots: &[PathBuf]) -> NormalizedWorkspaceRoots {
    let mut paths = Vec::<PathBuf>::new();
    let mut seen = BTreeMap::<String, ()>::new();
    for root in roots {
        let normalized = normalize_workspace_path(root);
        let key = workspace_path_key(&normalized);
        if seen.insert(key, ()).is_none() {
            paths.push(normalized);
        }
    }
    NormalizedWorkspaceRoots {
        requested_count: roots.len(),
        paths,
    }
}

fn build_workspace_file_index(root: &Path, file: &Path, source: &str) -> WorkspaceIndexedFile {
    let parse = parse_source(source);
    let ast = AstSourceFile::new(source, &parse);
    let catalog =
        SymbolCatalog::from_ast_with_metadata(source, &ast, workspace_source_metadata(root, file));
    let index = SymbolIndex::from_catalogs([&catalog]);
    WorkspaceIndexedFile {
        index,
        bytes: source.len(),
        parse_diagnostics: parse.diagnostics.len(),
    }
}

fn workspace_source_metadata(root: &Path, file: &Path) -> SourceFileMetadata {
    let relative_path = file.strip_prefix(root).unwrap_or(file).to_path_buf();
    SourceFileMetadata {
        kind: SourceKind::Workspace,
        category: source_category_for_path(SourceKind::Workspace, Some(&relative_path)),
        absolute_path: Some(file.to_path_buf()),
        root_path: Some(root.to_path_buf()),
        relative_path: Some(relative_path),
        priority: SOURCE_PRIORITY_WORKSPACE,
    }
}

fn collect_workspace_script_files(folder: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(folder).map_err(|error| {
        format!(
            "Failed to read workspace folder {}: {error}",
            folder.display()
        )
    })? {
        let entry = entry
            .map_err(|error| format!("Failed to read entry in {}: {error}", folder.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_workspace_script_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "c") {
            files.push(path);
        }
    }
    Ok(())
}

fn workspace_root_for_file(roots: &[PathBuf], file: &Path) -> Option<PathBuf> {
    roots
        .iter()
        .filter(|root| file.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned()
}

fn normalize_workspace_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn workspace_path_key(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        raw.to_ascii_lowercase()
    } else {
        raw
    }
}

fn workspace_summary_from_files(
    files: &BTreeMap<PathBuf, Arc<WorkspaceIndexedFile>>,
) -> RuntimeIndexSummary {
    let mut summary = RuntimeIndexSummary::default();
    for file in files.values() {
        summary.files += 1;
        summary.bytes += file.bytes;
        summary.indexed_symbols += file.index.symbols().len();
        summary.parse_diagnostics += file.parse_diagnostics;
    }
    summary
}

fn workspace_aggregate(
    files: &BTreeMap<PathBuf, Arc<WorkspaceIndexedFile>>,
) -> (Option<Arc<SymbolIndex>>, RuntimeIndexSummary) {
    let workspace_indexes = files.values().map(|file| &file.index).collect::<Vec<_>>();
    let workspace_index =
        (!workspace_indexes.is_empty()).then(|| Arc::new(SymbolIndex::merged(workspace_indexes)));
    (workspace_index, workspace_summary_from_files(files))
}

fn recompute_summary(
    state: &mut ExternalIndexState,
    game_data_summary: Option<RuntimeIndexSummary>,
) {
    let game_data_summary = game_data_summary.as_ref();
    state.summary = if state.workspace_index.is_some() || state.game_data_index.is_some() {
        let mut summary = state.workspace_summary.clone();
        if let Some(game_data_summary) = game_data_summary {
            summary.files += game_data_summary.files;
            summary.bytes += game_data_summary.bytes;
            summary.indexed_symbols += game_data_summary.indexed_symbols;
            summary.parse_diagnostics += game_data_summary.parse_diagnostics;
            summary.lossy_files += game_data_summary.lossy_files;
        }
        Some(summary)
    } else {
        None
    };
}
