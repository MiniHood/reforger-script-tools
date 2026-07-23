//! Semantic expected-type and constructible-class queries for incomplete
//! `new` expressions. LSP completion renders these facts but does not derive
//! them.

use crate::ast::{ClassMember, Declaration, Expression, MethodDecl};
use crate::callable::{
    callable_argument_context_at_offset, callable_signature_parts, callable_type_owner,
    CallableTarget,
};
use crate::expression_type::{
    generic_owner_and_args, strip_all_type_prefixes, ExpressionTypeEnvironment,
};
use crate::index::SymbolIndex;
use crate::index_query::{
    EditorCompletionCandidate, EditorCompletionOrigin, EditorTopLevelCompletionMode, IndexQuery,
};
use crate::lexer::{Keyword, Operator, TextSpan, Token, TokenKind};
use crate::model::SymbolKind;
use crate::resolver::{CandidateSource, ReferenceCandidate, ReferenceResolver};
use crate::scope::LexicalScopeModel;
use crate::syntax::{Parse, SyntaxElement, SyntaxKind, SyntaxNode};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextualConstruction {
    pub type_text: String,
    pub containing_class: Option<String>,
}

/// Recovers the expected construction type from an exact declaration or
/// collection-initializer prefix in the current lexical snapshot. This is the
/// source-only subset used while whole-file semantic analysis is converging.
pub fn lexical_construction_context_at_operand(
    source: &str,
    tokens: &[Token],
    operand_span: TextSpan,
) -> Option<ContextualConstruction> {
    let type_text = lexical_declaration_type_before_new(source, tokens, operand_span.start)
        .or_else(|| {
            lexical_initializer_element_type_before_new(source, tokens, operand_span.start)
        })?;
    Some(ContextualConstruction {
        type_text,
        containing_class: None,
    })
}

