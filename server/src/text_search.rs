use crate::index_build::IndexBuildControl;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::time::Instant;

pub const DEFAULT_LIMIT: usize = 20;
pub const MAX_LIMIT: usize = 100;
pub const MAX_QUERY_CHARS: usize = 256;
pub const MAX_CURSOR_BYTES: usize = 2048;
const MAX_TEXT_MATCHES: usize = 100_000;
const MAX_RESULT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone)]
pub struct TextSource {
    pub relative_path: String,
    pub content: Arc<str>,
}

#[derive(Debug, Clone, Default)]
pub struct TextSearchCorpus {
    pub sources: Vec<TextSource>,
    pub files_considered: usize,
    pub source_read_failures: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TextSearchResultSet {
    catalogue_revision: String,
    query: String,
    results: Vec<TextSearchHit>,
    truncated: bool,
    stats: TextSearchStats,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextSearchPage {
    pub catalogue_revision: String,
    pub query: String,
    pub returned: usize,
    pub total: usize,
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
    pub source_read_failures: usize,
    pub matches_found: usize,
    pub scan_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextSearchHit {
    pub relative_path: String,
    pub match_range: TextRange,
    pub excerpt: String,
    pub match_text: String,
    pub read_source_input: TextReadInput,
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
    pub relative_path: String,
    pub start_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSearchError {
    InvalidRequest(&'static str),
    InvalidCursor,
    StaleCursor,
    Cancelled,
}

impl fmt::Display for TextSearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(f, "{message}"),
            Self::InvalidCursor => write!(f, "invalid cursor"),
            Self::StaleCursor => write!(f, "stale cursor"),
            Self::Cancelled => write!(f, "search cancelled"),
        }
    }
}

pub fn search(
    corpus: TextSearchCorpus,
    control: &IndexBuildControl,
    catalogue_revision: &str,
    request: TextSearchRequest,
) -> Result<TextSearchPage, TextSearchError> {
    let result_set = scan(corpus, control, catalogue_revision, &request.query)?;
    page(&result_set, control, request)
}

pub fn scan(
    mut corpus: TextSearchCorpus,
    control: &IndexBuildControl,
    catalogue_revision: &str,
    query: &str,
) -> Result<TextSearchResultSet, TextSearchError> {
    let query = normalize_query(query)?;
    let started = Instant::now();
    corpus
        .sources
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let files_considered = corpus.files_considered.max(corpus.sources.len());
    let mut hits = Vec::new();
    let mut files_read: usize = 0;
    let mut files_with_matches: usize = 0;
    let mut matches_found: usize = 0;
    for source in &corpus.sources {
        control.check().map_err(|_| TextSearchError::Cancelled)?;
        files_read += 1;
        let starts = line_starts(&source.content);
        let mut source_match_count = 0;
        for (start, _) in source.content.match_indices(&query) {
            control.check().map_err(|_| TextSearchError::Cancelled)?;
            matches_found = matches_found.saturating_add(1);
            source_match_count += 1;
            if hits.len() < MAX_TEXT_MATCHES {
                hits.push(project_hit(
                    source,
                    &starts,
                    catalogue_revision,
                    &query,
                    start,
                ));
            }
        }
        if source_match_count > 0 {
            files_with_matches += 1;
        }
    }
    let total = hits.len();
    let truncated = matches_found > total;
    Ok(TextSearchResultSet {
        catalogue_revision: catalogue_revision.to_string(),
        query,
        results: hits,
        truncated,
        stats: TextSearchStats {
            files_considered,
            files_read,
            files_with_matches,
            source_read_failures: corpus.source_read_failures,
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
    if query != result_set.query {
        return Err(TextSearchError::InvalidCursor);
    }
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let cursor = request.cursor.as_deref().map(decode_cursor).transpose()?;
    if let Some(cursor) = &cursor {
        if cursor.catalogue_revision != result_set.catalogue_revision {
            return Err(TextSearchError::StaleCursor);
        }
        if cursor.query != query {
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
    let next_cursor = (offset + returned < total).then(|| {
        encode_cursor(&Cursor {
            version: 1,
            catalogue_revision: result_set.catalogue_revision.clone(),
            query: query.clone(),
            offset: offset + returned,
        })
    });
    let mut page = TextSearchPage {
        catalogue_revision: result_set.catalogue_revision.clone(),
        query,
        returned,
        total,
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
            version: 1,
            catalogue_revision: page.catalogue_revision.clone(),
            query: page.query.clone(),
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
    query: &str,
    start: usize,
) -> TextSearchHit {
    let end = start + query.len();
    let start_line_index = line_for_offset(starts, start);
    let end_line_index = line_for_offset(starts, end.saturating_sub(1));
    let start_line_offset = starts[start_line_index];
    let end_line_offset = starts[end_line_index];
    let start_character = source.content[start_line_offset..start].chars().count();
    let end_character = source.content[end_line_offset..end].chars().count();
    let excerpt_start = start_line_offset;
    let excerpt_end = source.content[start_line_offset..]
        .find('\n')
        .map(|offset| start_line_offset + offset)
        .unwrap_or(source.content.len());
    let excerpt = source.content[excerpt_start..excerpt_end]
        .trim_end_matches('\r')
        .to_string();
    TextSearchHit {
        relative_path: source.relative_path.clone(),
        match_range: TextRange {
            start_line: start_line_index + 1,
            start_character,
            end_line: end_line_index + 1,
            end_character,
        },
        excerpt,
        match_text: query.to_string(),
        read_source_input: TextReadInput {
            catalogue_revision: catalogue_revision.to_string(),
            relative_path: source.relative_path.clone(),
            start_line: start_line_index + 1,
        },
    }
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
    (cursor.version == 1)
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
            source_read_failures: 0,
            sources: vec![
                TextSource {
                    relative_path: "Game/Z.c".to_string(),
                    content: Arc::from("// SCR_ in a comment\nvoid Z() { string s = \"SCR_\"; }\n"),
                },
                TextSource {
                    relative_path: "Game/A.c".to_string(),
                    content: Arc::from("void A() { SCR_(); }\n"),
                },
            ],
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
                limit: Some(10),
                cursor: None,
            },
        )
        .expect("text search");

        assert_eq!(page.total, 3);
        assert_eq!(page.results[0].relative_path, "Game/A.c");
        assert_eq!(page.results[0].match_range.start_line, 1);
        assert_eq!(page.results[0].match_range.start_character, 11);
        assert_eq!(page.results[0].match_text, "SCR_");
        assert_eq!(page.results[1].relative_path, "Game/Z.c");
        assert_eq!(page.results[1].match_range.start_line, 1);
        assert_eq!(page.results[2].match_range.start_line, 2);
        assert_eq!(page.stats.files_read, 2);
        assert_eq!(page.stats.files_with_matches, 2);
        assert_eq!(page.stats.matches_found, 3);
        assert_eq!(
            page.results[0].read_source_input.catalogue_revision,
            "ws1:test"
        );
    }

    #[test]
    fn cursor_is_opaque_revision_bound_and_pages_literal_matches() {
        let first = search(
            corpus(),
            &IndexBuildControl::default(),
            "ws1:test",
            TextSearchRequest {
                query: "SCR_".to_string(),
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
                limit: Some(1),
                cursor: Some(cursor),
            },
        )
        .expect("second page");
        assert_eq!(second.results[0].relative_path, "Game/Z.c");
        assert_eq!(second.results[0].match_range.start_line, 1);

        assert_eq!(
            search(
                corpus(),
                &IndexBuildControl::default(),
                "ws1:other",
                TextSearchRequest {
                    query: "SCR_".to_string(),
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
                    limit: Some(10),
                    cursor: None,
                },
            ),
            Err(TextSearchError::Cancelled)
        );
    }
}
