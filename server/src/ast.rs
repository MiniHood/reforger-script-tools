use crate::lexer::{Keyword, Operator, TextSpan, Token, TokenKind};
use crate::syntax::{Parse, SyntaxElement, SyntaxKind, SyntaxNode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextValue<'source> {
    pub span: TextSpan,
    source: &'source str,
}

impl<'source> TextValue<'source> {
    pub const fn new(source: &'source str, span: TextSpan) -> Self {
        Self { span, source }
    }

    pub fn text(self) -> &'source str {
        &self.source[self.span.start..self.span.end]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocCommentKind {
    Line,
    Block,
}

#[derive(Debug, Clone, Copy)]
pub struct DocComment<'source> {
    source: &'source str,
    token: Token,
}

impl<'source> DocComment<'source> {
    pub const fn span(&self) -> TextSpan {
        self.token.span
    }

    pub fn text(self) -> &'source str {
        &self.source[self.token.span.start..self.token.span.end]
    }

    pub fn kind(&self) -> DocCommentKind {
        match self.token.kind {
            TokenKind::DocLineComment => DocCommentKind::Line,
            TokenKind::DocBlockComment => DocCommentKind::Block,
            _ => unreachable!(),
        }
    }
}

pub struct AstSourceFile<'source, 'tree> {
    source: &'source str,
    parse: &'tree Parse,
}

impl<'source, 'tree> AstSourceFile<'source, 'tree> {
    pub const fn new(source: &'source str, parse: &'tree Parse) -> Self {
        Self { source, parse }
    }

    pub fn declarations(&self) -> Vec<Declaration<'source, 'tree>> {
        self.parse
            .root
            .children
            .iter()
            .filter_map(|child| match child {
                SyntaxElement::Node(node) => {
                    declaration_from_node(self.source, &self.parse.root, node)
                }
                SyntaxElement::Token(_) => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Declaration<'source, 'tree> {
    Class(ClassDecl<'source, 'tree>),
    Enum(EnumDecl<'source, 'tree>),
    Typedef(TypedefDecl<'source, 'tree>),
    Function(MethodDecl<'source, 'tree>),
    Field(FieldDecl<'source, 'tree>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodKind {
    Method,
    Constructor,
    Destructor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterKind {
    Declaration,
    NonDeclarationFragment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalVariableKind {
    LocalVariable,
    ForeachVariable,
    ForInitializer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionKind {
    Name,
    Literal,
    Call,
    MemberAccess,
    Index,
    Cast,
    New,
    Unary,
    Binary,
    Assignment,
    Ternary,
    Initializer,
    Parenthesized,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum Expression<'source, 'tree> {
    Name(ExpressionNode<'source, 'tree>),
    Literal(ExpressionNode<'source, 'tree>),
    Call(ExpressionNode<'source, 'tree>),
    MemberAccess(ExpressionNode<'source, 'tree>),
    Index(ExpressionNode<'source, 'tree>),
    Cast(ExpressionNode<'source, 'tree>),
    New(ExpressionNode<'source, 'tree>),
    Unary(ExpressionNode<'source, 'tree>),
    Binary(ExpressionNode<'source, 'tree>),
    Assignment(ExpressionNode<'source, 'tree>),
    Ternary(ExpressionNode<'source, 'tree>),
    Initializer(ExpressionNode<'source, 'tree>),
    Parenthesized(ExpressionNode<'source, 'tree>),
    Unknown(ExpressionNode<'source, 'tree>),
}

impl<'source, 'tree> Expression<'source, 'tree> {
    pub fn from_node(source: &'source str, node: &'tree SyntaxNode) -> Option<Self> {
        if !is_expression_syntax_kind(node.kind) {
            return None;
        }

        let view = ExpressionNode { source, node };
        Some(match node.kind {
            SyntaxKind::NameExpression => Self::Name(view),
            SyntaxKind::LiteralExpression => Self::Literal(view),
            SyntaxKind::CallExpression => Self::Call(view),
            SyntaxKind::MemberAccessExpression => Self::MemberAccess(view),
            SyntaxKind::IndexExpression => Self::Index(view),
            SyntaxKind::CastExpression => Self::Cast(view),
            SyntaxKind::NewExpression => Self::New(view),
            SyntaxKind::UnaryExpression | SyntaxKind::PostfixExpression => Self::Unary(view),
            SyntaxKind::BinaryExpression => Self::Binary(view),
            SyntaxKind::AssignmentExpression => Self::Assignment(view),
            SyntaxKind::TernaryExpression => Self::Ternary(view),
            SyntaxKind::InitializerExpression => Self::Initializer(view),
            SyntaxKind::ParenthesizedExpression => Self::Parenthesized(view),
            _ => Self::Unknown(view),
        })
    }

    pub fn kind(&self) -> ExpressionKind {
        match self {
            Self::Name(_) => ExpressionKind::Name,
            Self::Literal(_) => ExpressionKind::Literal,
            Self::Call(_) => ExpressionKind::Call,
            Self::MemberAccess(_) => ExpressionKind::MemberAccess,
            Self::Index(_) => ExpressionKind::Index,
            Self::Cast(_) => ExpressionKind::Cast,
            Self::New(_) => ExpressionKind::New,
            Self::Unary(_) => ExpressionKind::Unary,
            Self::Binary(_) => ExpressionKind::Binary,
            Self::Assignment(_) => ExpressionKind::Assignment,
            Self::Ternary(_) => ExpressionKind::Ternary,
            Self::Initializer(_) => ExpressionKind::Initializer,
            Self::Parenthesized(_) => ExpressionKind::Parenthesized,
            Self::Unknown(_) => ExpressionKind::Unknown,
        }
    }

    pub const fn node(&self) -> ExpressionNode<'source, 'tree> {
        match self {
            Self::Name(node)
            | Self::Literal(node)
            | Self::Call(node)
            | Self::MemberAccess(node)
            | Self::Index(node)
            | Self::Cast(node)
            | Self::New(node)
            | Self::Unary(node)
            | Self::Binary(node)
            | Self::Assignment(node)
            | Self::Ternary(node)
            | Self::Initializer(node)
            | Self::Parenthesized(node)
            | Self::Unknown(node) => *node,
        }
    }

    pub const fn span(&self) -> TextSpan {
        self.node().span()
    }

    pub fn selection_span(&self) -> TextSpan {
        self.node().selection_span()
    }

    pub fn source_text(&self) -> &'source str {
        self.node().source_text()
    }

    pub fn name_text(&self) -> Option<TextValue<'source>> {
        self.node().name_text()
    }

    pub fn receiver(&self) -> Option<Expression<'source, 'tree>> {
        self.node().receiver()
    }

    pub fn member_name(&self) -> Option<TextValue<'source>> {
        self.node().member_name()
    }

    pub fn callee(&self) -> Option<Expression<'source, 'tree>> {
        self.node().callee()
    }

    pub fn arguments(&self) -> Vec<Expression<'source, 'tree>> {
        self.node().arguments()
    }

    pub fn index_expression(&self) -> Option<Expression<'source, 'tree>> {
        self.node().index_expression()
    }

    pub fn return_like_type_text(&self) -> Option<TextValue<'source>> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExpressionNode<'source, 'tree> {
    source: &'source str,
    node: &'tree SyntaxNode,
}

impl<'source, 'tree> ExpressionNode<'source, 'tree> {
    pub const fn syntax_node(&self) -> &'tree SyntaxNode {
        self.node
    }

    pub const fn span(&self) -> TextSpan {
        self.node.span
    }

    pub fn selection_span(&self) -> TextSpan {
        self.name_text()
            .map(|value| value.span)
            .unwrap_or(self.node.span)
    }

    pub fn source_text(&self) -> &'source str {
        &self.source[self.node.span.start..self.node.span.end]
    }

    pub fn name_text(&self) -> Option<TextValue<'source>> {
        match self.node.kind {
            SyntaxKind::NameExpression | SyntaxKind::LiteralExpression => direct_tokens(self.node)
                .find(|token| !token.kind.is_trivia())
                .map(|token| text_value(self.source, token.span)),
            _ => None,
        }
    }

    pub fn receiver(&self) -> Option<Expression<'source, 'tree>> {
        match self.node.kind {
            SyntaxKind::MemberAccessExpression
            | SyntaxKind::IndexExpression
            | SyntaxKind::CallExpression => first_expression_child(self.source, self.node),
            _ => None,
        }
    }

    pub fn member_name(&self) -> Option<TextValue<'source>> {
        if self.node.kind != SyntaxKind::MemberAccessExpression {
            return None;
        }

        self.node
            .children
            .iter()
            .rev()
            .find_map(|child| match child {
                SyntaxElement::Node(node) if node.kind == SyntaxKind::NameExpression => {
                    Expression::from_node(self.source, node)?.name_text()
                }
                _ => None,
            })
    }

    pub fn callee(&self) -> Option<Expression<'source, 'tree>> {
        if self.node.kind != SyntaxKind::CallExpression {
            return None;
        }

        self.node.children.iter().find_map(|child| match child {
            SyntaxElement::Node(node) if node.kind == SyntaxKind::ArgumentList => None,
            SyntaxElement::Node(node) => Expression::from_node(self.source, node)
                .or_else(|| first_expression_descendant(self.source, node)),
            SyntaxElement::Token(_) => None,
        })
    }

    pub fn arguments(&self) -> Vec<Expression<'source, 'tree>> {
        let Some(argument_list) = first_child_node(self.node, SyntaxKind::ArgumentList) else {
            return Vec::new();
        };

        argument_list
            .children
            .iter()
            .filter_map(|child| match child {
                SyntaxElement::Node(node) => first_expression_in_argument(self.source, node),
                SyntaxElement::Token(_) => None,
            })
            .collect()
    }

    pub fn index_expression(&self) -> Option<Expression<'source, 'tree>> {
        if self.node.kind != SyntaxKind::IndexExpression {
            return None;
        }

        self.node
            .children
            .iter()
            .filter_map(|child| match child {
                SyntaxElement::Node(node) => Expression::from_node(self.source, node),
                SyntaxElement::Token(_) => None,
            })
            .nth(1)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemberAccessExpression<'source, 'tree> {
    pub expression: Expression<'source, 'tree>,
    pub receiver: Expression<'source, 'tree>,
    pub member_name: TextValue<'source>,
}

#[derive(Debug, Clone, Copy)]
pub struct NamedArgumentLabel<'source> {
    pub name: TextValue<'source>,
}

#[derive(Debug, Clone, Copy)]
pub struct ClassDecl<'source, 'tree> {
    source: &'source str,
    container: &'tree SyntaxNode,
    node: &'tree SyntaxNode,
}

impl<'source, 'tree> ClassDecl<'source, 'tree> {
    pub const fn span(&self) -> TextSpan {
        self.node.span
    }

    pub fn name(&self) -> Option<TextValue<'source>> {
        let mut seen_class = false;
        for token in direct_tokens(self.node).filter(|token| !token.kind.is_trivia()) {
            if seen_class && is_name_token(token.kind) {
                return Some(text_value(self.source, token.span));
            }
            if token.kind == TokenKind::Keyword(Keyword::Class) {
                seen_class = true;
            }
        }
        None
    }

    pub fn base_type(&self) -> Option<TextValue<'source>> {
        first_child_node(self.node, SyntaxKind::TypeRef)
            .and_then(|node| trimmed_node_text(self.source, node))
    }

    pub fn attributes(&self) -> Vec<Attribute<'source, 'tree>> {
        attributes(self.source, self.node)
    }

    pub fn doc_comments(&self) -> Vec<DocComment<'source>> {
        leading_doc_comments(self.source, self.container, self.node)
    }

    pub fn modifiers(&self) -> Vec<TextValue<'source>> {
        modifiers(self.source, self.node)
    }

