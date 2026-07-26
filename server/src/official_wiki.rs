use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, OnceLock};
use url::Url;

const MAX_PAGE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_REPORTED_PATHS: usize = 20;
const EXCLUDED_INDEX: &str = "wiki-index.md";
const CANONICAL_HOST: &str = "community.bistudio.com";
const DEFAULT_SEARCH_LIMIT: usize = 20;
const MAX_SEARCH_LIMIT: usize = 100;
const MAX_CURSOR_BYTES: usize = 2 * 1024;
const MAX_EXCERPT_LINES: usize = 12;
const MAX_EXCERPT_BYTES: usize = 4 * 1024;
const MAX_SEARCH_RESULT_BYTES: usize = 256 * 1024;
const DEFAULT_READ_LINES: usize = 200;
const MAX_READ_LINES: usize = 500;
const MAX_READ_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone)]
pub struct OfficialWikiCorpus {
    root: Option<PathBuf>,
    validated: Arc<OnceLock<ValidatedCorpus>>,
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
    title: String,
    source_url: String,
}

#[derive(Debug, Clone)]
struct ValidatedCorpus {
    status: OfficialWikiStatus,
    pages: Vec<ValidatedPage>,
}

#[derive(Debug, Clone)]
pub struct OfficialWikiSearchRequest {
    pub query: String,
    pub path_prefix: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OfficialWikiSearchPage {
    pub source: String,
    pub corpus_revision: String,
    pub query: String,
    pub applied_filters: OfficialWikiSearchFilters,
    pub returned: usize,
    pub total: usize,
    pub next_cursor: Option<String>,
    pub results: Vec<OfficialWikiSearchHit>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OfficialWikiSearchFilters {
    pub path_prefix: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OfficialWikiSearchHit {
    pub relative_path: String,
    pub title: String,
    pub heading: String,
    pub start_line: usize,
    pub end_line: usize,
    pub excerpt: String,
    pub source_url: String,
    pub matched_fields: Vec<String>,
    pub match_kind: OfficialWikiMatchKind,
    pub read_input: OfficialWikiReadInput,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OfficialWikiMatchKind {
    ExactTitle,
    TitlePhrase,
    Path,
    Heading,
    Body,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OfficialWikiReadInput {
    pub corpus_revision: String,
    pub relative_path: String,
    pub start_line: usize,
    pub line_count: usize,
}

#[derive(Debug, Clone)]
pub struct OfficialWikiReadRequest {
    pub corpus_revision: String,
    pub relative_path: String,
    pub start_line: Option<usize>,
    pub line_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OfficialWikiReadPage {
    pub source: String,
    pub corpus_revision: String,
    pub relative_path: String,
    pub title: String,
    pub source_url: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub truncated: bool,
    pub continuation: Option<OfficialWikiReadInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfficialWikiSearchError {
    Unavailable,
    InvalidQuery,
    InvalidFilter,
    InvalidCursor,
    StaleCursor,
    Changed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfficialWikiReadError {
    Unavailable,
    InvalidPath,
    InvalidRange,
    StaleRevision,
    Changed,
    Cancelled,
}

#[derive(Debug, Clone, Default)]
pub struct OfficialWikiControl(Arc<AtomicBool>);

impl OfficialWikiControl {
    pub fn cancel(&self) { self.0.store(true, Ordering::Release); }
    fn is_cancelled(&self) -> bool { self.0.load(Ordering::Acquire) }
}

impl OfficialWikiCorpus {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: Some(root),
            validated: Arc::new(OnceLock::new()),
        }
    }

    pub fn packaged() -> Self {
        Self {
            root: packaged_root(),
            validated: Arc::new(OnceLock::new()),
        }
    }

    pub fn status(&self) -> OfficialWikiStatus {
        self.validated
            .get_or_init(|| self.validate())
            .status
            .clone()
    }

    pub fn search(
        &self,
        request: OfficialWikiSearchRequest,
    ) -> Result<OfficialWikiSearchPage, OfficialWikiSearchError> {
        self.search_with_control(request, &OfficialWikiControl::default())
    }

    pub fn search_with_control(
        &self,
        request: OfficialWikiSearchRequest,
        control: &OfficialWikiControl,
    ) -> Result<OfficialWikiSearchPage, OfficialWikiSearchError> {
        let corpus = self.validated.get_or_init(|| self.validate());
        let revision = corpus
            .status
            .corpus_revision
            .clone()
            .ok_or(OfficialWikiSearchError::Unavailable)?;
        let query = normalize_query(&request.query).ok_or(OfficialWikiSearchError::InvalidQuery)?;
        let prefix = normalize_prefix(request.path_prefix.as_deref())?;
        let limit = request
            .limit
            .unwrap_or(DEFAULT_SEARCH_LIMIT)
            .clamp(1, MAX_SEARCH_LIMIT);
        let offset = match request.cursor.as_deref() {
            Some(cursor) => {
                if cursor.len() > MAX_CURSOR_BYTES {
                    return Err(OfficialWikiSearchError::InvalidCursor);
                }
                let cursor = decode_cursor(cursor).ok_or(OfficialWikiSearchError::InvalidCursor)?;
                if cursor.query_hash != cursor_hash(&query)
                    || cursor.prefix_hash != cursor_hash(prefix.as_deref().unwrap_or("")) {
                    return Err(OfficialWikiSearchError::InvalidCursor);
                }
                if cursor.revision != revision {
                    return Err(OfficialWikiSearchError::StaleCursor);
                }
                cursor.offset
            }
            None => 0,
        };
        let mut hits = Vec::new();
        for page in &corpus.pages {
            if control.is_cancelled() { return Err(OfficialWikiSearchError::Cancelled); }
            if prefix
                .as_ref()
                .is_some_and(|prefix| !page.logical_path.starts_with(prefix))
            {
                continue;
            }
            let contents = self.read_current_page(page)?;
            hits.extend(search_page(page, &contents, &query, &revision));
        }
        hits.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.relative_path.cmp(&right.1.relative_path))
                .then_with(|| left.1.start_line.cmp(&right.1.start_line))
        });
        if offset > hits.len() {
            return Err(OfficialWikiSearchError::InvalidCursor);
        }
        let total = hits.len();
        let mut results: Vec<_> = hits
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|(_, hit)| hit)
            .collect();
        let mut next_offset = offset + results.len();
        let filters = OfficialWikiSearchFilters {
            path_prefix: prefix.clone(),
            limit,
        };
        loop {
            let next_cursor = (next_offset < total).then(|| {
                encode_cursor(&SearchCursor {
                    revision: revision.clone(),
                    query_hash: cursor_hash(&query),
                    prefix_hash: cursor_hash(prefix.as_deref().unwrap_or("")),
                    offset: next_offset,
                })
            });
            let page = OfficialWikiSearchPage {
                source: "evidence-catalogue".to_string(),
                corpus_revision: revision.clone(),
                query: query.clone(),
                applied_filters: filters.clone(),
                returned: results.len(),
                total,
                next_cursor,
                results: results.clone(),
            };
            if serde_json::to_vec(&page).is_ok_and(|json| json.len() <= MAX_SEARCH_RESULT_BYTES)
                || results.is_empty()
            {
                return Ok(page);
            }
            results.pop();
            next_offset = offset + results.len();
        }
    }

    pub fn read_with_control(
        &self,
        request: OfficialWikiReadRequest,
        control: &OfficialWikiControl,
    ) -> Result<OfficialWikiReadPage, OfficialWikiReadError> {
        let corpus = self.validated.get_or_init(|| self.validate());
        let revision = corpus
            .status
            .corpus_revision
            .clone()
            .ok_or(OfficialWikiReadError::Unavailable)?;
        if request.corpus_revision != revision {
            return Err(OfficialWikiReadError::StaleRevision);
        }
        validate_logical_path(&request.relative_path).map_err(|_| OfficialWikiReadError::InvalidPath)?;
        let page = corpus
            .pages
            .iter()
            .find(|page| page.logical_path == request.relative_path)
            .ok_or(OfficialWikiReadError::InvalidPath)?;
        if control.is_cancelled() {
            return Err(OfficialWikiReadError::Cancelled);
        }
        wait_for_test_read_delay(control)?;
        let contents = self.read_current_page_bytes(page)?;
        if control.is_cancelled() {
            return Err(OfficialWikiReadError::Cancelled);
        }
        let start_line = request.start_line.unwrap_or(1);
        if start_line == 0 {
            return Err(OfficialWikiReadError::InvalidRange);
        }
        let line_count = request
            .line_count
            .unwrap_or(DEFAULT_READ_LINES)
            .clamp(1, MAX_READ_LINES);
        let lines = contents.split_inclusive('\n').collect::<Vec<_>>();
        if start_line > lines.len() {
            return Err(OfficialWikiReadError::InvalidRange);
        }
        let mut content = String::new();
        let mut taken = 0usize;
        for line in lines.iter().skip(start_line - 1).take(line_count) {
            if control.is_cancelled() {
                return Err(OfficialWikiReadError::Cancelled);
            }
            if content.len() + line.len() > MAX_READ_BYTES {
                break;
            }
            content.push_str(line);
            taken += 1;
        }
        if taken == 0 && start_line <= lines.len() {
            return Err(OfficialWikiReadError::InvalidRange);
        }
        let end_line = if taken == 0 { start_line - 1 } else { start_line + taken - 1 };
        let truncated = end_line < lines.len();
        Ok(OfficialWikiReadPage {
            source: "evidence-catalogue".to_string(),
            corpus_revision: revision.clone(),
            relative_path: page.logical_path.clone(),
            title: page.title.clone(),
            source_url: page.source_url.clone(),
            start_line,
            end_line,
            content,
            truncated,
            continuation: truncated.then_some(OfficialWikiReadInput {
                corpus_revision: revision,
                relative_path: page.logical_path.clone(),
                start_line: end_line + 1,
                line_count,
            }),
        })
    }

    fn read_current_page(&self, page: &ValidatedPage) -> Result<String, OfficialWikiSearchError> {
        self.read_current_page_bytes(page)
            .map_err(|error| match error {
                OfficialWikiReadError::Unavailable => OfficialWikiSearchError::Unavailable,
                OfficialWikiReadError::Changed => OfficialWikiSearchError::Changed,
                _ => OfficialWikiSearchError::Changed,
            })
    }

    fn read_current_page_bytes(&self, page: &ValidatedPage) -> Result<String, OfficialWikiReadError> {
        let root = self
            .root
            .as_ref()
            .and_then(|root| fs::canonicalize(root).ok())
            .ok_or(OfficialWikiReadError::Unavailable)?;
        let path = root.join(
            page.logical_path
                .replace('/', std::path::MAIN_SEPARATOR_STR),
        );
        let canonical_path = fs::canonicalize(path).map_err(|_| OfficialWikiReadError::Changed)?;
        if !canonical_path.starts_with(&root) {
            return Err(OfficialWikiReadError::Changed);
        }
        let bytes = fs::read(canonical_path).map_err(|_| OfficialWikiReadError::Changed)?;
        if bytes.len() as u64 != page.bytes || <[u8; 32]>::from(Sha256::digest(&bytes)) != page.hash
        {
            return Err(OfficialWikiReadError::Changed);
        }
        String::from_utf8(bytes).map_err(|_| OfficialWikiReadError::Changed)
    }

    fn validate(&self) -> ValidatedCorpus {
        let root = match self
            .root
            .as_ref()
            .and_then(|root| fs::canonicalize(root).ok())
        {
            Some(root) if root.is_dir() => root,
            _ => {
                return ValidatedCorpus {
                    status: unavailable_status(
                        "official_wiki_unavailable",
                        "The packaged Official Wiki Corpus is unavailable.",
                    ),
                    pages: Vec::new(),
                }
            }
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
                None => {
                    invalid.insert("<invalid logical path>".to_string());
                    continue;
                }
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
                Err(path) => {
                    invalid.insert(path);
                }
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
                message:
                    "Malformed Official Wiki pages were excluded from the authoritative corpus."
                        .to_string(),
            });
        }
        if pages.is_empty() {
            warnings.push(OfficialWikiWarning {
                code: "official_wiki_empty".to_string(),
                message: "No valid Official Wiki Markdown pages were found.".to_string(),
            });
        }
        let status = OfficialWikiStatus {
            source: "evidence-catalogue".to_string(),
            available: !pages.is_empty(),
            corpus_revision,
            file_count: pages.len(),
            total_bytes,
            excluded_files: excluded,
            invalid_file_count: invalid.len(),
            invalid_files,
            limits: OfficialWikiLimits {
                max_page_bytes: MAX_PAGE_BYTES,
                max_reported_invalid_files: MAX_REPORTED_PATHS,
            },
            cold_search_target_ms: 5_000,
            warnings,
            recovery: vec![
                "Reinstall or update Reforger Script Tools, then restart the MCP process."
                    .to_string(),
            ],
        };
        ValidatedCorpus { status, pages }
    }
}

