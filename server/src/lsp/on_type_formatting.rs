use crate::lexer::{lex, Keyword, Operator, Token, TokenKind};

const MAX_ON_TYPE_SOURCE_BYTES: usize = 64 * 1024;

/// Finds the byte offset where a semicolon can be inserted after Enter.
///
/// This is intentionally a narrow typing assist rather than a formatter. It
/// only accepts a complete standalone call/member-call, typed variable
/// declaration, or value-return statement on the physical line before the
/// cursor. Every uncertain, malformed, or unsupported shape is a no-edit
/// result.
pub(super) fn semicolon_insertion_offset(source: &str, cursor: usize) -> Option<usize> {
    if source.len() > MAX_ON_TYPE_SOURCE_BYTES || cursor > source.len() {
        return None;
    }

    let current_line_start = line_start_before(source, cursor)?;
    if !source[current_line_start..cursor]
        .chars()
        .all(char::is_whitespace)
    {
        return None;
    }
    let previous_line_end = trim_line_ending(source, current_line_start);
    let previous_line_start = source[..previous_line_end]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let line = &source[previous_line_start..previous_line_end];

    // A bounded full-prefix lex is only used to reject a line inside an
    // unterminated block comment/string. The bound is deliberately small and
    // a larger document safely receives no edit.
    let prefix_tokens = lex(&source[..cursor]);
    if prefix_tokens.iter().any(|token| {
        matches!(
            token.kind,
            TokenKind::UnterminatedBlockComment | TokenKind::UnterminatedString
        )
    }) {
        return None;
    }

    let tokens = lex(line);
    let comment_start = tokens
        .iter()
        .find(|token| {
            matches!(
                token.kind,
                TokenKind::LineComment | TokenKind::DocLineComment
            )
        })
        .map(|token| token.span.start);
    let code_end = comment_start.unwrap_or(line.len());
    let insertion_in_line = line[..code_end].trim_end_matches(char::is_whitespace).len();
    if insertion_in_line == 0 {
        return None;
    }

    let code_tokens = tokens
        .iter()
        .copied()
        .filter(|token| token.span.start < insertion_in_line && !token.kind.is_trivia())
        .collect::<Vec<_>>();
    if code_tokens.is_empty()
        || code_tokens
            .iter()
            .any(|token| token.kind.is_error() || token.kind == TokenKind::Semicolon)
        || !(is_complete_call_expression(&code_tokens)
            || is_complete_variable_declaration(&code_tokens)
            || is_complete_value_return_statement(&code_tokens))
    {
        return None;
    }

    // A call-shaped callable declaration/constructor header is not a
    // statement. If the next significant token begins a body, leave it alone.
    let following_tokens = lex(&source[cursor..]);
    if following_tokens
        .iter()
        .find(|token| !token.kind.is_trivia() && token.kind != TokenKind::Eof)
        .is_some_and(|token| token.kind == TokenKind::LeftBrace)
    {
        return None;
    }

    Some(previous_line_start + insertion_in_line)
}

fn line_start_before(source: &str, cursor: usize) -> Option<usize> {
    let before_cursor = &source[..cursor];
    before_cursor.rfind('\n').map(|newline| newline + 1)
}

fn trim_line_ending(source: &str, line_start: usize) -> usize {
    let mut end = line_start.saturating_sub(1);
    if end > 0 && source.as_bytes().get(end - 1) == Some(&b'\r') {
        end = end.saturating_sub(1);
    }
    end
}

fn is_complete_call_expression(tokens: &[Token]) -> bool {
    let mut index = 0;
    if !is_receiver_start(tokens.get(index).map(|token| token.kind)) {
        return false;
    }
    index += 1;
    let mut ends_with_call = false;

    while index < tokens.len() {
        match tokens[index].kind {
            TokenKind::Dot => {
                index += 1;
                if !matches!(
                    tokens.get(index).map(|token| token.kind),
                    Some(TokenKind::Identifier)
                ) {
                    return false;
                }
                index += 1;
                ends_with_call = false;
            }
            TokenKind::LeftParen => {
                let Some(next) = consume_balanced(tokens, index, TokenKind::RightParen) else {
                    return false;
                };
                index = next;
                ends_with_call = true;
            }
            TokenKind::LeftBracket => {
                let Some(next) = consume_balanced(tokens, index, TokenKind::RightBracket) else {
                    return false;
                };
                index = next;
                ends_with_call = false;
            }
            _ => return false,
        }
    }

    ends_with_call
}

