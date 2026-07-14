use crate::index::SymbolIndex;
use crate::index_query::{
    EditorCompletionCandidate, EditorCompletionOrigin, EditorTopLevelCompletionMode, IndexQuery,
};
use crate::lexer::TextSpan;
use crate::lsp::{
    file_index_for_source, offset_for_position, range_for_span, FileIndexAnalysis,
    LspMarkupContent, LspPosition, LspRange,
};
use crate::model::{SourceKind, SymbolKind};
use crate::resolver::{IdentifierContext, ReferenceResolver};
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspCompletionList {
    pub is_incomplete: bool,
    pub items: Vec<LspCompletionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspCompletionItem {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_details: Option<LspCompletionItemLabelDetails>,
    pub kind: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<LspMarkupContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text_format: Option<u32>,
    pub text_edit: LspTextEdit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspCompletionItemLabelDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspTextEdit {
    pub range: LspRange,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspCompletionReport {
    pub list: LspCompletionList,
    pub parse_diagnostics: usize,
    pub completion_context: String,
    pub receiver_text: Option<String>,
    pub owner_type: Option<String>,
    pub prefix: String,
    pub candidate_count: usize,
    pub source_kind_counts: BTreeMap<SourceKind, usize>,
    pub origin_counts: BTreeMap<String, usize>,
    pub failure_reason: Option<String>,
    pub timings: LspCompletionTimings,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LspCompletionTimings {
    pub context_detection: Duration,
    pub receiver_inference: Duration,
    pub candidate_lookup: Duration,
    pub item_rendering: Duration,
    pub total: Duration,
}

pub fn completion_report_for_source_position_with_external(
    source: &str,
    position: LspPosition,
    external_index: Option<&SymbolIndex>,
) -> LspCompletionReport {
    let analysis = file_index_for_source(source);
    completion_report_for_cached_analysis_with_external(source, &analysis, position, external_index)
}

pub fn completion_report_for_cached_analysis_with_external(
    source: &str,
    analysis: &FileIndexAnalysis,
    position: LspPosition,
    external_index: Option<&SymbolIndex>,
) -> LspCompletionReport {
    let Some(offset) = offset_for_position(source, position) else {
        return empty_completion_report(analysis.parse_diagnostics);
    };
    completion_report_for_offset(source, analysis, offset, external_index)
}

fn completion_report_for_offset(
    source: &str,
    analysis: &FileIndexAnalysis,
    offset: usize,
    external_index: Option<&SymbolIndex>,
) -> LspCompletionReport {
    let total_start = Instant::now();
    let context_start = Instant::now();
    let resolver = ReferenceResolver::new_with_parse_and_scope(
        source,
        &analysis.index,
        &analysis.parse,
        &analysis.scope,
        external_index,
    );
    if let Some(context) = resolver.member_completion_context_at_offset(offset) {
        let context_elapsed = context_start.elapsed();
        let receiver_text = Some(context.receiver.receiver_text.clone());
        let owner_type = context.receiver.owner_type.clone();
        let receiver_is_static = context.receiver.is_static;
        let prefix = context.prefix.clone();
        let failure_reason = context.receiver.failure_reason.clone();
        let Some(owner) = owner_type.clone() else {
            return LspCompletionReport {
                list: empty_completion_list(),
                parse_diagnostics: analysis.parse_diagnostics,
                completion_context: "member".to_string(),
                receiver_text,
                owner_type,
                prefix,
                candidate_count: 0,
                source_kind_counts: BTreeMap::new(),
                origin_counts: BTreeMap::new(),
                failure_reason,
                timings: LspCompletionTimings {
                    context_detection: context_elapsed,
                    receiver_inference: context_elapsed,
                    total: total_start.elapsed(),
                    ..LspCompletionTimings::default()
                },
            };
        };

        return member_completion_report_for_indexes(
            source,
            analysis.parse_diagnostics,
            &owner,
            receiver_text,
            owner_type,
            prefix,
            context.prefix_span,
            failure_reason,
            receiver_is_static,
            &analysis.index,
            external_index,
            LspCompletionTimings {
                context_detection: context_elapsed,
                receiver_inference: context_elapsed,
                ..LspCompletionTimings::default()
            },
            total_start,
        );
    }

    let top_level_context = resolver.top_level_completion_context_at_offset(offset);
    let context_elapsed = context_start.elapsed();
    let Some(context) = top_level_context else {
        let mut report = empty_completion_report(analysis.parse_diagnostics);
        report.timings.context_detection = context_elapsed;
        report.timings.total = total_start.elapsed();
        return report;
    };
    let mode = if context.identifier_context == IdentifierContext::TypePosition {
        EditorTopLevelCompletionMode::Type
    } else {
        EditorTopLevelCompletionMode::Value
    };
    let completion_context = match mode {
        EditorTopLevelCompletionMode::Type => "type",
        EditorTopLevelCompletionMode::Value => "top-level",
    };

    top_level_completion_report_for_indexes(
        source,
        analysis.parse_diagnostics,
        completion_context,
        context.prefix,
        context.prefix_span,
        mode,
        &analysis.index,
        external_index,
        LspCompletionTimings {
            context_detection: context_elapsed,
            ..LspCompletionTimings::default()
        },
        total_start,
    )
}

#[allow(clippy::too_many_arguments)]
fn member_completion_report_for_indexes(
    source: &str,
    parse_diagnostics: usize,
    owner: &str,
    receiver_text: Option<String>,
    owner_type: Option<String>,
    prefix: String,
    prefix_span: TextSpan,
    failure_reason: Option<String>,
    receiver_is_static: bool,
    local_index: &SymbolIndex,
    external_index: Option<&SymbolIndex>,
    mut timings: LspCompletionTimings,
    total_start: Instant,
) -> LspCompletionReport {
    let lookup_start = Instant::now();
    let mut candidates = completion_candidates_for_owner(local_index, owner, receiver_is_static);
    if let Some(external_index) = external_index {
        candidates.extend(completion_candidates_for_owner(
            external_index,
            owner,
            receiver_is_static,
        ));
    }
    let candidates = combine_completion_candidates(candidates);
    timings.candidate_lookup = lookup_start.elapsed();

    let edit_range = range_for_span(source, prefix_span);
    let render_start = Instant::now();
    let (items, source_kind_counts, origin_counts) =
        completion_items_for_candidates(&candidates, edit_range, None);
    timings.item_rendering = render_start.elapsed();
    timings.total = total_start.elapsed();

    LspCompletionReport {
        candidate_count: items.len(),
        list: LspCompletionList {
            is_incomplete: false,
            items,
        },
        parse_diagnostics,
        completion_context: "member".to_string(),
        receiver_text,
        owner_type,
        prefix,
        source_kind_counts,
        origin_counts,
        failure_reason,
        timings,
    }
}

fn completion_candidates_for_owner(
    index: &SymbolIndex,
    owner: &str,
    receiver_is_static: bool,
) -> Vec<EditorCompletionCandidate> {
    let query = IndexQuery::new(index);
    if receiver_is_static {
        query.completion_static_members_for_type(owner)
    } else {
        query.completion_members_for_class(owner).candidates
    }
}

fn completion_items_for_candidates(
    candidates: &[EditorCompletionCandidate],
    edit_range: LspRange,
    origin_override: Option<&str>,
) -> (
    Vec<LspCompletionItem>,
    BTreeMap<SourceKind, usize>,
    BTreeMap<String, usize>,
) {
    let mut source_kind_counts = BTreeMap::new();
    let mut origin_counts = BTreeMap::new();
    let items = candidates
        .iter()
        .filter_map(|candidate| {
            *source_kind_counts.entry(candidate.source_kind).or_default() += 1;
            let origin = origin_override
                .map(str::to_string)
                .unwrap_or_else(|| format!("{:?}", candidate.origin));
            *origin_counts.entry(origin).or_default() += 1;
            completion_item_for_candidate(candidate, edit_range)
        })
        .collect::<Vec<_>>();

    (items, source_kind_counts, origin_counts)
}

#[allow(clippy::too_many_arguments)]
fn top_level_completion_report_for_indexes(
    source: &str,
    parse_diagnostics: usize,
    completion_context: &str,
    prefix: String,
    prefix_span: TextSpan,
    mode: EditorTopLevelCompletionMode,
    local_index: &SymbolIndex,
    external_index: Option<&SymbolIndex>,
    mut timings: LspCompletionTimings,
    total_start: Instant,
) -> LspCompletionReport {
    let lookup_start = Instant::now();
    let mut candidates = IndexQuery::new(local_index).completion_top_level(&prefix, mode);
    if let Some(external_index) = external_index {
        candidates.extend(IndexQuery::new(external_index).completion_top_level(&prefix, mode));
    }
    let candidates = combine_completion_candidates(candidates);
    timings.candidate_lookup = lookup_start.elapsed();

    let edit_range = range_for_span(source, prefix_span);
    let render_start = Instant::now();
    let (items, source_kind_counts, origin_counts) =
        completion_items_for_candidates(&candidates, edit_range, Some("TopLevel"));
    timings.item_rendering = render_start.elapsed();
    timings.total = total_start.elapsed();

    LspCompletionReport {
        candidate_count: items.len(),
        list: LspCompletionList {
            is_incomplete: false,
            items,
        },
        parse_diagnostics,
        completion_context: completion_context.to_string(),
        receiver_text: None,
        owner_type: None,
        prefix,
        source_kind_counts,
        origin_counts,
        failure_reason: None,
        timings,
    }
}

fn combine_completion_candidates(
    candidates: Vec<EditorCompletionCandidate>,
) -> Vec<EditorCompletionCandidate> {
    let mut by_key = BTreeMap::<String, EditorCompletionCandidate>::new();
    let mut order = Vec::<String>::new();
    for candidate in candidates {
        let key = completion_candidate_key(&candidate);
        if !by_key.contains_key(&key) {
            order.push(key.clone());
            by_key.insert(key, candidate);
        }
    }
    order
        .into_iter()
        .filter_map(|key| by_key.remove(&key))
        .collect()
}

fn completion_candidate_key(candidate: &EditorCompletionCandidate) -> String {
    let name = candidate
        .name
        .as_deref()
        .unwrap_or(candidate.display.label.as_str());
    let signature = candidate.signature.as_deref().unwrap_or("");
    format!("{:?}:{name}:{signature}", candidate.kind)
}

pub(crate) fn empty_completion_list() -> LspCompletionList {
    LspCompletionList {
        is_incomplete: false,
        items: Vec::new(),
    }
}

fn empty_completion_report(parse_diagnostics: usize) -> LspCompletionReport {
    LspCompletionReport {
        list: empty_completion_list(),
        parse_diagnostics,
        completion_context: "none".to_string(),
        receiver_text: None,
        owner_type: None,
        prefix: String::new(),
        candidate_count: 0,
        source_kind_counts: BTreeMap::new(),
        origin_counts: BTreeMap::new(),
        failure_reason: None,
        timings: LspCompletionTimings::default(),
    }
}

fn completion_item_for_candidate(
    candidate: &EditorCompletionCandidate,
    edit_range: LspRange,
) -> Option<LspCompletionItem> {
    let label = candidate
        .name
        .clone()
        .or_else(|| Some(candidate.display.label.clone()))?;
    let detail = candidate.signature.clone().or(candidate.detail.clone());
    let label_details = completion_label_details(&label, candidate);
    let new_text = completion_insert_text(&label, candidate);
    let insert_text_format = (new_text != label).then_some(2);
    let documentation = candidate
        .display
        .documentation_preview
        .as_ref()
        .map(|preview| LspMarkupContent {
            kind: "markdown".to_string(),
            value: preview.clone(),
        });

    Some(LspCompletionItem {
        label: label.clone(),
        label_details,
        kind: completion_item_kind(candidate.kind),
        detail,
        documentation,
        sort_text: Some(completion_sort_text(candidate, &label)),
        filter_text: Some(label.clone()),
        insert_text_format,
        text_edit: LspTextEdit {
            range: edit_range,
            new_text,
        },
    })
}

fn completion_label_details(
    label: &str,
    candidate: &EditorCompletionCandidate,
) -> Option<LspCompletionItemLabelDetails> {
    let signature = candidate.signature.as_deref()?;
    let call = callable_signature_parts(label, signature)?;
    Some(LspCompletionItemLabelDetails {
        detail: Some(call.parameters),
        description: call.result,
    })
}

fn completion_insert_text(label: &str, candidate: &EditorCompletionCandidate) -> String {
    if !matches!(
        candidate.kind,
        SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Destructor
    ) {
        return label.to_string();
    }
    let Some(signature) = candidate.signature.as_deref() else {
        return label.to_string();
    };
    let Some(call) = callable_signature_parts(label, signature) else {
        return label.to_string();
    };
    let arguments = call
        .parameter_names
        .into_iter()
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    format!("{label}({arguments})")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallableSignatureParts {
    parameters: String,
    parameter_names: Vec<String>,
    result: Option<String>,
}

fn callable_signature_parts(label: &str, signature: &str) -> Option<CallableSignatureParts> {
    let open = signature.find('(')?;
    let close = matching_close_paren(signature, open)?;
    let prefix = signature[..open].trim();
    if !prefix.ends_with(label) {
        return None;
    }
    let parameters_text = signature[open + 1..close].trim();
    let result = signature[close + 1..]
        .trim()
        .strip_prefix("->")
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| format!("-> {text}"));
    let parameters = format!("({parameters_text})");
    let parameter_names = split_completion_parameters(parameters_text)
        .into_iter()
        .filter_map(|parameter| completion_parameter_name(&parameter))
        .collect();

    Some(CallableSignatureParts {
        parameters,
        parameter_names,
        result,
    })
}

fn matching_close_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in text[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_completion_parameters(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut angle = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    for (offset, ch) in text.char_indices() {
        match ch {
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            '[' => bracket += 1,
            ']' => bracket = bracket.saturating_sub(1),
            ',' if angle == 0 && paren == 0 && bracket == 0 => {
                let part = text[start..offset].trim();
                if !part.is_empty() {
                    parts.push(part.to_string());
                }
                start = offset + ch.len_utf8();
            }
            _ => {}
        }
    }
    let part = text[start..].trim();
    if !part.is_empty() {
        parts.push(part.to_string());
    }
    parts
}

fn completion_parameter_name(parameter: &str) -> Option<String> {
    let before_default = parameter.split('=').next().unwrap_or(parameter).trim();
    let before_array = before_default
        .split('[')
        .next()
        .unwrap_or(before_default)
        .trim();
    let name = before_array
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_');
    (!name.is_empty()).then(|| name.to_string())
}

fn completion_sort_text(candidate: &EditorCompletionCandidate, label: &str) -> String {
    format!(
        "{:02}:{:02}:{:03}:{}",
        completion_source_rank(candidate),
        completion_origin_sort_rank(candidate.origin),
        completion_item_kind_rank(candidate.kind),
        label
    )
}

fn completion_source_rank(candidate: &EditorCompletionCandidate) -> u8 {
    match candidate.source_kind {
        SourceKind::Unknown => 0,
        SourceKind::Workspace => 1,
        SourceKind::GameData => 2,
        SourceKind::Fixture => 3,
    }
}

fn completion_origin_sort_rank(origin: EditorCompletionOrigin) -> u8 {
    match origin {
        EditorCompletionOrigin::Direct => 0,
        EditorCompletionOrigin::Overlay => 1,
        EditorCompletionOrigin::Inherited => 2,
        EditorCompletionOrigin::Unknown => 3,
    }
}

fn completion_item_kind_rank(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Class => 0,
        SymbolKind::Enum => 1,
        SymbolKind::Typedef => 2,
        SymbolKind::Function => 3,
        SymbolKind::Method => 4,
        SymbolKind::Constructor => 5,
        SymbolKind::Destructor => 6,
        SymbolKind::Field | SymbolKind::GlobalField => 7,
        SymbolKind::EnumMember => 8,
        SymbolKind::Parameter | SymbolKind::LocalVariable | SymbolKind::PreprocessorMacro => 9,
        _ => 99,
    }
}

fn completion_item_kind(kind: SymbolKind) -> u32 {
    match kind {
        SymbolKind::Method | SymbolKind::Destructor => 2,
        SymbolKind::Function => 3,
        SymbolKind::Constructor => 4,
        SymbolKind::Field => 5,
        SymbolKind::GlobalField
        | SymbolKind::LocalVariable
        | SymbolKind::Parameter
        | SymbolKind::PreprocessorMacro => 6,
        SymbolKind::Class => 7,
        SymbolKind::Enum => 13,
        SymbolKind::EnumMember => 20,
        SymbolKind::Typedef => 25,
        _ => 10,
    }
}
