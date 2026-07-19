use crate::analysis_runtime::QueryQuality;
use crate::expression_type::expression_type_from_index_symbol;
use crate::index::SymbolIndex;
use crate::index_query::{
    completion_name_match_rank, EditorCompletionCandidate, EditorCompletionOrigin,
    EditorTopLevelCompletionMode, IndexQuery,
};
use crate::lexer::{lex, Keyword, TextSpan, Token, TokenKind};
use crate::lsp::callable::{
    callable_argument_context_at_offset, callable_signature_parts, callable_type_owner,
    CallableParameter, CallableSignatureParts, CallableTarget,
};
use crate::lsp::{
    file_index_for_source, offset_for_position, range_for_span, FileIndexAnalysis,
    LspMarkupContent, LspPosition, LspRange,
};
use crate::model::{SourceKind, SymbolKind};
use crate::resolver::{CandidateSource, IdentifierContext, ReferenceCandidate, ReferenceResolver};
use crate::syntax::SyntaxNode;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const MAX_COMPLETION_ITEMS: usize = 250;
/// A foreground local query deliberately has a small, source-free admission
/// bound. Larger documents continue on the runtime semantic lane and use the
/// documented lexical/top-level result meanwhile.
/// Foreground queries inspect a fixed syntax window around the cursor.  This
/// deliberately bounds both lexing and declaration recovery for arbitrarily
/// large current snapshots; full parser/index construction stays on the
/// runtime semantic lane.
const LOCAL_SCOPE_QUERY_MAX_WINDOW_BYTES: usize = 16 * 1024;
const LOCAL_SCOPE_QUERY_TRAILING_BYTES: usize = 2 * 1024;
/// When the fixed window begins or ends in a multi-line lexical construct, a
/// small current snapshot may be lexed once to establish its real state. This
/// preserves immediate externally-proven completion in normal large files
/// without making foreground requests scale without bound.
const LEXICAL_CONTEXT_RECOVERY_MAX_SOURCE_BYTES: usize = 128 * 1024;
const LOCAL_SCOPE_QUERY_DEADLINE: Duration = Duration::from_millis(50);
const COMMAND_TRIGGER_PARAMETER_HINTS: &str = "editor.action.triggerParameterHints";
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<LspCommand>,
    pub text_edit: LspTextEdit,
    #[serde(skip)]
    pub required_parameter_count: usize,
    #[serde(skip)]
    pub optional_parameter_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspCommand {
    pub title: String,
    pub command: String,
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
    /// The candidate guarantee for this current-revision result. This is an
    /// internal contract/logging field, not an LSP `isIncomplete` surrogate.
    pub query_quality: QueryQuality,
    /// Why a non-exact result was selected. `RecoveryExact` is deliberately
    /// unused until a recovery query can prove candidate equivalence.
    pub recovery_reason: Option<String>,
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

#[derive(Debug, Clone, Copy)]
struct CompletionRenderContext<'a> {
    local_index: &'a SymbolIndex,
    workspace_index: Option<&'a SymbolIndex>,
    game_data_index: Option<&'a SymbolIndex>,
}

impl<'a> CompletionRenderContext<'a> {
    fn new(
        local_index: &'a SymbolIndex,
        workspace_index: Option<&'a SymbolIndex>,
        game_data_index: Option<&'a SymbolIndex>,
    ) -> Self {
        Self {
            local_index,
            workspace_index,
            game_data_index,
        }
    }

