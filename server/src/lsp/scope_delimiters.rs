use super::external_indexes::ExternalIndexes;
use super::FileIndexAnalysis;
use crate::index::SymbolIndex;
use crate::lexer::{Keyword, Operator, TextSpan, Token, TokenKind};
use crate::model::SymbolKind;
use crate::resolver::ReferenceResolver;
use crate::syntax::{Parse, SyntaxElement, SyntaxKind, SyntaxNode};
use std::collections::BTreeMap;

pub(crate) const MAX_ACTIVE_SCOPE_DELIMITER_SOURCE_BYTES: usize = 128 * 1024;
const MAX_SCOPE_DELIMITERS: usize = 200_000;
const MAX_RESOLVED_SCOPE_DELIMITER_OWNERS: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeDelimiterAnchorKind {
    SemanticToken,
    Decorator,
    ResolvedCall,
    ResolvedConstructor,
    ResolvedIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScopeDelimiterAnchor {
    span: TextSpan,
    kind: ScopeDelimiterAnchorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScopeDelimiter {
    pub(crate) opener: TextSpan,
    pub(crate) closer: Option<TextSpan>,
    pub(crate) anchor: TextSpan,
    pub(crate) anchor_kind: ScopeDelimiterAnchorKind,
}

pub(crate) fn scope_delimiters_for_analysis(
    source: &str,
    analysis: &FileIndexAnalysis,
    external_indexes: ExternalIndexes<'_>,
    resolve_dynamic_owners: bool,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> Option<Vec<ScopeDelimiter>> {
    let mut delimiters = collect_scope_delimiters(
        &analysis.parse,
        &analysis.lexer_tokens,
        Some(&analysis.index),
        should_cancel,
    )?;
    if !resolve_dynamic_owners {
        delimiters.retain(delimiter_anchor_is_structurally_proven);
        return Some(delimiters);
    }
    let resolver = ReferenceResolver::new_with_parse_scope_and_external_indexes(
        source,
        &analysis.index,
        &analysis.parse,
        &analysis.scope,
        external_indexes.ordered(),
    );
    let mut proven = Vec::with_capacity(delimiters.len());
    let mut dynamic_owners = 0usize;
    for (index, delimiter) in delimiters.into_iter().enumerate() {
        if index % 64 == 0 && should_cancel.is_some_and(|should_cancel| should_cancel()) {
            return None;
        }
        if delimiter_anchor_is_structurally_proven(&delimiter) {
            proven.push(delimiter);
            continue;
        }
        if dynamic_owners >= MAX_RESOLVED_SCOPE_DELIMITER_OWNERS {
            continue;
        }
        dynamic_owners += 1;
        if delimiter_anchor_is_proven(&delimiter, &analysis.lexer_tokens, &resolver) {
            proven.push(delimiter);
        }
    }
    Some(proven)
}

pub(crate) fn scope_delimiters_for_syntax(
    parse: &Parse,
    lexer_tokens: &[Token],
) -> Vec<ScopeDelimiter> {
    let mut delimiters =
        collect_scope_delimiters(parse, lexer_tokens, None, None).unwrap_or_default();
    delimiters.retain(delimiter_anchor_is_structurally_proven);
    delimiters
}

fn collect_scope_delimiters(
    parse: &Parse,
    lexer_tokens: &[Token],
    index: Option<&SymbolIndex>,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> Option<Vec<ScopeDelimiter>> {
    let mut collector = DelimiterCollector {
        lexer_tokens,
        index,
        delimiters: BTreeMap::new(),
        should_cancel,
        visited_nodes: 0,
    };
    collector.collect_node(&parse.root, None)?;
    collector.collect_indexed_type_angles()?;
    Some(collector.delimiters.into_values().collect())
}

pub(crate) fn active_scope_delimiters(
    delimiters: &[ScopeDelimiter],
    offsets: &[usize],
) -> Vec<ScopeDelimiter> {
    let mut active = BTreeMap::new();
    for offset in offsets {
        let selected = delimiters
            .iter()
            .filter(|delimiter| {
                delimiter.closer.is_some_and(|closer| {
                    delimiter.opener.end <= *offset && *offset <= closer.start
                })
            })
            .min_by_key(|delimiter| {
                let closer = delimiter.closer.expect("matched delimiter");
                (
                    closer.end - delimiter.opener.start,
                    usize::MAX - delimiter.opener.start,
                )
            });
        if let Some(delimiter) = selected {
            active.insert(
                (
                    delimiter.opener.start,
                    delimiter.closer.expect("active delimiter is matched").start,
                ),
                *delimiter,
            );
        }
    }
    active.into_values().collect()
}

struct DelimiterCollector<'analysis> {
    lexer_tokens: &'analysis [Token],
    index: Option<&'analysis SymbolIndex>,
    delimiters: BTreeMap<usize, ScopeDelimiter>,
    should_cancel: Option<&'analysis dyn Fn() -> bool>,
    visited_nodes: usize,
}

impl DelimiterCollector<'_> {
    fn collect_node(
        &mut self,
        node: &SyntaxNode,
        inherited_anchor: Option<ScopeDelimiterAnchor>,
    ) -> Option<()> {
        if self.visited_nodes % 64 == 0
            && self
                .should_cancel
                .is_some_and(|should_cancel| should_cancel())
        {
            return None;
        }
        self.visited_nodes += 1;
        if node.kind == SyntaxKind::PreprocessorDirective {
            return Some(());
        }

        let anchor = self.node_anchor(node).or(inherited_anchor);
        if standard_delimiter_node(node.kind) {
            let tokens = direct_tokens(node);
            self.collect_standard_pairs(node, &tokens, anchor);
        }
        if matches!(node.kind, SyntaxKind::GenericArgList | SyntaxKind::TypeRef) {
            let tokens = direct_tokens(node);
            self.collect_angle_pairs(&tokens, anchor);
        }

        for child in &node.children {
            if let SyntaxElement::Node(child) = child {
                self.collect_node(child, anchor)?;
            }
        }
        Some(())
    }

    fn node_anchor(&self, node: &SyntaxNode) -> Option<ScopeDelimiterAnchor> {
        let (span, kind) = match node.kind {
            SyntaxKind::ClassDecl => (
                name_after_keyword(node, Keyword::Class)?.span,
                ScopeDelimiterAnchorKind::SemanticToken,
            ),
            SyntaxKind::EnumDecl => (
                name_after_keyword(node, Keyword::Enum)?.span,
                ScopeDelimiterAnchorKind::SemanticToken,
            ),
            SyntaxKind::FunctionDecl | SyntaxKind::MethodDecl => {
                let parameter_start = first_child(node, SyntaxKind::ParameterList)
                    .map_or(node.span.end, |parameters| parameters.span.start);
                (
                    last_name_token_before(node, parameter_start)?.span,
                    ScopeDelimiterAnchorKind::SemanticToken,
                )
            }
            SyntaxKind::IfStatement
            | SyntaxKind::ForStatement
            | SyntaxKind::ForeachStatement
            | SyntaxKind::WhileStatement
            | SyntaxKind::DoWhileStatement
            | SyntaxKind::SwitchStatement
            | SyntaxKind::ElseClause => (
                first_direct_keyword(node)?.span,
                ScopeDelimiterAnchorKind::SemanticToken,
            ),
            SyntaxKind::CallExpression => {
                let arguments = first_child(node, SyntaxKind::ArgumentList)?;
                (
                    last_name_token_before(node, arguments.span.start)?.span,
                    ScopeDelimiterAnchorKind::ResolvedCall,
                )
            }
            SyntaxKind::NewExpression => (
                name_after_keyword(node, Keyword::New)?.span,
                ScopeDelimiterAnchorKind::ResolvedConstructor,
            ),
            SyntaxKind::IndexExpression => {
                let opener = direct_tokens(node)
                    .into_iter()
                    .find(|token| token.kind == TokenKind::LeftBracket)?;
                (
                    last_name_token_before(node, opener.span.start)?.span,
                    ScopeDelimiterAnchorKind::ResolvedIndex,
                )
            }
            SyntaxKind::AttributeList | SyntaxKind::Attribute => (
                first_name_token(node)?.span,
                ScopeDelimiterAnchorKind::Decorator,
            ),
            SyntaxKind::InitializerExpression => (
                self.initializer_type_anchor(node)?,
                ScopeDelimiterAnchorKind::SemanticToken,
            ),
            SyntaxKind::FieldDecl | SyntaxKind::LocalDeclStatement => (
                first_child(node, SyntaxKind::TypeRef)
                    .and_then(first_name_token)?
                    .span,
                ScopeDelimiterAnchorKind::SemanticToken,
            ),
            SyntaxKind::NameExpression | SyntaxKind::TypeRef => (
                first_name_token(node)?.span,
                ScopeDelimiterAnchorKind::SemanticToken,
            ),
            _ => return None,
        };
        Some(ScopeDelimiterAnchor { span, kind })
    }

    fn initializer_type_anchor(&self, node: &SyntaxNode) -> Option<TextSpan> {
        self.index?
            .symbols()
            .iter()
            .filter(|symbol| {
                symbol.span.start <= node.span.start
                    && node.span.end <= symbol.span.end
                    && symbol.detail.type_text_span.is_some()
            })
            .min_by_key(|symbol| symbol.span.len())
            .and_then(|symbol| symbol.detail.type_text_span)
            .and_then(|span| {
                self.lexer_tokens
                    .iter()
                    .copied()
                    .find(|token| {
                        span.start <= token.span.start
                            && token.span.end <= span.end
                            && is_name_token(token.kind)
                    })
                    .map(|token| token.span)
            })
    }

    fn collect_standard_pairs(
        &mut self,
        node: &SyntaxNode,
        tokens: &[Token],
        inherited_anchor: Option<ScopeDelimiterAnchor>,
    ) {
        let mut stack: Vec<(Token, Option<ScopeDelimiterAnchor>)> = Vec::new();
        for token in tokens {
            if is_standard_opener(token.kind) {
                let anchor = if matches!(node.kind, SyntaxKind::Declarator | SyntaxKind::Parameter)
                {
                    previous_name_token(tokens, token.span.start)
                        .map(|previous| ScopeDelimiterAnchor {
                            span: previous.span,
                            kind: ScopeDelimiterAnchorKind::SemanticToken,
                        })
                        .or(inherited_anchor)
                } else {
                    inherited_anchor
                };
                stack.push((*token, anchor));
                continue;
            }
            if !is_standard_closer(token.kind) {
                continue;
            }
            let Some((opener, anchor)) = stack.last().copied() else {
                continue;
            };
            if !matching_delimiters(opener.kind, token.kind) {
                continue;
            }
            stack.pop();
            if let Some(anchor) = anchor {
                self.insert(ScopeDelimiter {
                    opener: opener.span,
                    closer: Some(token.span),
                    anchor: anchor.span,
                    anchor_kind: anchor.kind,
                });
            }
        }
        for (opener, anchor) in stack {
            if let Some(anchor) = anchor {
                self.insert(ScopeDelimiter {
                    opener: opener.span,
                    closer: None,
                    anchor: anchor.span,
                    anchor_kind: anchor.kind,
                });
            }
        }
    }

    fn collect_angle_pairs(
        &mut self,
        tokens: &[Token],
        inherited_anchor: Option<ScopeDelimiterAnchor>,
    ) {
        let mut stack: Vec<(TextSpan, Option<ScopeDelimiterAnchor>)> = Vec::new();
        for token in tokens {
            match token.kind {
                TokenKind::Operator(Operator::Less) => {
                    let anchor = previous_name_token(tokens, token.span.start)
                        .map(|previous| ScopeDelimiterAnchor {
                            span: previous.span,
                            kind: ScopeDelimiterAnchorKind::SemanticToken,
                        })
                        .or(inherited_anchor);
                    stack.push((token.span, anchor));
                }
                TokenKind::Operator(Operator::Greater) => {
                    self.close_angle(&mut stack, token.span);
                }
                TokenKind::Operator(Operator::GreaterGreater) if token.span.len() == 2 => {
                    self.close_angle(
                        &mut stack,
                        TextSpan::new(token.span.start, token.span.start + 1),
                    );
                    self.close_angle(
                        &mut stack,
                        TextSpan::new(token.span.start + 1, token.span.end),
                    );
                }
                _ => {}
            }
        }
        for (opener, anchor) in stack {
            if let Some(anchor) = anchor {
                self.insert(ScopeDelimiter {
                    opener,
                    closer: None,
                    anchor: anchor.span,
                    anchor_kind: anchor.kind,
                });
            }
        }
    }

    fn close_angle(
        &mut self,
        stack: &mut Vec<(TextSpan, Option<ScopeDelimiterAnchor>)>,
        closer: TextSpan,
    ) {
        let Some((opener, anchor)) = stack.pop() else {
            return;
        };
        if let Some(anchor) = anchor {
            self.insert(ScopeDelimiter {
                opener,
                closer: Some(closer),
                anchor: anchor.span,
                anchor_kind: anchor.kind,
            });
        }
    }

    fn collect_indexed_type_angles(&mut self) -> Option<()> {
        let Some(index) = self.index else {
            return Some(());
        };
        for (symbol_index, symbol) in index.symbols().iter().enumerate() {
            if symbol_index % 64 == 0
                && self
                    .should_cancel
                    .is_some_and(|should_cancel| should_cancel())
            {
                return None;
            }
            for span in [
                symbol.detail.type_text_span,
                symbol.detail.return_type_text_span,
                symbol.detail.base_type_span,
            ]
            .into_iter()
            .flatten()
            {
                let start = self
                    .lexer_tokens
                    .partition_point(|token| token.span.end <= span.start);
                let end = self
                    .lexer_tokens
                    .partition_point(|token| token.span.start < span.end);
                let tokens = self.lexer_tokens[start..end]
                    .iter()
                    .copied()
                    .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
                    .collect::<Vec<_>>();
                let anchor = tokens
                    .iter()
                    .copied()
                    .find(|token| is_name_token(token.kind))
                    .map(|token| ScopeDelimiterAnchor {
                        span: token.span,
                        kind: ScopeDelimiterAnchorKind::SemanticToken,
                    });
                self.collect_angle_pairs(&tokens, anchor);
            }
        }
        Some(())
    }

    fn insert(&mut self, delimiter: ScopeDelimiter) {
        if self.delimiters.len() < MAX_SCOPE_DELIMITERS {
            self.delimiters.insert(delimiter.opener.start, delimiter);
        }
    }
}

fn delimiter_anchor_is_structurally_proven(delimiter: &ScopeDelimiter) -> bool {
    matches!(
        delimiter.anchor_kind,
        ScopeDelimiterAnchorKind::SemanticToken | ScopeDelimiterAnchorKind::Decorator
    )
}

fn delimiter_anchor_is_proven(
    delimiter: &ScopeDelimiter,
    lexer_tokens: &[Token],
    resolver: &ReferenceResolver<'_, '_>,
) -> bool {
    if delimiter_anchor_is_structurally_proven(delimiter) {
        return true;
    }
    let token_index =
        lexer_tokens.partition_point(|token| token.span.start < delimiter.anchor.start);
    let Some(token) = lexer_tokens
        .get(token_index)
        .copied()
        .filter(|token| token.span == delimiter.anchor)
    else {
        return false;
    };
    if delimiter.anchor_kind == ScopeDelimiterAnchorKind::ResolvedConstructor {
        if let TokenKind::Keyword(keyword) = token.kind {
            return keyword.is_class_like_type();
        }
    }
    if token.kind != TokenKind::Identifier {
        return false;
    }
    let Some(candidate) = resolver
        .resolve_identifier_token(token.span)
        .and_then(|resolution| resolution.selected)
    else {
        return false;
    };
    match delimiter.anchor_kind {
        ScopeDelimiterAnchorKind::SemanticToken | ScopeDelimiterAnchorKind::Decorator => true,
        ScopeDelimiterAnchorKind::ResolvedCall => matches!(
            candidate.kind,
            SymbolKind::Function | SymbolKind::Method | SymbolKind::Constructor
        ),
        ScopeDelimiterAnchorKind::ResolvedConstructor => {
            matches!(candidate.kind, SymbolKind::Class | SymbolKind::Typedef)
        }
        ScopeDelimiterAnchorKind::ResolvedIndex => !matches!(
            candidate.kind,
            SymbolKind::Class
                | SymbolKind::TypeParameter
                | SymbolKind::Enum
                | SymbolKind::Typedef
                | SymbolKind::PreprocessorMacro
        ),
    }
}

fn standard_delimiter_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::AttributeList
            | SyntaxKind::AttributeArgs
            | SyntaxKind::ParameterList
            | SyntaxKind::Declarator
            | SyntaxKind::Parameter
            | SyntaxKind::Block
            | SyntaxKind::Condition
            | SyntaxKind::ForHeader
            | SyntaxKind::ForeachHeader
            | SyntaxKind::SwitchHeader
            | SyntaxKind::ParenthesizedExpression
            | SyntaxKind::CastExpression
            | SyntaxKind::ArgumentList
            | SyntaxKind::IndexExpression
            | SyntaxKind::InitializerExpression
    )
}

