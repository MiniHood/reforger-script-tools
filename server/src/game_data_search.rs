use crate::index::{GlobalSymbolId, IndexedSymbol, SourceFileId, SymbolIndex};
use crate::index_build::IndexBuildControl;
use crate::model::{SourceCategory, SymbolKind};
use crate::symbol_display::documentation_display;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use url::Url;

pub const DEFAULT_LIMIT: usize = 20;
pub const MAX_LIMIT: usize = 100;
pub const MAX_QUERY_CHARS: usize = 256;
pub const MAX_CURSOR_BYTES: usize = 2048;
pub const MAX_OWNER_CHARS: usize = 256;
pub const MAX_SEARCH_RESULT_BYTES: usize = 256 * 1024;
const MAX_SEARCH_TEXT_CHARS: usize = 16 * 1024;

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

    pub(crate) fn from_cached_starts(starts: Vec<usize>) -> Self {
        Self(if starts.is_empty() { vec![0] } else { starts })
    }

    pub(crate) fn range(&self, start: usize, end: usize) -> SourceLineRange {
        SourceLineRange {
            start_line: line_for_offset(&self.0, start),
            end_line: line_for_offset(&self.0, end),
        }
    }

    pub(crate) fn line_count(&self) -> usize {
        self.0.len().max(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDataSearchRequest {
    pub query: String,
    pub addon_guids: Option<Vec<String>>,
    pub kinds: Option<Vec<String>>,
    pub owner: Option<String>,
    pub source_categories: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub offset: Option<usize>,
}

impl GameDataSearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            addon_guids: None,
            kinds: None,
            owner: None,
            source_categories: None,
            limit: None,
            cursor: None,
            offset: None,
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
    pub truncated: bool,
    pub totals_by_addon: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub results: Vec<GameDataSearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppliedFilters {
    pub addon_guids: Vec<String>,
    pub kinds: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub source_categories: Vec<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameDataSearchHit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_color: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Optional editor-client source identity. Treat as opaque; do not construct or use it as an MCP source-read input."
    )]
    pub source_uri: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon_guid: Option<String>,
    pub relative_path: String,
    pub start_line: usize,
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
    origin_rank: u8,
    match_kind: &'static str,
    qualified_name: String,
    kind: String,
    path: String,
    position: usize,
    addon_guid: Option<String>,
    addon_label: Option<String>,
    thumbnail_color: Option<String>,
}