    fn is_enum_owner(self, owner: &str) -> bool {
        index_has_enum_owner(self.local_index, owner)
            || self
                .workspace_index
                .is_some_and(|index| index_has_enum_owner(index, owner))
            || self
                .game_data_index
                .is_some_and(|index| index_has_enum_owner(index, owner))
    }
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

/// Returns the current-revision lexical/top-level completion contract while a
/// whole-file analysis is still converging.  It intentionally supplies no
/// stale local scope or syntax facts: the lightweight analysis has current
/// lexer tokens plus an empty local index, so candidates are limited to
/// independently valid workspace/game-data and language facts.
pub(crate) fn completion_report_for_lexical_source_with_external_indexes(
    source: &str,
    position: LspPosition,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> LspCompletionReport {
    let Some(offset) = offset_for_position(source, position) else {
        return unavailable_completion_report(empty_completion_report(0), "invalid-position");
    };
    completion_report_for_lexical_source_at_offset_with_external_indexes(
        source,
        offset,
        workspace_index,
        game_data_index,
    )
}

pub(crate) fn completion_report_for_lexical_source_at_offset_with_external_indexes(
    source: &str,
    offset: usize,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> LspCompletionReport {
    let Some(region) = BoundedCompletionRegion::new(source, offset) else {
        // The fixed window can end inside a valid multi-line comment or
        // string.  It is not safe to recover local or receiver facts without
        // lexical context, but the cursor-local identifier prefix is enough
        // to retain independently indexed top-level suggestions.
        let report = top_level_fallback_report_for_prefix_span(
            source,
            raw_completion_prefix_span(source, offset),
            workspace_index,
            game_data_index,
            UnavailableCompletionContext::TopLevel,
        );
        return unavailable_completion_report(report, "invalid-or-unterminated-lexical-window");
    };
    let context = unavailable_completion_context(&region.tokens, offset);
    let report = lexical_top_level_fallback_report(
        source,
        &region.tokens,
        offset,
        workspace_index,
        game_data_index,
        context,
    );
    unavailable_completion_report(report, context.reason())
}

/// Runs `LocalScopeQuery` over a fixed lexical/syntax window from the current
/// revision.  It intentionally does not parse, index, or project the file:
/// when the callable block cannot be proved inside that window, callers use
/// the documented unavailable fallback.
pub(crate) fn completion_report_for_current_local_scope_at_offset_with_external_indexes(
    source: &str,
    offset: usize,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> Option<LspCompletionReport> {
    let start = Instant::now();
    let region = BoundedCompletionRegion::new(source, offset)?;
    if unavailable_completion_context(&region.tokens, offset)
        != UnavailableCompletionContext::TopLevel
    {
        return None;
    }
    let facts = BoundedCompletionFacts::recover(source, &region, offset)?;
    if start.elapsed() > LOCAL_SCOPE_QUERY_DEADLINE {
        return None;
    }
    Some(facts.local_completion_report(
        source,
        &region,
        offset,
        workspace_index,
        game_data_index,
        start,
    ))
}

/// Runs the current-revision `ReceiverResolutionQuery` for one bare local,
/// parameter, or field receiver. It also accepts a zero-argument call chain
/// whose callable facts are fully supplied by the captured external indexes.
/// Other chained, call, index, static, malformed, and over-budget expressions
/// use the lexical fallback rather than joining current text to older local
/// analysis.
pub(crate) fn completion_report_for_current_receiver_at_offset_with_external_indexes(
    source: &str,
    offset: usize,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> Option<LspCompletionReport> {
    let start = Instant::now();
    let region = BoundedCompletionRegion::new(source, offset)?;
    if unavailable_completion_context(&region.tokens, offset)
        != UnavailableCompletionContext::Member
    {
        return None;
    }
    let external_chain =
        region.external_call_chain_member_context(source, offset, workspace_index, game_data_index);
    let (receiver, receiver_span, owner, prefix, prefix_span, facts) =
        if let Some((receiver, receiver_span, owner, prefix, prefix_span)) = external_chain {
            (receiver, receiver_span, owner, prefix, prefix_span, None)
        } else {
            if region.has_completed_call_receiver(source, offset) {
                let report = lexical_top_level_fallback_report(
                    source,
                    &region.tokens,
                    offset,
                    workspace_index,
                    game_data_index,
                    UnavailableCompletionContext::Member,
                );
                return Some(unavailable_completion_report(
                    report,
                    "current-revision-external-call-chain-unresolved",
                ));
            }
            let facts = BoundedCompletionFacts::recover(source, &region, offset)?;
            let (receiver, receiver_span, prefix, prefix_span) =
                region.simple_member_context(source, offset)?;
            let owner = facts.visible_type(&receiver)?;
            (
                receiver,
                receiver_span,
                owner,
                prefix,
                prefix_span,
                Some(facts),
            )
        };
    if start.elapsed() > LOCAL_SCOPE_QUERY_DEADLINE {
        return None;
    }
    let empty_local_index = SymbolIndex::default();
    let mut report = member_completion_report_for_indexes(
        source,
        0,
        &owner,
        Some(receiver),
        receiver_span,
        Some(owner.clone()),
        prefix.clone(),
        prefix_span,
        None,
        false,
        MemberVisibilityContext::ExternalReceiver,
        &empty_local_index,
        workspace_index,
        game_data_index,
        LspCompletionTimings::default(),
        start,
    );
    if let Some(facts) = facts {
        facts.append_current_member_items(
            source,
            &region,
            &owner,
            &prefix,
            prefix_span,
            &mut report,
        );
    }
    Some(report)
}

/// Runs the bounded current-revision argument-label query for one bare,
/// already-resolved callable. The query admits only a valid current CST and a
/// simple identifier callee, so it cannot combine current argument text with
/// a prior document's callable facts. Member/delegate calls, constructors,
/// malformed text, values after a label, and over-budget work intentionally
/// remain on the documented unavailable fallback.
pub(crate) fn completion_report_for_current_argument_labels_at_offset_with_external_indexes(
    source: &str,
    offset: usize,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> Option<LspCompletionReport> {
    let start = Instant::now();
    let region = BoundedCompletionRegion::new(source, offset)?;
    if unavailable_completion_context(&region.tokens, offset)
        != UnavailableCompletionContext::Argument
    {
        return None;
    }
    let context = region.bare_argument_context(source, offset)?;
    let mut callable_candidates = Vec::new();
    for index in layered_external_indexes(workspace_index, game_data_index) {
        callable_candidates.extend(
            exact_top_level_candidates(index, &context.callee)
                .into_iter()
                .filter(|candidate| {
                    matches!(candidate.kind, SymbolKind::Function | SymbolKind::Method)
                }),
        );
    }
    let callable_candidates = combine_completion_candidates(callable_candidates);
    if start.elapsed() > LOCAL_SCOPE_QUERY_DEADLINE {
        return None;
    }
    if callable_candidates.is_empty() {
        let labels =
            BoundedCompletionFacts::callable_parameter_names(source, &region, &context.callee)?;
        return (start.elapsed() <= LOCAL_SCOPE_QUERY_DEADLINE).then_some(
            bounded_argument_label_report(source, context, labels, start),
        );
    }
    let parameter_candidates =
        parameter_label_candidates_for_callables(&callable_candidates, &context);
    let edit_range = range_for_span(source, context.prefix_span);
    let empty_local_index = SymbolIndex::default();
    let (items, source_kind_counts, origin_counts) = completion_items_for_parameter_labels(
        &parameter_candidates,
        edit_range,
        CompletionRenderContext::new(&empty_local_index, workspace_index, game_data_index),
    );
    let (items, is_incomplete) = cap_completion_items(items);
    (start.elapsed() <= LOCAL_SCOPE_QUERY_DEADLINE).then_some(LspCompletionReport {
        candidate_count: items.len(),
        list: LspCompletionList {
            is_incomplete,
            items,
        },
        query_quality: QueryQuality::Exact,
        recovery_reason: None,
        parse_diagnostics: 0,
        completion_context: "argument-label".to_string(),
        receiver_text: None,
        owner_type: None,
        prefix: context.prefix,
        source_kind_counts,
        origin_counts,
        failure_reason: None,
        timings: LspCompletionTimings {
            total: start.elapsed(),
            ..LspCompletionTimings::default()
        },
    })
}

fn is_simple_current_receiver(receiver: &str) -> bool {
    let mut chars = receiver.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// A source-free foreground view.  The window starts at a line boundary, so
/// lexer spans can be translated back to the immutable snapshot without ever
/// traversing its root syntax tree.
struct BoundedCompletionRegion {
    tokens: Vec<Token>,
}

impl BoundedCompletionRegion {
    fn new(source: &str, offset: usize) -> Option<Self> {
        let (start, end) = Self::window_bounds(source, offset)?;
        Self::from_window(source, start, end)
            .or_else(|| Self::from_current_snapshot_context(source, start, end))
    }

    fn window_bounds(source: &str, offset: usize) -> Option<(usize, usize)> {
        if offset > source.len() || !source.is_char_boundary(offset) {
            return None;
        }
        let mut start = offset.saturating_sub(LOCAL_SCOPE_QUERY_MAX_WINDOW_BYTES);
        while start < offset && !source.is_char_boundary(start) {
            start += 1;
        }
        if let Some(newline) = source.get(start..offset)?.find('\n') {
            start += newline + 1;
        }
        let mut end = (offset + LOCAL_SCOPE_QUERY_TRAILING_BYTES).min(source.len());
        while end > offset && !source.is_char_boundary(end) {
            end -= 1;
        }
        Some((start, end))
    }

    fn from_window(source: &str, start: usize, end: usize) -> Option<Self> {
        let mut tokens = lex(source.get(start..end)?);
        for token in &mut tokens {
            token.span.start += start;
            token.span.end += start;
        }
        (!tokens.iter().any(|token| token.kind.is_error())).then_some(Self { tokens })
    }

    fn from_current_snapshot_context(source: &str, start: usize, end: usize) -> Option<Self> {
        (source.len() <= LEXICAL_CONTEXT_RECOVERY_MAX_SOURCE_BYTES).then_some(())?;
        let mut tokens = lex(source);
        tokens.retain(|token| token.span.start < end && token.span.end > start);
        (!tokens.iter().any(|token| token.kind.is_error())).then_some(Self { tokens })
    }

    fn significant_before(&self, offset: usize) -> Vec<Token> {
        self.tokens
            .iter()
            .copied()
            .filter(|token| {
                token.span.end <= offset && !token.kind.is_trivia() && token.kind != TokenKind::Eof
            })
            .collect()
    }

    fn simple_member_context(
        &self,
        source: &str,
        offset: usize,
    ) -> Option<(String, TextSpan, String, TextSpan)> {
        let prefix_span = lexical_completion_prefix_span(&self.tokens, offset);
        let prefix = source.get(prefix_span.start..prefix_span.end)?.to_string();
        let tokens = self.significant_before(prefix_span.start);
        let dot = tokens.last()?;
        (dot.kind == TokenKind::Dot).then_some(())?;
        let receiver = tokens.get(tokens.len().checked_sub(2)?)?;
        (receiver.kind == TokenKind::Identifier).then_some(())?;
        let receiver_text = source
            .get(receiver.span.start..receiver.span.end)?
            .to_string();
        is_simple_current_receiver(&receiver_text).then_some((
            receiver_text,
            receiver.span,
            prefix,
            prefix_span,
        ))
    }

    fn external_call_chain_member_context(
        &self,
        source: &str,
        offset: usize,
        workspace_index: Option<&SymbolIndex>,
        game_data_index: Option<&SymbolIndex>,
    ) -> Option<(String, TextSpan, String, String, TextSpan)> {
        let prefix_span = lexical_completion_prefix_span(&self.tokens, offset);
        let prefix = source.get(prefix_span.start..prefix_span.end)?.to_string();
        let tokens = self.significant_before(prefix_span.start);
        let dot = tokens.last()?;
        (dot.kind == TokenKind::Dot).then_some(())?;

        let mut cursor = tokens.len().checked_sub(1)?;
        let mut calls = Vec::new();
        let receiver_end = tokens.get(cursor.checked_sub(1)?)?.span.end;
        loop {
            cursor = cursor.checked_sub(1)?;
            (tokens.get(cursor)?.kind == TokenKind::RightParen).then_some(())?;
            cursor = cursor.checked_sub(1)?;
            (tokens.get(cursor)?.kind == TokenKind::LeftParen).then_some(())?;
            cursor = cursor.checked_sub(1)?;
            let name = tokens.get(cursor)?;
            (name.kind == TokenKind::Identifier).then_some(())?;
            calls.push(source.get(name.span.start..name.span.end)?.to_string());

            let Some(previous) = cursor.checked_sub(1) else {
                break;
            };
            if tokens.get(previous)?.kind != TokenKind::Dot {
                break;
            }
            cursor = previous;
        }
        calls.reverse();
        // A completed external call is already a member receiver path:
        // `GetGame().GetPlayer...` must not wait for whole-file analysis.
        // Each additional `.Method()` is another proven external step.
        (!calls.is_empty()).then_some(())?;
        let receiver_start = tokens.get(cursor)?.span.start;
        let receiver_span = TextSpan::new(receiver_start, receiver_end);
        let receiver = source
            .get(receiver_span.start..receiver_span.end)?
            .to_string();
        let owner = external_call_chain_owner(
            &calls,
            [workspace_index, game_data_index].into_iter().flatten(),
        )?;
        Some((receiver, receiver_span, owner, prefix, prefix_span))
    }

    fn has_completed_call_receiver(&self, source: &str, offset: usize) -> bool {
        let prefix_span = lexical_completion_prefix_span(&self.tokens, offset);
        let tokens = self.significant_before(prefix_span.start);
        tokens
            .last()
            .is_some_and(|token| token.kind == TokenKind::Dot)
            && tokens
                .get(tokens.len().saturating_sub(2))
                .is_some_and(|token| token.kind == TokenKind::RightParen)
            && source.get(prefix_span.start..prefix_span.end).is_some()
    }

    fn bare_argument_context(
        &self,
        source: &str,
        offset: usize,
    ) -> Option<ArgumentLabelCompletionContext> {
        let (prefix, prefix_span) =
            completion_identifier_prefix_for_argument_label(source, &self.tokens, offset)?;
        let tokens = self.significant_before(prefix_span.start);
        let left_paren_index = tokens
            .iter()
            .rposition(|token| token.kind == TokenKind::LeftParen)?;
        let callee = tokens.get(left_paren_index.checked_sub(1)?)?;
        (callee.kind == TokenKind::Identifier).then_some(())?;
        let callee = source.get(callee.span.start..callee.span.end)?.to_string();
        is_simple_current_receiver(&callee).then_some(())?;
        let mut supplied_labels = BTreeSet::new();
        for pair in tokens[left_paren_index + 1..].windows(2) {
            if pair[0].kind == TokenKind::Identifier && pair[1].kind == TokenKind::Colon {
                supplied_labels.insert(
                    source
                        .get(pair[0].span.start..pair[0].span.end)?
                        .to_string(),
                );
            }
        }
        Some(ArgumentLabelCompletionContext {
            prefix,
            prefix_span,
            target: CallableTarget::Call {
                callee_span: TextSpan::new(0, 0),
            },
            argument_index: tokens[left_paren_index + 1..]
                .iter()
                .filter(|token| token.kind == TokenKind::Comma)
                .count(),
            supplied_labels,
            callee,
        })
    }
}

fn external_call_chain_owner<'a>(
    calls: &[String],
    indexes: impl IntoIterator<Item = &'a SymbolIndex>,
) -> Option<String> {
    let indexes = indexes.into_iter().collect::<Vec<_>>();
    let first = calls.first()?;
    let mut owner = None;
    for index in &indexes {
        for id in index.preferred_from_symbols(index.functions_by_name(first)) {
            if !callable_accepts_zero_arguments(index, id) {
                continue;
            }
            let Some(symbol) = index.symbol(id) else {
                continue;
            };
            if let Some(result) = expression_type_from_index_symbol(index, symbol) {
                owner = Some(result.owner_type);
                break;
            }
        }
        if owner.is_some() {
            break;
        }
    }
    let mut owner = owner?;

    for call in calls.iter().skip(1) {
        let mut next = None;
        for index in &indexes {
            let candidates = IndexQuery::new(index).completion_members_for_class(&owner);
            for candidate in candidates.candidates {
                if candidate.name.as_deref() != Some(call.as_str()) {
                    continue;
                }
                if !callable_accepts_zero_arguments(index, candidate.id) {
                    continue;
                }
                let Some(symbol) = index.symbol(candidate.id) else {
                    continue;
                };
                if let Some(result) = expression_type_from_index_symbol(index, symbol) {
                    next = Some(result.owner_type);
                    break;
                }
            }
            if next.is_some() {
                break;
            }
        }
        owner = next?;
    }

    Some(owner)
}

fn callable_accepts_zero_arguments(index: &SymbolIndex, id: crate::index::GlobalSymbolId) -> bool {
    index.children(id).iter().all(|parameter| {
        index.symbol(*parameter).is_some_and(|symbol| {
            symbol.kind != SymbolKind::Parameter || symbol.detail.default_text.is_some()
        })
    })
}

#[derive(Clone)]
struct BoundedDeclaration {
    name: String,
    owner_type: String,
    scope: Vec<usize>,
    declared_at: usize,
}

struct BoundedCompletionFacts {
    declarations: Vec<BoundedDeclaration>,
    cursor_scope: Vec<usize>,
}

impl BoundedCompletionFacts {
    fn recover(source: &str, region: &BoundedCompletionRegion, offset: usize) -> Option<Self> {
        let tokens = region.significant_before(offset);
        let mut scopes = Vec::new();
        let mut declarations = Vec::new();
        for (index, token) in tokens.iter().enumerate() {
            match token.kind {
                TokenKind::LeftBrace => scopes.push(token.span.start),
                TokenKind::RightBrace => {
                    scopes.pop()?;
                }
                _ => {}
            }
            let Some(name) = tokens.get(index + 1) else {
                continue;
            };
            if !is_bounded_type_token(token.kind) || name.kind != TokenKind::Identifier {
                continue;
            }
            let Some(next) = tokens.get(index + 2) else {
                continue;
            };
            if next.kind == TokenKind::LeftParen
                || !matches!(
                    next.kind,
                    TokenKind::Semicolon
                        | TokenKind::Comma
                        | TokenKind::Operator(_)
                        | TokenKind::RightParen
                )
            {
                continue;
            }
            let owner_type = source.get(token.span.start..token.span.end)?.to_string();
            let name_text = source.get(name.span.start..name.span.end)?.to_string();
            let mut declaration_scope = scopes.clone();
            // A parameter belongs to the following callable body, not the
            // enclosing class.  Requiring that body inside the window avoids
            // leaking parameters into sibling methods.
            if next.kind == TokenKind::Comma || next.kind == TokenKind::RightParen {
                let body = tokens[index + 2..]
                    .iter()
                    .find(|candidate| candidate.kind == TokenKind::LeftBrace)?;
                declaration_scope.push(body.span.start);
            }
            declarations.push(BoundedDeclaration {
                name: name_text,
                owner_type,
                scope: declaration_scope,
                declared_at: name.span.start,
            });
        }
        // A callable-local query is exact only with a proven enclosing body
        // and its closing delimiter inside the same bounded region.  This
        // deliberately declines an in-progress unterminated body rather than
        // guessing from a partial file projection.
        (!scopes.is_empty()
            && region
                .tokens
                .iter()
                .any(|token| token.span.start >= offset && token.kind == TokenKind::RightBrace))
        .then_some(Self {
            declarations,
            cursor_scope: scopes,
        })
    }

    fn visible_declarations(&self) -> Vec<&BoundedDeclaration> {
        let mut latest = BTreeMap::<String, &BoundedDeclaration>::new();
        for declaration in &self.declarations {
            if declaration.scope.len() <= self.cursor_scope.len()
                && self.cursor_scope.starts_with(&declaration.scope)
            {
                latest
                    .entry(declaration.name.clone())
                    .and_modify(|current| {
                        if declaration.scope.len() > current.scope.len()
                            || (declaration.scope.len() == current.scope.len()
                                && declaration.declared_at > current.declared_at)
                        {
                            *current = declaration;
                        }
                    })
                    .or_insert(declaration);
            }
        }
        latest.into_values().collect()
    }

    fn visible_type(&self, name: &str) -> Option<String> {
        self.visible_declarations()
            .into_iter()
            .find(|declaration| declaration.name == name)
            .map(|declaration| declaration.owner_type.clone())
    }

    fn local_completion_report(
        &self,
        source: &str,
        region: &BoundedCompletionRegion,
        offset: usize,
        workspace_index: Option<&SymbolIndex>,
        game_data_index: Option<&SymbolIndex>,
        start: Instant,
    ) -> LspCompletionReport {
        let prefix_span = lexical_completion_prefix_span(&region.tokens, offset);
        let prefix = source[prefix_span.start..prefix_span.end].to_string();
        let mut items = self
            .visible_declarations()
            .into_iter()
            .filter_map(|declaration| {
                candidate_matches_prefix_name(&declaration.name, &prefix).then_some(
                    LspCompletionItem {
                        label: declaration.name.clone(),
                        label_details: Some(LspCompletionItemLabelDetails {
                            detail: Some(format!(": {}", declaration.owner_type)),
                            description: None,
                        }),
                        kind: 6,
                        detail: Some(declaration.owner_type.clone()),
                        documentation: None,
                        sort_text: Some(format!("000:local:{}", declaration.name)),
                        filter_text: Some(declaration.name.clone()),
                        insert_text_format: None,
                        command: None,
                        text_edit: LspTextEdit {
                            range: range_for_span(source, prefix_span),
                            new_text: declaration.name.clone(),
                        },
                        required_parameter_count: 0,
                        optional_parameter_count: 0,
                    },
                )
            })
            .collect::<Vec<_>>();
        // Local facts are exact, but ordinary completion also retains the
        // independently valid external/keyword candidates from this same
        // lexical window.  This is a merge of current bounded facts, never a
        // projection of the document snapshot.
        let fallback = lexical_top_level_fallback_report(
            source,
            &region.tokens,
            offset,
            workspace_index,
            game_data_index,
            UnavailableCompletionContext::TopLevel,
        );
        items.extend(fallback.list.items);
        let mut seen = BTreeSet::new();
        items.retain(|item| {
            seen.insert((
                item.label.to_ascii_lowercase(),
                item.text_edit.new_text.clone(),
            ))
        });
        let (items, is_incomplete) = cap_completion_items(items);
        LspCompletionReport {
            candidate_count: items.len(),
            list: LspCompletionList {
                is_incomplete,
                items,
            },
            query_quality: QueryQuality::Exact,
            recovery_reason: None,
            parse_diagnostics: 0,
            completion_context: "local".to_string(),
            receiver_text: None,
            owner_type: None,
            prefix,
            source_kind_counts: fallback.source_kind_counts,
            origin_counts: fallback.origin_counts,
            failure_reason: None,
            timings: LspCompletionTimings {
                total: start.elapsed(),
                ..LspCompletionTimings::default()
            },
        }
    }

    fn append_current_member_items(
        &self,
        source: &str,
        region: &BoundedCompletionRegion,
        owner: &str,
        prefix: &str,
        prefix_span: TextSpan,
        report: &mut LspCompletionReport,
    ) {
        let mut items = Self::class_member_names(source, region, owner)
            .into_iter()
            .filter(|name| candidate_matches_prefix_name(name, prefix))
            .map(|name| {
                bounded_completion_item(source, prefix_span, name, Some(owner.to_string()), 2)
            })
            .collect::<Vec<_>>();
        if items.is_empty() {
            return;
        }
        items.extend(std::mem::take(&mut report.list.items));
        let (items, is_incomplete) = cap_completion_items(items);
        report.candidate_count = items.len();
        report.list = LspCompletionList {
            is_incomplete,
            items,
        };
    }

    fn class_member_names(
        source: &str,
        region: &BoundedCompletionRegion,
        owner: &str,
    ) -> Vec<String> {
        let tokens = region
            .tokens
            .iter()
            .copied()
            .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Eof)
            .collect::<Vec<_>>();
        let class_index = tokens.windows(3).position(|window| {
            matches!(window[0].kind, TokenKind::Keyword(Keyword::Class))
                && source.get(window[1].span.start..window[1].span.end) == Some(owner)
                && window[2].kind == TokenKind::LeftBrace
        });
        let Some(class_index) = class_index else {
            return Vec::new();
        };
        let mut depth = 1usize;
        let mut names = Vec::new();
        for window in tokens[class_index + 3..].windows(3) {
            match window[0].kind {
                TokenKind::LeftBrace => depth += 1,
                TokenKind::RightBrace => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                _ if depth == 1
                    && is_bounded_type_token(window[0].kind)
                    && window[1].kind == TokenKind::Identifier
                    && window[2].kind == TokenKind::LeftParen =>
                {
                    if let Some(name) = source.get(window[1].span.start..window[1].span.end) {
                        names.push(name.to_string());
                    }
                }
                _ => {}
            }
        }
        names
    }

    fn callable_parameter_names(
        source: &str,
        region: &BoundedCompletionRegion,
        callee: &str,
    ) -> Option<Vec<String>> {
        let tokens = region.significant_before(source.len());
        let callable = tokens.windows(3).find(|window| {
            is_bounded_type_token(window[0].kind)
                && window[1].kind == TokenKind::Identifier
                && source.get(window[1].span.start..window[1].span.end) == Some(callee)
                && window[2].kind == TokenKind::LeftParen
        })?;
        let open = tokens
            .iter()
            .position(|token| token.span == callable[2].span)?;
        let close = tokens[open + 1..]
            .iter()
            .position(|token| token.kind == TokenKind::RightParen)?
            + open
            + 1;
        let mut names = Vec::new();
        for window in tokens[open + 1..close].windows(2) {
            if is_bounded_type_token(window[0].kind) && window[1].kind == TokenKind::Identifier {
                names.push(
                    source
                        .get(window[1].span.start..window[1].span.end)?
                        .to_string(),
                );
            }
        }
        Some(names)
    }
}

fn bounded_completion_item(
    source: &str,
    prefix_span: TextSpan,
    label: String,
    detail: Option<String>,
    kind: u32,
) -> LspCompletionItem {
    LspCompletionItem {
        label: label.clone(),
        label_details: detail.as_ref().map(|detail| LspCompletionItemLabelDetails {
            detail: Some(format!(": {detail}")),
            description: None,
        }),
        kind,
        detail,
        documentation: None,
        sort_text: Some(format!("000:bounded:{label}")),
        filter_text: Some(label.clone()),
        insert_text_format: None,
        command: None,
        text_edit: LspTextEdit {
            range: range_for_span(source, prefix_span),
            new_text: label,
        },
        required_parameter_count: 0,
        optional_parameter_count: 0,
    }
}

fn bounded_argument_label_report(
    source: &str,
    context: ArgumentLabelCompletionContext,
    labels: Vec<String>,
    start: Instant,
) -> LspCompletionReport {
    let mut items = labels
        .into_iter()
        .filter(|label| starts_with_ignore_ascii_case(label, &context.prefix))
        .filter(|label| {
            !context
                .supplied_labels
                .iter()
                .any(|supplied| supplied.eq_ignore_ascii_case(label))
        })
        .map(|label| bounded_completion_item(source, context.prefix_span, label, None, 5))
        .collect::<Vec<_>>();
    let (items, is_incomplete) = cap_completion_items(std::mem::take(&mut items));
    LspCompletionReport {
        candidate_count: items.len(),
        list: LspCompletionList {
            is_incomplete,
            items,
        },
        query_quality: QueryQuality::Exact,
        recovery_reason: None,
        parse_diagnostics: 0,
        completion_context: "argument-label".to_string(),
        receiver_text: None,
        owner_type: None,
        prefix: context.prefix,
        source_kind_counts: BTreeMap::new(),
        origin_counts: BTreeMap::new(),
        failure_reason: None,
        timings: LspCompletionTimings {
            total: start.elapsed(),
            ..LspCompletionTimings::default()
        },
    }
}

fn is_bounded_type_token(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::Keyword(
                Keyword::Int
                    | Keyword::Void
                    | Keyword::Float
                    | Keyword::Bool
                    | Keyword::String
                    | Keyword::Vector
                    | Keyword::Typename
                    | Keyword::Auto
            )
    )
}

fn candidate_matches_prefix_name(name: &str, prefix: &str) -> bool {
    completion_name_match_rank(name, prefix).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnavailableCompletionContext {
    TopLevel,
    Member,
    Argument,
}

impl UnavailableCompletionContext {
    const fn reason(self) -> &'static str {
        match self {
            Self::TopLevel => "current-revision-local-facts-pending",
            Self::Member => "current-revision-receiver-facts-pending",
            Self::Argument => "current-revision-argument-facts-pending",
        }
    }
}

fn unavailable_completion_report(
    mut report: LspCompletionReport,
    reason: impl Into<String>,
) -> LspCompletionReport {
    report.query_quality = QueryQuality::Unavailable;
    report.recovery_reason = Some(reason.into());
    report
}

fn unavailable_completion_context(
    tokens: &[crate::lexer::Token],
    offset: usize,
) -> UnavailableCompletionContext {
    let prefix_span = lexical_completion_prefix_span(tokens, offset);
    let previous = previous_significant_completion_token_before_span(tokens, prefix_span);
    if previous.is_some_and(|token| token.kind == TokenKind::Dot) {
        return UnavailableCompletionContext::Member;
    }
    if previous.is_some_and(|token| matches!(token.kind, TokenKind::LeftParen | TokenKind::Comma)) {
        return UnavailableCompletionContext::Argument;
    }
    UnavailableCompletionContext::TopLevel
}

fn lexical_completion_prefix_span(tokens: &[crate::lexer::Token], offset: usize) -> TextSpan {
    tokens
        .iter()
        .find(|token| {
            token.kind == TokenKind::Identifier
                && token.span.start <= offset
                && offset <= token.span.end
        })
        .map(|token| TextSpan::new(token.span.start, offset))
        .unwrap_or_else(|| TextSpan::new(offset, offset))
}

/// Returns the identifier immediately before the current cursor without
/// attempting to derive lexical context. This is only used after the bounded
/// lexer has rejected its arbitrary source window, so it must not be used to
/// infer local scope, member ownership, or argument facts.
fn raw_completion_prefix_span(source: &str, offset: usize) -> TextSpan {
    let Some(before_cursor) = source.get(..offset) else {
        return TextSpan::new(offset, offset);
    };
    let start = before_cursor
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            (!matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
                .then_some(index + character.len_utf8())
        })
        .unwrap_or(0);
    let Some(prefix) = source.get(start..offset) else {
        return TextSpan::new(offset, offset);
    };
    prefix
        .chars()
        .next()
        .is_some_and(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '_'))
        .then_some(TextSpan::new(start, offset))
        .unwrap_or_else(|| TextSpan::new(offset, offset))
}

fn lexical_top_level_fallback_report(
    source: &str,
    tokens: &[Token],
    offset: usize,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    context: UnavailableCompletionContext,
) -> LspCompletionReport {
    top_level_fallback_report_for_prefix_span(
        source,
        lexical_completion_prefix_span(tokens, offset),
        workspace_index,
        game_data_index,
        context,
    )
}

fn top_level_fallback_report_for_prefix_span(
    source: &str,
    prefix_span: TextSpan,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    context: UnavailableCompletionContext,
) -> LspCompletionReport {
    let total_start = Instant::now();
    let prefix = source
        .get(prefix_span.start..prefix_span.end)
        .unwrap_or_default()
        .to_string();
    let empty_local_index = SymbolIndex::default();
    let candidates = top_level_source_completion_candidates(
        &prefix,
        EditorTopLevelCompletionMode::Value,
        &empty_local_index,
        workspace_index,
        game_data_index,
        32,
    );
    let render_context =
        CompletionRenderContext::new(&empty_local_index, workspace_index, game_data_index);
    let (mut items, source_kind_counts, origin_counts) = completion_items_for_candidates(
        &candidates,
        range_for_span(source, prefix_span),
        None,
        CompletionInsertContext::Normal,
        Some(&prefix),
        render_context,
    );
    items.extend(keyword_completion_items(
        &prefix,
        range_for_span(source, prefix_span),
        EditorTopLevelCompletionMode::Value,
        false,
    ));
    let (items, is_incomplete) = cap_completion_items(items);
    let mut report = LspCompletionReport {
        candidate_count: items.len(),
        list: LspCompletionList {
            is_incomplete,
            items,
        },
        query_quality: QueryQuality::Exact,
        recovery_reason: None,
        parse_diagnostics: 0,
        completion_context: "top-level".to_string(),
        receiver_text: None,
        owner_type: None,
        prefix,
        source_kind_counts,
        origin_counts,
        failure_reason: None,
        timings: LspCompletionTimings {
            total: total_start.elapsed(),
            ..LspCompletionTimings::default()
        },
    };
    report.completion_context = match context {
        UnavailableCompletionContext::Member => "member-unavailable-top-level-fallback",
        UnavailableCompletionContext::Argument => "argument-unavailable-top-level-fallback",
        UnavailableCompletionContext::TopLevel => "top-level",
    }
    .to_string();
    report.receiver_text = None;
    report.owner_type = None;
    report.failure_reason = Some(context.reason().to_string());
    report
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
    output.push_str(&format!("- Query Quality: `{:?}`\n", report.query_quality));
    output.push_str(&format!(
        "- Recovery Reason: `{}`\n",
        escape_markdown_cell(report.recovery_reason.as_deref().unwrap_or("<none>"))
    ));
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
        "| # | Label | Kind | Detail | Label Details | Required | Optional | Insert Text | Command | Sort Text | Docs Preview |\n",
    );
    output.push_str("| ---: | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |\n");
    for (index, item) in report.list.items.iter().take(50).enumerate() {
        output.push_str(&format!(
            "| {} | `{}` | `{}` | `{}` | `{}` | {} | {} | `{}` | `{}` | `{}` | {} |\n",
            index + 1,
            escape_markdown_cell(&item.label),
            completion_lsp_kind_label(item.kind),
            escape_markdown_cell(item.detail.as_deref().unwrap_or("")),
            escape_markdown_cell(&format_label_details(item.label_details.as_ref())),
            item.required_parameter_count,
            item.optional_parameter_count,
            escape_markdown_cell(&item.text_edit.new_text),
            escape_markdown_cell(
                item.command
                    .as_ref()
                    .map(|command| command.command.as_str())
                    .unwrap_or("")
            ),
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
            "|  |  |  |  |  |  |  |  |  |  | +{} more |\n",
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
    let mut argument_label_fallback = None;
    if let Some(context) = argument_label_completion_context(source, &analysis.parse.root, offset) {
        let context_elapsed = context_start.elapsed();
        let argument_label_report = argument_label_completion_report_for_indexes(
            source,
            analysis.parse_diagnostics,
            context,
            &resolver,
            &analysis.index,
            workspace_index,
            game_data_index,
            LspCompletionTimings {
                context_detection: context_elapsed,
                ..LspCompletionTimings::default()
            },
            total_start,
        );
        if !argument_label_report.list.items.is_empty() {
            argument_label_fallback = Some(argument_label_report);
        }
    }

    if let Some(context) =
        resolver.member_completion_context_at_offset_with_tokens(offset, &analysis.lexer_tokens)
    {
        let context_elapsed = context_start.elapsed();
        let receiver_text = Some(context.receiver.receiver_text.clone());
        let receiver_span = context.receiver.receiver_span;
        let owner_type = context.receiver.owner_type.clone();
        let receiver_is_static = context.receiver.is_static;
        let prefix = context.prefix.clone();
        let failure_reason = context.receiver.failure_reason.clone();
        let Some(owner) = owner_type.clone() else {
            return argument_label_fallback.unwrap_or_else(|| LspCompletionReport {
                list: empty_completion_list(),
                query_quality: QueryQuality::Exact,
                recovery_reason: None,
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
            });
        };
        let visibility = member_visibility_context(
            receiver_text.as_deref(),
            &owner,
            containing_class_name(&analysis.index, offset).as_deref(),
        );
        let member_report = member_completion_report_for_indexes(
            source,
            analysis.parse_diagnostics,
            &owner,
            receiver_text,
            receiver_span,
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
        if !member_report.list.items.is_empty() {
            return member_report;
        }
        if let Some(fallback) = argument_label_fallback {
            return fallback;
        }
        return member_report;
    }

    let top_level_context =
        resolver.top_level_completion_context_at_offset_with_tokens(offset, &analysis.lexer_tokens);
    let context_elapsed = context_start.elapsed();
    let Some(context) = top_level_context else {
        if let Some(fallback) = argument_label_fallback {
            return fallback;
        }
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

    let top_level_report = top_level_completion_report_for_indexes(
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
    );
    if let Some(fallback) = argument_label_fallback {
        return merge_argument_label_and_value_reports(fallback, top_level_report, total_start);
    }
    if !top_level_report.list.items.is_empty() {
        return top_level_report;
    }
    top_level_report
}

fn merge_argument_label_and_value_reports(
    mut argument_report: LspCompletionReport,
    value_report: LspCompletionReport,
    total_start: Instant,
) -> LspCompletionReport {
    if value_report.list.items.is_empty() {
        argument_report.timings.total = total_start.elapsed();
        return argument_report;
    }

    let has_exact_value = value_report.list.items.iter().any(|item| {
        completion_item_is_argument_value(item)
            && item.label.eq_ignore_ascii_case(&value_report.prefix)
    });
    let argument_sort_prefix = if has_exact_value { "001" } else { "000" };
    for (index, item) in argument_report.list.items.iter_mut().enumerate() {
        item.sort_text = Some(format!(
            "{argument_sort_prefix}:argument:{index:03}:{}",
            item.label
        ));
    }

    let value_is_incomplete = value_report.list.is_incomplete;
    let argument_items = argument_report.list.items;
    let value_items = value_report.list.items;
    let combined_items: Box<dyn Iterator<Item = LspCompletionItem>> = if has_exact_value {
        Box::new(value_items.into_iter().chain(argument_items))
    } else {
        Box::new(argument_items.into_iter().chain(value_items))
    };

    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for item in combined_items {
        let key = format!(
            "{}:{}",
            item.label.to_ascii_lowercase(),
            item.text_edit.new_text
        );
        if seen.insert(key) {
            items.push(item);
        }
    }
    let (items, is_incomplete) = cap_completion_items(items);

    merge_count_maps(
        &mut argument_report.source_kind_counts,
        value_report.source_kind_counts,
    );
    merge_count_maps(
        &mut argument_report.origin_counts,
        value_report.origin_counts,
    );
    argument_report.candidate_count = items.len();
    argument_report.list = LspCompletionList {
        is_incomplete: is_incomplete || value_is_incomplete,
        items,
    };
    argument_report.timings.candidate_lookup += value_report.timings.candidate_lookup;
    argument_report.timings.item_rendering += value_report.timings.item_rendering;
    argument_report.timings.total = total_start.elapsed();
    argument_report
}

fn completion_item_is_argument_value(item: &LspCompletionItem) -> bool {
    matches!(item.kind, 2 | 3 | 5 | 6 | 12 | 14 | 21 | 22)
}

#[allow(clippy::too_many_arguments)]
fn member_completion_report_for_indexes(
    source: &str,
    parse_diagnostics: usize,
    owner: &str,
    receiver_text: Option<String>,
    _receiver_span: TextSpan,
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
    let render_context =
        CompletionRenderContext::new(local_index, workspace_index, game_data_index);
    let (items, source_kind_counts, origin_counts) = completion_items_for_candidates(
        &candidates,
        edit_range,
        None,
        CompletionInsertContext::Normal,
        Some(&prefix),
        render_context,
    );
    let (items, is_incomplete) = cap_completion_items(items);
    timings.item_rendering = render_start.elapsed();
    timings.total = total_start.elapsed();

    LspCompletionReport {
        candidate_count: items.len(),
        list: LspCompletionList {
            is_incomplete,
            items,
        },
        query_quality: QueryQuality::Exact,
        recovery_reason: None,
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

#[allow(clippy::too_many_arguments)]
fn argument_label_completion_report_for_indexes(
    source: &str,
    parse_diagnostics: usize,
    context: ArgumentLabelCompletionContext,
    resolver: &ReferenceResolver<'_, '_>,
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    mut timings: LspCompletionTimings,
    total_start: Instant,
) -> LspCompletionReport {
    let lookup_start = Instant::now();
    let callable_candidates = callable_candidates_for_argument_label_context(
        &context,
        resolver,
        local_index,
        workspace_index,
        game_data_index,
    );
    let parameter_candidates =
        parameter_label_candidates_for_callables(&callable_candidates, &context);
    timings.candidate_lookup = lookup_start.elapsed();

    let render_start = Instant::now();
    let edit_range = range_for_span(source, context.prefix_span);
    let render_context =
        CompletionRenderContext::new(local_index, workspace_index, game_data_index);
    let (items, source_kind_counts, origin_counts) =
        completion_items_for_parameter_labels(&parameter_candidates, edit_range, render_context);
    let (items, is_incomplete) = cap_completion_items(items);
    timings.item_rendering = render_start.elapsed();
    timings.total = total_start.elapsed();

    LspCompletionReport {
        candidate_count: items.len(),
        list: LspCompletionList {
            is_incomplete,
            items,
        },
        query_quality: QueryQuality::Exact,
        recovery_reason: None,
        parse_diagnostics,
        completion_context: "argument-label".to_string(),
        receiver_text: None,
        owner_type: None,
        prefix: context.prefix,
        source_kind_counts,
        origin_counts,
        failure_reason: if callable_candidates.is_empty() {
            Some("callable target was not resolved".to_string())
        } else {
            None
        },
        timings,
    }
}

fn callable_candidates_for_argument_label_context(
    context: &ArgumentLabelCompletionContext,
    resolver: &ReferenceResolver<'_, '_>,
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> Vec<EditorCompletionCandidate> {
    match &context.target {
        CallableTarget::Attribute { name } | CallableTarget::New { type_name: name } => {
            let mut candidates = exact_top_level_candidates(local_index, name);
            if let Some(index) = workspace_index {
                candidates.extend(exact_top_level_candidates(index, name));
            }
            if let Some(index) = game_data_index {
                candidates.extend(exact_top_level_candidates(index, name));
            }
            combine_completion_candidates(candidates)
        }
        CallableTarget::Call { callee_span } => resolver
            .resolve_at_offset(callee_span.start)
            .and_then(|resolution| resolution.selected)
            .and_then(|selected| {
                completion_candidate_for_reference(
                    &selected,
                    local_index,
                    workspace_index,
                    game_data_index,
                )
            })
            .into_iter()
            .collect(),
    }
}

fn exact_top_level_candidates(index: &SymbolIndex, name: &str) -> Vec<EditorCompletionCandidate> {
    IndexQuery::new(index)
        .completion_top_level_limited(name, EditorTopLevelCompletionMode::Type, 32)
        .into_iter()
        .chain(IndexQuery::new(index).completion_top_level_limited(
            name,
            EditorTopLevelCompletionMode::Value,
            32,
        ))
        .filter(|candidate| {
            candidate
                .name
                .as_deref()
                .unwrap_or(candidate.display.label.as_str())
                == name
        })
        .collect()
}

fn completion_candidate_for_reference(
    reference: &ReferenceCandidate,
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> Option<EditorCompletionCandidate> {
    let indexes = match reference.source {
        CandidateSource::FileLocal => vec![local_index],
        CandidateSource::External => workspace_index.into_iter().chain(game_data_index).collect(),
    };

    for index in indexes {
        let Some(symbol) = index.symbol(reference.id) else {
            continue;
        };
        if symbol.kind != reference.kind || symbol.name != reference.name {
            continue;
        }
        if let Some(expected_path) = reference.absolute_path.as_ref() {
            let Some(actual_path) = index
                .file(reference.id.file_id)
                .and_then(|file| file.metadata.absolute_path.as_ref())
            else {
                continue;
            };
            if actual_path != expected_path {
                continue;
            }
        }
        return IndexQuery::new(index)
            .completion_symbols([reference.id], EditorCompletionOrigin::Direct)
            .into_iter()
            .next();
    }

    None
}

#[derive(Debug, Clone)]
struct ParameterLabelCandidate {
    parameter: CallableParameter,
    required: bool,
    active_positional: bool,
    source_kind: SourceKind,
    origin: EditorCompletionOrigin,
    sort_index: usize,
}

fn parameter_label_candidates_for_callables(
    callables: &[EditorCompletionCandidate],
    context: &ArgumentLabelCompletionContext,
) -> Vec<ParameterLabelCandidate> {
    let mut by_name = BTreeMap::<String, ParameterLabelCandidate>::new();
    let mut order = 0usize;

    for callable in callables {
        let label = callable
            .name
            .as_deref()
            .unwrap_or(callable.display.label.as_str());
        let signature = callable
            .signature
            .as_deref()
            .or(callable.constructor_signature.as_deref());
        let Some(signature) = signature else {
            continue;
        };
        let Some(parts) = callable_signature_parts(label, signature) else {
            continue;
        };
        for (parameter_index, parameter) in parts.parameters_info.into_iter().enumerate() {
            if !starts_with_ignore_ascii_case(&parameter.name, &context.prefix) {
                continue;
            }
            if context
                .supplied_labels
                .contains(&parameter.name.to_ascii_lowercase())
            {
                continue;
            }
            let key = parameter.name.to_ascii_lowercase();
            by_name.entry(key).or_insert_with(|| {
                let required = parameter.default_text.is_none();
                let candidate = ParameterLabelCandidate {
                    parameter,
                    required,
                    active_positional: parameter_index == context.argument_index,
                    source_kind: callable.source_kind,
                    origin: callable.origin,
                    sort_index: order,
                };
                order += 1;
                candidate
            });
        }
    }

    let mut candidates = by_name.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        (!left.active_positional)
            .cmp(&(!right.active_positional))
            .then_with(|| (!left.required).cmp(&(!right.required)))
            .then_with(|| left.sort_index.cmp(&right.sort_index))
            .then_with(|| left.parameter.name.cmp(&right.parameter.name))
    });
    candidates
}

fn completion_items_for_parameter_labels(
    candidates: &[ParameterLabelCandidate],
    edit_range: LspRange,
    render_context: CompletionRenderContext<'_>,
) -> (
    Vec<LspCompletionItem>,
    BTreeMap<SourceKind, usize>,
    BTreeMap<String, usize>,
) {
    let mut source_kind_counts = BTreeMap::new();
    let mut origin_counts = BTreeMap::new();
    let items = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            *source_kind_counts.entry(candidate.source_kind).or_default() += 1;
            *origin_counts
                .entry(format!("{:?}", candidate.origin))
                .or_default() += 1;
            completion_item_for_parameter_label(candidate, edit_range, index, render_context)
        })
        .collect();
    (items, source_kind_counts, origin_counts)
}

fn completion_item_for_parameter_label(
    candidate: &ParameterLabelCandidate,
    edit_range: LspRange,
    index: usize,
    render_context: CompletionRenderContext<'_>,
) -> LspCompletionItem {
    let parameter = &candidate.parameter;
    let optionality = if candidate.required {
        "required"
    } else {
        "optional"
    };
    let detail = parameter_label_detail(parameter, optionality);
    let documentation = Some(LspMarkupContent {
        kind: "markdown".to_string(),
        value: parameter_label_documentation(parameter, optionality),
    });
    let insert_value = parameter_label_insert_value(parameter, Some(render_context));
    let command = Some(trigger_parameter_hints_command());
    let new_text = if candidate.active_positional && insert_value.contains('.') {
        insert_value.clone()
    } else if candidate.active_positional {
        parameter.name.clone()
    } else {
        format!("{}: {}", parameter.name, insert_value)
    };

    LspCompletionItem {
        label: parameter.name.clone(),
        label_details: Some(LspCompletionItemLabelDetails {
            detail: Some(format!(": {}", parameter.type_and_modifiers)),
            description: parameter
                .default_text
                .as_ref()
                .map(|default| format!("= {default}"))
                .or_else(|| Some(optionality.to_string())),
        }),
        kind: 10,
        detail: Some(detail),
        documentation,
        sort_text: Some(format!(
            "00:00:{:03}:{:03}:{:03}:{}",
            if candidate.active_positional { 0 } else { 1 },
            if candidate.required { 0 } else { 1 },
            index,
            parameter.name
        )),
        filter_text: Some(parameter.name.clone()),
        insert_text_format: Some(2),
        command,
        text_edit: LspTextEdit {
            range: edit_range,
            new_text,
        },
        required_parameter_count: usize::from(candidate.required),
        optional_parameter_count: usize::from(!candidate.required),
    }
}

fn parameter_label_detail(parameter: &CallableParameter, optionality: &str) -> String {
    let mut detail = optionality.to_string();
    if !parameter.type_and_modifiers.is_empty() {
        detail.push(' ');
        detail.push_str(&parameter.type_and_modifiers);
    }
    if let Some(default) = parameter.default_text.as_ref() {
        detail.push_str(" = ");
        detail.push_str(default);
    }
    detail
}

fn parameter_label_documentation(parameter: &CallableParameter, optionality: &str) -> String {
    let mut output = format!("**{} parameter**", optionality);
    if !parameter.type_and_modifiers.is_empty() {
        output.push_str(&format!(
            "\n\nType: `{}`",
            escape_markdown_inline_code(&parameter.type_and_modifiers)
        ));
    }
    if let Some(default) = parameter.default_text.as_ref() {
        output.push_str(&format!(
            "\n\nDefault: `{}`",
            escape_markdown_inline_code(default)
        ));
    }
    output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberVisibilityContext {
    UnqualifiedOrSelf,
    ExternalReceiver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionInsertContext {
    Normal,
    ConstructorCall,
    AttributeShorthand,
}

#[derive(Debug, Clone)]
struct ArgumentLabelCompletionContext {
    prefix: String,
    prefix_span: TextSpan,
    target: CallableTarget,
    argument_index: usize,
    supplied_labels: BTreeSet<String>,
    /// Present only for the bounded lexical query.  Parser-backed callers
    /// retain `target` as their source of callee identity.
    callee: String,
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
    insert_context: CompletionInsertContext,
    match_prefix: Option<&str>,
    render_context: CompletionRenderContext<'_>,
) -> (
    Vec<LspCompletionItem>,
    BTreeMap<SourceKind, usize>,
    BTreeMap<String, usize>,
) {
    let mut source_kind_counts = BTreeMap::new();
    let mut origin_counts = BTreeMap::new();
    let items = candidates
        .iter()
        .enumerate()
        .filter_map(|(order, candidate)| {
            *source_kind_counts.entry(candidate.source_kind).or_default() += 1;
            let origin = origin_override
                .map(str::to_string)
                .unwrap_or_else(|| format!("{:?}", candidate.origin));
            *origin_counts.entry(origin).or_default() += 1;
            completion_item_for_candidate(
                candidate,
                edit_range,
                insert_context,
                match_prefix,
                order,
                render_context,
            )
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
    let declaration_context = declaration_keyword_context(source, prefix_span.start);
    let mut candidates = Vec::new();
    let override_candidates = override_completion_candidates_for_context(
        &prefix,
        offset,
        declaration_context,
        local_index,
        workspace_index,
        game_data_index,
    );
    if !override_candidates.is_empty() {
        let source_candidates = top_level_source_completion_candidates(
            &prefix,
            mode,
            local_index,
            workspace_index,
            game_data_index,
            MAX_COMPLETION_ITEMS + 1,
        );
        timings.candidate_lookup = lookup_start.elapsed();
        let edit_range = range_for_span(source, prefix_span);
        let typed_modifiers = typed_declaration_modifiers_before(source, prefix_span.start);
        let render_start = Instant::now();
        let render_context =
            CompletionRenderContext::new(local_index, workspace_index, game_data_index);
        let (mut items, mut source_kind_counts, mut origin_counts) =
            completion_items_for_override_candidates(
                &override_candidates,
                edit_range,
                &typed_modifiers,
                &prefix,
            );
        let insert_context = completion_insert_context(source, prefix_span.start, mode);
        let (source_items, source_counts, source_origins) = completion_items_for_candidates(
            &source_candidates,
            edit_range,
            Some("TopLevel"),
            insert_context,
            Some(&prefix),
            render_context,
        );
        merge_count_maps(&mut source_kind_counts, source_counts);
        merge_count_maps(&mut origin_counts, source_origins);
        let mut keyword_items =
            keyword_completion_items(&prefix, edit_range, mode, declaration_context);
        if !keyword_items.is_empty() {
            *origin_counts.entry("Keyword".to_string()).or_default() += keyword_items.len();
            prioritize_keyword_item(&mut keyword_items, "override");
            remove_items_shadowed_by_keywords(&mut items, &keyword_items);
            keyword_items.extend(items);
            items = keyword_items;
        }
        items.extend(source_items);
        let (items, is_incomplete) = cap_completion_items(items);
        timings.item_rendering = render_start.elapsed();
        timings.total = total_start.elapsed();

        return LspCompletionReport {
            candidate_count: items.len(),
            list: LspCompletionList {
                is_incomplete,
                items,
            },
            query_quality: QueryQuality::Exact,
            recovery_reason: None,
            parse_diagnostics,
            completion_context: "override".to_string(),
            receiver_text: None,
            owner_type: None,
            prefix,
            source_kind_counts,
            origin_counts,
            failure_reason: None,
            timings,
        };
    }

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

    candidates.extend(top_level_source_completion_candidates(
        &prefix,
        mode,
        local_index,
        workspace_index,
        game_data_index,
        MAX_COMPLETION_ITEMS + 1,
    ));
    candidates = candidates
        .into_iter()
        .filter(|candidate| !is_current_prefix_self_candidate(candidate, &prefix, offset))
        .collect();
    let candidates = combine_completion_candidates(candidates);
    timings.candidate_lookup = lookup_start.elapsed();

    let edit_range = range_for_span(source, prefix_span);
    let render_start = Instant::now();
    let insert_context = completion_insert_context(source, prefix_span.start, mode);
    let render_context =
        CompletionRenderContext::new(local_index, workspace_index, game_data_index);
    let (mut items, source_kind_counts, mut origin_counts) = completion_items_for_candidates(
        &candidates,
        edit_range,
        Some("TopLevel"),
        insert_context,
        Some(&prefix),
        render_context,
    );
    let mut keyword_items =
        keyword_completion_items(&prefix, edit_range, mode, declaration_context);
    if !keyword_items.is_empty() {
        *origin_counts.entry("Keyword".to_string()).or_default() += keyword_items.len();
        remove_items_shadowed_by_keywords(&mut items, &keyword_items);
        keyword_items.extend(items);
        items = keyword_items;
    }
    let (items, is_incomplete) = cap_completion_items(items);
    timings.item_rendering = render_start.elapsed();
    timings.total = total_start.elapsed();

    LspCompletionReport {
        candidate_count: items.len(),
        list: LspCompletionList {
            is_incomplete,
            items,
        },
        query_quality: QueryQuality::Exact,
        recovery_reason: None,
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

fn top_level_source_completion_candidates(
    prefix: &str,
    mode: EditorTopLevelCompletionMode,
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    limit: usize,
) -> Vec<EditorCompletionCandidate> {
    let mut candidates = Vec::new();
    candidates
        .extend(IndexQuery::new(local_index).completion_top_level_limited(prefix, mode, limit));
    if let Some(external_index) = workspace_index {
        candidates.extend(
            IndexQuery::new(external_index).completion_top_level_limited(prefix, mode, limit),
        );
    }
    if let Some(external_index) = game_data_index {
        candidates.extend(
            IndexQuery::new(external_index).completion_top_level_limited(prefix, mode, limit),
        );
    }
    candidates
}

fn merge_count_maps<K: Ord>(target: &mut BTreeMap<K, usize>, source: BTreeMap<K, usize>) {
    for (key, count) in source {
        *target.entry(key).or_default() += count;
    }
}

fn is_current_prefix_self_candidate(
    candidate: &EditorCompletionCandidate,
    prefix: &str,
    offset: usize,
) -> bool {
    candidate
        .name
        .as_deref()
        .unwrap_or(candidate.display.label.as_str())
        == prefix
        && span_contains(candidate.selection_span, offset)
}

fn prioritize_keyword_item(items: &mut Vec<LspCompletionItem>, keyword: &str) {
    if let Some(position) = items.iter().position(|item| item.label == keyword) {
        let mut item = items.remove(position);
        item.sort_text = Some(format!("00:00:000:00:{keyword}"));
        items.insert(0, item);
    }
}

fn remove_items_shadowed_by_keywords(
    items: &mut Vec<LspCompletionItem>,
    keyword_items: &[LspCompletionItem],
) {
    let keyword_labels = keyword_items
        .iter()
        .map(|item| item.label.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    items.retain(|item| !keyword_labels.contains(&item.label.to_ascii_lowercase()));
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

fn override_completion_candidates_for_context(
    prefix: &str,
    offset: usize,
    declaration_context: bool,
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> Vec<EditorCompletionCandidate> {
    if prefix.is_empty() || !declaration_context || inside_callable_after_name(local_index, offset)
    {
        return Vec::new();
    }
    let Some(class_name) = containing_class_name(local_index, offset) else {
        return Vec::new();
    };

    let owners = containing_class_completion_owners(
        local_index,
        workspace_index,
        game_data_index,
        &class_name,
    );
    let mut candidates = Vec::new();
    for owner in owners.into_iter().skip(1) {
        candidates.extend(prefixed_candidates(
            override_candidates_for_owner(local_index, &owner),
            prefix,
        ));
        if let Some(external_index) = workspace_index {
            candidates.extend(prefixed_candidates(
                override_candidates_for_owner(external_index, &owner),
                prefix,
            ));
        }
        if let Some(external_index) = game_data_index {
            candidates.extend(prefixed_candidates(
                override_candidates_for_owner(external_index, &owner),
                prefix,
            ));
        }
    }

    combine_completion_candidates(candidates)
}

fn override_candidates_for_owner(
    index: &SymbolIndex,
    owner: &str,
) -> Vec<EditorCompletionCandidate> {
    IndexQuery::new(index)
        .completion_members_for_class(owner)
        .candidates
        .into_iter()
        .filter(is_overridable_method_candidate)
        .collect()
}

fn is_overridable_method_candidate(candidate: &EditorCompletionCandidate) -> bool {
    candidate.kind == SymbolKind::Method
        && !candidate.display.modifiers.iter().any(|modifier| {
            matches!(
                modifier.as_str(),
                "private" | "static" | "sealed" | "proto" | "external" | "native"
            )
        })
}

fn completion_items_for_override_candidates(
    candidates: &[EditorCompletionCandidate],
    edit_range: LspRange,
    typed_modifiers: &BTreeSet<String>,
    match_prefix: &str,
) -> (
    Vec<LspCompletionItem>,
    BTreeMap<SourceKind, usize>,
    BTreeMap<String, usize>,
) {
    let mut source_kind_counts = BTreeMap::new();
    let mut origin_counts = BTreeMap::new();
    let items = candidates
        .iter()
        .enumerate()
        .filter_map(|(order, candidate)| {
            *source_kind_counts.entry(candidate.source_kind).or_default() += 1;
            *origin_counts.entry("Override".to_string()).or_default() += 1;
            completion_item_for_override_candidate(
                candidate,
                edit_range,
                typed_modifiers,
                match_prefix,
                order,
            )
        })
        .collect::<Vec<_>>();

    (items, source_kind_counts, origin_counts)
}

fn completion_item_for_override_candidate(
    candidate: &EditorCompletionCandidate,
    edit_range: LspRange,
    typed_modifiers: &BTreeSet<String>,
    match_prefix: &str,
    order: usize,
) -> Option<LspCompletionItem> {
    let label = candidate
        .name
        .clone()
        .or_else(|| Some(candidate.display.label.clone()))?;
    let signature = candidate.signature.as_deref()?;
    let call = callable_signature_parts(&label, signature)?;
    let return_type = call
        .result
        .as_deref()
        .and_then(|result| result.strip_prefix("->"))
        .map(str::trim)
        .filter(|result| !result.is_empty())
        .unwrap_or("void");
    let modifiers = override_completion_modifiers(candidate, typed_modifiers);
    let declaration_prefix = if modifiers.is_empty() {
        return_type.to_string()
    } else {
        format!("{} {return_type}", modifiers.join(" "))
    };
    let declaration = format!("{declaration_prefix} {label}{}", call.parameters);
    let new_text = format!("{declaration}\n{{\n\t$0\n}}");

    Some(LspCompletionItem {
        label: label.clone(),
        label_details: Some(LspCompletionItemLabelDetails {
            detail: Some(call.parameters.clone()),
            description: call.result.clone(),
        }),
        kind: 2,
        detail: Some(declaration_prefix),
        documentation: candidate
            .display
            .documentation_preview
            .as_ref()
            .map(|preview| LspMarkupContent {
                kind: "markdown".to_string(),
                value: preview.clone(),
            }),
        sort_text: Some(completion_sort_text(
            candidate,
            &label,
            Some(match_prefix),
            order,
        )),
        filter_text: Some(label),
        insert_text_format: Some(2),
        command: None,
        text_edit: LspTextEdit {
            range: edit_range,
            new_text,
        },
        required_parameter_count: call.required_parameter_count(),
        optional_parameter_count: call.optional_parameter_count(),
    })
}

fn override_completion_modifiers(
    candidate: &EditorCompletionCandidate,
    typed_modifiers: &BTreeSet<String>,
) -> Vec<String> {
    let mut modifiers = vec!["override".to_string()];
    for modifier in &candidate.display.modifiers {
        if matches!(modifier.as_str(), "protected" | "const" | "notnull") {
            modifiers.push(modifier.clone());
        }
    }
    modifiers.retain(|modifier| !typed_modifiers.contains(modifier.as_str()));
    modifiers
}

fn typed_declaration_modifiers_before(source: &str, offset: usize) -> BTreeSet<String> {
    let mut modifiers = BTreeSet::new();
    for token in lex(source)
        .into_iter()
        .take_while(|token| token.span.end <= offset)
        .filter(|token| !token.kind.is_trivia())
    {
        match token.kind {
            TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Semicolon => {
                modifiers.clear();
            }
            TokenKind::Keyword(_) => {
                if let Some(text) = source.get(token.span.start..token.span.end) {
                    if is_override_completion_modifier_text(text) {
                        modifiers.insert(text.to_ascii_lowercase());
                    }
                }
            }
            _ => {}
        }
    }
    modifiers
}

fn is_override_completion_modifier_text(text: &str) -> bool {
    matches!(text, "override" | "protected" | "const" | "notnull")
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
        if mode == EditorTopLevelCompletionMode::Value {
            keywords.extend(TYPE_COMPLETION_KEYWORDS);
        }
    }
    keywords.sort_by(|left, right| {
        keyword_completion_sort_key(left, prefix).cmp(&keyword_completion_sort_key(right, prefix))
    });
    keywords.dedup();
    keywords
        .into_iter()
        .filter(|keyword| starts_with_ignore_ascii_case(keyword, prefix))
        .map(|keyword| LspCompletionItem {
            label: keyword.to_string(),
            label_details: None,
            kind: 14,
            detail: Some("keyword".to_string()),
            documentation: None,
            sort_text: Some(format!(
                "00:00:{:03}:{:03}:{}",
                keyword_completion_match_rank(keyword, prefix),
                keyword.chars().count(),
                keyword
            )),
            filter_text: Some(keyword.to_string()),
            insert_text_format: None,
            command: None,
            text_edit: LspTextEdit {
                range: edit_range,
                new_text: keyword.to_string(),
            },
            required_parameter_count: 0,
            optional_parameter_count: 0,
        })
        .collect()
}

fn keyword_completion_sort_key<'keyword>(
    keyword: &'keyword str,
    prefix: &str,
) -> (u16, usize, &'keyword str) {
    (
        keyword_completion_match_rank(keyword, prefix),
        keyword.chars().count(),
        keyword,
    )
}

fn keyword_completion_match_rank(keyword: &str, prefix: &str) -> u16 {
    completion_name_match_rank(keyword, prefix).unwrap_or(u16::MAX)
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

fn previous_significant_completion_token_before_span(
    tokens: &[crate::lexer::Token],
    span: TextSpan,
) -> Option<crate::lexer::Token> {
    tokens
        .iter()
        .rev()
        .find(|token| {
            token.span.end <= span.start && !token.kind.is_trivia() && token.kind != TokenKind::Eof
        })
        .copied()
}

fn token_blocks_argument_label_completion(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LineComment
            | TokenKind::DocLineComment
            | TokenKind::BlockComment
            | TokenKind::DocBlockComment
            | TokenKind::UnterminatedBlockComment
            | TokenKind::String
            | TokenKind::UnterminatedString
    )
}

fn completion_insert_context(
    source: &str,
    offset: usize,
    mode: EditorTopLevelCompletionMode,
) -> CompletionInsertContext {
    match previous_significant_token_kind(source, offset) {
        Some(TokenKind::LeftBracket) | Some(TokenKind::Keyword(crate::lexer::Keyword::New)) => {
            CompletionInsertContext::ConstructorCall
        }
        None | Some(TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Semicolon)
            if mode == EditorTopLevelCompletionMode::Type =>
        {
            CompletionInsertContext::AttributeShorthand
        }
        _ => CompletionInsertContext::Normal,
    }
}

fn containing_class_name(index: &SymbolIndex, offset: usize) -> Option<String> {
    index
        .symbols()
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Class && span_contains(symbol.span, offset))
        .min_by_key(|symbol| symbol.span.len())
        .and_then(|symbol| symbol.name.clone())
}

fn inside_callable_after_name(index: &SymbolIndex, offset: usize) -> bool {
    index.symbols().iter().any(|symbol| {
        matches!(
            symbol.kind,
            SymbolKind::Function
                | SymbolKind::Method
                | SymbolKind::Constructor
                | SymbolKind::Destructor
        ) && span_contains(symbol.span, offset)
            && offset > symbol.selection_span.end
    })
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
    completion_name_match_rank(name, prefix).is_some()
}

fn span_contains(span: TextSpan, offset: usize) -> bool {
    offset >= span.start && offset <= span.end
}

fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn argument_label_completion_context(
    source: &str,
    root: &SyntaxNode,
    offset: usize,
) -> Option<ArgumentLabelCompletionContext> {
    if offset > source.len() || !source.is_char_boundary(offset) {
        return None;
    }

    let tokens = lex(source);
    let (prefix, prefix_span) =
        completion_identifier_prefix_for_argument_label(source, &tokens, offset)?;
    if previous_significant_completion_token_before_span(&tokens, prefix_span)
        .is_some_and(|token| token.kind == TokenKind::Dot)
    {
        return None;
    }
    if !is_argument_label_position(source, prefix_span) {
        return None;
    }

    let context = callable_argument_context_at_offset(source, root, prefix_span.start)?;
    Some(ArgumentLabelCompletionContext {
        prefix,
        prefix_span,
        target: context.target,
        argument_index: context.argument_index,
        supplied_labels: context.supplied_labels,
        callee: String::new(),
    })
}

fn completion_identifier_prefix_for_argument_label(
    source: &str,
    tokens: &[crate::lexer::Token],
    offset: usize,
) -> Option<(String, TextSpan)> {
    if tokens.iter().any(|token| {
        token.span.start < offset
            && offset <= token.span.end
            && token_blocks_argument_label_completion(token.kind)
    }) {
        return None;
    }
    if let Some(token) = tokens.iter().find(|token| {
        token.kind == TokenKind::Identifier
            && token.span.start <= offset
            && offset <= token.span.end
    }) {
        return Some((
            source[token.span.start..offset].to_string(),
            TextSpan::new(token.span.start, offset),
        ));
    }
    let prefix_span = TextSpan::new(offset, offset);
    previous_significant_completion_token_before_span(tokens, prefix_span)
        .is_some_and(|token| matches!(token.kind, TokenKind::LeftParen | TokenKind::Comma))
        .then_some((String::new(), prefix_span))
}

fn is_argument_label_position(source: &str, prefix_span: TextSpan) -> bool {
    let tokens = lex(source);
    let previous = tokens
        .into_iter()
        .take_while(|token| token.span.end <= prefix_span.start)
        .filter(|token| !token.kind.is_trivia())
        .last();
    matches!(
        previous.map(|token| token.kind),
        Some(TokenKind::LeftParen | TokenKind::Comma)
    )
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

fn cap_completion_items(mut items: Vec<LspCompletionItem>) -> (Vec<LspCompletionItem>, bool) {
    items.sort_by(|left, right| {
        left.sort_text
            .as_deref()
            .unwrap_or(left.label.as_str())
            .cmp(right.sort_text.as_deref().unwrap_or(right.label.as_str()))
            .then_with(|| left.label.cmp(&right.label))
    });
    let is_incomplete = items.len() >= MAX_COMPLETION_ITEMS;
    if is_incomplete {
        items.truncate(MAX_COMPLETION_ITEMS);
    }
    (items, is_incomplete)
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
        query_quality: QueryQuality::Exact,
        recovery_reason: None,
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
    insert_context: CompletionInsertContext,
    match_prefix: Option<&str>,
    order: usize,
    render_context: CompletionRenderContext<'_>,
) -> Option<LspCompletionItem> {
    let label = candidate
        .name
        .clone()
        .or_else(|| Some(candidate.display.label.clone()))?;
    let callable = callable_completion_render(&label, candidate, insert_context, render_context);
    let detail = candidate.signature.clone().or(candidate.detail.clone());
    let label_details = callable
        .as_ref()
        .map(|render| LspCompletionItemLabelDetails {
            detail: Some(render.call.parameters.clone()),
            description: render.call.result.clone(),
        })
        .or_else(|| completion_label_details(&label, candidate));
    let (new_text, insert_text_format) = callable
        .as_ref()
        .map(|render| (render.insert_text.clone(), Some(2)))
        .unwrap_or_else(|| (label.clone(), None));
    let command = callable.as_ref().map(|_| trigger_parameter_hints_command());
    let documentation = completion_documentation(candidate, callable.as_ref());
    let required_parameter_count = callable
        .as_ref()
        .map(|render| render.call.required_parameter_count())
        .unwrap_or(0);
    let optional_parameter_count = callable
        .as_ref()
        .map(|render| render.call.optional_parameter_count())
        .unwrap_or(0);

    let text_edit = LspTextEdit {
        range: edit_range,
        new_text,
    };
    let filter_text = label.clone();
    Some(LspCompletionItem {
        label: label.clone(),
        label_details,
        kind: completion_item_kind(candidate),
        detail,
        documentation,
        sort_text: Some(completion_sort_text(candidate, &label, match_prefix, order)),
        filter_text: Some(filter_text),
        insert_text_format,
        command,
        text_edit,
        required_parameter_count,
        optional_parameter_count,
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

fn callable_completion_render(
    label: &str,
    candidate: &EditorCompletionCandidate,
    insert_context: CompletionInsertContext,
    render_context: CompletionRenderContext<'_>,
) -> Option<CallableCompletionRender> {
    let signature = match candidate.kind {
        SymbolKind::Function
        | SymbolKind::Method
        | SymbolKind::Constructor
        | SymbolKind::Destructor => candidate.signature.as_deref()?,
        SymbolKind::Class if insert_context == CompletionInsertContext::ConstructorCall => {
            candidate.constructor_signature.as_deref()?
        }
        SymbolKind::Class
            if insert_context == CompletionInsertContext::AttributeShorthand
                && is_attribute_like_completion_candidate(candidate) =>
        {
            let signature = candidate.constructor_signature.as_deref()?;
            let call = callable_signature_parts(label, signature)?;
            let insert = callable_insert_text_with_context(label, &call, Some(render_context));
            let insert_text = format!("[{}]", insert.text);
            return Some(CallableCompletionRender { call, insert_text });
        }
        _ => return None,
    };
    let call = callable_signature_parts(label, signature)?;
    let insert = callable_insert_text_with_context(label, &call, Some(render_context));
    Some(CallableCompletionRender {
        call,
        insert_text: insert.text,
    })
}

fn is_attribute_like_completion_candidate(candidate: &EditorCompletionCandidate) -> bool {
    candidate.kind == SymbolKind::Class && candidate.is_attribute_like
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallableCompletionRender {
    call: CallableSignatureParts,
    insert_text: String,
}

#[cfg(test)]
fn callable_insert_text(label: &str, call: &CallableSignatureParts) -> String {
    callable_insert_text_with_context(label, call, None).text
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallableInsertText {
    text: String,
}

fn callable_insert_text_with_context(
    label: &str,
    call: &CallableSignatureParts,
    render_context: Option<CompletionRenderContext<'_>>,
) -> CallableInsertText {
    let required = call.required_parameters().collect::<Vec<_>>();
    if call.parameters_info.is_empty() {
        return CallableInsertText {
            text: format!("{label}()"),
        };
    }
    if required.is_empty() {
        return CallableInsertText {
            text: format!("{label}($0)"),
        };
    }

    let mut enum_placeholder_seen = false;
    let mut arguments = Vec::new();
    for (index, parameter) in required.iter().enumerate() {
        let argument = if let Some(owner) = enum_parameter_owner(parameter, render_context) {
            if enum_placeholder_seen {
                break;
            }
            enum_placeholder_seen = true;
            let placeholder = format!("{owner}.");
            format!(
                "${{{}:{}}}",
                index + 1,
                snippet_placeholder_text(&placeholder)
            )
        } else {
            let placeholder = parameter.name.clone();
            format!(
                "${{{}:{}}}",
                index + 1,
                snippet_placeholder_text(&placeholder)
            )
        };
        arguments.push(argument);
    }
    let arguments = arguments.join(", ");
    CallableInsertText {
        text: format!("{label}({arguments})"),
    }
}

fn trigger_parameter_hints_command() -> LspCommand {
    LspCommand {
        title: "Trigger Parameter Hints".to_string(),
        command: COMMAND_TRIGGER_PARAMETER_HINTS.to_string(),
    }
}

fn parameter_label_insert_value(
    parameter: &CallableParameter,
    render_context: Option<CompletionRenderContext<'_>>,
) -> String {
    enum_parameter_owner(parameter, render_context)
        .map(|owner| format!("${{0:{}}}", snippet_placeholder_text(&format!("{owner}."))))
        .unwrap_or_else(|| "$0".to_string())
}

fn enum_parameter_owner(
    parameter: &CallableParameter,
    render_context: Option<CompletionRenderContext<'_>>,
) -> Option<String> {
    let owner = callable_type_owner(&parameter.type_and_modifiers)?;
    render_context?.is_enum_owner(&owner).then_some(owner)
}

fn index_has_enum_owner(index: &SymbolIndex, owner: &str) -> bool {
    index.top_level_symbols_for_name(owner).iter().any(|id| {
        index
            .symbol(*id)
            .is_some_and(|symbol| symbol.kind == SymbolKind::Enum)
    })
}

fn snippet_placeholder_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('$', "\\$")
        .replace('}', "\\}")
}

fn completion_documentation(
    candidate: &EditorCompletionCandidate,
    callable: Option<&CallableCompletionRender>,
) -> Option<LspMarkupContent> {
    let mut sections = Vec::new();
    if let Some(render) = callable {
        let parameter_docs = callable_parameter_documentation(&render.call);
        if !parameter_docs.is_empty() {
            sections.push(parameter_docs);
        }
    }
    if let Some(preview) = candidate.display.documentation_preview.as_ref() {
        if !preview.trim().is_empty() {
            sections.push(preview.clone());
        }
    }
    if sections.is_empty() {
        None
    } else {
        Some(LspMarkupContent {
            kind: "markdown".to_string(),
            value: sections.join("\n\n"),
        })
    }
}

fn callable_parameter_documentation(call: &CallableSignatureParts) -> String {
    let mut output = String::new();
    let required = call.required_parameters().collect::<Vec<_>>();
    let optional = call.optional_parameters().collect::<Vec<_>>();

    if !required.is_empty() {
        output.push_str("**Required**\n");
        for parameter in required {
            output.push_str(&format!(
                "- `{}`{}\n",
                parameter.name,
                parameter_type_suffix(parameter)
            ));
        }
    }
    if !optional.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str("**Optional**\n");
        for parameter in optional {
            output.push_str(&format!(
                "- `{}`{} = `{}`\n",
                parameter.name,
                parameter_type_suffix(parameter),
                parameter
                    .default_text
                    .as_deref()
                    .map(escape_markdown_inline_code)
                    .unwrap_or_default()
            ));
        }
    }

    output
}

fn parameter_type_suffix(parameter: &CallableParameter) -> String {
    if parameter.type_and_modifiers.is_empty() {
        String::new()
    } else {
        format!(
            ": `{}`",
            escape_markdown_inline_code(&parameter.type_and_modifiers)
        )
    }
}

fn escape_markdown_inline_code(value: &str) -> String {
    value.replace('`', "\\`")
}

fn completion_sort_text(
    candidate: &EditorCompletionCandidate,
    label: &str,
    match_prefix: Option<&str>,
    order: usize,
) -> String {
    let match_rank = match_prefix
        .and_then(|prefix| completion_name_match_rank(label, prefix))
        .unwrap_or(u16::MAX);
    format!(
        "{:03}:{:02}:{:02}:{:05}:{:03}:{}",
        match_rank,
        completion_source_rank(candidate),
        completion_origin_sort_rank(candidate.origin),
        order,
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
    fn lexical_pending_completion_uses_current_prefix_and_external_top_level_symbols() {
        let external = file_index_for_source("class GetGameMode {}").index;
        let report = completion_report_for_lexical_source_with_external_indexes(
            "getga",
            LspPosition {
                line: 0,
                character: 5,
            },
            Some(&external),
            None,
        );

        assert_eq!(report.prefix, "getga");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "GetGameMode"));
        assert_eq!(report.query_quality, QueryQuality::Unavailable);
        assert_eq!(
            report.recovery_reason.as_deref(),
            Some("current-revision-local-facts-pending")
        );
    }

    #[test]
    fn pending_completion_keeps_top_level_suggestions_when_a_large_valid_file_window_ends_in_a_comment(
    ) {
        let external = file_index_for_source("class GetGameMode {}").index;
        let mut source = "void Padding() {}\n".repeat(1_100);
        source.push_str("getga\n/*");
        source.push_str(&"comment ".repeat(400));
        source.push_str("*/\n");
        let offset = source.find("getga").unwrap() + "getga".len();

        let report = completion_report_for_lexical_source_at_offset_with_external_indexes(
            &source,
            offset,
            Some(&external),
            None,
        );

        assert!(BoundedCompletionRegion::new(&source, offset).is_some());
        assert_eq!(report.prefix, "getga");
        assert_eq!(report.query_quality, QueryQuality::Unavailable);
        assert_eq!(
            report.recovery_reason.as_deref(),
            Some("current-revision-local-facts-pending")
        );
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "GetGameMode"));
    }

    #[test]
    fn pending_member_completion_uses_top_level_fallback_without_receiver_facts() {
        let external = file_index_for_source("class GetGameMode {}").index;
        let source = "instance.getga";
        let report = completion_report_for_lexical_source_with_external_indexes(
            source,
            LspPosition {
                line: 0,
                character: source.len() as u32,
            },
            Some(&external),
            None,
        );

        assert_eq!(report.query_quality, QueryQuality::Unavailable);
        assert_eq!(
            report.completion_context,
            "member-unavailable-top-level-fallback"
        );
        assert_eq!(
            report.recovery_reason.as_deref(),
            Some("current-revision-receiver-facts-pending")
        );
        assert!(report.receiver_text.is_none());
        assert!(report.owner_type.is_none());
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "GetGameMode"));
    }

    #[test]
    fn pending_argument_completion_uses_top_level_fallback_without_argument_facts() {
        let external = file_index_for_source("class GetGameMode {}").index;
        let source = "Run(getga";
        let report = completion_report_for_lexical_source_with_external_indexes(
            source,
            LspPosition {
                line: 0,
                character: source.len() as u32,
            },
            Some(&external),
            None,
        );

        assert_eq!(report.query_quality, QueryQuality::Unavailable);
        assert_eq!(
            report.completion_context,
            "argument-unavailable-top-level-fallback"
        );
        assert_eq!(
            report.recovery_reason.as_deref(),
            Some("current-revision-argument-facts-pending")
        );
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "GetGameMode"));
    }

    #[test]
    fn current_argument_query_returns_labels_for_a_resolved_bare_callable() {
        let external = file_index_for_source(
            "void Run(int firstValue, string secondValue, bool optionalValue = true) {}",
        )
        .index;
        let source = "class Example { void Test() { Run(sec); } }";
        let offset = source.find("Run(sec").unwrap() + "Run(sec".len();

        let report = completion_report_for_current_argument_labels_at_offset_with_external_indexes(
            source,
            offset,
            Some(&external),
            None,
        )
        .expect("a valid bare callable with source-backed external facts should be exact");

        assert_eq!(report.query_quality, QueryQuality::Exact);
        assert_eq!(report.completion_context, "argument-label");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "secondValue"));
        assert!(!report
            .list
            .items
            .iter()
            .any(|item| item.label == "firstValue"));
    }

    #[test]
    fn current_local_scope_query_returns_current_locals_without_ready_analysis() {
        let source = r#"class Example
{
	void Run(int parameter)
	{
		string localValue;
		loc
	}
}"#;
        let offset = source.find("loc\n").unwrap() + 3;

        let report = completion_report_for_current_local_scope_at_offset_with_external_indexes(
            source, offset, None, None,
        )
        .expect("valid callable-local source should use the current local query");

        assert_eq!(report.query_quality, QueryQuality::Exact);
        assert_eq!(report.completion_context, "local");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "localValue"));
    }

    #[test]
    fn current_local_scope_query_declines_malformed_or_member_source() {
        let malformed = "void Run() { int localValue; loc";
        assert!(
            completion_report_for_current_local_scope_at_offset_with_external_indexes(
                malformed,
                malformed.len(),
                None,
                None,
            )
            .is_none()
        );

        let member = "void Run() { localValue.loc }";
        let offset = member.find("loc }").unwrap() + 3;
        assert!(
            completion_report_for_current_local_scope_at_offset_with_external_indexes(
                member, offset, None, None,
            )
            .is_none()
        );
    }

    #[test]
    fn current_receiver_query_resolves_simple_current_locals_parameters_and_fields() {
        let external =
            file_index_for_source("class Widget { void GetName() {} void GetSize() {} }").index;
        let source = r#"class Example
{
	Widget m_widget;
	void Run(Widget parameter)
	{
		Widget localValue;
		localValue.Get
		parameter.Get
		m_widget.Get
	}
}"#;

        for receiver in ["localValue.Get", "parameter.Get", "m_widget.Get"] {
            let offset = source.find(receiver).unwrap() + receiver.len();
            let report = completion_report_for_current_receiver_at_offset_with_external_indexes(
                source,
                offset,
                Some(&external),
                None,
            )
            .unwrap_or_else(|| panic!("{receiver} should use the current receiver query"));

            assert_eq!(report.query_quality, QueryQuality::Exact);
            assert_eq!(report.completion_context, "member");
            assert_eq!(
                report.receiver_text.as_deref(),
                Some(&receiver[..receiver.len() - 4])
            );
            assert_eq!(report.owner_type.as_deref(), Some("Widget"));
            assert!(report.list.items.iter().any(|item| item.label == "GetName"));
        }
    }

    #[test]
    fn current_receiver_query_resolves_external_zero_argument_call_paths() {
        let external = file_index_for_source(
            r#"class ChimeraGame
{
	proto external PlayerController GetPlayerController();
}

class ArmaReforgerScripted : ChimeraGame {}

ArmaReforgerScripted GetGame();

class PlayerController
{
	proto external IEntity GetControlledEntity();
}
"#,
        )
        .index;
        let source = r#"class Example
{
	void Run()
	{
		GetGame().GetPlayer
		GetGame().GetPlayerController().
	}
}
"#;
        let first_member_offset = source.find("GetPlayer\n").unwrap() + "GetPlayer".len();
        let first_member = completion_report_for_current_receiver_at_offset_with_external_indexes(
            source,
            first_member_offset,
            None,
            Some(&external),
        )
        .expect("an external root call should use the bounded receiver query");

        assert_eq!(first_member.query_quality, QueryQuality::Exact);
        assert_eq!(first_member.completion_context, "member");
        assert_eq!(first_member.receiver_text.as_deref(), Some("GetGame()"));
        assert_eq!(
            first_member.owner_type.as_deref(),
            Some("ArmaReforgerScripted")
        );
        assert!(first_member
            .list
            .items
            .iter()
            .any(|item| item.label == "GetPlayerController"));

        let chained_member_offset =
            source.find("GetPlayerController().").unwrap() + "GetPlayerController().".len();
        let chained_member =
            completion_report_for_current_receiver_at_offset_with_external_indexes(
                source,
                chained_member_offset,
                None,
                Some(&external),
            )
            .expect("an external call chain should use the bounded receiver query");

        assert_eq!(chained_member.query_quality, QueryQuality::Exact);
        assert_eq!(chained_member.completion_context, "member");
        assert_eq!(
            chained_member.receiver_text.as_deref(),
            Some("GetGame().GetPlayerController()")
        );
        assert_eq!(
            chained_member.owner_type.as_deref(),
            Some("PlayerController")
        );
        assert!(chained_member
            .list
            .items
            .iter()
            .any(|item| item.label == "GetControlledEntity"));
    }

    #[test]
    fn current_receiver_query_resolves_the_real_get_game_definition_shape() {
        let game_data = file_index_for_source(
            r#"class ChimeraGame
{
	proto external PlayerController GetPlayerController();
}

class ArmaReforgerScripted : ChimeraGame {}

ArmaReforgerScripted g_ARGame;

ArmaReforgerScripted GetGame()
{
	return g_ARGame;
}

class PlayerController
{
	proto external IEntity GetControlledEntity();
}
"#,
        )
        .index;
        let source = r#"class Example
{
	void Run()
	{
		GetGame().GetPlayer
	}
}
"#;
        let offset = source.find("GetPlayer\n").unwrap() + "GetPlayer".len();

        let report = completion_report_for_current_receiver_at_offset_with_external_indexes(
            source,
            offset,
            None,
            Some(&game_data),
        )
        .expect("a later externally indexed overload can prove the member chain");

        assert_eq!(report.owner_type.as_deref(), Some("ArmaReforgerScripted"));
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "GetPlayerController"));
    }

    #[test]
    fn current_receiver_query_recovers_external_root_calls_across_multiline_lexical_context() {
        let external = file_index_for_source(
            r#"class ChimeraGame
{
	proto external PlayerController GetPlayerController();
}

class ArmaReforgerScripted : ChimeraGame {}
ArmaReforgerScripted GetGame();
"#,
        )
        .index;
        let mut source = "void Padding() {}\n".repeat(1_100);
        source.push_str("class Example { void Run() { GetGame().GetPlayer\n/*");
        source.push_str(&"comment ".repeat(400));
        source.push_str("*/\n} }\n");
        let offset = source.find("GetPlayer\n").unwrap() + "GetPlayer".len();

        let report = completion_report_for_current_receiver_at_offset_with_external_indexes(
            &source,
            offset,
            None,
            Some(&external),
        )
        .expect("current lexical context should recover an externally proven root call");

        assert_eq!(report.query_quality, QueryQuality::Exact);
        assert_eq!(report.receiver_text.as_deref(), Some("GetGame()"));
        assert_eq!(report.owner_type.as_deref(), Some("ArmaReforgerScripted"));
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "GetPlayerController"));
    }

    #[test]
    fn oversized_ambiguous_lexical_windows_keep_the_safe_fallback() {
        let mut source = "void Padding() {}\n".repeat(15_000);
        source.push_str("GetGame().GetPlayer\n/*");
        source.push_str(&"comment ".repeat(400));
        source.push_str("*/\n");
        let offset = source.find("GetPlayer\n").unwrap() + "GetPlayer".len();

        assert!(source.len() > LEXICAL_CONTEXT_RECOVERY_MAX_SOURCE_BYTES);
        assert!(BoundedCompletionRegion::new(&source, offset).is_none());
    }

    #[test]
    fn current_receiver_query_labels_unresolved_completed_call_receivers() {
        let chained = "class Example { void Run(Widget value) { value.GetWidget().Get } }";
        let chained_offset = chained.find(".Get }").unwrap() + 4;
        let chained_report =
            completion_report_for_current_receiver_at_offset_with_external_indexes(
                chained,
                chained_offset,
                None,
                None,
            )
            .expect("a completed call receiver should retain its bounded fallback reason");
        assert_eq!(
            chained_report.recovery_reason.as_deref(),
            Some("current-revision-external-call-chain-unresolved")
        );

        let static_receiver = "class Example { void Run() { Widget.Get } }";
        assert!(
            completion_report_for_current_receiver_at_offset_with_external_indexes(
                static_receiver,
                static_receiver.find("Get }").unwrap() + 3,
                None,
                None,
            )
            .is_none()
        );

        let malformed = "class Example { void Run(Widget value) { value.Get";
        assert!(
            completion_report_for_current_receiver_at_offset_with_external_indexes(
                malformed,
                malformed.len(),
                None,
                None,
            )
            .is_none()
        );
    }

    #[test]
    fn bounded_pending_queries_admit_large_snapshots_without_file_projection() {
        // The query window is anchored at the cursor, so a large unrelated
        // prefix cannot force parse/catalog/index/scope construction on the
        // request thread.  The old implementation rejected this source at
        // 64KiB before it could answer any of these current-revision facts.
        let padding = "// unrelated background text\n".repeat(5_000);
        let external = file_index_for_source(
            "class Widget { void GetName() {} } void Run(int firstValue, string secondValue) {}",
        )
        .index;
        let source = format!(
            "{padding}class Example {{ Widget m_widget; void Test(Widget parameter) {{ Widget localValue; localValue.Get; Run(sec); loc }} }}"
        );

        let local_offset = source.rfind("loc }").unwrap() + 3;
        assert!(
            completion_report_for_current_local_scope_at_offset_with_external_indexes(
                &source,
                local_offset,
                Some(&external),
                None,
            )
            .is_some()
        );

        let member_offset = source.rfind("localValue.Get").unwrap() + "localValue.Get".len();
        let member = completion_report_for_current_receiver_at_offset_with_external_indexes(
            &source,
            member_offset,
            Some(&external),
            None,
        )
        .expect("the bounded region contains the local declaration and receiver");
        assert!(member.list.items.iter().any(|item| item.label == "GetName"));

        let argument_offset = source.rfind("Run(sec").unwrap() + "Run(sec".len();
        let argument =
            completion_report_for_current_argument_labels_at_offset_with_external_indexes(
                &source,
                argument_offset,
                Some(&external),
                None,
            )
            .expect("the bounded region resolves the captured external callable");
        assert!(argument
            .list
            .items
            .iter()
            .any(|item| item.label == "secondValue"));
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

    #[test]
    fn no_arg_callable_inserts_empty_call() {
        let call = callable_signature_parts("Run", "Example.Run() -> void").unwrap();

        assert_eq!(callable_insert_text("Run", &call), "Run()");
        assert_eq!(call.required_parameter_count(), 0);
        assert_eq!(call.optional_parameter_count(), 0);
    }

    #[test]
    fn required_callable_parameters_insert_name_placeholders() {
        let call = callable_signature_parts(
            "GetComponentsByType",
            "Example.GetComponentsByType(typename componentType, out int foundCount) -> void",
        )
        .unwrap();

        assert_eq!(
            callable_insert_text("GetComponentsByType", &call),
            "GetComponentsByType(${1:componentType}, ${2:foundCount})"
        );
        assert_eq!(call.required_parameter_count(), 2);
        assert_eq!(call.optional_parameter_count(), 0);
    }

    #[test]
    fn optional_callable_parameters_are_not_inserted() {
        let call = callable_signature_parts(
            "SendToEveryone",
            "SCR_NotificationsComponent.SendToEveryone(ENotification notificationID, int param1 = 0, string label = \"ok\") -> bool",
        )
        .unwrap();

        assert_eq!(
            callable_insert_text("SendToEveryone", &call),
            "SendToEveryone(${1:notificationID})"
        );
        assert_eq!(call.required_parameter_count(), 1);
        assert_eq!(call.optional_parameter_count(), 2);
    }

    #[test]
    fn all_optional_callable_parameters_leave_cursor_inside_call() {
        let call = callable_signature_parts(
            "Attribute",
            "Attribute(string defvalue = \"\", string uiwidget = \"auto\", int precision = 3)",
        )
        .unwrap();

        assert_eq!(callable_insert_text("Attribute", &call), "Attribute($0)");
        assert_eq!(call.required_parameter_count(), 0);
        assert_eq!(call.optional_parameter_count(), 3);
    }

    #[test]
    fn generic_parameter_types_split_at_top_level_commas_only() {
        let call = callable_signature_parts(
            "UseValues",
            "Example.UseValues(map<string, ref array<IEntity>> values, inout array<int> outValues) -> void",
        )
        .unwrap();

        assert_eq!(call.parameters_info.len(), 2);
        assert_eq!(call.parameters_info[0].name, "values");
        assert_eq!(
            call.parameters_info[0].type_and_modifiers,
            "map<string, ref array<IEntity>>"
        );
        assert_eq!(call.parameters_info[1].name, "outValues");
        assert_eq!(
            call.parameters_info[1].type_and_modifiers,
            "inout array<int>"
        );
        assert_eq!(
            callable_insert_text("UseValues", &call),
            "UseValues(${1:values}, ${2:outValues})"
        );
    }

    #[test]
    fn completion_type_owner_strips_parameter_modifiers() {
        assert_eq!(
            callable_type_owner("notnull SCR_InstigatorContextData").as_deref(),
            Some("SCR_InstigatorContextData")
        );
        assert_eq!(
            callable_type_owner("out RplChannel").as_deref(),
            Some("RplChannel")
        );
        assert_eq!(
            callable_type_owner("ref array<IEntity>").as_deref(),
            Some("array")
        );
        assert_eq!(callable_type_owner("int").as_deref(), Some("int"));
    }
}
