use crate::index_build::{
    build_index_from_sources, IndexBuildControl, IndexBuildResult, IndexSourceText,
};
use crate::index_cache::{
    cache_format_identity, load_or_build_archive_index, write_atomic_bytes,
    GameDataIndexCacheResult, SourceFingerprint,
};
use crate::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, VirtualSourceIdentity,
    SOURCE_PRIORITY_GAME_DATA,
};
use crate::pack::{PakArchive, PakEntry, PakSelection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use url::Url;

pub const BASE_GAME_GUID: &str = "58D0FB3206B6F859";
pub const VIRTUAL_SOURCE_SCHEME: &str = "reforger-pak";

#[derive(Debug, Deserialize)]
struct Inventory {
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

struct BaseGameInspection {
    root: PathBuf,
    archives: Vec<(PakArchive, Vec<PakEntry>)>,
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
    build_inspected_base_game(inspection, control)
}

pub fn load_or_build_base_game_index(
    inventory_path: &Path,
    storage_root: &Path,
    control: &IndexBuildControl,
) -> Result<GameDataIndexCacheResult, String> {
    let inventory = read_inventory(inventory_path)?;
    publish_inventory_addon_manifests(&inventory, storage_root)?;
    let inspection_started = std::time::Instant::now();
    let inspection = inspect_base_game(inventory_path, control)?;
    let inspection_elapsed = inspection_started.elapsed();
    let source_root = inspection.root.clone();
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
    let addon_root = storage_root.join(BASE_GAME_GUID);
    let revision_relative = format!("revisions/{artifact_digest}");
    let revision_root = addon_root.join(&revision_relative);
    let cache = revision_root.join("symbols.bin");
    let source_revision = packed_source_revision(&inspection);
    let mut result =
        load_or_build_archive_index(&cache, fingerprint, artifact_digest.clone(), || {
            build_inspected_base_game(inspection, control)
        })?;
    result.timings.fingerprint = inspection_elapsed;
    result.timings.total += inspection_elapsed;
    control.check()?;
    let (cache_schema, cache_format_version, cache_index_shape) = cache_format_identity();
    let manifest = AddonIndexManifest {
        schema: "reforger-addon-index-manifest-v2".to_string(),
        cache_schema: cache_schema.to_string(),
        cache_format_version,
        cache_index_shape: cache_index_shape.to_string(),
        extractor_schema: "pac1-selected-script-payload-v2".to_string(),
        guid: BASE_GAME_GUID.to_string(),
        display_id: "Arma Reforger base game".to_string(),
        source_root,
        source_precedence:
            "installed game data007 scripts followed by installed game core; Workbench core is inventory-only to avoid a duplicate semantic layer"
                .to_string(),
        revision: artifact_digest.clone(),
        pack_count,
        script_count,
        pack_artifacts,
        scripts: scripts
            .into_iter()
            .map(|mut script| {
                script.uri =
                    virtual_source_uri(BASE_GAME_GUID, &artifact_digest, &script.logical_path)?;
                Ok(script)
            })
            .collect::<Result<Vec<_>, String>>()?,
        index_file: "symbols.bin".to_string(),
        index_bytes: result.cache_file_bytes.unwrap_or(0),
    };
    write_json_atomic(&revision_root.join("manifest.json"), &manifest)?;
    control.check()?;
    let current = CurrentRevision {
        schema: "reforger-addon-current-revision-v1".to_string(),
        guid: BASE_GAME_GUID.to_string(),
        revision: artifact_digest.clone(),
        manifest: format!("{revision_relative}/manifest.json"),
        index: format!("{revision_relative}/symbols.bin"),
    };
    write_json_atomic(&addon_root.join("current.json"), &current)?;
    register_source_revision(BASE_GAME_GUID, &artifact_digest, source_revision);
    Ok(result)
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
        .get(uri)
        .ok_or_else(|| format!("Pack source does not exist: {logical_path}"))?;
    validate_entry_payload(source_entry)?;
    let entry = &source_entry.entry;
    let archive = PakArchive::inspect(entry.archive_path()).map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(entry.original_length() as usize);
    archive
        .reader()
        .map_err(|error| error.to_string())?
        .read_to(entry, &mut bytes)
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
    let mut hasher = Sha256::new();
    hasher.update(b"reforger-base-pac-catalogue-v2");
    let (cache_schema, cache_version, cache_shape) = cache_format_identity();
    hasher.update(cache_schema.as_bytes());
    hasher.update(cache_version.to_le_bytes());
    hasher.update(cache_shape.as_bytes());
    hasher.update(b"pac1-selected-script-payload-v2");
    let mut archives = Vec::new();
    let mut latest_modified = 0_u128;
    let mut script_count = 0_usize;
    let mut artifacts = Vec::new();
    let mut scripts = Vec::new();
    for archive_path in base_game_archive_paths(&root) {
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
    let artifact_digest = format!("{:x}", hasher.finalize());
    Ok(BaseGameInspection {
        root,
        fingerprint: SourceFingerprint::Addon {
            guid: BASE_GAME_GUID.to_string(),
            artifact_digest: artifact_digest.clone(),
            pack_count: archives.len(),
            catalogue_entry_count: script_count,
        },
        artifact_digest,
        archives,
        artifacts,
        scripts,
    })
}

fn build_inspected_base_game(
    inspection: BaseGameInspection,
    control: &IndexBuildControl,
) -> Result<IndexBuildResult, String> {
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
            let mut bytes = Vec::with_capacity(entry.original_length() as usize);
            reader
                .read_to_with_cancel(&entry, &mut bytes, || control.is_cancelled())
                .map_err(|error| {
                    format!(
                        "Failed to read {logical} from {}: {error}",
                        archive.path().display()
                    )
                })?;
            let relative = PathBuf::from(&logical);
            let uri = virtual_source_uri(BASE_GAME_GUID, &revision, &logical)?;
            sources.push(IndexSourceText {
                display_path: PathBuf::from(&uri),
                bytes,
                metadata: SourceFileMetadata {
                    kind: SourceKind::GameData,
                    category: source_category_for_path(SourceKind::GameData, Some(&relative)),
                    absolute_path: None,
                    virtual_source: Some(VirtualSourceIdentity {
                        uri,
                        addon_guid: BASE_GAME_GUID.to_string(),
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
    build_index_from_sources(sources, control)
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
    let raw = fs::read_to_string(inventory_path).map_err(|error| {
        format!(
            "Failed to read add-on source inventory {}: {error}",
            inventory_path.display()
        )
    })?;
    let inventory: Inventory = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "Invalid add-on source inventory {}: {error}",
            inventory_path.display()
        )
    })?;
    if inventory.schema != "reforger-addon-source-inventory-v1" {
        return Err("Unsupported add-on source inventory schema".to_string());
    }
    Ok(inventory)
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
        hasher.update(entry.compressed_length().to_le_bytes());
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
                BASE_GAME_GUID,
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

fn validate_entry_payload(source: &PackedSourceEntry) -> Result<(), String> {
    let entry = &source.entry;
    let mut file = fs::File::open(entry.archive_path()).map_err(|error| error.to_string())?;
    file.seek(SeekFrom::Start(entry.offset()))
        .map_err(|error| error.to_string())?;
    let mut remaining = entry.compressed_length();
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    while remaining > 0 {
        let count = usize::try_from(remaining.min(buffer.len() as u64)).unwrap();
        file.read_exact(&mut buffer[..count])
            .map_err(|error| error.to_string())?;
        hasher.update(&buffer[..count]);
        remaining -= count as u64;
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != source.compressed_payload_sha256 {
        return Err(format!(
            "Packed source entry changed on disk: {} in {}",
            entry.logical_path(),
            entry.archive_path().display()
        ));
    }
    Ok(())
}

fn revision_key(guid: &str, revision: &str) -> String {
    format!(
        "{}/{}",
        guid.to_ascii_uppercase(),
        revision.to_ascii_lowercase()
    )
}

fn register_source_revision(guid: &str, revision: &str, sources: Arc<PackedSourceRevision>) {
    if let Ok(mut revisions) = SOURCE_REVISIONS.get_or_init(Default::default).lock() {
        revisions.insert(revision_key(guid, revision), sources);
    }
}

fn publish_inventory_addon_manifests(
    inventory: &Inventory,
    storage_root: &Path,
) -> Result<(), String> {
    let mut identities = BTreeMap::<String, PathBuf>::new();
    for addon in inventory
        .addons
        .iter()
        .filter(|addon| addon.root_kind != "base-game")
    {
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
            if existing != addon.path {
                return Err(format!(
                    "Duplicate add-on GUID {guid}: {} and {}",
                    existing.display(),
                    addon.path.display()
                ));
            }
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
        write_json_atomic(
            &storage_root.join(&guid).join("inventory.json"),
            &InventoryAddonManifest {
                schema: "reforger-addon-inventory-manifest-v1",
                guid,
                display_id,
                directory_name: addon.directory_name.clone(),
                root_kind: addon.root_kind.clone(),
                source_root: addon.path.clone(),
                semantic_status: "inventory-only",
                pack_artifacts,
            },
        )?;
    }
    Ok(())
}

fn extract_project_property(source: &str, property: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let (left, right) = line.split_once(':')?;
        (left.trim().trim_matches('"') == property).then(|| {
            right
                .trim()
                .trim_end_matches(',')
                .trim()
                .trim_matches('"')
                .to_string()
        })
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
        assert!(
            build_base_game_index(&inventory, &IndexBuildControl::default())
                .unwrap_err()
                .contains("Unsupported")
        );
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
        fs::write(
            &inventory,
            format!(
                r#"{{"schema":"reforger-addon-source-inventory-v1","roots":[{{"kind":"base-game","path":{}}}]}}"#,
                serde_json::to_string(&addons).unwrap()
            ),
        )
        .unwrap();
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

        let loaded =
            load_or_build_base_game_index(&inventory, &storage, &IndexBuildControl::default())
                .unwrap();
        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert!(storage.join(BASE_GAME_GUID).join("current.json").is_file());
        assert!(storage.join(BASE_GAME_GUID).join(&current.index).is_file());
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
        fs::write(
            &inventory,
            format!(
                r#"{{"schema":"reforger-addon-source-inventory-v1","roots":[{{"kind":"base-game","path":{}}}]}}"#,
                serde_json::to_string(&addons).unwrap()
            ),
        )
        .unwrap();
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
            "GUID : \"1337C0DE5DABBEEF\"\nID : \"RHS\"",
        )
        .unwrap();
        fs::write(
            second.join("addon.gproj"),
            "GUID : \"1337C0DE5DABBEEF\"\nID : \"Duplicate\"",
        )
        .unwrap();
        fs::write(first.join("data.pak"), b"fixture").unwrap();
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
        publish_inventory_addon_manifests(&one, &storage).unwrap();
        assert!(storage.join("1337C0DE5DABBEEF/inventory.json").is_file());

        let duplicate = Inventory {
            schema: "reforger-addon-source-inventory-v1".to_string(),
            roots: Vec::new(),
            addons: vec![addon(&first, "First"), addon(&second, "Second")],
        };
        assert!(publish_inventory_addon_manifests(&duplicate, &storage)
            .unwrap_err()
            .contains("Duplicate add-on GUID"));
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