fn is_receiver_start(kind: Option<TokenKind>) -> bool {
    matches!(
        kind,
        Some(TokenKind::Identifier) | Some(TokenKind::Keyword(Keyword::This | Keyword::Super))
    )
}

fn is_complete_value_return_statement(tokens: &[Token]) -> bool {
    if tokens.first().map(|token| token.kind) != Some(TokenKind::Keyword(Keyword::Return)) {
        return false;
    }
    let Some(end) = consume_initializer(tokens, 1) else {
        return false;
    };
    end == tokens.len() && is_complete_value_expression(&tokens[1..])
}

/// Return values are deliberately stricter than a merely balanced token
/// sequence. This bounded lexical check rejects control keywords and adjacent
/// primary values (for example `owner GetOwner()`), which are not a complete
/// Enforce expression and therefore must not receive an automatic edit.
fn is_complete_value_expression(tokens: &[Token]) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };
    if !can_start_value_expression(first.kind) || !has_only_complete_new_expressions(tokens) {
        return false;
    }
    for (index, token) in tokens.iter().enumerate() {
        let kind = token.kind;
        if !(is_value_expression_token(kind) || is_new_type_keyword(tokens, index)) {
            return false;
        }
        if let Some(next) = tokens.get(index + 1) {
            if can_end_value_expression(kind)
                && can_start_value_expression(next.kind)
                && next.kind != TokenKind::LeftParen
            {
                return false;
            }
        }
    }
    true
}

/// Primitive keywords can only participate in a returned expression as type
/// arguments of a `new Type<...>(...)` construction. Scanning back to the
/// construction keyword is bounded by the already-small physical line.
fn is_new_type_keyword(tokens: &[Token], index: usize) -> bool {
    if !matches!(
        tokens[index].kind,
        TokenKind::Keyword(
            Keyword::Int
                | Keyword::Float
                | Keyword::Bool
                | Keyword::String
                | Keyword::Vector
                | Keyword::Typename
        )
    ) {
        return false;
    }
    for token in tokens[..index].iter().rev() {
        match token.kind {
            TokenKind::Keyword(Keyword::New) => return true,
            TokenKind::Operator(Operator::Less | Operator::Greater | Operator::GreaterGreater) => {}
            TokenKind::Operator(_)
            | TokenKind::LeftParen
            | TokenKind::RightParen
            | TokenKind::LeftBracket
            | TokenKind::RightBracket
            | TokenKind::LeftBrace
            | TokenKind::RightBrace => return false,
            _ => {}
        }
    }
    false
}

/// `new Type` is syntactically unfinished until its constructor argument list
/// closes.  Keep this explicit instead of relying on generic delimiter balance
/// so the typing assist remains fail-closed around construction expressions.
fn has_only_complete_new_expressions(tokens: &[Token]) -> bool {
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Keyword(Keyword::New) {
            index += 1;
            continue;
        }
        index += 1;
        if !matches!(
            tokens.get(index).map(|token| token.kind),
            Some(TokenKind::Identifier)
        ) {
            return false;
        }
        index += 1;
        let Some(next) = consume_generic_arguments(tokens, index) else {
            return false;
        };
        index = next;
        if tokens.get(index).map(|token| token.kind) != Some(TokenKind::LeftParen) {
            return false;
        }
        let Some(next) = consume_balanced(tokens, index, TokenKind::RightParen) else {
            return false;
        };
        index = next;
    }
    true
}

fn is_value_expression_token(kind: TokenKind) -> bool {
    !matches!(kind, TokenKind::Keyword(keyword) if !matches!(
        keyword,
        Keyword::This | Keyword::Super | Keyword::True | Keyword::False | Keyword::Null | Keyword::New
    ))
}

fn can_start_value_expression(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::Number
            | TokenKind::String
            | TokenKind::LeftParen
            | TokenKind::LeftBrace
            | TokenKind::Keyword(
                Keyword::This
                    | Keyword::Super
                    | Keyword::True
                    | Keyword::False
                    | Keyword::Null
                    | Keyword::New
            )
            | TokenKind::Operator(
                Operator::Plus
                    | Operator::Minus
                    | Operator::Bang
                    | Operator::Tilde
                    | Operator::PlusPlus
                    | Operator::MinusMinus
            )
    )
}

