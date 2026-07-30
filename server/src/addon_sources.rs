use crate::index::SymbolIndex;
use crate::index_build::{
    build_index_from_sources, IndexBuildControl, IndexBuildResult, IndexSourceText,
};
use crate::index_cache::{
    cache_format_identity, load_archive_index_cache, load_or_build_archive_index_with_reuse, write_atomic_bytes,
    GameDataIndexCacheResult, SourceFingerprint,
};
use crate::index_cache::{IndexCacheStatus, RuntimeIndexSummary};
use crate::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, VirtualSourceIdentity,
    SOURCE_PRIORITY_GAME_DATA,
};
use crate::pack::{PakArchive, PakEntry, PakSelection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};
use url::Url;

pub const BASE_GAME_GUID: &str = "58D0FB3206B6F859";
pub const VIRTUAL_SOURCE_SCHEME: &str = "reforger-pak";
const MAX_ADDON_INDEX_WORKERS: usize = 4;

pub struct LoadedAddonIndexResult {
    pub index: SymbolIndex,
    pub summary: RuntimeIndexSummary,
    pub rebuilt_instances: usize,
    pub loaded_instances: usize,
    /// Instances from the current Workbench graph which have no compatible
    /// published snapshot yet. Optimistic delivery omits only these instances
    /// until the validation pass builds and publishes them.
    pub missing_instances: usize,
    pub workspace_excluded_instances: usize,
    pub timings: LoadedAddonIndexTimings,
    pub instances: Vec<LoadedAddonIndexInstance>,
}

/// Bounded performance facts for one authoritative Workbench graph refresh.
/// Paths and source text deliberately stay out of this reporting surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoadedAddonIndexTimings {
    pub graph_read: Duration,
    pub workspace_root_resolution: Duration,
    pub cache_prune: Duration,
    /// Wall-clock time to read and validate `current.json` plus the compact
    /// manifest for every loaded instance. It never reads source files.
    pub cache_metadata_read: Duration,
    /// Wall-clock time spent proving the live loaded source contents before a
    /// cache can be trusted. This includes archive selection and its strong
    /// content identity, not source parsing.
    pub source_inspection: Duration,
    /// Wall-clock time spent loading or rebuilding the inspected add-on
    /// indexes. This is intentionally separate from inspection so warm-cache
    /// regressions cannot be mistaken for source-verification cost.
    pub index_load_or_build: Duration,
    pub layer_rebase: Duration,
    pub layer_file_projection: Duration,
    pub layer_lookup_projection: Duration,
    pub layer_compose: Duration,
    pub total: Duration,
}

/// One non-workspace add-on's cache and indexing outcome from a graph refresh.
#[derive(Debug, Clone)]
pub struct LoadedAddonIndexInstance {
    pub guid: String,
    pub display_id: String,
    pub pack_count: usize,
    pub script_count: usize,
    pub cache_status: String,
    pub cache_detail: Option<String>,
    pub summary: RuntimeIndexSummary,
    pub timings: crate::index_cache::IndexCacheTimings,
    pub cache_file_bytes: Option<u64>,
}

struct InspectedAddonTask {
    sequence: usize,
    addon: LoadedAddonSource,
    inspection: BaseGameInspection,
    inspection_elapsed: Duration,
}

struct PendingAddonInspection {
    sequence: usize,
    addon: LoadedAddonSource,
}

struct CompletedAddonInspection {
    sequence: usize,
    result: Result<InspectedAddonTask, String>,
}

struct CompletedAddonTask {
    sequence: usize,
    addon: LoadedAddonSource,
    result: Result<GameDataIndexCacheResult, String>,
}

struct CachedAddonDescriptor {
    sequence: usize,
    addon: LoadedAddonSource,
    cache_path: PathBuf,
    fingerprint: SourceFingerprint,
    source_digest: String,
}

struct CompletedCachedAddonDescriptor {
    sequence: usize,
    result: Result<Option<CachedAddonDescriptor>, String>,
}

struct CompletedCachedAddonLoad {
    sequence: usize,
    addon: LoadedAddonSource,
    result: Result<GameDataIndexCacheResult, String>,
}

#[derive(Debug, Deserialize)]
struct Inventory {
    #[allow(dead_code)]
    schema: String,
    roots: Vec<InventoryRoot>,
    #[serde(default)]
    addons: Vec<InventoryAddon>,
}

