use crate::game_data_inspection::{inspect, GameDataInspectionError, InspectionConditionalContext};
use crate::game_data_search::{
    compact_signature, documentation_summary, encode_symbol_ref, kind_name, logical_path,
    owner_name, qualify, GameDataAddonMap, ReadSourceInput, SourceLineRange, SourceLineStarts,
};
use crate::index::{GlobalSymbolId, IndexedSymbol, SymbolIndex};
use crate::index_build::IndexBuildControl;
use schemars::JsonSchema;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ALTERNATIVES: usize = 2;
const MAX_RELEVANT_MEMBERS: usize = 5;
const MAX_MODIFIERS: usize = 8;
const MAX_ATTRIBUTES: usize = 16;
const MAX_CONDITIONAL_CONTEXTS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDataIntentRequest {
    pub query: String,
    pub addon_guids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameDataIntentResult {
    pub status: IntentStatus,
    pub catalogue_revision: String,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<IntentSymbol>,
    #[schemars(length(max = 2))]
    pub alternatives: Vec<IntentAlternative>,
    pub follow_up: IntentFollowUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum IntentStatus {
    Resolved,
    Ambiguous,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum IntentFollowUp {
    None,
    RefineQuery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntentSymbol {
    pub symbol_ref: String,
    pub qualified_name: String,
    pub kind: String,
    pub signature: String,
    pub matched_terms: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_type: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callable_form: Option<String>,
    #[schemars(length(max = 8))]
    pub modifiers: Vec<String>,
    #[schemars(length(max = 16))]
    pub attributes: Vec<String>,
    #[schemars(length(max = 8))]
    pub conditional_context: Vec<InspectionConditionalContext>,
    pub source_category: String,
    pub relative_path: String,
    pub declaration_range: SourceLineRange,
    pub read_source_input: ReadSourceInput,
    #[schemars(length(max = 5))]
    pub relevant_members: Vec<IntentMember>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntentAlternative {
    pub symbol_ref: String,
    pub qualified_name: String,
    pub kind: String,
    pub signature: String,
    pub matched_terms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntentMember {
    pub symbol_ref: String,
    pub name: String,
    pub kind: String,
    pub signature: String,
    pub matched_terms: Vec<String>,
}

#[derive(Debug)]
pub enum GameDataIntentError {
    InvalidRequest(String),
    Inspection(GameDataInspectionError),
    Cancelled,
}

#[derive(Debug, Clone)]
struct IntentCandidate {
    id: GlobalSymbolId,
    score: u32,
    whole_query_exact: bool,
    origin_rank: u8,
    matched_terms: Vec<String>,
}

pub fn research_game_data(
    index: &SymbolIndex,
    source_line_starts: &BTreeMap<crate::index::SourceFileId, SourceLineStarts>,
    addon_map: &GameDataAddonMap,
    control: &IndexBuildControl,
    catalogue_revision: &str,
    request: GameDataIntentRequest,
) -> Result<GameDataIntentResult, GameDataIntentError> {
    let query = request
        .query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if query.is_empty() {
        return Err(GameDataIntentError::InvalidRequest(
            "query must be non-empty".to_string(),
        ));
    }
    if query.chars().count() > 256 {
        return Err(GameDataIntentError::InvalidRequest(
            "query exceeds 256 characters".to_string(),
        ));
    }
    let selected_addons = selected_addons(request.addon_guids.as_deref(), addon_map)?;
    let query_terms = meaningful_terms(&query);
    let mut candidates = Vec::new();
    for symbol in index.symbol_iter() {
        control
            .check()
            .map_err(|_| GameDataIntentError::Cancelled)?;
        if !symbol_is_in_scope(symbol, addon_map, &selected_addons) {
            continue;
        }
        if let Some(candidate) = score_candidate(index, symbol, &query, &query_terms) {
            candidates.push(candidate);
        }
    }
    candidates.sort_by(compare_candidates);
    let exact_matches = candidates
        .iter()
        .filter(|candidate| candidate.whole_query_exact)
        .count();
    candidates.truncate(MAX_ALTERNATIVES + 1);
    let Some(primary_candidate) = candidates.first() else {
        return Ok(not_found(catalogue_revision, query));
    };
    let exact_is_unique = primary_candidate.whole_query_exact && exact_matches == 1;
    let primary = project_primary(
        index,
        source_line_starts,
        addon_map,
        control,
        catalogue_revision,
        primary_candidate,
        &query_terms,
    )?;
    let alternatives = if exact_is_unique {
        Vec::new()
    } else {
        candidates
            .iter()
            .skip(1)
            .take(MAX_ALTERNATIVES)
            .map(|candidate| project_alternative(index, addon_map, catalogue_revision, candidate))
            .collect::<Result<Vec<_>, _>>()?
    };
    let resolved = exact_is_unique
        || (primary_candidate.matched_terms.len() >= 2
            && candidates
                .get(1)
                .is_none_or(|next| primary_candidate.score >= next.score.saturating_add(100)));
    Ok(GameDataIntentResult {
        status: if resolved {
            IntentStatus::Resolved
        } else {
            IntentStatus::Ambiguous
        },
        catalogue_revision: catalogue_revision.to_string(),
        query,
        primary: Some(primary),
        alternatives,
        follow_up: if resolved {
            IntentFollowUp::None
        } else {
            IntentFollowUp::RefineQuery
        },
    })
}

fn not_found(catalogue_revision: &str, query: String) -> GameDataIntentResult {
    GameDataIntentResult {
        status: IntentStatus::NotFound,
        catalogue_revision: catalogue_revision.to_string(),
        query,
        primary: None,
        alternatives: Vec::new(),
        follow_up: IntentFollowUp::RefineQuery,
    }
}

fn project_primary(
    index: &SymbolIndex,
    source_line_starts: &BTreeMap<crate::index::SourceFileId, SourceLineStarts>,
    addon_map: &GameDataAddonMap,
    control: &IndexBuildControl,
    catalogue_revision: &str,
    candidate: &IntentCandidate,
    query_terms: &[String],
) -> Result<IntentSymbol, GameDataIntentError> {
    let symbol = index.symbol(candidate.id).ok_or_else(|| {
        GameDataIntentError::InvalidRequest("candidate symbol is stale".to_string())
    })?;
    let file = index.file(candidate.id.file_id).ok_or_else(|| {
        GameDataIntentError::InvalidRequest("candidate source is stale".to_string())
    })?;
    let starts = source_line_starts
        .get(&candidate.id.file_id)
        .ok_or_else(|| {
            GameDataIntentError::InvalidRequest("candidate lines are unavailable".to_string())
        })?;
    let name = symbol
        .name
        .as_deref()
        .ok_or_else(|| GameDataIntentError::InvalidRequest("candidate has no name".to_string()))?;
    let qualified_name = qualify(owner_name(index, symbol).as_deref(), name);
    let symbol_ref = symbol_ref_for(
        index,
        addon_map,
        catalogue_revision,
        symbol,
        &qualified_name,
    )?;
    let inspection = inspect(
        index,
        source_line_starts,
        addon_map,
        control,
        catalogue_revision,
        &symbol_ref,
    )
    .map_err(GameDataIntentError::Inspection)?;
    let declaration_range = starts.range(symbol.span.start, symbol.span.end);
    let owner_terms = identifier_terms(name).into_iter().collect::<BTreeSet<_>>();
    let member_query_terms = query_terms
        .iter()
        .filter(|term| !owner_terms.contains(*term))
        .cloned()
        .collect::<Vec<_>>();
    let required_member_matches = member_query_terms.len().min(2);
    let mut relevant_members = index
        .children(candidate.id)
        .iter()
        .filter_map(|id| index.symbol(*id))
        .filter_map(|member| {
            let member_candidate = score_candidate(index, member, "", &member_query_terms)?;
            if member_candidate.matched_terms.len() < required_member_matches {
                return None;
            }
            let projected = IntentMember {
                symbol_ref: symbol_ref_for(
                    index,
                    addon_map,
                    catalogue_revision,
                    member,
                    &qualify(
                        owner_name(index, member).as_deref(),
                        member.name.as_deref()?,
                    ),
                )
                .ok()?,
                name: member.name.clone()?,
                kind: kind_name(member.kind).to_string(),
                signature: compact_signature(
                    member,
                    &qualify(
                        owner_name(index, member).as_deref(),
                        member.name.as_deref()?,
                    ),
                ),
                matched_terms: member_candidate.matched_terms.clone(),
            };
            Some((member_candidate, projected))
        })
        .collect::<Vec<_>>();
    relevant_members.sort_by(|(left, _), (right, _)| compare_candidates(left, right));
    let relevant_members = relevant_members
        .into_iter()
        .take(MAX_RELEVANT_MEMBERS)
        .map(|(_, member)| member)
        .collect();
    Ok(IntentSymbol {
        symbol_ref,
        qualified_name: qualified_name.clone(),
        kind: kind_name(symbol.kind).to_string(),
        signature: inspection.signature,
        matched_terms: candidate.matched_terms.clone(),
        documentation_summary: documentation_summary(symbol),
        base_type: inspection.base_type,
        type_text: inspection.type_text,
        return_type: inspection.return_type,
        default_value: inspection.default_value,
        enum_value: inspection.enum_value,
        callable_form: inspection.callable_form,
        modifiers: inspection
            .modifiers
            .into_iter()
            .take(MAX_MODIFIERS)
            .collect(),
        attributes: inspection
            .attributes
            .into_iter()
            .take(MAX_ATTRIBUTES)
            .collect(),
        conditional_context: inspection
            .conditional_context
            .into_iter()
            .take(MAX_CONDITIONAL_CONTEXTS)
            .collect(),
        source_category: file.metadata.category.as_str().to_string(),
        relative_path: logical_path(file),
        declaration_range: declaration_range.clone(),
        read_source_input: ReadSourceInput {
            catalogue_revision: catalogue_revision.to_string(),
            addon_guid: addon_map
                .get(&candidate.id.file_id)
                .map(|addon| addon.guid.clone()),
            relative_path: logical_path(file),
            start_line: declaration_range.start_line,
        },
        relevant_members,
    })
}

fn project_alternative(
    index: &SymbolIndex,
    addon_map: &GameDataAddonMap,
    catalogue_revision: &str,
    candidate: &IntentCandidate,
) -> Result<IntentAlternative, GameDataIntentError> {
    let symbol = index.symbol(candidate.id).ok_or_else(|| {
        GameDataIntentError::InvalidRequest("candidate symbol is stale".to_string())
    })?;
    let name = symbol
        .name
        .as_deref()
        .ok_or_else(|| GameDataIntentError::InvalidRequest("candidate has no name".to_string()))?;
    let qualified_name = qualify(owner_name(index, symbol).as_deref(), name);
    Ok(IntentAlternative {
        symbol_ref: symbol_ref_for(
            index,
            addon_map,
            catalogue_revision,
            symbol,
            &qualified_name,
        )?,
        qualified_name: qualified_name.clone(),
        kind: kind_name(symbol.kind).to_string(),
        signature: compact_signature(symbol, &qualified_name),
        matched_terms: candidate.matched_terms.clone(),
    })
}

fn symbol_ref_for(
    index: &SymbolIndex,
    addon_map: &GameDataAddonMap,
    catalogue_revision: &str,
    symbol: &IndexedSymbol,
    qualified_name: &str,
) -> Result<String, GameDataIntentError> {
    let file = index.file(symbol.id.file_id).ok_or_else(|| {
        GameDataIntentError::InvalidRequest("candidate source is stale".to_string())
    })?;
    Ok(encode_symbol_ref(
        catalogue_revision,
        addon_map
            .get(&symbol.id.file_id)
            .map(|addon| addon.guid.as_str()),
        &logical_path(file),
        kind_name(symbol.kind),
        qualified_name,
        symbol.selection_span.start,
    ))
}

fn selected_addons(
    requested: Option<&[String]>,
    addon_map: &GameDataAddonMap,
) -> Result<BTreeSet<String>, GameDataIntentError> {
    let available = addon_map
        .values()
        .map(|addon| addon.guid.clone())
        .collect::<BTreeSet<_>>();
    let Some(requested) = requested else {
        return Ok(available);
    };
    if requested.is_empty() {
        return Err(GameDataIntentError::InvalidRequest(
            "addonGuids must be non-empty when provided".to_string(),
        ));
    }
    let selected = requested
        .iter()
        .map(|guid| guid.to_ascii_uppercase())
        .collect::<BTreeSet<_>>();
    if selected.len() != requested.len()
        || selected.iter().any(|guid| {
            guid.len() != 16
                || !guid.bytes().all(|byte| byte.is_ascii_hexdigit())
                || !available.contains(guid)
        })
    {
        return Err(GameDataIntentError::InvalidRequest(
            "addonGuids must contain unique loaded 16-character hexadecimal GUIDs".to_string(),
        ));
    }
    Ok(selected)
}

fn symbol_is_in_scope(
    symbol: &IndexedSymbol,
    addon_map: &GameDataAddonMap,
    selected_addons: &BTreeSet<String>,
) -> bool {
    addon_map
        .get(&symbol.id.file_id)
        .is_some_and(|addon| selected_addons.contains(&addon.guid))
}

fn score_candidate(
    index: &SymbolIndex,
    symbol: &IndexedSymbol,
    raw_query: &str,
    query_terms: &[String],
) -> Option<IntentCandidate> {
    let name = symbol.name.as_deref()?;
    let owner = owner_name(index, symbol);
    let qualified = qualify(owner.as_deref(), name);
    let whole_query_exact = !raw_query.is_empty()
        && (name.eq_ignore_ascii_case(raw_query) || qualified.eq_ignore_ascii_case(raw_query));
    let identifier_anchor = whole_query_exact
        || raw_query
            .split_whitespace()
            .map(|term| {
                term.trim_matches(|character: char| {
                    !character.is_alphanumeric() && character != '_'
                })
            })
            .any(|term| name.eq_ignore_ascii_case(term) || qualified.eq_ignore_ascii_case(term));
    let name_terms = identifier_terms(name);
    let owner_terms = owner.as_deref().map(identifier_terms).unwrap_or_default();
    let supporting_text = [
        index.callable_signature(symbol.id),
        symbol.detail.type_text.clone(),
        symbol.detail.return_type_text.clone(),
        symbol.detail.base_type.clone(),
        documentation_summary(symbol),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
    .to_lowercase();
    let mut matched_terms = Vec::new();
    let mut score = if identifier_anchor { 10_000 } else { 0 };
    for term in query_terms {
        let term_score = if name_terms.contains(term) {
            240
        } else if owner_terms.contains(term) {
            150
        } else if name.to_lowercase().contains(term) {
            100
        } else if supporting_text.contains(term) {
            35
        } else {
            0
        };
        if term_score > 0 {
            score += term_score;
            matched_terms.push(term.clone());
        }
    }
    if query_terms
        .iter()
        .any(|term| matches!(term.as_str(), "callback" | "hook" | "method" | "function"))
        && matches!(
            symbol.kind,
            crate::model::SymbolKind::Method | crate::model::SymbolKind::Function
        )
    {
        score += 120;
    }
    if !identifier_anchor && matched_terms.is_empty() {
        return None;
    }
    score += u32::try_from(matched_terms.len())
        .unwrap_or(u32::MAX)
        .saturating_mul(80);
    Some(IntentCandidate {
        id: symbol.id,
        score,
        whole_query_exact,
        origin_rank: crate::game_data_search::declaration_origin_rank(index, symbol),
        matched_terms,
    })
}

fn compare_candidates(left: &IntentCandidate, right: &IntentCandidate) -> Ordering {
    right
        .score
        .cmp(&left.score)
        .then_with(|| left.origin_rank.cmp(&right.origin_rank))
        .then_with(|| left.id.file_id.cmp(&right.id.file_id))
        .then_with(|| left.id.symbol_id.cmp(&right.id.symbol_id))
}

fn meaningful_terms(value: &str) -> Vec<String> {
    identifier_terms(value)
        .into_iter()
        .filter(|term| !STOP_WORDS.contains(&term.as_str()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn identifier_terms(value: &str) -> Vec<String> {
    let mut expanded = String::with_capacity(value.len() * 2);
    let mut previous = None;
    for character in value.chars() {
        if character == '_' || !character.is_alphanumeric() {
            expanded.push(' ');
            previous = None;
            continue;
        }
        if character.is_uppercase()
            && previous
                .is_some_and(|previous: char| previous.is_lowercase() || previous.is_ascii_digit())
        {
            expanded.push(' ');
        }
        expanded.push(character.to_ascii_lowercase());
        previous = Some(character);
    }
    expanded
        .split_whitespace()
        .map(normalize_term)
        .filter(|term| !term.is_empty())
        .collect()
}

fn normalize_term(value: &str) -> String {
    if value.len() > 4 && value.ends_with("ies") {
        return format!("{}y", &value[..value.len() - 3]);
    }
    if value.len() > 4
        && ["ses", "xes", "zes", "ches", "shes"]
            .iter()
            .any(|suffix| value.ends_with(suffix))
    {
        return value[..value.len() - 2].to_string();
    }
    if value.len() > 3 && value.ends_with('s') {
        return value[..value.len() - 1].to_string();
    }
    value.to_string()
}

const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "for", "from", "get", "how", "in", "me", "of", "on", "the", "this", "to",
    "when", "where", "with",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_data_search::GameDataAddonIdentity;
    use crate::index::SymbolIndex;
    use crate::model::{SourceCategory, SourceFileMetadata, SourceKind, SOURCE_PRIORITY_GAME_DATA};
    use crate::parser::parse_source;
    use crate::semantic_file::SemanticFile;
    use std::path::PathBuf;

    fn fixture(
        source: &str,
    ) -> (
        SymbolIndex,
        BTreeMap<crate::index::SourceFileId, SourceLineStarts>,
        GameDataAddonMap,
    ) {
        let parse = parse_source(source);
        let semantic_file = SemanticFile::build(source, &parse);
        let mut index = SymbolIndex::default();
        let file_id = index.add_semantic_file(
            &semantic_file,
            SourceFileMetadata {
                kind: SourceKind::GameData,
                category: SourceCategory::Game,
                absolute_path: None,
                virtual_source: None,
                root_path: None,
                relative_path: Some(PathBuf::from("Scripts/Game/Fixture.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let starts = BTreeMap::from([(file_id, SourceLineStarts::from_source(source))]);
        let addons = BTreeMap::from([(
            file_id,
            GameDataAddonIdentity {
                guid: "0123456789ABCDEF".to_string(),
                label: "Arma Reforger".to_string(),
                thumbnail_color: None,
            },
        )]);
        (index, starts, addons)
    }

    #[test]
    fn exact_identifier_returns_one_compact_primary_result() {
        let (index, starts, addons) =
            fixture("class SCR_BaseGameMode { void OnGameEnd() {} void UnrelatedHelper() {} }\n");

        let result = research_game_data(
            &index,
            &starts,
            &addons,
            &IndexBuildControl::default(),
            "revision-1",
            GameDataIntentRequest {
                query: "SCR_BaseGameMode".to_string(),
                addon_guids: None,
            },
        )
        .unwrap();

        assert_eq!(result.status, IntentStatus::Resolved);
        assert_eq!(
            result
                .primary
                .as_ref()
                .map(|symbol| symbol.qualified_name.as_str()),
            Some("SCR_BaseGameMode")
        );
        assert!(result.alternatives.is_empty());
        assert_eq!(result.follow_up, IntentFollowUp::None);
    }

    #[test]
    fn repeated_exact_member_name_remains_ambiguous() {
        let (index, starts, addons) = fixture(
            "class SCR_FirstMode { void OnInit() {} }\n\
             class SCR_SecondMode { void OnInit() {} }\n\
             class SCR_ThirdMode { void OnInit() {} }\n",
        );

        let result = research_game_data(
            &index,
            &starts,
            &addons,
            &IndexBuildControl::default(),
            "revision-1",
            GameDataIntentRequest {
                query: "OnInit".to_string(),
                addon_guids: None,
            },
        )
        .unwrap();

        assert_eq!(result.status, IntentStatus::Ambiguous);
        assert_eq!(result.alternatives.len(), MAX_ALTERNATIVES);
        assert_eq!(result.follow_up, IntentFollowUp::RefineQuery);
    }

    #[test]
    fn natural_language_resolves_a_member_from_its_name_and_owner_terms() {
        let (index, starts, addons) =
            fixture("class SCR_BaseGameMode { void OnGameEnd(int reason, string message) {} void RestartRound() {} }\n");

        let result = research_game_data(
            &index,
            &starts,
            &addons,
            &IndexBuildControl::default(),
            "revision-1",
            GameDataIntentRequest {
                query: "base game mode lifecycle callback when the game ends".to_string(),
                addon_guids: None,
            },
        )
        .unwrap();

        assert_eq!(
            result
                .primary
                .as_ref()
                .map(|symbol| symbol.qualified_name.as_str()),
            Some("SCR_BaseGameMode.OnGameEnd")
        );
        assert!(result
            .primary
            .as_ref()
            .is_some_and(|symbol| symbol.matched_terms.contains(&"end".to_string())));
        assert!(result
            .primary
            .as_ref()
            .is_some_and(|symbol| symbol.signature.contains("int reason")
                && symbol.signature.contains("string message")));
    }

    #[test]
    fn explicit_owner_anchor_returns_only_query_relevant_members() {
        let (index, starts, addons) = fixture(
            "class SCR_FactionManager {\n\
                void UnregisterFactionGroup() {}\n\
                void RegisterFactionGroup() {}\n\
                void RestartSession() {}\n\
            }\n",
        );

        let result = research_game_data(
            &index,
            &starts,
            &addons,
            &IndexBuildControl::default(),
            "revision-1",
            GameDataIntentRequest {
                query: "SCR_FactionManager unregister faction group".to_string(),
                addon_guids: None,
            },
        )
        .unwrap();
        let primary = result.primary.expect("anchored owner");

        assert_eq!(primary.qualified_name, "SCR_FactionManager");
        assert_eq!(
            primary
                .relevant_members
                .iter()
                .map(|member| member.name.as_str())
                .collect::<Vec<_>>(),
            vec!["UnregisterFactionGroup"]
        );
        assert!(primary.relevant_members.len() <= MAX_RELEVANT_MEMBERS);
    }

    #[test]
    fn ambiguous_queries_return_only_two_compact_alternatives() {
        let (index, starts, addons) = fixture(
            "class SCR_FactionGroupManager {}\n\
             class SCR_AIGroupManager {}\n\
             class SCR_PlayerGroupManager {}\n\
             class SCR_GroupManagerBase {}\n",
        );

        let result = research_game_data(
            &index,
            &starts,
            &addons,
            &IndexBuildControl::default(),
            "revision-1",
            GameDataIntentRequest {
                query: "group manager".to_string(),
                addon_guids: None,
            },
        )
        .unwrap();

        assert_eq!(result.status, IntentStatus::Ambiguous);
        assert_eq!(result.alternatives.len(), MAX_ALTERNATIVES);
        assert_eq!(result.follow_up, IntentFollowUp::RefineQuery);
    }

    #[test]
    fn relevant_members_are_ranked_before_the_five_member_bound() {
        let (index, starts, addons) = fixture(
            "class SCR_FactionManager {\n\
                void FactionGroupHelperOne() {}\n\
                void FactionGroupHelperTwo() {}\n\
                void FactionGroupHelperThree() {}\n\
                void FactionGroupHelperFour() {}\n\
                void FactionGroupHelperFive() {}\n\
                void UnregisterFactionGroup() {}\n\
            }\n",
        );

        let result = research_game_data(
            &index,
            &starts,
            &addons,
            &IndexBuildControl::default(),
            "revision-1",
            GameDataIntentRequest {
                query: "SCR_FactionManager unregister faction group".to_string(),
                addon_guids: None,
            },
        )
        .unwrap();

        assert_eq!(
            result.primary.expect("primary").relevant_members[0].name,
            "UnregisterFactionGroup"
        );
    }

    #[test]
    fn original_declaration_precedes_a_modded_duplicate() {
        let (index, starts, addons) = fixture("class SCR_Mode {}\nmodded class SCR_Mode {}\n");

        let result = research_game_data(
            &index,
            &starts,
            &addons,
            &IndexBuildControl::default(),
            "revision-1",
            GameDataIntentRequest {
                query: "SCR_Mode".to_string(),
                addon_guids: None,
            },
        )
        .unwrap();

        assert_eq!(result.status, IntentStatus::Ambiguous);
        assert!(!result
            .primary
            .expect("original primary")
            .modifiers
            .contains(&"modded".to_string()));
    }

    #[test]
    fn no_match_returns_a_small_terminal_shape_without_placeholder_symbols() {
        let (index, starts, addons) = fixture("class SCR_BaseGameMode {}\n");

        let result = research_game_data(
            &index,
            &starts,
            &addons,
            &IndexBuildControl::default(),
            "revision-1",
            GameDataIntentRequest {
                query: "quantum orchard serializer".to_string(),
                addon_guids: None,
            },
        )
        .unwrap();

        assert_eq!(result.status, IntentStatus::NotFound);
        assert!(result.primary.is_none());
        assert!(result.alternatives.is_empty());
        assert_eq!(result.follow_up, IntentFollowUp::RefineQuery);
    }
}