fn compare_candidates(left: &Candidate, right: &Candidate) -> Ordering {
    (
        left.rank,
        left.origin_rank,
        &left.qualified_name,
        &left.kind,
        &left.path,
        &left.addon_guid,
        left.position,
        left.id,
    )
        .cmp(&(
            right.rank,
            right.origin_rank,
            &right.qualified_name,
            &right.kind,
            &right.path,
            &right.addon_guid,
            right.position,
            right.id,
        ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDataAddonIdentity {
    pub guid: String,
    pub label: String,
    pub thumbnail_color: Option<String>,
}

pub type GameDataAddonMap = BTreeMap<SourceFileId, GameDataAddonIdentity>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Cursor {
    version: u8,
    catalogue_revision: String,
    query: String,
    kinds: Vec<String>,
    owner: Option<String>,
    source_categories: Vec<String>,
    addon_guids: Vec<String>,
    offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SymbolReference {
    version: u8,
    pub(crate) catalogue_revision: String,
    pub(crate) addon_guid: Option<String>,
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) qualified_name: String,
    pub(crate) selection_start: usize,
}

pub fn search(
    index: &SymbolIndex,
    source_line_starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    control: &IndexBuildControl,
    catalogue_revision: &str,
    request: GameDataSearchRequest,
) -> Result<GameDataSearchPage, GameDataSearchError> {
    let identity = GameDataAddonIdentity {
        guid: crate::addon_sources::BASE_GAME_GUID.to_string(),
        label: "Arma Reforger".to_string(),
        thumbnail_color: None,
    };
    let addon_map = index
        .files()
        .iter()
        .map(|file| (file.id, identity.clone()))
        .collect();
    search_scoped(
        index,
        source_line_starts,
        &addon_map,
        control,
        catalogue_revision,
        request,
    )
}

pub fn search_scoped(
    index: &SymbolIndex,
    source_line_starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    addon_map: &GameDataAddonMap,
    control: &IndexBuildControl,
    catalogue_revision: &str,
    request: GameDataSearchRequest,
) -> Result<GameDataSearchPage, GameDataSearchError> {
    search_with_scope(
        index,
        source_line_starts,
        addon_map,
        control,
        catalogue_revision,
        request,
        false,
    )
}

/// Search a language-owned workspace index. The result shape intentionally
/// stays identical to Game Data search so symbol references, pagination, and
/// downstream inspection remain one protocol.
pub fn search_workspace(
    index: &SymbolIndex,
    source_line_starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    control: &IndexBuildControl,
    workspace_revision: &str,
    request: GameDataSearchRequest,
) -> Result<GameDataSearchPage, GameDataSearchError> {
    let addon_map = GameDataAddonMap::new();
    search_with_scope(
        index,
        source_line_starts,
        &addon_map,
        control,
        workspace_revision,
        request,
        true,
    )
}

fn search_with_scope(
    index: &SymbolIndex,
    source_line_starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    addon_map: &GameDataAddonMap,
    control: &IndexBuildControl,
    catalogue_revision: &str,
    request: GameDataSearchRequest,
    workspace_scope: bool,
) -> Result<GameDataSearchPage, GameDataSearchError> {
    let query = normalize_query(&request.query)?;
    let kinds = canonical_kinds(request.kinds.as_deref())?;
    let source_categories =
        canonical_categories(request.source_categories.as_deref(), workspace_scope)?;
    let addon_guids =
        canonical_addon_guids(request.addon_guids.as_deref(), addon_map, workspace_scope)?;
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
    if owner
        .as_ref()
        .is_some_and(|value| value.chars().count() > MAX_OWNER_CHARS)
    {
        return Err(GameDataSearchError::InvalidRequest(
            "owner exceeds 256 characters",
        ));
    }
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    if request.cursor.is_some() && request.offset.is_some() {
        return Err(GameDataSearchError::InvalidRequest(
            "offset cannot be combined with cursor",
        ));
    }
    if request
        .offset
        .is_some_and(|offset| offset > crate::search_limits::MAX_RANDOM_ACCESS_OFFSET)
    {
        return Err(GameDataSearchError::InvalidRequest(
            "offset exceeds the random-access search limit",
        ));
    }
    let cursor = request.cursor.as_deref().map(decode_cursor).transpose()?;
    if let Some(cursor) = &cursor {
        if cursor.catalogue_revision != catalogue_revision {
            return Err(GameDataSearchError::StaleCursor);
        }
        if cursor.query != query
            || cursor.kinds != kinds
            || cursor.owner != owner
            || cursor.source_categories != source_categories
            || cursor.addon_guids != addon_guids
        {
            return Err(GameDataSearchError::InvalidCursor);
        }
    }
    let offset = request
        .offset
        .or_else(|| cursor.as_ref().map(|cursor| cursor.offset))
        .unwrap_or(0);
    let query_folded = query.to_lowercase();
    let mut candidates = Vec::new();
    for symbol in index.symbol_iter() {
        control
            .check()
            .map_err(|_| GameDataSearchError::Cancelled)?;
        let Some(file) = index.file(symbol.id.file_id) else {
            continue;
        };
        let addon = addon_map.get(&symbol.id.file_id);
        if !workspace_scope
            && addon.is_none_or(|identity| addon_guids.binary_search(&identity.guid).is_err())
        {
            continue;
        }
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
            origin_rank: declaration_origin_rank(index, symbol),
            match_kind,
            qualified_name,
            kind,
            path: logical_path(file),
            position: symbol.span.start,
            addon_guid: addon.map(|identity| identity.guid.clone()),
            addon_label: addon.map(|identity| identity.label.clone()),
            thumbnail_color: addon.and_then(|identity| identity.thumbnail_color.clone()),
        });
    }
    control
        .check()
        .map_err(|_| GameDataSearchError::Cancelled)?;
    let truncated = candidates.len() > crate::search_limits::MAX_SEARCH_RESULTS;
    if truncated {
        candidates
            .select_nth_unstable_by(crate::search_limits::MAX_SEARCH_RESULTS, compare_candidates);
        candidates.truncate(crate::search_limits::MAX_SEARCH_RESULTS);
    }
    candidates.sort_by(compare_candidates);
    let total = candidates.len();
    let mut totals_by_addon = BTreeMap::new();
    for candidate in &candidates {
        if let Some(guid) = &candidate.addon_guid {
            *totals_by_addon.entry(guid.clone()).or_insert(0) += 1;
        }
    }
    let page_candidates = candidates
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let returned = page_candidates.len();
    let next_cursor = (offset + returned < total).then(|| {
        encode_cursor(&Cursor {
            version: 2,
            catalogue_revision: catalogue_revision.to_string(),
            query: query.clone(),
            kinds: kinds.clone(),
            owner: owner.clone(),
            source_categories: source_categories.clone(),
            addon_guids: addon_guids.clone(),
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
            addon_guids,
            limit,
        },
        returned,
        total,
        truncated,
        totals_by_addon,
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
            version: 2,
            catalogue_revision: page.catalogue_revision.clone(),
            query: page.query.clone(),
            kinds: page.applied_filters.kinds.clone(),
            owner: page.applied_filters.owner.clone(),
            source_categories: page.applied_filters.source_categories.clone(),
            addon_guids: page.applied_filters.addon_guids.clone(),
            offset: offset + page.returned,
        }));
    }
    Ok(page)
}

