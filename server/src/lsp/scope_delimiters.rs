use super::FileIndexAnalysis;
use crate::index::SymbolIndex;
use crate::lexer::{Keyword, Operator, TextSpan, Token, TokenKind};
use crate::syntax::{Parse, SyntaxElement, SyntaxKind, SyntaxNode};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScopeDelimiter {
    pub(crate) opener: TextSpan,
    pub(crate) closer: Option<TextSpan>,
    pub(crate) anchor: TextSpan,
}

pub(crate) fn scope_delimiters_for_analysis(analysis: &FileIndexAnalysis) -> Vec<ScopeDelimiter> {
    collect_scope_delimiters(
        &analysis.parse,
        &analysis.lexer_tokens,
        Some(&analysis.index),
    )
}

pub(crate) fn scope_delimiters_for_syntax(
    parse: &Parse,
    lexer_tokens: &[Token],
) -> Vec<ScopeDelimiter> {
    collect_scope_delimiters(parse, lexer_tokens, None)
}

fn collect_scope_delimiters(
    parse: &Parse,
    lexer_tokens: &[Token],
    index: Option<&SymbolIndex>,
) -> Vec<ScopeDelimiter> {
    let mut collector = DelimiterCollector {
        lexer_tokens,
        index,
        delimiters: BTreeMap::new(),
    };
    collector.collect_node(&parse.root, None);
    collector.collect_indexed_type_angles();
    collector.delimiters.into_values().collect()
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
}

impl DelimiterCollector<'_> {
    fn collect_node(&mut self, node: &SyntaxNode, inherited_anchor: Option<TextSpan>) {
        if node.kind == SyntaxKind::PreprocessorDirective {
            return;
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
                self.collect_node(child, anchor);
            }
        }
    }

    fn node_anchor(&self, node: &SyntaxNode) -> Option<TextSpan> {
        match node.kind {
            SyntaxKind::ClassDecl => {
                name_after_keyword(node, Keyword::Class).map(|token| token.span)
            }
            SyntaxKind::EnumDecl => name_after_keyword(node, Keyword::Enum).map(|token| token.span),
            SyntaxKind::FunctionDecl | SyntaxKind::MethodDecl => {
                let parameter_start = first_child(node, SyntaxKind::ParameterList)
                    .map_or(node.span.end, |parameters| parameters.span.start);
                last_name_token_before(node, parameter_start).map(|token| token.span)
            }
            SyntaxKind::IfStatement
            | SyntaxKind::ForStatement
            | SyntaxKind::ForeachStatement
            | SyntaxKind::WhileStatement
            | SyntaxKind::DoWhileStatement
            | SyntaxKind::SwitchStatement
            | SyntaxKind::ElseClause => first_direct_keyword(node).map(|token| token.span),
            SyntaxKind::CallExpression => first_child(node, SyntaxKind::ArgumentList)
                .and_then(|arguments| last_name_token_before(node, arguments.span.start))
                .map(|token| token.span),
            SyntaxKind::NewExpression => {
                name_after_keyword(node, Keyword::New).map(|token| token.span)
            }
            SyntaxKind::IndexExpression => direct_tokens(node)
                .iter()
                .find(|token| token.kind == TokenKind::LeftBracket)
                .and_then(|opener| last_name_token_before(node, opener.span.start))
                .map(|token| token.span),
            SyntaxKind::AttributeList | SyntaxKind::Attribute => {
                first_name_token(node).map(|token| token.span)
            }
            SyntaxKind::InitializerExpression => self.initializer_type_anchor(node),
            SyntaxKind::FieldDecl | SyntaxKind::LocalDeclStatement => {
                first_child(node, SyntaxKind::TypeRef)
                    .and_then(first_name_token)
                    .map(|token| token.span)
            }
            SyntaxKind::NameExpression | SyntaxKind::TypeRef => {
                first_name_token(node).map(|token| token.span)
            }
            _ => None,
        }
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
        inherited_anchor: Option<TextSpan>,
    ) {
        let mut stack: Vec<(Token, Option<TextSpan>)> = Vec::new();
        for token in tokens {
            if is_standard_opener(token.kind) {
                let anchor = if matches!(node.kind, SyntaxKind::Declarator | SyntaxKind::Parameter)
                {
                    previous_name_token(tokens, token.span.start)
                        .map(|previous| previous.span)
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
                    anchor,
                });
            }
        }
        for (opener, anchor) in stack {
            if let Some(anchor) = anchor {
                self.insert(ScopeDelimiter {
                    opener: opener.span,
                    closer: None,
                    anchor,
                });
            }
        }
    }

    fn collect_angle_pairs(&mut self, tokens: &[Token], inherited_anchor: Option<TextSpan>) {
        let mut stack: Vec<(TextSpan, Option<TextSpan>)> = Vec::new();
        for token in tokens {
            match token.kind {
                TokenKind::Operator(Operator::Less) => {
                    let anchor = previous_name_token(tokens, token.span.start)
                        .map(|previous| previous.span)
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
                    anchor,
                });
            }
        }
    }

    fn close_angle(&mut self, stack: &mut Vec<(TextSpan, Option<TextSpan>)>, closer: TextSpan) {
        let Some((opener, anchor)) = stack.pop() else {
            return;
        };
        if let Some(anchor) = anchor {
            self.insert(ScopeDelimiter {
                opener,
                closer: Some(closer),
                anchor,
            });
        }
    }

    fn collect_indexed_type_angles(&mut self) {
        let Some(index) = self.index else {
            return;
        };
        for symbol in index.symbols() {
            for span in [
                symbol.detail.type_text_span,
                symbol.detail.return_type_text_span,
                symbol.detail.base_type_span,
            ]
            .into_iter()
            .flatten()
            {
                let tokens = self
                    .lexer_tokens
                    .iter()
                    .copied()
                    .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
                    .collect::<Vec<_>>();
                let anchor = tokens
                    .iter()
                    .copied()
                    .find(|token| is_name_token(token.kind))
                    .map(|token| token.span);
                self.collect_angle_pairs(&tokens, anchor);
            }
        }
    }

    fn insert(&mut self, delimiter: ScopeDelimiter) {
        self.delimiters.insert(delimiter.opener.start, delimiter);
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