    pub fn type_parameters(&self) -> Vec<TypeParameter<'source>> {
        first_child_node(self.node, SyntaxKind::GenericArgList)
            .map(|node| type_parameters_from_generic_arg_list(self.source, node))
            .unwrap_or_default()
    }

    pub fn members(&self) -> Vec<ClassMember<'source, 'tree>> {
        first_child_node(self.node, SyntaxKind::Block)
            .map(|block| {
                block
                    .children
                    .iter()
                    .filter_map(|child| match child {
                        SyntaxElement::Node(node) => {
                            class_member_from_node(self.source, block, node)
                        }
                        SyntaxElement::Token(_) => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn classify_method(&self, method: MethodDecl<'source, 'tree>) -> MethodKind {
        if method.is_destructor() {
            return MethodKind::Destructor;
        }

        match (self.name(), method.name()) {
            (Some(class_name), Some(method_name)) if class_name.text() == method_name.text() => {
                MethodKind::Constructor
            }
            _ => MethodKind::Method,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ClassMember<'source, 'tree> {
    Field(FieldDecl<'source, 'tree>),
    Method(MethodDecl<'source, 'tree>),
    Empty(EmptyDecl<'tree>),
}

#[derive(Debug, Clone, Copy)]
pub struct FieldDecl<'source, 'tree> {
    source: &'source str,
    container: &'tree SyntaxNode,
    node: &'tree SyntaxNode,
}

impl<'source, 'tree> FieldDecl<'source, 'tree> {
    pub const fn span(&self) -> TextSpan {
        self.node.span
    }

    pub fn name(&self) -> Option<TextValue<'source>> {
        self.declarators()
            .first()
            .map(|declarator| declarator.name())
    }

    pub fn type_text(&self) -> Option<TextValue<'source>> {
        self.declarators()
            .first()
            .and_then(|declarator| declarator.type_text())
    }

    pub fn declarators(&self) -> Vec<FieldDeclarator<'source>> {
        field_declarators(self.source, self.node)
    }

    pub fn attributes(&self) -> Vec<Attribute<'source, 'tree>> {
        attributes(self.source, self.node)
    }

    pub fn doc_comments(&self) -> Vec<DocComment<'source>> {
        leading_doc_comments(self.source, self.container, self.node)
    }

    pub fn modifiers(&self) -> Vec<TextValue<'source>> {
        modifiers(self.source, self.node)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FieldDeclarator<'source> {
    source: &'source str,
    name: Token,
    type_span: Option<TextSpan>,
    span: TextSpan,
}

impl<'source> FieldDeclarator<'source> {
    pub const fn span(&self) -> TextSpan {
        self.span
    }

    pub fn name(&self) -> TextValue<'source> {
        text_value(self.source, self.name.span)
    }

    pub fn type_text(&self) -> Option<TextValue<'source>> {
        self.type_span.map(|span| text_value(self.source, span))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MethodDecl<'source, 'tree> {
    source: &'source str,
    container: &'tree SyntaxNode,
    node: &'tree SyntaxNode,
}

impl<'source, 'tree> MethodDecl<'source, 'tree> {
    pub const fn span(&self) -> TextSpan {
        self.node.span
    }

    pub fn name(&self) -> Option<TextValue<'source>> {
        method_name_token(self.node).map(|token| text_value(self.source, token.span))
    }

    pub fn is_destructor(&self) -> bool {
        destructor_marker_token(self.node).is_some()
    }

    pub fn return_type_text(&self) -> Option<TextValue<'source>> {
        let name = method_name_token(self.node)?;
        let before = destructor_marker_token(self.node)
            .map(|token| token.span.start)
            .unwrap_or(name.span.start);
        leading_decl_text_before(self.source, self.node, before)
    }

    pub fn parameters(&self) -> Vec<Parameter<'source, 'tree>> {
        self.all_parameters()
            .into_iter()
            .filter(|parameter| parameter.kind() == ParameterKind::Declaration)
            .collect()
    }

    pub fn parameter_fragments(&self) -> Vec<Parameter<'source, 'tree>> {
        self.all_parameters()
            .into_iter()
            .filter(|parameter| parameter.kind() == ParameterKind::NonDeclarationFragment)
            .collect()
    }

    fn all_parameters(&self) -> Vec<Parameter<'source, 'tree>> {
        first_child_node(self.node, SyntaxKind::ParameterList)
            .map(|list| {
                list.children
                    .iter()
                    .filter_map(|child| match child {
                        SyntaxElement::Node(node) if node.kind == SyntaxKind::Parameter => {
                            Some(Parameter {
                                source: self.source,
                                node,
                            })
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn attributes(&self) -> Vec<Attribute<'source, 'tree>> {
        attributes(self.source, self.node)
    }

    pub fn doc_comments(&self) -> Vec<DocComment<'source>> {
        leading_doc_comments(self.source, self.container, self.node)
    }

    pub fn modifiers(&self) -> Vec<TextValue<'source>> {
        modifiers(self.source, self.node)
    }

    pub fn body_span(&self) -> Option<TextSpan> {
        first_child_node(self.node, SyntaxKind::Block).map(|node| node.span)
    }

    pub fn local_variables(&self) -> Vec<LocalVariable<'source>> {
        first_child_node(self.node, SyntaxKind::Block)
            .map(|block| local_variables_in_block(self.source, block))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TypeParameter<'source> {
    source: &'source str,
    name: Token,
    constraint_span: Option<TextSpan>,
    span: TextSpan,
}

impl<'source> TypeParameter<'source> {
    pub const fn span(&self) -> TextSpan {
        self.span
    }

    pub fn name(&self) -> TextValue<'source> {
        text_value(self.source, self.name.span)
    }

    pub fn constraint_text(&self) -> Option<TextValue<'source>> {
        self.constraint_span
            .map(|span| text_value(self.source, span))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Parameter<'source, 'tree> {
    source: &'source str,
    node: &'tree SyntaxNode,
}

impl<'source, 'tree> Parameter<'source, 'tree> {
    pub const fn span(&self) -> TextSpan {
        self.node.span
    }

    pub fn text(&self) -> Option<TextValue<'source>> {
        trimmed_node_text(self.source, self.node)
    }

    pub fn kind(&self) -> ParameterKind {
        if self.type_text().is_none()
            && self
                .text()
                .is_some_and(|text| is_literal_parameter_fragment(text.text().trim()))
        {
            ParameterKind::NonDeclarationFragment
        } else {
            ParameterKind::Declaration
        }
    }

    pub fn name(&self) -> Option<TextValue<'source>> {
        parameter_name_token(self.node).map(|token| text_value(self.source, token.span))
    }

    pub fn type_text(&self) -> Option<TextValue<'source>> {
        let name = parameter_name_token(self.node)?;
        leading_parameter_type_text_before(self.source, self.node, name.span.start)
    }

    pub fn default_text(&self) -> Option<TextValue<'source>> {
        let equal = parameter_default_equal_token(self.node)?;
        trailing_parameter_default_text_after(self.source, self.node, equal.span.end)
    }

    pub fn modifiers(&self) -> Vec<TextValue<'source>> {
        parameter_modifier_tokens(self.node)
            .into_iter()
            .map(|token| text_value(self.source, token.span))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct LocalVariable<'source> {
    source: &'source str,
    kind: LocalVariableKind,
    name: Token,
    type_span: Option<TextSpan>,
    default_span: Option<TextSpan>,
    span: TextSpan,
    modifiers: Vec<Token>,
}

impl<'source> LocalVariable<'source> {
    pub const fn span(&self) -> TextSpan {
        self.span
    }

    pub const fn kind(&self) -> LocalVariableKind {
        self.kind
    }

    pub fn name(&self) -> TextValue<'source> {
        text_value(self.source, self.name.span)
    }

    pub fn type_text(&self) -> Option<TextValue<'source>> {
        self.type_span.map(|span| text_value(self.source, span))
    }

    pub fn default_text(&self) -> Option<TextValue<'source>> {
        self.default_span.map(|span| text_value(self.source, span))
    }

    pub fn modifiers(&self) -> Vec<TextValue<'source>> {
        self.modifiers
            .iter()
            .map(|token| text_value(self.source, token.span))
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EnumDecl<'source, 'tree> {
    source: &'source str,
    container: &'tree SyntaxNode,
    node: &'tree SyntaxNode,
}

impl<'source, 'tree> EnumDecl<'source, 'tree> {
    pub const fn span(&self) -> TextSpan {
        self.node.span
    }

    pub fn name(&self) -> Option<TextValue<'source>> {
        let mut seen_enum = false;
        for token in direct_tokens(self.node).filter(|token| !token.kind.is_trivia()) {
            if seen_enum && is_name_token(token.kind) {
                return Some(text_value(self.source, token.span));
            }
            if token.kind == TokenKind::Keyword(Keyword::Enum) {
                seen_enum = true;
            }
        }
        None
    }

    pub fn attributes(&self) -> Vec<Attribute<'source, 'tree>> {
        attributes(self.source, self.node)
    }

    pub fn base_type(&self) -> Option<TextValue<'source>> {
        first_child_node(self.node, SyntaxKind::TypeRef)
            .and_then(|node| trimmed_node_text(self.source, node))
    }

    pub fn doc_comments(&self) -> Vec<DocComment<'source>> {
        leading_doc_comments(self.source, self.container, self.node)
    }

    pub fn members(&self) -> Vec<EnumMember<'source, 'tree>> {
        self.node
            .children
            .iter()
            .filter_map(|child| match child {
                SyntaxElement::Node(node) if node.kind == SyntaxKind::EnumMember => {
                    Some(EnumMember {
                        source: self.source,
                        node,
                    })
                }
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EnumMember<'source, 'tree> {
    source: &'source str,
    node: &'tree SyntaxNode,
}

impl<'source, 'tree> EnumMember<'source, 'tree> {
    pub const fn span(&self) -> TextSpan {
        self.node.span
    }

    pub fn name(&self) -> Option<TextValue<'source>> {
        direct_tokens(self.node)
            .find(|token| is_name_token(token.kind))
            .map(|token| text_value(self.source, token.span))
    }

    pub fn value_text(&self) -> Option<TextValue<'source>> {
        enum_member_value_text(self.source, self.node)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TypedefDecl<'source, 'tree> {
    source: &'source str,
    container: &'tree SyntaxNode,
    node: &'tree SyntaxNode,
}

impl<'source, 'tree> TypedefDecl<'source, 'tree> {
    pub const fn text_span(&self) -> TextSpan {
        self.node.span
    }

    pub fn name(&self) -> Option<TextValue<'source>> {
        typedef_name_token(self.node).map(|token| text_value(self.source, token.span))
    }

    pub fn type_text(&self) -> Option<TextValue<'source>> {
        let name = typedef_name_token(self.node)?;
        typedef_type_text_before(self.source, self.node, name.span.start)
    }

    pub fn doc_comments(&self) -> Vec<DocComment<'source>> {
        leading_doc_comments(self.source, self.container, self.node)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Attribute<'source, 'tree> {
    source: &'source str,
    node: &'tree SyntaxNode,
}

impl<'source, 'tree> Attribute<'source, 'tree> {
    pub const fn span(&self) -> TextSpan {
        self.node.span
    }

    pub fn text(&self) -> Option<TextValue<'source>> {
        trimmed_node_text(self.source, self.node)
    }

    pub fn name(&self) -> Option<TextValue<'source>> {
        direct_tokens(self.node)
            .find(|token| is_name_token(token.kind))
            .map(|token| text_value(self.source, token.span))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EmptyDecl<'tree> {
    node: &'tree SyntaxNode,
}

impl EmptyDecl<'_> {
    pub const fn span(&self) -> TextSpan {
        self.node.span
    }
}

