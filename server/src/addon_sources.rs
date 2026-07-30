use crate::index_build::{
    build_index_from_sources, IndexBuildControl, IndexBuildResult, IndexSourceText,
};
use crate::index_cache::{
    load_or_build_archive_index, GameDataIndexCacheResult, SourceFingerprint,
};
use crate::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, SOURCE_PRIORITY_GAME_DATA,
};
use crate::pack::{PakArchive, PakEntry, PakSelection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use url::Url;

pub const BASE_GAME_GUID: &str = "58D0FB3206B6F859";
pub const VIRTUAL_SOURCE_SCHEME: &str = "reforger-pak";

#[derive(Debug, Deserialize)]
struct Inventory {
    schema: String,
    roots: Vec<InventoryRoot>,
}

#[derive(Debug, Deserialize)]
struct InventoryRoot {
    kind: String,
    path: Option<PathBuf>,
}

struct BaseGameInspection {
    root: PathBuf,
    archives: Vec<(PakArchive, Vec<PakEntry>)>,
    fingerprint: SourceFingerprint,
    artifact_digest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddonIndexManifest<'a> {
    schema: &'static str,
    guid: &'a str,
    artifact_digest: &'a str,
    pack_count: usize,
    script_count: usize,
    index_file: &'static str,
    index_bytes: u64,
}

static ADDON_ROOTS: OnceLock<Mutex<BTreeMap<String, PathBuf>>> = OnceLock::new();

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
    let inspection = inspect_base_game(inventory_path, control)?;
    register_addon_root(BASE_GAME_GUID, inspection.root.clone());
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
    let current = storage_root.join(BASE_GAME_GUID).join("current");
    let cache = current.join("symbols.bin");
    let result = load_or_build_archive_index(
        &cache,
        fingerprint,
        artifact_digest.clone(),
        || build_inspected_base_game(inspection, control),
    )?;
    write_manifest(
        &current,
        AddonIndexManifest {
            schema: "reforger-addon-index-manifest-v1",
            guid: BASE_GAME_GUID,
            artifact_digest: &artifact_digest,
            pack_count,
            script_count,
            index_file: "symbols.bin",
            index_bytes: result.cache_file_bytes.unwrap_or(0),
        },
    )?;
    Ok(result)
}

/// Resolves one immutable virtual document from its PAC catalogue entry. Only
/// the requested source payload is decoded.
pub fn read_virtual_source(uri: &str) -> Result<String, String> {
    let parsed = Url::parse(uri).map_err(|error| format!("Invalid pack source URI: {error}"))?;
    if parsed.scheme() != VIRTUAL_SOURCE_SCHEME {
        return Err(format!("Unsupported source URI scheme '{}'", parsed.scheme()));
    }
    let guid = parsed
        .host_str()
        .ok_or_else(|| "Pack source URI has no add-on GUID".to_string())?
        .to_ascii_uppercase();
    let logical_path = parsed.path().trim_start_matches('/');
    if logical_path.is_empty() {
        return Err("Pack source URI has no logical path".to_string());
    }
    let root = ADDON_ROOTS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "Add-on source registry is unavailable".to_string())?
        .get(&guid)
        .cloned()
        .ok_or_else(|| format!("Add-on {guid} is not loaded"))?;
    for archive_path in base_game_archive_paths(&root) {
        let archive = PakArchive::inspect(&archive_path)
            .map_err(|error| format!("Failed to inspect {}: {error}", archive_path.display()))?;
        match archive.select(PakSelection::exact_paths(&[logical_path])) {
            Ok(mut entries) => {
                let entry = entries.remove(0);
                let mut bytes = Vec::with_capacity(entry.original_length() as usize);
                archive
                    .reader()
                    .map_err(|error| error.to_string())?
                    .read_to(&entry, &mut bytes)
                    .map_err(|error| error.to_string())?;
                return String::from_utf8(bytes)
                    .map_err(|_| format!("Pack source {logical_path} is not UTF-8"));
            }
            Err(crate::pack::PackError::MissingPath(_)) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Err(format!("Pack source does not exist: {logical_path}"))
}

fn inspect_base_game(
    inventory_path: &Path,
    control: &IndexBuildControl,
) -> Result<BaseGameInspection, String> {
    let root = base_game_root(inventory_path)?;
    let mut hasher = Sha256::new();
    hasher.update(b"reforger-base-pac-catalogue-v1");
    let mut archives = Vec::new();
    let mut latest_modified = 0_u128;
    let mut script_count = 0_usize;
    for archive_path in base_game_archive_paths(&root) {
        control.check()?;
        let metadata = fs::metadata(&archive_path)
            .map_err(|error| format!("Failed to stat {}: {error}", archive_path.display()))?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|time| time.as_millis())
            .unwrap_or(0);
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
        for entry in &entries {
            hasher.update(entry.logical_path().as_bytes());
            hasher.update(entry.offset().to_le_bytes());
            hasher.update(entry.compressed_length().to_le_bytes());
            hasher.update(entry.original_length().to_le_bytes());
            hasher.update(entry.compression().to_le_bytes());
        }
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
    })
}

