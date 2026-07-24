use crate::lexer::TextSpan;
use crate::lsp::{range_for_span, LspRange};
use crate::syntax::ParseDiagnostic;
use serde::Serialize;
use serde_json::{json, Value};

pub(crate) const PARSER_DIAGNOSTIC_SOURCE: &str = "Reforger Script Tools parser";
pub(crate) const PARSER_DIAGNOSTIC_CODE: &str = "reforger.parser.syntax";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: u32,
    pub source: String,
    pub code: String,
    pub message: String,
}

pub(crate) fn publish_diagnostics_message(
    uri: &str,
    version: i32,
    source: &str,
    diagnostics: &[ParseDiagnostic],
) -> Value {
    let diagnostics = parser_diagnostics_for_source(source, diagnostics);
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "version": version,
            "diagnostics": diagnostics
        }
    })
}

pub fn parser_diagnostics_for_source(
    source: &str,
    diagnostics: &[ParseDiagnostic],
) -> Vec<LspDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| LspDiagnostic {
            range: diagnostic_range_for_span(source, diagnostic.span),
            severity: 1,
            source: PARSER_DIAGNOSTIC_SOURCE.to_string(),
            code: PARSER_DIAGNOSTIC_CODE.to_string(),
            message: diagnostic.message.clone(),
        })
        .collect()
}

pub(crate) fn clear_diagnostics_message(uri: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": []
        }
    })
}

fn diagnostic_range_for_span(source: &str, span: TextSpan) -> LspRange {
    let start = clamp_char_boundary(source, span.start.min(source.len()));
    let end = clamp_char_boundary(source, span.end.min(source.len()));
    if end > start {
        return range_for_span(source, TextSpan::new(start, end));
    }

    if let Some((visible_start, visible_end)) = visible_char_at_or_after(source, start) {
        return range_for_span(source, TextSpan::new(visible_start, visible_end));
    }

    if let Some((visible_start, visible_end)) = visible_char_before(source, start) {
        return range_for_span(source, TextSpan::new(visible_start, visible_end));
    }

    range_for_span(source, TextSpan::new(start, end))
}

fn clamp_char_boundary(source: &str, mut offset: usize) -> usize {
    offset = offset.min(source.len());
    while offset > 0 && !source.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn visible_char_at_or_after(source: &str, offset: usize) -> Option<(usize, usize)> {
    source
        .get(offset..)?
        .char_indices()
        .find_map(|(relative, ch)| {
            (!ch.is_whitespace()).then_some((offset + relative, offset + relative + ch.len_utf8()))
        })
}

fn visible_char_before(source: &str, offset: usize) -> Option<(usize, usize)> {
    source
        .get(..offset)?
        .char_indices()
        .filter(|(_, ch)| !ch.is_whitespace())
        .last()
        .map(|(start, ch)| (start, start + ch.len_utf8()))
}