#[cfg(debug_assertions)]
fn wait_for_test_read_delay(control: &OfficialWikiControl) -> Result<(), OfficialWikiReadError> {
    let Some(delay_ms) = std::env::var("REFORGER_MCP_TEST_OFFICIAL_WIKI_READ_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return Ok(());
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(delay_ms);
    while std::time::Instant::now() < deadline {
        if control.is_cancelled() {
            return Err(OfficialWikiReadError::Cancelled);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(())
}

#[cfg(not(debug_assertions))]
fn wait_for_test_read_delay(_control: &OfficialWikiControl) -> Result<(), OfficialWikiReadError> {
    Ok(())
}

fn packaged_root() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.canonicalize().ok())
        .and_then(|path| {
            path.parent()?
                .parent()?
                .parent()?
                .parent()
                .map(|root| root.join("data").join("official-wiki"))
        })
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
        limits: OfficialWikiLimits {
            max_page_bytes: MAX_PAGE_BYTES,
            max_reported_invalid_files: MAX_REPORTED_PATHS,
        },
        cold_search_target_ms: 5_000,
        warnings: vec![OfficialWikiWarning {
            code: code.to_string(),
            message: message.to_string(),
        }],
        recovery: vec![
            "Reinstall or update Reforger Script Tools, then restart the MCP process.".to_string(),
        ],
    }
}