fn declaration_from_node<'source, 'tree>(
    source: &'source str,
    container: &'tree SyntaxNode,
    node: &'tree SyntaxNode,
) -> Option<Declaration<'source, 'tree>> {
    match node.kind {
        SyntaxKind::ClassDecl => Some(Declaration::Class(ClassDecl {
            source,
            container,
            node,
        })),
        SyntaxKind::EnumDecl => Some(Declaration::Enum(EnumDecl {
            source,
            container,
            node,
        })),
        SyntaxKind::TypedefDecl => Some(Declaration::Typedef(TypedefDecl {
            source,
            container,
            node,
        })),
        SyntaxKind::FunctionDecl => Some(Declaration::Function(MethodDecl {
            source,
            container,
            node,
        })),
        SyntaxKind::FieldDecl => Some(Declaration::Field(FieldDecl {
            source,
            container,
            node,
        })),
        _ => None,
    }
}

fn class_member_from_node<'source, 'tree>(
    source: &'source str,
    container: &'tree SyntaxNode,
    node: &'tree SyntaxNode,
) -> Option<ClassMember<'source, 'tree>> {
    match node.kind {
        SyntaxKind::FieldDecl => Some(ClassMember::Field(FieldDecl {
            source,
            container,
            node,
        })),
        SyntaxKind::MethodDecl => Some(ClassMember::Method(MethodDecl {
            source,
            container,
            node,
        })),
        SyntaxKind::EmptyDecl => Some(ClassMember::Empty(EmptyDecl { node })),
        _ => None,
    }
}

fn leading_doc_comments<'source>(
    source: &'source str,
    container: &SyntaxNode,
    target: &SyntaxNode,
) -> Vec<DocComment<'source>> {
    let Some(target_index) = container.children.iter().position(|child| match child {
        SyntaxElement::Node(node) => std::ptr::eq(node.as_ref(), target),
        SyntaxElement::Token(_) => false,
    }) else {
        return Vec::new();
    };

    let mut comments = Vec::new();
    'outer: for child in container.children[..target_index].iter().rev() {
        match child {
            SyntaxElement::Token(token) => match token.kind {
                TokenKind::Whitespace => {}
                kind if is_doc_comment_token(kind) => comments.push(DocComment {
                    source,
                    token: *token,
                }),
                _ => break,
            },
            SyntaxElement::Node(node) => {
                for child in node.children.iter().rev() {
                    match child {
                        SyntaxElement::Token(token) => match token.kind {
                            TokenKind::Whitespace => {}
                            kind if is_doc_comment_token(kind) => comments.push(DocComment {
                                source,
                                token: *token,
                            }),
                            _ => break 'outer,
                        },
                        SyntaxElement::Node(_) => break 'outer,
                    }
                }
            }
        }
    }

    comments.reverse();
    comments
}

fn attributes<'source, 'tree>(
    source: &'source str,
    node: &'tree SyntaxNode,
) -> Vec<Attribute<'source, 'tree>> {
    node.children
        .iter()
        .filter_map(|child| match child {
            SyntaxElement::Node(attribute_list)
                if attribute_list.kind == SyntaxKind::AttributeList =>
            {
                Some(attribute_list)
            }
            _ => None,
        })
        .flat_map(|attribute_list| {
            attribute_list
                .children
                .iter()
                .filter_map(move |child| match child {
                    SyntaxElement::Node(attribute) if attribute.kind == SyntaxKind::Attribute => {
                        Some(Attribute {
                            source,
                            node: attribute,
                        })
                    }
                    _ => None,
                })
        })
        .collect()
}

fn modifiers<'source>(source: &'source str, node: &SyntaxNode) -> Vec<TextValue<'source>> {
    node.children
        .iter()
        .filter_map(|child| match child {
            SyntaxElement::Node(modifier_list)
                if modifier_list.kind == SyntaxKind::ModifierList =>
            {
                Some(modifier_list)
            }
            _ => None,
        })
        .flat_map(|modifier_list| {
            direct_tokens(modifier_list)
                .filter(|token| !token.kind.is_trivia())
                .map(|token| text_value(source, token.span))
        })
        .collect()
}

fn field_declarators<'source>(
    source: &'source str,
    node: &SyntaxNode,
) -> Vec<FieldDeclarator<'source>> {
    let type_span = declaration_type_span(source, node);
    declaration_declarators(source, node)
        .into_iter()
        .map(|(name, span, _)| FieldDeclarator {
            source,
            name,
            type_span,
            span,
        })
        .collect()
}

fn declaration_declarators(
    source: &str,
    node: &SyntaxNode,
) -> Vec<(Token, TextSpan, Option<TextSpan>)> {
    let Some(list) = first_child_node(node, SyntaxKind::DeclaratorList) else {
        return Vec::new();
    };
    list.children.iter().filter_map(|child| match child {
        SyntaxElement::Node(declarator) if declarator.kind == SyntaxKind::Declarator => {
            let name = direct_tokens(declarator).find(|token| !token.kind.is_trivia() && is_name_token(token.kind))?;
            let before_default = direct_tokens(declarator)
                .find(|token| token.kind == TokenKind::Operator(Operator::Equal))
                .map(|token| token.span.start)
                .unwrap_or(declarator.span.end);
            let span = trim_text_span(source, TextSpan::new(name.span.start, before_default));
            let default_span = declarator.children.iter().skip_while(|child| !matches!(child, SyntaxElement::Token(token) if token.kind == TokenKind::Operator(Operator::Equal))).skip(1).find_map(|child| match child {
                SyntaxElement::Node(expression) => Some(trim_text_span(source, expression.span)),
                SyntaxElement::Token(token) if !token.kind.is_trivia() => Some(token.span),
                _ => None,
            });
            (!span.is_empty()).then_some((name, span, default_span))
        }
        _ => None,
    }).collect()
}

fn declaration_type_span(source: &str, node: &SyntaxNode) -> Option<TextSpan> {
    let type_ref = first_child_node(node, SyntaxKind::TypeRef)?;
    let mut tokens = direct_tokens(type_ref)
        .filter(|token| !token.kind.is_trivia() && !is_local_modifier_token(token.kind));
    let first = tokens.next()?;
    let last = tokens.last().unwrap_or(first);
    let span = trim_text_span(source, TextSpan::new(first.span.start, last.span.end));
    (!span.is_empty()).then_some(span)
}

fn type_parameters_from_generic_arg_list<'source>(
    source: &'source str,
    node: &SyntaxNode,
) -> Vec<TypeParameter<'source>> {
    let tokens = direct_tokens(node)
        .filter(|token| {
            !token.kind.is_trivia()
                && !matches!(
                    token.kind,
                    TokenKind::Operator(Operator::Less)
                        | TokenKind::Operator(Operator::Greater)
                        | TokenKind::Operator(Operator::GreaterGreater)
                )
        })
        .collect::<Vec<_>>();
    split_top_level_preserving_angles(&tokens, TokenKind::Comma)
        .into_iter()
        .filter_map(|segment| type_parameter_from_segment(source, segment))
        .collect()
}

fn type_parameter_from_segment<'source>(
    source: &'source str,
    segment: &[Token],
) -> Option<TypeParameter<'source>> {
    let segment = trim_token_slice(segment);
    let name = segment
        .iter()
        .rev()
        .find(|token| is_name_token(token.kind))
        .copied()?;
    let first = segment.first()?;
    let last = segment.last()?;
    let span = trim_text_span(source, TextSpan::new(first.span.start, last.span.end));
    let constraint_span = if first.span.start < name.span.start {
        let constraint = trim_text_span(source, TextSpan::new(first.span.start, name.span.start));
        (!constraint.is_empty()).then_some(constraint)
    } else {
        None
    };
    Some(TypeParameter {
        source,
        name,
        constraint_span,
        span,
    })
}

fn method_name_token(node: &SyntaxNode) -> Option<Token> {
    direct_tokens(node)
        .take_while(|token| token.kind != TokenKind::LeftParen)
        .filter(|token| is_name_token(token.kind))
        .last()
}

fn typedef_name_token(node: &SyntaxNode) -> Option<Token> {
    recursive_tokens(node)
        .take_while(|token| token.kind != TokenKind::Semicolon)
        .filter(|token| is_name_token(token.kind))
        .last()
}

fn parameter_name_token(node: &SyntaxNode) -> Option<Token> {
    let mut candidate = None;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut angle_depth = 0usize;

    for token in recursive_tokens(node).filter(|token| !token.kind.is_trivia()) {
        let kind = token.kind;
        let at_top_level = paren_depth == 0 && bracket_depth == 0 && angle_depth == 0;

        if at_top_level && kind == TokenKind::Operator(Operator::Equal) {
            break;
        }

        if at_top_level && is_name_token(kind) && !is_parameter_modifier_token(kind) {
            candidate = Some(token);
        }

        match kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Operator(Operator::Less) => angle_depth += 1,
            TokenKind::Operator(Operator::Greater) => angle_depth = angle_depth.saturating_sub(1),
            TokenKind::Operator(Operator::GreaterGreater) => {
                angle_depth = angle_depth.saturating_sub(2)
            }
            _ => {}
        }
    }

    candidate
}

fn parameter_default_equal_token(node: &SyntaxNode) -> Option<Token> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut angle_depth = 0usize;

    for token in direct_tokens(node).filter(|token| !token.kind.is_trivia()) {
        let kind = token.kind;
        if paren_depth == 0
            && bracket_depth == 0
            && angle_depth == 0
            && kind == TokenKind::Operator(Operator::Equal)
        {
            return Some(token);
        }

        match kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Operator(Operator::Less) => angle_depth += 1,
            TokenKind::Operator(Operator::Greater) => angle_depth = angle_depth.saturating_sub(1),
            TokenKind::Operator(Operator::GreaterGreater) => {
                angle_depth = angle_depth.saturating_sub(2)
            }
            _ => {}
        }
    }

    None
}

