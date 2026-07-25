use crate::lexer::{lex, Keyword, Operator, TextSpan, TokenKind};

const MAX_COLLECTION_DECLARATION_SOURCE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CollectionDeclaration {
    pub type_span: TextSpan,
    pub name_span: TextSpan,
    pub collection: &'static str,
}

/// Proves the completed, single builtin-collection declaration immediately
/// before a caret. Both completion and input routing consume this language
/// context; neither owns the classification.
pub(super) fn collection_declaration_before_cursor(
    source: &str,
    cursor: usize,
    allow_trailing_whitespace: bool,
) -> Option<CollectionDeclaration> {
    if source.len() > MAX_COLLECTION_DECLARATION_SOURCE_BYTES || cursor > source.len() {
        return None;
    }
    let tokens = lex(source)
        .into_iter()
        .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    let name_index = tokens
        .iter()
        .rposition(|token| token.span.end <= cursor && token.kind == TokenKind::Identifier)?;
    let name = tokens[name_index];
    let trailing = source.get(name.span.end..cursor)?;
    if !((allow_trailing_whitespace
        && !trailing.is_empty()
        && trailing.chars().all(char::is_whitespace))
        || (!allow_trailing_whitespace && trailing.is_empty()))
    {
        return None;
    }
    let close_index = name_index.checked_sub(1)?;
    let closing_angle_count = generic_closing_angle_count(tokens[close_index].kind)?;
    if closing_angle_count == 0 {
        return None;
    }
    let mut depth = closing_angle_count;
    let mut open_index = None;
    for index in (0..close_index).rev() {
        match tokens[index].kind {
            kind if generic_closing_angle_count(kind).is_some() => {
                depth += generic_closing_angle_count(kind)?;
            }
            TokenKind::Operator(Operator::Less) => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    open_index = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let open_index = open_index?;
    let collection_index = open_index.checked_sub(1)?;
    if tokens[collection_index].kind != TokenKind::Identifier {
        return None;
    }
    let collection =
        source.get(tokens[collection_index].span.start..tokens[collection_index].span.end)?;
    if !matches!(collection, "array" | "set" | "map")
        || open_index + 1 == close_index
        || tokens[close_index - 1].kind == TokenKind::Comma
    {
        return None;
    }
    let mut nested_depth = 0usize;
    let mut generic_argument_count = 1usize;
    for token in &tokens[open_index + 1..close_index] {
        match token.kind {
            TokenKind::Operator(Operator::Less) => nested_depth += 1,
            kind if generic_closing_angle_count(kind).is_some() => {
                nested_depth = nested_depth.checked_sub(generic_closing_angle_count(kind)?)?;
            }
            TokenKind::Comma if nested_depth == 0 => generic_argument_count += 1,
            _ => {}
        }
    }
    if generic_argument_count != if collection == "map" { 2 } else { 1 } {
        return None;
    }
    let mut boundary_index = collection_index;
    while let Some(previous) = boundary_index.checked_sub(1).map(|index| tokens[index]) {
        if matches!(
            previous.kind,
            TokenKind::Keyword(
                Keyword::Private
                    | Keyword::Protected
                    | Keyword::Static
                    | Keyword::Const
                    | Keyword::Ref
                    | Keyword::Autoptr
            )
        ) {
            boundary_index -= 1;
            continue;
        }
        if !matches!(previous.kind, TokenKind::LeftBrace | TokenKind::Semicolon) {
            return None;
        }
        break;
    }
    if boundary_index == 0
        || tokens.iter().any(|token| {
            token.span.start >= cursor
                && token.kind != TokenKind::RightBrace
                && !source[cursor..token.span.start].contains(['\r', '\n'])
        })
    {
        return None;
    }
    Some(CollectionDeclaration {
        type_span: TextSpan::new(
            tokens[collection_index].span.start,
            tokens[close_index].span.end,
        ),
        name_span: name.span,
        collection: match collection {
            "array" => "array",
            "set" => "set",
            "map" => "map",
            _ => unreachable!(),
        },
    })
}

/// The lexer preserves shift operators, so nested generic closers such as
/// `array<array<int>>` arrive as one `>>` token. In a verified type span they
/// close two generic levels rather than denoting a shift expression.
fn generic_closing_angle_count(kind: TokenKind) -> Option<usize> {
    match kind {
        TokenKind::Operator(Operator::Greater) => Some(1),
        TokenKind::Operator(Operator::GreaterGreater) => Some(2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::collection_declaration_before_cursor;

    #[test]
    fn recognizes_only_complete_field_and_local_collection_declaration_tails() {
        let field = "class Example\n{\n\tprotected ref map<string, ref Widget> m_Widgets\n}";
        let declaration = collection_declaration_before_cursor(
            field,
            field.find("m_Widgets").unwrap() + "m_Widgets".len(),
            false,
        )
        .expect("expected field declaration");
        assert_eq!(declaration.collection, "map");
        let local = "class Example\n{\n\tvoid Run()\n\t{\n\t\tarray<int> values\n\t}\n}";
        assert!(collection_declaration_before_cursor(
            local,
            local.find("values").unwrap() + "values".len(),
            false
        )
        .is_some());
        let parameter = "class Example { void Run(array<int> values ) {} }";
        assert!(collection_declaration_before_cursor(
            parameter,
            parameter.find("values").unwrap() + "values".len(),
            false
        )
        .is_none());
        let existing = "class Example { array<int> values; }";
        assert!(collection_declaration_before_cursor(
            existing,
            existing.find("values").unwrap() + "values".len(),
            false
        )
        .is_none());
        let one_line = "class Example { array<int> values }";
        assert!(collection_declaration_before_cursor(
            one_line,
            one_line.find("values").unwrap() + "values".len(),
            false
        )
        .is_some());
        let malformed_map = "class Example { map<int> values }";
        assert!(collection_declaration_before_cursor(
            malformed_map,
            malformed_map.find("values").unwrap() + "values".len(),
            false
        )
        .is_none());
    }

    #[test]
    fn recognizes_nested_builtin_collections_with_compact_closing_angles() {
        for (source, collection, type_text) in [
            (
                "class Example { array<array<ref Widget>> values }",
                "array",
                "array<array<ref Widget>>",
            ),
            (
                "class Example { set<array<int>> values }",
                "set",
                "set<array<int>>",
            ),
            (
                "class Example { map<string, array<array<int>>> values }",
                "map",
                "map<string, array<array<int>>>",
            ),
        ] {
            let declaration = collection_declaration_before_cursor(
                source,
                source.find("values").unwrap() + "values".len(),
                false,
            )
            .expect("expected nested collection declaration");
            assert_eq!(declaration.collection, collection);
            assert_eq!(
                &source[declaration.type_span.start..declaration.type_span.end],
                type_text
            );
        }
    }
}