#[derive(Debug, Deserialize)]
struct InventoryRoot {
    kind: String,
    path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryAddon {
    root_kind: String,
    directory_name: String,
    path: PathBuf,
    project_file: Option<PathBuf>,
    #[serde(default)]
    pack_files: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkbenchLoadedAddonGraphInventory {
    schema: String,
    bridge_version: String,
    protocol_version: u32,
    addons: Vec<WorkbenchLoadedAddonInventory>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkbenchLoadedAddonInventory {
    guid: String,
    id: String,
    title: String,
    source_root: PathBuf,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct LoadedAddonSource {
    guid: String,
    id: String,
    title: String,
    source_root: PathBuf,
}

#[derive(Debug)]
struct LoadedAddonGraph {
    addons: Vec<LoadedAddonSource>,
}

struct BaseGameInspection {
    guid: String,
    display_id: String,
    root: PathBuf,
    archives: Vec<(PakArchive, Vec<PakEntry>)>,
    loose_files: Vec<PathBuf>,
    fingerprint: SourceFingerprint,
    artifact_digest: String,
    artifacts: Vec<PackArtifact>,
    scripts: Vec<ScriptLocator>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackArtifact {
    relative_path: String,
    bytes: u64,
    modified_unix_ms: u128,
    selected_payload_sha256: String,
    strong_manifest_sha512: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScriptLocator {
    uri: String,
    logical_path: String,
    pack_relative_path: String,
    offset: u64,
    compressed_length: u64,
    original_length: u64,
    compression: u32,
    compressed_payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddonIndexManifest {
    schema: String,
    cache_schema: String,
    cache_format_version: u32,
    cache_index_shape: String,
    extractor_schema: String,
    guid: String,
    display_id: String,
    source_root: PathBuf,
    source_precedence: String,
    revision: String,
    pack_count: usize,
    script_count: usize,
    pack_artifacts: Vec<PackArtifact>,
    scripts: Vec<ScriptLocator>,
    index_file: String,
    index_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurrentRevision {
    schema: String,
    guid: String,
    instance_key: String,
    source_root: PathBuf,
    revision: String,
    manifest: String,
    index: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InventoryAddonManifest {
    schema: &'static str,
    guid: String,
    display_id: String,
    directory_name: String,
    root_kind: String,
    source_root: PathBuf,
    semantic_status: &'static str,
    pack_artifacts: Vec<InventoryPackArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InventoryPackArtifact {
    path: PathBuf,
    bytes: u64,
    modified_unix_ms: u128,
    strong_manifest_sha512: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryPublication {
    schema: String,
    revision: String,
}

#[derive(Debug)]
struct PackedSourceRevision {
    artifacts: Vec<ArtifactStamp>,
    entries: BTreeMap<String, PackedSourceEntry>,
}

#[derive(Debug)]
struct PackedSourceEntry {
    entry: PakEntry,
    compressed_payload_sha256: String,
}

#[derive(Debug)]
struct ArtifactStamp {
    path: PathBuf,
    bytes: u64,
    modified_unix_ms: u128,
}

static SOURCE_REVISIONS: OnceLock<Mutex<BTreeMap<String, Arc<PackedSourceRevision>>>> =
    OnceLock::new();

/// Builds the installed base-game index directly from selected PAC entries.
/// User add-ons remain inventory-only until load ordering is implemented.
pub fn build_base_game_index(
    inventory_path: &Path,
    control: &IndexBuildControl,
) -> Result<IndexBuildResult, String> {
    let inspection = inspect_base_game(inventory_path, control)?;
    let sources = packed_source_revision(&inspection);
    build_inspected_base_game(
        inspection,
        &sources,
        control,
        standalone_source_build_worker_count(),
    )
}

pub fn load_or_build_base_game_index(
    inventory_path: &Path,
    storage_root: &Path,
    control: &IndexBuildControl,
) -> Result<GameDataIndexCacheResult, String> {
    let inspection_started = std::time::Instant::now();
    let inspection = inspect_base_game(inventory_path, control)?;
    load_or_build_inspected_addon(
        inspection,
        storage_root,
        control,
        inspection_started.elapsed(),
        standalone_source_build_worker_count(),
    )
}

/// Builds an independent compact index for every packed add-on that the live
/// Workbench graph names, then composes those immutable indexes by stable
/// instance identity without copying their symbol records. A graph entry is
/// never inferred from a directory scan.
pub fn load_or_build_loaded_addon_indexes(
    inventory_path: &Path,
    storage_root: &Path,
    workspace_roots: &[PathBuf],
    control: &IndexBuildControl,
) -> Result<LoadedAddonIndexResult, String> {
    let total_start = Instant::now();
    let graph_start = Instant::now();
    let graph = read_loaded_addon_graph(inventory_path)?;
    let graph_read = graph_start.elapsed();
    let workspace_root_start = Instant::now();
    let workspace_roots = workspace_roots
        .iter()
        .filter_map(|root| fs::canonicalize(root).ok())
        .collect::<Vec<_>>();
    let workspace_root_resolution = workspace_root_start.elapsed();
    let mut addons = graph.addons;
    let cache_prune_start = Instant::now();
    prune_unloaded_addon_caches(storage_root, &addons)?;
    let cache_prune = cache_prune_start.elapsed();
    addons.sort_by(|left, right| {
        (&left.guid, &left.source_root, &left.id).cmp(&(&right.guid, &right.source_root, &right.id))
    });
    let mut pending_inspections = Vec::with_capacity(addons.len());
    let mut summary = RuntimeIndexSummary::default();
    let mut rebuilt_instances = 0;
    let mut loaded_instances = 0;
    let mut workspace_excluded_instances = 0;
    let source_inspection_start = Instant::now();
    for (sequence, addon) in addons.into_iter().enumerate() {
        control.check()?;
        if workspace_roots
            .iter()
            .any(|workspace_root| workspace_root.starts_with(&addon.source_root))
        {
            remove_workspace_addon_cache(storage_root, &addon)?;
            workspace_excluded_instances += 1;
            continue;
        }
        pending_inspections.push(PendingAddonInspection {
            sequence,
            addon,
        });
    }
    let inspection_task_count = pending_inspections.len();
    let pending_inspections = Arc::new(Mutex::new(pending_inspections));
    let (inspection_sender, inspection_receiver) = mpsc::channel();
    let mut inspection_workers = Vec::with_capacity(addon_inspection_worker_count(
        inspection_task_count,
    ));
    for _ in 0..addon_inspection_worker_count(inspection_task_count) {
        let pending_inspections = pending_inspections.clone();
        let inspection_sender = inspection_sender.clone();
        let control = control.clone();
        inspection_workers.push(thread::spawn(move || loop {
            let pending = pending_inspections.lock().unwrap().pop();
            let Some(pending) = pending else {
                return;
            };
            let sequence = pending.sequence;
            let result = control.check().and_then(|()| {
                let started = Instant::now();
                let archives = addon_archive_paths(&pending.addon.source_root)?;
                let inspection = inspect_packed_addon(
                    pending.addon.guid.clone(),
                    format!("{} ({})", pending.addon.id, pending.addon.title),
                    pending.addon.source_root.clone(),
                    archives,
                    &control,
                )?;
                Ok(InspectedAddonTask {
                    sequence,
                    addon: pending.addon,
                    inspection,
                    inspection_elapsed: started.elapsed(),
                })
            });
            let _ = inspection_sender.send(CompletedAddonInspection { sequence, result });
        }));
    }
    drop(inspection_sender);
    let mut tasks = Vec::with_capacity(inspection_task_count);
    for _ in 0..inspection_task_count {
        let completed = inspection_receiver
            .recv()
            .map_err(|error| format!("Add-on inspection worker ended unexpectedly: {error}"))?;
        tasks.push(completed);
    }
    for worker in inspection_workers {
        worker
            .join()
            .map_err(|_| "Add-on inspection worker panicked".to_string())?;
    }
    tasks.sort_by_key(|task| task.sequence);
    let mut tasks = tasks
        .into_iter()
        .map(|task| task.result)
        .collect::<Result<Vec<_>, _>>()?;
    let source_inspection = source_inspection_start.elapsed();
    let task_count = tasks.len();
    // Largest script sets start first, so the bounded worker pool overlaps the
    // known long tail without attaching behavior to a particular add-on name.
    tasks.sort_by(|left, right| {
        let left_weight = left.inspection.scripts.len() + left.inspection.loose_files.len();
        let right_weight = right.inspection.scripts.len() + right.inspection.loose_files.len();
        left_weight
            .cmp(&right_weight)
            .then_with(|| right.sequence.cmp(&left.sequence))
    });
    let tasks = Arc::new(Mutex::new(tasks));
    let (completed_sender, completed_receiver) = mpsc::channel();
    let index_load_or_build_start = Instant::now();
    let worker_count = addon_index_worker_count(storage_root, task_count)?;
    let source_build_worker_count = source_build_worker_count(worker_count);
    let mut workers = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let tasks = tasks.clone();
        let completed_sender = completed_sender.clone();
        let storage_root = storage_root.to_path_buf();
        let control = control.clone();
        workers.push(thread::spawn(move || loop {
            let task = tasks.lock().unwrap().pop();
            let Some(task) = task else {
                return;
            };
            let result = control.check().and_then(|()| {
                load_or_build_inspected_addon(
                    task.inspection,
                    &storage_root,
                    &control,
                    task.inspection_elapsed,
                    source_build_worker_count,
                )
            });
            let _ = completed_sender.send(CompletedAddonTask {
                sequence: task.sequence,
                addon: task.addon,
                result,
            });
        }));
    }
    drop(completed_sender);
    let mut completed = Vec::with_capacity(task_count);
    for _ in 0..task_count {
        completed.push(
            completed_receiver
                .recv()
                .map_err(|error| format!("Add-on index worker ended unexpectedly: {error}"))?,
        );
    }
    for worker in workers {
        worker
            .join()
            .map_err(|_| "Add-on index worker panicked".to_string())?;
    }
    let index_load_or_build = index_load_or_build_start.elapsed();
    completed.sort_by_key(|task| task.sequence);
    let mut indexes = Vec::with_capacity(completed.len());
    let mut instances = Vec::with_capacity(completed.len());
    for completed in completed {
        let addon = completed.addon;
        let result = completed.result?;
        match &result.cache_status {
            IndexCacheStatus::Loaded => loaded_instances += 1,
            IndexCacheStatus::Rebuilt { .. } => rebuilt_instances += 1,
        }
        summary.files += result.summary.files;
        summary.bytes += result.summary.bytes;
        summary.indexed_symbols += result.summary.indexed_symbols;
        summary.parse_diagnostics += result.summary.parse_diagnostics;
        summary.lossy_files += result.summary.lossy_files;
        let (pack_count, script_count) = match &result.fingerprint {
            SourceFingerprint::Addon {
                pack_count,
                catalogue_entry_count,
                ..
            } => (*pack_count, *catalogue_entry_count),
            _ => unreachable!("loaded add-on indexing always has an add-on fingerprint"),
        };
        instances.push(LoadedAddonIndexInstance {
            guid: addon.guid,
            display_id: addon.id,
            pack_count,
            script_count,
            cache_status: result.cache_status.as_str().to_string(),
            cache_detail: result.cache_status.detail().map(str::to_string),
            summary: result.summary.clone(),
            timings: result.timings,
            cache_file_bytes: result.cache_file_bytes,
        });
        indexes.push(result.index);
    }
    let (index, layer_timings) = SymbolIndex::layered_with_timings(indexes);
    Ok(LoadedAddonIndexResult {
        index,
        summary,
        rebuilt_instances,
        loaded_instances,
        missing_instances: 0,
        workspace_excluded_instances,
        timings: LoadedAddonIndexTimings {
            graph_read,
            workspace_root_resolution,
            cache_prune,
            cache_metadata_read: Duration::ZERO,
            source_inspection,
            index_load_or_build,
            layer_rebase: layer_timings.rebase,
            layer_file_projection: layer_timings.file_projection,
            layer_lookup_projection: layer_timings.lookup_projection,
            layer_compose: layer_timings.total,
            total: total_start.elapsed(),
        },
        instances,
    })
}

/// Delivers every compatible cache revision named by the current Workbench
/// graph without inspecting its packed or loose source bytes. The validation
/// pass always follows this function and is the sole authority that replaces
/// a stale snapshot. Both source forms share this exact lifecycle.
pub fn load_cached_loaded_addon_indexes(
    inventory_path: &Path,
    storage_root: &Path,
    workspace_roots: &[PathBuf],
    control: &IndexBuildControl,
) -> Result<LoadedAddonIndexResult, String> {
    let total_start = Instant::now();
    let graph_start = Instant::now();
    let graph = read_loaded_addon_graph(inventory_path)?;
    let graph_read = graph_start.elapsed();
    let workspace_root_start = Instant::now();
    let workspace_roots = workspace_roots
        .iter()
        .filter_map(|root| fs::canonicalize(root).ok())
        .collect::<Vec<_>>();
    let workspace_root_resolution = workspace_root_start.elapsed();
    let mut addons = graph.addons;
    let cache_prune_start = Instant::now();
    prune_unloaded_addon_caches(storage_root, &addons)?;
    let cache_prune = cache_prune_start.elapsed();
    addons.sort_by(|left, right| {
        (&left.guid, &left.source_root, &left.id).cmp(&(&right.guid, &right.source_root, &right.id))
    });

    let mut pending = Vec::with_capacity(addons.len());
    let mut workspace_excluded_instances = 0;
    for (sequence, addon) in addons.into_iter().enumerate() {
        control.check()?;
        if workspace_roots
            .iter()
            .any(|workspace_root| workspace_root.starts_with(&addon.source_root))
        {
            remove_workspace_addon_cache(storage_root, &addon)?;
            workspace_excluded_instances += 1;
            continue;
        }
        pending.push((sequence, addon));
    }

    let cache_metadata_start = Instant::now();
    let metadata_task_count = pending.len();
    let pending = Arc::new(Mutex::new(pending));
    let (metadata_sender, metadata_receiver) = mpsc::channel();
    let metadata_worker_count = addon_index_worker_count(storage_root, metadata_task_count)?;
    let mut metadata_workers = Vec::with_capacity(metadata_worker_count);
    for _ in 0..metadata_worker_count {
        let pending = pending.clone();
        let metadata_sender = metadata_sender.clone();
        let storage_root = storage_root.to_path_buf();
        let control = control.clone();
        metadata_workers.push(thread::spawn(move || loop {
            let Some((sequence, addon)) = pending.lock().unwrap().pop() else {
                return;
            };
            let result = control.check().and_then(|()| {
                current_addon_cache_descriptor(&storage_root, &addon).map(|descriptor| {
                    descriptor.map(|(cache_path, fingerprint, source_digest)| CachedAddonDescriptor {
                        sequence,
                        addon: addon.clone(),
                        cache_path,
                        fingerprint,
                        source_digest,
                    })
                })
            });
            let _ = metadata_sender.send(CompletedCachedAddonDescriptor { sequence, result });
        }));
    }
    drop(metadata_sender);
    let mut metadata = Vec::with_capacity(metadata_task_count);
    for _ in 0..metadata_task_count {
        metadata.push(metadata_receiver.recv().map_err(|error| {
            format!("Add-on cache metadata worker ended unexpectedly: {error}")
        })?);
    }
    for worker in metadata_workers {
        worker.join().map_err(|_| "Add-on cache metadata worker panicked".to_string())?;
    }
    let cache_metadata_read = cache_metadata_start.elapsed();
    metadata.sort_by_key(|completed| completed.sequence);
    let mut missing_instances = 0;
    let mut descriptors = Vec::with_capacity(metadata.len());
    for completed in metadata {
        match completed.result? {
            Some(descriptor) => descriptors.push(descriptor),
            None => missing_instances += 1,
        }
    }

    let cache_load_start = Instant::now();
    let descriptors = Arc::new(Mutex::new(descriptors));
    let (cache_sender, cache_receiver) = mpsc::channel();
    let cache_task_count = descriptors.lock().unwrap().len();
    let cache_worker_count = addon_index_worker_count(storage_root, cache_task_count)?;
    let mut cache_workers = Vec::with_capacity(cache_worker_count);
    for _ in 0..cache_worker_count {
        let descriptors = descriptors.clone();
        let cache_sender = cache_sender.clone();
        let control = control.clone();
        cache_workers.push(thread::spawn(move || loop {
            let Some(descriptor) = descriptors.lock().unwrap().pop() else {
                return;
            };
            let result = control.check().and_then(|()| {
                load_archive_index_cache(
                    &descriptor.cache_path,
                    descriptor.fingerprint,
                    descriptor.source_digest,
                )?
                .ok_or_else(|| "Current add-on cache became unavailable during hydration".to_string())
            });
            let _ = cache_sender.send(CompletedCachedAddonLoad {
                sequence: descriptor.sequence,
                addon: descriptor.addon,
                result,
            });
        }));
    }
    drop(cache_sender);
    let mut completed = Vec::with_capacity(cache_task_count);
    for _ in 0..cache_task_count {
        completed.push(cache_receiver.recv().map_err(|error| {
            format!("Add-on cache load worker ended unexpectedly: {error}")
        })?);
    }
    for worker in cache_workers {
        worker.join().map_err(|_| "Add-on cache load worker panicked".to_string())?;
    }
    let index_load_or_build = cache_load_start.elapsed();
    completed.sort_by_key(|completed| completed.sequence);
    let mut summary = RuntimeIndexSummary::default();
    let mut loaded_instances = 0;
    let mut indexes = Vec::with_capacity(completed.len());
    let mut instances = Vec::with_capacity(completed.len());
    for completed in completed {
        let addon = completed.addon;
        let result = completed.result?;
        loaded_instances += 1;
        summary.files += result.summary.files;
        summary.bytes += result.summary.bytes;
        summary.indexed_symbols += result.summary.indexed_symbols;
        summary.parse_diagnostics += result.summary.parse_diagnostics;
        summary.lossy_files += result.summary.lossy_files;
        let (pack_count, script_count) = match &result.fingerprint {
            SourceFingerprint::Addon { pack_count, catalogue_entry_count, .. } => {
                (*pack_count, *catalogue_entry_count)
            }
            _ => unreachable!("loaded add-on cache always has an add-on fingerprint"),
        };
        instances.push(LoadedAddonIndexInstance {
            guid: addon.guid,
            display_id: addon.id,
            pack_count,
            script_count,
            cache_status: "optimistic-loaded".to_string(),
            cache_detail: Some("source-validation-pending".to_string()),
            summary: result.summary.clone(),
            timings: result.timings,
            cache_file_bytes: result.cache_file_bytes,
        });
        indexes.push(result.index);
    }
    let (index, layer_timings) = SymbolIndex::layered_with_timings(indexes);
    Ok(LoadedAddonIndexResult {
        index,
        summary,
        rebuilt_instances: 0,
        loaded_instances,
        missing_instances,
        workspace_excluded_instances,
        timings: LoadedAddonIndexTimings {
            graph_read,
            workspace_root_resolution,
            cache_prune,
            cache_metadata_read,
            source_inspection: Duration::ZERO,
            index_load_or_build,
            layer_rebase: layer_timings.rebase,
            layer_file_projection: layer_timings.file_projection,
            layer_lookup_projection: layer_timings.lookup_projection,
            layer_compose: layer_timings.total,
            total: total_start.elapsed(),
        },
        instances,
    })
}

fn addon_index_worker_count(storage_root: &Path, task_count: usize) -> Result<usize, String> {
    if task_count == 0 {
        return Ok(0);
    }
    if !addon_index_storage_is_empty(storage_root)? {
        return Ok(2.min(task_count));
    }
    let logical_cpus = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);
    Ok(addon_index_worker_count_for(logical_cpus, task_count))
}

fn addon_index_storage_is_empty(storage_root: &Path) -> Result<bool, String> {
    match fs::read_dir(storage_root) {
        Ok(mut entries) => Ok(entries.next().is_none()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(format!(
            "Failed to inspect add-on index storage {}: {error}",
            storage_root.display()
        )),
    }
}

fn addon_index_worker_count_for(logical_cpus: usize, task_count: usize) -> usize {
    logical_cpus
        .max(1)
        .min(MAX_ADDON_INDEX_WORKERS)
        .min(task_count)
}

fn source_build_worker_count(addon_worker_count: usize) -> usize {
    let logical_cpus = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);
    (logical_cpus / addon_worker_count.max(1)).max(1)
}

fn standalone_source_build_worker_count() -> usize {
    source_build_worker_count(1)
}

fn addon_inspection_worker_count(task_count: usize) -> usize {
    if task_count == 0 {
        return 0;
    }
    let logical_cpus = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);
    addon_index_worker_count_for(logical_cpus, task_count)
}

/// Removes cache roots which are not physical instances in the current
/// Workbench graph. A GUID can move from an unpacked workspace copy to a
/// packed copy (or vice versa), but only the root currently selected by
/// Workbench may remain persisted.
fn prune_unloaded_addon_caches(
    storage_root: &Path,
    loaded_addons: &[LoadedAddonSource],
) -> Result<(), String> {
    let active = loaded_addons
        .iter()
        .map(|addon| addon_instance_key(&addon.guid, &addon.source_root))
        .collect::<HashSet<_>>();
    let entries = match fs::read_dir(storage_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Failed to read add-on index storage {}: {error}",
                storage_root.display()
            ))
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let key = entry.file_name();
        let key = key.to_string_lossy();
        if !active.contains(key.as_ref())
            && is_addon_instance_key(&key)
            && entry.file_type().map_err(|error| error.to_string())?.is_dir()
        {
            fs::remove_dir_all(entry.path()).map_err(|error| {
                format!("Failed to remove inactive add-on cache {}: {error}", entry.path().display())
            })?;
        }
    }
    Ok(())
}

fn is_addon_instance_key(value: &str) -> bool {
    let Some((guid, digest)) = value.split_once('-') else {
        return false;
    };
    guid.len() == 16
        && guid.bytes().all(|byte| byte.is_ascii_hexdigit())
        && digest.len() == 64
        && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn manifest_matches_current_source(
    manifest: &AddonIndexManifest,
    cache_schema: &str,
    cache_format_version: u32,
    cache_index_shape: &str,
    guid: &str,
    display_id: &str,
    source_root: &Path,
    revision: &str,
    pack_count: usize,
    script_count: usize,
    pack_artifacts: &[PackArtifact],
    index_bytes: u64,
) -> bool {
    manifest.schema == "reforger-addon-index-manifest-v2"
        && manifest.cache_schema == cache_schema
        && manifest.cache_format_version == cache_format_version
        && manifest.cache_index_shape == cache_index_shape
        && manifest.extractor_schema == "pac1-selected-script-payload-v2"
        && manifest.guid == guid
        && manifest.display_id == display_id
        && manifest.source_root == source_root
        && manifest.source_precedence == "Workbench loaded add-on order"
        && manifest.revision == revision
        && manifest.pack_count == pack_count
        && manifest.script_count == script_count
        && manifest.pack_artifacts == pack_artifacts
        && manifest.index_file == "symbols.bin"
        && manifest.index_bytes == index_bytes
}

/// Opens the one revision that this exact Workbench instance last published.
/// This validates cache-format and instance identity only; it deliberately
/// performs no source I/O so delivery stays independent from inspection.
fn current_addon_cache_descriptor(
    storage_root: &Path,
    addon: &LoadedAddonSource,
) -> Result<Option<(PathBuf, SourceFingerprint, String)>, String> {
    let instance_key = addon_instance_key(&addon.guid, &addon.source_root);
    let addon_root = storage_root.join(&instance_key);
    let current_path = addon_root.join("current.json");
    let Some(current) = fs::read(&current_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<CurrentRevision>(&bytes).ok())
    else {
        return Ok(None);
    };
    if current.schema != "reforger-addon-current-revision-v1"
        || current.guid != addon.guid
        || current.instance_key != instance_key
        || current.source_root != addon.source_root
        || !is_sha256_digest(&current.revision)
        || current.manifest != format!("revisions/{}/manifest.json", current.revision)
        || current.index != format!("revisions/{}/symbols.bin", current.revision)
    {
        return Ok(None);
    }
    let manifest_path = addon_root.join(&current.manifest);
    let Some(manifest) = fs::read(&manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<AddonIndexManifest>(&bytes).ok())
    else {
        return Ok(None);
    };
    let (cache_schema, cache_format_version, cache_index_shape) = cache_format_identity();
    if manifest.schema != "reforger-addon-index-manifest-v2"
        || manifest.cache_schema != cache_schema
        || manifest.cache_format_version != cache_format_version
        || manifest.cache_index_shape != cache_index_shape
        || manifest.extractor_schema != "pac1-selected-script-payload-v2"
        || manifest.guid != addon.guid
        || manifest.display_id != format!("{} ({})", addon.id, addon.title)
        || manifest.source_root != addon.source_root
        || manifest.source_precedence != "Workbench loaded add-on order"
        || manifest.revision != current.revision
        || manifest.index_file != "symbols.bin"
    {
        return Ok(None);
    }
    let fingerprint = SourceFingerprint::Addon {
        guid: addon.guid.clone(),
        artifact_digest: current.revision.clone(),
        pack_count: manifest.pack_count,
        catalogue_entry_count: manifest.script_count,
    };
    Ok(Some((addon_root.join(&current.index), fingerprint, current.revision)))
}

fn is_sha256_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn load_or_build_inspected_addon(
    inspection: BaseGameInspection,
    storage_root: &Path,
    control: &IndexBuildControl,
    inspection_elapsed: std::time::Duration,
    source_build_worker_count: usize,
) -> Result<GameDataIndexCacheResult, String> {
    let source_root = inspection.root.clone();
    let inspection_source_root = inspection.root.clone();
    let addon_guid = inspection.guid.clone();
    let addon_display_id = inspection.display_id.clone();
    let pack_artifacts = inspection.artifacts.clone();
    let scripts = inspection.scripts.clone();
    let fingerprint = inspection.fingerprint.clone();
    let artifact_digest = inspection.artifact_digest.clone();
    let (pack_count, script_count) = match &fingerprint {
        SourceFingerprint::Addon {
            pack_count,
            catalogue_entry_count,
            ..
        } => (*pack_count, *catalogue_entry_count),
        _ => unreachable!("base-game PAC inspection always creates an add-on fingerprint"),
    };
    let instance_key = addon_instance_key(&inspection.guid, &source_root);
    let addon_root = storage_root.join(&instance_key);
    let revision_relative = format!("revisions/{artifact_digest}");
    let revision_root = addon_root.join(&revision_relative);
    let cache = revision_root.join("symbols.bin");
    let manifest_path = revision_root.join("manifest.json");
    let (cache_schema, cache_format_version, cache_index_shape) = cache_format_identity();
    let cache_bytes = cache.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    let manifest_reusable = fs::read(&manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<AddonIndexManifest>(&bytes).ok())
        .is_some_and(|manifest| manifest_matches_current_source(
            &manifest, cache_schema, cache_format_version, cache_index_shape,
            &addon_guid, &addon_display_id, &source_root, &artifact_digest,
            pack_count, script_count, &pack_artifacts, cache_bytes,
        ));
    let rebuild_reason = if manifest_reusable {
        "cache-missing-invalid-or-source-changed"
    } else if cache.is_file() {
        "manifest-mismatch"
    } else {
        "cache-missing"
    };
    let source_revision = packed_source_revision(&inspection);
    let build_sources = source_revision.clone();
    let mut result = load_or_build_archive_index_with_reuse(
        &cache,
        fingerprint,
        artifact_digest.clone(),
        manifest_reusable,
        rebuild_reason,
        || {
            build_inspected_base_game(
                inspection,
                &build_sources,
                control,
                source_build_worker_count,
            )
        },
    )?;
    result.timings.fingerprint = inspection_elapsed;
    result.timings.total += inspection_elapsed;
    if matches!(result.cache_status, IndexCacheStatus::Rebuilt { .. }) {
        let cache_metadata_publish_start = Instant::now();
        control.check()?;
        let manifest_scripts = scripts
            .into_iter()
            .map(|mut script| {
                script.uri =
                    virtual_source_uri(&addon_guid, &artifact_digest, &script.logical_path)?;
                Ok(script)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let manifest = AddonIndexManifest {
            schema: "reforger-addon-index-manifest-v2".to_string(),
            cache_schema: cache_schema.to_string(),
            cache_format_version,
            cache_index_shape: cache_index_shape.to_string(),
            extractor_schema: "pac1-selected-script-payload-v2".to_string(),
            guid: addon_guid.clone(),
            display_id: addon_display_id,
            source_root,
            source_precedence: "Workbench loaded add-on order".to_string(),
            revision: artifact_digest.clone(),
            pack_count,
            script_count,
            pack_artifacts,
            scripts: manifest_scripts,
            index_file: "symbols.bin".to_string(),
            index_bytes: result.cache_file_bytes.unwrap_or(0),
        };
        write_json_atomic(&manifest_path, &manifest)?;
        control.check()?;
        let current = CurrentRevision {
            schema: "reforger-addon-current-revision-v1".to_string(),
            guid: addon_guid.clone(),
            instance_key,
            source_root: inspection_source_root,
            revision: artifact_digest.clone(),
            manifest: format!("{revision_relative}/manifest.json"),
            index: format!("{revision_relative}/symbols.bin"),
        };
        write_json_atomic(&addon_root.join("current.json"), &current)?;
        prune_stale_revisions(&addon_root, &artifact_digest)?;
        result.timings.cache_metadata_publish = cache_metadata_publish_start.elapsed();
        result.timings.total += result.timings.cache_metadata_publish;
    }
    register_source_revision(&addon_guid, &artifact_digest, source_revision);
    Ok(result)
}

/// Keeps the completed cache for this loaded instance bounded to its current
/// authoritative source identity. A replacement is published before stale
/// revisions are removed, so a failed rebuild never deletes the usable cache.
fn prune_stale_revisions(addon_root: &Path, current_revision: &str) -> Result<(), String> {
    let revisions_root = addon_root.join("revisions");
    let entries = match fs::read_dir(&revisions_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Failed to read add-on cache revisions {}: {error}",
                revisions_root.display()
            ))
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_name() != current_revision
            && entry
                .file_type()
                .map_err(|error| error.to_string())?
                .is_dir()
        {
            fs::remove_dir_all(entry.path()).map_err(|error| {
                format!("Failed to remove stale add-on cache revision: {error}")
            })?;
        }
    }
    Ok(())
}

/// Workspace scripts are live inputs, so their matching Workbench instance
/// must never retain a packed cache that can shadow or duplicate that source.
fn remove_workspace_addon_cache(
    storage_root: &Path,
    addon: &LoadedAddonSource,
) -> Result<(), String> {
    let addon_root = storage_root.join(addon_instance_key(&addon.guid, &addon.source_root));
    match fs::remove_dir_all(&addon_root) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "Failed to remove obsolete workspace add-on cache {}: {error}",
            addon_root.display()
        )),
    }
}

fn loose_script_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let entries = fs::read_dir(root).map_err(|error| {
        format!(
            "Failed to read Workbench add-on root {}: {error}",
            root.display()
        )
    })?;
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir()
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("scripts"))
        {
            collect_loose_script_paths(&path, &mut paths)?;
        }
    }
    paths.sort();
    Ok(paths)
}

fn collect_loose_script_paths(root: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|error| {
        format!(
            "Failed to read Workbench add-on root {}: {error}",
            root.display()
        )
    })? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            collect_loose_script_paths(&path, paths)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("c"))
        {
            paths.push(path);
        }
    }
    Ok(())
}

