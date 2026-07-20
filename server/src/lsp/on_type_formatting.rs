use crate::lexer::{lex, Keyword, Token, TokenKind};

const MAX_ON_TYPE_SOURCE_BYTES: usize = 64 * 1024;

/// Finds the byte offset where a semicolon can be inserted after Enter.
///
/// This is intentionally a narrow typing assist rather than a formatter. It
/// only accepts a complete standalone call/member-call on the physical line
/// before the cursor. Every uncertain, malformed, or unsupported shape is a
/// no-edit result.
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
        || !is_complete_call_expression(&code_tokens)
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
    let mut saw_call = false;

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
            }
            TokenKind::LeftParen => {
                let Some(next) = consume_balanced(tokens, index, TokenKind::RightParen) else {
                    return false;
                };
                saw_call = true;
                index = next;
            }
            TokenKind::LeftBracket => {
                let Some(next) = consume_balanced(tokens, index, TokenKind::RightBracket) else {
                    return false;
                };
                index = next;
            }
            _ => return false,
        }
    }

    saw_call
}

fn is_receiver_start(kind: Option<TokenKind>) -> bool {
    matches!(
        kind,
        Some(TokenKind::Identifier) | Some(TokenKind::Keyword(Keyword::This | Keyword::Super))
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
            "return Run()\n",
            "value = Run()\n",
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