fn collect_markdown(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
    invalid: &mut BTreeSet<String>,
) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => {
            invalid.insert(display_logical(root, directory));
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                invalid.insert(display_logical(root, directory));
                continue;
            }
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                invalid.insert(display_logical(root, &path));
                continue;
            }
        };
        if metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
            invalid.insert(display_logical(root, &path));
            continue;
        }
        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
            && !metadata.is_file()
        {
            invalid.insert(display_logical(root, &path));
            continue;
        }
        if metadata.is_dir() {
            collect_markdown(root, &path, files, invalid);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            files.push(path);
        }
    }
}

fn logical_path(root: &Path, path: &Path) -> Option<String> {
    let canonical = fs::canonicalize(path).ok()?;
    let relative = canonical.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_string()),
            _ => return None,
        }
    }
    let path = (!parts.is_empty()).then(|| parts.join("/"))?;
    validate_logical_path(&path).ok()?;
    Some(path)
}

fn validate_logical_path(path: &str) -> Result<(), ()> {
    if path.is_empty()
        || path.contains('\0')
        || path.contains('\\')
        || path.starts_with('/')
        || path.starts_with("//")
        || path.contains(':')
        || !path.ends_with(".md")
    {
        return Err(());
    }
    if path
        .split('/')
        .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(());
    }
    Ok(())
}

