//! Source-faithful, explicit formatting operations.
//!
//! This module exposes no LSP or VS Code types. LSP range and document
//! handlers will project current immutable snapshots into these edits.

use crate::lexer::{lex, TextSpan, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattingEdit {
    pub span: TextSpan,
    pub replacement: String,
}

/// Normalizes indentation for one explicit, comment-only source region.
pub fn format_comment_region(source: &str, range: TextSpan) -> Vec<FormattingEdit> {
    if range.start >= range.end
        || range.end > source.len()
        || !source.is_char_boundary(range.start)
        || !source.is_char_boundary(range.end)
    {
        return Vec::new();
    }

    let lines = selected_line_spans(source, range);
    let comments = lex(source)
        .into_iter()
        .filter(|token| is_comment(token.kind))
        .collect::<Vec<_>>();
    if lines.is_empty()
        || comments.is_empty()
        || comments
            .iter()
            .any(|comment| overlaps(comment.span, range) && !contains(range, comment.span))
        || lines.iter().any(|line| {
            let text = &source[line.start..line.end];
            !text.trim().is_empty() && !line_is_comment_only(*line, text, &comments)
        })
    {
        return Vec::new();
    }

    let mut edits = Vec::new();
    let mut group_indent: Option<&str> = None;
    for line in lines {
        let text = &source[line.start..line.end];
        if text.trim().is_empty() {
            group_indent = None;
            continue;
        }

        let indent_len = text.len() - text.trim_start_matches([' ', '\t']).len();
        let indent = &text[..indent_len];
        let replacement = match group_indent {
            Some(base) if first_content_character(text) == Some('*') => format!("{base} "),
            Some(base) => base.to_owned(),
            None => {
                group_indent = Some(indent);
                continue;
            }
        };
        if indent != replacement {
            edits.push(FormattingEdit {
                span: TextSpan::new(line.start, line.start + indent_len),
                replacement,
            });
        }
    }
    edits
}

fn is_comment(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LineComment
            | TokenKind::DocLineComment
            | TokenKind::BlockComment
            | TokenKind::DocBlockComment
    )
}

fn selected_line_spans(source: &str, range: TextSpan) -> Vec<TextSpan> {
    let start = source[..range.start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let end = source[range.end..]
        .find('\n')
        .map_or(source.len(), |index| range.end + index);
    let mut lines = Vec::new();
    let mut line_start = start;
    while line_start <= end {
        let line_end = source[line_start..end]
            .find('\n')
            .map_or(end, |index| line_start + index);
        let content_end = source[line_start..line_end]
            .strip_suffix('\r')
            .map_or(line_end, |_| line_end - 1);
        lines.push(TextSpan::new(line_start, content_end));
        if line_end == end {
            break;
        }
        line_start = line_end + 1;
    }
    lines
}

fn line_is_comment_only(line: TextSpan, text: &str, comments: &[Token]) -> bool {
    let mut offset = line.start;
    for character in text.chars() {
        if !character.is_whitespace()
            && !comments
                .iter()
                .any(|comment| comment.span.start <= offset && offset < comment.span.end)
        {
            return false;
        }
        offset += character.len_utf8();
    }
    true
}

fn first_content_character(line: &str) -> Option<char> {
    line.trim_start_matches([' ', '\t']).chars().next()
}

fn overlaps(left: TextSpan, right: TextSpan) -> bool {
    left.start < right.end && right.start < left.end
}

fn contains(outer: TextSpan, inner: TextSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected(source: &str) -> TextSpan {
        TextSpan::new(0, source.len())
    }

    fn apply_edits(source: &str, edits: &[FormattingEdit]) -> String {
        let mut formatted = source.to_owned();
        for edit in edits.iter().rev() {
            formatted.replace_range(edit.span.start..edit.span.end, &edit.replacement);
        }
        formatted
    }

    #[test]
    fn normalizes_line_comment_indentation_without_touching_payloads() {
        let source = "\t//! First\n  //! \\param value Input\n\t/// Alternate\n";
        let formatted = apply_edits(source, &format_comment_region(source, selected(source)));

        assert_eq!(
            formatted,
            "\t//! First\n\t//! \\param value Input\n\t/// Alternate\n"
        );
    }

    #[test]
    fn normalizes_block_comment_continuations_and_is_idempotent() {
        let source = "\t/*!\n\t* \\brief Summary\n\t  * \\warning Keep prose\n\t*/\n";
        let formatted = apply_edits(source, &format_comment_region(source, selected(source)));

        assert_eq!(
            formatted,
            "\t/*!\n\t * \\brief Summary\n\t * \\warning Keep prose\n\t */\n"
        );
        assert!(format_comment_region(&formatted, selected(&formatted)).is_empty());
    }

    #[test]
    fn rejects_code_trailing_comments_partial_tokens_and_non_comment_text() {
        for source in [
            "Run(); //!< trailing\n",
            "[Attribute()]\n//! documentation\n",
            "\"//! string\"\n",
        ] {
            assert!(
                format_comment_region(source, selected(source)).is_empty(),
                "{source:?}"
            );
        }

        let source = "//! documentation\n";
        let partial = TextSpan::new(1, source.len());
        assert!(format_comment_region(source, partial).is_empty());
    }

    #[test]
    fn preserves_crlf_and_blank_line_group_boundaries() {
        let source = concat!(
            "\t//! First",
            "\r\n",
            "  //! Second",
            "\r\n",
            "\r\n",
            "  //! Nested",
            "\r\n",
            "\t//! Nested second",
            "\r\n"
        );
        let formatted = apply_edits(source, &format_comment_region(source, selected(source)));

        assert_eq!(
            formatted,
            concat!(
                "\t//! First",
                "\r\n",
                "\t//! Second",
                "\r\n",
                "\r\n",
                "  //! Nested",
                "\r\n",
                "  //! Nested second",
                "\r\n"
            )
        );
    }
}
