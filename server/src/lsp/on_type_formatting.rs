use crate::lexer::{lex, Keyword, Operator, TextSpan, Token, TokenKind};

const MAX_ON_TYPE_SOURCE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BlockCommentPairPlan {
    pub span: TextSpan,
    pub replacement: String,
    pub selection_character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IncompleteIfHeaderPlan {
    pub span: TextSpan,
    pub replacement: String,
    pub selection_character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ControlBodyOutdentPlan {
    pub span: TextSpan,
    pub replacement: String,
    pub selection_character: u32,
}

/// Restores the enclosing indentation after one complete, unbraced `if` body
/// statement. The editor owns initial body indentation; this only corrects
/// the next blank line once the two physical lines prove the scope ended.
pub(super) fn unbraced_if_body_outdent_plan(
    source: &str,
    cursor: usize,
    tab_size: usize,
    insert_spaces: bool,
    accept_single_unit_tab_indent: bool,
) -> Option<ControlBodyOutdentPlan> {
    if source.len() > MAX_ON_TYPE_SOURCE_BYTES || cursor > source.len() {
        return None;
    }
    let current_line_start = line_start_before(source, cursor)?;
    let current_indent = &source[current_line_start..cursor];
    if !current_indent.chars().all(char::is_whitespace) {
        return None;
    }
    let body_line_end = trim_line_ending(source, current_line_start);
    let body_line_start = source[..body_line_end]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let header_line_end = trim_line_ending(source, body_line_start);
    let header_line_start = source[..header_line_end]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let header = &source[header_line_start..header_line_end];
    let body = &source[body_line_start..body_line_end];
    let header_indent_end = header.find(|character: char| !character.is_whitespace())?;
    let header_indent = &header[..header_indent_end];
    let unit = if insert_spaces {
        " ".repeat(tab_size.clamp(1, 16))
    } else {
        "\t".to_string()
    };
    let body_indent = format!("{header_indent}{unit}");
    // A trusted Tab trigger on an otherwise empty line starts from column zero.
    // It therefore produces one indentation unit, not the `if` body's indent.
    // Accept that exact Tab-triggered shape too, then restore the header indent.
    // Enter keeps the stricter body-indent-only contract.
    let is_single_unit_tab_indent = accept_single_unit_tab_indent && current_indent == unit;
    if (current_indent != body_indent && !is_single_unit_tab_indent)
        || !body.starts_with(&body_indent)
    {
        return None;
    }
    let header_tokens = significant_tokens(header)?;
    let body_tokens = significant_tokens(&body[body_indent.len()..])?;
    if !is_complete_unbraced_if_header(&header_tokens)
        || !is_complete_one_line_body_statement(&body_tokens)
    {
        return None;
    }
    Some(ControlBodyOutdentPlan {
        span: TextSpan::new(current_line_start, cursor),
        replacement: header_indent.to_string(),
        selection_character: header_indent
            .chars()
            .map(|character| character.len_utf16() as u32)
            .sum(),
    })
}

fn significant_tokens(source: &str) -> Option<Vec<Token>> {
    let tokens = lex(source);
    if tokens.iter().any(|token| token.kind.is_error()) {
        return None;
    }
    Some(
        tokens
            .into_iter()
            .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Eof)
            .collect(),
    )
}

fn is_complete_unbraced_if_header(tokens: &[Token]) -> bool {
    let condition_start = match tokens {
        [Token {
            kind: TokenKind::Keyword(Keyword::If),
            ..
        }, Token {
            kind: TokenKind::LeftParen,
            ..
        }, ..] => 2,
        [Token {
            kind: TokenKind::Keyword(Keyword::Else),
            ..
        }, Token {
            kind: TokenKind::Keyword(Keyword::If),
            ..
        }, Token {
            kind: TokenKind::LeftParen,
            ..
        }, ..] => 3,
        _ => return false,
    };
    let condition = &tokens[condition_start..];
    has_completed_if_paren(condition)
        && is_complete_value_expression(&condition[..condition.len().saturating_sub(1)])
}

fn is_complete_one_line_body_statement(tokens: &[Token]) -> bool {
    let statement = if tokens.last().map(|token| token.kind) == Some(TokenKind::Semicolon) {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    };
    !statement.is_empty()
        && (is_complete_call_expression(statement)
            || is_complete_variable_declaration(statement)
            || is_complete_value_return_statement(statement))
}

/// Completes a single-line unbraced `if` header when Enter split its still
/// unfinished condition.  This is intentionally smaller than a formatter:
/// every unsupported or recovered shape leaves the editor's native Enter
/// result alone.
pub(super) fn incomplete_if_header_enter_plan(
    source: &str,
    cursor: usize,
    tab_size: usize,
    insert_spaces: bool,
) -> Option<IncompleteIfHeaderPlan> {
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
    let mut current_line_end = source[cursor..]
        .find('\n')
        .map_or(source.len(), |offset| cursor + offset);
    if current_line_end > cursor && source.as_bytes().get(current_line_end - 1) == Some(&b'\r') {
        current_line_end -= 1;
    }
    let header_prefix = &source[previous_line_start..previous_line_end];
    let condition_suffix = &source[cursor..current_line_end];
    let header = format!("{header_prefix}{condition_suffix}");
    let indent_end = header.find(|character: char| !character.is_whitespace())?;
    let indent = &header[..indent_end];
    let header_tokens = lex(&header);
    if header_tokens.iter().any(|token| {
        token.kind.is_error()
            || matches!(
                token.kind,
                TokenKind::LineComment
                    | TokenKind::DocLineComment
                    | TokenKind::BlockComment
                    | TokenKind::DocBlockComment
            )
    }) {
        return None;
    }
    let tokens = header_tokens
        .iter()
        .copied()
        .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    let tokens = tokens.as_slice();
    if tokens.first().map(|token| token.kind) != Some(TokenKind::Keyword(Keyword::If))
        || tokens.get(1).map(|token| token.kind) != Some(TokenKind::LeftParen)
        || tokens.iter().any(|token| {
            matches!(
                token.kind,
                TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Semicolon
            )
        })
    {
        return None;
    }
    let prefix_tokens = lex(header_prefix)
        .into_iter()
        .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    if prefix_tokens.first().map(|token| token.kind) != Some(TokenKind::Keyword(Keyword::If))
        || prefix_tokens.get(1).map(|token| token.kind) != Some(TokenKind::LeftParen)
        || !has_only_open_if_paren(&prefix_tokens[2..])
    {
        return None;
    }
    let (condition, closing) = if has_only_open_if_paren(&tokens[2..]) {
        (&tokens[2..], ")")
    } else if has_completed_if_paren(&tokens[2..]) {
        (&tokens[2..tokens.len() - 1], "")
    } else {
        return None;
    };
    if condition.is_empty()
        || !is_complete_value_expression(condition)
        || !condition
            .last()
            .is_some_and(|token| can_end_value_expression(token.kind))
    {
        return None;
    }

    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let unit = if insert_spaces {
        " ".repeat(tab_size.clamp(1, 16))
    } else {
        "\t".to_string()
    };
    let body_indent = format!("{indent}{unit}");
    Some(IncompleteIfHeaderPlan {
        span: TextSpan::new(previous_line_start, current_line_end),
        replacement: format!("{header}{closing}{newline}{body_indent}"),
        selection_character: body_indent
            .chars()
            .map(|character| character.len_utf16() as u32)
            .sum(),
    })
}

fn has_only_open_if_paren(tokens: &[Token]) -> bool {
    let mut paren_depth = 1usize;
    let mut bracket_depth = 0usize;
    for token in tokens {
        match token.kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => {
                let Some(depth) = paren_depth.checked_sub(1) else {
                    return false;
                };
                paren_depth = depth;
            }
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => {
                let Some(depth) = bracket_depth.checked_sub(1) else {
                    return false;
                };
                bracket_depth = depth;
            }
            _ => {}
        }
    }
    paren_depth == 1 && bracket_depth == 0
}