pub fn compatible_construction_candidates<'index>(
    context: &ContextualConstruction,
    local_index: &'index SymbolIndex,
    external_indexes: impl IntoIterator<Item = &'index SymbolIndex>,
) -> Vec<EditorCompletionCandidate> {
    let Some(owner) = callable_type_owner(&context.type_text) else {
        return Vec::new();
    };
    let external_indexes = external_indexes.into_iter().collect::<Vec<_>>();
    let mut candidates = IndexQuery::new(local_index)
        .completion_top_level("", EditorTopLevelCompletionMode::Type);
    for index in &external_indexes {
        candidates.extend(
            IndexQuery::new(index).completion_top_level("", EditorTopLevelCompletionMode::Type),
        );
    }
    let mut candidates = combine_candidates(candidates)
        .into_iter()
        .filter(|candidate| candidate.kind == SymbolKind::Class)
        .filter(|candidate| {
            let name = candidate_name(candidate);
            name == owner
                || class_inherits_from_indexes(
                    local_index,
                    &external_indexes,
                    name,
                    &owner,
                )
        })
        .filter(|candidate| {
            constructor_is_accessible_from_indexes(
                local_index,
                &external_indexes,
                candidate,
                context,
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        let left_name = candidate_name(left);
        let right_name = candidate_name(right);
        (left_name != owner, left_name).cmp(&(right_name != owner, right_name))
    });
    candidates
}

pub struct ConstructionQuery<'source, 'index> {
    source: &'source str,
    parse: &'index Parse,
    tokens: &'index [Token],
    local_index: &'index SymbolIndex,
    scope: &'index LexicalScopeModel,
    external_indexes: Vec<&'index SymbolIndex>,
}

impl<'source, 'index> ConstructionQuery<'source, 'index> {
    pub fn new(
        source: &'source str,
        parse: &'index Parse,
        tokens: &'index [Token],
        local_index: &'index SymbolIndex,
        scope: &'index LexicalScopeModel,
        external_indexes: impl IntoIterator<Item = &'index SymbolIndex>,
    ) -> Self {
        Self {
            source,
            parse,
            tokens,
            local_index,
            scope,
            external_indexes: external_indexes.into_iter().collect(),
        }
    }

    pub fn context_at_new(&self, new_keyword_span: TextSpan) -> Option<ContextualConstruction> {
        let semantic_type = self
            .assignment_type(new_keyword_span)
            .or_else(|| self.return_type(new_keyword_span))
            .or_else(|| self.argument_type(new_keyword_span))
            .or_else(|| self.declaration_type(new_keyword_span));
        let recovered_type =
            lexical_declaration_type_before_new(self.source, self.tokens, new_keyword_span.start)
                .or_else(|| {
                    lexical_initializer_element_type_before_new(
                        self.source,
                        self.tokens,
                        new_keyword_span.start,
                    )
                });
        let type_text = match (semantic_type, recovered_type) {
            // Both lexical recovery functions prove an exact declarator
            // immediately before this same bare `new`. If that type conflicts
            // with a containing semantic expression/declaration, the parser
            // recovery node cannot describe this declarator and was widened
            // across the missing operand.
            (Some(semantic), Some(recovered)) if semantic != recovered => recovered,
            (Some(semantic), _) => semantic,
            (None, Some(recovered)) => recovered,
            (None, None) => return None,
        };
        Some(ContextualConstruction {
            type_text,
            containing_class: containing_class_name(self.local_index, new_keyword_span.start),
        })
    }

    pub fn compatible_candidates(
        &self,
        context: &ContextualConstruction,
    ) -> Vec<EditorCompletionCandidate> {
        compatible_construction_candidates(
            context,
            self.local_index,
            self.external_indexes.iter().copied(),
        )
    }

    fn declaration_type(&self, new_keyword_span: TextSpan) -> Option<String> {
        for declaration in self.parse.declaration_iter(self.source) {
            match declaration {
                Declaration::Class(class) => {
                    for member in class.members() {
                        match member {
                            ClassMember::Field(field)
                                if contains_span(field.span(), new_keyword_span) =>
                            {
                                return field
                                    .type_text()
                                    .map(|value| value.text().to_string())
                                    .and_then(|type_text| {
                                        self.initializer_element_type(type_text, new_keyword_span)
                                    });
                            }
                            ClassMember::Method(method)
                                if contains_span(method.span(), new_keyword_span) =>
                            {
                                if let Some(type_text) =
                                    self.local_initializer_type(method, new_keyword_span)
                                {
                                    return Some(type_text);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Declaration::Function(method) if contains_span(method.span(), new_keyword_span) => {
                    if let Some(type_text) = self.local_initializer_type(method, new_keyword_span) {
                        return Some(type_text);
                    }
                }
                Declaration::Field(field) if contains_span(field.span(), new_keyword_span) => {
                    return field
                        .type_text()
                        .map(|value| value.text().to_string())
                        .and_then(|type_text| {
                            self.initializer_element_type(type_text, new_keyword_span)
                        });
                }
                _ => {}
            }
        }
        None
    }

    fn local_initializer_type(
        &self,
        method: MethodDecl<'_, '_>,
        new_keyword_span: TextSpan,
    ) -> Option<String> {
        method.local_variables().into_iter().find_map(|local| {
            local
                .default_text()
                .filter(|default| contains_span(default.span, new_keyword_span))
                .and_then(|_| local.type_text())
                .map(|value| value.text().to_string())
                .and_then(|type_text| self.initializer_element_type(type_text, new_keyword_span))
        })
    }

    fn initializer_element_type(
        &self,
        declared_type: String,
        new_keyword_span: TextSpan,
    ) -> Option<String> {
        if let Some(initializer) = innermost_node_containing(
            &self.parse.root,
            SyntaxKind::InitializerExpression,
            new_keyword_span,
        ) {
            if !container_value_is_direct_new(self.source, initializer, new_keyword_span) {
                return None;
            }
            let stripped = strip_all_type_prefixes(&declared_type);
            let (owner, arguments) = generic_owner_and_args(stripped)?;
            if matches!(owner, "array" | "set") && arguments.len() == 1 {
                return arguments.into_iter().next();
            }
            return None;
        }
        if declarator_initializer_is_direct_new(self.source, &self.parse.root, new_keyword_span) {
            return Some(declared_type);
        }
        None
    }

    fn assignment_type(&self, new_keyword_span: TextSpan) -> Option<String> {
        let lhs = assignment_lhs_containing_new(self.source, &self.parse.root, new_keyword_span)?;
        let environment = ExpressionTypeEnvironment::new_with_external_indexes(
            self.source,
            self.local_index,
            self.parse,
            self.scope,
            self.external_indexes.iter().copied(),
        );
        let inferred =
            environment.infer_expression_type(lhs, new_keyword_span.start, &mut Vec::new())?;
        inferred.raw_type_text.or(Some(inferred.owner_type))
    }

    fn return_type(&self, new_keyword_span: TextSpan) -> Option<String> {
        let statement = innermost_node_containing(
            &self.parse.root,
            SyntaxKind::ReturnStatement,
            new_keyword_span,
        )?;
        if !container_value_is_direct_new(self.source, statement, new_keyword_span) {
            return None;
        }
        containing_callable(self.source, self.parse, new_keyword_span)?
            .return_type_text()
            .map(|value| value.text().to_string())
    }

    fn argument_type(&self, new_keyword_span: TextSpan) -> Option<String> {
        let context = callable_argument_context_at_offset(
            self.source,
            &self.parse.root,
            new_keyword_span.end,
        )?;
        let argument_list = innermost_node_with_span(
            &self.parse.root,
            SyntaxKind::ArgumentList,
            context.argument_span,
        )?;
        if !container_value_is_direct_new(self.source, argument_list, new_keyword_span) {
            return None;
        }
        let resolver = ReferenceResolver::new_with_parse_scope_and_external_indexes(
            self.source,
            self.local_index,
            self.parse,
            self.scope,
            self.external_indexes.iter().copied(),
        );
        let callables = match &context.target {
            CallableTarget::Attribute { name } | CallableTarget::New { type_name: name } => {
                self.exact_top_level_candidates(name)
            }
            CallableTarget::Call { callee_span } => resolver
                .resolve_at_offset(callee_span.start)
                .map(|resolution| {
                    combine_candidates(
                        resolution
                            .candidates
                            .iter()
                            .filter_map(|candidate| {
                                self.completion_candidate_for_reference(candidate)
                            })
                            .collect(),
                    )
                })
                .unwrap_or_default(),
        };
        let mut expected_types = BTreeSet::new();
        for callable in callables {
            let label = candidate_name(&callable);
            let signature = match &context.target {
                CallableTarget::New { .. } => callable.constructor_signature.as_deref(),
                _ => callable.signature.as_deref(),
            };
            let Some(call) =
                signature.and_then(|signature| callable_signature_parts(label, signature))
            else {
                continue;
            };
            let parameter = context
                .active_label
                .as_deref()
                .and_then(|label| {
                    call.parameters_info
                        .iter()
                        .find(|parameter| parameter.name == label)
                })
                .or_else(|| call.parameters_info.get(context.argument_index));
            if let Some(parameter) = parameter {
                if parameter_is_output_only(&parameter.type_and_modifiers) {
                    continue;
                }
                expected_types.insert(parameter.type_and_modifiers.clone());
            }
        }
        (expected_types.len() == 1).then(|| expected_types.into_iter().next().unwrap())
    }

    fn exact_top_level_candidates(&self, name: &str) -> Vec<EditorCompletionCandidate> {
        let mut candidates = exact_top_level_candidates(self.local_index, name);
        for index in &self.external_indexes {
            candidates.extend(exact_top_level_candidates(index, name));
        }
        combine_candidates(candidates)
    }

    fn completion_candidate_for_reference(
        &self,
        reference: &ReferenceCandidate,
    ) -> Option<EditorCompletionCandidate> {
        let indexes: Vec<_> = match reference.source {
            CandidateSource::FileLocal => vec![self.local_index],
            CandidateSource::External => self.external_indexes.clone(),
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

}

fn class_inherits_from_indexes(
    local_index: &SymbolIndex,
    external_indexes: &[&SymbolIndex],
    class_name: &str,
    expected_base: &str,
) -> bool {
    let mut current = class_name.to_string();
    let mut seen = BTreeSet::new();
    for _ in 0..32 {
        if !seen.insert(current.clone()) {
            return false;
        }
        let base = class_base_type(local_index, &current).or_else(|| {
            external_indexes
                .iter()
                .find_map(|index| class_base_type(index, &current))
        });
        let Some(base) = base else {
            return false;
        };
        if base == expected_base {
            return true;
        }
        current = base;
    }
    false
}

fn constructor_is_accessible_from_indexes(
    local_index: &SymbolIndex,
    external_indexes: &[&SymbolIndex],
    candidate: &EditorCompletionCandidate,
    context: &ContextualConstruction,
) -> bool {
    let candidate_name = candidate_name(candidate);
    let Some(index) = std::iter::once(local_index)
        .chain(external_indexes.iter().copied())
        .find(|index| candidate_belongs_to_index(candidate, candidate_name, index))
    else {
        return false;
    };
    let constructors = index
        .children(candidate.id)
        .iter()
        .filter_map(|id| index.symbol(*id))
        .filter(|symbol| symbol.kind == SymbolKind::Constructor)
        .collect::<Vec<_>>();
    if constructors.is_empty() {
        return true;
    }
    constructors.into_iter().any(|constructor| {
        let private = constructor
            .modifiers
            .iter()
            .any(|modifier| modifier == "private");
        let protected = constructor
            .modifiers
            .iter()
            .any(|modifier| modifier == "protected");
        if !private && !protected {
            return true;
        }
        if context.containing_class.as_deref() == Some(candidate_name) {
            return true;
        }
        protected
            && context.containing_class.as_deref().is_some_and(|owner| {
                class_inherits_from_indexes(
                    local_index,
                    external_indexes,
                    owner,
                    candidate_name,
                )
            })
    })
}

fn containing_callable<'source, 'tree>(
    source: &'source str,
    parse: &'tree Parse,
    span: TextSpan,
) -> Option<MethodDecl<'source, 'tree>> {
    for declaration in parse.declaration_iter(source) {
        match declaration {
            Declaration::Class(class) => {
                for member in class.members() {
                    if let ClassMember::Method(method) = member {
                        if contains_span(method.span(), span) {
                            return Some(method);
                        }
                    }
                }
            }
            Declaration::Function(method) if contains_span(method.span(), span) => {
                return Some(method);
            }
            _ => {}
        }
    }
    None
}

fn assignment_lhs_containing_new<'source, 'tree>(
    source: &'source str,
    node: &'tree SyntaxNode,
    new_keyword_span: TextSpan,
) -> Option<Expression<'source, 'tree>> {
    if !contains_span(node.span, new_keyword_span) {
        return None;
    }
    if node.kind == SyntaxKind::AssignmentExpression {
        let mut lhs = None;
        let mut after_equal = false;
        for child in &node.children {
            match child {
                SyntaxElement::Token(token)
                    if token.kind == TokenKind::Operator(Operator::Equal) =>
                {
                    after_equal = true;
                }
                SyntaxElement::Node(child) if !after_equal => {
                    if let Some(expression) = Expression::from_node(source, child) {
                        lhs = Some(expression);
                    }
                }
                SyntaxElement::Node(child)
                    if after_equal && contains_span(child.span, new_keyword_span) =>
                {
                    return direct_new_expression(source, child, new_keyword_span)
                        .then_some(lhs)
                        .flatten();
                }
                _ => {}
            }
        }
        return None;
    }
    node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(child) => {
            assignment_lhs_containing_new(source, child, new_keyword_span)
        }
        SyntaxElement::Token(_) => None,
    })
}

fn declarator_initializer_is_direct_new(
    source: &str,
    node: &SyntaxNode,
    new_keyword_span: TextSpan,
) -> bool {
    if !contains_span(node.span, new_keyword_span) {
        return false;
    }
    if node.kind == SyntaxKind::Declarator {
        let mut after_equal = false;
        for child in &node.children {
            match child {
                SyntaxElement::Token(token)
                    if token.kind == TokenKind::Operator(Operator::Equal) =>
                {
                    after_equal = true;
                }
                SyntaxElement::Node(child)
                    if after_equal && contains_span(child.span, new_keyword_span) =>
                {
                    return direct_new_expression(source, child, new_keyword_span);
                }
                _ => {}
            }
        }
        return false;
    }
    node.children.iter().any(|child| match child {
        SyntaxElement::Node(child) => {
            declarator_initializer_is_direct_new(source, child, new_keyword_span)
        }
        SyntaxElement::Token(_) => false,
    })
}

fn container_value_is_direct_new(
    source: &str,
    container: &SyntaxNode,
    new_keyword_span: TextSpan,
) -> bool {
    container.children.iter().any(|child| match child {
        SyntaxElement::Node(child) if contains_span(child.span, new_keyword_span) => {
            if child.kind == SyntaxKind::NamedArgument {
                container_value_is_direct_new(source, child, new_keyword_span)
            } else {
                direct_new_expression(source, child, new_keyword_span)
            }
        }
        _ => false,
    })
}

fn direct_new_expression(source: &str, node: &SyntaxNode, new_keyword_span: TextSpan) -> bool {
    if !contains_span(node.span, new_keyword_span) {
        return false;
    }
    if node.kind == SyntaxKind::NewExpression {
        return node.children.iter().any(|child| {
            matches!(
                child,
                SyntaxElement::Token(token)
                    if token.kind == TokenKind::Keyword(Keyword::New)
                        && token.span == new_keyword_span
            )
        });
    }
    if !matches!(
        node.kind,
        SyntaxKind::ParenthesizedExpression
            | SyntaxKind::MemberAccessExpression
            | SyntaxKind::CallExpression
            | SyntaxKind::IndexExpression
            | SyntaxKind::PostfixExpression
    ) {
        return false;
    }
    node.children
        .iter()
        .filter_map(|child| match child {
            SyntaxElement::Node(child) => Expression::from_node(source, child).map(|_| child),
            SyntaxElement::Token(_) => None,
        })
        .next()
        .is_some_and(|operand| direct_new_expression(source, operand, new_keyword_span))
}

fn innermost_node_containing<'tree>(
    node: &'tree SyntaxNode,
    kind: SyntaxKind,
    contained: TextSpan,
) -> Option<&'tree SyntaxNode> {
    if !contains_span(node.span, contained) {
        return None;
    }
    let nested = node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(child) => innermost_node_containing(child, kind, contained),
        SyntaxElement::Token(_) => None,
    });
    nested.or((node.kind == kind).then_some(node))
}

fn innermost_node_with_span(
    node: &SyntaxNode,
    kind: SyntaxKind,
    span: TextSpan,
) -> Option<&SyntaxNode> {
    if !contains_span(node.span, span) {
        return None;
    }
    let nested = node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(child) => innermost_node_with_span(child, kind, span),
        SyntaxElement::Token(_) => None,
    });
    nested.or((node.kind == kind && node.span == span).then_some(node))
}

