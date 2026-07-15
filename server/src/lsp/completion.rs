use crate::index::SymbolIndex;
use crate::index_query::{
    EditorCompletionCandidate, EditorCompletionOrigin, EditorTopLevelCompletionMode, IndexQuery,
};
use crate::lexer::{lex, TextSpan, TokenKind};
use crate::lsp::{
    file_index_for_source, offset_for_position, range_for_span, FileIndexAnalysis,
    LspMarkupContent, LspPosition, LspRange,
};
use crate::model::{SourceKind, SymbolKind};
use crate::resolver::{IdentifierContext, ReferenceResolver};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const MAX_COMPLETION_ITEMS: usize = 250;

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

fn layered_external_indexes<'a>(
    workspace_index: Option<&'a SymbolIndex>,
    game_data_index: Option<&'a SymbolIndex>,
) -> Vec<&'a SymbolIndex> {
    workspace_index.into_iter().chain(game_data_index).collect()
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
    completion_report_for_cached_analysis_with_external_indexes(
        source,
        analysis,
        position,
        None,
        external_index,
    )
}

pub(crate) fn completion_report_for_cached_analysis_with_external_indexes(
    source: &str,
    analysis: &FileIndexAnalysis,
    position: LspPosition,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> LspCompletionReport {
    let Some(offset) = offset_for_position(source, position) else {
        return empty_completion_report(analysis.parse_diagnostics);
    };
    completion_report_for_offset(source, analysis, offset, workspace_index, game_data_index)
}

pub(crate) fn completion_debug_markdown(
    report: &LspCompletionReport,
    uri: &str,
    bytes: usize,
    revision: u64,
    external_index_status: &str,
) -> String {
    let mut output = String::new();
    output.push_str("# Reforger Completion Debug\n\n");
    output.push_str("## Request\n\n");
    output.push_str(&format!("- URI: `{}`\n", escape_markdown_cell(uri)));
    output.push_str(&format!("- Bytes: `{bytes}`\n"));
    output.push_str(&format!("- Revision: `{revision}`\n"));
    output.push_str("- Cached Analysis: `true`\n");
    output.push_str(&format!(
        "- External Index Status: `{}`\n",
        escape_markdown_cell(external_index_status)
    ));
    output.push_str(&format!(
        "- Parse Diagnostics: `{}`\n\n",
        report.parse_diagnostics
    ));

    output.push_str("## Completion Context\n\n");
    output.push_str(&format!(
        "- Context: `{}`\n",
        escape_markdown_cell(&report.completion_context)
    ));
    output.push_str(&format!(
        "- Receiver: `{}`\n",
        escape_markdown_cell(report.receiver_text.as_deref().unwrap_or("<none>"))
    ));
    output.push_str(&format!(
        "- Owner Type: `{}`\n",
        escape_markdown_cell(report.owner_type.as_deref().unwrap_or("<none>"))
    ));
    output.push_str(&format!(
        "- Prefix: `{}`\n",
        escape_markdown_cell(&report.prefix)
    ));
    output.push_str(&format!(
        "- Candidate Count: `{}`\n",
        report.candidate_count
    ));
    output.push_str(&format!(
        "- Failure Reason: `{}`\n\n",
        escape_markdown_cell(report.failure_reason.as_deref().unwrap_or("<none>"))
    ));

    output.push_str("## Candidate Sources\n\n");
    if report.source_kind_counts.is_empty() {
        output.push_str("None.\n\n");
    } else {
        output.push_str("| Source Kind | Count |\n");
        output.push_str("| --- | ---: |\n");
        for (kind, count) in &report.source_kind_counts {
            output.push_str(&format!("| `{:?}` | {} |\n", kind, count));
        }
        output.push('\n');
    }

    output.push_str("## Candidate Origins\n\n");
    if report.origin_counts.is_empty() {
        output.push_str("None.\n\n");
    } else {
        output.push_str("| Origin | Count |\n");
        output.push_str("| --- | ---: |\n");
        for (origin, count) in &report.origin_counts {
            output.push_str(&format!(
                "| `{}` | {} |\n",
                escape_markdown_cell(origin),
                count
            ));
        }
        output.push('\n');
    }

    output.push_str("## Timings\n\n");
    output.push_str("| Phase | Milliseconds |\n");
    output.push_str("| --- | ---: |\n");
    output.push_str(&format!(
        "| Context detection | {} |\n",
        report.timings.context_detection.as_millis()
    ));
    output.push_str(&format!(
        "| Receiver inference | {} |\n",
        report.timings.receiver_inference.as_millis()
    ));
    output.push_str(&format!(
        "| Candidate lookup | {} |\n",
        report.timings.candidate_lookup.as_millis()
    ));
    output.push_str(&format!(
        "| Item rendering | {} |\n",
        report.timings.item_rendering.as_millis()
    ));
    output.push_str(&format!(
        "| Total | {} |\n\n",
        report.timings.total.as_millis()
    ));

    output.push_str("## Completion Items\n\n");
    if report.list.items.is_empty() {
        output.push_str("None.\n");
        return output;
    }

    output.push_str(
        "| # | Label | Kind | Detail | Label Details | Insert Text | Sort Text | Docs Preview |\n",
    );
    output.push_str("| ---: | --- | --- | --- | --- | --- | --- | --- |\n");
    for (index, item) in report.list.items.iter().take(50).enumerate() {
        output.push_str(&format!(
            "| {} | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | {} |\n",
            index + 1,
            escape_markdown_cell(&item.label),
            completion_lsp_kind_label(item.kind),
            escape_markdown_cell(item.detail.as_deref().unwrap_or("")),
            escape_markdown_cell(&format_label_details(item.label_details.as_ref())),
            escape_markdown_cell(&item.text_edit.new_text),
            escape_markdown_cell(item.sort_text.as_deref().unwrap_or("")),
            markdown_table_text(
                item.documentation
                    .as_ref()
                    .map(|documentation| documentation.value.as_str())
                    .unwrap_or("")
            )
        ));
    }
    if report.list.items.len() > 50 {
        output.push_str(&format!(
            "|  |  |  |  |  |  |  | +{} more |\n",
            report.list.items.len() - 50
        ));
    }

    output
}

fn completion_report_for_offset(
    source: &str,
    analysis: &FileIndexAnalysis,
    offset: usize,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> LspCompletionReport {
    let total_start = Instant::now();
    let context_start = Instant::now();
    let resolver = ReferenceResolver::new_with_parse_scope_and_external_indexes(
        source,
        &analysis.index,
        &analysis.parse,
        &analysis.scope,
        layered_external_indexes(workspace_index, game_data_index),
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
        let visibility = member_visibility_context(
            receiver_text.as_deref(),
            &owner,
            containing_class_name(&analysis.index, offset).as_deref(),
        );

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
            visibility,
            &analysis.index,
            workspace_index,
            game_data_index,
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
        offset,
        mode,
        analysis,
        &analysis.index,
        workspace_index,
        game_data_index,
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
    visibility: MemberVisibilityContext,
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    mut timings: LspCompletionTimings,
    total_start: Instant,
) -> LspCompletionReport {
    let lookup_start = Instant::now();
    let mut candidates = completion_candidates_for_owner(local_index, owner, receiver_is_static);
    if let Some(external_index) = workspace_index {
        candidates.extend(completion_candidates_for_owner(
            external_index,
            owner,
            receiver_is_static,
        ));
    }
    if let Some(external_index) = game_data_index {
        candidates.extend(completion_candidates_for_owner(
            external_index,
            owner,
            receiver_is_static,
        ));
    }
    let candidates = filter_member_candidates_by_visibility(candidates, visibility);
    let candidates = combine_completion_candidates(candidates);
    timings.candidate_lookup = lookup_start.elapsed();

    let edit_range = range_for_span(source, prefix_span);
    let render_start = Instant::now();
    let (items, source_kind_counts, origin_counts) =
        completion_items_for_candidates(&candidates, edit_range, None);
    let items = cap_completion_items(items);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberVisibilityContext {
    UnqualifiedOrSelf,
    ExternalReceiver,
}

fn member_visibility_context(
    receiver_text: Option<&str>,
    owner: &str,
    containing_class: Option<&str>,
) -> MemberVisibilityContext {
    let Some(receiver) = receiver_text.map(str::trim) else {
        return MemberVisibilityContext::UnqualifiedOrSelf;
    };
    if matches!(receiver, "this" | "super") || containing_class == Some(owner) {
        MemberVisibilityContext::UnqualifiedOrSelf
    } else {
        MemberVisibilityContext::ExternalReceiver
    }
}

fn filter_member_candidates_by_visibility(
    candidates: Vec<EditorCompletionCandidate>,
    visibility: MemberVisibilityContext,
) -> Vec<EditorCompletionCandidate> {
    if visibility == MemberVisibilityContext::UnqualifiedOrSelf {
        return candidates;
    }
    candidates
        .into_iter()
        .filter(|candidate| !is_restricted_member_candidate(candidate))
        .collect()
}

fn is_restricted_member_candidate(candidate: &EditorCompletionCandidate) -> bool {
    candidate
        .display
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier.as_str(), "private" | "protected"))
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
    offset: usize,
    mode: EditorTopLevelCompletionMode,
    analysis: &FileIndexAnalysis,
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    mut timings: LspCompletionTimings,
    total_start: Instant,
) -> LspCompletionReport {
    let lookup_start = Instant::now();
    let mut candidates = Vec::new();
    if mode == EditorTopLevelCompletionMode::Value {
        candidates.extend(scoped_value_completion_candidates(
            analysis, &prefix, offset,
        ));
        if let Some(class_name) = containing_class_name(local_index, offset) {
            for owner in containing_class_completion_owners(
                local_index,
                workspace_index,
                game_data_index,
                &class_name,
            ) {
                candidates.extend(prefixed_candidates(
                    completion_candidates_for_owner(local_index, &owner, false),
                    &prefix,
                ));
                if let Some(external_index) = workspace_index {
                    candidates.extend(prefixed_candidates(
                        completion_candidates_for_owner(external_index, &owner, false),
                        &prefix,
                    ));
                }
                if let Some(external_index) = game_data_index {
                    candidates.extend(prefixed_candidates(
                        completion_candidates_for_owner(external_index, &owner, false),
                        &prefix,
                    ));
                }
            }
        }
    }

    candidates.extend(IndexQuery::new(local_index).completion_top_level_limited(
        &prefix,
        mode,
        remaining_completion_slots(candidates.len()),
    ));
    if let Some(external_index) = workspace_index {
        candidates.extend(
            IndexQuery::new(external_index).completion_top_level_limited(
                &prefix,
                mode,
                remaining_completion_slots(candidates.len()),
            ),
        );
    }
    if let Some(external_index) = game_data_index {
        candidates.extend(
            IndexQuery::new(external_index).completion_top_level_limited(
                &prefix,
                mode,
                remaining_completion_slots(candidates.len()),
            ),
        );
    }
    let candidates = combine_completion_candidates(candidates);
    timings.candidate_lookup = lookup_start.elapsed();

    let edit_range = range_for_span(source, prefix_span);
    let render_start = Instant::now();
    let (mut items, source_kind_counts, mut origin_counts) =
        completion_items_for_candidates(&candidates, edit_range, Some("TopLevel"));
    let mut keyword_items = keyword_completion_items(
        &prefix,
        edit_range,
        mode,
        declaration_keyword_context(source, prefix_span.start),
    );
    if !keyword_items.is_empty() {
        *origin_counts.entry("Keyword".to_string()).or_default() += keyword_items.len();
        keyword_items.extend(items);
        items = keyword_items;
    }
    let items = cap_completion_items(items);
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

fn scoped_value_completion_candidates(
    analysis: &FileIndexAnalysis,
    prefix: &str,
    offset: usize,
) -> Vec<EditorCompletionCandidate> {
    let ids = analysis
        .scope
        .visible_symbols_with_prefix(&analysis.index, prefix, offset);
    IndexQuery::new(&analysis.index).completion_symbols(ids, EditorCompletionOrigin::Direct)
}

fn keyword_completion_items(
    prefix: &str,
    edit_range: LspRange,
    mode: EditorTopLevelCompletionMode,
    declaration_context: bool,
) -> Vec<LspCompletionItem> {
    let mut keywords = match mode {
        EditorTopLevelCompletionMode::Type => TYPE_COMPLETION_KEYWORDS.to_vec(),
        EditorTopLevelCompletionMode::Value => STATEMENT_COMPLETION_KEYWORDS.to_vec(),
    };
    if declaration_context {
        keywords.extend(DECLARATION_COMPLETION_KEYWORDS);
    }
    keywords
        .iter()
        .copied()
        .filter(|keyword| starts_with_ignore_ascii_case(keyword, prefix))
        .map(|keyword| LspCompletionItem {
            label: keyword.to_string(),
            label_details: None,
            kind: 14,
            detail: Some("keyword".to_string()),
            documentation: None,
            sort_text: Some(format!("00:00:000:{keyword}")),
            filter_text: Some(keyword.to_string()),
            insert_text_format: None,
            text_edit: LspTextEdit {
                range: edit_range,
                new_text: keyword.to_string(),
            },
        })
        .collect()
}

const STATEMENT_COMPLETION_KEYWORDS: &[&str] = &[
    "return", "if", "else", "for", "foreach", "while", "do", "switch", "case", "default", "break",
    "continue", "true", "false", "null", "new", "delete", "thread", "this", "super",
];

const DECLARATION_COMPLETION_KEYWORDS: &[&str] = &[
    "static",
    "protected",
    "private",
    "const",
    "ref",
    "out",
    "inout",
    "notnull",
    "override",
    "event",
    "proto",
    "external",
    "native",
    "class",
    "modded",
    "sealed",
    "enum",
    "typedef",
];

const TYPE_COMPLETION_KEYWORDS: &[&str] = &[
    "void", "int", "float", "bool", "string", "vector", "typename", "auto",
];

fn declaration_keyword_context(source: &str, offset: usize) -> bool {
    previous_significant_token_kind(source, offset).is_none_or(|kind| match kind {
        TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Semicolon => true,
        TokenKind::Keyword(keyword) => matches!(
            keyword,
            crate::lexer::Keyword::Class
                | crate::lexer::Keyword::Modded
                | crate::lexer::Keyword::Sealed
                | crate::lexer::Keyword::Typedef
                | crate::lexer::Keyword::Proto
                | crate::lexer::Keyword::External
                | crate::lexer::Keyword::Native
                | crate::lexer::Keyword::Private
                | crate::lexer::Keyword::Protected
                | crate::lexer::Keyword::Static
                | crate::lexer::Keyword::Override
                | crate::lexer::Keyword::Const
                | crate::lexer::Keyword::Ref
                | crate::lexer::Keyword::Out
                | crate::lexer::Keyword::Inout
                | crate::lexer::Keyword::Notnull
                | crate::lexer::Keyword::Autoptr
                | crate::lexer::Keyword::Owned
                | crate::lexer::Keyword::Event
        ),
        _ => false,
    })
}

fn previous_significant_token_kind(source: &str, offset: usize) -> Option<TokenKind> {
    lex(source)
        .into_iter()
        .take_while(|token| token.span.end <= offset)
        .filter(|token| !token.kind.is_trivia())
        .last()
        .map(|token| token.kind)
}

fn containing_class_name(index: &SymbolIndex, offset: usize) -> Option<String> {
    index
        .symbols()
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Class && span_contains(symbol.span, offset))
        .min_by_key(|symbol| symbol.span.len())
        .and_then(|symbol| symbol.name.clone())
}

fn containing_class_completion_owners(
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    class_name: &str,
) -> Vec<String> {
    let indexes = layered_external_indexes(workspace_index, game_data_index);
    let mut owners = Vec::new();
    let mut pending = vec![class_name.to_string()];
    let mut seen = BTreeSet::new();

    while let Some(owner) = pending.pop() {
        if owners.len() >= 32 || !seen.insert(owner.clone()) {
            continue;
        }

        let base = class_base_type(local_index, &owner).or_else(|| {
            indexes
                .iter()
                .find_map(|external_index| class_base_type(external_index, &owner))
        });
        owners.push(owner);

        if let Some(base) = base {
            pending.push(base);
        }
    }

    owners
}

fn class_base_type(index: &SymbolIndex, class_name: &str) -> Option<String> {
    index
        .preferred_classes_by_name(class_name)
        .into_iter()
        .find_map(|id| {
            index
                .symbol(id)
                .and_then(|symbol| symbol.detail.base_type.as_deref())
                .map(str::trim)
                .filter(|base| !base.is_empty())
                .map(str::to_string)
        })
}

fn prefixed_candidates(
    candidates: Vec<EditorCompletionCandidate>,
    prefix: &str,
) -> Vec<EditorCompletionCandidate> {
    candidates
        .into_iter()
        .filter(|candidate| candidate_matches_prefix(candidate, prefix))
        .collect()
}

fn candidate_matches_prefix(candidate: &EditorCompletionCandidate, prefix: &str) -> bool {
    let name = candidate
        .name
        .as_deref()
        .unwrap_or(candidate.display.label.as_str());
    starts_with_ignore_ascii_case(name, prefix)
}

fn span_contains(span: TextSpan, offset: usize) -> bool {
    offset >= span.start && offset <= span.end
}

fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
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

fn cap_completion_items(mut items: Vec<LspCompletionItem>) -> Vec<LspCompletionItem> {
    if items.len() > MAX_COMPLETION_ITEMS {
        items.truncate(MAX_COMPLETION_ITEMS);
    }
    items
}

fn remaining_completion_slots(current_len: usize) -> usize {
    MAX_COMPLETION_ITEMS.saturating_sub(current_len)
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
        kind: completion_item_kind(candidate),
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

fn completion_item_kind(candidate: &EditorCompletionCandidate) -> u32 {
    match candidate.kind {
        SymbolKind::Method | SymbolKind::Destructor => 2,
        SymbolKind::Function => 3,
        SymbolKind::Constructor => 4,
        SymbolKind::Field if is_constant_completion_candidate(candidate) => 21,
        SymbolKind::Field => 5,
        SymbolKind::GlobalField if is_constant_completion_candidate(candidate) => 21,
        SymbolKind::GlobalField | SymbolKind::LocalVariable | SymbolKind::Parameter => 6,
        SymbolKind::Class => 7,
        SymbolKind::Enum => 13,
        SymbolKind::EnumMember => 20,
        SymbolKind::PreprocessorMacro => 21,
        SymbolKind::Typedef | SymbolKind::TypeParameter => 25,
    }
}

fn is_constant_completion_candidate(candidate: &EditorCompletionCandidate) -> bool {
    candidate
        .display
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier.as_str(), "const"))
}

fn completion_lsp_kind_label(kind: u32) -> &'static str {
    match kind {
        2 => "Method",
        3 => "Function",
        4 => "Constructor",
        5 => "Field",
        6 => "Variable",
        7 => "Class",
        13 => "Enum",
        14 => "Keyword",
        20 => "EnumMember",
        21 => "Constant",
        25 => "TypeParameter",
        _ => "Property",
    }
}

