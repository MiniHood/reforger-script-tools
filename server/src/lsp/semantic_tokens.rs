use crate::index::SymbolIndex;
use crate::lexer::{lex, Keyword, TextSpan, Token, TokenKind};
use crate::lsp::external_indexes::ExternalIndexes;
use crate::lsp::{
    file_index_for_source, range_for_span, span_text, FileIndexAnalysis, LspPositionIndex, LspRange,
};
use crate::model::SymbolKind;
use crate::resolver::{CandidateSource, ReferenceCandidate, ReferenceResolver, ResolutionReason};
use crate::syntax::{SyntaxElement, SyntaxKind, SyntaxNode};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

pub(crate) const SEMANTIC_TOKEN_TYPES: &[&str] = &[
    "class",
    "enum",
    "type",
    "function",
    "reforgerField",
    "variable",
    "parameter",
    "enumMember",
    "keyword",
    "comment",
    "string",
    "number",
    "operator",
    "reforgerPunctuation",
    "reforgerPreprocessor",
    "typeParameter",
];

pub(crate) const SEMANTIC_TOKEN_MODIFIERS: &[&str] = &[
    "declaration",
    "static",
    "readonly",
    "deprecated",
    "abstract",
    "modification",
];

const SEMANTIC_MOD_DECLARATION: u32 = 1 << 0;
const SEMANTIC_MOD_STATIC: u32 = 1 << 1;
const SEMANTIC_MOD_READONLY: u32 = 1 << 2;
const SEMANTIC_MOD_MODIFICATION: u32 = 1 << 5;
const TYPE_SPAN_PRIORITY: u8 = 70;
const RESOLVER_REFERENCE_PRIORITY: u8 = 60;
const SCOPE_REFERENCE_PRIORITY: u8 = 75;
const RESOLVER_TYPE_REFERENCE_PRIORITY: u8 = 80;
const MAX_RAW_SEMANTIC_TOKENS: usize = 200_000;
/// Rich resolver coloring is optional refinement. Keeping this bounded means
/// one pathological file cannot monopolize a background worker after the
/// lexical/fast baseline is already available to the editor.
const MAX_RICH_RESOLVER_CALLS: usize = 96;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LspSemanticTokens {
    pub data: Vec<u32>,
}

/// Full-response wire form for `textDocument/semanticTokens/full`.
///
/// The current server intentionally does not advertise delta tokens. `resultId`
/// still identifies the exact lexical baseline or rich overlay that produced a
/// full response, so a later delta-capable implementation has an explicit,
/// revision-safe lifecycle to extend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspSemanticTokensFull {
    pub(crate) result_id: String,
    pub(crate) data: Vec<u32>,
}

