use crate::index::SymbolIndex;
use crate::lexer::{lex, Keyword, TextSpan, Token, TokenKind};
use crate::lsp::{
    file_index_for_source, range_for_span, span_text, FileIndexAnalysis, LspPosition, LspRange,
};
use crate::model::SymbolKind;
use crate::resolver::{CandidateSource, ReferenceCandidate, ReferenceResolver, ResolutionReason};
use crate::syntax::{SyntaxElement, SyntaxKind, SyntaxNode};
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

pub(crate) const SEMANTIC_TOKEN_TYPES: &[&str] = &[
    "class",
    "enum",
    "type",
    "function",
    "method",
    "field",
    "variable",
    "parameter",
    "enumMember",
    "keyword",
    "comment",
    "string",
    "number",
    "operator",
    "punctuation",
    "preprocessor",
    "decorator",
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
const RESOLVER_TYPE_REFERENCE_PRIORITY: u8 = 80;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LspSemanticTokens {
    pub data: Vec<u32>,
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
    pub sort_filter_split_ms: u128,
    pub encode_ms: u128,
    pub decode_debug_ms: u128,
    pub identifier_resolver_calls: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticTokenDebug {
    pub text: String,
    pub range: LspRange,
    pub token_type: &'static str,
    pub modifiers: Vec<&'static str>,
    pub color: &'static str,
}

pub fn semantic_tokens_report_for_source(source: &str) -> LspSemanticTokenReport {
    let analysis = file_index_for_source(source);
    semantic_tokens_report_for_cached_analysis(source, &analysis)
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
    let raw_projection =
        semantic_raw_tokens(source, analysis, external_index, SemanticTokenMode::Rich);
    let decode_start = Instant::now();
    let decoded = raw_projection
        .tokens
        .iter()
        .map(|token| SemanticTokenDebug {
            text: span_text(source, token.span).to_string(),
            range: range_for_span(source, token.span),
            token_type: semantic_token_type_name(token.token_type),
            modifiers: semantic_modifier_names(token.modifiers),
            color: semantic_token_color(token.token_type),
        })
        .collect::<Vec<_>>();
    let encode_start = Instant::now();
    let data = encode_semantic_tokens(source, &raw_projection.tokens);
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
    let raw_projection =
        semantic_raw_tokens(source, analysis, external_index, SemanticTokenMode::Rich);
    encode_projection(source, analysis, raw_projection)
}

pub(crate) fn fast_semantic_tokens_for_cached_analysis(
    source: &str,
    analysis: &FileIndexAnalysis,
) -> LspSemanticTokenProjection {
    let raw_projection = semantic_raw_tokens(source, analysis, None, SemanticTokenMode::Fast);
    encode_projection(source, analysis, raw_projection)
}

fn encode_projection(
    source: &str,
    analysis: &FileIndexAnalysis,
    raw_projection: RawSemanticTokenProjection,
) -> LspSemanticTokenProjection {
    let token_count = raw_projection.tokens.len();
    let encode_start = Instant::now();
    let data = encode_semantic_tokens(source, &raw_projection.tokens);
    let mut timings = raw_projection.timings;
    timings.encode_ms = encode_start.elapsed().as_millis();
    LspSemanticTokenProjection {
        tokens: LspSemanticTokens { data },
        token_count,
        parse_diagnostics: analysis.parse_diagnostics,
        timings,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawSemanticToken {
    span: TextSpan,
    token_type: u32,
    modifiers: u32,
    priority: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawSemanticTokenProjection {
    tokens: Vec<RawSemanticToken>,
    timings: LspSemanticTokenTimings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenMode {
    Fast,
    Rich,
}

fn semantic_raw_tokens(
    source: &str,
    analysis: &FileIndexAnalysis,
    external_index: Option<&SymbolIndex>,
    mode: SemanticTokenMode,
) -> RawSemanticTokenProjection {
    let lex_start = Instant::now();
    let lexer_tokens = lex(source);
    let lex_elapsed = lex_start.elapsed();
    let mut tokens = Vec::new();
    let attribute_roles = attribute_identifier_roles(source, &lexer_tokens);
    let call_roles = call_identifier_roles(&lexer_tokens);
    let static_member_roles = static_member_identifier_roles(source, &lexer_tokens);
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
        ReferenceResolver::new_with_parse_and_scope(
            source,
            &analysis.index,
            &analysis.parse,
            &analysis.scope,
            external_index,
        )
    });

    let mut resolver_elapsed = Duration::default();
    let mut identifier_resolver_calls = 0usize;
    let token_loop_start = Instant::now();
    for (token_index, token) in lexer_tokens.iter().enumerate() {
        if token.kind == TokenKind::Whitespace || token.kind == TokenKind::Eof {
            continue;
        }
        if is_preprocessor_line_token(source, *token) {
            if let Some(token_type) = preprocessor_line_semantic_type(source, *token) {
                tokens.push(raw_semantic(*token, token_type, 0, 20));
            }
            continue;
        }
        if let Some(role) = attribute_roles.get(&token_index).copied() {
            let token_type = match role {
                AttributeIdentifierRole::AttributeName => semantic_type_index("class"),
                AttributeIdentifierRole::NamedArgumentLabel => semantic_type_index("variable"),
                AttributeIdentifierRole::StaticOwner => semantic_type_index("class"),
                AttributeIdentifierRole::MemberCallName => semantic_type_index("method"),
                AttributeIdentifierRole::MemberValueName => semantic_type_index("enumMember"),
                AttributeIdentifierRole::TypeLikeUnqualifiedValue => semantic_type_index("class"),
                AttributeIdentifierRole::UnqualifiedValue => semantic_type_index("variable"),
            };
            let priority = match role {
                AttributeIdentifierRole::UnqualifiedValue => RESOLVER_REFERENCE_PRIORITY,
                AttributeIdentifierRole::TypeLikeUnqualifiedValue => TYPE_SPAN_PRIORITY,
                _ => TYPE_SPAN_PRIORITY,
            };
            tokens.push(raw_semantic(*token, token_type, 0, priority));
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
                CallIdentifierRole::MemberCall => semantic_type_index("method"),
            };
            tokens.push(raw_semantic(
                *token,
                token_type,
                0,
                RESOLVER_REFERENCE_PRIORITY,
            ));
        }
        if let Some(role) = static_member_roles.get(&token_index).copied() {
            let token_type = match role {
                StaticMemberIdentifierRole::Owner => semantic_type_index("class"),
                StaticMemberIdentifierRole::MemberValue => semantic_type_index("enumMember"),
            };
            tokens.push(raw_semantic(*token, token_type, 0, TYPE_SPAN_PRIORITY));
        }
        if let Some(token_type) = lexical_semantic_type(token.kind) {
            let priority = if is_comment_token_kind(token.kind) {
                200
            } else {
                10
            };
            tokens.push(raw_semantic(*token, token_type, 0, priority));
        }
        if token.kind == TokenKind::Identifier && mode == SemanticTokenMode::Rich {
            if declaration_spans.contains(&(token.span.start, token.span.end)) {
                continue;
            }
            identifier_resolver_calls += 1;
            let resolver_start = Instant::now();
            let resolution = resolver
                .as_ref()
                .and_then(|resolver| resolver.resolve_identifier_token(token.span));
            resolver_elapsed += resolver_start.elapsed();
            if let Some(resolution) = resolution {
                if let Some(candidate) = resolution.selected {
                    if let Some(token_type) =
                        candidate_semantic_type(&candidate, &analysis.index, external_index)
                    {
                        tokens.push(RawSemanticToken {
                            span: token.span,
                            token_type,
                            modifiers: 0,
                            priority: resolver_reference_priority(candidate.kind),
                        });
                    }
                }
            }
        }
    }
    let token_loop_elapsed = token_loop_start.elapsed();

    let type_detail_overlay_start = Instant::now();
    overlay_source_backed_type_details(source, &analysis.index, &mut tokens);
    overlay_source_backed_new_expression_types(source, &analysis.parse.root, &mut tokens);
    let type_detail_overlay_elapsed = type_detail_overlay_start.elapsed();

    let declaration_overlay_start = Instant::now();
    for symbol in analysis.index.symbols() {
        if symbol.selection_span.is_empty() || symbol.selection_span.end > source.len() {
            continue;
        }
        let Some(token_type) = symbol_semantic_type(symbol.kind) else {
            continue;
        };
        tokens.push(RawSemanticToken {
            span: symbol.selection_span,
            token_type,
            modifiers: symbol_semantic_modifiers(symbol),
            priority: 100,
        });
    }
    let declaration_overlay_elapsed = declaration_overlay_start.elapsed();

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
        if filtered
            .last()
            .is_some_and(|last: &RawSemanticToken| token.span.start < last.span.end)
        {
            continue;
        }
        filtered.push(token);
    }
    filtered.sort_by_key(|token| (token.span.start, token.span.end));
    let tokens = split_multiline_semantic_tokens(source, filtered);
    RawSemanticTokenProjection {
        tokens,
        timings: LspSemanticTokenTimings {
            lex_ms: lex_elapsed.as_millis(),
            token_loop_ms: token_loop_elapsed.as_millis(),
            resolver_ms: resolver_elapsed.as_millis(),
            declaration_overlay_ms: type_detail_overlay_elapsed.as_millis()
                + declaration_overlay_elapsed.as_millis(),
            sort_filter_split_ms: sort_filter_split_start.elapsed().as_millis(),
            encode_ms: 0,
            decode_debug_ms: 0,
            identifier_resolver_calls,
        },
    }
}

fn overlay_source_backed_type_details(
    source: &str,
    index: &SymbolIndex,
    tokens: &mut Vec<RawSemanticToken>,
) {
    for symbol in index.symbols() {
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
}

fn overlay_source_backed_new_expression_types(
    source: &str,
    node: &SyntaxNode,
    tokens: &mut Vec<RawSemanticToken>,
) {
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
            overlay_source_backed_new_expression_types(source, child, tokens);
        }
    }
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
        tokens.push(RawSemanticToken {
            span: TextSpan::new(span.start + token.span.start, span.start + token.span.end),
            token_type,
            modifiers: 0,
            priority,
        });
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
        tokens.push(RawSemanticToken {
            span: TextSpan::new(span.start + token.span.start, span.start + token.span.end),
            token_type: semantic_type,
            modifiers: 0,
            priority,
        });
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

fn lexical_semantic_type(kind: TokenKind) -> Option<u32> {
    match kind {
        TokenKind::LineComment
        | TokenKind::DocLineComment
        | TokenKind::BlockComment
        | TokenKind::DocBlockComment
        | TokenKind::UnterminatedBlockComment => Some(semantic_type_index("comment")),
        TokenKind::String | TokenKind::UnterminatedString => Some(semantic_type_index("string")),
        TokenKind::Number | TokenKind::InvalidNumber => Some(semantic_type_index("number")),
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
        | TokenKind::Question => Some(semantic_type_index("punctuation")),
        TokenKind::Hash => Some(semantic_type_index("preprocessor")),
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
        SymbolKind::GlobalField | SymbolKind::Field => Some(semantic_type_index("field")),
        SymbolKind::Method | SymbolKind::Constructor | SymbolKind::Destructor => {
            Some(semantic_type_index("method"))
        }
        SymbolKind::Parameter => Some(semantic_type_index("parameter")),
        SymbolKind::LocalVariable | SymbolKind::PreprocessorMacro => {
            Some(semantic_type_index("variable"))
        }
    }
}

fn candidate_semantic_type(
    candidate: &ReferenceCandidate,
    file_index: &SymbolIndex,
    external_index: Option<&SymbolIndex>,
) -> Option<u32> {
    if matches!(candidate.kind, SymbolKind::GlobalField | SymbolKind::Field)
        && candidate.reason == ResolutionReason::StaticMember
    {
        let index = match candidate.source {
            CandidateSource::FileLocal => Some(file_index),
            CandidateSource::External => external_index,
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
    first.is_ascii_uppercase() && text.chars().any(|character| character.is_ascii_lowercase())
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

        if token.kind == TokenKind::LeftBracket && starts_attribute_context(tokens, index) {
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

fn starts_attribute_context(tokens: &[Token], index: usize) -> bool {
    previous_non_trivia(tokens, index).is_none_or(|token| {
        matches!(
            token.kind,
            TokenKind::LeftBrace
                | TokenKind::RightBrace
                | TokenKind::RightBracket
                | TokenKind::Semicolon
        )
    })
}

fn split_multiline_semantic_tokens(
    source: &str,
    tokens: Vec<RawSemanticToken>,
) -> Vec<RawSemanticToken> {
    let mut result = Vec::new();
    for token in tokens {
        let mut segment_start = token.span.start;
        while segment_start < token.span.end {
            let line_end = source[segment_start..token.span.end]
                .find('\n')
                .map(|offset| segment_start + offset)
                .unwrap_or(token.span.end);
            if segment_start < line_end {
                result.push(RawSemanticToken {
                    span: TextSpan::new(segment_start, line_end),
                    ..token
                });
            }
            if line_end == token.span.end {
                break;
            }
            segment_start = line_end + 1;
        }
    }
    result
}

fn encode_semantic_tokens(source: &str, tokens: &[RawSemanticToken]) -> Vec<u32> {
    let mut data = Vec::with_capacity(tokens.len() * 5);
    let mut previous_line = 0u32;
    let mut previous_start = 0u32;
    let line_index = SemanticLineIndex::new(source);

    for token in tokens {
        let start = line_index.position_for_offset(source, token.span.start);
        let end = line_index.position_for_offset(source, token.span.end);
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

    data
}

struct SemanticLineIndex {
    line_starts: Vec<usize>,
}

impl SemanticLineIndex {
    fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (index, character) in source.char_indices() {
            if character == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    fn position_for_offset(&self, source: &str, offset: usize) -> LspPosition {
        let bounded_offset = offset.min(source.len());
        let line_index = self
            .line_starts
            .partition_point(|line_start| *line_start <= bounded_offset)
            .saturating_sub(1);
        let line_start = self.line_starts.get(line_index).copied().unwrap_or(0);
        let character = source[line_start..bounded_offset]
            .chars()
            .map(|character| character.len_utf16() as u32)
            .sum();
        LspPosition {
            line: line_index as u32,
            character,
        }
    }
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

pub(crate) fn semantic_token_color_for_type(token_type: &str) -> &'static str {
    match token_type {
        "class" | "enum" | "type" | "typeParameter" => "#40b5ac",
        "function" | "method" => "#f3ad58",
        "enumMember" | "variable" | "field" | "parameter" | "number" => "#cfcfcf",
        "keyword" => "#59A6E9",
        "comment" => "#59aa59",
        "string" => "#c178dd",
        "operator" | "punctuation" => "#bfbfbf",
        "preprocessor" | "decorator" => "#d4fd95",
        _ => "<default>",
    }
}

fn semantic_token_color(token_type: u32) -> &'static str {
    semantic_token_color_for_type(semantic_token_type_name(token_type))
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
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let before_token = &source[line_start..token.span.start];
    let from_token = &source[token.span.start..];
    before_token.trim_start().starts_with('#')
        || before_token.trim().is_empty() && from_token.trim_start().starts_with('#')
}

fn preprocessor_line_semantic_type(source: &str, token: Token) -> Option<u32> {
    if token.kind == TokenKind::Hash || is_preprocessor_directive_token(source, token) {
        return Some(semantic_type_index("preprocessor"));
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
        .rfind('\n')
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