fn parameter_is_output_only(type_and_modifiers: &str) -> bool {
    matches!(
        type_and_modifiers.split_whitespace().next(),
        Some("out" | "inout")
    )
}

fn lexical_declaration_type_before_new(
    source: &str,
    tokens: &[Token],
    new_keyword_start: usize,
) -> Option<String> {
    let tokens = tokens
        .iter()
        .copied()
        .filter(|token| {
            token.span.end <= new_keyword_start
                && !token.kind.is_trivia()
                && token.kind != TokenKind::Eof
        })
        .collect::<Vec<_>>();
    let equal_index = tokens.len().checked_sub(1)?;
    if tokens[equal_index].kind != TokenKind::Operator(Operator::Equal) {
        return None;
    }
    let name_index = equal_index.checked_sub(1)?;
    if tokens[name_index].kind != TokenKind::Identifier {
        return None;
    }
    let structural_start_index = tokens[..name_index]
        .iter()
        .rposition(|token| {
            matches!(
                token.kind,
                TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Semicolon
            )
        })
        .map_or(0, |index| index + 1);
    let line_start = source
        .get(..new_keyword_start)?
        .rfind(['\n', '\r'])
        .map_or(0, |offset| offset + 1);
    let line_start_index = tokens[..name_index]
        .iter()
        .position(|token| token.span.start >= line_start)
        .unwrap_or(name_index);
    let type_start_index = structural_start_index.max(line_start_index);
    let type_tokens = tokens.get(type_start_index..name_index)?;
    if !tokens_form_declaration_type(type_tokens) {
        return None;
    }
    let type_span = TextSpan::new(
        type_tokens.first()?.span.start,
        type_tokens.last()?.span.end,
    );
    Some(
        source
            .get(type_span.start..type_span.end)?
            .trim()
            .to_string(),
    )
}