fn build_inspected_base_game(
    inspection: BaseGameInspection,
    control: &IndexBuildControl,
) -> Result<IndexBuildResult, String> {
    register_addon_root(BASE_GAME_GUID, inspection.root);
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
            let uri = virtual_source_uri(BASE_GAME_GUID, &logical)?;
            sources.push(IndexSourceText {
                display_path: PathBuf::from(&uri),
                bytes,
                metadata: SourceFileMetadata {
                    kind: SourceKind::GameData,
                    category: source_category_for_path(SourceKind::GameData, Some(&relative)),
                    absolute_path: Some(PathBuf::from(uri)),
                    root_path: None,
                    relative_path: Some(relative),
                    priority: SOURCE_PRIORITY_GAME_DATA,
                },
            });
        }
    }
    build_index_from_sources(sources, control)
}

fn virtual_source_uri(guid: &str, logical_path: &str) -> Result<String, String> {
    let base = Url::parse(&format!("{VIRTUAL_SOURCE_SCHEME}://{guid}/"))
        .map_err(|error| error.to_string())?;
    base.join(logical_path).map(|uri| uri.to_string()).map_err(|error| error.to_string())
}

fn register_addon_root(guid: &str, root: PathBuf) {
    if let Ok(mut roots) = ADDON_ROOTS.get_or_init(Default::default).lock() {
        roots.insert(guid.to_ascii_uppercase(), root);
    }
}

fn base_game_archive_paths(root: &Path) -> [PathBuf; 2] {
    [
        root.join("data").join("data007.pak"),
        root.join("core").join("data.pak"),
    ]
}

fn write_manifest(current: &Path, manifest: AddonIndexManifest<'_>) -> Result<(), String> {
    fs::create_dir_all(current).map_err(|error| error.to_string())?;
    let target = current.join("manifest.json");
    let temporary = current.join(format!("manifest.{}.tmp", std::process::id()));
    let json = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(&temporary, json).map_err(|error| error.to_string())?;
    fs::rename(&temporary, &target).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("Failed to publish {}: {error}", target.display())
    })
}

fn base_game_root(inventory_path: &Path) -> Result<PathBuf, String> {
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
    inventory
        .roots
        .into_iter()
        .find(|root| root.kind == "base-game")
        .and_then(|root| root.path)
        .ok_or_else(|| "Arma Reforger base-game add-ons folder is unavailable".to_string())
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
            virtual_source_uri(BASE_GAME_GUID, "scripts/Game/My File.c").unwrap(),
            "reforger-pak://58D0FB3206B6F859/scripts/Game/My%20File.c"
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
            &[("Feature.c", b"class Feature {}")],
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

        let rebuilt = load_or_build_base_game_index(
            &inventory,
            &storage,
            &IndexBuildControl::default(),
        )
        .unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert_eq!(rebuilt.summary.files, 2);
        assert_eq!(
            read_virtual_source(
                "reforger-pak://58D0FB3206B6F859/Root/Feature.c"
            )
            .unwrap(),
            "class Feature {}"
        );

        let loaded = load_or_build_base_game_index(
            &inventory,
            &storage,
            &IndexBuildControl::default(),
        )
        .unwrap();
        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert!(storage
            .join(BASE_GAME_GUID)
            .join("current/manifest.json")
            .is_file());
        assert!(!storage.join(BASE_GAME_GUID).join("scripts").exists());
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
        let data_length = entries.iter().map(|(_, content)| content.len()).sum::<usize>();
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