pub fn publish_inventory_addon_manifests_from_path(
    inventory_path: &Path,
    storage_root: &Path,
    control: &IndexBuildControl,
) -> Result<(), String> {
    control.check()?;
    let inventory = read_inventory(inventory_path)?;
    publish_inventory_addon_manifests(&inventory, storage_root, control)
}

/// Resolves one immutable virtual document from its PAC catalogue entry. Only
/// the requested source payload is decoded.
pub fn read_virtual_source(uri: &str) -> Result<String, String> {
    let parsed = Url::parse(uri).map_err(|error| format!("Invalid pack source URI: {error}"))?;
    if parsed.scheme() != VIRTUAL_SOURCE_SCHEME {
        return Err(format!(
            "Unsupported source URI scheme '{}'",
            parsed.scheme()
        ));
    }
    let guid = parsed
        .host_str()
        .ok_or_else(|| "Pack source URI has no add-on GUID".to_string())?
        .to_ascii_uppercase();
    let mut path = parsed.path().trim_start_matches('/').splitn(2, '/');
    let revision = path
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Pack source URI has no revision".to_string())?;
    let logical_path = path
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Pack source URI has no logical path".to_string())?;
    // VS Code normalizes custom URI authorities (including the GUID's case).
    // The registry is keyed by the canonical identity emitted during indexing,
    // never by a client-provided serialization.
    let canonical_uri = virtual_source_uri(&guid, revision, logical_path)?;
    let key = revision_key(&guid, revision);
    let sources = SOURCE_REVISIONS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "Packed source revisions are unavailable".to_string())?
        .get(&key)
        .cloned()
        .ok_or_else(|| format!("Add-on {guid} revision {revision} is not loaded"))?;
    sources.validate_artifacts()?;
    let source_entry = sources
        .entries
        .get(&canonical_uri)
        .ok_or_else(|| format!("Pack source does not exist: {logical_path}"))?;
    let entry = &source_entry.entry;
    let archive = PakArchive::inspect(entry.archive_path()).map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(entry.original_length() as usize);
    archive
        .reader()
        .map_err(|error| error.to_string())?
        .read_verified_to_with_cancel(
            entry,
            &source_entry.compressed_payload_sha256,
            &mut bytes,
            || false,
        )
        .map_err(|error| error.to_string())?;
    String::from_utf8(bytes).map_err(|_| format!("Pack source {logical_path} is not UTF-8"))
}