fn display_logical(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .and_then(|path| path.to_str())
        .map(|path| path.replace('\\', "/"))
        .unwrap_or_else(|| "<invalid logical path>".to_string())
}

fn validate_page(path: &Path, logical_path: String) -> Result<ValidatedPage, String> {
    let metadata = fs::metadata(path).map_err(|_| logical_path.clone())?;
    if !metadata.is_file() || metadata.len() > MAX_PAGE_BYTES {
        return Err(logical_path);
    }
    let contents = fs::read(path).map_err(|_| logical_path.clone())?;
    let text = std::str::from_utf8(&contents).map_err(|_| logical_path.clone())?;
    let (title, source) = text
        .lines()
        .next()
        .and_then(parse_h1_source)
        .ok_or_else(|| logical_path.clone())?;
    let parsed = Url::parse(source).map_err(|_| logical_path.clone())?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some(CANONICAL_HOST)
        || !parsed.path().starts_with("/wiki/")
    {
        return Err(logical_path);
    }
    let hash: [u8; 32] = Sha256::digest(&contents).into();
    Ok(ValidatedPage {
        logical_path,
        bytes: contents.len() as u64,
        hash,
        title: title.to_string(),
        source_url: source.to_string(),
    })
}

fn normalize_query(value: &str) -> Option<String> {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    (!value.is_empty() && value.chars().count() <= 256).then_some(value.to_ascii_lowercase())
}

