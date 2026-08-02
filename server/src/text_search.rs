use crate::index_build::IndexBuildControl;
use regex::{Regex, RegexBuilder};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use url::Url;

pub const DEFAULT_LIMIT: usize = 20;
pub const MAX_LIMIT: usize = 100;
pub const MAX_QUERY_CHARS: usize = 256;
pub const MAX_CURSOR_BYTES: usize = 2048;
const MAX_RESULT_BYTES: usize = 256 * 1024;
const MAX_EXCERPT_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct TextSource {
    pub relative_path: String,
    pub addon_guid: Option<String>,
    pub addon_label: Option<String>,
    pub source_uri: Option<String>,
    pub content: Arc<str>,
}

#[derive(Debug, Clone, Default)]
pub struct TextSearchCorpus {
    pub sources: Vec<TextSource>,
    pub files_considered: usize,
    pub source_read_ms: u64,
    pub source_read_failures: usize,
    pub source_read_failures_by_addon: BTreeMap<String, usize>,
    pub source_read_ms_by_addon: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSearchRequest {
    pub query: String,
    pub addon_guids: Option<Vec<String>>,
    pub options: TextSearchOptions,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSearchOptions {
    pub match_case: bool,
    pub match_whole_word: bool,
    pub use_regex: bool,
}

#[derive(Debug, Clone)]
pub struct TextSearchResultSet {
    catalogue_revision: String,
    query: String,
    options: TextSearchOptions,
    addon_guids: Vec<String>,
    results: Vec<TextSearchHit>,
    truncated: bool,
    stats: TextSearchStats,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextSearchPage {
    pub catalogue_revision: String,
    pub query: String,
    pub addon_guids: Vec<String>,
    pub returned: usize,
    pub total: usize,
    pub totals_by_addon: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub truncated: bool,
    pub stats: TextSearchStats,
    pub results: Vec<TextSearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextSearchStats {
    pub files_considered: usize,
    pub files_read: usize,
    pub files_with_matches: usize,
    pub source_read_ms: u64,
    pub source_read_failures: usize,
    pub source_read_failures_by_addon: BTreeMap<String, usize>,
    pub source_read_ms_by_addon: BTreeMap<String, u64>,
    pub matches_found: usize,
    pub scan_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextSearchHit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,
    pub relative_path: String,
    pub match_range: TextRange,
    pub excerpt_match_start: usize,
    pub excerpt: String,
    pub match_text: String,
    pub read_source_input: TextReadInput,
}

pub(crate) fn physical_source_uri(path: &Path) -> Option<String> {
    Url::from_file_path(path).ok().map(|uri| uri.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextRange {
    pub start_line: usize,
    pub start_character: usize,
    pub end_line: usize,
    pub end_character: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextReadInput {
    pub catalogue_revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon_guid: Option<String>,
    pub relative_path: String,
    pub start_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSearchError {
    InvalidRequest(&'static str),
    InvalidPattern(String),
    InvalidCursor,
    StaleCursor,
    Cancelled,
}

impl fmt::Display for TextSearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(f, "{message}"),
            Self::InvalidPattern(message) => f.write_str(message),
            Self::InvalidCursor => write!(f, "invalid cursor"),
            Self::StaleCursor => write!(f, "stale cursor"),
            Self::Cancelled => write!(f, "search cancelled"),
        }
    }
}

pub fn search(
    mut corpus: TextSearchCorpus,
    control: &IndexBuildControl,
    catalogue_revision: &str,
    request: TextSearchRequest,
) -> Result<TextSearchPage, TextSearchError> {
    let result_set = scan(&mut corpus, control, catalogue_revision, &request)?;
    page(&result_set, control, request)
}

pub fn scan(
    corpus: &mut TextSearchCorpus,
    control: &IndexBuildControl,
    catalogue_revision: &str,
    request: &TextSearchRequest,
) -> Result<TextSearchResultSet, TextSearchError> {
    let query = normalize_query(&request.query)?;
    let matcher = compile_matcher(&query, request.options)?;
    let started = Instant::now();
    corpus
        .sources
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let files_considered = corpus.files_considered.max(corpus.sources.len());
    let mut hits = Vec::new();
    let mut files_read: usize = 0;
    let mut files_with_matches: usize = 0;
    let mut matches_found: usize = 0;
    let mut truncated = false;
    'sources: for source in &corpus.sources {
        control.check().map_err(|_| TextSearchError::Cancelled)?;
        files_read += 1;
        let mut starts = None;
        let mut source_has_matches = false;
        for matched in matcher.regex.find_iter(&source.content) {
            control.check().map_err(|_| TextSearchError::Cancelled)?;
            if matched.is_empty()
                || (request.options.match_whole_word
                    && !is_whole_word_match(&source.content, matched.start(), matched.end()))
            {
                continue;
            }
            matches_found = matches_found.saturating_add(1);
            if !source_has_matches {
                files_with_matches += 1;
                source_has_matches = true;
            }
            if hits.len() == crate::search_limits::MAX_SEARCH_RESULTS {
                truncated = true;
                break 'sources;
            }
            let starts = starts.get_or_insert_with(|| line_starts(&source.content));
            hits.push(project_hit(
                source,
                starts,
                catalogue_revision,
                matched.start(),
                matched.end(),
            ));
        }
    }
    Ok(TextSearchResultSet {
        catalogue_revision: catalogue_revision.to_string(),
        query,
        options: request.options,
        addon_guids: request.addon_guids.clone().unwrap_or_default(),
        results: hits,
        truncated,
        stats: TextSearchStats {
            files_considered,
            files_read,
            files_with_matches,
            source_read_ms: corpus.source_read_ms,
            source_read_failures: corpus.source_read_failures,
            source_read_failures_by_addon: corpus.source_read_failures_by_addon.clone(),
            source_read_ms_by_addon: corpus.source_read_ms_by_addon.clone(),
            matches_found,
            scan_ms: started.elapsed().as_millis() as u64,
        },
    })
}

pub fn page(
    result_set: &TextSearchResultSet,
    control: &IndexBuildControl,
    request: TextSearchRequest,
) -> Result<TextSearchPage, TextSearchError> {
    control.check().map_err(|_| TextSearchError::Cancelled)?;
    let query = normalize_query(&request.query)?;
    let addon_guids = request.addon_guids.clone().unwrap_or_default();
    if query != result_set.query
        || request.options != result_set.options
        || addon_guids != result_set.addon_guids
    {
        return Err(TextSearchError::InvalidCursor);
    }
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let cursor = request.cursor.as_deref().map(decode_cursor).transpose()?;
    if let Some(cursor) = &cursor {
        if cursor.catalogue_revision != result_set.catalogue_revision {
            return Err(TextSearchError::StaleCursor);
        }
        if cursor.query != query
            || cursor.options != request.options
            || cursor.addon_guids != addon_guids
        {
            return Err(TextSearchError::InvalidCursor);
        }
    }
    let offset = cursor.map(|cursor| cursor.offset).unwrap_or(0);
    let total = result_set.results.len();
    let page_hits = result_set
        .results
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let returned = page_hits.len();
    let mut totals_by_addon = BTreeMap::new();
    for hit in &result_set.results {
        if let Some(guid) = &hit.addon_guid {
            *totals_by_addon.entry(guid.clone()).or_insert(0) += 1;
        }
    }
    let next_cursor = (offset + returned < total).then(|| {
        encode_cursor(&Cursor {
            version: 3,
            catalogue_revision: result_set.catalogue_revision.clone(),
            query: query.clone(),
            options: request.options,
            addon_guids: addon_guids.clone(),
            offset: offset + returned,
        })
    });
    let mut page = TextSearchPage {
        catalogue_revision: result_set.catalogue_revision.clone(),
        query,
        addon_guids: addon_guids.clone(),
        returned,
        total,
        totals_by_addon,
        next_cursor,
        truncated: result_set.truncated,
        stats: result_set.stats.clone(),
        results: page_hits,
    };
    while serde_json::to_vec(&page)
        .map_err(|_| TextSearchError::InvalidRequest("text search result could not serialize"))?
        .len()
        > MAX_RESULT_BYTES
        && page.results.len() > 1
    {
        page.results.pop();
        page.returned = page.results.len();
        page.next_cursor = Some(encode_cursor(&Cursor {
            version: 3,
            catalogue_revision: page.catalogue_revision.clone(),
            query: page.query.clone(),
            options: request.options,
            addon_guids: addon_guids.clone(),
            offset: offset + page.returned,
        }));
    }
    Ok(page)
}

fn normalize_query(query: &str) -> Result<String, TextSearchError> {
    if query.is_empty() {
        return Err(TextSearchError::InvalidRequest("query must be non-empty"));
    }
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(TextSearchError::InvalidRequest(
            "query exceeds 256 characters",
        ));
    }
    Ok(query.to_string())
}

struct CompiledMatcher {
    regex: Regex,
}

fn compile_matcher(
    query: &str,
    options: TextSearchOptions,
) -> Result<CompiledMatcher, TextSearchError> {
    let pattern = if options.use_regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    let regex = RegexBuilder::new(&pattern)
        .case_insensitive(!options.match_case)
        .size_limit(4 * 1024 * 1024)
        .build()
        .map_err(|error| {
            TextSearchError::InvalidPattern(format!("regular expression is invalid: {error}"))
        })?;
    Ok(CompiledMatcher { regex })
}

fn is_whole_word_match(source: &str, start: usize, end: usize) -> bool {
    let before = source[..start].chars().next_back();
    let after = source[end..].chars().next();
    before.is_none_or(|character| !is_word_character(character))
        && after.is_none_or(|character| !is_word_character(character))
}

fn is_word_character(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        source
            .bytes()
            .enumerate()
            .filter_map(|(offset, byte)| (byte == b'\n').then_some(offset + 1)),
    );
    starts
}

fn project_hit(
    source: &TextSource,
    starts: &[usize],
    catalogue_revision: &str,
    start: usize,
    end: usize,
) -> TextSearchHit {
    let start_line_index = line_for_offset(starts, start);
    let end_line_index = line_for_offset(starts, end.saturating_sub(1));
    let start_line_offset = starts[start_line_index];
    let end_line_offset = starts[end_line_index];
    let start_character = source.content[start_line_offset..start].chars().count();
    let end_character = source.content[end_line_offset..end].chars().count();
    let line_end_offset = source.content[start_line_offset..]
        .find('\n')
        .map(|offset| start_line_offset + offset)
        .unwrap_or(source.content.len());
    let (excerpt_start, excerpt_end) = bounded_excerpt_range(
        source.content.as_ref(),
        start_line_offset,
        line_end_offset,
        start,
        end,
    );
    let excerpt = source.content[excerpt_start..excerpt_end]
        .trim_end_matches('\r')
        .to_string();
    TextSearchHit {
        addon_guid: source.addon_guid.clone(),
        addon_label: source.addon_label.clone(),
        source_uri: source.source_uri.clone(),
        relative_path: source.relative_path.clone(),
        match_range: TextRange {
            start_line: start_line_index + 1,
            start_character,
            end_line: end_line_index + 1,
            end_character,
        },
        excerpt_match_start: source.content[excerpt_start..start].encode_utf16().count(),
        excerpt,
        match_text: source.content[start..end].to_string(),
        read_source_input: TextReadInput {
            catalogue_revision: catalogue_revision.to_string(),
            addon_guid: source.addon_guid.clone(),
            relative_path: source.relative_path.clone(),
            start_line: start_line_index + 1,
        },
    }
}

fn bounded_excerpt_range(
    source: &str,
    line_start: usize,
    line_end: usize,
    match_start: usize,
    match_end: usize,
) -> (usize, usize) {
    if line_end.saturating_sub(line_start) <= MAX_EXCERPT_BYTES {
        return (line_start, line_end);
    }
    let max_start = line_end.saturating_sub(MAX_EXCERPT_BYTES);
    let mut excerpt_start = match_start
        .saturating_sub(MAX_EXCERPT_BYTES / 2)
        .clamp(line_start, max_start);
    while excerpt_start > line_start && !source.is_char_boundary(excerpt_start) {
        excerpt_start -= 1;
    }
    let mut excerpt_end = (excerpt_start + MAX_EXCERPT_BYTES).min(line_end);
    while excerpt_end < line_end && !source.is_char_boundary(excerpt_end) {
        excerpt_end += 1;
    }
    if excerpt_end < match_end {
        excerpt_end = match_end;
    }
    (excerpt_start, excerpt_end.min(line_end))
}

fn line_for_offset(starts: &[usize], offset: usize) -> usize {
    starts
        .partition_point(|start| *start <= offset)
        .saturating_sub(1)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Cursor {
    version: u8,
    catalogue_revision: String,
    query: String,
    options: TextSearchOptions,
    addon_guids: Vec<String>,
    offset: usize,
}

fn encode_cursor(cursor: &Cursor) -> String {
    hex(&serde_json::to_vec(cursor).expect("text cursor serializes"))
}

fn decode_cursor(value: &str) -> Result<Cursor, TextSearchError> {
    if value.len() > MAX_CURSOR_BYTES {
        return Err(TextSearchError::InvalidCursor);
    }
    let bytes = unhex(value).ok_or(TextSearchError::InvalidCursor)?;
    let cursor =
        serde_json::from_slice::<Cursor>(&bytes).map_err(|_| TextSearchError::InvalidCursor)?;
    (cursor.version == 3)
        .then_some(cursor)
        .ok_or(TextSearchError::InvalidCursor)
}

fn hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut value, "{byte:02x}").expect("text cursor write");
    }
    value
}

fn unhex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&value[offset..offset + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> TextSearchCorpus {
        TextSearchCorpus {
            files_considered: 2,
            source_read_ms: 42,
            source_read_failures: 0,
            sources: vec![
                TextSource {
                    relative_path: "Game/Z.c".to_string(),
                    addon_guid: None,
                    addon_label: None,
                    source_uri: None,
                    content: Arc::from("// SCR_ in a comment\nvoid Z() { string s = \"SCR_\"; }\n"),
                },
                TextSource {
                    relative_path: "Game/A.c".to_string(),
                    addon_guid: None,
                    addon_label: None,
                    source_uri: None,
                    content: Arc::from("😀 void A() { SCR_(); }\n"),
                },
            ],
            ..TextSearchCorpus::default()
        }
    }

    #[test]
    fn scans_literal_matches_in_comments_and_strings_in_deterministic_source_order() {
        let page = search(
            corpus(),
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: "SCR_".to_string(),
                addon_guids: None,
                options: TextSearchOptions::default(),
                limit: Some(10),
                cursor: None,
            },
        )
        .expect("text search");

        assert_eq!(page.total, 3);
        assert_eq!(page.results[0].relative_path, "Game/A.c");
        assert_eq!(page.results[0].match_range.start_line, 1);
        assert_eq!(page.results[0].match_range.start_character, 13);
        assert_eq!(page.results[0].match_text, "SCR_");
        assert_eq!(page.results[0].excerpt_match_start, 14);
        assert_eq!(page.results[1].relative_path, "Game/Z.c");
        assert_eq!(page.results[1].match_range.start_line, 1);
        assert_eq!(page.results[2].match_range.start_line, 2);
        assert_eq!(page.stats.files_read, 2);
        assert_eq!(page.stats.source_read_ms, 42);
        assert_eq!(page.stats.files_with_matches, 2);
        assert_eq!(page.stats.matches_found, 3);
        assert_eq!(
            page.results[0].read_source_input.catalogue_revision,
            "ws1:test"
        );
    }

