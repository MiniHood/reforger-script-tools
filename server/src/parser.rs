use crate::lexer::{lex, Keyword, Operator, TextSpan, Token, TokenKind};
use crate::syntax::{Parse, ParseDiagnostic, SyntaxElement, SyntaxKind, SyntaxNode};

pub fn parse_source(source: &str) -> Parse {
    let tokens = lex(source);
    let mut diagnostics = lexer_diagnostics(&tokens);
    let mut parser = Parser {
        source,
        tokens,
        position: 0,
        diagnostics: Vec::new(),
    };
    let root = parser.parse_source_file();
    diagnostics.extend(parser.diagnostics);

    Parse { root, diagnostics }
}

fn lexer_diagnostics(tokens: &[Token]) -> Vec<ParseDiagnostic> {
    tokens
        .iter()
        .filter(|token| token.kind.is_error())
        .map(|token| ParseDiagnostic {
            message: format!("Lexer error token: {:?}", token.kind),
            span: token.span,
        })
        .collect()
}

struct Parser<'source> {
    source: &'source str,
    tokens: Vec<Token>,
    position: usize,
    diagnostics: Vec<ParseDiagnostic>,
}

impl Parser<'_> {
    fn parse_source_file(&mut self) -> SyntaxNode {
        let mut children = Vec::new();

        while !self.at(TokenKind::Eof) {
            if self.current().kind.is_trivia() {
                children.push(self.bump_token());
            } else if self.at(TokenKind::Hash) {
                children.push(self.parse_preprocessor_directive());
            } else {
                children.push(self.parse_declaration_or_error(false));
            }
        }

        children.push(self.bump_token());
        SyntaxNode::new(SyntaxKind::SourceFile, children)
    }

    fn parse_declaration_or_error(&mut self, in_class: bool) -> SyntaxElement {
        let mut prefix = Vec::new();
        self.collect_trivia(&mut prefix);
        self.collect_attributes(&mut prefix);
        self.collect_trivia(&mut prefix);
        self.collect_modifier_list(&mut prefix);
        self.collect_trivia(&mut prefix);

        let kind = self.current().kind;
        if self.at_keyword(Keyword::Class) {
            return self.parse_class_decl(prefix);
        }
        if self.at_keyword(Keyword::Enum) {
            return self.parse_enum_decl(prefix);
        }
        if self.at_keyword(Keyword::Typedef) {
            return self.parse_typedef_decl(prefix);
        }
        if self.at(TokenKind::Hash) {
            prefix.push(self.parse_preprocessor_directive());
            return node(SyntaxKind::PreprocessorDirective, prefix);
        }
        if self.at(TokenKind::Semicolon) {
            prefix.push(self.bump_token());
            return node(SyntaxKind::EmptyDecl, prefix);
        }
        if self.at(TokenKind::RightBrace) {
            prefix.push(self.bump_token());
            return node(SyntaxKind::Error, prefix);
        }

        if self.looks_like_callable_decl() {
            self.parse_callable_decl(prefix, in_class)
        } else if is_declaration_start(kind) {
            self.parse_field_decl(prefix)
        } else {
            self.parse_error_until_sync(prefix)
        }
    }

    fn parse_class_decl(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children.push(self.bump_token());
        self.collect_trivia(&mut children);

        if self.is_name_token() {
            children.push(self.bump_token());
        } else {
            self.error_here("Expected class name");
        }

        self.collect_trivia(&mut children);
        if self.at_operator(Operator::Less) {
            children.push(self.parse_angle_list(SyntaxKind::GenericArgList));
        }

        self.collect_trivia(&mut children);
        if self.at(TokenKind::Colon) || self.at_keyword(Keyword::Extends) {
            children.push(self.bump_token());
            self.collect_trivia(&mut children);
            children.push(self.parse_type_ref_until(&[TokenKind::LeftBrace, TokenKind::Semicolon]));
        }

        self.collect_trivia(&mut children);
        if self.at(TokenKind::LeftBrace) {
            children.push(self.parse_class_body());
        } else {
            self.error_here("Expected class body");
        }

        self.collect_trivia(&mut children);
        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        }

        node(SyntaxKind::ClassDecl, children)
    }

    fn parse_enum_decl(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children.push(self.bump_token());
        self.collect_trivia(&mut children);
        if self.is_name_token() {
            children.push(self.bump_token());
        }

        self.collect_trivia(&mut children);
        if self.at(TokenKind::Colon) {
            children.push(self.bump_token());
            self.collect_trivia(&mut children);
            children.push(self.parse_type_ref_until(&[TokenKind::LeftBrace, TokenKind::Semicolon]));
        }

        self.collect_trivia(&mut children);
        if self.at(TokenKind::LeftBrace) {
            children.push(self.bump_token());
            while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
                if self.current().kind.is_trivia() {
                    children.push(self.bump_token());
                } else if self.is_name_token() {
                    children.push(self.parse_enum_member());
                } else {
                    children.push(self.bump_token());
                }
            }
            self.expect(
                TokenKind::RightBrace,
                &mut children,
                "Expected enum closing brace",
            );
        } else {
            self.error_here("Expected enum body");
        }

        self.collect_trivia(&mut children);
        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        }

        node(SyntaxKind::EnumDecl, children)
    }

    fn parse_enum_member(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());
        while !matches!(
            self.current().kind,
            TokenKind::Comma | TokenKind::RightBrace | TokenKind::Eof
        ) {
            children.push(self.bump_token());
        }
        if self.at(TokenKind::Comma) {
            children.push(self.bump_token());
        }
        node(SyntaxKind::EnumMember, children)
    }

    fn parse_typedef_decl(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children.push(self.bump_token());
        while !matches!(
            self.current().kind,
            TokenKind::Semicolon | TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Eof
        ) {
            if self.current().kind.is_trivia() {
                children.push(self.bump_token());
            } else {
                children.push(self.parse_type_ref_until(&[
                    TokenKind::Semicolon,
                    TokenKind::LeftBrace,
                    TokenKind::RightBrace,
                ]));
                break;
            }
        }
        self.expect(
            TokenKind::Semicolon,
            &mut children,
            "Expected typedef semicolon",
        );
        node(SyntaxKind::TypedefDecl, children)
    }

    fn parse_callable_decl(
        &mut self,
        mut children: Vec<SyntaxElement>,
        in_class: bool,
    ) -> SyntaxElement {
        while !matches!(
            self.current().kind,
            TokenKind::LeftParen | TokenKind::LeftBrace | TokenKind::Semicolon | TokenKind::Eof
        ) {
            children.push(self.bump_token());
        }

        if self.at(TokenKind::LeftParen) {
            children.push(self.parse_parameter_list());
        }

        self.collect_trivia(&mut children);
        if self.at(TokenKind::LeftBrace) {
            children.push(self.parse_balanced_block());
            self.collect_trivia(&mut children);
            if self.at(TokenKind::Semicolon) {
                children.push(self.bump_token());
            }
        } else {
            self.expect(
                TokenKind::Semicolon,
                &mut children,
                "Expected callable semicolon or body",
            );
        }

        if in_class {
            node(SyntaxKind::MethodDecl, children)
        } else {
            node(SyntaxKind::FunctionDecl, children)
        }
    }

    fn parse_field_decl(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        let mut has_assignment = false;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;
        let mut brace_depth = 0usize;

        while !self.at(TokenKind::Eof) {
            let kind = self.current().kind;
            let at_top_level =
                paren_depth == 0 && bracket_depth == 0 && angle_depth == 0 && brace_depth == 0;

            if at_top_level && matches!(kind, TokenKind::Semicolon | TokenKind::RightBrace) {
                break;
            }

            if at_top_level && has_assignment && self.at(TokenKind::LeftBrace) {
                children.push(self.parse_balanced_initializer_list());
                continue;
            }

            match kind {
                TokenKind::LeftParen => paren_depth += 1,
                TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LeftBracket => bracket_depth += 1,
                TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LeftBrace => brace_depth += 1,
                TokenKind::RightBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::Operator(Operator::Less) => angle_depth += 1,
                TokenKind::Operator(Operator::Greater) => {
                    angle_depth = angle_depth.saturating_sub(1)
                }
                TokenKind::Operator(Operator::GreaterGreater) => {
                    angle_depth = angle_depth.saturating_sub(2)
                }
                TokenKind::Operator(Operator::Equal) => has_assignment = true,
                _ => {}
            }

            children.push(self.bump_token());
        }

        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        } else if !self.at(TokenKind::RightBrace) {
            self.error_here("Expected field semicolon");
        }
        node(SyntaxKind::FieldDecl, children)
    }

    fn parse_class_body(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());

        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            if self.current().kind.is_trivia() {
                children.push(self.bump_token());
            } else if self.at(TokenKind::Hash) {
                children.push(self.parse_preprocessor_directive());
            } else {
                children.push(self.parse_declaration_or_error(true));
            }
        }

        self.expect(
            TokenKind::RightBrace,
            &mut children,
            "Expected class closing brace",
        );
        node(SyntaxKind::Block, children)
    }

    fn parse_balanced_block(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        let mut depth = 0usize;

        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::LeftBrace) {
                depth += 1;
                children.push(self.bump_token());
                continue;
            }

            if self.at(TokenKind::RightBrace) {
                children.push(self.bump_token());
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return node(SyntaxKind::Block, children);
                }
                continue;
            }

            children.push(self.bump_token());
        }

        self.error_at_span("Expected block closing brace", children_span(&children));
        node(SyntaxKind::Block, children)
    }

    fn parse_balanced_initializer_list(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        let mut depth = 0usize;

        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::LeftBrace) {
                depth += 1;
                children.push(self.bump_token());
                continue;
            }

            if self.at(TokenKind::RightBrace) {
                children.push(self.bump_token());
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return node(SyntaxKind::InitializerList, children);
                }
                continue;
            }

            children.push(self.bump_token());
        }

        self.error_at_span(
            "Expected initializer-list closing brace",
            children_span(&children),
        );
        node(SyntaxKind::InitializerList, children)
    }

    fn parse_parameter_list(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());

        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            if self.current().kind.is_trivia() || self.at(TokenKind::Comma) {
                children.push(self.bump_token());
            } else {
                children.push(self.parse_parameter());
            }
        }

        self.expect(
            TokenKind::RightParen,
            &mut children,
            "Expected parameter-list closing paren",
        );
        node(SyntaxKind::ParameterList, children)
    }

    fn parse_parameter(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;
        let mut brace_depth = 0usize;

        while !self.at(TokenKind::Eof) {
            let kind = self.current().kind;
            if paren_depth == 0
                && bracket_depth == 0
                && angle_depth == 0
                && brace_depth == 0
                && matches!(kind, TokenKind::Comma | TokenKind::RightParen)
            {
                break;
            }

            match kind {
                TokenKind::LeftParen => paren_depth += 1,
                TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LeftBracket => bracket_depth += 1,
                TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LeftBrace => brace_depth += 1,
                TokenKind::RightBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::Operator(Operator::Less) => angle_depth += 1,
                TokenKind::Operator(Operator::Greater) => {
                    angle_depth = angle_depth.saturating_sub(1)
                }
                TokenKind::Operator(Operator::GreaterGreater) => {
                    angle_depth = angle_depth.saturating_sub(2)
                }
                _ => {}
            }

            children.push(self.bump_token());
        }
        node(SyntaxKind::Parameter, children)
    }

    fn parse_angle_list(&mut self, kind: SyntaxKind) -> SyntaxElement {
        let mut children = Vec::new();
        let mut depth = 0usize;

        while !self.at(TokenKind::Eof) {
            if self.at_operator(Operator::Less) {
                depth += 1;
                children.push(self.bump_token());
                continue;
            }

            if self.at_operator(Operator::Greater) {
                children.push(self.bump_token());
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return node(kind, children);
                }
                continue;
            }

            if self.at_operator(Operator::GreaterGreater) {
                children.push(self.bump_token());
                depth = depth.saturating_sub(2);
                if depth == 0 {
                    return node(kind, children);
                }
                continue;
            }

            if depth == 0 {
                break;
            }

            children.push(self.bump_token());
        }

        self.error_at_span("Expected generic closing angle", children_span(&children));
        node(kind, children)
    }

    fn parse_type_ref_until(&mut self, stop: &[TokenKind]) -> SyntaxElement {
        let mut children = Vec::new();
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;

        while !self.at(TokenKind::Eof) {
            let kind = self.current().kind;
            if paren_depth == 0 && bracket_depth == 0 && angle_depth == 0 && stop.contains(&kind) {
                break;
            }

            match kind {
                TokenKind::LeftParen => paren_depth += 1,
                TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LeftBracket => bracket_depth += 1,
                TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::Operator(Operator::Less) => angle_depth += 1,
                TokenKind::Operator(Operator::Greater) => {
                    angle_depth = angle_depth.saturating_sub(1)
                }
                TokenKind::Operator(Operator::GreaterGreater) => {
                    angle_depth = angle_depth.saturating_sub(2)
                }
                _ => {}
            }

            children.push(self.bump_token());
        }

        node(SyntaxKind::TypeRef, children)
    }

    fn parse_preprocessor_directive(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());

        while !self.at(TokenKind::Eof) {
            let token = self.current();
            children.push(self.bump_token());
            if self.token_text(token).contains('\n') {
                break;
            }
        }

        node(SyntaxKind::PreprocessorDirective, children)
    }

    fn parse_error_until_sync(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        self.error_here("Unexpected token in declaration context");
        while !matches!(
            self.current().kind,
            TokenKind::Semicolon | TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Eof
        ) {
            children.push(self.bump_token());
        }
        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        }
        node(SyntaxKind::Error, children)
    }

    fn collect_attributes(&mut self, children: &mut Vec<SyntaxElement>) {
        loop {
            self.collect_trivia(children);
            if !self.at(TokenKind::LeftBracket) {
                break;
            }
            children.push(self.parse_attribute_list());
            self.collect_trivia(children);
            if self.at(TokenKind::Semicolon) {
                children.push(self.bump_token());
            }
        }
    }

    fn parse_attribute_list(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());

        while !self.at(TokenKind::RightBracket) && !self.at(TokenKind::Eof) {
            if self.is_name_token() {
                children.push(self.parse_attribute());
            } else {
                children.push(self.bump_token());
            }
        }

        self.expect(
            TokenKind::RightBracket,
            &mut children,
            "Expected attribute-list closing bracket",
        );
        node(SyntaxKind::AttributeList, children)
    }

    fn parse_attribute(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());
        while !matches!(
            self.current().kind,
            TokenKind::Comma | TokenKind::RightBracket | TokenKind::Eof
        ) {
            if self.at(TokenKind::LeftParen) {
                children.push(self.parse_balanced_parens());
            } else {
                children.push(self.bump_token());
            }
        }
        if self.at(TokenKind::Comma) {
            children.push(self.bump_token());
        }
        node(SyntaxKind::Attribute, children)
    }

    fn parse_balanced_parens(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        let mut depth = 0usize;

        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::LeftParen) {
                depth += 1;
                children.push(self.bump_token());
                continue;
            }
            if self.at(TokenKind::RightParen) {
                children.push(self.bump_token());
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
                continue;
            }
            children.push(self.bump_token());
        }

        node(SyntaxKind::AttributeArgs, children)
    }

    fn collect_modifier_list(&mut self, children: &mut Vec<SyntaxElement>) {
        if !self.at_modifier() {
            return;
        }

        let mut modifiers = Vec::new();
        while self.at_modifier() || self.current().kind.is_trivia() {
            if self.current().kind.is_trivia() {
                modifiers.push(self.bump_token());
            } else {
                modifiers.push(self.bump_token());
            }
        }
        children.push(node(SyntaxKind::ModifierList, modifiers));
    }

    fn collect_trivia(&mut self, children: &mut Vec<SyntaxElement>) {
        while self.current().kind.is_trivia() {
            children.push(self.bump_token());
        }
    }

    fn looks_like_callable_decl(&self) -> bool {
        let mut index = self.position;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                TokenKind::LeftParen => return true,
                TokenKind::Operator(Operator::Equal) => return false,
                TokenKind::Semicolon
                | TokenKind::LeftBrace
                | TokenKind::RightBrace
                | TokenKind::Eof => return false,
                _ => index += 1,
            }
        }
        false
    }

    fn at_modifier(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Keyword(Keyword::Modded)
                | TokenKind::Keyword(Keyword::Sealed)
                | TokenKind::Keyword(Keyword::Proto)
                | TokenKind::Keyword(Keyword::External)
                | TokenKind::Keyword(Keyword::Native)
                | TokenKind::Keyword(Keyword::Volatile)
                | TokenKind::Keyword(Keyword::Private)
                | TokenKind::Keyword(Keyword::Protected)
                | TokenKind::Keyword(Keyword::Static)
                | TokenKind::Keyword(Keyword::Override)
                | TokenKind::Keyword(Keyword::Const)
                | TokenKind::Keyword(Keyword::Owned)
                | TokenKind::Keyword(Keyword::Event)
        )
    }

    fn is_name_token(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Identifier | TokenKind::Keyword(_)
        )
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        self.current().kind == TokenKind::Keyword(keyword)
    }

    fn at_operator(&self, operator: Operator) -> bool {
        self.current().kind == TokenKind::Operator(operator)
    }

    fn current(&self) -> Token {
        self.tokens[self.position]
    }

    fn bump_token(&mut self) -> SyntaxElement {
        let token = self.current();
        self.position += 1;
        SyntaxElement::Token(token)
    }

    fn expect(&mut self, kind: TokenKind, children: &mut Vec<SyntaxElement>, message: &str) {
        if self.at(kind) {
            children.push(self.bump_token());
        } else {
            self.error_here(message);
        }
    }

    fn error_here(&mut self, message: &str) {
        self.error_at_span(message, self.current().span);
    }

    fn error_at_span(&mut self, message: &str, span: TextSpan) {
        self.diagnostics.push(ParseDiagnostic {
            message: message.to_string(),
            span,
        });
    }

    fn token_text(&self, token: Token) -> &str {
        &self.source[token.span.start..token.span.end]
    }
}