impl PackedSourceRevision {
    fn validate_artifacts(&self) -> Result<(), String> {
        for expected in &self.artifacts {
            let metadata = fs::metadata(&expected.path)
                .map_err(|error| format!("Packed source revision is unavailable: {error}"))?;
            let modified = modified_unix_ms(&metadata);
            if metadata.len() != expected.bytes || modified != expected.modified_unix_ms {
                return Err(format!(
                    "Packed source revision changed on disk: {}",
                    expected.path.display()
                ));
            }
        }
        Ok(())
    }
}

fn inspect_base_game(
    inventory_path: &Path,
    control: &IndexBuildControl,
) -> Result<BaseGameInspection, String> {
    let root = base_game_root(inventory_path)?;
    inspect_packed_addon(
        BASE_GAME_GUID.to_string(),
        "Arma Reforger base game".to_string(),
        root.clone(),
        base_game_archive_paths(&root).to_vec(),
        control,
    )
}

fn inspect_packed_addon(
    guid: String,
    display_id: String,
    root: PathBuf,
    archive_paths: Vec<PathBuf>,
    control: &IndexBuildControl,
) -> Result<BaseGameInspection, String> {
    let mut hasher = Sha256::new();
    hasher.update(b"reforger-base-pac-catalogue-v2");
    let (cache_schema, cache_version, cache_shape) = cache_format_identity();
    hasher.update(cache_schema.as_bytes());
    hasher.update(cache_version.to_le_bytes());
    hasher.update(cache_shape.as_bytes());
    hasher.update(b"pac1-selected-script-payload-v2");
    // A virtual packed-source revision is also an add-on-instance identity.
    // Two loaded roots can legitimately share GUID and bytes, but their source
    // URIs must still never target the other instance's archive.
    hasher.update(root.to_string_lossy().to_ascii_lowercase().as_bytes());
    let mut archives = Vec::new();
    let mut latest_modified = 0_u128;
    let mut script_count = 0_usize;
    let mut artifacts = Vec::new();
    let mut scripts = Vec::new();
    for archive_path in archive_paths {
        control.check()?;
        let metadata = fs::metadata(&archive_path)
            .map_err(|error| format!("Failed to stat {}: {error}", archive_path.display()))?;
        let modified = modified_unix_ms(&metadata);
        latest_modified = latest_modified.max(modified);
        hasher.update(
            archive_path
                .strip_prefix(&root)
                .unwrap_or(&archive_path)
                .to_string_lossy()
                .replace('\\', "/")
                .as_bytes(),
        );
        hasher.update(metadata.len().to_le_bytes());
        hasher.update(modified.to_le_bytes());
        let archive = PakArchive::inspect_with_cancel(&archive_path, || control.is_cancelled())
            .map_err(|error| format!("Failed to inspect {}: {error}", archive_path.display()))?;
        let entries = archive.select(PakSelection::scripts()).map_err(|error| {
            format!(
                "Failed to select scripts from {}: {error}",
                archive_path.display()
            )
        })?;
        let (selected_payload_sha256, entry_payload_sha256) =
            selected_payload_digest(&archive_path, &entries, control)?;
        hasher.update(selected_payload_sha256.as_bytes());
        let relative_archive = normalized_relative(&root, &archive_path);
        for entry in &entries {
            hasher.update(entry.logical_path().as_bytes());
            hasher.update(entry.offset().to_le_bytes());
            hasher.update(entry.compressed_length().to_le_bytes());
            hasher.update(entry.original_length().to_le_bytes());
            hasher.update(entry.compression().to_le_bytes());
            scripts.push(ScriptLocator {
                uri: String::new(),
                logical_path: entry.logical_path().to_string(),
                pack_relative_path: relative_archive.clone(),
                offset: entry.offset(),
                compressed_length: entry.compressed_length(),
                original_length: entry.original_length(),
                compression: entry.compression(),
                compressed_payload_sha256: entry_payload_sha256
                    .get(entry.logical_path())
                    .cloned()
                    .unwrap_or_default(),
            });
        }
        artifacts.push(PackArtifact {
            relative_path: relative_archive,
            bytes: metadata.len(),
            modified_unix_ms: modified,
            selected_payload_sha256,
            strong_manifest_sha512: adjacent_manifest_sha512(&archive_path),
        });
        script_count += entries.len();
        archives.push((archive, entries));
    }
    let loose_files = loose_script_paths(&root)?;
    for file in &loose_files {
        control.check()?;
        let relative = normalized_relative(&root, file);
        hasher.update(b"loose-script");
        hasher.update(relative.as_bytes());
        hasher.update(
            fs::read(file)
                .map_err(|error| format!("Failed to read {}: {error}", file.display()))?,
        );
    }
    let artifact_digest = format!("{:x}", hasher.finalize());
    Ok(BaseGameInspection {
        guid: guid.clone(),
        display_id,
        root,
        fingerprint: SourceFingerprint::Addon {
            guid,
            artifact_digest: artifact_digest.clone(),
            pack_count: archives.len(),
            catalogue_entry_count: script_count + loose_files.len(),
        },
        artifact_digest,
        archives,
        loose_files,
        artifacts,
        scripts,
    })
}