fn parameter_modifier_tokens(node: &SyntaxNode) -> Vec<Token> {
    let mut modifiers = Vec::new();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut angle_depth = 0usize;

    for token in direct_tokens(node).filter(|token| !token.kind.is_trivia()) {
        let kind = token.kind;
        let at_top_level = paren_depth == 0 && bracket_depth == 0 && angle_depth == 0;

        if at_top_level && kind == TokenKind::Operator(Operator::Equal) {
            break;
        }

        if at_top_level && is_parameter_modifier_token(kind) {
            modifiers.push(token);
        }

        match kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Operator(Operator::Less) => angle_depth += 1,
            TokenKind::Operator(Operator::Greater) => angle_depth = angle_depth.saturating_sub(1),
            TokenKind::Operator(Operator::GreaterGreater) => {
                angle_depth = angle_depth.saturating_sub(2)
            }
            _ => {}
        }
    }

    modifiers
}

fn local_variables_in_block<'source>(
    source: &'source str,
    block: &SyntaxNode,
) -> Vec<LocalVariable<'source>> {
    let mut locals = Vec::new();
    collect_local_variables_from_node(source, block, &mut locals);
    locals
}

fn collect_local_variables_from_node<'source>(
    source: &'source str,
    node: &SyntaxNode,
    locals: &mut Vec<LocalVariable<'source>>,
) {
    match node.kind {
        SyntaxKind::LocalDeclStatement => {
            push_structured_local_variables(source, locals, node, LocalVariableKind::LocalVariable);
            return;
        }
        SyntaxKind::ForInitializer => {
            push_for_initializer_variables_from_node(source, locals, node);
            return;
        }
        SyntaxKind::ForeachVariable => {
            // Foreach is a parser-owned header item with one structured
            // type/declarator pair and no initializer.
            push_foreach_variable(source, locals, node);
            return;
        }
        _ => {}
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_local_variables_from_node(source, child, locals);
        }
    }
}

fn push_for_initializer_variables_from_node<'source>(
    source: &'source str,
    locals: &mut Vec<LocalVariable<'source>>,
    node: &SyntaxNode,
) {
    for child in &node.children {
        let SyntaxElement::Node(child) = child else {
            continue;
        };
        if child.kind != SyntaxKind::LocalDeclStatement {
            continue;
        }
        push_structured_local_variables(source, locals, child, LocalVariableKind::ForInitializer);
    }
}

fn push_structured_local_variables<'source>(
    source: &'source str,
    locals: &mut Vec<LocalVariable<'source>>,
    node: &SyntaxNode,
    kind: LocalVariableKind,
) {
    let type_span = declaration_type_span(source, node);
    let modifiers = first_child_node(node, SyntaxKind::TypeRef)
        .into_iter()
        .flat_map(direct_tokens)
        .filter(|token| is_local_modifier_token(token.kind))
        .collect::<Vec<_>>();
    for (name, span, default_span) in declaration_declarators(source, node) {
        locals.push(LocalVariable {
            source,
            kind,
            name,
            type_span,
            default_span,
            span,
            modifiers: modifiers.clone(),
        });
    }
}

fn push_foreach_variable<'source>(
    source: &'source str,
    locals: &mut Vec<LocalVariable<'source>>,
    node: &SyntaxNode,
) {
    let Some(declarator) = first_child_node(node, SyntaxKind::Declarator) else {
        return;
    };
    let Some(name) = direct_tokens(declarator)
        .find(|token| !token.kind.is_trivia() && is_name_token(token.kind))
    else {
        return;
    };
    let modifiers = first_child_node(node, SyntaxKind::TypeRef)
        .into_iter()
        .flat_map(direct_tokens)
        .filter(|token| is_local_modifier_token(token.kind))
        .collect();
    locals.push(LocalVariable {
        source,
        kind: LocalVariableKind::ForeachVariable,
        name,
        type_span: declaration_type_span(source, node),
        default_span: None,
        span: text_value(source, name.span).span,
        modifiers,
    });
}

fn split_top_level_preserving_angles(tokens: &[Token], delimiter: TokenKind) -> Vec<&[Token]> {
    let mut segments = Vec::new();
    let mut segment_start = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut angle_depth = 0usize;

    for (index, token) in tokens.iter().enumerate() {
        if paren_depth == 0 && bracket_depth == 0 && angle_depth == 0 && token.kind == delimiter {
            segments.push(trim_token_slice(&tokens[segment_start..index]));
            segment_start = index + 1;
            continue;
        }

        match token.kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Operator(Operator::Less) => angle_depth += 1,
            TokenKind::Operator(Operator::Greater) => angle_depth = angle_depth.saturating_sub(1),
            TokenKind::Operator(Operator::GreaterGreater) => {
                angle_depth = angle_depth.saturating_sub(2)
            }
            _ => {}
        }
    }

    segments.push(trim_token_slice(&tokens[segment_start..]));
    segments
}

fn trim_token_slice(tokens: &[Token]) -> &[Token] {
    let start = tokens
        .iter()
        .position(|token| {
            !matches!(
                token.kind,
                TokenKind::Semicolon | TokenKind::LeftBrace | TokenKind::RightBrace
            )
        })
        .unwrap_or(tokens.len());
    let end = tokens
        .iter()
        .rposition(|token| {
            !matches!(
                token.kind,
                TokenKind::Semicolon | TokenKind::LeftBrace | TokenKind::RightBrace
            )
        })
        .map(|index| index + 1)
        .unwrap_or(start);
    &tokens[start..end]
}

fn is_local_modifier_token(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Keyword(Keyword::Const)
            | TokenKind::Keyword(Keyword::Out)
            | TokenKind::Keyword(Keyword::Inout)
            | TokenKind::Keyword(Keyword::Notnull)
    )
}

fn enum_member_value_text<'source>(
    source: &'source str,
    node: &SyntaxNode,
) -> Option<TextValue<'source>> {
    let mut seen_equal = false;
    let mut tokens = Vec::new();

    for token in recursive_tokens(node).filter(|token| !token.kind.is_trivia()) {
        if !seen_equal {
            if token.kind == TokenKind::Operator(Operator::Equal) {
                seen_equal = true;
            }
            continue;
        }

        if token.kind == TokenKind::Comma {
            break;
        }

        tokens.push(token);
    }

    let first = tokens.first()?;
    let last = tokens.last()?;
    if first.span.start >= last.span.end {
        return None;
    }

    Some(text_value(
        source,
        TextSpan::new(first.span.start, last.span.end),
    ))
}

fn destructor_marker_token(node: &SyntaxNode) -> Option<Token> {
    let name = method_name_token(node)?;

    direct_tokens(node)
        .take_while(|token| token.kind != TokenKind::LeftParen)
        .filter(|token| !token.kind.is_trivia())
        .find(|token| {
            token.span.end <= name.span.start && token.kind == TokenKind::Operator(Operator::Tilde)
        })
}

fn leading_decl_text_before<'source>(
    source: &'source str,
    node: &SyntaxNode,
    before: usize,
) -> Option<TextValue<'source>> {
    let tokens: Vec<Token> = direct_tokens(node)
        .filter(|token| !token.kind.is_trivia())
        .filter(|token| token.span.end <= before)
        .filter(|token| token.kind != TokenKind::Semicolon)
        .filter(|token| !is_declaration_prefix_token(token.kind))
        .collect();
    let first = tokens.first()?;
    let last = tokens.last()?;
    if first.span.start >= last.span.end {
        return None;
    }
    Some(text_value(
        source,
        TextSpan::new(first.span.start, last.span.end),
    ))
}

fn typedef_type_text_before<'source>(
    source: &'source str,
    node: &SyntaxNode,
    before: usize,
) -> Option<TextValue<'source>> {
    let tokens = recursive_tokens(node)
        .take_while(|token| token.kind != TokenKind::Semicolon)
        .filter(|token| !token.kind.is_trivia())
        .filter(|token| token.span.end <= before)
        .filter(|token| token.kind != TokenKind::Keyword(Keyword::Typedef))
        .collect::<Vec<_>>();
    let first = tokens.first()?;
    let last = tokens.last()?;
    if first.span.start >= last.span.end {
        return None;
    }
    Some(text_value(
        source,
        TextSpan::new(first.span.start, last.span.end),
    ))
}

fn leading_parameter_type_text_before<'source>(
    source: &'source str,
    node: &SyntaxNode,
    before: usize,
) -> Option<TextValue<'source>> {
    let mut tokens = Vec::new();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut angle_depth = 0usize;

    for token in direct_tokens(node).filter(|token| !token.kind.is_trivia()) {
        if token.span.end > before {
            break;
        }

        let kind = token.kind;
        let at_top_level = paren_depth == 0 && bracket_depth == 0 && angle_depth == 0;
        if !(at_top_level && is_parameter_modifier_token(kind)) {
            tokens.push(token);
        }

        match kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Operator(Operator::Less) => angle_depth += 1,
            TokenKind::Operator(Operator::Greater) => angle_depth = angle_depth.saturating_sub(1),
            TokenKind::Operator(Operator::GreaterGreater) => {
                angle_depth = angle_depth.saturating_sub(2)
            }
            _ => {}
        }
    }

    let first = tokens.first()?;
    let last = tokens.last()?;
    if first.span.start >= last.span.end {
        return None;
    }
    Some(text_value(
        source,
        TextSpan::new(first.span.start, last.span.end),
    ))
}

fn trailing_parameter_default_text_after<'source>(
    source: &'source str,
    node: &SyntaxNode,
    after: usize,
) -> Option<TextValue<'source>> {
    let tokens: Vec<Token> = recursive_tokens(node)
        .filter(|token| !token.kind.is_trivia())
        .filter(|token| token.span.start >= after)
        .collect();
    let first = tokens.first()?;
    let last = tokens.last()?;
    if first.span.start >= last.span.end {
        return None;
    }
    Some(text_value(
        source,
        TextSpan::new(first.span.start, last.span.end),
    ))
}

fn first_child_node(node: &SyntaxNode, kind: SyntaxKind) -> Option<&SyntaxNode> {
    node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(node) if node.kind == kind => Some(node.as_ref()),
        _ => None,
    })
}

pub fn smallest_expression_at_offset<'source, 'tree>(
    source: &'source str,
    root: &'tree SyntaxNode,
    offset: usize,
) -> Option<Expression<'source, 'tree>> {
    let mut best = None;
    collect_smallest_expression_at_offset(source, root, offset, &mut best);
    best
}

pub fn member_access_for_member_name_at_offset<'source, 'tree>(
    source: &'source str,
    root: &'tree SyntaxNode,
    token_span: TextSpan,
) -> Option<MemberAccessExpression<'source, 'tree>> {
    let mut best = None;
    collect_member_access_for_member_name(source, root, token_span, &mut best);
    best
}