fn normalize_query(query: &str) -> Result<String, GameDataSearchError> {
    let normalized = query.split_whitespace().collect::<Vec<_>>().join(" ");
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
            ));
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
fn canonical_categories(
    values: Option<&[String]>,
    workspace_scope: bool,
) -> Result<Vec<String>, GameDataSearchError> {
    let values = match values {
        Some(values) if values.is_empty() => {
            return Err(GameDataSearchError::InvalidRequest(
                "sourceCategories must be non-empty",
            ));
        }
        Some(values) => values.to_vec(),
        None if workspace_scope => vec!["workspace".to_string()],
        None => all_categories().into_iter().map(str::to_string).collect(),
    };
    let allowed = if workspace_scope {
        vec!["workspace"]
    } else {
        all_categories()
    };
    let value_count = values.len();
    let unique = values.into_iter().collect::<BTreeSet<_>>();
    if unique.len() != value_count
        || unique.iter().any(|value| {
            !allowed.contains(&value.as_str())
                || (!workspace_scope && value == "workspace")
                || (workspace_scope && value != "workspace")
        })
    {
        let message = if workspace_scope {
            "sourceCategories must contain only the workspace category"
        } else {
            "sourceCategories must be unique game-data categories"
        };
        return Err(GameDataSearchError::InvalidRequest(message));
    }
    Ok(unique.into_iter().collect())
}