fn build_inspected_base_game(
    inspection: BaseGameInspection,
    source_revision: &PackedSourceRevision,
    control: &IndexBuildControl,
    source_build_worker_count: usize,
) -> Result<IndexBuildResult, String> {
    let source_acquisition_start = Instant::now();
    let revision = inspection.artifact_digest.clone();
    let mut logical_paths = BTreeSet::new();
    let mut sources = Vec::new();
    for (archive, entries) in inspection.archives {
        let mut reader = archive.reader().map_err(|error| error.to_string())?;
        for entry in entries {
            control.check()?;
            let logical = entry.logical_path().to_string();
            if !logical_paths.insert(logical.to_ascii_lowercase()) {
                return Err(format!("Duplicate base-game script path: {logical}"));
            }
            let relative = PathBuf::from(&logical);
            let uri = virtual_source_uri(&inspection.guid, &revision, &logical)?;
            let expected = source_revision
                .entries
                .get(&uri)
                .ok_or_else(|| format!("Missing packed source identity for {logical}"))?;
            let mut bytes = Vec::with_capacity(entry.original_length() as usize);
            reader
                .read_verified_to_with_cancel(
                    &entry,
                    &expected.compressed_payload_sha256,
                    &mut bytes,
                    || control.is_cancelled(),
                )
                .map_err(|error| {
                    format!(
                        "Failed to read {logical} from {}: {error}",
                        archive.path().display()
                    )
                })?;
            sources.push(IndexSourceText {
                display_path: PathBuf::from(&uri),
                bytes,
                metadata: SourceFileMetadata {
                    kind: SourceKind::GameData,
                    category: source_category_for_path(SourceKind::GameData, Some(&relative)),
                    absolute_path: None,
                    virtual_source: Some(VirtualSourceIdentity {
                        uri,
                        addon_guid: inspection.guid.clone(),
                        revision: revision.clone(),
                        logical_path: logical,
                    }),
                    root_path: None,
                    relative_path: Some(relative),
                    priority: SOURCE_PRIORITY_GAME_DATA,
                },
            });
        }
    }
    for file in inspection.loose_files {
        control.check()?;
        let relative = file
            .strip_prefix(&inspection.root)
            .unwrap_or(&file)
            .to_path_buf();
        sources.push(IndexSourceText {
            display_path: file.clone(),
            bytes: fs::read(&file)
                .map_err(|error| format!("Failed to read {}: {error}", file.display()))?,
            metadata: SourceFileMetadata {
                kind: SourceKind::GameData,
                category: source_category_for_path(SourceKind::GameData, Some(&relative)),
                absolute_path: Some(file),
                virtual_source: None,
                root_path: Some(inspection.root.clone()),
                relative_path: Some(relative),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        });
    }
    let source_acquisition = source_acquisition_start.elapsed();
    let mut result = build_index_from_sources(sources, control, source_build_worker_count)?;
    result.summary.timings.source_acquisition = source_acquisition;
    result.summary.timings.total += source_acquisition;
    Ok(result)
}

fn virtual_source_uri(guid: &str, revision: &str, logical_path: &str) -> Result<String, String> {
    let base = Url::parse(&format!("{VIRTUAL_SOURCE_SCHEME}://{guid}/{revision}/"))
        .map_err(|error| error.to_string())?;
    base.join(logical_path)
        .map(|uri| uri.to_string())
        .map_err(|error| error.to_string())
}

fn base_game_archive_paths(root: &Path) -> [PathBuf; 2] {
    [
        root.join("data").join("data007.pak"),
        root.join("core").join("data.pak"),
    ]
}

/// The Workbench graph authorizes one exact add-on root. The pack reader only
/// enumerates direct PAC artifacts at that root; it never discovers another
/// add-on folder or substitutes an installed duplicate.
fn addon_archive_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(root).map_err(|error| {
        format!(
            "Failed to read Workbench add-on root {}: {error}",
            root.display()
        )
    })?;
    let mut archives = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pak"))
        })
        .collect::<Vec<_>>();
    archives.sort();
    Ok(archives)
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut json = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    json.push(b'\n');
    if fs::read(path).ok().as_deref() == Some(json.as_slice()) {
        return Ok(());
    }
    write_atomic_bytes(path, &json)
}

fn base_game_root(inventory_path: &Path) -> Result<PathBuf, String> {
    read_inventory(inventory_path)?
        .roots
        .into_iter()
        .find(|root| root.kind == "base-game")
        .and_then(|root| root.path)
        .ok_or_else(|| "Arma Reforger base-game add-ons folder is unavailable".to_string())
}

fn read_inventory(inventory_path: &Path) -> Result<Inventory, String> {
    let graph = read_loaded_addon_graph(inventory_path)?;
    let base_game = graph
        .addons
        .iter()
        .find(|addon| addon.guid.eq_ignore_ascii_case(BASE_GAME_GUID))
        .ok_or_else(|| "Workbench has not loaded the Arma Reforger base-game add-on".to_string())?;
    let addons_root = base_game
        .source_root
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            "Workbench base-game source root has no add-ons parent directory".to_string()
        })?;
    Ok(Inventory {
        schema: "reforger-workbench-loaded-addon-graph-v1".to_string(),
        roots: vec![InventoryRoot {
            kind: "base-game".to_string(),
            path: Some(addons_root),
        }],
        addons: Vec::new(),
    })
}