fn node(kind: SyntaxKind, children: Vec<SyntaxElement>) -> SyntaxElement {
    SyntaxElement::Node(Box::new(SyntaxNode::new(kind, children)))
}

fn children_span(children: &[SyntaxElement]) -> TextSpan {
    let Some(first) = children.first() else {
        return TextSpan::new(0, 0);
    };
    let Some(last) = children.last() else {
        return first.span();
    };

    TextSpan::new(first.span().start, last.span().end)
}

fn is_declaration_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::Keyword(Keyword::Void)
            | TokenKind::Keyword(Keyword::Int)
            | TokenKind::Keyword(Keyword::Float)
            | TokenKind::Keyword(Keyword::Bool)
            | TokenKind::Keyword(Keyword::String)
            | TokenKind::Keyword(Keyword::Vector)
            | TokenKind::Keyword(Keyword::Typename)
            | TokenKind::Keyword(Keyword::Ref)
            | TokenKind::Keyword(Keyword::Notnull)
            | TokenKind::Keyword(Keyword::Auto)
            | TokenKind::Keyword(Keyword::Func)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex, TokenKind};
    use crate::syntax::SyntaxKind;

    fn count_kind(node: &SyntaxNode, kind: SyntaxKind) -> usize {
        let own = usize::from(node.kind == kind);
        own + node
            .children
            .iter()
            .map(|child| match child {
                SyntaxElement::Node(node) => count_kind(node, kind),
                SyntaxElement::Token(_) => 0,
            })
            .sum::<usize>()
    }

    fn non_eof_token_count(source: &str) -> usize {
        lex(source)
            .into_iter()
            .filter(|token| token.kind != TokenKind::Eof)
            .count()
    }

    #[test]
    fn parses_declaration_shapes() {
        let source = r#"[BaseContainerProps(configRoot: true)]
class SCR_Example : Managed
{
	[Attribute()]
	protected ref array<ref SCR_Item> m_aItems = {};
	proto native bool Find(TKey key, out TValue value);
}

typedef map<ref Managed, ref Managed> TManagedRefManagedRefMap;
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ClassDecl), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::AttributeList), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::Attribute), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::AttributeArgs), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::MethodDecl), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::FieldDecl), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::InitializerList), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::TypedefDecl), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ParameterList), 1);
    }

    #[test]
    fn keeps_generic_type_commas_inside_parameters() {
        let source = r#"class Example
{
	proto int Copy(map<TKey,TValue> from);
	static override bool GetEntitySourceBudgetCost(IEntityComponentSource editableEntitySource, out notnull array<ref SCR_EntityBudgetValue> budgetValues);
	void WithDefault(int value = Math.Clamp(1, 2, 3), string name = "ok");
	void WithBraceDefault(vector targetPosition[4] = { "1 0 0", "0 1 0", "0 0 1", "0 0 0" }, bool disableInput = false, SCR_LoiterCustomAnimData customAnimData = SCR_LoiterCustomAnimData.Default);
}
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::MethodDecl), 4);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ParameterList), 4);
        assert_eq!(count_kind(&parse.root, SyntaxKind::Parameter), 8);
    }

    #[test]
    fn separates_field_initializer_lists_from_blocks() {
        let source = r#"class Example
{
	protected ref array<int> m_aValues = {};
	static const ref array<string> NAMES = {"A", "B"};
	void Run()
	{
	}
}
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::FieldDecl), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::InitializerList), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::Block), 2);
    }

    #[test]
    fn keeps_nested_initializer_braces_inside_field_call_initializers() {
        let source = r#"class Example
{
	protected static ref TStringArray s_aVarsOut2 = SCR_AINodePortsHelpers.MergeTwoArrays(SCR_AIGetWaypointParameters.s_aVarsOut_Base, {PORT_ENTITY});
	protected ref array<int> m_aValues = {};
}
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::FieldDecl), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::InitializerList), 1);
    }

    #[test]
    fn accepts_game_data_optional_semicolons_after_attributes_and_bodies() {
        let source = r#"class ScriptedLoadContainer: LoadContainer
{
	event protected bool StartObject() {return false;};
	event protected bool StartArray(out int count) {return false;};
}

[BaseContainerProps()]
class SCR_DefendWaypointPreset
{
	[Attribute("", UIWidgets.EditBox, "Preset name, only informative. Switch using index.")];
	protected string m_sName;

	[Attribute("true", UIWidgets.CheckBox, "Use turrets?")];
	protected bool m_bUseTurrets;
}
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ClassDecl), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::MethodDecl), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::FieldDecl), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::AttributeList), 3);
    }

    #[test]
    fn preserves_empty_semicolon_declarations() {
        let source = r#";
class Example
{
	protected ref array<WeaponSlotComponent> m_aWeaponSlots = new array<WeaponSlotComponent>(); ;
	proto external void RequestPlayerSave(int iPlayerId);;
}
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ClassDecl), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::FieldDecl), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::MethodDecl), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::EmptyDecl), 3);
    }

    #[test]
    fn tolerates_field_before_class_close_without_semicolon() {
        let source = r#"class SerializerDefaultSpawnData: Managed
{
	vector Transform[4];
	ResourceName Prefab
}
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ClassDecl), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::FieldDecl), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::Block), 1);
    }

    #[test]
    fn preserves_all_tokens_in_committed_parser_fixtures() {
        let fixtures = [
            include_str!("../../tools/fixtures/parser/core_types_declarations.c"),
            include_str!("../../tools/fixtures/parser/attributes_rpc_workbench.c"),
            include_str!("../../tools/fixtures/parser/modded_game_mode_members.c"),
            include_str!("../../tools/fixtures/parser/preprocessor_directives.c"),
            include_str!("../../tools/fixtures/parser/game_building_network_component.c"),
            include_str!("../../tools/fixtures/parser/game_building_provider_excerpt.c"),
            include_str!("../../tools/fixtures/parser/game_editable_group_excerpt.c"),
            include_str!("../../tools/fixtures/parser/game_editor_preview_params.c"),
            include_str!("../../tools/fixtures/parser/workbench_basic_code_formatter_excerpt.c"),
            include_str!("../../tools/fixtures/parser/game_optional_semicolon_shapes.c"),
            include_str!("../../tools/fixtures/parser/game_field_initializer_call_shapes.c"),
        ];

        for fixture in fixtures {
            let parse = parse_source(fixture);
            assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
            assert_eq!(parse.root.token_count(), non_eof_token_count(fixture) + 1);
        }
    }

    #[test]
    fn recovers_with_diagnostics_for_malformed_source() {
        let parse = parse_source("class MissingBody\n{\nvoid Bad(int value\n");

        assert!(!parse.diagnostics.is_empty());
        assert_eq!(parse.root.kind, SyntaxKind::SourceFile);
    }
}
