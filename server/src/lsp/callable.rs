use crate::ast::Expression;
use crate::lexer::{lex, Operator, TextSpan, TokenKind};
use crate::syntax::{SyntaxElement, SyntaxKind, SyntaxNode};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallableSignatureParts {
    pub(crate) parameters: String,
    pub(crate) parameters_info: Vec<CallableParameter>,
    pub(crate) result: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallableParameter {
    pub(crate) raw: String,
    pub(crate) name: String,
    pub(crate) type_and_modifiers: String,
    pub(crate) default_text: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CallableArgumentContext {
    pub(crate) target: CallableTarget,
    pub(crate) argument_index: usize,
    pub(crate) active_label: Option<String>,
    pub(crate) supplied_labels: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum CallableTarget {
    Attribute { name: String },
    Call { callee_span: TextSpan },
    New { type_name: String },
}

impl CallableSignatureParts {
    pub(crate) fn required_parameters(&self) -> impl Iterator<Item = &CallableParameter> {
        self.parameters_info
            .iter()
            .filter(|parameter| parameter.default_text.is_none())
    }

    pub(crate) fn optional_parameters(&self) -> impl Iterator<Item = &CallableParameter> {
        self.parameters_info
            .iter()
            .filter(|parameter| parameter.default_text.is_some())
    }

    pub(crate) fn required_parameter_count(&self) -> usize {
        self.required_parameters().count()
    }

    pub(crate) fn optional_parameter_count(&self) -> usize {
        self.optional_parameters().count()
    }
}

pub(crate) fn callable_signature_parts(
    label: &str,
    signature: &str,
) -> Option<CallableSignatureParts> {
    let open = signature.find('(')?;
    let close = matching_close_paren(signature, open)?;
    let prefix = signature[..open].trim();
    if !prefix.ends_with(label) {
        return None;
    }
    let parameters_text = signature[open + 1..close].trim();
    let result = signature[close + 1..]
        .trim()
        .strip_prefix("->")
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| format!("-> {text}"));
    let parameters = format!("({parameters_text})");
    let parameters_info = split_callable_parameters(parameters_text)
        .into_iter()
        .filter_map(|parameter| callable_parameter(&parameter))
        .collect();

    Some(CallableSignatureParts {
        parameters,
        parameters_info,
        result,
    })
}

pub(crate) fn callable_type_owner(type_text: &str) -> Option<String> {
    let mut text = type_text.trim();
    loop {
        let stripped = ["out", "inout", "notnull", "ref", "autoptr", "const"]
            .iter()
            .find_map(|modifier| match text.strip_prefix(modifier) {
                Some(rest) if rest.chars().next().is_some_and(char::is_whitespace) => {
                    Some(rest.trim_start())
                }
                _ => None,
            });
        let Some(stripped) = stripped else {
            break;
        };
        text = stripped;
    }

    let end = text
        .char_indices()
        .find_map(|(offset, ch)| (!ch.is_ascii_alphanumeric() && ch != '_').then_some(offset))
        .unwrap_or(text.len());
    let owner = text[..end].trim();
    (!owner.is_empty()).then(|| owner.to_string())
}

pub(crate) fn callable_argument_context_at_offset(
    source: &str,
    root: &SyntaxNode,
    offset: usize,
) -> Option<CallableArgumentContext> {
    if offset > source.len() || !source.is_char_boundary(offset) {
        return None;
    }
    let position = TextSpan::new(offset, offset);
    let mut best = None;
    collect_callable_argument_context(source, root, position, &mut best);
    best
}

fn collect_callable_argument_context(
    source: &str,
    node: &SyntaxNode,
    position: TextSpan,
    best: &mut Option<CallableArgumentContext>,
) {
    if !span_contains_span(node.span, position) {
        return;
    }

    if node.kind == SyntaxKind::Attribute {
        if let Some(args) = direct_child_node(node, SyntaxKind::AttributeArgs) {
            if span_contains_span(args.span, position) {
                if let Some(name) = direct_child_name_text(source, node) {
                    replace_callable_argument_best(
                        best,
                        CallableArgumentContext {
                            target: CallableTarget::Attribute { name },
                            argument_index: argument_index_at_offset(source, args, position.start),
                            active_label: active_named_argument_label_at_offset(
                                source,
                                args,
                                position.start,
                            ),
                            supplied_labels: supplied_named_argument_labels(source, args, position),
                        },
                    );
                    return;
                }
            }
        }
    }

    if node.kind == SyntaxKind::CallExpression {
        if let Some(args) = direct_child_node(node, SyntaxKind::ArgumentList) {
            if span_contains_span(args.span, position) {
                if let Some(callee_span) = call_expression_callee_span(source, node) {
                    replace_callable_argument_best(
                        best,
                        CallableArgumentContext {
                            target: CallableTarget::Call { callee_span },
                            argument_index: argument_index_at_offset(source, args, position.start),
                            active_label: active_named_argument_label_at_offset(
                                source,
                                args,
                                position.start,
                            ),
                            supplied_labels: supplied_named_argument_labels(source, args, position),
                        },
                    );
                    return;
                }
            }
        }
    }

    if node.kind == SyntaxKind::NewExpression {
        if let Some(args) = direct_child_node(node, SyntaxKind::ArgumentList) {
            if span_contains_span(args.span, position) {
                if let Some(type_name) = new_expression_type_name(source, node) {
                    replace_callable_argument_best(
                        best,
                        CallableArgumentContext {
                            target: CallableTarget::New { type_name },
                            argument_index: argument_index_at_offset(source, args, position.start),
                            active_label: active_named_argument_label_at_offset(
                                source,
                                args,
                                position.start,
                            ),
                            supplied_labels: supplied_named_argument_labels(source, args, position),
                        },
                    );
                    return;
                }
            }
        }
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_callable_argument_context(source, child, position, best);
        }
    }
}

fn replace_callable_argument_best(
    best: &mut Option<CallableArgumentContext>,
    candidate: CallableArgumentContext,
) {
    let replace = best.as_ref().is_none_or(|current| {
        callable_target_span(&candidate).len() <= callable_target_span(current).len()
    });
    if replace {
        *best = Some(candidate);
    }
}

fn callable_target_span(context: &CallableArgumentContext) -> TextSpan {
    match &context.target {
        CallableTarget::Attribute { .. } => TextSpan::new(0, usize::MAX),
        CallableTarget::Call { callee_span } => *callee_span,
        CallableTarget::New { .. } => TextSpan::new(0, usize::MAX - 1),
    }
}

pub(crate) fn argument_index_at_offset(source: &str, args: &SyntaxNode, offset: usize) -> usize {
    let mut index = 0usize;
    let mut angle = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;

    for token in lex(&source[args.span.start..args.span.end]) {
        let span = TextSpan::new(
            args.span.start + token.span.start,
            args.span.start + token.span.end,
        );
        if span.start >= offset {
            break;
        }
        match token.kind {
            TokenKind::Operator(Operator::Less) => angle += 1,
            TokenKind::Operator(Operator::Greater) => angle = angle.saturating_sub(1),
            TokenKind::LeftParen => paren += 1,
            TokenKind::RightParen => paren = paren.saturating_sub(1),
            TokenKind::LeftBracket => bracket += 1,
            TokenKind::RightBracket => bracket = bracket.saturating_sub(1),
            TokenKind::LeftBrace => brace += 1,
            TokenKind::RightBrace => brace = brace.saturating_sub(1),
            TokenKind::Comma if angle == 0 && paren <= 1 && bracket == 0 && brace == 0 => {
                index += 1;
            }
            _ => {}
        }
    }

    index
}

fn active_named_argument_label_at_offset(
    source: &str,
    args: &SyntaxNode,
    offset: usize,
) -> Option<String> {
    let mut found = None;
    collect_active_named_argument_label(source, args, offset, &mut found);
    found
}

fn collect_active_named_argument_label(
    source: &str,
    node: &SyntaxNode,
    offset: usize,
    found: &mut Option<String>,
) {
    if !span_contains(node.span, offset) {
        return;
    }
    if node.kind == SyntaxKind::NamedArgument {
        if let Some((name, _)) = named_argument_label_text(source, node) {
            *found = Some(name);
        }
        return;
    }
    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_active_named_argument_label(source, child, offset, found);
        }
    }
}