fn has_completed_if_paren(tokens: &[Token]) -> bool {
    if tokens.last().map(|token| token.kind) != Some(TokenKind::RightParen) {
        return false;
    }
    let mut paren_depth = 1usize;
    let mut bracket_depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => {
                let Some(depth) = paren_depth.checked_sub(1) else {
                    return false;
                };
                paren_depth = depth;
                if paren_depth == 0 && index + 1 != tokens.len() {
                    return false;
                }
            }
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => {
                let Some(depth) = bracket_depth.checked_sub(1) else {
                    return false;
                };
                bracket_depth = depth;
            }
            _ => {}
        }
    }
    paren_depth == 0 && bracket_depth == 0
}

/// Expands the exact empty native `/**/` pair into a standalone multiline
/// block comment. The client only sends this after VS Code reports its native
/// `**/` auto-close edit, so ordinary `*` typing never reaches this classifier.
pub(super) fn block_comment_pair_plan(
    source: &str,
    cursor: usize,
    tab_size: usize,
    insert_spaces: bool,
) -> Option<BlockCommentPairPlan> {
    if source.len() > MAX_ON_TYPE_SOURCE_BYTES || cursor < 2 || cursor > source.len() {
        return None;
    }
    let comment = lex(source).into_iter().find(|token| {
        token.kind == TokenKind::BlockComment
            && token.span.start + 2 == cursor
            && token.span.end == cursor + 2
            && source.get(token.span.start..token.span.end) == Some("/**/")
    })?;
    let line_start = source[..comment.span.start]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let line_end = source[comment.span.end..]
        .find('\n')
        .map_or(source.len(), |offset| comment.span.end + offset);
    let indent = &source[line_start..comment.span.start];
    if !indent.chars().all(char::is_whitespace)
        || !source[comment.span.end..line_end].trim().is_empty()
    {
        return None;
    }

    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let unit = if insert_spaces {
        " ".repeat(tab_size.clamp(1, 16))
    } else {
        "\t".to_string()
    };
    let inner_indent = format!("{indent}{unit}");
    let replacement = format!("/*{newline}{inner_indent}{newline}{indent}*/");
    Some(BlockCommentPairPlan {
        span: comment.span,
        replacement,
        selection_character: inner_indent
            .chars()
            .map(|character| character.len_utf16() as u32)
            .sum(),
    })
}

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
    // A bare return is already a complete statement; `;` is the only
    // syntactically reasonable completion after Enter.
    if tokens.len() == 1 {
        return true;
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
    use super::{
        block_comment_pair_plan, incomplete_if_header_enter_plan, semicolon_insertion_offset,
        unbraced_if_body_outdent_plan,
    };

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
            "return\n",
            "return GetOwner()\n",
            "return new GRAY_TEST2()\n",
            "return new array<int>()\n",
            "return GetOwner().m_Name\n",
            "return owner == GetOwner() ? owner : null\n",
            "return owner // keep this\n",
            "return // keep this\n",
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
    fn completes_an_unfinished_if_header_and_places_the_body_caret() {
        let source = "\tif (owner == GetOwner()\n\t";
        let plan = incomplete_if_header_enter_plan(source, source.len(), 4, false).unwrap();
        assert_eq!(plan.span.start, 0);
        assert_eq!(plan.span.end, source.len());
        assert_eq!(plan.replacement, "\tif (owner == GetOwner())\n\t\t");
        assert_eq!(plan.selection_character, 2);

        let split_identifier = "if (own\n\ter";
        let cursor = "if (own\n\t".len();
        let plan = incomplete_if_header_enter_plan(split_identifier, cursor, 2, true).unwrap();
        assert_eq!(plan.replacement, "if (owner)\n  ");
        assert_eq!(plan.selection_character, 2);

        let auto_closed = "\tif (GetGame()\n\t)";
        let cursor = "\tif (GetGame()\n\t".len();
        let plan = incomplete_if_header_enter_plan(auto_closed, cursor, 4, false).unwrap();
        assert_eq!(plan.replacement, "\tif (GetGame())\n\t\t");
        assert_eq!(plan.selection_character, 2);
    }

    #[test]
    fn rejects_completed_or_ambiguous_if_header_enters() {
        for source in [
            "if (owner)\n\t",
            "if (owner &&\n\t",
            "if (owner // note\n\t",
            "if (\"owner\n\t",
            "[Attribute(\n\t",
            "while (owner\n\t",
            "if (owner {\n\t",
            "if (owners[\n\t",
        ] {
            assert!(
                incomplete_if_header_enter_plan(source, source.len(), 4, false).is_none(),
                "{source:?}"
            );
        }
    }

    #[test]
    fn incomplete_if_header_plan_preserves_crlf() {
        let source = "\tif (Ready()\r\n\t";
        let plan = incomplete_if_header_enter_plan(source, source.len(), 4, false).unwrap();
        assert_eq!(plan.replacement, "\tif (Ready())\r\n\t\t");
    }

    #[test]
    fn outdents_after_one_complete_unbraced_if_body_statement() {
        for source in [
            "\tif (owner == GetOwner())\n\t\treturn owner;\n\t\t",
            "\tif (owner == GetOwner())\n\t\treturn owner; // note\n\t\t",
            "\tif (owner == GetOwner())\n\t\treturn owner\n\t\t",
        ] {
            let plan =
                unbraced_if_body_outdent_plan(source, source.len(), 4, false, false).unwrap();
            assert_eq!(plan.replacement, "\t", "{source:?}");
            assert_eq!(plan.selection_character, 1, "{source:?}");
        }
    }

    #[test]
    fn does_not_outdent_after_block_or_ambiguous_if_body() {
        for source in [
            "\tif (owner == GetOwner())\n\t\t{\n\t\t",
            "\tif (owner == GetOwner())\n\t\treturn owner +\n\t\t",
            "\tif (owner == GetOwner())\n\t\tif (other)\n\t\t",
            "\tif (owner == GetOwner())\n\t\treturn owner;\n\t",
        ] {
            assert!(
                unbraced_if_body_outdent_plan(source, source.len(), 4, false, false).is_none(),
                "{source:?}"
            );
        }
    }

    #[test]
    fn tab_from_an_empty_line_restores_the_completed_if_header_indent() {
        let source = "\t\tif (owner == GetOwner())\n\t\t\treturn owner;\n\t";
        let plan = unbraced_if_body_outdent_plan(source, source.len(), 4, false, true).unwrap();
        assert_eq!(plan.replacement, "\t\t");
        assert_eq!(plan.selection_character, 2);
        assert!(unbraced_if_body_outdent_plan(source, source.len(), 4, false, false).is_none());
    }

    #[test]
    fn tab_from_an_empty_line_supports_space_indentation_when_the_trigger_is_trusted() {
        let source = "        if (owner == GetOwner())\n            return owner;\n    ";
        let plan = unbraced_if_body_outdent_plan(source, source.len(), 4, true, true).unwrap();
        assert_eq!(plan.replacement, "        ");
        assert_eq!(plan.selection_character, 8);
        assert!(unbraced_if_body_outdent_plan(source, source.len(), 4, true, false).is_none());
    }

    #[test]
    fn preserves_crlf_and_utf16_positions_by_returning_byte_offset() {
        let source = "😀Run()\r\n\t";
        assert_eq!(insertion(source), None);
        let source = "Run(\"😀\")\r\n\t";
        assert_eq!(insertion(source), Some("Run(\"😀\")".len()));
    }

    #[test]
    fn expands_only_an_empty_standalone_native_block_comment_pair() {
        let source = "\t/**/\r\n";
        let cursor = source.find("*/").unwrap();
        let plan = block_comment_pair_plan(source, cursor, 4, false).unwrap();
        assert_eq!(plan.span.start, 1);
        assert_eq!(plan.span.end, 5);
        assert_eq!(plan.replacement, "/*\r\n\t\t\r\n\t*/");
        assert_eq!(plan.selection_character, 2);

        let spaces = block_comment_pair_plan("/**/", 2, 3, true).unwrap();
        assert_eq!(spaces.replacement, "/*\n   \n*/");
        assert_eq!(spaces.selection_character, 3);
    }

    #[test]
    fn rejects_nonempty_inline_and_ambiguous_block_comment_pairs() {
        for (source, cursor) in [
            ("value /**/", "value /**/".find("*/").unwrap()),
            ("/**/ value", "/**/".find("*/").unwrap()),
            ("/** text */", "/** text */".find("*/").unwrap()),
            ("\"/**/\"", "\"/**/\"".find("*/").unwrap()),
            ("/*\n/**/\n*/", "/*\n/**/\n*/".find("*/").unwrap()),
        ] {
            assert!(
                block_comment_pair_plan(source, cursor, 4, false).is_none(),
                "{source:?}"
            );
        }
    }
}