fn lexical_initializer_element_type_before_new(
    source: &str,
    tokens: &[Token],
    new_keyword_start: usize,
) -> Option<String> {
    let tokens = tokens
        .iter()
        .copied()
        .filter(|token| {
            token.span.end <= new_keyword_start
                && !token.kind.is_trivia()
                && token.kind != TokenKind::Eof
        })
        .collect::<Vec<_>>();
    let last_index = tokens.len().checked_sub(1)?;
    if !matches!(
        tokens[last_index].kind,
        TokenKind::LeftBrace | TokenKind::Comma
    ) {
        return None;
    }

    let mut active_braces = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LeftBrace => active_braces.push(index),
            TokenKind::RightBrace => {
                active_braces.pop()?;
            }
            _ => {}
        }
    }
    let root_index = active_braces.iter().copied().find(|open_index| {
        open_index
            .checked_sub(1)
            .and_then(|index| tokens.get(index))
            .is_some_and(|token| token.kind == TokenKind::Operator(Operator::Equal))
    })?;
    let equal_index = root_index.checked_sub(1)?;
    let name_index = equal_index.checked_sub(1)?;
    if tokens[name_index].kind != TokenKind::Identifier {
        return None;
    }

    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut comma_is_element_separator = false;
    for (index, token) in tokens.iter().enumerate().skip(root_index) {
        match token.kind {
            TokenKind::LeftBrace => brace_depth += 1,
            TokenKind::RightBrace => brace_depth = brace_depth.checked_sub(1)?,
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.checked_sub(1)?,
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.checked_sub(1)?,
            TokenKind::Comma
                if index == last_index
                    && brace_depth > 0
                    && paren_depth == 0
                    && bracket_depth == 0 =>
            {
                comma_is_element_separator = true;
            }
            _ => {}
        }
    }
    if tokens[last_index].kind == TokenKind::Comma && !comma_is_element_separator {
        return None;
    }

    let structural_start_index = tokens[..name_index]
        .iter()
        .rposition(|token| {
            matches!(
                token.kind,
                TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Semicolon
            )
        })
        .map_or(0, |index| index + 1);
    let type_tokens = tokens.get(structural_start_index..name_index)?;
    if !tokens_form_declaration_type(type_tokens) {
        return None;
    }
    let type_span = TextSpan::new(
        type_tokens.first()?.span.start,
        type_tokens.last()?.span.end,
    );
    let mut expected_type = source
        .get(type_span.start..type_span.end)?
        .trim()
        .to_string();
    for _ in 0..brace_depth {
        let stripped = strip_all_type_prefixes(&expected_type);
        let (owner, mut arguments) = generic_owner_and_args(stripped)?;
        if !matches!(owner, "array" | "set") || arguments.len() != 1 {
            return None;
        }
        expected_type = arguments.remove(0);
    }
    Some(expected_type)
}