fn normalize_prefix(value: Option<&str>) -> Result<Option<String>, OfficialWikiSearchError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().trim_end_matches('/');
    if value.is_empty()
        || value.len() > 2048
        || value.contains('\\')
        || value.starts_with('/')
        || value.contains(':')
        || value.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(OfficialWikiSearchError::InvalidFilter);
    }
    Ok(Some(format!("{value}/")))
}

fn search_page(
    page: &ValidatedPage,
    contents: &str,
    query: &str,
    revision: &str,
) -> Vec<(u8, OfficialWikiSearchHit)> {
    let lines: Vec<&str> = contents.lines().collect();
    let terms: Vec<&str> = query.split(' ').collect();
    let mut sections = Vec::new();
    let mut start = 1usize;
    let mut heading = page.title.clone();
    for (index, line) in lines.iter().enumerate().skip(1) {
        if let Some(found) = markdown_heading(line) {
            sections.push((start, index, heading));
            start = index + 1;
            heading = found.to_string();
        }
    }
    sections.push((start, lines.len(), heading));
    sections
        .into_iter()
        .filter_map(|(start, end, heading)| {
            let body = lines
                .get(start.saturating_sub(1)..end)
                .unwrap_or_default()
                .join("\n");
            let path = page.logical_path.to_ascii_lowercase();
            let title = page.title.to_ascii_lowercase();
            let heading_lower = heading.to_ascii_lowercase();
            let body_lower = body.to_ascii_lowercase();
            if !terms.iter().all(|term| {
                path.contains(term)
                    || title.contains(term)
                    || heading_lower.contains(term)
                    || body_lower.contains(term)
            }) {
                return None;
            }
            let mut fields = Vec::new();
            if terms.iter().any(|term| title.contains(term)) {
                fields.push("title".to_string());
            }
            if terms.iter().any(|term| path.contains(term)) {
                fields.push("path".to_string());
            }
            if terms.iter().any(|term| heading_lower.contains(term)) {
                fields.push("heading".to_string());
            }
            if terms.iter().any(|term| body_lower.contains(term)) {
                fields.push("body".to_string());
            }
        let (rank, kind) = if title == query {
            (0, OfficialWikiMatchKind::ExactTitle)
        } else if title.contains(query) {
            (1, OfficialWikiMatchKind::TitlePhrase)
        } else if path.contains(query) {
            (2, OfficialWikiMatchKind::Path)
        } else if heading_lower.contains(query) {
            (3, OfficialWikiMatchKind::Heading)
        } else {
            (4, OfficialWikiMatchKind::Body)
            };
            let line_count = end
                .saturating_sub(start)
                .saturating_add(1)
                .min(MAX_EXCERPT_LINES);
            let excerpt =
                bounded_excerpt(lines.get(start.saturating_sub(1)..end).unwrap_or_default());
            Some((
                rank,
                OfficialWikiSearchHit {
                    relative_path: page.logical_path.clone(),
                    title: page.title.clone(),
                    heading,
                    start_line: start,
                    end_line: end.max(start),
                    excerpt,
                    source_url: page.source_url.clone(),
                    matched_fields: fields,
                match_kind: kind,
                    read_input: OfficialWikiReadInput {
                        corpus_revision: revision.to_string(),
                        relative_path: page.logical_path.clone(),
                        start_line: start,
                        line_count,
                    },
                },
            ))
        })
        .collect()
}