fn canonical_addon_guids(
    values: Option<&[String]>,
    addon_map: &GameDataAddonMap,
    workspace_scope: bool,
) -> Result<Vec<String>, GameDataSearchError> {
    if workspace_scope {
        if values.is_some() {
            return Err(GameDataSearchError::InvalidRequest(
                "addonGuids is supported only for Game Data search",
            ));
        }
        return Ok(Vec::new());
    }
    let available = addon_map
        .values()
        .map(|identity| identity.guid.clone())
        .collect::<BTreeSet<_>>();
    let Some(values) = values else {
        return Ok(available.into_iter().collect());
    };
    if values.is_empty() {
        return Err(GameDataSearchError::InvalidRequest(
            "addonGuids must be non-empty when provided",
        ));
    }
    let mut canonical = Vec::with_capacity(values.len());
    let mut unique = BTreeSet::new();
    for value in values {
        let guid = value.to_ascii_uppercase();
        if guid.len() != 16
            || !guid.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !unique.insert(guid.clone())
            || !available.contains(&guid)
        {
            return Err(GameDataSearchError::InvalidRequest(
                "addonGuids must contain unique loaded 16-character hexadecimal GUIDs",
            ));
        }
        canonical.push(guid);
    }
    canonical.sort();
    Ok(canonical)
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
    SourceCategory::GAME_DATA_FILTER_VALUES.to_vec()
}
pub(crate) fn kind_name(kind: SymbolKind) -> &'static str {
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
pub(crate) fn owner_name(index: &SymbolIndex, symbol: &IndexedSymbol) -> Option<String> {
    symbol
        .parent
        .and_then(|parent| index.symbol(parent))
        .and_then(|parent| parent.name.clone())
}
pub(crate) fn qualify(owner: Option<&str>, name: &str) -> String {
    owner
        .map(|owner| format!("{owner}.{name}"))
        .unwrap_or_else(|| name.to_string())
}
pub(crate) fn logical_path(file: &crate::index::IndexedFile) -> String {
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
    if query.is_empty() {
        return Some((8, "kind"));
    }
    // An identifier ending in `_` is the editor's common prefix-search shape
    // (`SCR_`, `GC_`, and similar). Do not turn that prefix into a broad
    // search over containing names, signatures, or types: those fields can
    // contain the prefix while the declared symbol is unrelated to it.
    let is_identifier_prefix = query_folded.ends_with('_')
        && query_folded
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    if name == query {
        Some((1, "exactName"))
    } else if name_folded == query_folded {
        Some((2, "caseInsensitiveName"))
    } else if name_folded.starts_with(query_folded) {
        Some((3, "namePrefix"))
    } else if !is_identifier_prefix
        && qualified_name != name
        && qualified_name.to_lowercase().contains(query_folded)
    {
        Some((4, "qualifiedName"))
    } else if name_folded.contains(query_folded) {
        Some((5, "nameSubstring"))
    } else if !is_identifier_prefix
        && signature.is_some_and(|value| value.to_lowercase().contains(query_folded))
    {
        Some((6, "signature"))
    } else if !is_identifier_prefix
        && [
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

fn declaration_origin_rank(index: &SymbolIndex, symbol: &IndexedSymbol) -> u8 {
    let mut declaration = Some(symbol);
    while let Some(current) = declaration {
        if current
            .modifiers
            .iter()
            .any(|modifier| modifier == "modded" || modifier == "override")
        {
            return 1;
        }
        declaration = current.parent.and_then(|parent| index.symbol(parent));
    }
    0
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
        candidate.addon_guid.as_deref(),
        &candidate.path,
        &candidate.kind,
        &candidate.qualified_name,
        symbol.selection_span.start,
    );
    let signature = index
        .callable_signature(candidate.id)
        .unwrap_or_else(|| compact_signature(symbol, &candidate.qualified_name));
    let source_path = candidate.path;
    let relative_path = bounded_search_text(source_path.clone());
    let source_uri = file
        .metadata
        .virtual_source
        .as_ref()
        .map(|source| source.uri.clone())
        .or_else(|| {
            file.metadata
                .absolute_path
                .as_ref()
                .and_then(|path| Url::from_file_path(path).ok())
                .map(|uri| uri.to_string())
        });
    GameDataSearchHit {
        inspect_input: InspectInput {
            symbol_ref: symbol_ref.clone(),
        },
        read_source_input: ReadSourceInput {
            catalogue_revision: revision.to_string(),
            addon_guid: candidate.addon_guid.clone(),
            relative_path: source_path,
            start_line: declaration_range.start_line,
        },
        symbol_ref,
        addon_guid: candidate.addon_guid,
        addon_label: candidate.addon_label,
        thumbnail_color: candidate.thumbnail_color,
        name: bounded_search_text(symbol.name.clone().unwrap_or_default()),
        kind: candidate.kind,
        qualified_name: bounded_search_text(candidate.qualified_name),
        owner: owner.map(bounded_search_text),
        signature: bounded_search_text(signature),
        documentation_summary: documentation_summary(symbol),
        source_category: file.metadata.category.as_str().to_string(),
        relative_path,
        source_uri,
        declaration_range,
        selection_range,
        match_kind: candidate.match_kind.to_string(),
    }
}
pub(crate) fn compact_signature(symbol: &IndexedSymbol, qualified_name: &str) -> String {
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
pub(crate) fn documentation_summary(symbol: &IndexedSymbol) -> Option<String> {
    documentation_display(&symbol.doc_comments)
        .summary
        .map(|summary| summary.chars().take(512).collect())
}

fn bounded_search_text(value: String) -> String {
    let mut characters = value.chars();
    let prefix = characters
        .by_ref()
        .take(MAX_SEARCH_TEXT_CHARS)
        .collect::<String>();
    if characters.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}
fn line_for_offset(starts: &[usize], offset: usize) -> usize {
    starts.partition_point(|start| *start <= offset).max(1)
}
pub(crate) fn encode_symbol_ref(
    revision: &str,
    addon_guid: Option<&str>,
    path: &str,
    kind: &str,
    qualified_name: &str,
    selection_start: usize,
) -> String {
    format!(
        "sr2:{}",
        hex(&serde_json::to_vec(&SymbolReference {
            version: 2,
            catalogue_revision: revision.to_string(),
            addon_guid: addon_guid.map(str::to_string),
            path: path.to_string(),
            kind: kind.to_string(),
            qualified_name: qualified_name.to_string(),
            selection_start,
        })
        .expect("symbol reference serializes"),)
    )
}
pub(crate) fn decode_symbol_ref(value: &str) -> Option<SymbolReference> {
    let encoded = value.strip_prefix("sr2:")?;
    if value.len() > MAX_CURSOR_BYTES || encoded.is_empty() {
        return None;
    }
    let reference = serde_json::from_slice::<SymbolReference>(&unhex(encoded)?).ok()?;
    (reference.version == 2
        && !reference.catalogue_revision.is_empty()
        && !reference.path.is_empty()
        && !reference.kind.is_empty()
        && !reference.qualified_name.is_empty())
    .then_some(reference)
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
    (cursor.version == 2)
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
