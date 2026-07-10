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
            children.push(self.parse_statement_block());
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
        let mut hit_preprocessor_boundary = false;
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

            if at_top_level && self.at(TokenKind::Hash) {
                hit_preprocessor_boundary = true;
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
        } else if !self.at(TokenKind::RightBrace) && !hit_preprocessor_boundary {
            self.error_here("Expected field semicolon");
        }
        if hit_preprocessor_boundary {
            node(SyntaxKind::Error, children)
        } else {
            node(SyntaxKind::FieldDecl, children)
        }
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

    fn parse_statement_block(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());

        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            children.push(self.parse_statement());
        }

        self.expect(
            TokenKind::RightBrace,
            &mut children,
            "Expected block closing brace",
        );
        node(SyntaxKind::Block, children)
    }

    fn parse_statement(&mut self) -> SyntaxElement {
        let mut prefix = Vec::new();
        self.collect_trivia(&mut prefix);

        if self.at(TokenKind::RightBrace) || self.at(TokenKind::Eof) {
            return node(SyntaxKind::EmptyStatement, prefix);
        }
        if self.at(TokenKind::Hash) {
            prefix.push(self.parse_preprocessor_directive());
            return node(SyntaxKind::PreprocessorDirective, prefix);
        }
        if self.at(TokenKind::LeftBrace) {
            if prefix.is_empty() {
                return self.parse_statement_block();
            }
            prefix.push(self.parse_statement_block());
            return node(SyntaxKind::Block, prefix);
        }
        if self.at(TokenKind::Semicolon) {
            prefix.push(self.bump_token());
            return node(SyntaxKind::EmptyStatement, prefix);
        }

        match self.current().kind {
            TokenKind::Keyword(Keyword::If) => self.parse_if_statement(prefix),
            TokenKind::Keyword(Keyword::For) => self.parse_for_statement(prefix),
            TokenKind::Keyword(Keyword::Foreach) => self.parse_foreach_statement(prefix),
            TokenKind::Keyword(Keyword::While) => self.parse_while_statement(prefix),
            TokenKind::Keyword(Keyword::Do) => self.parse_do_while_statement(prefix),
            TokenKind::Keyword(Keyword::Switch) => self.parse_switch_statement(prefix),
            TokenKind::Keyword(Keyword::Return) => self.parse_return_statement(prefix),
            TokenKind::Keyword(Keyword::Break) => {
                self.parse_flow_statement(prefix, SyntaxKind::BreakStatement)
            }
            TokenKind::Keyword(Keyword::Continue) => {
                self.parse_flow_statement(prefix, SyntaxKind::ContinueStatement)
            }
            TokenKind::Keyword(Keyword::Delete) => {
                self.parse_prefixed_expression_statement(prefix, SyntaxKind::DeleteStatement)
            }
            TokenKind::Keyword(Keyword::Thread) => {
                self.parse_prefixed_expression_statement(prefix, SyntaxKind::ThreadStatement)
            }
            _ if self.looks_like_local_decl_statement() => self.parse_local_decl_statement(prefix),
            _ => self.parse_expression_statement(prefix),
        }
    }

    fn parse_if_statement(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children.push(self.bump_token());
        self.collect_trivia(&mut children);
        if self.at(TokenKind::LeftParen) {
            children.push(self.parse_parenthesized_expression_node(SyntaxKind::Condition));
        } else {
            self.error_here("Expected if condition");
        }
        if !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            children.push(self.parse_statement());
        }
        self.collect_trivia(&mut children);
        if self.at_keyword(Keyword::Else) {
            let mut else_children = Vec::new();
            else_children.push(self.bump_token());
            if !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
                else_children.push(self.parse_statement());
            }
            children.push(node(SyntaxKind::ElseClause, else_children));
        }
        node(SyntaxKind::IfStatement, children)
    }

    fn parse_for_statement(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children.push(self.bump_token());
        self.collect_trivia(&mut children);
        if self.at(TokenKind::LeftParen) {
            children.push(self.parse_for_header());
        } else {
            self.error_here("Expected for header");
        }
        if !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            children.push(self.parse_statement());
        }
        node(SyntaxKind::ForStatement, children)
    }

    fn parse_foreach_statement(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children.push(self.bump_token());
        self.collect_trivia(&mut children);
        if self.at(TokenKind::LeftParen) {
            children.push(self.parse_foreach_header());
        } else {
            self.error_here("Expected foreach header");
        }
        if !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            children.push(self.parse_statement());
        }
        node(SyntaxKind::ForeachStatement, children)
    }

    fn parse_while_statement(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children.push(self.bump_token());
        self.collect_trivia(&mut children);
        if self.at(TokenKind::LeftParen) {
            children.push(self.parse_parenthesized_expression_node(SyntaxKind::Condition));
        } else {
            self.error_here("Expected while condition");
        }
        if !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            children.push(self.parse_statement());
        }
        node(SyntaxKind::WhileStatement, children)
    }

    fn parse_do_while_statement(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children.push(self.bump_token());
        if !self.at(TokenKind::Eof) {
            children.push(self.parse_statement());
        }
        self.collect_trivia(&mut children);
        if self.at_keyword(Keyword::While) {
            children.push(self.bump_token());
            self.collect_trivia(&mut children);
            if self.at(TokenKind::LeftParen) {
                children.push(self.parse_parenthesized_expression_node(SyntaxKind::Condition));
            } else {
                self.error_here("Expected do-while condition");
            }
            self.collect_trivia(&mut children);
            if self.at(TokenKind::Semicolon) {
                children.push(self.bump_token());
            }
        } else {
            self.error_here("Expected while after do body");
        }
        node(SyntaxKind::DoWhileStatement, children)
    }

    fn parse_switch_statement(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children.push(self.bump_token());
        self.collect_trivia(&mut children);
        if self.at(TokenKind::LeftParen) {
            children.push(self.parse_parenthesized_expression_node(SyntaxKind::SwitchHeader));
        } else {
            self.error_here("Expected switch header");
        }
        self.collect_trivia(&mut children);
        if self.at(TokenKind::LeftBrace) {
            children.push(self.bump_token());
            while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
                if self.current().kind.is_trivia() {
                    children.push(self.bump_token());
                } else if self.at_keyword(Keyword::Case) {
                    children.push(self.parse_case_clause());
                } else if self.at_keyword(Keyword::Default) {
                    children.push(self.parse_default_clause());
                } else {
                    children.push(self.parse_statement());
                }
            }
            self.expect(
                TokenKind::RightBrace,
                &mut children,
                "Expected switch closing brace",
            );
        } else {
            self.error_here("Expected switch body");
        }
        node(SyntaxKind::SwitchStatement, children)
    }

    fn parse_case_clause(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());
        self.collect_trivia(&mut children);
        if !self.at(TokenKind::Colon) {
            children.push(self.parse_expression_until(&[TokenKind::Colon], 0));
            self.collect_trivia(&mut children);
        }
        self.expect(TokenKind::Colon, &mut children, "Expected case colon");
        node(SyntaxKind::CaseClause, children)
    }

    fn parse_default_clause(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());
        self.collect_trivia(&mut children);
        self.expect(TokenKind::Colon, &mut children, "Expected default colon");
        node(SyntaxKind::DefaultClause, children)
    }

    fn parse_return_statement(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children.push(self.bump_token());
        if !self.next_non_trivia_is_any(&[TokenKind::Semicolon, TokenKind::RightBrace]) {
            children.push(
                self.parse_expression_until(&[TokenKind::Semicolon, TokenKind::RightBrace], 0),
            );
        }
        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        }
        node(SyntaxKind::ReturnStatement, children)
    }

    fn parse_flow_statement(
        &mut self,
        mut children: Vec<SyntaxElement>,
        kind: SyntaxKind,
    ) -> SyntaxElement {
        children.push(self.bump_token());
        self.collect_trivia(&mut children);
        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        }
        node(kind, children)
    }

    fn parse_prefixed_expression_statement(
        &mut self,
        mut children: Vec<SyntaxElement>,
        kind: SyntaxKind,
    ) -> SyntaxElement {
        children.push(self.bump_token());
        if !self.next_non_trivia_is_any(&[TokenKind::Semicolon, TokenKind::RightBrace]) {
            children.push(
                self.parse_expression_until(&[TokenKind::Semicolon, TokenKind::RightBrace], 0),
            );
        }
        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        }
        node(kind, children)
    }

    fn parse_expression_statement(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        children
            .push(self.parse_expression_until(&[TokenKind::Semicolon, TokenKind::RightBrace], 0));
        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        }
        node(SyntaxKind::ExpressionStatement, children)
    }

    fn parse_local_decl_statement(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxElement {
        while !self.at(TokenKind::Eof)
            && !matches!(
                self.current().kind,
                TokenKind::Semicolon | TokenKind::RightBrace
            )
        {
            if self.current().kind.is_trivia() || self.at(TokenKind::Comma) {
                children.push(self.bump_token());
            } else if self.at(TokenKind::LeftBrace) {
                children.push(self.parse_initializer_expression());
            } else if self.at_operator(Operator::Equal) {
                children.push(self.bump_token());
                children.push(self.parse_expression_until(
                    &[
                        TokenKind::Comma,
                        TokenKind::Semicolon,
                        TokenKind::RightBrace,
                    ],
                    0,
                ));
            } else {
                children.push(self.bump_token());
            }
        }
        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        }
        node(SyntaxKind::LocalDeclStatement, children)
    }

    fn parse_for_header(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());

        if !self.next_non_trivia_is_any(&[TokenKind::Semicolon, TokenKind::RightParen]) {
            children.push(self.parse_for_initializer());
        }
        self.collect_trivia(&mut children);
        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        } else if !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            self.error_here("Expected for initializer semicolon");
        }

        if !self.next_non_trivia_is_any(&[TokenKind::Semicolon, TokenKind::RightParen]) {
            children.push(self.parse_for_condition());
        }
        self.collect_trivia(&mut children);
        if self.at(TokenKind::Semicolon) {
            children.push(self.bump_token());
        } else if !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            self.error_here("Expected for condition semicolon");
        }

        if !self.next_non_trivia_is_any(&[TokenKind::RightParen]) {
            children.push(self.parse_for_increment());
        }
        self.collect_trivia(&mut children);
        self.expect(
            TokenKind::RightParen,
            &mut children,
            "Expected for header closing paren",
        );
        node(SyntaxKind::ForHeader, children)
    }

    fn parse_for_initializer(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        if self.looks_like_local_decl_statement_in_for_header() {
            while !self.at(TokenKind::Eof) && !self.at(TokenKind::Semicolon) {
                if self.current().kind.is_trivia() || self.at(TokenKind::Comma) {
                    children.push(self.bump_token());
                } else if self.at(TokenKind::LeftBrace) {
                    children.push(self.parse_initializer_expression());
                } else if self.at_operator(Operator::Equal) {
                    children.push(self.bump_token());
                    children.push(
                        self.parse_expression_until(&[TokenKind::Comma, TokenKind::Semicolon], 0),
                    );
                } else {
                    children.push(self.bump_token());
                }
            }
        } else {
            self.parse_for_expression_list(&mut children, &[TokenKind::Semicolon]);
        }
        node(SyntaxKind::ForInitializer, children)
    }

    fn parse_for_condition(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children
            .push(self.parse_expression_until(&[TokenKind::Semicolon, TokenKind::RightParen], 0));
        node(SyntaxKind::ForCondition, children)
    }

    fn parse_for_increment(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        self.parse_for_expression_list(&mut children, &[TokenKind::RightParen]);
        node(SyntaxKind::ForIncrement, children)
    }

    fn parse_for_expression_list(&mut self, children: &mut Vec<SyntaxElement>, stop: &[TokenKind]) {
        while !self.at(TokenKind::Eof) && !stop.contains(&self.current().kind) {
            if self.current().kind.is_trivia() || self.at(TokenKind::Comma) {
                children.push(self.bump_token());
            } else {
                let mut expression_stops = vec![TokenKind::Comma];
                expression_stops.extend_from_slice(stop);
                children.push(self.parse_expression_until(&expression_stops, 0));
            }
        }
    }

    fn parse_foreach_header(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            if self.current().kind.is_trivia()
                || self.at(TokenKind::Comma)
                || self.at(TokenKind::Colon)
            {
                children.push(self.bump_token());
            } else {
                children.push(self.parse_expression_until(
                    &[TokenKind::Comma, TokenKind::Colon, TokenKind::RightParen],
                    0,
                ));
            }
        }
        self.collect_trivia(&mut children);
        self.expect(
            TokenKind::RightParen,
            &mut children,
            "Expected foreach header closing paren",
        );
        node(SyntaxKind::ForeachHeader, children)
    }

    fn parse_parenthesized_expression_node(&mut self, kind: SyntaxKind) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());
        if !self.next_non_trivia_is_any(&[TokenKind::RightParen]) {
            children.push(self.parse_expression_until(&[TokenKind::RightParen], 0));
        }
        self.collect_trivia(&mut children);
        self.expect(
            TokenKind::RightParen,
            &mut children,
            "Expected parenthesized expression closing paren",
        );
        node(kind, children)
    }

    fn parse_expression_until(&mut self, stop: &[TokenKind], min_bp: u8) -> SyntaxElement {
        self.parse_expression_bp(stop, min_bp)
    }

    fn parse_expression_bp(&mut self, stop: &[TokenKind], min_bp: u8) -> SyntaxElement {
        let mut lhs = self.parse_prefix_expression(stop);

        loop {
            if self.at(TokenKind::Eof) || self.next_non_trivia_is_any(stop) {
                break;
            }

            if self.next_non_trivia_is(TokenKind::LeftParen) {
                let mut children = vec![lhs];
                self.collect_trivia(&mut children);
                children.push(self.parse_argument_list());
                lhs = node(SyntaxKind::CallExpression, children);
                continue;
            }

            if self.next_non_trivia_is(TokenKind::Dot) {
                let mut children = vec![lhs];
                self.collect_trivia(&mut children);
                children.push(self.bump_token());
                self.collect_trivia(&mut children);
                if self.is_name_token() {
                    children.push(self.parse_name_expression());
                } else {
                    self.error_here("Expected member name");
                }
                lhs = node(SyntaxKind::MemberAccessExpression, children);
                continue;
            }

            if self.next_non_trivia_is(TokenKind::LeftBracket) {
                let mut children = vec![lhs];
                self.collect_trivia(&mut children);
                children.push(self.bump_token());
                if !self.next_non_trivia_is(TokenKind::RightBracket) {
                    children.push(self.parse_expression_bp(&[TokenKind::RightBracket], 0));
                }
                self.collect_trivia(&mut children);
                self.expect(
                    TokenKind::RightBracket,
                    &mut children,
                    "Expected index expression closing bracket",
                );
                lhs = node(SyntaxKind::IndexExpression, children);
                continue;
            }

            if self.next_non_trivia_is_operator(Operator::PlusPlus)
                || self.next_non_trivia_is_operator(Operator::MinusMinus)
            {
                let mut children = vec![lhs];
                self.collect_trivia(&mut children);
                children.push(self.bump_token());
                lhs = node(SyntaxKind::PostfixExpression, children);
                continue;
            }

            if self.next_non_trivia_is(TokenKind::Question) {
                if min_bp > 1 {
                    break;
                }
                let mut children = vec![lhs];
                self.collect_trivia(&mut children);
                children.push(self.bump_token());
                children.push(self.parse_expression_bp(&[TokenKind::Colon], 0));
                self.collect_trivia(&mut children);
                self.expect(TokenKind::Colon, &mut children, "Expected ternary colon");
                children.push(self.parse_expression_bp(stop, 1));
                lhs = node(SyntaxKind::TernaryExpression, children);
                continue;
            }

            let Some((left_bp, right_bp, kind)) = self.current_binary_binding_power() else {
                break;
            };
            if left_bp < min_bp {
                break;
            }

            let mut children = vec![lhs];
            self.collect_trivia(&mut children);
            children.push(self.bump_token());
            children.push(self.parse_expression_bp(stop, right_bp));
            lhs = node(kind, children);
        }

        lhs
    }

    fn parse_prefix_expression(&mut self, stop: &[TokenKind]) -> SyntaxElement {
        let mut children = Vec::new();
        self.collect_trivia(&mut children);

        if self.at(TokenKind::Eof) || self.next_non_trivia_is_any(stop) {
            self.error_here("Expected expression");
            return node(SyntaxKind::Error, children);
        }

        if self.at(TokenKind::LeftBrace) {
            children.push(self.parse_initializer_expression());
            return single_or_wrapped_expression(children);
        }

        if self.at(TokenKind::LeftParen) {
            children.push(self.bump_token());
            if !self.next_non_trivia_is(TokenKind::RightParen) {
                children.push(self.parse_expression_bp(&[TokenKind::RightParen], 0));
            }
            self.collect_trivia(&mut children);
            self.expect(
                TokenKind::RightParen,
                &mut children,
                "Expected parenthesized expression closing paren",
            );
            if self.next_token_can_start_expression() {
                children.push(self.parse_expression_bp(stop, 14));
                return node(SyntaxKind::CastExpression, children);
            }
            return node(SyntaxKind::ParenthesizedExpression, children);
        }

        if self.at_keyword(Keyword::New) {
            children.push(self.bump_token());
            self.collect_trivia(&mut children);
            if self.is_name_token() {
                children.push(self.parse_type_name_expression());
            }
            if self.next_non_trivia_is(TokenKind::LeftParen) {
                self.collect_trivia(&mut children);
                children.push(self.parse_argument_list());
            }
            return node(SyntaxKind::NewExpression, children);
        }

        if self.at_prefix_operator() {
            children.push(self.bump_token());
            children.push(self.parse_expression_bp(stop, 14));
            return node(SyntaxKind::UnaryExpression, children);
        }

        if self.is_name_token() {
            children.push(self.parse_name_expression());
            return single_or_wrapped_expression(children);
        }

        if matches!(self.current().kind, TokenKind::Number | TokenKind::String) {
            children.push(self.bump_token());
            return node(SyntaxKind::LiteralExpression, children);
        }

        children.push(self.bump_token());
        node(SyntaxKind::Expression, children)
    }

    fn parse_name_expression(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());
        if self.next_non_trivia_is_operator(Operator::Less)
            && self.looks_like_generic_argument_list()
        {
            self.collect_trivia(&mut children);
            children.push(self.parse_angle_list(SyntaxKind::GenericArgList));
        }
        node(SyntaxKind::NameExpression, children)
    }

    fn parse_type_name_expression(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());
        if self.next_non_trivia_is_operator(Operator::Less) {
            self.collect_trivia(&mut children);
            children.push(self.parse_angle_list(SyntaxKind::GenericArgList));
        }
        node(SyntaxKind::NameExpression, children)
    }

    fn parse_argument_list(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());

        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            if self.current().kind.is_trivia() || self.at(TokenKind::Comma) {
                children.push(self.bump_token());
            } else {
                children.push(self.parse_argument());
            }
        }

        self.expect(
            TokenKind::RightParen,
            &mut children,
            "Expected argument-list closing paren",
        );
        node(SyntaxKind::ArgumentList, children)
    }

    fn parse_argument(&mut self) -> SyntaxElement {
        if self.is_name_token() && self.next_significant_kind(1) == Some(TokenKind::Colon) {
            let mut children = Vec::new();
            children.push(self.parse_name_expression());
            self.collect_trivia(&mut children);
            children.push(self.bump_token());
            children.push(self.parse_expression_bp(&[TokenKind::Comma, TokenKind::RightParen], 0));
            return node(SyntaxKind::NamedArgument, children);
        }

        self.parse_expression_bp(&[TokenKind::Comma, TokenKind::RightParen], 0)
    }

    fn parse_initializer_expression(&mut self) -> SyntaxElement {
        let mut children = Vec::new();
        children.push(self.bump_token());
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            if self.current().kind.is_trivia() || self.at(TokenKind::Comma) {
                children.push(self.bump_token());
            } else if self.at(TokenKind::LeftBrace) {
                children.push(self.parse_initializer_expression());
            } else {
                children
                    .push(self.parse_expression_bp(&[TokenKind::Comma, TokenKind::RightBrace], 0));
            }
        }
        self.expect(
            TokenKind::RightBrace,
            &mut children,
            "Expected initializer expression closing brace",
        );
        node(SyntaxKind::InitializerExpression, children)
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

    fn looks_like_local_decl_statement(&self) -> bool {
        if !is_declaration_start(self.current().kind) || self.at_keyword(Keyword::New) {
            return false;
        }

        let mut index = self.position;
        let mut saw_name_after_type = false;
        let mut saw_equal = false;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut angle_depth = 0usize;

        while index < self.tokens.len() {
            let kind = self.tokens[index].kind;
            let at_top_level =
                paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 && angle_depth == 0;

            if at_top_level && matches!(kind, TokenKind::Semicolon | TokenKind::RightParen) {
                return saw_name_after_type;
            }
            if at_top_level && matches!(kind, TokenKind::RightBrace | TokenKind::Eof) {
                return false;
            }
            if at_top_level && !saw_equal && matches!(kind, TokenKind::Dot | TokenKind::Question) {
                return false;
            }
            if at_top_level && kind == TokenKind::Operator(Operator::Equal) && !saw_name_after_type
            {
                return false;
            }
            if at_top_level && kind == TokenKind::Operator(Operator::Equal) {
                saw_equal = true;
            }
            if at_top_level && !saw_equal && matches!(kind, TokenKind::Colon) {
                return false;
            }

            if at_top_level
                && index > self.position
                && matches!(kind, TokenKind::Identifier | TokenKind::Keyword(_))
            {
                saw_name_after_type = true;
            }

            match kind {
                TokenKind::LeftParen => paren_depth += 1,
                TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LeftBracket => bracket_depth += 1,
                TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LeftBrace => brace_depth += 1,
                TokenKind::RightBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::Operator(Operator::Less) if !saw_equal => angle_depth += 1,
                TokenKind::Operator(Operator::Greater) if !saw_equal => {
                    angle_depth = angle_depth.saturating_sub(1)
                }
                TokenKind::Operator(Operator::GreaterGreater) if !saw_equal => {
                    angle_depth = angle_depth.saturating_sub(2)
                }
                _ => {}
            }

            index += 1;
        }

        false
    }

    fn looks_like_local_decl_statement_in_for_header(&self) -> bool {
        if !is_declaration_start(self.current().kind) || self.at_keyword(Keyword::New) {
            return false;
        }

        let mut index = self.position;
        let mut saw_name_after_type = false;
        let mut saw_equal = false;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut angle_depth = 0usize;

        while index < self.tokens.len() {
            let kind = self.tokens[index].kind;
            let at_top_level =
                paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 && angle_depth == 0;

            if at_top_level && kind == TokenKind::Semicolon {
                return saw_name_after_type;
            }
            if at_top_level
                && matches!(
                    kind,
                    TokenKind::RightParen | TokenKind::RightBrace | TokenKind::Eof
                )
            {
                return false;
            }
            if at_top_level
                && !saw_equal
                && matches!(
                    kind,
                    TokenKind::Dot | TokenKind::Question | TokenKind::Colon
                )
            {
                return false;
            }
            if at_top_level && kind == TokenKind::Operator(Operator::Equal) && !saw_name_after_type
            {
                return false;
            }
            if at_top_level && kind == TokenKind::Operator(Operator::Equal) {
                saw_equal = true;
            }
            if at_top_level
                && index > self.position
                && matches!(kind, TokenKind::Identifier | TokenKind::Keyword(_))
            {
                saw_name_after_type = true;
            }

            match kind {
                TokenKind::LeftParen => paren_depth += 1,
                TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LeftBracket => bracket_depth += 1,
                TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LeftBrace => brace_depth += 1,
                TokenKind::RightBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::Operator(Operator::Less) if !saw_equal => angle_depth += 1,
                TokenKind::Operator(Operator::Greater) if !saw_equal => {
                    angle_depth = angle_depth.saturating_sub(1)
                }
                TokenKind::Operator(Operator::GreaterGreater) if !saw_equal => {
                    angle_depth = angle_depth.saturating_sub(2)
                }
                _ => {}
            }

            index += 1;
        }

        false
    }

    fn current_binary_binding_power(&self) -> Option<(u8, u8, SyntaxKind)> {
        match self.peek_non_trivia_kind()? {
            TokenKind::Operator(Operator::Equal)
            | TokenKind::Operator(Operator::PlusEqual)
            | TokenKind::Operator(Operator::MinusEqual)
            | TokenKind::Operator(Operator::StarEqual)
            | TokenKind::Operator(Operator::SlashEqual)
            | TokenKind::Operator(Operator::PercentEqual)
            | TokenKind::Operator(Operator::AmpersandEqual)
            | TokenKind::Operator(Operator::PipeEqual)
            | TokenKind::Operator(Operator::CaretEqual)
            | TokenKind::Operator(Operator::LessLessEqual)
            | TokenKind::Operator(Operator::GreaterGreaterEqual) => {
                Some((2, 1, SyntaxKind::AssignmentExpression))
            }
            TokenKind::Operator(Operator::PipePipe) => Some((3, 4, SyntaxKind::BinaryExpression)),
            TokenKind::Operator(Operator::AmpersandAmpersand) => {
                Some((5, 6, SyntaxKind::BinaryExpression))
            }
            TokenKind::Operator(Operator::Pipe) => Some((7, 8, SyntaxKind::BinaryExpression)),
            TokenKind::Operator(Operator::Caret) => Some((9, 10, SyntaxKind::BinaryExpression)),
            TokenKind::Operator(Operator::Ampersand) => {
                Some((11, 12, SyntaxKind::BinaryExpression))
            }
            TokenKind::Operator(Operator::EqualEqual)
            | TokenKind::Operator(Operator::BangEqual) => {
                Some((13, 14, SyntaxKind::BinaryExpression))
            }
            TokenKind::Operator(Operator::Less)
            | TokenKind::Operator(Operator::LessEqual)
            | TokenKind::Operator(Operator::Greater)
            | TokenKind::Operator(Operator::GreaterEqual) => {
                Some((15, 16, SyntaxKind::BinaryExpression))
            }
            TokenKind::Operator(Operator::LessLess)
            | TokenKind::Operator(Operator::GreaterGreater) => {
                Some((17, 18, SyntaxKind::BinaryExpression))
            }
            TokenKind::Operator(Operator::Plus) | TokenKind::Operator(Operator::Minus) => {
                Some((19, 20, SyntaxKind::BinaryExpression))
            }
            TokenKind::Operator(Operator::Star)
            | TokenKind::Operator(Operator::Slash)
            | TokenKind::Operator(Operator::Percent) => {
                Some((21, 22, SyntaxKind::BinaryExpression))
            }
            _ => None,
        }
    }

    fn at_prefix_operator(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Operator(Operator::Plus)
                | TokenKind::Operator(Operator::Minus)
                | TokenKind::Operator(Operator::Bang)
                | TokenKind::Operator(Operator::Tilde)
                | TokenKind::Operator(Operator::PlusPlus)
                | TokenKind::Operator(Operator::MinusMinus)
        )
    }

    fn looks_like_generic_argument_list(&self) -> bool {
        let mut index = self.position;
        let mut depth = 0usize;

        while index < self.tokens.len() {
            match self.tokens[index].kind {
                TokenKind::Operator(Operator::Less) => depth += 1,
                TokenKind::Operator(Operator::Greater) => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return self.generic_argument_list_can_continue(index + 1);
                    }
                }
                TokenKind::Operator(Operator::GreaterGreater) => {
                    depth = depth.saturating_sub(2);
                    if depth == 0 {
                        return self.generic_argument_list_can_continue(index + 1);
                    }
                }
                TokenKind::Semicolon
                | TokenKind::LeftBrace
                | TokenKind::RightBrace
                | TokenKind::Eof => return false,
                _ => {}
            }
            index += 1;
        }

        false
    }

    fn generic_argument_list_can_continue(&self, mut index: usize) -> bool {
        while index < self.tokens.len() && self.tokens[index].kind.is_trivia() {
            index += 1;
        }
        matches!(
            self.tokens.get(index).map(|token| token.kind),
            Some(TokenKind::Dot | TokenKind::LeftParen | TokenKind::LeftBracket)
        )
    }

    fn next_non_trivia_is(&self, kind: TokenKind) -> bool {
        self.peek_non_trivia_kind() == Some(kind)
    }

    fn next_non_trivia_is_any(&self, kinds: &[TokenKind]) -> bool {
        self.peek_non_trivia_kind()
            .is_some_and(|kind| kinds.contains(&kind))
    }

    fn next_non_trivia_is_operator(&self, operator: Operator) -> bool {
        self.peek_non_trivia_kind() == Some(TokenKind::Operator(operator))
    }

    fn peek_non_trivia_kind(&self) -> Option<TokenKind> {
        let mut index = self.position;
        while index < self.tokens.len() && self.tokens[index].kind.is_trivia() {
            index += 1;
        }
        self.tokens.get(index).map(|token| token.kind)
    }

    fn next_significant_kind(&self, significant_offset: usize) -> Option<TokenKind> {
        let mut seen = 0usize;
        for token in self.tokens.iter().skip(self.position) {
            if token.kind.is_trivia() {
                continue;
            }
            if seen == significant_offset {
                return Some(token.kind);
            }
            seen += 1;
        }
        None
    }

    fn next_token_can_start_expression(&self) -> bool {
        matches!(
            self.peek_non_trivia_kind(),
            Some(
                TokenKind::Identifier
                    | TokenKind::Keyword(_)
                    | TokenKind::Number
                    | TokenKind::String
                    | TokenKind::LeftParen
                    | TokenKind::LeftBrace
                    | TokenKind::Operator(Operator::Plus)
                    | TokenKind::Operator(Operator::Minus)
                    | TokenKind::Operator(Operator::Bang)
                    | TokenKind::Operator(Operator::Tilde)
                    | TokenKind::Operator(Operator::PlusPlus)
                    | TokenKind::Operator(Operator::MinusMinus)
            )
        )
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

fn single_or_wrapped_expression(mut children: Vec<SyntaxElement>) -> SyntaxElement {
    if children.len() == 1 {
        children.remove(0)
    } else {
        node(SyntaxKind::Expression, children)
    }
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
            | TokenKind::Keyword(Keyword::Const)
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

    fn first_node(node: &SyntaxNode, kind: SyntaxKind) -> Option<&SyntaxNode> {
        if node.kind == kind {
            return Some(node);
        }

        node.children.iter().find_map(|child| match child {
            SyntaxElement::Node(node) => first_node(node, kind),
            SyntaxElement::Token(_) => None,
        })
    }

    fn direct_child_node_count(node: &SyntaxNode, kind: SyntaxKind) -> usize {
        node.children
            .iter()
            .filter(|child| matches!(child, SyntaxElement::Node(node) if node.kind == kind))
            .count()
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
    fn preprocessor_invalid_branch_text_does_not_swallow_later_declarations() {
        let source = r#"#ifdef BREAK_COMPILATION
	THIS DEFINE BREAKS GAME SCRIPT MODULE COMPILATION
	DO NOT REMOVE IT
#endif

class ArmaReforgerScripted : ChimeraGame
{
}

ArmaReforgerScripted g_ARGame;
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ClassDecl), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::FieldDecl), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::Error), 1);
    }

    #[test]
    fn parses_statement_and_expression_shapes_in_callable_bodies() {
        let source = r#"class Example
{
	void Run(array<IEntity> items, map<string, Widget> widgets, string key)
	{
		foreach (int index, IEntity item : items)
		{
			items[index].GetOrigin();
		}

		for (int i = items.Count() - 1; i >= 0; --i)
			widgets[key].SetVisible(true);

		SCR_WorkbenchHelper.PrintFormatDialog("Warning", level: LogLevel.WARNING);
		vector pos = { 1, 2, 3 };
		IEntity entity = new GenericEntity();
		set<IEntity> entities = new set<IEntity>;
		WorldEditorAPI worldEditorAPI = ((WorldEditor)Workbench.GetModule(WorldEditor)).GetApi();
		thread RunLater(entity);
		delete entity;
	}
}
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForeachStatement), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForStatement), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForHeader), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForInitializer), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForCondition), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForIncrement), 1);
        assert!(count_kind(&parse.root, SyntaxKind::CallExpression) >= 5);
        assert!(count_kind(&parse.root, SyntaxKind::MemberAccessExpression) >= 4);
        assert!(count_kind(&parse.root, SyntaxKind::IndexExpression) >= 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::NamedArgument), 1);
        assert_eq!(
            count_kind(&parse.root, SyntaxKind::InitializerExpression),
            1
        );
        assert_eq!(count_kind(&parse.root, SyntaxKind::NewExpression), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::CastExpression), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ThreadStatement), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::DeleteStatement), 1);
    }

    #[test]
    fn parses_switch_single_line_if_and_ternary_shapes() {
        let source = r#"class Example
{
	int Run(int value)
	{
		switch (value)
		{
			case 1:
			{
				if (value > 0)
					return value ? 1 : 2;
				break;
			}
			default:
				return 0;
		}
	}
}
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::SwitchStatement), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::CaseClause), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::DefaultClause), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::IfStatement), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::TernaryExpression), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::BreakStatement), 1);
    }

    #[test]
    fn parses_unbraced_if_body_as_one_following_statement() {
        let source = r#"class Example
{
	void Run(bool enabled)
	{
		if (enabled)
			DoFirst();
		DoSecond();
	}
}
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let if_node = first_node(&parse.root, SyntaxKind::IfStatement).expect("if statement");
        assert_eq!(
            direct_child_node_count(if_node, SyntaxKind::ExpressionStatement),
            1
        );
        assert_eq!(count_kind(&parse.root, SyntaxKind::ExpressionStatement), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::CallExpression), 2);
    }

    #[test]
    fn parses_inline_if_and_else_if_without_braces() {
        let source = r#"class Example
{
	bool Run(bool a, bool b)
	{
		if (a) return true;
		else if (b)
			return false;
		else
			return true;
	}
}
"#;

        let parse = parse_source(source);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::IfStatement), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ElseClause), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ReturnStatement), 3);
        let if_node = first_node(&parse.root, SyntaxKind::IfStatement).expect("if statement");
        assert_eq!(direct_child_node_count(if_node, SyntaxKind::ElseClause), 1);
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