pub(crate) fn supplied_named_argument_labels(
    source: &str,
    args: &SyntaxNode,
    current_prefix_span: TextSpan,
) -> BTreeSet<String> {
    let mut labels = BTreeSet::new();
    collect_supplied_named_argument_labels(source, args, current_prefix_span, &mut labels);
    labels
}

fn collect_supplied_named_argument_labels(
    source: &str,
    node: &SyntaxNode,
    current_prefix_span: TextSpan,
    labels: &mut BTreeSet<String>,
) {
    if node.kind == SyntaxKind::NamedArgument {
        if let Some((name, span)) = named_argument_label_text(source, node) {
            if span != current_prefix_span {
                labels.insert(name);
            }
        }
        return;
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_supplied_named_argument_labels(source, child, current_prefix_span, labels);
        }
    }
}

pub(crate) fn named_argument_label_text(
    source: &str,
    node: &SyntaxNode,
) -> Option<(String, TextSpan)> {
    let mut name = None;
    for child in &node.children {
        match child {
            SyntaxElement::Node(child) if child.kind == SyntaxKind::NameExpression => {
                name = direct_child_name_text_with_span(source, child);
            }
            SyntaxElement::Token(token) if token.kind == TokenKind::Colon => return name,
            _ => {}
        }
    }
    None
}

fn call_expression_callee_span(source: &str, node: &SyntaxNode) -> Option<TextSpan> {
    let expression = Expression::from_node(source, node)?;
    expression.callee().map(|callee| {
        callee
            .member_name()
            .map(|name| name.span)
            .unwrap_or_else(|| callee.selection_span())
    })
}

fn new_expression_type_name(source: &str, node: &SyntaxNode) -> Option<String> {
    for child in &node.children {
        match child {
            SyntaxElement::Token(token) if token.kind == TokenKind::Identifier => {
                return Some(source[token.span.start..token.span.end].to_string());
            }
            SyntaxElement::Node(child) => {
                if let Some(name) = direct_child_name_text(source, child) {
                    return Some(name);
                }
            }
            _ => {}
        }
    }
    None
}