    #[test]
    fn literal_search_ignores_case_unless_match_case_is_enabled() {
        let insensitive = search(
            corpus(),
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: "scr_".to_string(),
                addon_guids: None,
                options: TextSearchOptions::default(),
                limit: Some(10),
                cursor: None,
            },
        )
        .expect("case-insensitive text search");
        assert_eq!(insensitive.total, 3);
        assert_eq!(insensitive.results[0].match_text, "SCR_");

        let sensitive = search(
            corpus(),
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: "scr_".to_string(),
                addon_guids: None,
                options: TextSearchOptions {
                    match_case: true,
                    ..TextSearchOptions::default()
                },
                limit: Some(10),
                cursor: None,
            },
        )
        .expect("case-sensitive text search");
        assert_eq!(sensitive.total, 0);
    }

    #[test]
    fn repeated_matches_retain_their_exact_excerpt_offsets() {
        let page = search(
            TextSearchCorpus {
                files_considered: 1,
                sources: vec![TextSource {
                    relative_path: "Game/Repeated.c".to_string(),
                    addon_guid: None,
                    addon_label: None,
                    source_uri: None,
                    content: Arc::from("SCR_ SCR_"),
                }],
                ..TextSearchCorpus::default()
            },
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: "SCR_".to_string(),
                addon_guids: None,
                options: TextSearchOptions::default(),
                limit: Some(10),
                cursor: None,
            },
        )
        .expect("repeated text search");

        assert_eq!(page.results[0].excerpt_match_start, 0);
        assert_eq!(page.results[1].excerpt_match_start, 5);
    }

    #[test]
    fn physical_text_sources_publish_file_editor_identities() {
        let path = std::env::temp_dir().join("TextSource.c");
        let uri = physical_source_uri(&path).expect("file URI");

        assert_eq!(uri, Url::from_file_path(path).expect("file URI").as_str());
    }

    #[test]
    fn whole_word_and_regular_expression_options_constrain_matches() {
        let corpus = TextSearchCorpus {
            files_considered: 1,
            source_read_failures: 0,
            sources: vec![TextSource {
                relative_path: "Game/Words.c".to_string(),
                addon_guid: None,
                addon_label: None,
                source_uri: None,
                content: Arc::from("SCR SCR_Player scr\nSCR_One SCR_Two other"),
            }],
            ..TextSearchCorpus::default()
        };
        let whole_word = search(
            corpus.clone(),
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: "scr".to_string(),
                addon_guids: None,
                options: TextSearchOptions {
                    match_whole_word: true,
                    ..TextSearchOptions::default()
                },
                limit: Some(10),
                cursor: None,
            },
        )
        .expect("whole-word text search");
        assert_eq!(whole_word.total, 2);

        let regex = search(
            corpus,
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: r"SCR_(One|Two)".to_string(),
                addon_guids: None,
                options: TextSearchOptions {
                    use_regex: true,
                    ..TextSearchOptions::default()
                },
                limit: Some(10),
                cursor: None,
            },
        )
        .expect("regular-expression text search");
        assert_eq!(regex.total, 2);
        assert_eq!(regex.results[0].match_text, "SCR_One");
        assert_eq!(regex.results[1].match_text, "SCR_Two");
    }

    #[test]
    fn invalid_regular_expression_is_a_stable_request_error() {
        let error = search(
            corpus(),
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: "(".to_string(),
                addon_guids: None,
                options: TextSearchOptions {
                    use_regex: true,
                    ..TextSearchOptions::default()
                },
                limit: Some(10),
                cursor: None,
            },
        )
        .expect_err("invalid regular expression");
        assert!(matches!(error, TextSearchError::InvalidPattern(_)));
    }

    #[test]
    fn cursor_is_opaque_revision_bound_and_pages_literal_matches() {
        let first = search(
            corpus(),
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: "SCR_".to_string(),
                addon_guids: None,
                options: TextSearchOptions::default(),
                limit: Some(1),
                cursor: None,
            },
        )
        .expect("first page");
        let cursor = first.next_cursor.clone().expect("continuation");
        assert!(!cursor.contains("Game/A.c"));

        let second = search(
            corpus(),
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: "SCR_".to_string(),
                addon_guids: None,
                options: TextSearchOptions::default(),
                limit: Some(1),
                cursor: Some(cursor),
            },
        )
        .expect("second page");
        assert_eq!(second.results[0].relative_path, "Game/Z.c");
        assert_eq!(second.results[0].match_range.start_line, 1);

        assert!(matches!(
            search(
                corpus(),
                &IndexBuildControl::default(),
                "ws1:test",
                TextSearchRequest {
                    query: "SCR_".to_string(),
                    addon_guids: None,
                    options: TextSearchOptions {
                        match_case: true,
                        ..TextSearchOptions::default()
                    },
                    limit: Some(1),
                    cursor: first.next_cursor.clone(),
                },
            ),
            Err(TextSearchError::InvalidCursor)
        ));

        assert!(matches!(
            search(
                corpus(),
                &IndexBuildControl::default(),
                "ws1:test",
                TextSearchRequest {
                    query: "SCR_".to_string(),
                    addon_guids: Some(vec!["AAAAAAAAAAAAAAAA".to_string()]),
                    options: TextSearchOptions::default(),
                    limit: Some(1),
                    cursor: first.next_cursor.clone(),
                },
            ),
            Err(TextSearchError::InvalidCursor)
        ));

        assert_eq!(
            search(
                corpus(),
                &IndexBuildControl::default(),
                "ws1:other",
                TextSearchRequest {
                    query: "SCR_".to_string(),
                    addon_guids: None,
                    options: TextSearchOptions::default(),
                    limit: Some(1),
                    cursor: first.next_cursor,
                },
            ),
            Err(TextSearchError::StaleCursor)
        );
    }

    #[test]
    fn cancelled_scan_stops_before_returning_partial_text_evidence() {
        let control = IndexBuildControl::default();
        control.cancel();
        assert_eq!(
            search(
                corpus(),
                &control,
                "ws1:test",
                TextSearchRequest {
                    query: "SCR_".to_string(),
                    addon_guids: None,
                    options: TextSearchOptions::default(),
                    limit: Some(10),
                    cursor: None,
                },
            ),
            Err(TextSearchError::Cancelled)
        );
    }

    #[test]
    fn bounds_large_line_excerpts_while_retaining_the_literal_match() {
        let mut content = "x".repeat(MAX_EXCERPT_BYTES * 2);
        content.push_str("SCR_");
        let page = search(
            TextSearchCorpus {
                files_considered: 1,
                source_read_failures: 0,
                sources: vec![TextSource {
                    relative_path: "Game/Large.c".to_string(),
                    addon_guid: None,
                    addon_label: None,
                    source_uri: None,
                    content: Arc::from(content),
                }],
                ..TextSearchCorpus::default()
            },
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: "SCR_".to_string(),
                addon_guids: None,
                options: TextSearchOptions::default(),
                limit: Some(1),
                cursor: None,
            },
        )
        .expect("large-line text search");

        assert!(page.results[0].excerpt.len() <= MAX_EXCERPT_BYTES + 3);
        assert!(page.results[0].excerpt.contains("SCR_"));
    }
}