impl LspSemanticTokensFull {
    pub(crate) fn from_tokens(result_id: String, tokens: &LspSemanticTokens) -> Self {
        Self {
            result_id,
            data: tokens.data.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspSemanticTokenProjection {
    pub tokens: LspSemanticTokens,
    pub token_count: usize,
    pub parse_diagnostics: usize,
    pub timings: LspSemanticTokenTimings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspSemanticTokenReport {
    pub tokens: LspSemanticTokens,
    pub decoded: Vec<SemanticTokenDebug>,
    pub parse_diagnostics: usize,
    pub timings: LspSemanticTokenTimings,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LspSemanticTokenTimings {
    pub lex_ms: u128,
    pub token_loop_ms: u128,
    pub resolver_ms: u128,
    pub declaration_overlay_ms: u128,
    pub type_detail_overlay_ms: u128,
    pub symbol_declaration_overlay_ms: u128,
    pub delimiter_overlay_ms: u128,
    pub sort_filter_split_ms: u128,
    pub encode_ms: u128,
    pub decode_debug_ms: u128,
    pub identifier_resolver_calls: usize,
    pub delimiter_resolver_calls: usize,
    pub delimiter_owners_reused: usize,
    pub delimiter_owners_invalidated: usize,
    pub delimiter_owners_recomputed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticTokenDebug {
    pub text: String,
    pub range: LspRange,
    pub token_type: &'static str,
    pub modifiers: Vec<&'static str>,
}

pub fn semantic_tokens_report_for_source(source: &str) -> LspSemanticTokenReport {
    let analysis = file_index_for_source(source);
    semantic_tokens_report_for_cached_analysis(source, &analysis)
}

pub fn semantic_tokens_report_for_source_with_bracket_coloring(
    source: &str,
    bracket_coloring: BracketColoringMode,
) -> LspSemanticTokenReport {
    let analysis = file_index_for_source(source);
    semantic_tokens_report_for_cached_analysis_mode(
        source,
        &analysis,
        None,
        None,
        SemanticTokenMode::Rich,
        bracket_coloring,
    )
}

pub fn fast_semantic_tokens_report_for_source(source: &str) -> LspSemanticTokenReport {
    let analysis = file_index_for_source(source);
    semantic_tokens_report_for_cached_analysis_mode(
        source,
        &analysis,
        None,
        None,
        SemanticTokenMode::Fast,
        BracketColoringMode::Semantic,
    )
}

pub fn semantic_tokens_report_for_source_with_external(
    source: &str,
    external_index: Option<&SymbolIndex>,
) -> LspSemanticTokenReport {
    let analysis = file_index_for_source(source);
    semantic_tokens_report_for_cached_analysis_with_external(source, &analysis, external_index)
}

pub fn semantic_tokens_for_source_with_external(
    source: &str,
    external_index: Option<&SymbolIndex>,
) -> LspSemanticTokenProjection {
    let analysis = file_index_for_source(source);
    semantic_tokens_for_cached_analysis_with_external(source, &analysis, external_index)
}

pub fn fast_semantic_tokens_for_source(source: &str) -> LspSemanticTokenProjection {
    let analysis = file_index_for_source(source);
    fast_semantic_tokens_for_cached_analysis(source, &analysis)
}

/// Projects only facts available from the current lexer input.
///
/// This entrypoint deliberately does not construct a `FileIndexAnalysis`: it
/// is safe to use while a newer document revision is awaiting syntax and
/// semantic analysis. It preserves the shared semantic-token encoding rules,
/// including UTF-16 positions and CRLF-aware multiline token splitting.
#[cfg(test)]
pub(crate) fn lexical_semantic_tokens_for_source(source: &str) -> LspSemanticTokenProjection {
    lexical_semantic_tokens_for_source_with_bracket_coloring(
        source,
        BracketColoringMode::Semantic,
        &BTreeSet::new(),
    )
}

pub(crate) fn lexical_semantic_tokens_for_source_with_bracket_coloring(
    source: &str,
    bracket_coloring: BracketColoringMode,
    generic_angle_offsets: &BTreeSet<usize>,
) -> LspSemanticTokenProjection {
    let lex_start = Instant::now();
    let lexer_tokens = lex(source);
    let lex_elapsed = lex_start.elapsed();
    let raw_projection = lexical_raw_tokens(
        source,
        &lexer_tokens,
        lex_elapsed,
        bracket_coloring,
        generic_angle_offsets,
        None,
    )
    .expect("lexical semantic token projection is not cancellable through this entrypoint");
    encode_lexical_projection(source, raw_projection, None)
        .expect("lexical semantic token projection is not cancellable through this entrypoint")
}

fn semantic_tokens_report_for_cached_analysis(
    source: &str,
    analysis: &FileIndexAnalysis,
) -> LspSemanticTokenReport {
    semantic_tokens_report_for_cached_analysis_with_external(source, analysis, None)
}

pub(crate) fn semantic_tokens_report_for_cached_analysis_with_external(
    source: &str,
    analysis: &FileIndexAnalysis,
    external_index: Option<&SymbolIndex>,
) -> LspSemanticTokenReport {
    semantic_tokens_report_for_cached_analysis_with_external_indexes(
        source,
        analysis,
        None,
        external_index,
    )
}

pub(crate) fn semantic_tokens_report_for_cached_analysis_with_external_indexes(
    source: &str,
    analysis: &FileIndexAnalysis,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> LspSemanticTokenReport {
    semantic_tokens_report_for_cached_analysis_mode(
        source,
        analysis,
        workspace_index,
        game_data_index,
        SemanticTokenMode::Rich,
        BracketColoringMode::Semantic,
    )
}

fn semantic_tokens_report_for_cached_analysis_mode(
    source: &str,
    analysis: &FileIndexAnalysis,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    mode: SemanticTokenMode,
    bracket_coloring: BracketColoringMode,
) -> LspSemanticTokenReport {
    let raw_projection = semantic_raw_tokens(
        source,
        analysis,
        workspace_index,
        game_data_index,
        mode,
        bracket_coloring,
        None,
        None,
    )
    .expect("semantic token reports are not cancellable");
    let decode_start = Instant::now();
    let decoded = raw_projection
        .tokens
        .iter()
        .map(|token| SemanticTokenDebug {
            text: span_text(source, token.span).to_string(),
            range: range_for_span(source, token.span),
            token_type: semantic_token_type_name(token.token_type),
            modifiers: semantic_modifier_names(token.modifiers),
        })
        .collect::<Vec<_>>();
    let encode_start = Instant::now();
    let data = encode_semantic_tokens(source, &raw_projection.tokens, None)
        .expect("semantic token reports are not cancellable");
    let mut timings = raw_projection.timings;
    timings.decode_debug_ms = decode_start.elapsed().as_millis();
    timings.encode_ms = encode_start.elapsed().as_millis();

    LspSemanticTokenReport {
        tokens: LspSemanticTokens { data },
        decoded,
        parse_diagnostics: analysis.parse_diagnostics,
        timings,
    }
}

pub(crate) fn semantic_tokens_for_cached_analysis_with_external(
    source: &str,
    analysis: &FileIndexAnalysis,
    external_index: Option<&SymbolIndex>,
) -> LspSemanticTokenProjection {
    semantic_tokens_for_cached_analysis_with_external_indexes(
        source,
        analysis,
        None,
        external_index,
    )
}

pub(crate) fn semantic_tokens_for_cached_analysis_with_external_indexes(
    source: &str,
    analysis: &FileIndexAnalysis,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> LspSemanticTokenProjection {
    semantic_tokens_for_cached_analysis_with_external_indexes_and_bracket_coloring(
        source,
        analysis,
        workspace_index,
        game_data_index,
        BracketColoringMode::Semantic,
    )
}

pub(crate) fn semantic_tokens_for_cached_analysis_with_external_indexes_and_bracket_coloring(
    source: &str,
    analysis: &FileIndexAnalysis,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    bracket_coloring: BracketColoringMode,
) -> LspSemanticTokenProjection {
    let raw_projection = semantic_raw_tokens(
        source,
        analysis,
        workspace_index,
        game_data_index,
        SemanticTokenMode::Rich,
        bracket_coloring,
        None,
        None,
    )
    .expect("rich semantic token projection is not cancellable through this entrypoint");
    encode_projection(source, analysis, raw_projection, None)
        .expect("rich semantic token projection is not cancellable through this entrypoint")
}

pub(crate) struct IncrementalSemanticTokenProjection {
    pub(crate) projection: LspSemanticTokenProjection,
    pub(crate) delimiter_owner_cache: super::scope_delimiters::DelimiterOwnerProjectionCache,
}

pub(crate) fn semantic_tokens_for_cached_analysis_with_external_indexes_incremental_cancelled(
    source: &str,
    analysis: &FileIndexAnalysis,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    bracket_coloring: BracketColoringMode,
    delimiter_cache_context: super::scope_delimiters::DelimiterProjectionCacheContext<'_>,
    should_cancel: &dyn Fn() -> bool,
) -> Option<IncrementalSemanticTokenProjection> {
    let mut raw_projection = semantic_raw_tokens(
        source,
        analysis,
        workspace_index,
        game_data_index,
        SemanticTokenMode::Rich,
        bracket_coloring,
        Some(delimiter_cache_context),
        Some(should_cancel),
    )?;
    let delimiter_owner_cache = raw_projection
        .delimiter_owner_cache
        .take()
        .expect("incremental rich projection produces a delimiter-owner cache");
    let projection = encode_projection(source, analysis, raw_projection, Some(should_cancel))?;
    Some(IncrementalSemanticTokenProjection {
        projection,
        delimiter_owner_cache,
    })
}

pub(crate) fn fast_semantic_tokens_for_cached_analysis(
    source: &str,
    analysis: &FileIndexAnalysis,
) -> LspSemanticTokenProjection {
    let raw_projection = semantic_raw_tokens(
        source,
        analysis,
        None,
        None,
        SemanticTokenMode::Fast,
        BracketColoringMode::Semantic,
        None,
        None,
    )
    .expect("fast semantic token projection is not cancellable");
    encode_projection(source, analysis, raw_projection, None)
        .expect("fast semantic token projection is not cancellable")
}

fn encode_projection(
    source: &str,
    analysis: &FileIndexAnalysis,
    raw_projection: RawSemanticTokenProjection,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> Option<LspSemanticTokenProjection> {
    let token_count = raw_projection.tokens.len();
    let encode_start = Instant::now();
    let data = encode_semantic_tokens(source, &raw_projection.tokens, should_cancel)?;
    let mut timings = raw_projection.timings;
    timings.encode_ms = encode_start.elapsed().as_millis();
    Some(LspSemanticTokenProjection {
        tokens: LspSemanticTokens { data },
        token_count,
        parse_diagnostics: analysis.parse_diagnostics,
        timings,
    })
}

fn encode_lexical_projection(
    source: &str,
    raw_projection: RawSemanticTokenProjection,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> Option<LspSemanticTokenProjection> {
    let token_count = raw_projection.tokens.len();
    let encode_start = Instant::now();
    let data = encode_semantic_tokens(source, &raw_projection.tokens, should_cancel)?;
    let mut timings = raw_projection.timings;
    timings.encode_ms = encode_start.elapsed().as_millis();
    Some(LspSemanticTokenProjection {
        tokens: LspSemanticTokens { data },
        token_count,
        // Parsing has intentionally not run on this path.
        parse_diagnostics: 0,
        timings,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawSemanticToken {
    span: TextSpan,
    token_type: u32,
    modifiers: u32,
    priority: u8,
}

#[derive(Clone)]
struct RawSemanticTokenProjection {
    tokens: Vec<RawSemanticToken>,
    timings: LspSemanticTokenTimings,
    delimiter_owner_cache: Option<super::scope_delimiters::DelimiterOwnerProjectionCache>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenMode {
    Fast,
    Rich,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BracketColoringMode {
    #[default]
    Semantic,
    Punctuation,
    VsCode,
}

fn semantic_raw_tokens(
    source: &str,
    analysis: &FileIndexAnalysis,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
    mode: SemanticTokenMode,
    bracket_coloring: BracketColoringMode,
    delimiter_request: Option<super::scope_delimiters::DelimiterProjectionCacheContext<'_>>,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> Option<RawSemanticTokenProjection> {
    let lex_elapsed = Duration::default();
    let lexer_tokens = &analysis.lexer_tokens;
    if should_cancel.is_some_and(|should_cancel| should_cancel()) {
        return None;
    }
    let mut tokens = Vec::new();
    let attribute_roles = attribute_identifier_roles(source, lexer_tokens);
    let call_roles = call_identifier_roles(lexer_tokens);
    let static_member_roles = static_member_identifier_roles(source, lexer_tokens);
    let declaration_spans = analysis
        .index
        .symbols()
        .iter()
        .filter(|symbol| {
            !symbol.selection_span.is_empty() && symbol.selection_span.end <= source.len()
        })
        .map(|symbol| (symbol.selection_span.start, symbol.selection_span.end))
        .collect::<std::collections::BTreeSet<_>>();
    let resolver = (mode == SemanticTokenMode::Rich).then(|| {
        ReferenceResolver::new_with_parse_scope_and_external_indexes(
            source,
            &analysis.index,
            &analysis.parse,
            &analysis.scope,
            ExternalIndexes::new(workspace_index, game_data_index).ordered(),
        )
    });

    let mut resolver_elapsed = Duration::default();
    let mut identifier_resolver_calls = 0usize;
    let mut resolved_identifier_kinds = BTreeMap::new();
    let token_loop_start = Instant::now();
    for (token_index, token) in lexer_tokens.iter().enumerate() {
        if token_index % 64 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel()) {
            return None;
        }
        if token.kind == TokenKind::Whitespace || token.kind == TokenKind::Eof {
            continue;
        }
        if is_preprocessor_line_token(source, *token) {
            if let Some(token_type) = preprocessor_line_semantic_type(source, *token) {
                push_raw_semantic_token(&mut tokens, raw_semantic(*token, token_type, 0, 20));
            }
            continue;
        }
        if let Some(role) = attribute_roles.get(&token_index).copied() {
            let token_type = match role {
                AttributeIdentifierRole::AttributeName => semantic_type_index("class"),
                AttributeIdentifierRole::NamedArgumentLabel => semantic_type_index("variable"),
                AttributeIdentifierRole::StaticOwner => semantic_type_index("class"),
                AttributeIdentifierRole::MemberCallName => semantic_type_index("function"),
                AttributeIdentifierRole::MemberValueName => semantic_type_index("enumMember"),
                AttributeIdentifierRole::TypeLikeUnqualifiedValue => semantic_type_index("class"),
                AttributeIdentifierRole::UnqualifiedValue => semantic_type_index("variable"),
            };
            let priority = match role {
                AttributeIdentifierRole::UnqualifiedValue => RESOLVER_REFERENCE_PRIORITY,
                AttributeIdentifierRole::TypeLikeUnqualifiedValue => TYPE_SPAN_PRIORITY,
                _ => TYPE_SPAN_PRIORITY,
            };
            push_raw_semantic_token(&mut tokens, raw_semantic(*token, token_type, 0, priority));
            if !matches!(
                role,
                AttributeIdentifierRole::UnqualifiedValue
                    | AttributeIdentifierRole::TypeLikeUnqualifiedValue
            ) || mode == SemanticTokenMode::Fast
            {
                continue;
            }
        }
        if let Some(role) = call_roles.get(&token_index).copied() {
            let token_type = match role {
                CallIdentifierRole::UnqualifiedCall => semantic_type_index("function"),
                CallIdentifierRole::MemberCall => semantic_type_index("function"),
            };
            push_raw_semantic_token(
                &mut tokens,
                raw_semantic(*token, token_type, 0, RESOLVER_REFERENCE_PRIORITY),
            );
        }
        if let Some(role) = static_member_roles.get(&token_index).copied() {
            let token_type = match role {
                StaticMemberIdentifierRole::Owner => semantic_type_index("class"),
                StaticMemberIdentifierRole::MemberValue => semantic_type_index("enumMember"),
            };
            push_raw_semantic_token(
                &mut tokens,
                raw_semantic(*token, token_type, 0, TYPE_SPAN_PRIORITY),
            );
        }
        if bracket_coloring == BracketColoringMode::VsCode
            && is_standard_bracket_token_kind(token.kind)
        {
            continue;
        }
        if let Some(token_type) = lexical_semantic_type(token.kind) {
            let priority = if is_comment_token_kind(token.kind) {
                200
            } else {
                10
            };
            push_raw_semantic_token(&mut tokens, raw_semantic(*token, token_type, 0, priority));
        }
        if token.kind == TokenKind::Identifier
            && !declaration_spans.contains(&(token.span.start, token.span.end))
        {
            if let Some(token_type) = scope_reference_semantic_type(source, analysis, *token) {
                push_raw_semantic_token(
                    &mut tokens,
                    raw_semantic(*token, token_type, 0, SCOPE_REFERENCE_PRIORITY),
                );
                continue;
            }
        }
        if token.kind == TokenKind::Identifier
            && mode == SemanticTokenMode::Rich
            && identifier_resolver_calls < MAX_RICH_RESOLVER_CALLS
        {
            if declaration_spans.contains(&(token.span.start, token.span.end)) {
                continue;
            }
            identifier_resolver_calls += 1;
            let resolver_start = Instant::now();
            let resolution = resolver
                .as_ref()
                .and_then(|resolver| resolver.resolve_identifier_token(token.span));
            resolver_elapsed += resolver_start.elapsed();
            resolved_identifier_kinds.insert(
                (token.span.start, token.span.end),
                resolution
                    .as_ref()
                    .and_then(|resolution| resolution.selected.as_ref())
                    .map(|candidate| candidate.kind),
            );
            if let Some(resolution) = resolution {
                if let Some(candidate) = resolution.selected {
                    if let Some(token_type) = candidate_semantic_type(
                        &candidate,
                        &analysis.index,
                        workspace_index,
                        game_data_index,
                    ) {
                        push_raw_semantic_token(
                            &mut tokens,
                            RawSemanticToken {
                                span: token.span,
                                token_type,
                                modifiers: 0,
                                priority: resolver_reference_priority(candidate.kind),
                            },
                        );
                    }
                }
            }
        }
    }
    if should_cancel.is_some_and(|should_cancel| should_cancel()) {
        return None;
    }
    let token_loop_elapsed = token_loop_start.elapsed();

    let type_detail_overlay_start = Instant::now();
    overlay_syntax_type_references(
        source,
        &analysis.parse.root,
        &mut tokens,
        should_cancel,
        &mut 0,
    )?;
    overlay_source_backed_type_details(source, &analysis.index, &mut tokens, should_cancel)?;
    overlay_source_backed_new_expression_types(
        source,
        &analysis.parse.root,
        &mut tokens,
        should_cancel,
        &mut 0,
    )?;
    let type_detail_overlay_elapsed = type_detail_overlay_start.elapsed();

    let declaration_overlay_start = Instant::now();
    for symbol in analysis.index.symbols() {
        if should_cancel.is_some_and(|should_cancel| should_cancel()) {
            return None;
        }
        if symbol.selection_span.is_empty() || symbol.selection_span.end > source.len() {
            continue;
        }
        let Some(token_type) = symbol_semantic_type(symbol.kind) else {
            continue;
        };
        push_raw_semantic_token(
            &mut tokens,
            RawSemanticToken {
                span: symbol.selection_span,
                token_type,
                modifiers: symbol_semantic_modifiers(symbol),
                priority: 100,
            },
        );
    }
    let declaration_overlay_elapsed = declaration_overlay_start.elapsed();

    let delimiter_overlay_start = Instant::now();
    // Reuse selected kinds resolved earlier in this same immutable snapshot.
    // Dynamic delimiter proof otherwise repeats the identical resolver query.
    let delimiter_projection = if let Some(request) = delimiter_request {
        super::scope_delimiters::semantic_scope_delimiters_for_analysis_incremental(
            source,
            analysis,
            ExternalIndexes::new(workspace_index, game_data_index),
            &resolved_identifier_kinds,
            mode == SemanticTokenMode::Rich,
            request,
            should_cancel,
        )?
    } else {
        super::scope_delimiters::semantic_scope_delimiters_for_analysis(
            source,
            analysis,
            ExternalIndexes::new(workspace_index, game_data_index),
            &resolved_identifier_kinds,
            mode == SemanticTokenMode::Rich,
            should_cancel,
        )?
    };
    let super::scope_delimiters::ScopeDelimiterProjection {
        delimiters,
        dynamic_owner_resolver_calls: delimiter_resolver_calls,
        dynamic_owners_reused: delimiter_owners_reused,
        dynamic_owners_invalidated: delimiter_owners_invalidated,
        dynamic_owners_recomputed: delimiter_owners_recomputed,
        owner_cache: delimiter_owner_cache,
    } = delimiter_projection;
    let generic_angle_offsets = generic_angle_offsets_for_delimiters(source, &delimiters);
    let operator_type = semantic_type_index("operator");
    tokens = tokens
        .into_iter()
        .flat_map(|token| {
            if token.token_type == operator_type {
                split_raw_token_around_offsets(token, &generic_angle_offsets)
            } else {
                vec![token]
            }
        })
        .collect();
    if bracket_coloring != BracketColoringMode::VsCode {
        let mut owner_types = BTreeMap::new();
        for token in &tokens {
            let entry = owner_types
                .entry((token.span.start, token.span.end))
                .or_insert((token.priority, token.token_type));
            if token.priority > entry.0 {
                *entry = (token.priority, token.token_type);
            }
        }
        for (index, delimiter) in delimiters.into_iter().enumerate() {
            if index % 64 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel()) {
                return None;
            }
            let token_type = if bracket_coloring == BracketColoringMode::Punctuation
                || delimiter.anchor_kind
                    == super::scope_delimiters::ScopeDelimiterAnchorKind::Punctuation
            {
                semantic_type_index("reforgerPunctuation")
            } else if let Some((_, token_type)) =
                owner_types.get(&(delimiter.anchor.start, delimiter.anchor.end))
            {
                *token_type
            } else {
                continue;
            };
            for span in [Some(delimiter.opener), delimiter.closer]
                .into_iter()
                .flatten()
            {
                push_raw_semantic_token(
                    &mut tokens,
                    RawSemanticToken {
                        span,
                        token_type,
                        modifiers: 0,
                        priority: 90,
                    },
                );
            }
        }
    }
    let delimiter_overlay_elapsed = delimiter_overlay_start.elapsed();

    let sort_filter_split_start = Instant::now();
    tokens.sort_by_key(|token| {
        (
            token.span.start,
            std::cmp::Reverse(token.priority),
            std::cmp::Reverse(token.span.len()),
        )
    });

    let mut filtered = Vec::new();
    for token in tokens {
        if filtered.len() % 1024 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel())
        {
            return None;
        }
        if filtered
            .last()
            .is_some_and(|last: &RawSemanticToken| token.span.start < last.span.end)
        {
            continue;
        }
        filtered.push(token);
    }
    filtered.sort_by_key(|token| (token.span.start, token.span.end));
    let tokens = split_multiline_semantic_tokens(source, filtered, should_cancel)?
        .into_iter()
        .take(MAX_RAW_SEMANTIC_TOKENS)
        .collect();
    Some(RawSemanticTokenProjection {
        tokens,
        timings: LspSemanticTokenTimings {
            lex_ms: lex_elapsed.as_millis(),
            token_loop_ms: token_loop_elapsed.as_millis(),
            resolver_ms: resolver_elapsed.as_millis(),
            declaration_overlay_ms: type_detail_overlay_elapsed.as_millis()
                + declaration_overlay_elapsed.as_millis()
                + delimiter_overlay_elapsed.as_millis(),
            type_detail_overlay_ms: type_detail_overlay_elapsed.as_millis(),
            symbol_declaration_overlay_ms: declaration_overlay_elapsed.as_millis(),
            delimiter_overlay_ms: delimiter_overlay_elapsed.as_millis(),
            sort_filter_split_ms: sort_filter_split_start.elapsed().as_millis(),
            encode_ms: 0,
            decode_debug_ms: 0,
            identifier_resolver_calls,
            delimiter_resolver_calls,
            delimiter_owners_reused,
            delimiter_owners_invalidated,
            delimiter_owners_recomputed,
        },
        delimiter_owner_cache,
    })
}

fn lexical_raw_tokens(
    source: &str,
    lexer_tokens: &[Token],
    lex_elapsed: Duration,
    bracket_coloring: BracketColoringMode,
    generic_angle_offsets: &BTreeSet<usize>,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> Option<RawSemanticTokenProjection> {
    if should_cancel.is_some_and(|should_cancel| should_cancel()) {
        return None;
    }

    let mut tokens = Vec::new();
    let attribute_roles = attribute_identifier_roles(source, lexer_tokens);
    let call_roles = call_identifier_roles(lexer_tokens);
    let static_member_roles = static_member_identifier_roles(source, lexer_tokens);
    let token_loop_start = Instant::now();

    for (token_index, token) in lexer_tokens.iter().enumerate() {
        if token_index % 64 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel()) {
            return None;
        }
        if token.kind == TokenKind::Whitespace || token.kind == TokenKind::Eof {
            continue;
        }
        if is_preprocessor_line_token(source, *token) {
            if let Some(token_type) = preprocessor_line_semantic_type(source, *token) {
                push_raw_semantic_token(&mut tokens, raw_semantic(*token, token_type, 0, 20));
            }
            continue;
        }
        if let Some(role) = attribute_roles.get(&token_index).copied() {
            let token_type = match role {
                AttributeIdentifierRole::AttributeName => semantic_type_index("class"),
                AttributeIdentifierRole::NamedArgumentLabel => semantic_type_index("variable"),
                AttributeIdentifierRole::StaticOwner => semantic_type_index("class"),
                AttributeIdentifierRole::MemberCallName => semantic_type_index("function"),
                AttributeIdentifierRole::MemberValueName => semantic_type_index("enumMember"),
                AttributeIdentifierRole::TypeLikeUnqualifiedValue => semantic_type_index("class"),
                AttributeIdentifierRole::UnqualifiedValue => semantic_type_index("variable"),
            };
            let priority = match role {
                AttributeIdentifierRole::UnqualifiedValue => RESOLVER_REFERENCE_PRIORITY,
                AttributeIdentifierRole::TypeLikeUnqualifiedValue => TYPE_SPAN_PRIORITY,
                _ => TYPE_SPAN_PRIORITY,
            };
            push_raw_semantic_token(&mut tokens, raw_semantic(*token, token_type, 0, priority));
        }
        if let Some(role) = call_roles.get(&token_index).copied() {
            let token_type = match role {
                CallIdentifierRole::UnqualifiedCall => semantic_type_index("function"),
                CallIdentifierRole::MemberCall => semantic_type_index("function"),
            };
            push_raw_semantic_token(
                &mut tokens,
                raw_semantic(*token, token_type, 0, RESOLVER_REFERENCE_PRIORITY),
            );
        }
        if let Some(role) = static_member_roles.get(&token_index).copied() {
            let token_type = match role {
                StaticMemberIdentifierRole::Owner => semantic_type_index("class"),
                StaticMemberIdentifierRole::MemberValue => semantic_type_index("enumMember"),
            };
            push_raw_semantic_token(
                &mut tokens,
                raw_semantic(*token, token_type, 0, TYPE_SPAN_PRIORITY),
            );
        }
        if bracket_coloring == BracketColoringMode::VsCode
            && is_standard_bracket_token_kind(token.kind)
        {
            continue;
        }
        let generic_angles = generic_angle_offsets
            .range(token.span.start..token.span.end)
            .copied()
            .collect::<Vec<_>>();
        if bracket_coloring != BracketColoringMode::Semantic && !generic_angles.is_empty() {
            let operator_type = semantic_type_index("operator");
            if lexical_semantic_type(token.kind) == Some(operator_type) {
                for residual in split_raw_token_around_offsets(
                    raw_semantic(*token, operator_type, 0, 10),
                    generic_angle_offsets,
                ) {
                    push_raw_semantic_token(&mut tokens, residual);
                }
            }
            if bracket_coloring == BracketColoringMode::Punctuation {
                for offset in generic_angles {
                    push_raw_semantic_token(
                        &mut tokens,
                        RawSemanticToken {
                            span: TextSpan::new(offset, offset + 1),
                            token_type: semantic_type_index("reforgerPunctuation"),
                            modifiers: 0,
                            priority: 10,
                        },
                    );
                }
            }
            continue;
        }
        if let Some(token_type) = lexical_semantic_type(token.kind) {
            let priority = if is_comment_token_kind(token.kind) {
                200
            } else {
                10
            };
            push_raw_semantic_token(&mut tokens, raw_semantic(*token, token_type, 0, priority));
        }
    }

    let sort_filter_split_start = Instant::now();
    tokens.sort_by_key(|token| {
        (
            token.span.start,
            std::cmp::Reverse(token.priority),
            std::cmp::Reverse(token.span.len()),
        )
    });
    let mut filtered = Vec::new();
    for token in tokens {
        if filtered.len() % 1024 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel())
        {
            return None;
        }
        if filtered
            .last()
            .is_some_and(|last: &RawSemanticToken| token.span.start < last.span.end)
        {
            continue;
        }
        filtered.push(token);
    }
    filtered.sort_by_key(|token| (token.span.start, token.span.end));
    let tokens = split_multiline_semantic_tokens(source, filtered, should_cancel)?
        .into_iter()
        .take(MAX_RAW_SEMANTIC_TOKENS)
        .collect();
    Some(RawSemanticTokenProjection {
        tokens,
        timings: LspSemanticTokenTimings {
            lex_ms: lex_elapsed.as_millis(),
            token_loop_ms: token_loop_start.elapsed().as_millis(),
            resolver_ms: 0,
            declaration_overlay_ms: 0,
            type_detail_overlay_ms: 0,
            symbol_declaration_overlay_ms: 0,
            delimiter_overlay_ms: 0,
            sort_filter_split_ms: sort_filter_split_start.elapsed().as_millis(),
            encode_ms: 0,
            decode_debug_ms: 0,
            identifier_resolver_calls: 0,
            delimiter_resolver_calls: 0,
            delimiter_owners_reused: 0,
            delimiter_owners_invalidated: 0,
            delimiter_owners_recomputed: 0,
        },
        delimiter_owner_cache: None,
    })
}

pub(crate) fn generic_angle_offsets_for_delimiters(
    source: &str,
    delimiters: &[super::scope_delimiters::ScopeDelimiter],
) -> BTreeSet<usize> {
    delimiters
        .iter()
        .flat_map(|delimiter| [Some(delimiter.opener), delimiter.closer])
        .flatten()
        .filter_map(|span| {
            source
                .as_bytes()
                .get(span.start)
                .is_some_and(|byte| matches!(byte, b'<' | b'>'))
                .then_some(span.start)
        })
        .collect()
}

fn split_raw_token_around_offsets(
    token: RawSemanticToken,
    excluded_offsets: &BTreeSet<usize>,
) -> Vec<RawSemanticToken> {
    let mut residual = Vec::new();
    let mut start = token.span.start;
    for offset in excluded_offsets
        .range(token.span.start..token.span.end)
        .copied()
    {
        if start < offset {
            residual.push(RawSemanticToken {
                span: TextSpan::new(start, offset),
                ..token
            });
        }
        start = offset + 1;
    }
    if start < token.span.end {
        residual.push(RawSemanticToken {
            span: TextSpan::new(start, token.span.end),
            ..token
        });
    }
    residual
}

fn is_standard_bracket_token_kind(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LeftBrace
            | TokenKind::RightBrace
            | TokenKind::LeftParen
            | TokenKind::RightParen
            | TokenKind::LeftBracket
            | TokenKind::RightBracket
    )
}

fn scope_reference_semantic_type(
    source: &str,
    analysis: &FileIndexAnalysis,
    token: Token,
) -> Option<u32> {
    if token.span.end > source.len() || token.span.start >= token.span.end {
        return None;
    }
    let name = &source[token.span.start..token.span.end];
    let symbol_id = analysis
        .scope
        .visible_symbols_named(&analysis.index, name, token.span.start)
        .into_iter()
        .next()?;
    let symbol = analysis.index.symbol(symbol_id)?;
    match symbol.kind {
        SymbolKind::Parameter | SymbolKind::LocalVariable => symbol_semantic_type(symbol.kind),
        _ => None,
    }
}

fn overlay_source_backed_type_details(
    source: &str,
    index: &SymbolIndex,
    tokens: &mut Vec<RawSemanticToken>,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> Option<()> {
    for (symbol_index, symbol) in index.symbols().iter().enumerate() {
        if symbol_index % 64 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel()) {
            return None;
        }
        if let Some(type_text_span) = symbol.detail.type_text_span {
            push_type_tokens_in_span(
                source,
                type_text_span,
                semantic_type_index("class"),
                TYPE_SPAN_PRIORITY,
                tokens,
            );
        }
        if let Some(return_type_text_span) = symbol.detail.return_type_text_span {
            push_type_tokens_in_span(
                source,
                return_type_text_span,
                semantic_type_index("class"),
                TYPE_SPAN_PRIORITY,
                tokens,
            );
        }
        if let Some(base_type_span) = symbol.detail.base_type_span {
            let Some(token_type) = base_type_semantic_type(symbol.kind) else {
                continue;
            };
            push_identifier_tokens_in_span(
                source,
                base_type_span,
                token_type,
                TYPE_SPAN_PRIORITY,
                tokens,
            );
        }
    }
    Some(())
}

fn overlay_syntax_type_references(
    source: &str,
    node: &SyntaxNode,
    tokens: &mut Vec<RawSemanticToken>,
    should_cancel: Option<&dyn Fn() -> bool>,
    visited_nodes: &mut usize,
) -> Option<()> {
    if *visited_nodes % 64 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel()) {
        return None;
    }
    *visited_nodes += 1;
    if node.kind == SyntaxKind::TypeRef {
        push_type_tokens_in_span(
            source,
            node.span,
            semantic_type_index("class"),
            TYPE_SPAN_PRIORITY,
            tokens,
        );
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            overlay_syntax_type_references(source, child, tokens, should_cancel, visited_nodes)?;
        }
    }
    Some(())
}

fn overlay_source_backed_new_expression_types(
    source: &str,
    node: &SyntaxNode,
    tokens: &mut Vec<RawSemanticToken>,
    should_cancel: Option<&dyn Fn() -> bool>,
    visited_nodes: &mut usize,
) -> Option<()> {
    if *visited_nodes % 64 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel()) {
        return None;
    }
    *visited_nodes += 1;
    if node.kind == SyntaxKind::NewExpression {
        if let Some(type_name) = first_name_expression_child(node) {
            push_identifier_tokens_in_span(
                source,
                type_name.span,
                semantic_type_index("class"),
                TYPE_SPAN_PRIORITY,
                tokens,
            );
        }
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            overlay_source_backed_new_expression_types(
                source,
                child,
                tokens,
                should_cancel,
                visited_nodes,
            )?;
        }
    }
    Some(())
}

fn first_name_expression_child(node: &SyntaxNode) -> Option<&SyntaxNode> {
    node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(child) if child.kind == SyntaxKind::NameExpression => Some(&**child),
        _ => None,
    })
}

fn base_type_semantic_type(kind: SymbolKind) -> Option<u32> {
    match kind {
        SymbolKind::Class => Some(semantic_type_index("class")),
        SymbolKind::Enum => Some(semantic_type_index("enum")),
        _ => None,
    }
}

fn push_identifier_tokens_in_span(
    source: &str,
    span: TextSpan,
    token_type: u32,
    priority: u8,
    tokens: &mut Vec<RawSemanticToken>,
) {
    if span.end > source.len() || span.start >= span.end {
        return;
    }
    for token in lex(&source[span.start..span.end]) {
        if token.kind != TokenKind::Identifier {
            continue;
        }
        push_raw_semantic_token(
            tokens,
            RawSemanticToken {
                span: TextSpan::new(span.start + token.span.start, span.start + token.span.end),
                token_type,
                modifiers: 0,
                priority,
            },
        );
    }
}

fn push_type_tokens_in_span(
    source: &str,
    span: TextSpan,
    token_type: u32,
    priority: u8,
    tokens: &mut Vec<RawSemanticToken>,
) {
    if span.end > source.len() || span.start >= span.end {
        return;
    }
    for token in lex(&source[span.start..span.end]) {
        let semantic_type = match token.kind {
            TokenKind::Identifier => Some(token_type),
            TokenKind::Keyword(keyword) => type_keyword_semantic_type(keyword),
            _ => None,
        };
        let Some(semantic_type) = semantic_type else {
            continue;
        };
        push_raw_semantic_token(
            tokens,
            RawSemanticToken {
                span: TextSpan::new(span.start + token.span.start, span.start + token.span.end),
                token_type: semantic_type,
                modifiers: 0,
                priority,
            },
        );
    }
}

fn type_keyword_semantic_type(keyword: Keyword) -> Option<u32> {
    match keyword {
        Keyword::String | Keyword::Vector => Some(semantic_type_index("class")),
        Keyword::Void | Keyword::Int | Keyword::Float | Keyword::Bool | Keyword::Typename => {
            Some(semantic_type_index("keyword"))
        }
        _ => None,
    }
}

fn raw_semantic(token: Token, token_type: u32, modifiers: u32, priority: u8) -> RawSemanticToken {
    RawSemanticToken {
        span: token.span,
        token_type,
        modifiers,
        priority,
    }
}

fn push_raw_semantic_token(tokens: &mut Vec<RawSemanticToken>, token: RawSemanticToken) {
    if tokens.len() < MAX_RAW_SEMANTIC_TOKENS {
        tokens.push(token);
    }
}

fn lexical_semantic_type(kind: TokenKind) -> Option<u32> {
    match kind {
        TokenKind::LineComment
        | TokenKind::DocLineComment
        | TokenKind::BlockComment
        | TokenKind::DocBlockComment
        | TokenKind::UnterminatedBlockComment => Some(semantic_type_index("comment")),
        TokenKind::String | TokenKind::UnterminatedString => Some(semantic_type_index("string")),
        TokenKind::Number | TokenKind::InvalidNumber => Some(semantic_type_index("number")),
        TokenKind::Keyword(keyword) if keyword.is_class_like_type() => {
            Some(semantic_type_index("class"))
        }
        TokenKind::Keyword(_) => Some(semantic_type_index("keyword")),
        TokenKind::Operator(_) => Some(semantic_type_index("operator")),
        TokenKind::LeftBrace
        | TokenKind::RightBrace
        | TokenKind::LeftParen
        | TokenKind::RightParen
        | TokenKind::LeftBracket
        | TokenKind::RightBracket
        | TokenKind::Semicolon
        | TokenKind::Colon
        | TokenKind::Comma
        | TokenKind::Dot
        | TokenKind::Question => Some(semantic_type_index("reforgerPunctuation")),
        TokenKind::Hash => Some(semantic_type_index("reforgerPreprocessor")),
        _ => None,
    }
}

fn is_comment_token_kind(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LineComment
            | TokenKind::DocLineComment
            | TokenKind::BlockComment
            | TokenKind::DocBlockComment
            | TokenKind::UnterminatedBlockComment
    )
}

fn symbol_semantic_type(kind: SymbolKind) -> Option<u32> {
    match kind {
        SymbolKind::Class => Some(semantic_type_index("class")),
        SymbolKind::TypeParameter => Some(semantic_type_index("typeParameter")),
        SymbolKind::Enum => Some(semantic_type_index("enum")),
        SymbolKind::EnumMember => Some(semantic_type_index("enumMember")),
        SymbolKind::Typedef => Some(semantic_type_index("type")),
        SymbolKind::Function => Some(semantic_type_index("function")),
        SymbolKind::GlobalField | SymbolKind::Field => Some(semantic_type_index("reforgerField")),
        SymbolKind::Constructor | SymbolKind::Destructor => Some(semantic_type_index("class")),
        SymbolKind::Method => Some(semantic_type_index("function")),
        SymbolKind::Parameter => Some(semantic_type_index("parameter")),
        SymbolKind::LocalVariable | SymbolKind::PreprocessorMacro => {
            Some(semantic_type_index("variable"))
        }
    }
}

fn candidate_semantic_type(
    candidate: &ReferenceCandidate,
    file_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> Option<u32> {
    if matches!(candidate.kind, SymbolKind::GlobalField | SymbolKind::Field)
        && candidate.reason == ResolutionReason::StaticMember
    {
        let index = match candidate.source {
            CandidateSource::FileLocal => Some(file_index),
            CandidateSource::External => {
                ExternalIndexes::new(workspace_index, game_data_index).for_candidate(candidate)
            }
        };
        if index
            .and_then(|index| index.symbol(candidate.id))
            .is_some_and(is_static_const_symbol)
        {
            return Some(semantic_type_index("enumMember"));
        }
    }

    symbol_semantic_type(candidate.kind)
}

fn is_static_const_symbol(symbol: &crate::index::IndexedSymbol) -> bool {
    let has_static = symbol.modifiers.iter().any(|modifier| modifier == "static");
    let has_const = symbol.modifiers.iter().any(|modifier| modifier == "const");
    has_static && has_const
}

fn resolver_reference_priority(kind: SymbolKind) -> u8 {
    if matches!(
        kind,
        SymbolKind::Class | SymbolKind::Enum | SymbolKind::Typedef | SymbolKind::TypeParameter
    ) {
        RESOLVER_TYPE_REFERENCE_PRIORITY
    } else {
        RESOLVER_REFERENCE_PRIORITY
    }
}

fn symbol_semantic_modifiers(symbol: &crate::index::IndexedSymbol) -> u32 {
    let mut modifiers = SEMANTIC_MOD_DECLARATION;
    for modifier in &symbol.modifiers {
        match modifier.as_str() {
            "static" => modifiers |= SEMANTIC_MOD_STATIC,
            "const" => modifiers |= SEMANTIC_MOD_READONLY,
            "modded" | "override" | "vanilla" => modifiers |= SEMANTIC_MOD_MODIFICATION,
            _ => {}
        }
    }
    modifiers
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeIdentifierRole {
    AttributeName,
    NamedArgumentLabel,
    StaticOwner,
    MemberCallName,
    MemberValueName,
    TypeLikeUnqualifiedValue,
    UnqualifiedValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallIdentifierRole {
    UnqualifiedCall,
    MemberCall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticMemberIdentifierRole {
    Owner,
    MemberValue,
}

fn static_member_identifier_roles(
    source: &str,
    tokens: &[Token],
) -> BTreeMap<usize, StaticMemberIdentifierRole> {
    let mut result = BTreeMap::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Identifier {
            continue;
        }
        let text = &source[token.span.start..token.span.end];
        let Some(next) = next_non_trivia(tokens, index) else {
            continue;
        };
        if next.kind == TokenKind::Dot && is_type_like_static_owner(text) {
            result.insert(index, StaticMemberIdentifierRole::Owner);
            continue;
        }
        let Some(previous) = previous_non_trivia(tokens, index) else {
            continue;
        };
        if previous.kind != TokenKind::Dot
            || next.kind == TokenKind::LeftParen
            || !previous_dot_owner_is_type_like(source, tokens, index)
        {
            continue;
        }
        result.insert(index, StaticMemberIdentifierRole::MemberValue);
    }
    result
}

fn previous_dot_owner_is_type_like(source: &str, tokens: &[Token], dot_right_index: usize) -> bool {
    let Some(dot_index) = tokens[..dot_right_index]
        .iter()
        .rposition(|token| !token.kind.is_trivia())
    else {
        return false;
    };
    if tokens[dot_index].kind != TokenKind::Dot {
        return false;
    }
    let Some(owner) = tokens[..dot_index]
        .iter()
        .rev()
        .find(|token| !token.kind.is_trivia())
    else {
        return false;
    };
    owner.kind == TokenKind::Identifier
        && is_type_like_static_owner(&source[owner.span.start..owner.span.end])
}

fn is_type_like_static_owner(text: &str) -> bool {
    let Some(first) = text.chars().next() else {
        return false;
    };
    first.is_ascii_uppercase()
        && text
            .chars()
            .any(|character| character.is_ascii_alphanumeric())
}

fn call_identifier_roles(tokens: &[Token]) -> BTreeMap<usize, CallIdentifierRole> {
    let mut result = BTreeMap::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Identifier {
            continue;
        }
        let Some(next) = next_non_trivia(tokens, index) else {
            continue;
        };
        let previous = previous_non_trivia(tokens, index);
        if previous.is_some_and(|previous| previous.kind == TokenKind::Dot)
            && next.kind == TokenKind::LeftParen
        {
            result.insert(index, CallIdentifierRole::MemberCall);
        } else if next.kind == TokenKind::LeftParen
            && !previous.is_some_and(|previous| {
                previous.kind == TokenKind::Dot
                    || previous.kind == TokenKind::Keyword(Keyword::New)
                    || previous.kind == TokenKind::Keyword(Keyword::Class)
                    || previous.kind == TokenKind::Keyword(Keyword::Enum)
                    || previous.kind == TokenKind::Keyword(Keyword::Typedef)
            })
        {
            result.insert(index, CallIdentifierRole::UnqualifiedCall);
        }
    }
    result
}

fn attribute_identifier_roles(
    source: &str,
    tokens: &[Token],
) -> BTreeMap<usize, AttributeIdentifierRole> {
    let mut result = BTreeMap::new();
    let mut attribute_depth = 0usize;
    let mut expect_attribute_name = false;
    for (index, token) in tokens.iter().enumerate() {
        if attribute_depth > 0 {
            if expect_attribute_name
                && matches!(token.kind, TokenKind::Identifier | TokenKind::Keyword(_))
            {
                result.insert(index, AttributeIdentifierRole::AttributeName);
                expect_attribute_name = false;
            } else if token.kind == TokenKind::Identifier
                && next_non_trivia(tokens, index).is_some_and(|next| next.kind == TokenKind::Colon)
            {
                result.insert(index, AttributeIdentifierRole::NamedArgumentLabel);
            } else if token.kind == TokenKind::Identifier {
                let previous = previous_non_trivia(tokens, index);
                let next = next_non_trivia(tokens, index);
                let role = if previous.is_some_and(|previous| previous.kind == TokenKind::Dot) {
                    if next.is_some_and(|next| next.kind == TokenKind::LeftParen) {
                        AttributeIdentifierRole::MemberCallName
                    } else {
                        AttributeIdentifierRole::MemberValueName
                    }
                } else if next.is_some_and(|next| next.kind == TokenKind::Dot) {
                    AttributeIdentifierRole::StaticOwner
                } else if is_type_like_attribute_value(&source[token.span.start..token.span.end]) {
                    AttributeIdentifierRole::TypeLikeUnqualifiedValue
                } else {
                    AttributeIdentifierRole::UnqualifiedValue
                };
                result.insert(index, role);
            } else if !token.kind.is_trivia() {
                expect_attribute_name = false;
            }

            match token.kind {
                TokenKind::LeftBracket => attribute_depth += 1,
                TokenKind::RightBracket => attribute_depth = attribute_depth.saturating_sub(1),
                _ => {}
            }
            continue;
        }

        if token.kind == TokenKind::LeftBracket && starts_attribute_context(source, tokens, index) {
            attribute_depth = 1;
            expect_attribute_name = true;
        }
    }
    result
}

fn is_type_like_attribute_value(text: &str) -> bool {
    let Some(first) = text.chars().next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    text.chars().any(|character| character.is_ascii_lowercase())
}

fn starts_attribute_context(source: &str, tokens: &[Token], index: usize) -> bool {
    if previous_non_trivia(tokens, index).is_none_or(|token| {
        matches!(
            token.kind,
            TokenKind::LeftBrace
                | TokenKind::RightBrace
                | TokenKind::RightBracket
                | TokenKind::Semicolon
        )
    }) {
        return true;
    }

    let Some(previous_index) = tokens[..index]
        .iter()
        .rposition(|token| !token.kind.is_trivia())
    else {
        return true;
    };
    let has_line_break = tokens[previous_index + 1..index].iter().any(|token| {
        token.kind.is_trivia() && source[token.span.start..token.span.end].contains(['\r', '\n'])
    });
    has_line_break
        && next_non_trivia(tokens, index).is_some_and(|token| {
            matches!(token.kind, TokenKind::Identifier | TokenKind::Keyword(_))
        })
}

fn split_multiline_semantic_tokens(
    source: &str,
    tokens: Vec<RawSemanticToken>,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> Option<Vec<RawSemanticToken>> {
    let mut result = Vec::new();
    for (token_index, token) in tokens.into_iter().enumerate() {
        if token_index % 64 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel()) {
            return None;
        }
        let mut segment_start = token.span.start;
        let mut segment_index = 0usize;
        while segment_start < token.span.end {
            if segment_index % 64 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel())
            {
                return None;
            }
            segment_index += 1;
            let line_end = source[segment_start..token.span.end]
                .find(['\r', '\n'])
                .map(|offset| segment_start + offset)
                .unwrap_or(token.span.end);
            if segment_start < line_end {
                push_raw_semantic_token(
                    &mut result,
                    RawSemanticToken {
                        span: TextSpan::new(segment_start, line_end),
                        ..token
                    },
                );
            }
            if line_end == token.span.end {
                break;
            }
            segment_start = line_end
                + if source.as_bytes()[line_end] == b'\r'
                    && source.as_bytes().get(line_end + 1) == Some(&b'\n')
                {
                    2
                } else {
                    1
                };
        }
    }
    Some(result)
}

fn encode_semantic_tokens(
    source: &str,
    tokens: &[RawSemanticToken],
    should_cancel: Option<&dyn Fn() -> bool>,
) -> Option<Vec<u32>> {
    let encoded_capacity = tokens.len().min(MAX_RAW_SEMANTIC_TOKENS).saturating_mul(5);
    let mut data = Vec::with_capacity(encoded_capacity);
    let mut previous_line = 0u32;
    let mut previous_start = 0u32;
    let line_index = LspPositionIndex::new_cancellable(source, should_cancel)?;

    for (token_index, token) in tokens.iter().take(MAX_RAW_SEMANTIC_TOKENS).enumerate() {
        if token_index % 64 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel()) {
            return None;
        }
        let start = line_index.position_for_offset(token.span.start);
        let end = line_index.position_for_offset(token.span.end);
        if start.line != end.line || end.character <= start.character {
            continue;
        }
        let delta_line = start.line.saturating_sub(previous_line);
        let delta_start = if delta_line == 0 {
            start.character.saturating_sub(previous_start)
        } else {
            start.character
        };
        data.extend([
            delta_line,
            delta_start,
            end.character - start.character,
            token.token_type,
            token.modifiers,
        ]);
        previous_line = start.line;
        previous_start = start.character;
    }

    Some(data)
}

fn semantic_type_index(name: &str) -> u32 {
    SEMANTIC_TOKEN_TYPES
        .iter()
        .position(|token_type| *token_type == name)
        .expect("semantic token type should be in legend") as u32
}

fn semantic_token_type_name(index: u32) -> &'static str {
    SEMANTIC_TOKEN_TYPES
        .get(index as usize)
        .copied()
        .unwrap_or("<unknown>")
}

fn semantic_modifier_names(modifiers: u32) -> Vec<&'static str> {
    SEMANTIC_TOKEN_MODIFIERS
        .iter()
        .enumerate()
        .filter_map(|(index, name)| ((modifiers & (1 << index)) != 0).then_some(*name))
        .collect()
}

fn previous_non_trivia(tokens: &[Token], index: usize) -> Option<Token> {
    tokens[..index]
        .iter()
        .rev()
        .copied()
        .find(|token| !token.kind.is_trivia())
}

fn next_non_trivia(tokens: &[Token], index: usize) -> Option<Token> {
    tokens
        .get(index + 1..)?
        .iter()
        .copied()
        .find(|token| !token.kind.is_trivia())
}

fn is_preprocessor_line_token(source: &str, token: Token) -> bool {
    let line_start = source[..token.span.start]
        .rfind(['\r', '\n'])
        .map(|index| index + 1)
        .unwrap_or(0);
    let before_token = &source[line_start..token.span.start];
    let from_token = &source[token.span.start..];
    before_token.trim_start().starts_with('#')
        || before_token.trim().is_empty() && from_token.trim_start().starts_with('#')
}

fn preprocessor_line_semantic_type(source: &str, token: Token) -> Option<u32> {
    if token.kind == TokenKind::Hash || is_preprocessor_directive_token(source, token) {
        return Some(semantic_type_index("reforgerPreprocessor"));
    }
    if token.kind == TokenKind::Identifier {
        return Some(semantic_type_index("variable"));
    }
    lexical_semantic_type(token.kind)
}

fn is_preprocessor_directive_token(source: &str, token: Token) -> bool {
    if token.kind != TokenKind::Identifier {
        return false;
    }
    let line_start = source[..token.span.start]
        .rfind(['\r', '\n'])
        .map(|index| index + 1)
        .unwrap_or(0);
    let before_token = &source[line_start..token.span.start];
    if !before_token.trim_start().starts_with('#') {
        return false;
    }
    let after_hash = before_token
        .trim_start()
        .strip_prefix('#')
        .unwrap_or_default();
    if !after_hash.trim().is_empty() {
        return false;
    }
    matches!(
        &source[token.span.start..token.span.end],
        "if" | "ifdef" | "ifndef" | "elif" | "else" | "endif" | "define" | "undef" | "include"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;
    use std::cell::Cell;

    #[test]
    fn lexical_projection_colours_current_malformed_source_without_analysis() {
        let source = "class Broken {\n// current edit";

        let projection = lexical_semantic_tokens_for_source(source);

        assert_eq!(projection.parse_diagnostics, 0);
        assert_eq!(projection.timings.identifier_resolver_calls, 0);
        assert_eq!(projection.tokens.data.len() % 5, 0);
        assert!(projection.token_count >= 3);
    }

    #[test]
    fn lexical_projection_encodes_utf16_positions_across_crlf_multiline_comments() {
        let source = "/* 😀\r\nstill";

        let projection = lexical_semantic_tokens_for_source(source);

        assert_eq!(
            projection.tokens.data,
            vec![
                0,
                0,
                5,
                semantic_type_index("comment"),
                0,
                1,
                0,
                5,
                semantic_type_index("comment"),
                0,
            ]
        );
    }

    #[test]
    fn unresolved_statement_identifier_keeps_the_default_editor_foreground() {
        let before_control_statement = r#"class Example
{
	void Run()
	{
		asdasdsadasd
		if (true)
			return;
	}
}
"#;
        let before_call_statement = r#"class Example
{
	void Run()
	{
		asdasdsadasd
		DoThing();
	}
}
"#;
        let before_postfix_statement = r#"class Example
{
	void Run()
	{
		int testnum = 5;
		asdasdsadasd // Still showing as class green

		testnum++;
	}
}
"#;

        for source in [
            before_control_statement,
            before_call_statement,
            before_postfix_statement,
        ] {
            let report = semantic_tokens_report_for_source(source);

            assert!(
                report
                    .decoded
                    .iter()
                    .all(|token| token.text != "asdasdsadasd"),
                "an unresolved statement identifier must not receive a semantic token"
            );
        }
    }

    #[test]
    fn lexical_bracket_modes_classify_only_parser_proven_generic_angles() {
        let source = "class Example { array<int> values; array<int>> broken; void Run() { bool less = 1 < 2; int shifted = 8 >> 1; } }";
        let generic_open = source.find("array<int>").unwrap() + "array".len();
        let generic_close = generic_open + "<int".len();
        let mixed_closers = source.find("array<int>> broken").unwrap() + "array<int".len();
        let comparison = source.find("1 < 2").unwrap() + "1 ".len();
        let shift = source.find("8 >> 1").unwrap() + "8 ".len();
        let lexer_tokens = lex(source);
        let generic_angle_offsets = generic_angle_offsets_for_delimiters(
            source,
            &super::super::scope_delimiters::scope_delimiters_for_syntax(
                &parse_source(source),
                &lexer_tokens,
            ),
        );

        let punctuation = lexical_semantic_tokens_for_source_with_bracket_coloring(
            source,
            BracketColoringMode::Punctuation,
            &generic_angle_offsets,
        );
        assert_eq!(
            lexical_token_type_at_offset(&punctuation, generic_open),
            Some(semantic_type_index("reforgerPunctuation")),
        );
        assert_eq!(
            lexical_token_type_at_offset(&punctuation, generic_close),
            Some(semantic_type_index("reforgerPunctuation")),
        );
        assert_eq!(
            lexical_token_type_at_offset(&punctuation, mixed_closers),
            Some(semantic_type_index("reforgerPunctuation")),
        );
        assert_eq!(
            lexical_token_type_at_offset(&punctuation, mixed_closers + 1),
            Some(semantic_type_index("operator")),
        );
        assert_eq!(
            lexical_token_type_at_offset(&punctuation, comparison),
            Some(semantic_type_index("operator")),
        );
        assert_eq!(
            lexical_token_type_at_offset(&punctuation, shift),
            Some(semantic_type_index("operator")),
        );

        let vscode = lexical_semantic_tokens_for_source_with_bracket_coloring(
            source,
            BracketColoringMode::VsCode,
            &generic_angle_offsets,
        );
        assert_eq!(lexical_token_type_at_offset(&vscode, generic_open), None);
        assert_eq!(lexical_token_type_at_offset(&vscode, generic_close), None);
        assert_eq!(lexical_token_type_at_offset(&vscode, mixed_closers), None);
        assert_eq!(
            lexical_token_type_at_offset(&vscode, mixed_closers + 1),
            Some(semantic_type_index("operator")),
        );
        assert_eq!(
            lexical_token_type_at_offset(&vscode, comparison),
            Some(semantic_type_index("operator")),
        );
        assert_eq!(
            lexical_token_type_at_offset(&vscode, shift),
            Some(semantic_type_index("operator")),
        );
    }

    fn lexical_token_type_at_offset(
        projection: &LspSemanticTokenProjection,
        offset: usize,
    ) -> Option<u32> {
        let mut line = 0usize;
        let mut character = 0usize;
        for token in projection.tokens.data.chunks_exact(5) {
            line += token[0] as usize;
            character = if token[0] == 0 {
                character + token[1] as usize
            } else {
                token[1] as usize
            };
            if line == 0 && character <= offset && offset < character + token[2] as usize {
                return Some(token[3]);
            }
        }
        None
    }

    #[test]
    fn rich_resolver_refinement_has_a_fixed_call_budget() {
        let source = (0..(MAX_RICH_RESOLVER_CALLS * 3))
            .map(|index| format!("Unknown{index};\n"))
            .collect::<String>();

        let projection = semantic_tokens_for_source_with_external(&source, None);

        assert!(projection.timings.identifier_resolver_calls <= MAX_RICH_RESOLVER_CALLS);
        assert!(projection.token_count >= MAX_RICH_RESOLVER_CALLS * 2);
    }

    #[test]
    fn multiline_token_expansion_respects_the_final_output_cap() {
        let source = format!("/*\n{}", "x\n".repeat(MAX_RAW_SEMANTIC_TOKENS + 1));

        let projection = fast_semantic_tokens_for_source(&source);

        assert_eq!(projection.tokens.data.len() % 5, 0);
        assert!(projection.tokens.data.len() / 5 <= MAX_RAW_SEMANTIC_TOKENS);
    }

    #[test]
    fn encoding_stops_when_cancellation_arrives_mid_projection() {
        let source = "value ".repeat(128);
        let tokens = (0..128)
            .map(|index| {
                let start = index * 6;
                RawSemanticToken {
                    span: TextSpan::new(start, start + 5),
                    token_type: semantic_type_index("variable"),
                    modifiers: 0,
                    priority: 0,
                }
            })
            .collect::<Vec<_>>();
        let checks = Cell::new(0usize);

        let result = encode_semantic_tokens(
            &source,
            &tokens,
            Some(&|| {
                checks.set(checks.get() + 1);
                checks.get() >= 2
            }),
        );

        assert!(result.is_none());
    }

    #[test]
    fn type_detail_overlay_stops_when_cancellation_arrives_mid_scan() {
        let source = (0..128)
            .map(|index| format!("class Type{index} {{ Type{index} value; }}\n"))
            .collect::<String>();
        let analysis = file_index_for_source(&source);
        let checks = Cell::new(0usize);
        let mut tokens = Vec::new();

        let result = overlay_source_backed_type_details(
            &source,
            &analysis.index,
            &mut tokens,
            Some(&|| {
                checks.set(checks.get() + 1);
                checks.get() >= 2
            }),
        );

        assert!(result.is_none());
    }

    #[test]
    fn multiline_split_stops_within_one_large_token() {
        let source = format!("/*{}*/", "line\n".repeat(256));
        let token = RawSemanticToken {
            span: TextSpan::new(0, source.len()),
            token_type: semantic_type_index("comment"),
            modifiers: 0,
            priority: 0,
        };
        let checks = Cell::new(0usize);

        let result = split_multiline_semantic_tokens(
            &source,
            vec![token],
            Some(&|| {
                checks.set(checks.get() + 1);
                checks.get() >= 3
            }),
        );

        assert!(result.is_none());
    }
}
