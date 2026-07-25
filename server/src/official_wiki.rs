use schemars::JsonSchema;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use url::Url;

const MAX_PAGE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_REPORTED_PATHS: usize = 20;
const EXCLUDED_INDEX: &str = "wiki-index.md";
const CANONICAL_HOST: &str = "community.bistudio.com";

#[derive(Debug, Clone)]
pub struct OfficialWikiCorpus {
    root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OfficialWikiStatus {
    pub source: String,
    pub available: bool,
    pub corpus_revision: Option<String>,
    pub file_count: usize,
    pub total_bytes: u64,
    pub excluded_files: Vec<String>,
    pub invalid_file_count: usize,
    pub invalid_files: Vec<String>,
    pub limits: OfficialWikiLimits,
    pub cold_search_target_ms: u64,
    pub warnings: Vec<OfficialWikiWarning>,
    pub recovery: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OfficialWikiLimits {
    pub max_page_bytes: u64,
    pub max_reported_invalid_files: usize,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OfficialWikiWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone)]
struct ValidatedPage {
    logical_path: String,
    bytes: u64,
    hash: [u8; 32],
}

impl OfficialWikiCorpus {
    pub fn new(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    pub fn packaged() -> Self {
        Self { root: packaged_root() }
    }

    pub fn status(&self) -> OfficialWikiStatus {
        let root = match self.root.as_ref().and_then(|root| fs::canonicalize(root).ok()) {
            Some(root) if root.is_dir() => root,
            _ => return unavailable_status("official_wiki_unavailable", "The packaged Official Wiki Corpus is unavailable."),
        };
        let mut candidates = Vec::new();
        let mut invalid = BTreeSet::new();
        collect_markdown(&root, &root, &mut candidates, &mut invalid);
        candidates.sort();

        let mut pages = Vec::new();
        let mut excluded = Vec::new();
        let mut seen_paths = std::collections::BTreeMap::new();
        for path in candidates {
            let logical_path = match logical_path(&root, &path) {
                Some(path) => path,
                None => { invalid.insert("<invalid logical path>".to_string()); continue; }
            };
            if logical_path == EXCLUDED_INDEX {
                excluded.push(logical_path);
                continue;
            }
            let collision_key = logical_path.to_ascii_lowercase();
            if let Some(previous) = seen_paths.insert(collision_key, logical_path.clone()) {
                pages.retain(|page: &ValidatedPage| page.logical_path != previous);
                invalid.insert(previous);
                invalid.insert(logical_path);
                continue;
            }
            match validate_page(&path, logical_path) {
                Ok(page) => pages.push(page),
                Err(path) => { invalid.insert(path); }
            }
        }
        excluded.sort();
        pages.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
        let total_bytes = pages.iter().map(|page| page.bytes).sum();
        let corpus_revision = (!pages.is_empty()).then(|| revision(&pages));
        let invalid_files: Vec<_> = invalid.iter().take(MAX_REPORTED_PATHS).cloned().collect();
        let mut warnings = Vec::new();
        if !invalid.is_empty() {
            warnings.push(OfficialWikiWarning {
                code: "invalid_official_wiki_pages".to_string(),
                message: "Malformed Official Wiki pages were excluded from the authoritative corpus.".to_string(),
            });
        }
        if pages.is_empty() {
            warnings.push(OfficialWikiWarning {
                code: "official_wiki_empty".to_string(),
                message: "No valid Official Wiki Markdown pages were found.".to_string(),
            });
        }
        OfficialWikiStatus {
            source: "evidence-catalogue".to_string(),
            available: !pages.is_empty(),
            corpus_revision,
            file_count: pages.len(),
            total_bytes,
            excluded_files: excluded,
            invalid_file_count: invalid.len(),
            invalid_files,
            limits: OfficialWikiLimits { max_page_bytes: MAX_PAGE_BYTES, max_reported_invalid_files: MAX_REPORTED_PATHS },
            cold_search_target_ms: 5_000,
            warnings,
            recovery: vec!["Reinstall or update Reforger Script Tools, then restart the MCP process.".to_string()],
        }
    }
}

fn packaged_root() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.canonicalize().ok())
        .and_then(|path| path.parent()?.parent()?.parent()?.parent().map(|root| root.join("resources").join("official-wiki")))
}