fn tokens_form_declaration_type(tokens: &[Token]) -> bool {
    let mut tokens = tokens;
    while let Some(first) = tokens.first() {
        if matches!(
            first.kind,
            TokenKind::Keyword(
                Keyword::Ref
                    | Keyword::Autoptr
                    | Keyword::Owned
                    | Keyword::Const
                    | Keyword::Notnull
            )
        ) {
            tokens = &tokens[1..];
        } else {
            break;
        }
    }
    let Some((owner, generic_tokens)) = tokens.split_first() else {
        return false;
    };
    if !is_type_name_token(owner.kind) {
        return false;
    }
    if generic_tokens.is_empty() {
        return true;
    }
    if generic_tokens.first().map(|token| token.kind) != Some(TokenKind::Operator(Operator::Less)) {
        return false;
    }
    let mut depth = 0usize;
    for token in generic_tokens {
        match token.kind {
            TokenKind::Operator(Operator::Less) => depth += 1,
            TokenKind::Operator(Operator::Greater) => {
                let Some(next) = depth.checked_sub(1) else {
                    return false;
                };
                depth = next;
            }
            TokenKind::Operator(Operator::GreaterGreater) => {
                let Some(next) = depth.checked_sub(2) else {
                    return false;
                };
                depth = next;
            }
            TokenKind::Comma if depth > 0 => {}
            kind if depth > 0 && is_type_name_token(kind) => {}
            TokenKind::Keyword(
                Keyword::Ref | Keyword::Autoptr | Keyword::Owned | Keyword::Const,
            ) if depth > 0 => {}
            _ => return false,
        }
    }
    depth == 0
        && matches!(
            generic_tokens.last().map(|token| token.kind),
            Some(
                TokenKind::Operator(Operator::Greater)
                    | TokenKind::Operator(Operator::GreaterGreater)
            )
        )
}