fn format_label_details(details: Option<&LspCompletionItemLabelDetails>) -> String {
    let Some(details) = details else {
        return String::new();
    };
    match (details.detail.as_deref(), details.description.as_deref()) {
        (Some(detail), Some(description)) => format!("{detail} {description}"),
        (Some(detail), None) => detail.to_string(),
        (None, Some(description)) => description.to_string(),
        (None, None) => String::new(),
    }
}

fn escape_markdown_cell(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('|', "\\|")
        .replace('\r', "")
        .replace('\n', " ")
}

fn markdown_table_text(value: &str) -> String {
    escape_markdown_cell(value.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_range() -> LspRange {
        LspRange {
            start: LspPosition {
                line: 0,
                character: 0,
            },
            end: LspPosition {
                line: 0,
                character: 5,
            },
        }
    }

    #[test]
    fn declaration_keywords_are_available_in_type_mode_at_declaration_boundaries() {
        let items = keyword_completion_items(
            "overr",
            test_range(),
            EditorTopLevelCompletionMode::Type,
            true,
        );

        let first = items.first().unwrap();
        assert_eq!(first.label, "override");
        assert_eq!(first.kind, 14);
        assert_eq!(first.text_edit.new_text, "override");
    }

    #[test]
    fn declaration_keywords_stay_out_of_type_mode_without_declaration_boundary() {
        let items = keyword_completion_items(
            "overr",
            test_range(),
            EditorTopLevelCompletionMode::Type,
            false,
        );

        assert!(items.is_empty());
    }
}
