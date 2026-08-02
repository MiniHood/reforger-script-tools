//! Offline, metadata-only resource discovery for the loaded Game Data scope.

use crate::addon_sources::{
    loaded_addon_archive_paths, read_loaded_addon_sources, read_loaded_addon_sources_allow_stale,
};
use crate::index_build::IndexBuildControl;
use crate::pack::PakArchive;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceKind {
    Prefab,
    Script,
    Audio,
    World,
    Config,
    Model,
    Material,
    Texture,
    Layout,
    Animation,
    Particle,
    Ai,
    String,
    Other,
}

impl ResourceKind {
    pub fn from_extension(extension: &str) -> Self {
        match extension
            .trim_start_matches('.')
            .to_ascii_lowercase()
            .as_str()
        {
            "et" | "pre" => Self::Prefab,
            "c" => Self::Script,
            "wav" | "acp" | "snd" | "smap" | "afm" => Self::Audio,
            "ent" | "terr" | "topo" | "layer" | "bterr" | "bttile" | "ttile" => Self::World,
            "conf" | "ct" | "desc" | "gproj" | "meta" => Self::Config,
            "txo" | "fbx" | "xob" => Self::Model,
            "emat" | "gamemat" | "physmat" | "vhcsurf" | "ragdoll" => Self::Material,
            "dds" | "edds" | "imageset" => Self::Texture,
            "layout" | "styles" => Self::Layout,
            "anm" | "agr" | "asi" | "ast" | "asy" | "agf" | "adeb" | "txa" => Self::Animation,
            "ptc" => Self::Particle,
            "bt" => Self::Ai,
            "st" | "fnt" | "ttf" => Self::String,
            _ => Self::Other,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prefab => "prefab",
            Self::Script => "script",
            Self::Audio => "audio",
            Self::World => "world",
            Self::Config => "config",
            Self::Model => "model",
            Self::Material => "material",
            Self::Texture => "texture",
            Self::Layout => "layout",
            Self::Animation => "animation",
            Self::Particle => "particle",
            Self::Ai => "ai",
            Self::String => "string",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRecord {
    pub addon_guid: String,
    pub addon_id: String,
    pub logical_path: String,
    pub basename: String,
    pub extension: String,
    pub kind: ResourceKind,
    pub registered: bool,
    pub stale: bool,
    pub source_identity: String,
    pub workbench_link: String,
}

impl ResourceRecord {
    pub fn new(
        addon_guid: &str,
        addon_id: &str,
        logical_path: &str,
        source_identity: &str,
    ) -> Self {
        let normalized = logical_path.replace('\\', "/");
        let basename = normalized
            .rsplit('/')
            .next()
            .unwrap_or(&normalized)
            .to_string();
        let extension = basename
            .rsplit_once('.')
            .map(|(_, value)| value.to_ascii_lowercase())
            .unwrap_or_default();
        let kind = ResourceKind::from_extension(&extension);
        let editor = match kind {
            ResourceKind::Script => "ScriptEditor",
            ResourceKind::World => "WorldEditor",
            _ => "ResourceManager",
        };
        let workbench_link = format!("enfusion://{editor}/~{}:{}", addon_id, normalized);
        Self {
            addon_guid: addon_guid.to_ascii_uppercase(),
            addon_id: addon_id.to_string(),
            logical_path: normalized,
            basename,
            extension: extension.clone(),
            kind,
            registered: false,
            stale: false,
            source_identity: source_identity.to_string(),
            workbench_link,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSearchRequest {
    pub catalogue_revision: String,
    pub addon_guids: Option<Vec<String>>,
    pub query: String,
    pub kinds: Option<Vec<ResourceKind>>,
    pub cursor: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSearchResult {
    pub resource_name: String,
    pub addon_guid: String,
    pub addon_id: String,
    pub logical_path: String,
    pub basename: String,
    pub extension: String,
    pub kind: ResourceKind,
    pub registered: bool,
    pub stale: bool,
    pub source_identity: String,
    pub workbench_link: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSearchPage {
    pub catalogue_revision: String,
    pub limit: usize,
    pub total: usize,
    pub results: Vec<ResourceSearchResult>,
    pub truncated: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceCatalogue {
    revision: String,
    records: Vec<ResourceRecord>,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceCatalogueConfig {
    pub addon_source_inventory: Option<PathBuf>,
    pub addon_index_storage: Option<PathBuf>,
    pub workspace_roots: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct ResourceCatalogueService {
    config: ResourceCatalogueConfig,
    state: Mutex<Option<Arc<ResourceCatalogue>>>,
}

impl ResourceCatalogueService {
    pub fn new(config: ResourceCatalogueConfig) -> Self {
        Self {
            config,
            state: Mutex::new(None),
        }
    }

    pub fn search(
        &self,
        control: &IndexBuildControl,
        mut request: ResourceSearchRequest,
    ) -> Result<ResourceSearchPage, ResourceSearchError> {
        let catalogue = {
            let mut state = self.state.lock().unwrap();
            if state.is_none() {
                let (catalogue, _) = ResourceCatalogue::from_config(&self.config, control)
                    .map_err(|_| ResourceSearchError::Unavailable)?;
                *state = Some(Arc::new(catalogue));
            }
            state
                .as_ref()
                .cloned()
                .ok_or(ResourceSearchError::Unavailable)?
        };
        if request.catalogue_revision.is_empty() {
            request.catalogue_revision = catalogue.revision().to_string();
        }
        catalogue.search(control, request)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedAddonResources {
    schema: String,
    fingerprint: String,
    records: Vec<ResourceRecord>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCatalogueStats {
    pub addon_count: usize,
    pub resource_count: usize,
    pub packed_resource_count: usize,
    pub loose_resource_count: usize,
    pub stale_resource_count: usize,
    pub revision_ms: u64,
}

impl ResourceCatalogue {
    pub fn from_config(
        config: &ResourceCatalogueConfig,
        control: &IndexBuildControl,
    ) -> Result<(Self, ResourceCatalogueStats), String> {
        let Some(inventory) = config.addon_source_inventory.as_ref() else {
            return Err("The Workbench loaded add-on inventory is not configured.".to_string());
        };
        let started = Instant::now();
        let addons = match read_loaded_addon_sources(inventory) {
            Ok(addons) => addons,
            Err(_) => read_loaded_addon_sources_allow_stale(inventory)?,
        };
        let mut records = Vec::new();
        let mut packed = 0;
        let mut loose = 0;
        let mut revision_hasher = Sha256::new();
        let workspace_roots = config
            .workspace_roots
            .iter()
            .map(|root| {
                fs::canonicalize(root).map_err(|error| {
                    format!(
                        "Failed to resolve configured workspace root {}: {error}",
                        root.display()
                    )
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        for addon in &addons {
            control
                .check()
                .map_err(|_| "resource catalogue build cancelled".to_string())?;
            revision_hasher.update(addon.guid.as_bytes());
            revision_hasher.update(
                addon
                    .source_root
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .as_bytes(),
            );
            let source_files = match enumerate_loose_files(&addon.source_root, control) {
                Ok(files) => files,
                Err(_) if !workspace_addon_for(&workspace_roots, &addon.source_root) => {
                    if let Some(storage) = config.addon_index_storage.as_ref() {
                        if let Some(cached) =
                            read_cached_addon_any(storage, &addon.guid, &addon.source_root)
                        {
                            revision_hasher.update(cached.fingerprint.as_bytes());
                            packed += cached
                                .records
                                .iter()
                                .filter(|record| record.source_identity.starts_with("packed:"))
                                .count();
                            loose += cached
                                .records
                                .iter()
                                .filter(|record| record.source_identity.starts_with("loose:"))
                                .count();
                            records.extend(cached.records.into_iter().map(|mut record| {
                                record.source_identity =
                                    format!("stale-cache:{}", record.source_identity);
                                record.stale = true;
                                record
                            }));
                            continue;
                        }
                    }
                    return Err("The loaded add-on source is unavailable and has no compatible resource cache.".to_string());
                }
                Err(error) => return Err(error),
            };
            let fingerprint = source_fingerprint(&source_files);
            revision_hasher.update(fingerprint.as_bytes());
            let workspace_addon = workspace_addon_for(&workspace_roots, &addon.source_root);
            if !workspace_addon {
                if let Some(storage) = config.addon_index_storage.as_ref() {
                    if let Some(cached) =
                        read_cached_addon(storage, &addon.guid, &addon.source_root, &fingerprint)
                    {
                        packed += cached
                            .records
                            .iter()
                            .filter(|record| record.source_identity.starts_with("packed:"))
                            .count();
                        loose += cached
                            .records
                            .iter()
                            .filter(|record| record.source_identity.starts_with("loose:"))
                            .count();
                        records.extend(cached.records.into_iter().map(|mut record| {
                            record.registered = !workspace_roots
                                .iter()
                                .any(|root| root.starts_with(&addon.source_root));
                            record
                        }));
                        continue;
                    }
                }
            }
            let addon_start = records.len();
            let archives = loaded_addon_archive_paths(&addon.source_root)?;
            for archive_path in archives {
                control
                    .check()
                    .map_err(|_| "resource catalogue build cancelled".to_string())?;
                let archive =
                    PakArchive::inspect_with_cancel(&archive_path, || control.is_cancelled())
                        .map_err(|error| {
                            format!("Failed to inspect {}: {error}", archive_path.display())
                        })?;
                let identity = format!("packed:{}", archive_path.display());
                for entry in archive.entries() {
                    let mut record = ResourceRecord::new(
                        &addon.guid,
                        &addon.display_id,
                        entry.logical_path(),
                        &identity,
                    );
                    record.registered = !workspace_roots
                        .iter()
                        .any(|root| root.starts_with(&addon.source_root));
                    records.push(record);
                    packed += 1;
                }
            }
            for file in source_files {
                let logical = file
                    .strip_prefix(&addon.source_root)
                    .unwrap_or(&file)
                    .to_string_lossy()
                    .replace('\\', "/");
                if logical.eq_ignore_ascii_case("resourceDatabase.rdb")
                    || logical.to_ascii_lowercase().ends_with(".pak")
                {
                    continue;
                }
                let identity = format!("loose:{}", addon.source_root.display());
                let mut record =
                    ResourceRecord::new(&addon.guid, &addon.display_id, &logical, &identity);
                record.registered = !workspace_roots
                    .iter()
                    .any(|root| root.starts_with(&addon.source_root));
                records.push(record);
                loose += 1;
            }
            if !workspace_addon {
                if let Some(storage) = config.addon_index_storage.as_ref() {
                    let cached = CachedAddonResources {
                        schema: "reforger-resource-catalogue-addon-v1".to_string(),
                        fingerprint,
                        records: records[addon_start..].to_vec(),
                    };
                    write_cached_addon(storage, &addon.guid, &addon.source_root, &cached)?;
                }
            }
        }
        let revision = format!("resource-v1:{:x}", revision_hasher.finalize());
        let stats = ResourceCatalogueStats {
            addon_count: addons.len(),
            resource_count: records.len(),
            packed_resource_count: packed,
            loose_resource_count: loose,
            stale_resource_count: records.iter().filter(|record| record.stale).count(),
            revision_ms: started.elapsed().as_millis() as u64,
        };
        Ok((Self::from_records(revision, records), stats))
    }
}

const MAX_RESOURCE_FILES: usize = 200_000;
const MAX_RESOURCE_PATH_BYTES: usize = 1024;
const MAX_RESOURCE_NESTING: usize = 128;
const MAX_RESOURCE_RESULTS: usize = 10_000;
const RESOURCE_CACHE_MAGIC: &[u8; 8] = b"RSTRSC01";

fn source_fingerprint(files: &[PathBuf]) -> String {
    let mut hasher = Sha256::new();
    for file in files {
        hasher.update(file.to_string_lossy().replace('\\', "/").as_bytes());
        if let Ok(metadata) = fs::metadata(file) {
            hasher.update(metadata.len().to_le_bytes());
            hasher.update(
                metadata
                    .modified()
                    .ok()
                    .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|value| value.as_nanos())
                    .unwrap_or(0)
                    .to_le_bytes(),
            );
        }
    }
    format!("{:x}", hasher.finalize())
}

fn cache_path(storage: &Path, guid: &str, source_root: &Path) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(
        source_root
            .to_string_lossy()
            .to_ascii_lowercase()
            .as_bytes(),
    );
    storage.join(format!(
        "resource-{}-{:x}.bin",
        guid.to_ascii_uppercase(),
        hasher.finalize()
    ))
}

fn read_cached_addon_any(
    storage: &Path,
    guid: &str,
    source_root: &Path,
) -> Option<CachedAddonResources> {
    let bytes = fs::read(cache_path(storage, guid, source_root)).ok()?;
    if bytes.len() < RESOURCE_CACHE_MAGIC.len()
        || &bytes[..RESOURCE_CACHE_MAGIC.len()] != RESOURCE_CACHE_MAGIC
    {
        return None;
    }
    let mut decoder = ZlibDecoder::new(&bytes[RESOURCE_CACHE_MAGIC.len()..]);
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded).ok()?;
    let cached = serde_json::from_slice::<CachedAddonResources>(&decoded).ok()?;
    (cached.schema == "reforger-resource-catalogue-addon-v1").then_some(cached)
}

fn read_cached_addon(
    storage: &Path,
    guid: &str,
    source_root: &Path,
    fingerprint: &str,
) -> Option<CachedAddonResources> {
    let bytes = fs::read(cache_path(storage, guid, source_root)).ok()?;
    if bytes.len() < RESOURCE_CACHE_MAGIC.len()
        || &bytes[..RESOURCE_CACHE_MAGIC.len()] != RESOURCE_CACHE_MAGIC
    {
        return None;
    }
    let mut decoder = ZlibDecoder::new(&bytes[RESOURCE_CACHE_MAGIC.len()..]);
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded).ok()?;
    let cached = serde_json::from_slice::<CachedAddonResources>(&decoded).ok()?;
    (cached.schema == "reforger-resource-catalogue-addon-v1" && cached.fingerprint == fingerprint)
        .then_some(cached)
}

fn write_cached_addon(
    storage: &Path,
    guid: &str,
    source_root: &Path,
    cached: &CachedAddonResources,
) -> Result<(), String> {
    fs::create_dir_all(storage).map_err(|error| error.to_string())?;
    let encoded = serde_json::to_vec(cached).map_err(|error| error.to_string())?;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(&encoded)
        .map_err(|error| error.to_string())?;
    let bytes = encoder.finish().map_err(|error| error.to_string())?;
    let mut published = RESOURCE_CACHE_MAGIC.to_vec();
    published.extend_from_slice(&bytes);
    let path = cache_path(storage, guid, source_root);
    let temporary = path.with_extension("bin.tmp");
    fs::write(&temporary, published).map_err(|error| error.to_string())?;
    fs::rename(&temporary, &path).map_err(|error| error.to_string())
}

fn enumerate_loose_files(root: &Path, control: &IndexBuildControl) -> Result<Vec<PathBuf>, String> {
    let root = fs::canonicalize(root).map_err(|error| {
        format!(
            "Failed to resolve add-on source root {}: {error}",
            root.display()
        )
    })?;
    let mut stack = vec![(root.clone(), 0usize)];
    let mut files = Vec::new();
    while let Some((directory, depth)) = stack.pop() {
        control
            .check()
            .map_err(|_| "resource catalogue build cancelled".to_string())?;
        if depth > MAX_RESOURCE_NESTING {
            return Err("resource source nesting exceeds the bounded limit".to_string());
        }
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("Failed to enumerate {}: {error}", directory.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let relative = path.strip_prefix(&root).unwrap_or(&path).to_string_lossy();
            if relative.len() > MAX_RESOURCE_PATH_BYTES {
                return Err("resource logical path exceeds the bounded limit".to_string());
            }
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_dir() {
                stack.push((path, depth + 1));
            } else if file_type.is_file() {
                files.push(path);
                if files.len() > MAX_RESOURCE_FILES {
                    return Err("resource file count exceeds the bounded limit".to_string());
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

fn workspace_addon_for(workspace_roots: &[PathBuf], source_root: &Path) -> bool {
    workspace_roots
        .iter()
        .any(|root| root.starts_with(source_root))
}

impl ResourceCatalogue {
    pub fn from_records(revision: impl Into<String>, mut records: Vec<ResourceRecord>) -> Self {
        records.sort_by(resource_order);
        Self {
            revision: revision.into(),
            records,
        }
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn search(
        &self,
        control: &IndexBuildControl,
        request: ResourceSearchRequest,
    ) -> Result<ResourceSearchPage, ResourceSearchError> {
        control
            .check()
            .map_err(|_| ResourceSearchError::Cancelled)?;
        if request.catalogue_revision != self.revision {
            return Err(ResourceSearchError::StaleRevision);
        }
        let limit = request.limit.clamp(1, 200);
        let offset = decode_cursor(request.cursor.as_deref(), &request)?;
        let selected = request.addon_guids.as_ref().map(|values| {
            values
                .iter()
                .map(|value| value.to_ascii_uppercase())
                .collect::<std::collections::BTreeSet<_>>()
        });
        let kinds = request.kinds.as_ref().map(|values| {
            values
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
        });
        let terms = request
            .query
            .split_whitespace()
            .map(|term| term.to_ascii_lowercase())
            .collect::<Vec<_>>();
        let mut matches = Vec::new();
        for record in &self.records {
            control
                .check()
                .map_err(|_| ResourceSearchError::Cancelled)?;
            if selected
                .as_ref()
                .is_some_and(|values| !values.contains(&record.addon_guid))
                || kinds
                    .as_ref()
                    .is_some_and(|values| !values.contains(&record.kind))
            {
                continue;
            }
            let basename = record.basename.to_ascii_lowercase();
            let path = record.logical_path.to_ascii_lowercase();
            if terms
                .iter()
                .any(|term| !basename.contains(term) && !path.contains(term))
            {
                continue;
            }
            let rank = if terms.is_empty() {
                2
            } else if terms.iter().all(|term| basename.starts_with(term)) {
                0
            } else if terms.iter().all(|term| basename.contains(term)) {
                1
            } else {
                2
            };
            matches.push((rank, record));
        }
        matches.sort_by(|(left_rank, left), (right_rank, right)| {
            left_rank
                .cmp(right_rank)
                .then_with(|| resource_order(left, right))
        });
        let total_matches = matches.len();
        let total = total_matches.min(MAX_RESOURCE_RESULTS);
        let results = matches
            .into_iter()
            .take(MAX_RESOURCE_RESULTS)
            .skip(offset)
            .take(limit)
            .map(|(_, record)| ResourceSearchResult {
                resource_name: format!("{{{}}}{}", record.addon_guid, record.logical_path),
                addon_guid: record.addon_guid.clone(),
                addon_id: record.addon_id.clone(),
                logical_path: record.logical_path.clone(),
                basename: record.basename.clone(),
                extension: record.extension.clone(),
                kind: record.kind,
                registered: record.registered,
                stale: record.stale,
                source_identity: record.source_identity.clone(),
                workbench_link: record.workbench_link.clone(),
            })
            .collect::<Vec<_>>();
        let next_offset = offset + results.len();
        Ok(ResourceSearchPage {
            catalogue_revision: self.revision.clone(),
            limit,
            total,
            results,
            truncated: next_offset < total || total_matches > MAX_RESOURCE_RESULTS,
            next_cursor: (next_offset < total).then(|| encode_cursor(&request, next_offset)),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceSearchError {
    Cancelled,
    StaleRevision,
    InvalidCursor,
    Unavailable,
}

fn resource_order(left: &ResourceRecord, right: &ResourceRecord) -> Ordering {
    left.logical_path
        .to_ascii_lowercase()
        .cmp(&right.logical_path.to_ascii_lowercase())
        .then_with(|| left.addon_guid.cmp(&right.addon_guid))
        .then_with(|| left.source_identity.cmp(&right.source_identity))
}

fn encode_cursor(request: &ResourceSearchRequest, offset: usize) -> String {
    let value = serde_json::json!({
        "revision": request.catalogue_revision,
        "query": request.query.to_ascii_lowercase(),
        "addons": request.addon_guids,
        "kinds": request.kinds,
        "limit": request.limit,
        "offset": offset,
    });
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&value).expect("resource cursor serializes"))
}

fn decode_cursor(
    cursor: Option<&str>,
    request: &ResourceSearchRequest,
) -> Result<usize, ResourceSearchError> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| ResourceSearchError::InvalidCursor)?;
    let value = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|_| ResourceSearchError::InvalidCursor)?;
    let revision = value
        .get("revision")
        .and_then(|value| value.as_str())
        .ok_or(ResourceSearchError::InvalidCursor)?;
    let query = value
        .get("query")
        .and_then(|value| value.as_str())
        .ok_or(ResourceSearchError::InvalidCursor)?;
    let offset = value
        .get("offset")
        .and_then(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ResourceSearchError::InvalidCursor)?;
    let addons = value
        .get("addons")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let expected_addons = serde_json::to_value(&request.addon_guids)
        .map_err(|_| ResourceSearchError::InvalidCursor)?;
    let kinds = value
        .get("kinds")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let expected_kinds =
        serde_json::to_value(&request.kinds).map_err(|_| ResourceSearchError::InvalidCursor)?;
    let limit = value
        .get("limit")
        .and_then(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ResourceSearchError::InvalidCursor)?;
    if revision != request.catalogue_revision
        || query != request.query.to_ascii_lowercase()
        || addons != expected_addons
        || kinds != expected_kinds
        || limit != request.limit
    {
        return Err(ResourceSearchError::InvalidCursor);
    }
    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(path: &str) -> ResourceRecord {
        ResourceRecord::new("1111111111111111", "Test", path, "fixture")
    }

    #[test]
    fn searches_case_insensitive_basename_terms_and_ranks_prefixes() {
        let catalogue = ResourceCatalogue::from_records(
            "r1",
            vec![
                record("Prefabs/Radio.et"),
                record("Prefabs/OldRadio.et"),
                record("Scripts/Radio.c"),
            ],
        );
        let page = catalogue
            .search(
                &IndexBuildControl::default(),
                ResourceSearchRequest {
                    catalogue_revision: "r1".into(),
                    addon_guids: None,
                    query: "RADIO".into(),
                    kinds: None,
                    cursor: None,
                    limit: 10,
                },
            )
            .unwrap();
        assert_eq!(page.results[0].logical_path, "Prefabs/Radio.et");
        assert_eq!(page.results[1].logical_path, "Scripts/Radio.c");
    }

    #[test]
    fn cancelled_search_does_not_publish_a_partial_page() {
        let catalogue = ResourceCatalogue::from_records("r1", vec![record("A.et")]);
        let control = IndexBuildControl::default();
        control.cancel();
        let error = catalogue
            .search(
                &control,
                ResourceSearchRequest {
                    catalogue_revision: "r1".into(),
                    addon_guids: None,
                    query: "".into(),
                    kinds: None,
                    cursor: None,
                    limit: 10,
                },
            )
            .unwrap_err();
        assert_eq!(error, ResourceSearchError::Cancelled);
    }

    #[test]
    fn cursor_is_bound_to_query_and_revision() {
        let catalogue = ResourceCatalogue::from_records(
            "r1",
            vec![record("A.et"), record("B.et"), record("C.c")],
        );
        let first = catalogue
            .search(
                &IndexBuildControl::default(),
                ResourceSearchRequest {
                    catalogue_revision: "r1".into(),
                    addon_guids: None,
                    query: "".into(),
                    kinds: None,
                    cursor: None,
                    limit: 1,
                },
            )
            .unwrap();
        let error = catalogue
            .search(
                &IndexBuildControl::default(),
                ResourceSearchRequest {
                    catalogue_revision: "r1".into(),
                    addon_guids: None,
                    query: "other".into(),
                    kinds: None,
                    cursor: first.next_cursor,
                    limit: 1,
                },
            )
            .unwrap_err();
        assert_eq!(error, ResourceSearchError::InvalidCursor);
        let first = catalogue
            .search(
                &IndexBuildControl::default(),
                ResourceSearchRequest {
                    catalogue_revision: "r1".into(),
                    addon_guids: None,
                    query: "".into(),
                    kinds: Some(vec![ResourceKind::Prefab]),
                    cursor: None,
                    limit: 1,
                },
            )
            .unwrap();
        let error = catalogue
            .search(
                &IndexBuildControl::default(),
                ResourceSearchRequest {
                    catalogue_revision: "r1".into(),
                    addon_guids: None,
                    query: "".into(),
                    kinds: Some(vec![ResourceKind::Script]),
                    cursor: first.next_cursor,
                    limit: 1,
                },
            )
            .unwrap_err();
        assert_eq!(error, ResourceSearchError::InvalidCursor);
    }

    #[test]
    fn loose_catalogue_is_metadata_only_and_reuses_atomic_cache() {
        let root =
            std::env::temp_dir().join(format!("rst-resource-catalogue-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Scripts")).unwrap();
        fs::write(root.join("Scripts/Test.c"), "void Test() {}\n").unwrap();
        fs::write(root.join("Prefabs/Test.et"), b"metadata-only").unwrap_or_else(|_| {
            fs::create_dir_all(root.join("Prefabs")).unwrap();
            fs::write(root.join("Prefabs/Test.et"), b"metadata-only").unwrap();
        });
        let inventory = root.with_extension("inventory.json");
        fs::write(&inventory, format!(r#"{{"schema":"reforger-workbench-loaded-addon-graph-v1","bridgeVersion":"test","protocolVersion":1,"addons":[{{"guid":"1111111111111111","id":"Test","title":"Test","sourceRoot":{}}}]}}"#, serde_json::to_string(&root).unwrap())).unwrap();
        let storage = root.with_extension("cache");
        let config = ResourceCatalogueConfig {
            addon_source_inventory: Some(inventory.clone()),
            addon_index_storage: Some(storage.clone()),
            workspace_roots: Vec::new(),
        };
        let (first, stats) =
            ResourceCatalogue::from_config(&config, &IndexBuildControl::default()).unwrap();
        assert_eq!(stats.loose_resource_count, 2);
        assert!(first
            .records
            .iter()
            .any(|record| record.kind == ResourceKind::Script));
        assert!(fs::read_dir(&storage).unwrap().next().is_some());
        let (second, cached_stats) =
            ResourceCatalogue::from_config(&config, &IndexBuildControl::default()).unwrap();
        assert_eq!(second.records, first.records);
        assert_eq!(second.revision(), first.revision());
        assert_eq!(cached_stats.resource_count, 2);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(inventory);
        let _ = fs::remove_dir_all(storage);
    }
}