fn is_type_name_token(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::Keyword(
                Keyword::Void
                    | Keyword::Bool
                    | Keyword::Int
                    | Keyword::Float
                    | Keyword::String
                    | Keyword::Vector
                    | Keyword::Typename
                    | Keyword::Auto
            )
    )
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
        .filter(|candidate| candidate_name(candidate) == name)
        .collect()
}

fn combine_candidates(
    candidates: Vec<EditorCompletionCandidate>,
) -> Vec<EditorCompletionCandidate> {
    let mut by_key = BTreeMap::<String, EditorCompletionCandidate>::new();
    let mut order = Vec::new();
    for candidate in candidates {
        let key = format!(
            "{:?}:{}:{}",
            candidate.kind,
            candidate_name(&candidate),
            candidate.signature.as_deref().unwrap_or("")
        );
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

fn candidate_belongs_to_index(
    candidate: &EditorCompletionCandidate,
    candidate_name: &str,
    index: &SymbolIndex,
) -> bool {
    index.symbol(candidate.id).is_some_and(|symbol| {
        symbol.kind == SymbolKind::Class && symbol.name.as_deref() == Some(candidate_name)
    }) && index.file(candidate.id.file_id).is_some_and(|file| {
        file.metadata.kind == candidate.source_kind
            && candidate
                .absolute_path
                .as_ref()
                .is_none_or(|path| file.metadata.absolute_path.as_ref() == Some(path))
            && candidate
                .relative_path
                .as_ref()
                .is_none_or(|path| file.metadata.relative_path.as_ref() == Some(path))
    })
}

fn candidate_name(candidate: &EditorCompletionCandidate) -> &str {
    candidate
        .name
        .as_deref()
        .unwrap_or(candidate.display.label.as_str())
}

fn containing_class_name(index: &SymbolIndex, offset: usize) -> Option<String> {
    index
        .symbols()
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Class && contains_offset(symbol.span, offset))
        .min_by_key(|symbol| symbol.span.len())
        .and_then(|symbol| symbol.name.clone())
}

fn contains_span(container: TextSpan, contained: TextSpan) -> bool {
    container.start <= contained.start && container.end >= contained.end
}

fn contains_offset(span: TextSpan, offset: usize) -> bool {
    offset >= span.start && offset <= span.end
}
