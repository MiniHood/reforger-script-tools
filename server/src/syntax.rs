use crate::lexer::{TextSpan, Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyntaxKind {
    SourceFile,
    AttributeList,
    Attribute,
    AttributeArgs,
    ModifierList,
    ClassDecl,
    EnumDecl,
    EnumMember,
    TypedefDecl,
    FunctionDecl,
    MethodDecl,
    FieldDecl,
    EmptyDecl,
    ParameterList,
    Parameter,
    TypeRef,
    GenericArgList,
    Block,
    InitializerList,
    PreprocessorDirective,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxElement {
    Node(Box<SyntaxNode>),
    Token(Token),
}

impl SyntaxElement {
    pub const fn span(&self) -> TextSpan {
        match self {
            Self::Node(node) => node.span,
            Self::Token(token) => token.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxNode {
    pub kind: SyntaxKind,
    pub span: TextSpan,
    pub children: Vec<SyntaxElement>,
}

impl SyntaxNode {
    pub fn new(kind: SyntaxKind, children: Vec<SyntaxElement>) -> Self {
        let span = span_for_children(&children);
        Self {
            kind,
            span,
            children,
        }
    }

    pub fn token_count(&self) -> usize {
        self.children
            .iter()
            .map(|child| match child {
                SyntaxElement::Node(node) => node.token_count(),
                SyntaxElement::Token(_) => 1,
            })
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub message: String,
    pub span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parse {
    pub root: SyntaxNode,
    pub diagnostics: Vec<ParseDiagnostic>,
}

fn span_for_children(children: &[SyntaxElement]) -> TextSpan {
    let Some(first) = children.first() else {
        return TextSpan::new(0, 0);
    };
    let Some(last) = children.last() else {
        return first.span();
    };

    TextSpan::new(first.span().start, last.span().end)
}