fn direct_child_node(node: &SyntaxNode, kind: SyntaxKind) -> Option<&SyntaxNode> {
    node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(child) if child.kind == kind => Some(child.as_ref()),
        _ => None,
    })
}

fn direct_child_name_text(source: &str, node: &SyntaxNode) -> Option<String> {
    direct_child_name_text_with_span(source, node).map(|(text, _)| text)
}

fn direct_child_name_text_with_span(source: &str, node: &SyntaxNode) -> Option<(String, TextSpan)> {
    node.children.iter().find_map(|child| match child {
        SyntaxElement::Token(token) if token.kind == TokenKind::Identifier => Some((
            source[token.span.start..token.span.end].to_string(),
            token.span,
        )),
        SyntaxElement::Node(child) if child.kind == SyntaxKind::NameExpression => {
            first_identifier_token(source, child)
        }
        _ => None,
    })
}

fn first_identifier_token(source: &str, node: &SyntaxNode) -> Option<(String, TextSpan)> {
    for child in &node.children {
        match child {
            SyntaxElement::Token(token) if token.kind == TokenKind::Identifier => {
                return Some((
                    source[token.span.start..token.span.end].to_string(),
                    token.span,
                ));
            }
            SyntaxElement::Node(child) => {
                if let Some(found) = first_identifier_token(source, child) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

fn matching_close_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in text[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_callable_parameters(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut angle = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    for (offset, ch) in text.char_indices() {
        match ch {
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            '[' => bracket += 1,
            ']' => bracket = bracket.saturating_sub(1),
            ',' if angle == 0 && paren == 0 && bracket == 0 => {
                let part = text[start..offset].trim();
                if !part.is_empty() {
                    parts.push(part.to_string());
                }
                start = offset + ch.len_utf8();
            }
            _ => {}
        }
    }
    let part = text[start..].trim();
    if !part.is_empty() {
        parts.push(part.to_string());
    }
    parts
}

fn callable_parameter(parameter: &str) -> Option<CallableParameter> {
    let (before_default, default_text) = split_parameter_default(parameter);
    let before_array = before_default
        .split('[')
        .next()
        .unwrap_or(before_default)
        .trim();
    let name = before_array
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .to_string();
    if name.is_empty() {
        return None;
    }
    let type_and_modifiers = before_array
        .strip_suffix(&name)
        .unwrap_or(before_array)
        .trim()
        .to_string();
    Some(CallableParameter {
        raw: parameter.trim().to_string(),
        name,
        type_and_modifiers,
        default_text: default_text.map(str::to_string),
    })
}

fn split_parameter_default(parameter: &str) -> (&str, Option<&str>) {
    let mut angle = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    for (offset, ch) in parameter.char_indices() {
        match ch {
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            '[' => bracket += 1,
            ']' => bracket = bracket.saturating_sub(1),
            '=' if angle == 0 && paren == 0 && bracket == 0 => {
                return (
                    parameter[..offset].trim(),
                    Some(parameter[offset + ch.len_utf8()..].trim()),
                );
            }
            _ => {}
        }
    }
    (parameter.trim(), None)
}

fn span_contains(span: TextSpan, offset: usize) -> bool {
    offset >= span.start && offset <= span.end
}

fn span_contains_span(outer: TextSpan, inner: TextSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;

    #[test]
    fn callable_signature_splits_generic_parameters() {
        let parts = callable_signature_parts(
            "UseValues",
            "Example.UseValues(map<string, ref array<IEntity>> values, out int count) -> void",
        )
        .unwrap();
        assert_eq!(parts.parameters_info.len(), 2);
        assert_eq!(parts.parameters_info[0].name, "values");
        assert_eq!(
            parts.parameters_info[0].type_and_modifiers,
            "map<string, ref array<IEntity>>"
        );
        assert_eq!(parts.parameters_info[1].name, "count");
        assert_eq!(parts.parameters_info[1].type_and_modifiers, "out int");
    }

    #[test]
    fn callable_argument_context_finds_named_attribute_parameter() {
        let source = "class Example { [Attribute(defvalue: \"0\", uiwidget: UIWidgets.Flags)] int m_Value; }";
        let parse = parse_source(source);
        let offset = source.find("uiwidget").unwrap() + "uiwidget".len();
        let context = callable_argument_context_at_offset(source, &parse.root, offset).unwrap();
        assert_eq!(context.argument_index, 1);
        assert_eq!(context.active_label.as_deref(), Some("uiwidget"));
    }

    #[test]
    fn callable_argument_context_ignores_nested_commas() {
        let source = "class Example { void Run() { SendToEveryone(ENotification.PLAYER_JOINED, string.Format(\"%1, %2\", a, b), 3); } }";
        let parse = parse_source(source);
        let offset = source.rfind(", 3").unwrap() + 3;
        let context = callable_argument_context_at_offset(source, &parse.root, offset).unwrap();
        assert_eq!(context.argument_index, 2);
    }
}