fn direct_tokens(node: &SyntaxNode) -> Vec<Token> {
    node.children
        .iter()
        .filter_map(|child| match child {
            SyntaxElement::Token(token) => Some(*token),
            SyntaxElement::Node(_) => None,
        })
        .collect()
}

fn first_child(node: &SyntaxNode, kind: SyntaxKind) -> Option<&SyntaxNode> {
    node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(child) if child.kind == kind => Some(&**child),
        _ => None,
    })
}

fn first_direct_keyword(node: &SyntaxNode) -> Option<Token> {
    direct_tokens(node)
        .into_iter()
        .find(|token| matches!(token.kind, TokenKind::Keyword(_)))
}

fn name_after_keyword(node: &SyntaxNode, keyword: Keyword) -> Option<Token> {
    let mut saw_keyword = false;
    descendant_tokens(node).into_iter().find(|token| {
        if saw_keyword && is_name_token(token.kind) {
            return true;
        }
        if token.kind == TokenKind::Keyword(keyword) {
            saw_keyword = true;
        }
        false
    })
}

fn first_name_token(node: &SyntaxNode) -> Option<Token> {
    descendant_tokens(node)
        .into_iter()
        .find(|token| is_name_token(token.kind))
}

fn last_name_token_before(node: &SyntaxNode, offset: usize) -> Option<Token> {
    descendant_tokens(node)
        .into_iter()
        .filter(|token| token.span.end <= offset && is_name_token(token.kind))
        .next_back()
}