pub fn named_argument_label_at_offset<'source>(
    source: &'source str,
    root: &SyntaxNode,
    token_span: TextSpan,
) -> Option<NamedArgumentLabel<'source>> {
    let mut result = None;
    collect_named_argument_label(source, root, token_span, &mut result);
    result
}

fn collect_smallest_expression_at_offset<'source, 'tree>(
    source: &'source str,
    node: &'tree SyntaxNode,
    offset: usize,
    best: &mut Option<Expression<'source, 'tree>>,
) {
    if !span_contains_offset(node.span, offset) {
        return;
    }

    if let Some(expression) = Expression::from_node(source, node) {
        let replace = best
            .as_ref()
            .map(|best| expression.span().len() < best.span().len())
            .unwrap_or(true);
        if replace {
            *best = Some(expression);
        }
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_smallest_expression_at_offset(source, child, offset, best);
        }
    }
}

fn collect_member_access_for_member_name<'source, 'tree>(
    source: &'source str,
    node: &'tree SyntaxNode,
    token_span: TextSpan,
    best: &mut Option<MemberAccessExpression<'source, 'tree>>,
) {
    if !span_contains_span(node.span, token_span) {
        return;
    }

    if node.kind == SyntaxKind::MemberAccessExpression {
        if let Some(expression) = Expression::from_node(source, node) {
            if let (Some(receiver), Some(member_name)) =
                (expression.receiver(), expression.member_name())
            {
                if member_name.span == token_span {
                    let candidate = MemberAccessExpression {
                        expression,
                        receiver,
                        member_name,
                    };
                    let replace = best
                        .as_ref()
                        .map(|best| {
                            candidate.expression.span().len() < best.expression.span().len()
                        })
                        .unwrap_or(true);
                    if replace {
                        *best = Some(candidate);
                    }
                }
            }
        }
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_member_access_for_member_name(source, child, token_span, best);
        }
    }
}

fn collect_named_argument_label<'source>(
    source: &'source str,
    node: &SyntaxNode,
    token_span: TextSpan,
    result: &mut Option<NamedArgumentLabel<'source>>,
) {
    if result.is_some() || !span_contains_span(node.span, token_span) {
        return;
    }

    if node.kind == SyntaxKind::NamedArgument {
        let mut seen_label = None;
        for child in &node.children {
            match child {
                SyntaxElement::Token(token)
                    if token.span == token_span && is_name_token(token.kind) =>
                {
                    seen_label = Some(text_value(source, token.span));
                }
                SyntaxElement::Token(token) if token.kind == TokenKind::Colon => {
                    if let Some(name) = seen_label {
                        *result = Some(NamedArgumentLabel { name });
                    }
                    return;
                }
                SyntaxElement::Node(child) if child.kind == SyntaxKind::NameExpression => {
                    if let Some(name) =
                        Expression::from_node(source, child).and_then(|expr| expr.name_text())
                    {
                        if name.span == token_span {
                            seen_label = Some(name);
                        }
                    }
                }
                SyntaxElement::Node(_) => {}
                _ => {}
            }
        }
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_named_argument_label(source, child, token_span, result);
        }
    }
}

fn first_expression_child<'source, 'tree>(
    source: &'source str,
    node: &'tree SyntaxNode,
) -> Option<Expression<'source, 'tree>> {
    node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(node) => Expression::from_node(source, node),
        SyntaxElement::Token(_) => None,
    })
}

fn first_expression_descendant<'source, 'tree>(
    source: &'source str,
    node: &'tree SyntaxNode,
) -> Option<Expression<'source, 'tree>> {
    if let Some(expression) = Expression::from_node(source, node) {
        return Some(expression);
    }

    node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(node) => first_expression_descendant(source, node),
        SyntaxElement::Token(_) => None,
    })
}

fn first_expression_in_argument<'source, 'tree>(
    source: &'source str,
    node: &'tree SyntaxNode,
) -> Option<Expression<'source, 'tree>> {
    if node.kind == SyntaxKind::NamedArgument {
        return node.children.iter().find_map(|child| match child {
            SyntaxElement::Node(node) if node.kind == SyntaxKind::NameExpression => None,
            SyntaxElement::Node(node) => Expression::from_node(source, node),
            SyntaxElement::Token(_) => None,
        });
    }

    Expression::from_node(source, node)
}

fn is_expression_syntax_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Expression
            | SyntaxKind::NameExpression
            | SyntaxKind::LiteralExpression
            | SyntaxKind::ParenthesizedExpression
            | SyntaxKind::UnaryExpression
            | SyntaxKind::BinaryExpression
            | SyntaxKind::AssignmentExpression
            | SyntaxKind::TernaryExpression
            | SyntaxKind::CallExpression
            | SyntaxKind::NamedArgument
            | SyntaxKind::MemberAccessExpression
            | SyntaxKind::IndexExpression
            | SyntaxKind::CastExpression
            | SyntaxKind::PostfixExpression
            | SyntaxKind::NewExpression
            | SyntaxKind::InitializerExpression
    )
}

const fn span_contains_offset(span: TextSpan, offset: usize) -> bool {
    span.start <= offset && offset < span.end
}

const fn span_contains_span(outer: TextSpan, inner: TextSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn trimmed_node_text<'source>(
    source: &'source str,
    node: &SyntaxNode,
) -> Option<TextValue<'source>> {
    let mut tokens = recursive_tokens(node).filter(|token| !token.kind.is_trivia());
    let first = tokens.next()?;
    let last = tokens.last().unwrap_or(first);
    Some(text_value(
        source,
        TextSpan::new(first.span.start, last.span.end),
    ))
}

fn direct_tokens(node: &SyntaxNode) -> impl Iterator<Item = Token> + '_ {
    node.children.iter().filter_map(|child| match child {
        SyntaxElement::Token(token) => Some(*token),
        SyntaxElement::Node(_) => None,
    })
}

fn recursive_tokens(node: &SyntaxNode) -> Box<dyn Iterator<Item = Token> + '_> {
    Box::new(node.children.iter().flat_map(|child| match child {
        SyntaxElement::Token(token) => {
            Box::new(std::iter::once(*token)) as Box<dyn Iterator<Item = Token>>
        }
        SyntaxElement::Node(node) => recursive_tokens(node),
    }))
}

const fn text_value(source: &str, span: TextSpan) -> TextValue<'_> {
    TextValue { span, source }
}

fn trim_text_span(source: &str, span: TextSpan) -> TextSpan {
    let mut start = span.start;
    let mut end = span.end;

    while start < end {
        let Some(value) = source[start..end].chars().next() else {
            break;
        };
        if !value.is_whitespace() {
            break;
        }
        start += value.len_utf8();
    }

    while start < end {
        let Some(value) = source[start..end].chars().next_back() else {
            break;
        };
        if !value.is_whitespace() {
            break;
        }
        end -= value.len_utf8();
    }

    TextSpan::new(start, end)
}

fn is_name_token(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Identifier | TokenKind::Keyword(_))
}

fn is_declaration_prefix_token(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Keyword(Keyword::Class)
            | TokenKind::Keyword(Keyword::Enum)
            | TokenKind::Keyword(Keyword::Typedef)
            | TokenKind::Keyword(Keyword::Private)
            | TokenKind::Keyword(Keyword::Protected)
            | TokenKind::Keyword(Keyword::Static)
            | TokenKind::Keyword(Keyword::Override)
            | TokenKind::Keyword(Keyword::Const)
            | TokenKind::Keyword(Keyword::Proto)
            | TokenKind::Keyword(Keyword::External)
            | TokenKind::Keyword(Keyword::Native)
            | TokenKind::Keyword(Keyword::Volatile)
            | TokenKind::Keyword(Keyword::Owned)
            | TokenKind::Keyword(Keyword::Event)
            | TokenKind::Keyword(Keyword::Modded)
            | TokenKind::Keyword(Keyword::Sealed)
    )
}

fn is_parameter_modifier_token(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Keyword(Keyword::Out)
            | TokenKind::Keyword(Keyword::Inout)
            | TokenKind::Keyword(Keyword::Notnull)
    )
}

fn is_doc_comment_token(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::DocLineComment | TokenKind::DocBlockComment)
}