fn unavailable_status(code: &str, message: &str) -> OfficialWikiStatus {
    OfficialWikiStatus {
        source: "evidence-catalogue".to_string(),
        available: false,
        corpus_revision: None,
        file_count: 0,
        total_bytes: 0,
        excluded_files: Vec::new(),
        invalid_file_count: 0,
        invalid_files: Vec::new(),
        limits: OfficialWikiLimits { max_page_bytes: MAX_PAGE_BYTES, max_reported_invalid_files: MAX_REPORTED_PATHS },
        cold_search_target_ms: 5_000,
        warnings: vec![OfficialWikiWarning { code: code.to_string(), message: message.to_string() }],
        recovery: vec!["Reinstall or update Reforger Script Tools, then restart the MCP process.".to_string()],
    }
}

fn collect_markdown(root: &Path, directory: &Path, files: &mut Vec<PathBuf>, invalid: &mut BTreeSet<String>) {
    let entries = match fs::read_dir(directory) { Ok(entries) => entries, Err(_) => { invalid.insert(display_logical(root, directory)); return; } };
    for entry in entries.flatten() {
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) { Ok(metadata) => metadata, Err(_) => { invalid.insert(display_logical(root, &path)); continue; } };
        if metadata.file_type().is_symlink() || is_reparse_point(&metadata) { invalid.insert(display_logical(root, &path)); continue; }
        if metadata.is_dir() { collect_markdown(root, &path, files, invalid); }
        else if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("md")) { files.push(path); }
    }
}

fn logical_path(root: &Path, path: &Path) -> Option<String> {
    let canonical = fs::canonicalize(path).ok()?;
    let relative = canonical.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component { Component::Normal(part) => parts.push(part.to_str()?.to_string()), _ => return None }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn display_logical(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).ok().and_then(|path| path.to_str()).map(|path| path.replace('\\', "/")).unwrap_or_else(|| "<invalid logical path>".to_string())
}

fn validate_page(path: &Path, logical_path: String) -> Result<ValidatedPage, String> {
    let metadata = fs::metadata(path).map_err(|_| logical_path.clone())?;
    if !metadata.is_file() || metadata.len() > MAX_PAGE_BYTES { return Err(logical_path); }
    let contents = fs::read(path).map_err(|_| logical_path.clone())?;
    let text = std::str::from_utf8(&contents).map_err(|_| logical_path.clone())?;
    let source = text.lines().next().and_then(parse_h1_source).ok_or_else(|| logical_path.clone())?;
    let parsed = Url::parse(source).map_err(|_| logical_path.clone())?;
    if parsed.scheme() != "https" || parsed.host_str() != Some(CANONICAL_HOST) || !parsed.path().starts_with("/wiki/") { return Err(logical_path); }
    let hash: [u8; 32] = Sha256::digest(&contents).into();
    Ok(ValidatedPage { logical_path, bytes: contents.len() as u64, hash })
}

fn parse_h1_source(line: &str) -> Option<&str> {
    let value = line.strip_prefix("# [")?;
    let (_, source) = value.rsplit_once("](")?;
    source.strip_suffix(')')
}

fn revision(pages: &[ValidatedPage]) -> String {
    let mut hasher = Sha256::new();
    for page in pages {
        hasher.update(page.logical_path.as_bytes());
        hasher.update([0]);
        hasher.update(page.bytes.to_le_bytes());
        hasher.update(page.hash);
    }
    format!("ow1:{:x}", hasher.finalize())
}

#[cfg(target_os = "windows")]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(target_os = "windows"))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::OfficialWikiCorpus;
    use std::fs;

    #[test]
    fn validates_authoritative_pages_without_counting_the_rough_index() {
        let root = std::env::temp_dir().join(format!("official-wiki-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("index.md"), "# [Index](https://community.bistudio.com/wiki/Category:Arma_Reforger)\n").unwrap();
        fs::write(root.join("wiki-index.md"), "# Wiki Markdown Index\n").unwrap();
        let status = OfficialWikiCorpus::new(root.clone()).status();
        assert!(status.available);
        assert_eq!(status.file_count, 1);
        assert_eq!(status.excluded_files, ["wiki-index.md"]);
        assert!(status.corpus_revision.is_some());
        let _ = fs::remove_dir_all(root);
    }
}