fn markdown_heading(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let marker_length = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&marker_length) || !line.as_bytes().get(marker_length).is_some_and(u8::is_ascii_whitespace) {
        return None;
    }
    let value = line[marker_length..].trim();
    (!value.is_empty()).then_some(value.trim_end_matches('#').trim())
}

fn bounded_excerpt(lines: &[&str]) -> String {
    let mut excerpt = String::new();
    for line in lines.iter().take(MAX_EXCERPT_LINES) {
        let separator = (!excerpt.is_empty()).then_some("\n").unwrap_or("");
        if excerpt.len() + separator.len() + line.len() > MAX_EXCERPT_BYTES {
            break;
        }
        excerpt.push_str(separator);
        excerpt.push_str(line);
    }
    excerpt
}

#[derive(Serialize, Deserialize)]
struct SearchCursor {
    revision: String,
    query_hash: String,
    prefix_hash: String,
    offset: usize,
}

fn cursor_hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn encode_cursor(cursor: &SearchCursor) -> String {
    hex_encode(
        serde_json::to_vec(cursor)
            .expect("cursor serializes")
            .as_slice(),
    )
}
fn decode_cursor(value: &str) -> Option<SearchCursor> {
    serde_json::from_slice(&hex_decode(value)?).ok()
}
fn hex_encode(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

fn parse_h1_source(line: &str) -> Option<(&str, &str)> {
    let value = line.strip_prefix("# [")?;
    let (title, source) = value.rsplit_once("](")?;
    let source = source.strip_suffix(')')?;
    (!title.is_empty() && !source.is_empty()).then_some((title, source))
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
    use super::{encode_cursor, markdown_heading, OfficialWikiControl, SearchCursor, validate_logical_path, OfficialWikiCorpus, OfficialWikiSearchError, OfficialWikiSearchRequest, MAX_CURSOR_BYTES};
    use std::fs;

    #[test]
    fn validates_authoritative_pages_without_counting_the_rough_index() {
        let root = std::env::temp_dir().join(format!("official-wiki-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("index.md"),
            "# [Index](https://community.bistudio.com/wiki/Category:Arma_Reforger)\n",
        )
        .unwrap();
        fs::write(root.join("wiki-index.md"), "# Wiki Markdown Index\n").unwrap();
        let status = OfficialWikiCorpus::new(root.clone()).status();
        assert!(status.available);
        assert_eq!(status.file_count, 1);
        assert_eq!(status.excluded_files, ["wiki-index.md"]);
        assert!(status.corpus_revision.is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_non_authoritative_paths_and_malformed_page_metadata() {
        for path in [
            "C:/wiki.md",
            "//server/wiki.md",
            "/wiki.md",
            "\\wiki.md",
            "a\\wiki.md",
            "a//wiki.md",
            "./wiki.md",
            "a/../wiki.md",
            "wiki.txt",
            "bad\0wiki.md",
        ] {
            assert!(
                validate_logical_path(path).is_err(),
                "{path:?} must be rejected"
            );
        }

        let root =
            std::env::temp_dir().join(format!("official-wiki-invalid-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("index.md"),
            "# [](https://community.bistudio.com/wiki/Category:Arma_Reforger)\n",
        )
        .unwrap();
        fs::write(
            root.join("valid.md"),
            "# [Valid](https://community.bistudio.com/wiki/Arma_Reforger:Valid)\n",
        )
        .unwrap();
        fs::create_dir(root.join("directory.md")).unwrap();
        let status = OfficialWikiCorpus::new(root.clone()).status();
        assert!(status.available);
        assert_eq!(status.file_count, 1);
        assert_eq!(status.invalid_files, ["directory.md", "index.md"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn searches_each_matching_heading_section_once_with_stable_pagination() {
        let root =
            std::env::temp_dir().join(format!("official-wiki-search-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Guides")).unwrap();
        fs::write(
            root.join("wiki-index.md"),
            "# Wiki Markdown Index\nsearch-only noise\n",
        )
        .unwrap();
        fs::write(root.join("Guides/Alpha.md"), "# [Alpha Search](https://community.bistudio.com/wiki/Arma_Reforger:Alpha)\n\n## First\nsearch target here\n\n## Second\nsearch target again\n").unwrap();
        fs::write(root.join("Guides/Beta.md"), "# [Beta](https://community.bistudio.com/wiki/Arma_Reforger:Beta)\n\n## Search heading\ntarget\n").unwrap();
        let corpus = OfficialWikiCorpus::new(root.clone());
        let first = corpus
            .search(OfficialWikiSearchRequest {
                query: " search   target ".to_string(),
                path_prefix: Some("Guides/".to_string()),
                limit: Some(1),
                cursor: None,
            })
            .unwrap();
        assert_eq!(first.query, "search target");
        assert_eq!(first.source, "evidence-catalogue");
        assert_eq!(first.total, 3);
        assert_eq!(first.returned, 1);
        assert_eq!(first.results[0].relative_path, "Guides/Alpha.md");
        assert_eq!(first.results[0].heading, "First");
        assert!(first.next_cursor.is_some());
        assert_eq!(first.results[0].read_input.relative_path, "Guides/Alpha.md");
        let second = corpus
            .search(OfficialWikiSearchRequest {
                query: "search target".to_string(),
                path_prefix: Some("Guides".to_string()),
                limit: Some(1000),
                cursor: first.next_cursor,
            })
            .unwrap();
        assert_eq!(second.returned, 2);
        assert_eq!(second.results[0].heading, "Second");
        assert_eq!(second.results[1].relative_path, "Guides/Beta.md");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_all_markdown_heading_depths_and_keeps_cursors_within_the_protocol_bound() {
        assert_eq!(markdown_heading("### Details"), Some("Details"));
        assert_eq!(markdown_heading("###### Deep"), Some("Deep"));
        assert_eq!(markdown_heading("#not a heading"), None);
        let cursor = encode_cursor(&SearchCursor { revision: "ow1:revision".to_string(), query_hash: "q".repeat(64), prefix_hash: "p".repeat(64), offset: usize::MAX });
        assert!(cursor.len() <= MAX_CURSOR_BYTES);
    }

    #[test]
    fn cancellation_is_observed_before_the_direct_corpus_scan() {
        let root = std::env::temp_dir().join(format!("official-wiki-cancel-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("page.md"), "# [Page](https://community.bistudio.com/wiki/Arma_Reforger:Page)\nneedle\n").unwrap();
        let corpus = OfficialWikiCorpus::new(root.clone());
        let control = OfficialWikiControl::default();
        control.cancel();
        assert!(matches!(corpus.search_with_control(OfficialWikiSearchRequest { query: "needle".to_string(), path_prefix: None, limit: None, cursor: None }, &control), Err(OfficialWikiSearchError::Cancelled)));
        let _ = fs::remove_dir_all(root);
    }
}