fn can_end_value_expression(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::Number
            | TokenKind::String
            | TokenKind::RightParen
            | TokenKind::RightBracket
            | TokenKind::RightBrace
            | TokenKind::Keyword(
                Keyword::This | Keyword::Super | Keyword::True | Keyword::False | Keyword::Null
            )
    )
}

/// Recognizes a complete typed variable declaration without resolving its
/// type. A `Type name` shape is unambiguously a declaration in statement
/// position; callable headers and controls cannot satisfy this grammar.
fn is_complete_variable_declaration(tokens: &[Token]) -> bool {
    let mut index = 0;
    while matches!(
        tokens.get(index).map(|token| token.kind),
        Some(TokenKind::Keyword(
            Keyword::Const | Keyword::Ref | Keyword::Notnull | Keyword::Autoptr | Keyword::Owned
        ))
    ) {
        index += 1;
    }
    if !is_local_type_start(tokens.get(index).map(|token| token.kind)) {
        return false;
    }
    index += 1;
    let Some(next) = consume_generic_arguments(tokens, index) else {
        return false;
    };
    index = next;

    loop {
        if !matches!(
            tokens.get(index).map(|token| token.kind),
            Some(TokenKind::Identifier)
        ) {
            return false;
        }
        index += 1;
        while matches!(
            tokens.get(index).map(|token| token.kind),
            Some(TokenKind::LeftBracket)
        ) {
            let Some(next) = consume_balanced(tokens, index, TokenKind::RightBracket) else {
                return false;
            };
            index = next;
        }
        if matches!(
            tokens.get(index).map(|token| token.kind),
            Some(TokenKind::Operator(Operator::Equal))
        ) {
            index += 1;
            let Some(next) = consume_initializer(tokens, index) else {
                return false;
            };
            index = next;
        }
        if index == tokens.len() {
            return true;
        }
        if tokens.get(index).map(|token| token.kind) != Some(TokenKind::Comma) {
            return false;
        }
        index += 1;
    }
}

fn is_local_type_start(kind: Option<TokenKind>) -> bool {
    matches!(
        kind,
        Some(TokenKind::Identifier)
            | Some(TokenKind::Keyword(
                Keyword::Int
                    | Keyword::Float
                    | Keyword::Bool
                    | Keyword::String
                    | Keyword::Vector
                    | Keyword::Typename
                    | Keyword::Auto
            ))
    )
}

fn consume_generic_arguments(tokens: &[Token], mut index: usize) -> Option<usize> {
    if tokens.get(index).map(|token| token.kind) != Some(TokenKind::Operator(Operator::Less)) {
        return Some(index);
    }
    let mut depth = 0usize;
    while let Some(token) = tokens.get(index) {
        match token.kind {
            TokenKind::Operator(Operator::Less) => depth += 1,
            TokenKind::Operator(Operator::Greater) => depth = depth.checked_sub(1)?,
            TokenKind::Operator(Operator::GreaterGreater) => depth = depth.checked_sub(2)?,
            TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::LeftBrace => return None,
            _ => {}
        }
        index += 1;
        if depth == 0 {
            return Some(index);
        }
    }
    None
}

fn consume_initializer(tokens: &[Token], mut index: usize) -> Option<usize> {
    let start = index;
    let mut closes = Vec::new();
    while let Some(token) = tokens.get(index) {
        match token.kind {
            TokenKind::LeftParen => closes.push(TokenKind::RightParen),
            TokenKind::LeftBracket => closes.push(TokenKind::RightBracket),
            TokenKind::LeftBrace => closes.push(TokenKind::RightBrace),
            TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                if token.kind != closes.pop()? {
                    return None;
                }
            }
            TokenKind::Comma if closes.is_empty() => break,
            _ => {}
        }
        index += 1;
    }
    if index == start
        || !closes.is_empty()
        || ends_in_incomplete_expression(tokens[start..index].last()?.kind)
    {
        return None;
    }
    Some(index)
}

fn ends_in_incomplete_expression(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Dot | TokenKind::Question | TokenKind::Colon | TokenKind::Operator(_)
    )
}

