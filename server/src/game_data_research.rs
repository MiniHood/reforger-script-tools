use crate::ast::{AstSourceFile, Expression};
use crate::game_data_inspection::{resolve_symbol_ref, GameDataInspectionError};
use crate::game_data_search::{
    compact_signature, documentation_summary, encode_symbol_ref, kind_name, logical_path,
    owner_name, qualify, ReadSourceInput, SourceLineRange, SourceLineStarts, MAX_CURSOR_BYTES,
    MAX_LIMIT,
};
use crate::index::{GlobalSymbolId, SourceFileId, SymbolIndex};
use crate::index_build::IndexBuildControl;
use crate::lexer::{lex, TextSpan, TokenKind};
use crate::model::{SourceCategory, SymbolCatalog, SymbolKind};
use crate::parser::parse_source;
use crate::resolver::{
    callable_override_key, CandidateSource, ReferenceResolution, ReferenceResolver,
    ResolutionReason,
};
use crate::syntax::{SyntaxElement, SyntaxKind, SyntaxNode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

const DEFAULT_LIMIT: usize = 20;
const EXAMPLE_CONTEXT_BEFORE: usize = 4;
const EXAMPLE_CONTEXT_AFTER: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDataExampleSearchRequest {
    pub topic: String,
    pub subtopic: Option<String>,
    pub source_kinds: Option<Vec<String>>,
    pub source_categories: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameDataExamplePage {
    pub source: String,
    pub catalogue_revision: String,
    pub topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtopic: Option<String>,
    pub source_kinds: Vec<String>,
    pub source_categories: Vec<String>,
    pub returned: usize,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub verification_guidance: String,
    pub results: Vec<GameDataExampleHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameDataExampleHit {
    pub topic: String,
    pub subtopics: Vec<String>,
    pub source_kind: String,
    pub source_category: String,
    pub relative_path: String,
    pub evidence_terms: Vec<String>,
    pub evidence_symbols: Vec<String>,
    pub evidence_line: usize,
    pub line_range: SourceLineRange,
    pub read_source_input: ExampleReadSourceInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExampleReadSourceInput {
    pub catalogue_revision: String,
    pub relative_path: String,
    pub start_line: usize,
    pub line_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDataMemberRequest {
    pub symbol_ref: String,
    pub kinds: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameDataMemberPage {
    pub source: String,
    pub catalogue_revision: String,
    pub owner_symbol_ref: String,
    pub kinds: Vec<String>,
    pub returned: usize,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub results: Vec<GameDataMemberHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameDataMemberHit {
    pub symbol_ref: String,
    pub name: String,
    pub kind: String,
    pub qualified_name: String,
    pub signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_summary: Option<String>,
    pub source_category: String,
    pub relative_path: String,
    pub declaration_range: SourceLineRange,
    pub selection_range: SourceLineRange,
    pub inspect_input: crate::game_data_search::InspectInput,
    pub read_source_input: ReadSourceInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDataRelationshipRequest {
    pub symbol_ref: String,
    pub relationship_kinds: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameDataRelationshipPage {
    pub source: String,
    pub catalogue_revision: String,
    pub target_symbol_ref: String,
    pub relationship_kinds: Vec<String>,
    pub returned: usize,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub results: Vec<GameDataRelationshipHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameDataRelationshipHit {
    pub relationship_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub relative_path: String,
    pub range: SourceLineRange,
    pub evidence: String,
    pub read_source_input: ReadSourceInput,
}

#[derive(Debug)]
pub enum GameDataResearchError {
    InvalidRequest(&'static str),
    InvalidCursor,
    StaleCursor,
    Inspection(GameDataInspectionError),
    Cancelled,
}

impl fmt::Display for GameDataResearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => formatter.write_str(message),
            Self::InvalidCursor => formatter.write_str("invalid cursor"),
            Self::StaleCursor => formatter.write_str("stale cursor"),
            Self::Inspection(error) => write!(formatter, "{error:?}"),
            Self::Cancelled => formatter.write_str("request cancelled"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResearchCursor {
    version: u8,
    operation: String,
    catalogue_revision: String,
    binding: String,
    filters: Vec<String>,
    offset: usize,
}

pub fn search_examples(
    index: &SymbolIndex,
    sources: &BTreeMap<SourceFileId, Arc<str>>,
    starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    control: &IndexBuildControl,
    revision: &str,
    request: GameDataExampleSearchRequest,
) -> Result<GameDataExamplePage, GameDataResearchError> {
    let topic = normalized_required(&request.topic, "topic must be non-empty")?;
    let subtopic = request
        .subtopic
        .as_deref()
        .map(|value| normalized_required(value, "subtopic must be non-empty"))
        .transpose()?;
    let source_kinds = canonical_values(
        request.source_kinds.as_deref(),
        &["generated", "handwritten"],
        "sourceKinds must contain unique generated or handwritten values",
    )?;
    let source_categories = canonical_values(
        request.source_categories.as_deref(),
        SourceCategory::GAME_DATA_FILTER_VALUES,
        "sourceCategories must contain unique catalogue categories",
    )?;
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let filters = example_filters(subtopic.as_deref(), &source_kinds, &source_categories);
    let offset = cursor_offset(
        request.cursor.as_deref(),
        "examples",
        revision,
        &topic,
        &filters,
    )?;
    validate_example_topic(&topic, subtopic.as_deref())?;
    let terms = evidence_terms(&topic, subtopic.as_deref());
    let indexed_names = index
        .symbols()
        .iter()
        .filter_map(|symbol| symbol.name.as_deref())
        .collect::<BTreeSet<_>>();
    let mut candidates = Vec::<(i32, String, SourceFileId, Vec<String>, Vec<String>, usize)>::new();
    for file in index.files() {
        control
            .check()
            .map_err(|_| GameDataResearchError::Cancelled)?;
        let source_kind = example_source_kind(file);
        let category = file.metadata.category.as_str().to_string();
        if source_kinds
            .binary_search_by(|candidate| candidate.as_str().cmp(source_kind))
            .is_err()
            || source_categories.binary_search(&category).is_err()
        {
            continue;
        }
        let Some(source) = sources.get(&file.id) else {
            continue;
        };
        let line_starts = starts.get(&file.id).cloned().unwrap_or_default();
        let code_tokens = code_tokens(source, &line_starts);
        let mut hits = terms
            .iter()
            .filter(|term| code_contains_term(&code_tokens, term))
            .cloned()
            .collect::<Vec<_>>();
        hits.sort();
        hits.dedup();
        if !matches_example_topic(&topic, subtopic.as_deref(), &hits) {
            continue;
        }
        let evidence_line = best_code_evidence_line(&code_tokens, &hits).unwrap_or(1);
        let mut symbols = hits
            .iter()
            .flat_map(|term| term.split('.'))
            .filter(|name| indexed_names.contains(name))
            .map(str::to_string)
            .collect::<Vec<_>>();
        symbols.sort();
        symbols.dedup();
        let rank = example_score(source_kind, &hits, source.lines().count());
        candidates.push((
            rank,
            logical_path(file),
            file.id,
            hits,
            symbols,
            evidence_line,
        ));
    }
    candidates.sort_by(|left, right| {
        (std::cmp::Reverse(left.0), &left.1).cmp(&(std::cmp::Reverse(right.0), &right.1))
    });
    let total = candidates.len();
    let selected = candidates
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let returned = selected.len();
    let next_cursor = (offset + returned < total).then(|| {
        encode_cursor(&ResearchCursor {
            version: 1,
            operation: "examples".to_string(),
            catalogue_revision: revision.to_string(),
            binding: topic.clone(),
            filters: filters.clone(),
            offset: offset + returned,
        })
    });
    let results = selected
        .into_iter()
        .map(
            |(_, path, file_id, evidence_terms, evidence_symbols, evidence_line)| {
                let line_count = sources
                    .get(&file_id)
                    .map(|source| source.lines().count().max(1))
                    .unwrap_or(1);
                let start_line = evidence_line.saturating_sub(EXAMPLE_CONTEXT_BEFORE).max(1);
                let end_line = (evidence_line + EXAMPLE_CONTEXT_AFTER).min(line_count);
                GameDataExampleHit {
                    topic: topic.clone(),
                    subtopics: subtopic.clone().into_iter().collect(),
                    source_kind: example_source_kind(
                        index.file(file_id).expect("candidate file exists"),
                    )
                    .to_string(),
                    source_category: index
                        .file(file_id)
                        .expect("candidate file exists")
                        .metadata
                        .category
                        .as_str()
                        .to_string(),
                    relative_path: path.clone(),
                    evidence_terms,
                    evidence_symbols,
                    evidence_line,
                    line_range: SourceLineRange {
                        start_line,
                        end_line,
                    },
                    read_source_input: ExampleReadSourceInput {
                        catalogue_revision: revision.to_string(),
                        relative_path: path,
                        start_line,
                        line_count: end_line - start_line + 1,
                    },
                }
            },
        )
        .collect();
    let _ = starts;
    Ok(GameDataExamplePage {
        source: "evidence-catalogue".to_string(),
        catalogue_revision: revision.to_string(),
        topic,
        subtopic,
        source_kinds,
        source_categories,
        returned,
        total,
        next_cursor,
        verification_guidance: verification_guidance(request.subtopic.as_deref()),
        results,
    })
}

pub fn list_members(
    index: &SymbolIndex,
    starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    control: &IndexBuildControl,
    revision: &str,
    request: GameDataMemberRequest,
) -> Result<GameDataMemberPage, GameDataResearchError> {
    let owner = resolve_symbol_ref(index, control, revision, &request.symbol_ref)
        .map_err(GameDataResearchError::Inspection)?;
    let kinds = canonical_member_kinds(request.kinds.as_deref())?;
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = cursor_offset(
        request.cursor.as_deref(),
        "members",
        revision,
        &request.symbol_ref,
        &kinds,
    )?;
    let mut members = index
        .children(owner)
        .iter()
        .filter_map(|id| index.symbol(*id).map(|symbol| (*id, symbol)))
        .filter(|(_, symbol)| {
            kinds
                .binary_search(&kind_name(symbol.kind).to_string())
                .is_ok()
        })
        .collect::<Vec<_>>();
    members.sort_by_key(|(_, symbol)| symbol.selection_span.start);
    let total = members.len();
    let selected = members
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let returned = selected.len();
    let next_cursor = (offset + returned < total).then(|| {
        encode_cursor(&ResearchCursor {
            version: 1,
            operation: "members".to_string(),
            catalogue_revision: revision.to_string(),
            binding: request.symbol_ref.clone(),
            filters: kinds.clone(),
            offset: offset + returned,
        })
    });
    let results = selected
        .into_iter()
        .map(|(id, symbol)| project_member(index, starts, revision, id, symbol))
        .collect();
    Ok(GameDataMemberPage {
        source: "language-engine".to_string(),
        catalogue_revision: revision.to_string(),
        owner_symbol_ref: request.symbol_ref,
        kinds,
        returned,
        total,
        next_cursor,
        results,
    })
}

pub fn query_relationships(
    index: &SymbolIndex,
    sources: &BTreeMap<SourceFileId, Arc<str>>,
    starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    control: &IndexBuildControl,
    revision: &str,
    request: GameDataRelationshipRequest,
) -> Result<GameDataRelationshipPage, GameDataResearchError> {
    let target = resolve_symbol_ref(index, control, revision, &request.symbol_ref)
        .map_err(GameDataResearchError::Inspection)?;
    let kinds = canonical_relationship_kinds(request.relationship_kinds.as_deref())?;
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = cursor_offset(
        request.cursor.as_deref(),
        "relationships",
        revision,
        &request.symbol_ref,
        &kinds,
    )?;
    let mut results = Vec::new();
    collect_structural_relationships(
        index,
        starts,
        control,
        revision,
        target,
        &kinds,
        &mut results,
    )?;
    if kinds
        .iter()
        .any(|kind| matches!(kind.as_str(), "reference" | "caller"))
    {
        collect_resolved_usages(
            index,
            sources,
            starts,
            control,
            revision,
            target,
            &kinds,
            &mut results,
        )?;
    }
    sort_relationship_results(&mut results, control)?;
    deduplicate_relationship_results(&mut results, control)?;
    let total = results.len();
    let results = results
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let returned = results.len();
    let next_cursor = (offset + returned < total).then(|| {
        encode_cursor(&ResearchCursor {
            version: 1,
            operation: "relationships".to_string(),
            catalogue_revision: revision.to_string(),
            binding: request.symbol_ref.clone(),
            filters: kinds.clone(),
            offset: offset + returned,
        })
    });
    Ok(GameDataRelationshipPage {
        source: "language-engine".to_string(),
        catalogue_revision: revision.to_string(),
        target_symbol_ref: request.symbol_ref,
        relationship_kinds: kinds,
        returned,
        total,
        next_cursor,
        results,
    })
}

fn project_member(
    index: &SymbolIndex,
    starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    revision: &str,
    id: GlobalSymbolId,
    symbol: &crate::index::IndexedSymbol,
) -> GameDataMemberHit {
    let file = index.file(id.file_id).expect("member file exists");
    let name = symbol.name.clone().unwrap_or_default();
    let qualified_name = qualify(owner_name(index, symbol).as_deref(), &name);
    let path = logical_path(file);
    let lines = starts.get(&id.file_id).cloned().unwrap_or_default();
    let symbol_ref = encode_symbol_ref(
        revision,
        &path,
        kind_name(symbol.kind),
        &qualified_name,
        symbol.selection_span.start,
    );
    GameDataMemberHit {
        inspect_input: crate::game_data_search::InspectInput {
            symbol_ref: symbol_ref.clone(),
        },
        read_source_input: ReadSourceInput {
            catalogue_revision: revision.to_string(),
            relative_path: path.clone(),
            start_line: lines.range(symbol.span.start, symbol.span.end).start_line,
        },
        symbol_ref,
        name,
        kind: kind_name(symbol.kind).to_string(),
        qualified_name: qualified_name.clone(),
        signature: index
            .callable_signature(id)
            .unwrap_or_else(|| compact_signature(symbol, &qualified_name)),
        documentation_summary: documentation_summary(symbol),
        source_category: file.metadata.category.as_str().to_string(),
        relative_path: path,
        declaration_range: lines.range(symbol.span.start, symbol.span.end),
        selection_range: lines.range(symbol.selection_span.start, symbol.selection_span.end),
    }
}

fn collect_structural_relationships(
    index: &SymbolIndex,
    starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    control: &IndexBuildControl,
    revision: &str,
    target: GlobalSymbolId,
    requested: &[String],
    results: &mut Vec<GameDataRelationshipHit>,
) -> Result<(), GameDataResearchError> {
    check_research_control(control)?;
    let Some(symbol) = index.symbol(target) else {
        return Ok(());
    };
    if symbol.kind == SymbolKind::Class {
        if requested.iter().any(|kind| kind == "directBase") {
            if let Some(base) = symbol
                .detail
                .base_type
                .as_deref()
                .and_then(|name| unique_preferred_class(index, name))
            {
                results.push(project_declaration_relationship(
                    index,
                    starts,
                    revision,
                    "directBase",
                    base,
                    "indexed class base type",
                ));
            }
        }
        if requested.iter().any(|kind| kind == "derivedType") {
            let Some(name) = symbol.name.as_deref() else {
                return Ok(());
            };
            for candidate in index.symbols() {
                check_research_control(control)?;
                if candidate.kind != SymbolKind::Class
                    || candidate.detail.base_type.as_deref() != Some(name)
                    || unique_preferred_class(index, name) != Some(target)
                {
                    continue;
                }
                results.push(project_declaration_relationship(
                    index,
                    starts,
                    revision,
                    "derivedType",
                    candidate.id,
                    "indexed class base type",
                ));
            }
        }
    }
    if symbol.kind == SymbolKind::Method {
        collect_override_relationships(
            index, starts, control, revision, target, requested, results,
        )?;
    }
    Ok(())
}

fn collect_override_relationships(
    index: &SymbolIndex,
    starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    control: &IndexBuildControl,
    revision: &str,
    target: GlobalSymbolId,
    requested: &[String],
    results: &mut Vec<GameDataRelationshipHit>,
) -> Result<(), GameDataResearchError> {
    check_research_control(control)?;
    let Some(target_symbol) = index.symbol(target) else {
        return Ok(());
    };
    let Some(target_name) = target_symbol.name.as_deref() else {
        return Ok(());
    };
    let Some(target_owner) = owner_name(index, target_symbol) else {
        return Ok(());
    };
    let target_key = callable_override_key(index, target);
    if requested.iter().any(|kind| kind == "override")
        || (requested.iter().any(|kind| kind == "implementation")
            && target_symbol.callable_form != Some(crate::model::CallableForm::Implementation))
    {
        for candidate in index.symbols() {
            check_research_control(control)?;
            if candidate.kind != SymbolKind::Method
                || candidate.name.as_deref() != Some(target_name)
                || !candidate
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "override")
                || callable_override_key(index, candidate.id) != target_key
            {
                continue;
            }
            let Some(owner) = owner_name(index, candidate) else {
                continue;
            };
            if !class_derives_from(index, control, &owner, &target_owner)? {
                continue;
            }
            let mut relationship_kinds = Vec::new();
            if requested.iter().any(|kind| kind == "override") {
                relationship_kinds.push("override");
            }
            if requested.iter().any(|kind| kind == "implementation")
                && target_symbol.callable_form != Some(crate::model::CallableForm::Implementation)
                && candidate.callable_form == Some(crate::model::CallableForm::Implementation)
            {
                relationship_kinds.push("implementation");
            }
            for relationship_kind in relationship_kinds {
                results.push(project_declaration_relationship(
                    index,
                    starts,
                    revision,
                    relationship_kind,
                    candidate.id,
                    "override modifier, callable shape, and indexed inheritance",
                ));
            }
        }
    }
    if requested.iter().any(|kind| kind == "overriddenDeclaration")
        && target_symbol
            .modifiers
            .iter()
            .any(|modifier| modifier == "override")
    {
        let mut owner = target_owner;
        while let Some(base) = unique_base_class(index, &owner) {
            check_research_control(control)?;
            let Some(base_name) = index.symbol(base).and_then(|symbol| symbol.name.as_deref())
            else {
                break;
            };
            if let Some(candidate) = index
                .methods_by_owner_name(base_name, target_name)
                .iter()
                .copied()
                .find(|candidate| callable_override_key(index, *candidate) == target_key)
            {
                results.push(project_declaration_relationship(
                    index,
                    starts,
                    revision,
                    "overriddenDeclaration",
                    candidate,
                    "override modifier, callable shape, and indexed inheritance",
                ));
                break;
            }
            owner = base_name.to_string();
        }
    }
    Ok(())
}

fn collect_resolved_usages(
    index: &SymbolIndex,
    sources: &BTreeMap<SourceFileId, Arc<str>>,
    starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    control: &IndexBuildControl,
    revision: &str,
    target: GlobalSymbolId,
    requested: &[String],
    results: &mut Vec<GameDataRelationshipHit>,
) -> Result<(), GameDataResearchError> {
    let Some(target_symbol) = index.symbol(target) else {
        return Ok(());
    };
    let Some(target_name) = target_symbol.name.as_deref() else {
        return Ok(());
    };
    for file in index.files() {
        control
            .check()
            .map_err(|_| GameDataResearchError::Cancelled)?;
        let Some(source) = sources.get(&file.id) else {
            continue;
        };
        if !source.contains(target_name) {
            continue;
        }
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let catalog = SymbolCatalog::from_ast_with_metadata(source, &ast, file.metadata.clone());
        let local = SymbolIndex::from_catalogs([&catalog]);
        control
            .check()
            .map_err(|_| GameDataResearchError::Cancelled)?;
        let resolver = ReferenceResolver::new_with_parse(source, &local, &parse, Some(index));
        for token in lex(source).into_iter().filter(|token| {
            token.kind == TokenKind::Identifier
                && source
                    .get(token.span.start..token.span.end)
                    .is_some_and(|text| text == target_name)
        }) {
            control
                .check()
                .map_err(|_| GameDataResearchError::Cancelled)?;
            let Some(resolution) = resolver.resolve_at_offset(token.span.start) else {
                continue;
            };
            let Some(selected) = resolution.selected.as_ref() else {
                continue;
            };
            if resolution.reason == ResolutionReason::DeclarationHit
                || !resolution_uniquely_selects_target(index, file.id, &local, &resolution, target)
                || !selected_matches_target(index, file.id, &local, selected, target)
            {
                continue;
            }
            let range = starts
                .get(&file.id)
                .cloned()
                .unwrap_or_default()
                .range(token.span.start, token.span.end);
            let path = logical_path(file);
            if requested.iter().any(|kind| kind == "reference") {
                results.push(GameDataRelationshipHit {
                    relationship_kind: "reference".to_string(),
                    symbol_ref: None,
                    name: Some(target_name.to_string()),
                    kind: None,
                    qualified_name: None,
                    signature: None,
                    relative_path: path.clone(),
                    range: range.clone(),
                    evidence: resolution.reason.as_str().to_string(),
                    read_source_input: ReadSourceInput {
                        catalogue_revision: revision.to_string(),
                        relative_path: path.clone(),
                        start_line: range.start_line,
                    },
                });
            }
            if requested.iter().any(|kind| kind == "caller")
                && matches!(
                    target_symbol.kind,
                    SymbolKind::Function | SymbolKind::Method
                )
                && is_call_callee(&parse.root, source, token.span)
            {
                if let Some(caller) = containing_callable(&local, token.span) {
                    if let Some(global_caller) = map_local_symbol(index, file.id, &local, caller) {
                        let mut hit = project_declaration_relationship(
                            index,
                            starts,
                            revision,
                            "caller",
                            global_caller,
                            resolution.reason.as_str(),
                        );
                        hit.range = range.clone();
                        hit.read_source_input.start_line = range.start_line;
                        results.push(hit);
                    }
                }
            }
        }
    }
    Ok(())
}

fn resolution_uniquely_selects_target(
    global: &SymbolIndex,
    file_id: SourceFileId,
    local: &SymbolIndex,
    resolution: &ReferenceResolution,
    target: GlobalSymbolId,
) -> bool {
    let candidates = resolution
        .candidates
        .iter()
        .filter_map(|candidate| match candidate.source {
            CandidateSource::External => Some(candidate.id),
            CandidateSource::FileLocal => map_local_symbol(global, file_id, local, candidate.id),
        })
        .collect::<BTreeSet<_>>();
    candidates.len() == 1 && candidates.contains(&target)
}

fn is_call_callee(root: &SyntaxNode, source: &str, span: TextSpan) -> bool {
    if root.kind == SyntaxKind::CallExpression {
        if let Some(Expression::Call(call)) = Expression::from_node(source, root) {
            if call
                .callee()
                .is_some_and(|callee| contains_span(callee.span(), span))
            {
                return true;
            }
        }
    }
    root.children.iter().any(|child| match child {
        SyntaxElement::Node(child) if contains_span(child.span, span) => {
            is_call_callee(child, source, span)
        }
        _ => false,
    })
}

fn contains_span(container: TextSpan, contained: TextSpan) -> bool {
    container.start <= contained.start && contained.end <= container.end
}

fn selected_matches_target(
    global: &SymbolIndex,
    file_id: SourceFileId,
    local: &SymbolIndex,
    selected: &crate::resolver::ReferenceCandidate,
    target: GlobalSymbolId,
) -> bool {
    match selected.source {
        CandidateSource::External => selected.id == target,
        CandidateSource::FileLocal => {
            map_local_symbol(global, file_id, local, selected.id) == Some(target)
        }
    }
}

fn map_local_symbol(
    global: &SymbolIndex,
    file_id: SourceFileId,
    local: &SymbolIndex,
    local_id: GlobalSymbolId,
) -> Option<GlobalSymbolId> {
    let symbol = local.symbol(local_id)?;
    global
        .symbols_for_file(global.file(file_id)?)
        .iter()
        .find(|candidate| {
            candidate.kind == symbol.kind
                && candidate.name == symbol.name
                && candidate.selection_span == symbol.selection_span
        })
        .map(|candidate| candidate.id)
}

fn containing_callable(index: &SymbolIndex, span: TextSpan) -> Option<GlobalSymbolId> {
    index
        .symbols()
        .iter()
        .filter(|symbol| {
            matches!(
                symbol.kind,
                SymbolKind::Function
                    | SymbolKind::Method
                    | SymbolKind::Constructor
                    | SymbolKind::Destructor
            ) && symbol.span.start <= span.start
                && span.end <= symbol.span.end
        })
        .min_by_key(|symbol| symbol.span.len())
        .map(|symbol| symbol.id)
}

fn project_declaration_relationship(
    index: &SymbolIndex,
    starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    revision: &str,
    relationship_kind: &str,
    id: GlobalSymbolId,
    evidence: &str,
) -> GameDataRelationshipHit {
    let symbol = index.symbol(id).expect("relationship symbol exists");
    let file = index.file(id.file_id).expect("relationship file exists");
    let name = symbol.name.clone().unwrap_or_default();
    let qualified = qualify(owner_name(index, symbol).as_deref(), &name);
    let path = logical_path(file);
    let lines = starts.get(&id.file_id).cloned().unwrap_or_default();
    let range = lines.range(symbol.selection_span.start, symbol.selection_span.end);
    GameDataRelationshipHit {
        relationship_kind: relationship_kind.to_string(),
        symbol_ref: Some(encode_symbol_ref(
            revision,
            &path,
            kind_name(symbol.kind),
            &qualified,
            symbol.selection_span.start,
        )),
        name: Some(name),
        kind: Some(kind_name(symbol.kind).to_string()),
        qualified_name: Some(qualified.clone()),
        signature: Some(
            index
                .callable_signature(id)
                .unwrap_or_else(|| compact_signature(symbol, &qualified)),
        ),
        relative_path: path.clone(),
        range: range.clone(),
        evidence: evidence.to_string(),
        read_source_input: ReadSourceInput {
            catalogue_revision: revision.to_string(),
            relative_path: path,
            start_line: range.start_line,
        },
    }
}

fn unique_preferred_class(index: &SymbolIndex, name: &str) -> Option<GlobalSymbolId> {
    let preferred = index.preferred_classes_by_name(name);
    (preferred.len() == 1).then_some(preferred[0])
}

fn unique_base_class(index: &SymbolIndex, owner: &str) -> Option<GlobalSymbolId> {
    let owner = unique_preferred_class(index, owner)?;
    let base = index.symbol(owner)?.detail.base_type.as_deref()?;
    unique_preferred_class(index, base)
}

fn class_derives_from(
    index: &SymbolIndex,
    control: &IndexBuildControl,
    owner: &str,
    expected_base: &str,
) -> Result<bool, GameDataResearchError> {
    let mut current = owner.to_string();
    let mut visited = BTreeSet::new();
    while visited.insert(current.clone()) {
        check_research_control(control)?;
        let Some(base) = unique_base_class(index, &current) else {
            return Ok(false);
        };
        let Some(name) = index.symbol(base).and_then(|symbol| symbol.name.as_deref()) else {
            return Ok(false);
        };
        if name == expected_base {
            return Ok(true);
        }
        current = name.to_string();
    }
    Ok(false)
}

fn check_research_control(control: &IndexBuildControl) -> Result<(), GameDataResearchError> {
    control
        .check()
        .map_err(|_| GameDataResearchError::Cancelled)
}

fn compare_relationship_hits(
    left: &GameDataRelationshipHit,
    right: &GameDataRelationshipHit,
) -> std::cmp::Ordering {
    (
        &left.relationship_kind,
        &left.relative_path,
        left.range.start_line,
        &left.qualified_name,
    )
        .cmp(&(
            &right.relationship_kind,
            &right.relative_path,
            right.range.start_line,
            &right.qualified_name,
        ))
}

fn sort_relationship_results(
    results: &mut [GameDataRelationshipHit],
    control: &IndexBuildControl,
) -> Result<(), GameDataResearchError> {
    const RUN_SIZE: usize = 128;
    for run in results.chunks_mut(RUN_SIZE) {
        check_research_control(control)?;
        run.sort_by(compare_relationship_hits);
    }

    let mut width = RUN_SIZE;
    let mut merged = Vec::with_capacity(results.len());
    while width < results.len() {
        merged.clear();
        for start in (0..results.len()).step_by(width.saturating_mul(2)) {
            check_research_control(control)?;
            let middle = (start + width).min(results.len());
            let end = (start + width.saturating_mul(2)).min(results.len());
            let mut left = start;
            let mut right = middle;
            while left < middle && right < end {
                if merged.len() % RUN_SIZE == 0 {
                    check_research_control(control)?;
                }
                if compare_relationship_hits(&results[left], &results[right]).is_gt() {
                    merged.push(results[right].clone());
                    right += 1;
                } else {
                    merged.push(results[left].clone());
                    left += 1;
                }
            }
            for index in left..middle {
                if merged.len() % RUN_SIZE == 0 {
                    check_research_control(control)?;
                }
                merged.push(results[index].clone());
            }
            for index in right..end {
                if merged.len() % RUN_SIZE == 0 {
                    check_research_control(control)?;
                }
                merged.push(results[index].clone());
            }
        }
        results.clone_from_slice(&merged);
        width = width.saturating_mul(2);
    }
    Ok(())
}

fn deduplicate_relationship_results(
    results: &mut Vec<GameDataRelationshipHit>,
    control: &IndexBuildControl,
) -> Result<(), GameDataResearchError> {
    let mut deduplicated = Vec::with_capacity(results.len());
    for result in results.drain(..) {
        if deduplicated.len() % 128 == 0 {
            check_research_control(control)?;
        }
        let duplicate = deduplicated
            .last()
            .is_some_and(|previous: &GameDataRelationshipHit| {
                previous.relationship_kind == result.relationship_kind
                    && previous.relative_path == result.relative_path
                    && previous.range == result.range
                    && previous.symbol_ref == result.symbol_ref
            });
        if !duplicate {
            deduplicated.push(result);
        }
    }
    *results = deduplicated;
    Ok(())
}

fn example_source_kind(file: &crate::index::IndexedFile) -> &'static str {
    if file.metadata.category.is_generated() {
        "generated"
    } else {
        "handwritten"
    }
}

fn evidence_terms(topic: &str, subtopic: Option<&str>) -> Vec<String> {
    match (topic, subtopic) {
        ("resource-loading", Some("spawn-prefab")) => [
            "Resource.Load",
            "ResourceName",
            "SpawnEntityPrefab",
            "EntitySpawnParams",
            "PrefabResource",
        ],
        ("resource-loading", None) => [
            "Resource.Load",
            "ResourceName",
            "EntitySpawnParams",
            "PrefabResource",
            "SpawnEntityPrefab",
        ],
        _ => unreachable!("validated example topic"),
    }
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn validate_example_topic(
    topic: &str,
    subtopic: Option<&str>,
) -> Result<(), GameDataResearchError> {
    match (topic, subtopic) {
        ("resource-loading", None | Some("spawn-prefab")) => Ok(()),
        ("resource-loading", Some(_)) => Err(GameDataResearchError::InvalidRequest(
            "unsupported subtopic for resource-loading; use spawn-prefab",
        )),
        _ => Err(GameDataResearchError::InvalidRequest(
            "unsupported topic; use resource-loading",
        )),
    }
}

fn matches_example_topic(topic: &str, subtopic: Option<&str>, hits: &[String]) -> bool {
    let has = |term: &str| hits.iter().any(|hit| hit == term);
    let resource_evidence = [
        "Resource.Load",
        "ResourceName",
        "EntitySpawnParams",
        "PrefabResource",
    ]
    .into_iter()
    .any(has);
    match (topic, subtopic) {
        ("resource-loading", Some("spawn-prefab")) => resource_evidence && has("SpawnEntityPrefab"),
        ("resource-loading", None) => resource_evidence,
        _ => false,
    }
}

#[derive(Debug, Clone, Copy)]
struct CodeToken<'source> {
    text: &'source str,
    line: usize,
}

fn code_tokens<'source>(
    source: &'source str,
    line_starts: &SourceLineStarts,
) -> Vec<CodeToken<'source>> {
    lex(source)
        .into_iter()
        .filter(|token| !token.kind.is_trivia())
        .filter_map(|token| {
            source
                .get(token.span.start..token.span.end)
                .map(|text| CodeToken {
                    text,
                    line: line_starts
                        .range(token.span.start, token.span.end)
                        .start_line,
                })
        })
        .collect()
}

fn code_contains_term(tokens: &[CodeToken<'_>], term: &str) -> bool {
    !matching_term_lines(tokens, term).is_empty()
}

fn matching_term_lines(tokens: &[CodeToken<'_>], term: &str) -> Vec<usize> {
    let parts = term.split('.').collect::<Vec<_>>();
    if parts.len() == 1 {
        return tokens
            .iter()
            .filter(|token| token.text.eq_ignore_ascii_case(parts[0]))
            .map(|token| token.line)
            .collect();
    }
    tokens
        .windows(parts.len() * 2 - 1)
        .filter(|window| {
            parts.iter().enumerate().all(|(index, part)| {
                window[index * 2].text.eq_ignore_ascii_case(part)
                    && (index == parts.len() - 1 || window[index * 2 + 1].text == ".")
            })
        })
        .map(|window| window[0].line)
        .collect()
}

fn best_code_evidence_line(tokens: &[CodeToken<'_>], terms: &[String]) -> Option<usize> {
    let mut scores = BTreeMap::<usize, i32>::new();
    for term in terms {
        for line in matching_term_lines(tokens, term) {
            *scores.entry(line).or_default() += evidence_weight(term);
        }
    }
    scores
        .into_iter()
        .max_by_key(|(line, score)| (*score, std::cmp::Reverse(*line)))
        .map(|(line, _)| line)
}

fn example_score(source_kind: &str, terms: &[String], line_count: usize) -> i32 {
    let mut score = terms.iter().map(|term| evidence_weight(term)).sum::<i32>();
    if terms.iter().any(|term| term == "Resource.Load")
        && terms.iter().any(|term| term == "SpawnEntityPrefab")
    {
        score += 20;
    }
    if source_kind == "handwritten" {
        score += 20;
    }
    score - i32::try_from(line_count / 200).unwrap_or(i32::MAX).min(20)
}

fn evidence_weight(term: &str) -> i32 {
    match term {
        "SpawnEntityPrefab" => 20,
        "EntitySpawnParams" => 12,
        "Resource.Load" => 10,
        "PrefabResource" => 8,
        "ResourceName" => 4,
        _ => 1,
    }
}

fn verification_guidance(subtopic: Option<&str>) -> String {
    if subtopic == Some("spawn-prefab") {
        "Verify resource paths and prefab dependencies in Workbench; spawning behavior can differ by world, authority, and server context.".to_string()
    } else {
        "Examples show source-backed implementation patterns. Verify resources and editor wiring in Workbench, then verify authority-sensitive behavior at runtime.".to_string()
    }
}

fn normalized_required(
    value: &str,
    message: &'static str,
) -> Result<String, GameDataResearchError> {
    let value = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if value.is_empty() || value.chars().count() > 256 {
        return Err(GameDataResearchError::InvalidRequest(message));
    }
    Ok(value)
}

fn canonical_values(
    values: Option<&[String]>,
    allowed: &[&str],
    message: &'static str,
) -> Result<Vec<String>, GameDataResearchError> {
    let values = values
        .map(|values| values.to_vec())
        .unwrap_or_else(|| allowed.iter().map(|value| (*value).to_string()).collect());
    if values.is_empty() {
        return Err(GameDataResearchError::InvalidRequest(message));
    }
    let count = values.len();
    let unique = values.into_iter().collect::<BTreeSet<_>>();
    if unique.len() != count
        || unique
            .iter()
            .any(|value| !allowed.contains(&value.as_str()))
    {
        return Err(GameDataResearchError::InvalidRequest(message));
    }
    Ok(unique.into_iter().collect())
}

fn canonical_member_kinds(values: Option<&[String]>) -> Result<Vec<String>, GameDataResearchError> {
    canonical_values(
        values,
        &[
            "class",
            "constructor",
            "destructor",
            "enum",
            "enumMember",
            "field",
            "method",
            "typeParameter",
        ],
        "kinds must contain unique direct-member symbol kinds",
    )
}

fn canonical_relationship_kinds(
    values: Option<&[String]>,
) -> Result<Vec<String>, GameDataResearchError> {
    canonical_values(
        values,
        &[
            "caller",
            "derivedType",
            "directBase",
            "implementation",
            "override",
            "overriddenDeclaration",
            "reference",
        ],
        "relationshipKinds must contain unique supported semantic relationships",
    )
}

fn example_filters(
    subtopic: Option<&str>,
    source_kinds: &[String],
    source_categories: &[String],
) -> Vec<String> {
    let mut filters = vec![subtopic.unwrap_or("").to_string()];
    filters.extend(source_kinds.iter().cloned());
    filters.push("|".to_string());
    filters.extend(source_categories.iter().cloned());
    filters
}

fn cursor_offset(
    value: Option<&str>,
    operation: &str,
    revision: &str,
    binding: &str,
    filters: &[String],
) -> Result<usize, GameDataResearchError> {
    let Some(value) = value else {
        return Ok(0);
    };
    let cursor = decode_cursor(value)?;
    if cursor.catalogue_revision != revision {
        return Err(GameDataResearchError::StaleCursor);
    }
    if cursor.operation != operation || cursor.binding != binding || cursor.filters != filters {
        return Err(GameDataResearchError::InvalidCursor);
    }
    Ok(cursor.offset)
}

fn encode_cursor(cursor: &ResearchCursor) -> String {
    format!(
        "rc1:{}",
        hex(&serde_json::to_vec(cursor).expect("research cursor serializes"))
    )
}

fn decode_cursor(value: &str) -> Result<ResearchCursor, GameDataResearchError> {
    if value.len() > MAX_CURSOR_BYTES {
        return Err(GameDataResearchError::InvalidCursor);
    }
    let encoded = value
        .strip_prefix("rc1:")
        .ok_or(GameDataResearchError::InvalidCursor)?;
    let cursor = serde_json::from_slice::<ResearchCursor>(
        &unhex(encoded).ok_or(GameDataResearchError::InvalidCursor)?,
    )
    .map_err(|_| GameDataResearchError::InvalidCursor)?;
    (cursor.version == 1)
        .then_some(cursor)
        .ok_or(GameDataResearchError::InvalidCursor)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotted_term_matching_supplies_search_and_evidence() {
        let tokens = [
            CodeToken {
                text: "Resource",
                line: 12,
            },
            CodeToken {
                text: ".",
                line: 12,
            },
            CodeToken {
                text: "Load",
                line: 12,
            },
        ];
        let terms = vec!["Resource.Load".to_string()];

        assert!(code_contains_term(&tokens, "Resource.Load"));
        assert_eq!(matching_term_lines(&tokens, "Resource.Load"), vec![12]);
        assert_eq!(best_code_evidence_line(&tokens, &terms), Some(12));
    }

    #[test]
    fn cancelled_relationship_postprocessing_stops_before_work() {
        let control = IndexBuildControl::default();
        control.cancel();
        assert!(matches!(
            collect_structural_relationships(
                &SymbolIndex::default(),
                &BTreeMap::new(),
                &control,
                "gd1:test",
                GlobalSymbolId {
                    file_id: SourceFileId(0),
                    symbol_id: crate::model::SymbolId(0),
                },
                &["derivedType".to_string()],
                &mut Vec::new(),
            ),
            Err(GameDataResearchError::Cancelled)
        ));
        let mut results = vec![GameDataRelationshipHit {
            relationship_kind: "reference".to_string(),
            symbol_ref: None,
            name: None,
            kind: None,
            qualified_name: None,
            signature: None,
            relative_path: "Game/example.c".to_string(),
            range: SourceLineRange {
                start_line: 1,
                end_line: 1,
            },
            evidence: "test".to_string(),
            read_source_input: ReadSourceInput {
                catalogue_revision: "gd1:test".to_string(),
                relative_path: "Game/example.c".to_string(),
                start_line: 1,
            },
        }];

        assert!(matches!(
            sort_relationship_results(&mut results, &control),
            Err(GameDataResearchError::Cancelled)
        ));
        assert!(matches!(
            deduplicate_relationship_results(&mut results, &control),
            Err(GameDataResearchError::Cancelled)
        ));
    }
}
