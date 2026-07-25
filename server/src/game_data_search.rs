use crate::index::{GlobalSymbolId, IndexedSymbol, SourceFileId, SymbolIndex};
use crate::index_build::IndexBuildControl;
use crate::model::SymbolKind;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const DEFAULT_LIMIT: usize = 20;
pub const MAX_LIMIT: usize = 100;
pub const MAX_QUERY_CHARS: usize = 256;
pub const MAX_CURSOR_BYTES: usize = 2048;
pub const MAX_OWNER_CHARS: usize = 256;
pub const MAX_SEARCH_RESULT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Default)]
pub struct SourceLineStarts(Vec<usize>);

impl SourceLineStarts {
    pub fn from_source(source: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(
            source
                .bytes()
                .enumerate()
                .filter_map(|(offset, byte)| (byte == b'\n').then_some(offset + 1)),
        );
        Self(starts)
    }

    fn range(&self, start: usize, end: usize) -> SourceLineRange {
        SourceLineRange {
            start_line: line_for_offset(&self.0, start),
            end_line: line_for_offset(&self.0, end),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDataSearchRequest {
    pub query: String,
    pub kinds: Option<Vec<String>>,
    pub owner: Option<String>,
    pub source_categories: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

impl GameDataSearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            kinds: None,
            owner: None,
            source_categories: None,
            limit: None,
            cursor: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameDataSearchPage {
    pub catalogue_revision: String,
    pub query: String,
    pub applied_filters: AppliedFilters,
    pub returned: usize,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub results: Vec<GameDataSearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppliedFilters {
    pub kinds: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub source_categories: Vec<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameDataSearchHit {
    pub symbol_ref: String,
    pub name: String,
    pub kind: String,
    pub qualified_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_summary: Option<String>,
    pub source_category: String,
    pub relative_path: String,
    pub declaration_range: SourceLineRange,
    pub selection_range: SourceLineRange,
    pub match_kind: String,
    pub inspect_input: InspectInput,
    pub read_source_input: ReadSourceInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceLineRange {
    pub start_line: usize,
    pub end_line: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InspectInput {
    pub symbol_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadSourceInput {
    pub catalogue_revision: String,
    pub relative_path: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameDataSearchError {
    InvalidRequest(&'static str),
    InvalidCursor,
    StaleCursor,
    Cancelled,
}
impl fmt::Display for GameDataSearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(f, "{message}"),
            Self::InvalidCursor => write!(f, "invalid cursor"),
            Self::StaleCursor => write!(f, "stale cursor"),
            Self::Cancelled => write!(f, "search cancelled"),
        }
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    id: GlobalSymbolId,
    rank: u8,
    match_kind: &'static str,
    qualified_name: String,
    kind: String,
    path: String,
    position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Cursor {
    version: u8,
    catalogue_revision: String,
    query: String,
    kinds: Vec<String>,
    owner: Option<String>,
    source_categories: Vec<String>,
    offset: usize,
}

pub fn search(
    index: &SymbolIndex,
    source_line_starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    control: &IndexBuildControl,
    catalogue_revision: &str,
    request: GameDataSearchRequest,
) -> Result<GameDataSearchPage, GameDataSearchError> {
    let query = normalize_query(&request.query)?;
    let kinds = canonical_kinds(request.kinds.as_deref())?;
    let source_categories = canonical_categories(request.source_categories.as_deref())?;
    let owner = request
        .owner
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if request.owner.is_some() && owner.is_none() {
        return Err(GameDataSearchError::InvalidRequest(
            "owner must be non-empty",
        ));
    }
    if owner.as_ref().is_some_and(|value| value.chars().count() > MAX_OWNER_CHARS) {
        return Err(GameDataSearchError::InvalidRequest("owner exceeds 256 characters"));
    }
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let cursor = request.cursor.as_deref().map(decode_cursor).transpose()?;
    if let Some(cursor) = &cursor {
        if cursor.catalogue_revision != catalogue_revision {
            return Err(GameDataSearchError::StaleCursor);
        }
        if cursor.query != query
            || cursor.kinds != kinds
            || cursor.owner != owner
            || cursor.source_categories != source_categories
        {
            return Err(GameDataSearchError::InvalidCursor);
        }
    }
    let offset = cursor.map(|cursor| cursor.offset).unwrap_or(0);
    let query_folded = query.to_lowercase();
    let mut candidates = Vec::new();
    for symbol in index.symbols() {
        control
            .check()
            .map_err(|_| GameDataSearchError::Cancelled)?;
        let Some(file) = index.file(symbol.id.file_id) else {
            continue;
        };
        let Some(name) = symbol.name.as_deref() else {
            continue;
        };
        let kind = kind_name(symbol.kind).to_string();
        if kinds.binary_search(&kind).is_err()
            || source_categories
                .binary_search(&file.metadata.category.as_str().to_string())
                .is_err()
        {
            continue;
        }
        let symbol_owner = owner_name(index, symbol);
        if owner
            .as_deref()
            .is_some_and(|expected| symbol_owner.as_deref() != Some(expected))
        {
            continue;
        }
        let qualified_name = qualify(symbol_owner.as_deref(), name);
        let signature = index.callable_signature(symbol.id);
        let Some((rank, match_kind)) = match_rank(
            symbol,
            &qualified_name,
            signature.as_deref(),
            &query,
            &query_folded,
        ) else {
            continue;
        };
        candidates.push(Candidate {
            id: symbol.id,
            rank,
            match_kind,
            qualified_name,
            kind,
            path: logical_path(file),
            position: symbol.span.start,
        });
    }
    control
        .check()
        .map_err(|_| GameDataSearchError::Cancelled)?;
    candidates.sort_by(|left, right| {
        (
            left.rank,
            &left.qualified_name,
            &left.kind,
            &left.path,
            left.position,
        )
            .cmp(&(
                right.rank,
                &right.qualified_name,
                &right.kind,
                &right.path,
                right.position,
            ))
    });
    let total = candidates.len();
    let page_candidates = candidates
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let returned = page_candidates.len();
    let next_cursor = (offset + returned < total).then(|| {
        encode_cursor(&Cursor {
            version: 1,
            catalogue_revision: catalogue_revision.to_string(),
            query: query.clone(),
            kinds: kinds.clone(),
            owner: owner.clone(),
            source_categories: source_categories.clone(),
            offset: offset + returned,
        })
    });
    let results = page_candidates
        .into_iter()
        .map(|candidate| -> Result<_, GameDataSearchError> {
            control
                .check()
                .map_err(|_| GameDataSearchError::Cancelled)?;
            Ok(project_hit(
                index,
                source_line_starts,
                catalogue_revision,
                candidate,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut page = GameDataSearchPage {
        catalogue_revision: catalogue_revision.to_string(),
        query,
        applied_filters: AppliedFilters {
            kinds,
            owner,
            source_categories,
            limit,
        },
        returned,
        total,
        next_cursor,
        results,
    };
    while serde_json::to_vec(&page)
        .map_err(|_| GameDataSearchError::InvalidRequest("search result could not serialize"))?
        .len()
        > MAX_SEARCH_RESULT_BYTES
        && page.results.len() > 1
    {
        page.results.pop();
        page.returned = page.results.len();
        page.next_cursor = Some(encode_cursor(&Cursor {
            version: 1,
            catalogue_revision: page.catalogue_revision.clone(),
            query: page.query.clone(),
            kinds: page.applied_filters.kinds.clone(),
            owner: page.applied_filters.owner.clone(),
            source_categories: page.applied_filters.source_categories.clone(),
            offset: offset + page.returned,
        }));
    }
    Ok(page)
}

fn normalize_query(query: &str) -> Result<String, GameDataSearchError> {
    let normalized = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Err(GameDataSearchError::InvalidRequest(
            "query must be non-empty",
        ));
    }
    if normalized.chars().count() > MAX_QUERY_CHARS {
        return Err(GameDataSearchError::InvalidRequest(
            "query exceeds 256 characters",
        ));
    }
    Ok(normalized)
}
fn canonical_kinds(values: Option<&[String]>) -> Result<Vec<String>, GameDataSearchError> {
    let values = match values {
        Some(values) if values.is_empty() => {
            return Err(GameDataSearchError::InvalidRequest(
                "kinds must be non-empty",
            ))
        }
        Some(values) => values.to_vec(),
        None => default_kinds().into_iter().map(str::to_string).collect(),
    };
    let allowed = all_kinds();
    let value_count = values.len();
    let unique = values.into_iter().collect::<BTreeSet<_>>();
    if unique.len() != value_count
        || unique
            .iter()
            .any(|value| !allowed.contains(&value.as_str()))
    {
        return Err(GameDataSearchError::InvalidRequest(
            "kinds must be unique supported symbol kinds",
        ));
    }
    Ok(unique.into_iter().collect())
}
fn canonical_categories(values: Option<&[String]>) -> Result<Vec<String>, GameDataSearchError> {
    let values = match values {
        Some(values) if values.is_empty() => {
            return Err(GameDataSearchError::InvalidRequest(
                "sourceCategories must be non-empty",
            ))
        }
        Some(values) => values.to_vec(),
        None => all_categories().into_iter().map(str::to_string).collect(),
    };
    let allowed = all_categories();
    let value_count = values.len();
    let unique = values.into_iter().collect::<BTreeSet<_>>();
    if unique.len() != value_count
        || unique
            .iter()
            .any(|value| !allowed.contains(&value.as_str()) || value == "workspace")
    {
        return Err(GameDataSearchError::InvalidRequest(
            "sourceCategories must be unique game-data categories",
        ));
    }
    Ok(unique.into_iter().collect())
}
fn default_kinds() -> Vec<&'static str> {
    vec![
        "class",
        "constructor",
        "destructor",
        "enum",
        "enumMember",
        "field",
        "function",
        "globalField",
        "method",
        "preprocessorMacro",
        "typedef",
    ]
}
fn all_kinds() -> Vec<&'static str> {
    vec![
        "class",
        "constructor",
        "destructor",
        "enum",
        "enumMember",
        "field",
        "function",
        "globalField",
        "localVariable",
        "method",
        "parameter",
        "preprocessorMacro",
        "typeParameter",
        "typedef",
    ]
}
fn all_categories() -> Vec<&'static str> {
    vec![
        "core",
        "docs/doxygen",
        "game",
        "gamecode",
        "gamelib",
        "generated",
        "test/autotest",
        "unknown",
        "workbench",
    ]
}
fn kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "class",
        SymbolKind::TypeParameter => "typeParameter",
        SymbolKind::Enum => "enum",
        SymbolKind::EnumMember => "enumMember",
        SymbolKind::Typedef => "typedef",
        SymbolKind::Function => "function",
        SymbolKind::GlobalField => "globalField",
        SymbolKind::Field => "field",
        SymbolKind::Method => "method",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Destructor => "destructor",
        SymbolKind::Parameter => "parameter",
        SymbolKind::LocalVariable => "localVariable",
        SymbolKind::PreprocessorMacro => "preprocessorMacro",
    }
}
fn owner_name(index: &SymbolIndex, symbol: &IndexedSymbol) -> Option<String> {
    symbol
        .parent
        .and_then(|parent| index.symbol(parent))
        .and_then(|parent| parent.name.clone())
}
fn qualify(owner: Option<&str>, name: &str) -> String {
    owner
        .map(|owner| format!("{owner}.{name}"))
        .unwrap_or_else(|| name.to_string())
}
fn logical_path(file: &crate::index::IndexedFile) -> String {
    file.metadata
        .relative_path
        .as_ref()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| "<unknown>".to_string())
}
fn match_rank(
    symbol: &IndexedSymbol,
    qualified_name: &str,
    signature: Option<&str>,
    query: &str,
    query_folded: &str,
) -> Option<(u8, &'static str)> {
    let name = symbol.name.as_deref()?;
    let name_folded = name.to_lowercase();
    if name == query {
        Some((1, "exactName"))
    } else if name_folded == query_folded {
        Some((2, "caseInsensitiveName"))
    } else if name_folded.starts_with(query_folded) {
        Some((3, "namePrefix"))
    } else if qualified_name != name && qualified_name.to_lowercase().contains(query_folded) {
        Some((4, "qualifiedName"))
    } else if name_folded.contains(query_folded) {
        Some((5, "nameSubstring"))
    } else if signature.is_some_and(|value| value.to_lowercase().contains(query_folded)) {
        Some((6, "signature"))
    } else if [
        symbol.detail.type_text.as_deref(),
        symbol.detail.return_type_text.as_deref(),
        symbol.detail.base_type.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| value.to_lowercase().contains(query_folded))
    {
        Some((7, "type"))
    } else {
        None
    }
}
fn project_hit(
    index: &SymbolIndex,
    source_line_starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    revision: &str,
    candidate: Candidate,
) -> GameDataSearchHit {
    let symbol = index.symbol(candidate.id).expect("candidate symbol exists");
    let file = index
        .file(candidate.id.file_id)
        .expect("candidate file exists");
    let owner = owner_name(index, symbol);
    let line_starts = source_line_starts
        .get(&candidate.id.file_id)
        .cloned()
        .unwrap_or_default();
    let declaration_range = line_starts.range(symbol.span.start, symbol.span.end);
    let selection_range = line_starts.range(symbol.selection_span.start, symbol.selection_span.end);
    let symbol_ref = encode_symbol_ref(
        revision,
        &candidate.path,
        &candidate.kind,
        &candidate.qualified_name,
        symbol.selection_span.start,
    );
    let signature = index
        .callable_signature(candidate.id)
        .unwrap_or_else(|| compact_signature(symbol, &candidate.qualified_name));
    let relative_path = candidate.path;
    GameDataSearchHit {
        inspect_input: InspectInput {
            symbol_ref: symbol_ref.clone(),
        },
        read_source_input: ReadSourceInput {
            catalogue_revision: revision.to_string(),
            relative_path: relative_path.clone(),
            start_line: declaration_range.start_line,
            end_line: declaration_range.end_line,
        },
        symbol_ref,
        name: symbol.name.clone().unwrap_or_default(),
        kind: candidate.kind,
        qualified_name: candidate.qualified_name,
        owner,
        signature,
        documentation_summary: documentation_summary(symbol),
        source_category: file.metadata.category.as_str().to_string(),
        relative_path,
        declaration_range,
        selection_range,
        match_kind: candidate.match_kind.to_string(),
    }
}
fn compact_signature(symbol: &IndexedSymbol, qualified_name: &str) -> String {
    match symbol.kind {
        SymbolKind::Class => symbol
            .detail
            .base_type
            .as_deref()
            .map(|base| format!("class {qualified_name} : {base}"))
            .unwrap_or_else(|| format!("class {qualified_name}")),
        SymbolKind::Enum => format!("enum {qualified_name}"),
        _ => symbol
            .detail
            .type_text
            .as_deref()
            .map(|value| format!("{qualified_name}: {value}"))
            .or_else(|| {
                symbol
                    .detail
                    .return_type_text
                    .as_deref()
                    .map(|value| format!("{qualified_name} -> {value}"))
            })
            .unwrap_or_else(|| qualified_name.to_string()),
    }
}
fn documentation_summary(symbol: &IndexedSymbol) -> Option<String> {
    symbol
        .doc_comments
        .iter()
        .flat_map(|comment| comment.text.lines())
        .map(|line| {
            line.trim()
                .trim_start_matches('/')
                .trim_start_matches('*')
                .trim()
        })
        .find(|line| !line.is_empty())
        .map(|line| line.chars().take(512).collect())
}
fn line_for_offset(starts: &[usize], offset: usize) -> usize {
    starts.partition_point(|start| *start <= offset).max(1)
}
fn encode_symbol_ref(
    revision: &str,
    path: &str,
    kind: &str,
    qualified_name: &str,
    selection_start: usize,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"sr1\0");
    for part in [revision, path, kind, qualified_name, &selection_start.to_string()] {
        digest.update(part.as_bytes());
        digest.update([0]);
    }
    format!("sr1:{}", hex(&digest.finalize()))
}
fn encode_cursor(cursor: &Cursor) -> String {
    hex(&serde_json::to_vec(cursor).expect("cursor serializes"))
}
fn decode_cursor(value: &str) -> Result<Cursor, GameDataSearchError> {
    if value.len() > MAX_CURSOR_BYTES {
        return Err(GameDataSearchError::InvalidCursor);
    }
    let bytes = unhex(value).ok_or(GameDataSearchError::InvalidCursor)?;
    let cursor =
        serde_json::from_slice::<Cursor>(&bytes).map_err(|_| GameDataSearchError::InvalidCursor)?;
    (cursor.version == 1)
        .then_some(cursor)
        .ok_or(GameDataSearchError::InvalidCursor)
}
fn hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut value, "{byte:02x}").expect("string write");
    }
    value
}
fn unhex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}