fn consume_balanced(tokens: &[Token], start: usize, close: TokenKind) -> Option<usize> {
    let mut stack = vec![close];
    let mut index = start + 1;
    while let Some(token) = tokens.get(index) {
        match token.kind {
            TokenKind::LeftParen => stack.push(TokenKind::RightParen),
            TokenKind::LeftBracket => stack.push(TokenKind::RightBracket),
            TokenKind::LeftBrace | TokenKind::RightBrace => return None,
            kind if kind == *stack.last()? => {
                stack.pop();
                if stack.is_empty() {
                    return Some(index + 1);
                }
            }
            TokenKind::RightParen | TokenKind::RightBracket => return None,
            _ => {}
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::semicolon_insertion_offset;

    fn insertion(source: &str) -> Option<usize> {
        semicolon_insertion_offset(source, source.len())
    }

    #[test]
    fn inserts_after_complete_calls_and_member_calls() {
        let source = "\tGetGame().GetPlayerController().GetControlledEntity()\n\t";
        assert_eq!(insertion(source), Some(source.find('\n').unwrap()));
        assert_eq!(
            insertion("Run(value, Other())\n"),
            Some("Run(value, Other())".len())
        );
    }

    #[test]
    fn inserts_after_complete_variable_declarations() {
        for source in [
            "GRAY_TEST2 test44\n",
            "int testnum = 44\n",
            "ref array<ref SCR_EntityBudgetValue> budgetCosts\n",
            "vector debugPoints[4]\n",
            "int first = 1, second = Other()\n",
            "SCR_OutfitFactionData currentData = outfitDataArray[i]\n",
            "int number = 44 // keep this\n",
        ] {
            let expected = source
                .find(" //")
                .unwrap_or_else(|| source.find('\n').unwrap());
            assert_eq!(insertion(source), Some(expected), "{source:?}");
        }
    }

    #[test]
    fn inserts_after_complete_value_return_statements() {
        for source in [
            "return owner\n",
            "return GetOwner()\n",
            "return new GRAY_TEST2()\n",
            "return new array<int>()\n",
            "return GetOwner().m_Name\n",
            "return owner == GetOwner() ? owner : null\n",
            "return owner // keep this\n",
        ] {
            let expected = source
                .find(" //")
                .unwrap_or_else(|| source.find('\n').unwrap());
            assert_eq!(insertion(source), Some(expected), "{source:?}");
        }
    }

    #[test]
    fn inserts_before_a_trailing_line_comment() {
        let source = "Run() // keep this\n";
        assert_eq!(insertion(source), Some("Run()".len()));
    }

    #[test]
    fn rejects_non_statements_and_ambiguous_shapes() {
        for source in [
            "if (owner == GetOwner())\n",
            "while (Running())\n",
            "[RplRpc()]\n",
            "void Run()\n{\n",
            "Example()\n{\n",
            "Run(\n",
            "GetGame().\n",
            "Run();\n",
            "return\n",
            "return owner.\n",
            "return GetOwner(\n",
            "return owner +\n",
            "return if\n",
            "return owner GetOwner()\n",
            "return owner new GRAY_TEST2()\n",
            "return owner < int\n",
            "return new\n",
            "return new GRAY_TEST2\n",
            "Run().member\n",
            "Run()[0]\n",
            "Run().member[0]\n",
            "value = Run()\n",
            "int\n",
            "GRAY_TEST2\n",
            "int testnum =\n",
            "int testnum = Other(\n",
            "void Run\n",
            "int Run()\n",
            "int first = 1,\n",
            "// Run()\n",
            "\"Run()\"\n",
            "#ifdef Run\n",
        ] {
            assert_eq!(insertion(source), None, "{source:?}");
        }
    }

    #[test]
    fn rejects_unterminated_lexical_context_and_large_documents() {
        assert_eq!(insertion("/*\nRun()\n"), None);
        assert_eq!(insertion("\"unterminated\nRun()\n"), None);
        let source = format!("{}Run()\n", " ".repeat(64 * 1024));
        assert_eq!(insertion(&source), None);
    }

    #[test]
    fn preserves_crlf_and_utf16_positions_by_returning_byte_offset() {
        let source = "😀Run()\r\n\t";
        assert_eq!(insertion(source), None);
        let source = "Run(\"😀\")\r\n\t";
        assert_eq!(insertion(source), Some("Run(\"😀\")".len()));
    }
}