fn previous_name_token(tokens: &[Token], offset: usize) -> Option<Token> {
    tokens
        .iter()
        .copied()
        .filter(|token| token.span.end <= offset && is_name_token(token.kind))
        .next_back()
}

fn descendant_tokens(node: &SyntaxNode) -> Vec<Token> {
    let mut result = Vec::new();
    collect_descendant_tokens(node, &mut result);
    result
}

fn collect_descendant_tokens(node: &SyntaxNode, result: &mut Vec<Token>) {
    for child in &node.children {
        match child {
            SyntaxElement::Token(token) => result.push(*token),
            SyntaxElement::Node(child) => collect_descendant_tokens(child, result),
        }
    }
}

fn is_name_token(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Identifier | TokenKind::Keyword(_))
}

fn is_standard_opener(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LeftBrace | TokenKind::LeftParen | TokenKind::LeftBracket
    )
}

fn is_standard_closer(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::RightBrace | TokenKind::RightParen | TokenKind::RightBracket
    )
}

fn matching_delimiters(opener: TokenKind, closer: TokenKind) -> bool {
    matches!(
        (opener, closer),
        (TokenKind::LeftBrace, TokenKind::RightBrace)
            | (TokenKind::LeftParen, TokenKind::RightParen)
            | (TokenKind::LeftBracket, TokenKind::RightBracket)
    )
}