fn read_loaded_addon_graph(inventory_path: &Path) -> Result<LoadedAddonGraph, String> {
    let raw = fs::read_to_string(inventory_path).map_err(|error| {
        format!(
            "Failed to read add-on source inventory {}: {error}",
            inventory_path.display()
        )
    })?;
    let graph: WorkbenchLoadedAddonGraphInventory =
        serde_json::from_str(&raw).map_err(|error| {
            format!(
                "Invalid Workbench loaded add-on graph {}: {error}",
                inventory_path.display()
            )
        })?;
    if graph.schema != "reforger-workbench-loaded-addon-graph-v1" || graph.protocol_version != 1 {
        return Err("Unsupported Workbench loaded add-on graph schema or protocol".to_string());
    }
    if graph.bridge_version.is_empty() || graph.addons.is_empty() {
        return Err("Workbench loaded add-on graph is empty or malformed".to_string());
    }
    let mut loaded_instances = BTreeSet::new();
    let addons = graph
        .addons
        .into_iter()
        .map(|addon| {
            let guid = addon.guid.to_ascii_uppercase();
            let source_root = fs::canonicalize(&addon.source_root).map_err(|_| {
                "Workbench loaded add-on graph contains an inaccessible source root".to_string()
            })?;
            if guid.len() != 16
                || !guid.bytes().all(|byte| byte.is_ascii_hexdigit())
                || addon.id.is_empty()
                || addon.title.is_empty()
                || !loaded_instances.insert((guid.clone(), source_root.clone()))
            {
                return Err(
                    "Workbench loaded add-on graph contains an invalid or duplicate GUID/source-root instance"
                        .to_string(),
                );
            }
            Ok(LoadedAddonSource {
                guid,
                id: addon.id,
                title: addon.title,
                source_root,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(LoadedAddonGraph { addons })
}

fn modified_unix_ms(metadata: &fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|time| time.as_millis())
        .unwrap_or(0)
}

fn normalized_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn selected_payload_digest(
    archive_path: &Path,
    entries: &[PakEntry],
    control: &IndexBuildControl,
) -> Result<(String, BTreeMap<String, String>), String> {
    let mut entries = entries.iter().collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.offset());
    let mut file = fs::File::open(archive_path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut entry_digests = BTreeMap::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    for entry in entries {
        control.check()?;
        hasher.update(entry.logical_path().as_bytes());
        hasher.update(entry.offset().to_le_bytes());
        hasher.update(entry.compressed_length().to_le_bytes());
        hasher.update(entry.original_length().to_le_bytes());
        hasher.update(entry.compression().to_le_bytes());
        let mut entry_hasher = Sha256::new();
        file.seek(SeekFrom::Start(entry.offset()))
            .map_err(|error| error.to_string())?;
        let mut remaining = entry.compressed_length();
        while remaining > 0 {
            control.check()?;
            let count = usize::try_from(remaining.min(buffer.len() as u64)).unwrap();
            file.read_exact(&mut buffer[..count])
                .map_err(|error| error.to_string())?;
            hasher.update(&buffer[..count]);
            entry_hasher.update(&buffer[..count]);
            remaining -= count as u64;
        }
        entry_digests.insert(
            entry.logical_path().to_string(),
            format!("{:x}", entry_hasher.finalize()),
        );
    }
    Ok((format!("{:x}", hasher.finalize()), entry_digests))
}

fn packed_source_revision(inspection: &BaseGameInspection) -> Arc<PackedSourceRevision> {
    let mut entries = BTreeMap::new();
    for (_, selected) in &inspection.archives {
        for entry in selected {
            let uri = virtual_source_uri(
                &inspection.guid,
                &inspection.artifact_digest,
                entry.logical_path(),
            )
            .expect("validated logical paths always produce a URI");
            let compressed_payload_sha256 = inspection
                .scripts
                .iter()
                .find(|locator| {
                    locator.logical_path == entry.logical_path()
                        && Path::new(&locator.pack_relative_path)
                            .file_name()
                            .is_some_and(|name| {
                                name == entry.archive_path().file_name().unwrap_or_default()
                            })
                })
                .map(|locator| locator.compressed_payload_sha256.clone())
                .unwrap_or_default();
            entries.insert(
                uri,
                PackedSourceEntry {
                    entry: entry.clone(),
                    compressed_payload_sha256,
                },
            );
        }
    }
    let artifacts = inspection
        .archives
        .iter()
        .map(|(archive, _)| {
            let metadata = fs::metadata(archive.path()).expect("inspected archive still exists");
            ArtifactStamp {
                path: archive.path().to_path_buf(),
                bytes: metadata.len(),
                modified_unix_ms: modified_unix_ms(&metadata),
            }
        })
        .collect();
    Arc::new(PackedSourceRevision { artifacts, entries })
}

fn revision_key(guid: &str, revision: &str) -> String {
    format!(
        "{}/{}",
        guid.to_ascii_uppercase(),
        revision.to_ascii_lowercase()
    )
}

fn addon_instance_key(guid: &str, source_root: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(guid.to_ascii_uppercase().as_bytes());
    hasher.update(b"\0");
    hasher.update(
        source_root
            .to_string_lossy()
            .to_ascii_lowercase()
            .as_bytes(),
    );
    format!("{}-{:x}", guid.to_ascii_uppercase(), hasher.finalize())
}

fn register_source_revision(guid: &str, revision: &str, sources: Arc<PackedSourceRevision>) {
    if let Ok(mut revisions) = SOURCE_REVISIONS.get_or_init(Default::default).lock() {
        revisions.insert(revision_key(guid, revision), sources);
    }
}

fn publish_inventory_addon_manifests(
    inventory: &Inventory,
    storage_root: &Path,
    control: &IndexBuildControl,
) -> Result<(), String> {
    control.check()?;
    let revision = inventory_publication_revision(inventory, control)?;
    let publication_path = storage_root.join("inventory-current.json");
    let mut identities = BTreeMap::<String, PathBuf>::new();
    let mut manifests = Vec::new();
    for addon in inventory
        .addons
        .iter()
        .filter(|addon| addon.root_kind != "base-game")
    {
        control.check()?;
        let Some(project_file) = &addon.project_file else {
            continue;
        };
        let project = fs::read_to_string(project_file)
            .map_err(|error| format!("Failed to read {}: {error}", project_file.display()))?;
        let guid = extract_project_property(&project, "GUID")
            .ok_or_else(|| format!("Add-on project has no GUID: {}", project_file.display()))?
            .to_ascii_uppercase();
        if guid.len() != 16 || !guid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "Invalid add-on GUID '{guid}': {}",
                project_file.display()
            ));
        }
        if let Some(existing) = identities.insert(guid.clone(), addon.path.clone()) {
            return Err(format!(
                "Duplicate add-on GUID {guid}: {} and {}",
                existing.display(),
                addon.path.display()
            ));
        }
        let display_id = extract_project_property(&project, "ID")
            .or_else(|| extract_project_property(&project, "TITLE"))
            .unwrap_or_else(|| addon.directory_name.clone());
        let pack_artifacts = addon
            .pack_files
            .iter()
            .map(|path| {
                let metadata = fs::metadata(path)
                    .map_err(|error| format!("Failed to stat {}: {error}", path.display()))?;
                Ok(InventoryPackArtifact {
                    path: path.clone(),
                    bytes: metadata.len(),
                    modified_unix_ms: modified_unix_ms(&metadata),
                    strong_manifest_sha512: adjacent_manifest_sha512(path),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        manifests.push((
            storage_root.join(&guid).join("inventory.json"),
            InventoryAddonManifest {
                schema: "reforger-addon-inventory-manifest-v1",
                guid,
                display_id,
                directory_name: addon.directory_name.clone(),
                root_kind: addon.root_kind.clone(),
                source_root: addon.path.clone(),
                semantic_status: "inventory-only",
                pack_artifacts,
            },
        ));
    }
    let publication_reusable = fs::read(&publication_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<InventoryPublication>(&bytes).ok())
        .is_some_and(|publication| {
            publication.schema == "reforger-addon-inventory-publication-v1"
                && publication.revision == revision
        })
        && manifests.iter().all(|(path, expected)| {
            serde_json::to_vec_pretty(expected)
                .ok()
                .is_some_and(|mut expected_bytes| {
                    expected_bytes.push(b'\n');
                    fs::read(path)
                        .ok()
                        .is_some_and(|actual_bytes| actual_bytes == expected_bytes)
                })
        });
    if publication_reusable {
        return Ok(());
    }
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(8)
        .min(manifests.len().max(1));
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        let manifests = &manifests;
        for worker in 0..workers {
            handles.push(scope.spawn(move || {
                for (path, manifest) in manifests.iter().skip(worker).step_by(workers) {
                    control.check()?;
                    write_json_atomic(path, manifest)?;
                }
                Ok::<(), String>(())
            }));
        }
        for handle in handles {
            handle
                .join()
                .map_err(|_| "Add-on inventory publication worker panicked".to_string())??;
        }
        Ok::<(), String>(())
    })?;
    write_json_atomic(
        &publication_path,
        &InventoryPublication {
            schema: "reforger-addon-inventory-publication-v1".to_string(),
            revision,
        },
    )?;
    Ok(())
}

fn inventory_publication_revision(
    inventory: &Inventory,
    control: &IndexBuildControl,
) -> Result<String, String> {
    let mut hasher = Sha256::new();
    hasher.update(b"reforger-addon-inventory-publication-v2");
    for addon in inventory
        .addons
        .iter()
        .filter(|addon| addon.root_kind != "base-game")
    {
        control.check()?;
        hasher.update(addon.root_kind.as_bytes());
        hasher.update(addon.directory_name.as_bytes());
        hasher.update(addon.path.to_string_lossy().as_bytes());
        if let Some(project_file) = &addon.project_file {
            hasher.update(project_file.to_string_lossy().as_bytes());
            hasher.update(
                fs::read(project_file).map_err(|error| {
                    format!("Failed to read {}: {error}", project_file.display())
                })?,
            );
        }
        for path in &addon.pack_files {
            control.check()?;
            let metadata = fs::metadata(path)
                .map_err(|error| format!("Failed to stat {}: {error}", path.display()))?;
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(metadata.len().to_le_bytes());
            if let Some(strong_manifest) = adjacent_manifest_sha512(path) {
                hasher.update(b"reforger-manifest-sha512");
                hasher.update(strong_manifest.as_bytes());
                continue;
            }
            let archive = PakArchive::inspect_with_cancel(path, || control.is_cancelled())
                .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?;
            let entries = archive.select(PakSelection::scripts()).map_err(|error| {
                format!("Failed to select scripts from {}: {error}", path.display())
            })?;
            let (selected_digest, _) = selected_payload_digest(path, &entries, control)?;
            hasher.update(selected_digest.as_bytes());
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn extract_project_property(source: &str, property: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let remainder = line.trim().strip_prefix(property)?;
        if !remainder
            .chars()
            .next()
            .is_some_and(|value| value.is_whitespace() || value == ':')
        {
            return None;
        }
        let remainder = remainder
            .trim_start()
            .strip_prefix(':')
            .unwrap_or(remainder)
            .trim_start();
        let quoted = remainder.strip_prefix('"')?;
        let end = quoted.find('"')?;
        (!quoted[..end].is_empty()).then(|| quoted[..end].to_string())
    })
}

fn adjacent_manifest_sha512(pack: &Path) -> Option<String> {
    let file_name = pack.file_name()?.to_string_lossy();
    let prefix = format!("{file_name}_");
    let parent = pack.parent()?;
    let mut candidates = fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .map(|name| {
                    let name = name.to_string_lossy();
                    name.starts_with(&prefix) && name.ends_with("_manifest.json")
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    candidates.sort();
    let raw = fs::read_to_string(candidates.first()?).ok()?;
    serde_json::from_str::<serde_json::Value>(&raw)
        .ok()?
        .get("sha512")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index_cache::IndexCacheStatus;

    #[test]
    fn rejects_unknown_inventory_schema() {
        let root =
            std::env::temp_dir().join(format!("reforger_inventory_test_{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let inventory = root.join("inventory.json");
        fs::write(&inventory, r#"{"schema":"unknown","roots":[]}"#).unwrap();
        assert!(build_base_game_index(&inventory, &IndexBuildControl::default()).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_loose_scripts_only_from_the_addon_scripts_directory() {
        let root = test_root("loose_script_paths");
        let scripts = root.join("Scripts").join("Game");
        let assets = root.join("Assets");
        fs::create_dir_all(&scripts).unwrap();
        fs::create_dir_all(&assets).unwrap();
        fs::write(scripts.join("Listed.c"), "class Listed {}").unwrap();
        fs::write(assets.join("Ignored.c"), "class Ignored {}").unwrap();
        fs::write(root.join("Ignored.c"), "class Ignored {}").unwrap();

        assert_eq!(loose_script_paths(&root).unwrap(), vec![scripts.join("Listed.c")]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn loaded_addon_index_workers_are_bounded_by_tasks_cpus_and_the_cold_start_cap() {
        assert_eq!(addon_index_worker_count_for(1, 140), 1);
        assert_eq!(addon_index_worker_count_for(16, 0), 0);
        assert_eq!(addon_index_worker_count_for(16, 2), 2);
        assert_eq!(addon_index_worker_count_for(16, 140), MAX_ADDON_INDEX_WORKERS);
    }

    #[test]
    fn accepts_only_the_workbench_loaded_graph_and_derives_the_base_addons_parent() {
        let root = test_root("workbench_graph");
        let graph = root.join("graph.json");
        let data_root = root.join("addons").join("data");
        fs::create_dir_all(&data_root).unwrap();
        fs::write(data_root.join("ArmaReforger.gproj"), "{}").unwrap();
        fs::write(
            &graph,
            format!(
                r#"{{"schema":"reforger-workbench-loaded-addon-graph-v1","bridgeVersion":"1.52.0","protocolVersion":1,"addons":[{{"guid":"{BASE_GAME_GUID}","id":"ArmaReforger","title":"Arma Reforger","sourceRoot":{}}}]}}"#,
                serde_json::to_string(&data_root).unwrap(),
            ),
        )
        .unwrap();

        let inventory = read_inventory(&graph).unwrap();

        assert_eq!(inventory.roots.len(), 1);
        assert_eq!(
            inventory.roots[0].path,
            data_root.parent().map(Path::to_path_buf)
        );
        let legacy = root.join("legacy.json");
        fs::write(
            &legacy,
            r#"{"schema":"reforger-addon-source-inventory-v1","roots":[]}"#,
        )
        .unwrap();
        assert!(read_inventory(&legacy).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn distinguishes_loaded_instances_by_guid_and_canonical_source_root() {
        let root = test_root("duplicate_guid_loaded_instances");
        let packed = root.join("packed");
        let unpacked = root.join("unpacked");
        fs::create_dir_all(&packed).unwrap();
        fs::create_dir_all(&unpacked).unwrap();
        let graph = root.join("graph.json");
        fs::write(&graph, format!(
            r#"{{"schema":"reforger-workbench-loaded-addon-graph-v1","bridgeVersion":"1.52.0","protocolVersion":1,"addons":[{{"guid":"1111111111111111","id":"Same","title":"Packed","sourceRoot":{}}},{{"guid":"1111111111111111","id":"Same","title":"Unpacked","sourceRoot":{}}}]}}"#,
            serde_json::to_string(&packed).unwrap(),
            serde_json::to_string(&unpacked).unwrap(),
        )).unwrap();

        let loaded = read_loaded_addon_graph(&graph).unwrap();

        assert_eq!(loaded.addons.len(), 2);
        assert_ne!(
            addon_instance_key(&loaded.addons[0].guid, &loaded.addons[0].source_root),
            addon_instance_key(&loaded.addons[1].guid, &loaded.addons[1].source_root),
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn indexes_workspace_addons_only_through_live_sources_and_removes_their_cache() {
        let root = test_root("loaded_addon_instances");
        let packed = root.join("packed");
        let loose = root.join("loose");
        fs::create_dir_all(&packed).unwrap();
        fs::create_dir_all(loose.join("scripts")).unwrap();
        write_fixture_pak(
            &packed.join("data.pak"),
            &[("Packed.c", b"class Packed {}")],
        );
        write_fixture_pak(
            &loose.join("data.pak"),
            &[("LoosePacked.c", b"class LoosePacked {}")],
        );
        fs::write(loose.join("scripts/Loose.c"), "class Loose {}").unwrap();
        let graph = root.join("graph.json");
        fs::write(&graph, format!(
            r#"{{"schema":"reforger-workbench-loaded-addon-graph-v1","bridgeVersion":"1.52.0","protocolVersion":1,"addons":[{{"guid":"1111111111111111","id":"Packed","title":"Packed","sourceRoot":{}}},{{"guid":"2222222222222222","id":"Loose","title":"Loose","sourceRoot":{}}}]}}"#,
            serde_json::to_string(&packed).unwrap(),
            serde_json::to_string(&loose).unwrap(),
        )).unwrap();
        fs::write(loose.join("addon.gproj"), "{}").unwrap();
        let storage = root.join("indexes");

        let workspace_roots = Vec::new();
        let first = load_or_build_loaded_addon_indexes(
            &graph,
            &storage,
            &workspace_roots,
            &IndexBuildControl::default(),
        )
        .unwrap();
        assert_eq!(first.summary.files, 3);
        assert_eq!(first.rebuilt_instances, 2);
        assert_eq!(first.instances.len(), 2);
        assert!(first
            .instances
            .iter()
            .all(|instance| instance.cache_status == "rebuilt"));
        assert!(first
            .instances
            .iter()
            .all(|instance| instance.cache_detail.as_deref() == Some("cache-missing")));
        assert_eq!(fs::read_dir(&storage).unwrap().count(), 2);
        let optimistic = load_cached_loaded_addon_indexes(
            &graph,
            &storage,
            &workspace_roots,
            &IndexBuildControl::default(),
        )
        .unwrap();
        assert_eq!(optimistic.loaded_instances, 2);
        assert_eq!(optimistic.missing_instances, 0);
        assert_eq!(optimistic.summary.files, 3);
        assert!(optimistic
            .instances
            .iter()
            .all(|instance| instance.cache_status == "optimistic-loaded"));
        let workspace_roots = vec![loose.join("scripts")];
        let second = load_or_build_loaded_addon_indexes(
            &graph,
            &storage,
            &workspace_roots,
            &IndexBuildControl::default(),
        )
        .unwrap();
        assert_eq!(second.loaded_instances, 1);
        assert_eq!(second.workspace_excluded_instances, 1);
        assert_eq!(second.instances.len(), 1);
        assert_eq!(second.instances[0].cache_status, "loaded");
        assert!(second.instances[0].cache_file_bytes.is_some());
        assert_eq!(fs::read_dir(&storage).unwrap().count(), 1);
        write_fixture_pak(
            &packed.join("data.pak"),
            &[("Packed.c", b"class ChangedPacked {}")],
        );
        let rebuilt = load_or_build_loaded_addon_indexes(
            &graph,
            &storage,
            &workspace_roots,
            &IndexBuildControl::default(),
        )
        .unwrap();
        assert_eq!(rebuilt.rebuilt_instances, 1);
        let packed_revisions = fs::read_dir(&storage)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
            .join("revisions");
        assert_eq!(
            fs::read_dir(packed_revisions).unwrap().count(),
            1,
            "only the current loaded revision is retained"
        );
        fs::write(loose.join("scripts/Loose.c"), "class ChangedLoose {}").unwrap();
        let changed = load_or_build_loaded_addon_indexes(
            &graph,
            &storage,
            &workspace_roots,
            &IndexBuildControl::default(),
        )
        .unwrap();
        assert_eq!(changed.loaded_instances, 1);
        assert_eq!(changed.rebuilt_instances, 0);
        fs::write(&graph, format!(
            r#"{{"schema":"reforger-workbench-loaded-addon-graph-v1","bridgeVersion":"1.52.0","protocolVersion":1,"addons":[{{"guid":"1111111111111111","id":"Packed","title":"Packed","sourceRoot":{}}}]}}"#,
            serde_json::to_string(&packed).unwrap(),
        )).unwrap();
        fs::write(packed.join("addon.gproj"), "{}").unwrap();
        let removed = load_or_build_loaded_addon_indexes(
            &graph,
            &storage,
            &[],
            &IndexBuildControl::default(),
        )
        .unwrap();
        assert_eq!(removed.index.files().len(), 1);
        assert_eq!(fs::read_dir(&storage).unwrap().count(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn virtual_source_uri_preserves_the_addon_and_logical_path() {
        assert_eq!(
            virtual_source_uri(BASE_GAME_GUID, "abc123", "scripts/Game/My File.c").unwrap(),
            "reforger-pak://58D0FB3206B6F859/abc123/scripts/Game/My%20File.c"
        );
    }

    #[test]
    fn builds_and_reuses_one_durable_index_without_materializing_sources() {
        let root = test_root("cache");
        let addons = root.join("addons");
        let data = addons.join("data");
        let core = addons.join("core");
        fs::create_dir_all(&data).unwrap();
        fs::create_dir_all(&core).unwrap();
        write_fixture_pak(
            &data.join("data007.pak"),
            &[
                ("Feature.c", b"class Feature {}"),
                ("Texture.bin", b"not script data"),
            ],
        );
        write_fixture_pak(
            &core.join("data.pak"),
            &[("CoreFeature.c", b"class CoreFeature {}")],
        );
        let inventory = root.join("inventory.json");
        write_workbench_graph_fixture(&inventory, &data);
        let storage = root.join("indexes");

        let rebuilt =
            load_or_build_base_game_index(&inventory, &storage, &IndexBuildControl::default())
                .unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert_eq!(rebuilt.summary.files, 2);
        let current: CurrentRevision = serde_json::from_slice(
            &fs::read(storage.join(BASE_GAME_GUID).join("current.json")).unwrap(),
        )
        .unwrap();
        let manifest: AddonIndexManifest = serde_json::from_slice(
            &fs::read(storage.join(BASE_GAME_GUID).join(&current.manifest)).unwrap(),
        )
        .unwrap();
        let feature_uri = manifest
            .scripts
            .iter()
            .find(|script| script.logical_path == "Root/Feature.c")
            .unwrap()
            .uri
            .clone();
        assert_eq!(
            read_virtual_source(&feature_uri).unwrap(),
            "class Feature {}"
        );
        let editor_normalized_uri =
            feature_uri.replacen(BASE_GAME_GUID, &BASE_GAME_GUID.to_ascii_lowercase(), 1);
        assert_eq!(
            read_virtual_source(&editor_normalized_uri).unwrap(),
            "class Feature {}"
        );

        let loaded =
            load_or_build_base_game_index(&inventory, &storage, &IndexBuildControl::default())
                .unwrap();
        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert!(storage.join(BASE_GAME_GUID).join("current.json").is_file());
        assert!(storage.join(BASE_GAME_GUID).join(&current.index).is_file());
        fs::write(
            storage.join(BASE_GAME_GUID).join(&current.manifest),
            b"{\"schema\":\"corrupt\"}",
        )
        .unwrap();
        let repaired =
            load_or_build_base_game_index(&inventory, &storage, &IndexBuildControl::default())
                .unwrap();
        assert!(matches!(
            repaired.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert!(!storage.join(BASE_GAME_GUID).join("scripts").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_same_size_script_change_publishes_a_new_revision_without_removing_the_old_one() {
        let root = test_root("revision");
        let addons = root.join("addons");
        fs::create_dir_all(addons.join("data")).unwrap();
        fs::create_dir_all(addons.join("core")).unwrap();
        write_fixture_pak(
            &addons.join("data/data007.pak"),
            &[("Feature.c", b"class FeatureA {}")],
        );
        write_fixture_pak(
            &addons.join("core/data.pak"),
            &[("Core.c", b"class Core {}")],
        );
        let inventory = root.join("inventory.json");
        write_workbench_graph_fixture(&inventory, &addons.join("data"));
        let storage = root.join("indexes");
        load_or_build_base_game_index(&inventory, &storage, &IndexBuildControl::default()).unwrap();
        let first: CurrentRevision = serde_json::from_slice(
            &fs::read(storage.join(BASE_GAME_GUID).join("current.json")).unwrap(),
        )
        .unwrap();

        write_fixture_pak(
            &addons.join("data/data007.pak"),
            &[("Feature.c", b"class FeatureB {}")],
        );
        let first_manifest: AddonIndexManifest = serde_json::from_slice(
            &fs::read(storage.join(BASE_GAME_GUID).join(&first.manifest)).unwrap(),
        )
        .unwrap();
        let old_uri = &first_manifest
            .scripts
            .iter()
            .find(|script| script.logical_path == "Root/Feature.c")
            .unwrap()
            .uri;
        assert!(read_virtual_source(old_uri)
            .unwrap_err()
            .contains("changed on disk"));
        let cancelled = IndexBuildControl::default();
        cancelled.cancel();
        assert_eq!(
            load_or_build_base_game_index(&inventory, &storage, &cancelled).unwrap_err(),
            crate::index_build::INDEX_BUILD_CANCELLED
        );
        let after_cancel: CurrentRevision = serde_json::from_slice(
            &fs::read(storage.join(BASE_GAME_GUID).join("current.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(after_cancel, first);

        let rebuilt =
            load_or_build_base_game_index(&inventory, &storage, &IndexBuildControl::default())
                .unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        let second: CurrentRevision = serde_json::from_slice(
            &fs::read(storage.join(BASE_GAME_GUID).join("current.json")).unwrap(),
        )
        .unwrap();
        assert_ne!(first.revision, second.revision);
        assert!(storage.join(BASE_GAME_GUID).join(first.index).is_file());
        assert!(storage.join(BASE_GAME_GUID).join(second.index).is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inventory_only_addons_are_guid_keyed_and_duplicate_guids_are_rejected() {
        let root = test_root("inventory_addons");
        let storage = root.join("indexes");
        let first = root.join("First");
        let second = root.join("Second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(
            first.join("addon.gproj"),
            "GameProject {\n GUID \"1337C0DE5DABBEEF\"\n ID \"RHS\"\n}",
        )
        .unwrap();
        fs::write(
            second.join("addon.gproj"),
            "GameProject {\n GUID \"1337C0DE5DABBEEF\"\n ID \"Duplicate\"\n}",
        )
        .unwrap();
        write_fixture_pak(&first.join("data.pak"), &[("First.c", b"class First {}")]);
        write_fixture_pak(
            &second.join("data.pak"),
            &[("Second.c", b"class Second {}")],
        );
        let addon = |path: &Path, name: &str| InventoryAddon {
            root_kind: "user-addons".to_string(),
            directory_name: name.to_string(),
            path: path.to_path_buf(),
            project_file: Some(path.join("addon.gproj")),
            pack_files: vec![path.join("data.pak")],
        };
        let one = Inventory {
            schema: "reforger-addon-source-inventory-v1".to_string(),
            roots: Vec::new(),
            addons: vec![addon(&first, "First")],
        };
        publish_inventory_addon_manifests(&one, &storage, &IndexBuildControl::default()).unwrap();
        let manifest_path = storage.join("1337C0DE5DABBEEF/inventory.json");
        assert!(manifest_path.is_file());
        fs::write(&manifest_path, b"corrupt").unwrap();
        publish_inventory_addon_manifests(&one, &storage, &IndexBuildControl::default()).unwrap();
        assert!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&manifest_path).unwrap()).is_ok()
        );

        let duplicate = Inventory {
            schema: "reforger-addon-source-inventory-v1".to_string(),
            roots: Vec::new(),
            addons: vec![addon(&first, "First"), addon(&second, "Second")],
        };
        assert!(publish_inventory_addon_manifests(
            &duplicate,
            &storage,
            &IndexBuildControl::default()
        )
        .unwrap_err()
        .contains("Duplicate add-on GUID"));
        let same_path_duplicate = Inventory {
            schema: "reforger-addon-source-inventory-v1".to_string(),
            roots: Vec::new(),
            addons: vec![addon(&first, "First"), addon(&first, "FirstAgain")],
        };
        assert!(publish_inventory_addon_manifests(
            &same_path_duplicate,
            &storage,
            &IndexBuildControl::default()
        )
        .unwrap_err()
        .contains("Duplicate add-on GUID"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inventory_publication_detects_same_length_script_content_changes() {
        let root = test_root("inventory_content_identity");
        let storage = root.join("indexes");
        let addon_root = root.join("Addon");
        fs::create_dir_all(&addon_root).unwrap();
        fs::write(
            addon_root.join("addon.gproj"),
            "GameProject {\n GUID \"1337C0DE5DABBEEF\"\n ID \"ContentIdentity\"\n}",
        )
        .unwrap();
        let pack = addon_root.join("data.pak");
        write_fixture_pak(&pack, &[("Feature.c", b"class First {}")]);
        let inventory = Inventory {
            schema: "reforger-addon-source-inventory-v1".to_string(),
            roots: Vec::new(),
            addons: vec![InventoryAddon {
                root_kind: "user-addons".to_string(),
                directory_name: "Addon".to_string(),
                path: addon_root.clone(),
                project_file: Some(addon_root.join("addon.gproj")),
                pack_files: vec![pack.clone()],
            }],
        };
        let control = IndexBuildControl::default();
        publish_inventory_addon_manifests(&inventory, &storage, &control).unwrap();
        let first: InventoryPublication =
            serde_json::from_slice(&fs::read(storage.join("inventory-current.json")).unwrap())
                .unwrap();
        let first_pack_bytes = fs::metadata(&pack).unwrap().len();

        write_fixture_pak(&pack, &[("Feature.c", b"class Other {}")]);
        publish_inventory_addon_manifests(&inventory, &storage, &control).unwrap();
        let second: InventoryPublication =
            serde_json::from_slice(&fs::read(storage.join("inventory-current.json")).unwrap())
                .unwrap();

        assert_eq!(first_pack_bytes, fs::metadata(&pack).unwrap().len());
        assert_ne!(first.revision, second.revision);
        let _ = fs::remove_dir_all(root);
    }

    fn test_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "reforger_addon_sources_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn write_workbench_graph_fixture(path: &Path, data_root: &Path) {
        fs::write(data_root.join("ArmaReforger.gproj"), "{}").unwrap();
        fs::write(
            path,
            format!(
                r#"{{"schema":"reforger-workbench-loaded-addon-graph-v1","bridgeVersion":"1.52.0","protocolVersion":1,"addons":[{{"guid":"{BASE_GAME_GUID}","id":"ArmaReforger","title":"Arma Reforger","sourceRoot":{}}}]}}"#,
                serde_json::to_string(data_root).unwrap(),
            ),
        )
        .unwrap();
    }

    fn write_fixture_pak(path: &Path, entries: &[(&str, &[u8])]) {
        let mut table = vec![0, 4];
        table.extend_from_slice(b"Root");
        table.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        let table_length = 1
            + 1
            + 4
            + 4
            + entries
                .iter()
                .map(|(logical, _)| 1 + 1 + logical.len() + 24)
                .sum::<usize>();
        let mut offset = 12 + 8 + 28 + 8 + table_length + 8;
        for (logical, content) in entries {
            table.push(1);
            table.push(logical.len() as u8);
            table.extend_from_slice(logical.as_bytes());
            table.extend_from_slice(&(offset as u32).to_le_bytes());
            table.extend_from_slice(&(content.len() as u32).to_le_bytes());
            table.extend_from_slice(&(content.len() as u32).to_le_bytes());
            table.extend_from_slice(&[0; 8]);
            table.extend_from_slice(&0_u32.to_be_bytes());
            offset += content.len();
        }
        let data_length = entries
            .iter()
            .map(|(_, content)| content.len())
            .sum::<usize>();
        let mut bytes = b"FORM".to_vec();
        bytes.extend_from_slice(
            &((4 + 8 + 28 + 8 + table.len() + 8 + data_length) as u32).to_be_bytes(),
        );
        bytes.extend_from_slice(b"PAC1HEAD");
        bytes.extend_from_slice(&28_u32.to_be_bytes());
        bytes.extend_from_slice(&[0; 28]);
        bytes.extend_from_slice(b"FILE");
        bytes.extend_from_slice(&(table.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&table);
        bytes.extend_from_slice(b"DATA");
        bytes.extend_from_slice(&(data_length as u32).to_be_bytes());
        for (_, content) in entries {
            bytes.extend_from_slice(content);
        }
        fs::write(path, bytes).unwrap();
    }
}