fn is_literal_parameter_fragment(text: &str) -> bool {
    matches!(text, "true" | "false" | "null" | "NULL")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;

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

    fn first_method<'source, 'tree>(
        ast: &AstSourceFile<'source, 'tree>,
    ) -> MethodDecl<'source, 'tree> {
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let ClassMember::Method(method) = class.members()[0] else {
            panic!("expected method");
        };
        method
    }

    fn local_facts(
        method: MethodDecl<'_, '_>,
    ) -> Vec<(String, Option<String>, Option<String>, LocalVariableKind)> {
        method
            .local_variables()
            .iter()
            .map(|local| {
                (
                    local.name().text().to_string(),
                    local.type_text().map(|value| value.text().to_string()),
                    local.default_text().map(|value| value.text().to_string()),
                    local.kind(),
                )
            })
            .collect()
    }

    #[test]
    fn extracts_top_level_declarations() {
        let source = r#"[BaseContainerProps()]
modded class SCR_Example : Managed
{
	[Attribute()]
	protected ref array<ref SCR_Item> m_aItems = {};
	proto native bool Find(map<TKey,TValue> from, out TValue value);
}

[EnumBitFlag()]
enum EExample
{
	One,
	Two = 2,
}

typedef map<ref Managed, ref Managed> TManagedMap;

Game g_Game;
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(declarations.len(), 4);

        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        assert_eq!(class.name().unwrap().text(), "SCR_Example");
        assert_eq!(class.base_type().unwrap().text(), "Managed");
        assert_eq!(class.attributes().len(), 1);
        assert_eq!(
            class
                .modifiers()
                .into_iter()
                .map(TextValue::text)
                .collect::<Vec<_>>(),
            vec!["modded"]
        );

        let Declaration::Enum(enum_decl) = declarations[1] else {
            panic!("expected enum");
        };
        assert_eq!(enum_decl.name().unwrap().text(), "EExample");
        assert_eq!(enum_decl.attributes().len(), 1);
        assert_eq!(
            enum_decl.attributes()[0].name().unwrap().text(),
            "EnumBitFlag"
        );
        assert_eq!(
            enum_decl
                .members()
                .into_iter()
                .map(|member| member.name().unwrap().text())
                .collect::<Vec<_>>(),
            vec!["One", "Two"]
        );
        assert_eq!(enum_decl.members()[0].value_text(), None);
        assert_eq!(enum_decl.members()[1].value_text().unwrap().text(), "2");

        let Declaration::Typedef(typedef_decl) = declarations[2] else {
            panic!("expected typedef");
        };
        assert_eq!(typedef_decl.name().unwrap().text(), "TManagedMap");
        assert_eq!(
            typedef_decl.type_text().unwrap().text(),
            "map<ref Managed, ref Managed>"
        );

        let Declaration::Field(field) = declarations[3] else {
            panic!("expected top-level field");
        };
        assert_eq!(field.name().unwrap().text(), "g_Game");
        assert_eq!(field.type_text().unwrap().text(), "Game");
    }

    #[test]
    fn extracts_typedef_aliased_type_text() {
        let source = r#"typedef string FactionKey;
typedef func Callback;
typedef map<ref Managed, ref Managed> TManagedMap;
typedef ScriptInvokerBase<Callback> ScriptInvoker;
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(declarations.len(), 4);

        let expected = [
            ("FactionKey", "string"),
            ("Callback", "func"),
            ("TManagedMap", "map<ref Managed, ref Managed>"),
            ("ScriptInvoker", "ScriptInvokerBase<Callback>"),
        ];

        for (declaration, (expected_name, expected_type)) in declarations.into_iter().zip(expected)
        {
            let Declaration::Typedef(typedef_decl) = declaration else {
                panic!("expected typedef");
            };
            assert_eq!(typedef_decl.name().unwrap().text(), expected_name);
            assert_eq!(typedef_decl.type_text().unwrap().text(), expected_type);
        }
    }

    #[test]
    fn extracts_class_generic_type_parameters() {
        let source = r#"class array<Class T>: Managed
{
}

class map<Class TKey,Class TValue>: Managed
{
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let classes = ast
            .declarations()
            .into_iter()
            .filter_map(|declaration| match declaration {
                Declaration::Class(class) => Some(class),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0].type_parameters()[0].name().text(), "T");
        assert_eq!(
            classes[0].type_parameters()[0]
                .constraint_text()
                .unwrap()
                .text(),
            "Class"
        );
        assert_eq!(classes[1].type_parameters()[0].name().text(), "TKey");
        assert_eq!(classes[1].type_parameters()[1].name().text(), "TValue");
    }

    #[test]
    fn extracts_top_level_fields_separately_from_class_fields() {
        let source = r#"Game g_Game;

class Example
{
	protected int m_iValue;
}

ArmaReforgerScripted g_ARGame;
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(declarations.len(), 3);

        let Declaration::Field(first_global) = declarations[0] else {
            panic!("expected first top-level field");
        };
        assert_eq!(first_global.name().unwrap().text(), "g_Game");
        assert_eq!(first_global.type_text().unwrap().text(), "Game");

        let Declaration::Class(class) = declarations[1] else {
            panic!("expected class");
        };
        let members = class.members();
        let ClassMember::Field(class_field) = members[0] else {
            panic!("expected class field");
        };
        assert_eq!(class_field.name().unwrap().text(), "m_iValue");
        assert_eq!(class_field.type_text().unwrap().text(), "int");

        let Declaration::Field(second_global) = declarations[2] else {
            panic!("expected second top-level field");
        };
        assert_eq!(second_global.name().unwrap().text(), "g_ARGame");
        assert_eq!(
            second_global.type_text().unwrap().text(),
            "ArmaReforgerScripted"
        );
    }

    #[test]
    fn extracts_class_members_and_parameters() {
        let source = r#"class Example extends Base
{
	protected ref array<int> m_aValues = {};
	;
	static override bool GetCost(IEntityComponentSource source, out notnull array<ref SCR_Value> values);
	void WithDefault(int value = Math.Clamp(1, 2, 3), string name = "ok")
	{
	}
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let members = class.members();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(class.base_type().unwrap().text(), "Base");
        assert_eq!(members.len(), 4);

        let ClassMember::Field(field) = members[0] else {
            panic!("expected field");
        };
        assert_eq!(field.name().unwrap().text(), "m_aValues");
        assert_eq!(field.type_text().unwrap().text(), "ref array<int>");
        assert_eq!(field.modifiers()[0].text(), "protected");

        let ClassMember::Empty(empty) = members[1] else {
            panic!("expected empty declaration");
        };
        assert!(!empty.span().is_empty());

        let ClassMember::Method(method) = members[2] else {
            panic!("expected method");
        };
        assert_eq!(method.name().unwrap().text(), "GetCost");
        assert_eq!(method.return_type_text().unwrap().text(), "bool");
        assert_eq!(method.parameters().len(), 2);
        assert_eq!(
            method.parameters()[1].text().unwrap().text(),
            "out notnull array<ref SCR_Value> values"
        );
        assert_eq!(method.parameters()[1].name().unwrap().text(), "values");
        assert_eq!(
            method.parameters()[1].type_text().unwrap().text(),
            "array<ref SCR_Value>"
        );
        assert_eq!(
            method.parameters()[1]
                .modifiers()
                .into_iter()
                .map(TextValue::text)
                .collect::<Vec<_>>(),
            vec!["out", "notnull"]
        );

        let ClassMember::Method(default_method) = members[3] else {
            panic!("expected method");
        };
        assert_eq!(default_method.parameters().len(), 2);
        assert_eq!(
            default_method.parameters()[0]
                .default_text()
                .unwrap()
                .text(),
            "Math.Clamp(1, 2, 3)"
        );
        assert_eq!(
            default_method.parameters()[1]
                .default_text()
                .unwrap()
                .text(),
            "\"ok\""
        );
        assert!(default_method.body_span().is_some());
    }

    #[test]
    fn extracts_rich_parameter_details() {
        let source = r#"class Example
{
	proto int Copy(map<TKey,TValue> from);
	static override bool GetEntitySourceBudgetCost(IEntityComponentSource editableEntitySource, out notnull array<ref SCR_EntityBudgetValue> budgetValues);
	void WithDefaults(int value = Math.Clamp(1, 2, 3), string name = "ok", void param1 = NULL);
	proto void WriteQuaternion(float val[4]);
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let members = class.members();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let ClassMember::Method(copy) = members[0] else {
            panic!("expected copy method");
        };
        let from = copy.parameters()[0];
        assert_eq!(from.name().unwrap().text(), "from");
        assert_eq!(from.type_text().unwrap().text(), "map<TKey,TValue>");
        assert!(from.default_text().is_none());
        assert!(from.modifiers().is_empty());

        let ClassMember::Method(cost) = members[1] else {
            panic!("expected cost method");
        };
        let budget_values = cost.parameters()[1];
        assert_eq!(budget_values.name().unwrap().text(), "budgetValues");
        assert_eq!(
            budget_values.type_text().unwrap().text(),
            "array<ref SCR_EntityBudgetValue>"
        );
        assert_eq!(
            budget_values
                .modifiers()
                .into_iter()
                .map(TextValue::text)
                .collect::<Vec<_>>(),
            vec!["out", "notnull"]
        );

        let ClassMember::Method(defaults) = members[2] else {
            panic!("expected defaults method");
        };
        let value = defaults.parameters()[0];
        assert_eq!(value.name().unwrap().text(), "value");
        assert_eq!(value.type_text().unwrap().text(), "int");
        assert_eq!(value.default_text().unwrap().text(), "Math.Clamp(1, 2, 3)");

        let name = defaults.parameters()[1];
        assert_eq!(name.name().unwrap().text(), "name");
        assert_eq!(name.type_text().unwrap().text(), "string");
        assert_eq!(name.default_text().unwrap().text(), "\"ok\"");

        let param1 = defaults.parameters()[2];
        assert_eq!(param1.name().unwrap().text(), "param1");
        assert_eq!(param1.type_text().unwrap().text(), "void");
        assert_eq!(param1.default_text().unwrap().text(), "NULL");

        let ClassMember::Method(write_quaternion) = members[3] else {
            panic!("expected quaternion method");
        };
        let val = write_quaternion.parameters()[0];
        assert_eq!(val.text().unwrap().text(), "float val[4]");
        assert_eq!(val.name().unwrap().text(), "val");
        assert_eq!(val.type_text().unwrap().text(), "float");
        assert!(val.default_text().is_none());
    }

    #[test]
    fn classifies_non_declaration_parameter_fragments() {
        let source = r#"class Example
{
	void FilterOutStorages(false);
	void RealParameter(bool bShow = true);
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let members = class.members();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let ClassMember::Method(fragment_method) = members[0] else {
            panic!("expected fragment method");
        };
        assert_eq!(fragment_method.parameters().len(), 0);
        assert_eq!(fragment_method.parameter_fragments().len(), 1);
        assert_eq!(
            fragment_method.parameter_fragments()[0].kind(),
            ParameterKind::NonDeclarationFragment
        );
        assert_eq!(
            fragment_method.parameter_fragments()[0]
                .text()
                .unwrap()
                .text(),
            "false"
        );

        let ClassMember::Method(real_parameter_method) = members[1] else {
            panic!("expected real parameter method");
        };
        assert_eq!(real_parameter_method.parameters().len(), 1);
        assert_eq!(real_parameter_method.parameter_fragments().len(), 0);
        assert_eq!(
            real_parameter_method.parameters()[0].kind(),
            ParameterKind::Declaration
        );
        assert_eq!(
            real_parameter_method.parameters()[0].name().unwrap().text(),
            "bShow"
        );
        assert_eq!(
            real_parameter_method.parameters()[0]
                .type_text()
                .unwrap()
                .text(),
            "bool"
        );
        assert_eq!(
            real_parameter_method.parameters()[0]
                .default_text()
                .unwrap()
                .text(),
            "true"
        );
    }

    #[test]
    fn extracts_enum_member_values_as_source_text() {
        let source = r#"enum EExample
{
	Foo,
	Bar = 4,
	Baz = Foo | Bar,
	Qux = (1 << 3),
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Enum(enum_decl) = declarations[0] else {
            panic!("expected enum");
        };
        let members = enum_decl.members();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(members[0].name().unwrap().text(), "Foo");
        assert_eq!(members[0].value_text(), None);
        assert_eq!(members[1].name().unwrap().text(), "Bar");
        assert_eq!(members[1].value_text().unwrap().text(), "4");
        assert_eq!(members[2].name().unwrap().text(), "Baz");
        assert_eq!(members[2].value_text().unwrap().text(), "Foo | Bar");
        assert_eq!(members[3].name().unwrap().text(), "Qux");
        assert_eq!(members[3].value_text().unwrap().text(), "(1 << 3)");
    }

    #[test]
    fn attaches_leading_doc_comments_to_declarations_and_members() {
        let source = r#"//! Class docs
//! More class docs

[BaseContainerProps()]
class Example
{
	/*! Field docs */
	protected int m_iValue;

	//! Method docs
	void Run();
}

// normal comment
//! Blocked by normal comment above only if closer than doc
enum EDocumented
{
	Value,
}

//! Blocked docs
#ifdef SOMETHING
class Blocked
{
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let Declaration::Class(class) = declarations[0] else {
            panic!("expected documented class");
        };
        let class_docs = class.doc_comments();
        assert_eq!(class_docs.len(), 2);
        assert_eq!(class_docs[0].kind(), DocCommentKind::Line);
        assert_eq!(class_docs[0].text(), "//! Class docs");
        assert_eq!(class_docs[1].text(), "//! More class docs");

        let members = class.members();
        let ClassMember::Field(field) = members[0] else {
            panic!("expected field");
        };
        assert_eq!(field.doc_comments().len(), 1);
        assert_eq!(field.doc_comments()[0].kind(), DocCommentKind::Block);
        assert_eq!(field.doc_comments()[0].text(), "/*! Field docs */");

        let ClassMember::Method(method) = members[1] else {
            panic!("expected method");
        };
        assert_eq!(method.doc_comments().len(), 1);
        assert_eq!(method.doc_comments()[0].text(), "//! Method docs");

        let Declaration::Enum(enum_decl) = declarations[1] else {
            panic!("expected enum");
        };
        assert_eq!(enum_decl.doc_comments().len(), 1);
        assert_eq!(
            enum_decl.doc_comments()[0].text(),
            "//! Blocked by normal comment above only if closer than doc"
        );

        let Declaration::Class(blocked) = declarations[2] else {
            panic!("expected blocked class");
        };
        assert!(blocked.doc_comments().is_empty());
    }

    #[test]
    fn ignores_attribute_semicolons_when_extracting_field_names() {
        let source = r#"class SCR_DefendWaypointPreset
{
	[Attribute("true", UIWidgets.CheckBox, "Use turrets?")];
	protected bool m_bUseTurrets;

	[Attribute(defvalue: "25")];
	protected float m_fFlashMinDurationMillis;
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let members = class.members();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let ClassMember::Field(first_field) = members[0] else {
            panic!("expected field");
        };
        assert_eq!(first_field.name().unwrap().text(), "m_bUseTurrets");
        assert_eq!(first_field.type_text().unwrap().text(), "bool");
        assert_eq!(first_field.attributes().len(), 1);
        assert_eq!(first_field.modifiers()[0].text(), "protected");

        let ClassMember::Field(second_field) = members[1] else {
            panic!("expected field");
        };
        assert_eq!(
            second_field.name().unwrap().text(),
            "m_fFlashMinDurationMillis"
        );
        assert_eq!(second_field.type_text().unwrap().text(), "float");
    }

    #[test]
    fn extracts_static_array_field_names_before_array_bounds() {
        let source = r#"class Example
{
	static const int TYPE_NAMES_COUNT = 2;
	static const string TYPE_TAGS[TYPE_NAMES_COUNT] =
	{
		"feedback",
		"bug",
	};
	private SCR_BuildingRegion m_RegionConnect_Out[MAX_REGION_CONNECT];
	protected vector m_aDebugLine[POINTS];
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let members = class.members();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let expected = [
            ("TYPE_NAMES_COUNT", "int"),
            ("TYPE_TAGS", "string"),
            ("m_RegionConnect_Out", "SCR_BuildingRegion"),
            ("m_aDebugLine", "vector"),
        ];

        for (member, (expected_name, expected_type)) in members.into_iter().zip(expected) {
            let ClassMember::Field(field) = member else {
                panic!("expected field");
            };
            assert_eq!(field.name().unwrap().text(), expected_name);
            assert_eq!(field.type_text().unwrap().text(), expected_type);
        }
    }

    #[test]
    fn extracts_comma_separated_field_declarators_with_shared_type() {
        let source = r#"class Example
{
	protected Widget m_ContentWidget, m_ButtonPrevWidget, m_ButtonNextWidget;
	protected ref array<int> m_aValues, m_aOtherValues;
	ref SCR_AIEntityWaypointParameters m_EntityWaypointParameters;
	protected int count, values[COUNT], other = 4;
	protected map<Widget, SCR_Item> m_mItems, m_mOtherItems;
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let members = class.members();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let ClassMember::Field(widgets) = members[0] else {
            panic!("expected widget field list");
        };
        let widget_declarators = widgets.declarators();
        assert_eq!(widgets.name().unwrap().text(), "m_ContentWidget");
        assert_eq!(widgets.type_text().unwrap().text(), "Widget");
        assert_eq!(
            widget_declarators
                .iter()
                .map(|declarator| declarator.name().text())
                .collect::<Vec<_>>(),
            vec![
                "m_ContentWidget",
                "m_ButtonPrevWidget",
                "m_ButtonNextWidget"
            ]
        );
        assert!(widget_declarators
            .iter()
            .all(|declarator| declarator.type_text().unwrap().text() == "Widget"));

        let ClassMember::Field(arrays) = members[1] else {
            panic!("expected array field list");
        };
        assert_eq!(
            arrays
                .declarators()
                .iter()
                .map(|declarator| (
                    declarator.name().text(),
                    declarator.type_text().unwrap().text()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("m_aValues", "ref array<int>"),
                ("m_aOtherValues", "ref array<int>")
            ]
        );

        let ClassMember::Field(ref_field) = members[2] else {
            panic!("expected ref field");
        };
        assert_eq!(
            ref_field
                .declarators()
                .iter()
                .map(|declarator| (
                    declarator.name().text(),
                    declarator.type_text().unwrap().text()
                ))
                .collect::<Vec<_>>(),
            vec![(
                "m_EntityWaypointParameters",
                "ref SCR_AIEntityWaypointParameters"
            )]
        );

        let ClassMember::Field(mixed) = members[3] else {
            panic!("expected mixed field list");
        };
        let mixed_declarators = mixed.declarators();
        assert_eq!(
            mixed_declarators
                .iter()
                .map(|declarator| declarator.name().text())
                .collect::<Vec<_>>(),
            vec!["count", "values", "other"]
        );
        assert!(mixed_declarators
            .iter()
            .all(|declarator| declarator.type_text().unwrap().text() == "int"));
        let values_span = mixed_declarators[1].span();
        let other_span = mixed_declarators[2].span();
        assert_eq!(&source[values_span.start..values_span.end], "values[COUNT]");
        assert_eq!(&source[other_span.start..other_span.end], "other");

        let ClassMember::Field(generic) = members[4] else {
            panic!("expected generic field list");
        };
        assert_eq!(
            generic
                .declarators()
                .iter()
                .map(|declarator| (
                    declarator.name().text(),
                    declarator.type_text().unwrap().text()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("m_mItems", "map<Widget, SCR_Item>"),
                ("m_mOtherItems", "map<Widget, SCR_Item>")
            ]
        );
    }

    #[test]
    fn keeps_attribute_attached_to_ref_field_before_methods() {
        let source = r#"class SCR_BoardingEntityWaypoint : SCR_BoardingWaypoint
{
	[Attribute("", UIWidgets.Object, "Related entity")]
	ref SCR_AIEntityWaypointParameters m_EntityWaypointParameters;
	
	string GetEntityName()
	{
		if (m_EntityWaypointParameters)
			return m_EntityWaypointParameters.GetEntityName();
		return "";
	}
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let Declaration::Class(class) = ast.declarations()[0] else {
            panic!("expected class");
        };
        let members = class.members();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ClassMember::Field(field) = members[0] else {
            panic!("expected field");
        };
        assert_eq!(field.attributes().len(), 1);
        assert_eq!(field.name().unwrap().text(), "m_EntityWaypointParameters");
        assert_eq!(
            field.type_text().unwrap().text(),
            "ref SCR_AIEntityWaypointParameters"
        );
    }

    #[test]
    fn extracts_local_variables_from_method_bodies() {
        let source = include_str!("../../tools/fixtures/parser/local_block_symbols.c");
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let ClassMember::Method(method) = class.members()[0] else {
            panic!("expected method");
        };

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let locals = method.local_variables();
        let local_facts = locals
            .iter()
            .map(|local| {
                (
                    local.name().text(),
                    local.type_text().map(TextValue::text),
                    local.default_text().map(TextValue::text),
                    local.kind(),
                    local
                        .modifiers()
                        .into_iter()
                        .map(TextValue::text)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        assert!(
            local_facts.contains(&(
                "outfitDataArray",
                Some("array<SCR_OutfitFactionData>"),
                Some("{}"),
                LocalVariableKind::LocalVariable,
                Vec::new()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "currentBudgetValue",
                Some("int"),
                Some("GetBudgetValue()"),
                LocalVariableKind::LocalVariable,
                vec!["const"]
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "dataEvent",
                Some("ref SCR_PlayerDataEvent"),
                Some("new SCR_PlayerDataEvent"),
                LocalVariableKind::LocalVariable,
                Vec::new()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "playerID",
                Some("int"),
                None,
                LocalVariableKind::LocalVariable,
                Vec::new()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "param2",
                Some("int"),
                None,
                LocalVariableKind::LocalVariable,
                Vec::new()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "debugPoints",
                Some("vector"),
                None,
                LocalVariableKind::LocalVariable,
                Vec::new()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "data",
                Some("SCR_OutfitFactionData"),
                None,
                LocalVariableKind::ForeachVariable,
                Vec::new()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "idx",
                Some("int"),
                None,
                LocalVariableKind::ForeachVariable,
                Vec::new()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "quickslot",
                Some("auto"),
                None,
                LocalVariableKind::ForeachVariable,
                Vec::new()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "i",
                Some("int"),
                Some("0"),
                LocalVariableKind::ForInitializer,
                Vec::new()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "count",
                Some("int"),
                Some("outfitDataArray.Count()"),
                LocalVariableKind::ForInitializer,
                Vec::new()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "currentData",
                Some("SCR_OutfitFactionData"),
                Some("outfitDataArray[i]"),
                LocalVariableKind::LocalVariable,
                Vec::new()
            )),
            "{local_facts:?}"
        );
    }

    #[test]
    fn extracts_static_array_local_defaults_with_brace_initializers() {
        let source = r#"class Example
{
	void Run()
	{
		vector coefMatrix[4] = {m_vTransform[0], m_vTransform[1], m_vTransform[2], vector.Zero};
		vector offsetMatrix[4] = { vector.Zero, vector.Zero, vector.Zero, m_Offset};
		int values[2] = {1, 2};
		int nested[2] = {{1, 2}, {3, 4}};
	}
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let ClassMember::Method(method) = class.members()[0] else {
            panic!("expected method");
        };

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let local_facts = method
            .local_variables()
            .iter()
            .map(|local| {
                (
                    local.name().text().to_string(),
                    local.type_text().map(|value| value.text().to_string()),
                    local.default_text().map(|value| value.text().to_string()),
                    source[local.span().start..local.span().end].to_string(),
                )
            })
            .collect::<Vec<_>>();

        assert!(
            local_facts.contains(&(
                "coefMatrix".to_string(),
                Some("vector".to_string()),
                Some(
                    "{m_vTransform[0], m_vTransform[1], m_vTransform[2], vector.Zero}".to_string()
                ),
                "coefMatrix[4]".to_string()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "offsetMatrix".to_string(),
                Some("vector".to_string()),
                Some("{ vector.Zero, vector.Zero, vector.Zero, m_Offset}".to_string()),
                "offsetMatrix[4]".to_string()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "values".to_string(),
                Some("int".to_string()),
                Some("{1, 2}".to_string()),
                "values[2]".to_string()
            )),
            "{local_facts:?}"
        );
        assert!(
            local_facts.contains(&(
                "nested".to_string(),
                Some("int".to_string()),
                Some("{{1, 2}, {3, 4}}".to_string()),
                "nested[2]".to_string()
            )),
            "{local_facts:?}"
        );
    }

    #[test]
    fn extracts_locals_from_statement_syntax_nodes() {
        let source = r#"class Example
{
	void Run(array<Widget> items)
	{
		Widget value;
		int a, b;
		vector points[4];
		int count = items.Count();
		vector values[4] = {vector.Zero, vector.Zero, vector.Zero, vector.Zero};
		if (count > 0)
		{
			string nested = "ok";
		}
		for (int i = 0, total = items.Count(); i < total; i++)
		{
			Widget current = items[i];
		}
		foreach (int index, Widget item : items)
		{
			item.SetVisible(true);
		}
		string absPath;
		addonsDir += absPath;
		Widget.Make().value = 1;
	}
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let method = first_method(&ast);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert!(count_kind(&parse.root, SyntaxKind::LocalDeclStatement) >= 7);
        assert_eq!(
            count_kind(&parse.root, SyntaxKind::ForStatement),
            1,
            "{:?}",
            parse.root
        );
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForInitializer), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForeachHeader), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForeachVariableList), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForeachVariable), 2);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForeachIterable), 1);

        let facts = local_facts(method);
        assert!(
            facts.contains(&(
                "value".to_string(),
                Some("Widget".to_string()),
                None,
                LocalVariableKind::LocalVariable
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "a".to_string(),
                Some("int".to_string()),
                None,
                LocalVariableKind::LocalVariable
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "b".to_string(),
                Some("int".to_string()),
                None,
                LocalVariableKind::LocalVariable
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "points".to_string(),
                Some("vector".to_string()),
                None,
                LocalVariableKind::LocalVariable
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "count".to_string(),
                Some("int".to_string()),
                Some("items.Count()".to_string()),
                LocalVariableKind::LocalVariable
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "values".to_string(),
                Some("vector".to_string()),
                Some("{vector.Zero, vector.Zero, vector.Zero, vector.Zero}".to_string()),
                LocalVariableKind::LocalVariable
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "nested".to_string(),
                Some("string".to_string()),
                Some("\"ok\"".to_string()),
                LocalVariableKind::LocalVariable
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "i".to_string(),
                Some("int".to_string()),
                Some("0".to_string()),
                LocalVariableKind::ForInitializer
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "total".to_string(),
                Some("int".to_string()),
                Some("items.Count()".to_string()),
                LocalVariableKind::ForInitializer
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "index".to_string(),
                Some("int".to_string()),
                None,
                LocalVariableKind::ForeachVariable
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "item".to_string(),
                Some("Widget".to_string()),
                None,
                LocalVariableKind::ForeachVariable
            )),
            "{facts:?}"
        );
        assert!(
            facts.contains(&(
                "current".to_string(),
                Some("Widget".to_string()),
                Some("items[i]".to_string()),
                LocalVariableKind::LocalVariable
            )),
            "{facts:?}"
        );
        assert_eq!(
            facts
                .iter()
                .filter(|(name, _, _, _)| name == "absPath")
                .count(),
            1,
            "{facts:?}"
        );
        assert!(
            !facts
                .iter()
                .any(|(_, type_text, _, _)| type_text.as_deref() == Some("addonsDir +=")),
            "{facts:?}"
        );
        assert!(
            !facts.iter().any(|(name, _, _, _)| name == "Make"),
            "{facts:?}"
        );
    }

    #[test]
    fn extracts_for_initializer_local_from_compact_header_with_call_condition() {
        let source = r#"class Example
{
	void Run()
	{
		for( int iRow = 0; iRow < m_iMatrix.Count(); iRow++ )
		{
			GetRow(iRow);
		}
	}
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let method = first_method(&ast);
        let facts = local_facts(method);

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        assert_eq!(count_kind(&parse.root, SyntaxKind::ForInitializer), 1);
        assert_eq!(count_kind(&parse.root, SyntaxKind::LocalDeclStatement), 1);
        assert!(
            facts.contains(&(
                "iRow".to_string(),
                Some("int".to_string()),
                Some("0".to_string()),
                LocalVariableKind::ForInitializer
            )),
            "{facts:?}"
        );
    }

    #[test]
    fn exposes_expression_wrappers_and_lookup_helpers() {
        let source = r#"class Example
{
	void Run(array<IEntity> items, int value)
	{
		items[value].GetOrigin();
		IEntity entity = IEntity.Cast(items[0]);
		bool ok = value > 0 ? true : false;
		vector pos = { 1, 2, 3 };
		IEntity spawned = new GenericEntity();
	}
}
"#;
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let get_origin_offset = source.find("GetOrigin").unwrap();
        let expression =
            smallest_expression_at_offset(source, &parse.root, get_origin_offset).unwrap();
        assert_eq!(expression.kind(), ExpressionKind::Name);
        assert_eq!(expression.name_text().unwrap().text(), "GetOrigin");

        let member = member_access_for_member_name_at_offset(
            source,
            &parse.root,
            expression.selection_span(),
        )
        .unwrap();
        assert_eq!(member.member_name.text(), "GetOrigin");
        assert_eq!(member.receiver.source_text(), "items[value]");
        assert_eq!(member.receiver.kind(), ExpressionKind::Index);
        assert_eq!(
            member
                .receiver
                .receiver()
                .and_then(|receiver| receiver.name_text())
                .unwrap()
                .text(),
            "items"
        );
        assert_eq!(
            member
                .receiver
                .index_expression()
                .and_then(|index| index.name_text())
                .unwrap()
                .text(),
            "value"
        );

        let cast_offset = source.find("IEntity.Cast").unwrap() + "IEntity.".len();
        let cast_name = smallest_expression_at_offset(source, &parse.root, cast_offset).unwrap();
        let cast_member = member_access_for_member_name_at_offset(
            source,
            &parse.root,
            cast_name.selection_span(),
        )
        .unwrap();
        assert_eq!(cast_member.expression.kind(), ExpressionKind::MemberAccess);
        assert_eq!(cast_member.receiver.source_text().trim(), "IEntity");

        for (needle, kind) in [
            ("? true", ExpressionKind::Ternary),
            ("{ 1, 2, 3 }", ExpressionKind::Initializer),
            ("new GenericEntity", ExpressionKind::New),
        ] {
            let offset = source.find(needle).unwrap();
            let expression = smallest_expression_at_offset(source, &parse.root, offset).unwrap();
            assert_eq!(expression.kind(), kind, "{needle}");
        }
    }

    #[test]
    fn identifies_named_argument_labels_separately_from_values() {
        let source = r#"class Example
{
	void Run()
	{
		Print("hello", level: LogLevel.WARNING);
	}
}
"#;
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let label_start = source.find("level:").unwrap();
        let label = named_argument_label_at_offset(
            source,
            &parse.root,
            TextSpan::new(label_start, label_start + "level".len()),
        )
        .unwrap();
        assert_eq!(label.name.text(), "level");

        let value_start = source.find("LogLevel").unwrap();
        assert!(named_argument_label_at_offset(
            source,
            &parse.root,
            TextSpan::new(value_start, value_start + "LogLevel".len())
        )
        .is_none());
    }

    #[test]
    fn extracts_destructors_without_tilde_in_return_type() {
        let source = r#"class Example
{
	void Normal();
	void ~Example() {}
	protected void ~Serializer() {}
	proto void ~Shape();
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let members = class.members();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let ClassMember::Method(normal) = members[0] else {
            panic!("expected normal method");
        };
        assert!(!normal.is_destructor());
        assert_eq!(normal.name().unwrap().text(), "Normal");
        assert_eq!(normal.return_type_text().unwrap().text(), "void");

        let ClassMember::Method(simple_destructor) = members[1] else {
            panic!("expected destructor");
        };
        assert!(simple_destructor.is_destructor());
        assert_eq!(simple_destructor.name().unwrap().text(), "Example");
        assert_eq!(simple_destructor.return_type_text().unwrap().text(), "void");

        let ClassMember::Method(protected_destructor) = members[2] else {
            panic!("expected protected destructor");
        };
        assert!(protected_destructor.is_destructor());
        assert_eq!(protected_destructor.name().unwrap().text(), "Serializer");
        assert_eq!(
            protected_destructor
                .modifiers()
                .into_iter()
                .map(TextValue::text)
                .collect::<Vec<_>>(),
            vec!["protected"]
        );
        assert_eq!(
            protected_destructor.return_type_text().unwrap().text(),
            "void"
        );

        let ClassMember::Method(proto_destructor) = members[3] else {
            panic!("expected proto destructor");
        };
        assert!(proto_destructor.is_destructor());
        assert_eq!(proto_destructor.name().unwrap().text(), "Shape");
        assert_eq!(
            proto_destructor
                .modifiers()
                .into_iter()
                .map(TextValue::text)
                .collect::<Vec<_>>(),
            vec!["proto"]
        );
        assert_eq!(proto_destructor.return_type_text().unwrap().text(), "void");
    }

    #[test]
    fn classifies_constructor_methods_with_class_context() {
        let source = r#"class Example
{
	void Example(int value) {}
	private void Example();
	void ~Example() {}
	void Normal() {}
}
"#;
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let declarations = ast.declarations();
        let Declaration::Class(class) = declarations[0] else {
            panic!("expected class");
        };
        let members = class.members();

        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);

        let ClassMember::Method(constructor) = members[0] else {
            panic!("expected constructor");
        };
        assert_eq!(class.classify_method(constructor), MethodKind::Constructor);
        assert_eq!(constructor.name().unwrap().text(), "Example");
        assert_eq!(constructor.return_type_text().unwrap().text(), "void");
        assert_eq!(constructor.parameters().len(), 1);

        let ClassMember::Method(private_constructor) = members[1] else {
            panic!("expected private constructor");
        };
        assert_eq!(
            class.classify_method(private_constructor),
            MethodKind::Constructor
        );
        assert_eq!(
            private_constructor
                .modifiers()
                .into_iter()
                .map(TextValue::text)
                .collect::<Vec<_>>(),
            vec!["private"]
        );
        assert_eq!(
            private_constructor.return_type_text().unwrap().text(),
            "void"
        );

        let ClassMember::Method(destructor) = members[2] else {
            panic!("expected destructor");
        };
        assert_eq!(class.classify_method(destructor), MethodKind::Destructor);
        assert_eq!(destructor.name().unwrap().text(), "Example");
        assert_eq!(destructor.return_type_text().unwrap().text(), "void");

        let ClassMember::Method(normal) = members[3] else {
            panic!("expected normal method");
        };
        assert_eq!(class.classify_method(normal), MethodKind::Method);
        assert_eq!(normal.name().unwrap().text(), "Normal");
        assert_eq!(normal.return_type_text().unwrap().text(), "void");
    }

    #[test]
    fn committed_parser_fixtures_have_extractable_declarations() {
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
            include_str!("../../tools/fixtures/parser/local_block_symbols.c"),
        ];

        for fixture in fixtures {
            let parse = parse_source(fixture);
            let ast = AstSourceFile::new(fixture, &parse);

            assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
            assert!(!ast.declarations().is_empty());
        }
    }
}
