use super::{FileIndexAnalysis, LspPositionIndex};
use crate::model::SymbolKind;
use serde::Serialize;

pub(crate) const MAX_AUTO_PREVIEW_LINES: u32 = 80;
const AUTO_PREVIEW_LEAD_LINES: u32 = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewContext {
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) kind: &'static str,
    pub(crate) truncated: bool,
}

pub(crate) fn preview_context(
    source: &str,
    analysis: &FileIndexAnalysis,
    requested_line: u32,
) -> PreviewContext {
    let positions = LspPositionIndex::new(source);
    let candidate = analysis
        .index
        .symbols()
        .iter()
        .filter(|symbol| supports_auto_context(symbol.kind))
        .filter_map(|symbol| {
            let range = positions.range_for_span(symbol.span);
            let end_line = if range.end.character == 0 && range.end.line > range.start.line {
                range.end.line - 1
            } else {
                range.end.line
            };
            (range.start.line <= requested_line && requested_line <= end_line).then_some((
                symbol.kind,
                range.start.line,
                end_line,
                symbol.span.end.saturating_sub(symbol.span.start),
            ))
        })
        .min_by_key(|(_, _, _, span_len)| *span_len);

    let Some((kind, start_line, end_line, _)) = candidate else {
        return one_line_context(requested_line, "line");
    };
    if is_single_line_context(kind) {
        return one_line_context(requested_line, symbol_kind(kind));
    }

    let line_count = end_line.saturating_sub(start_line).saturating_add(1);
    if line_count <= MAX_AUTO_PREVIEW_LINES {
        return PreviewContext {
            start_line,
            end_line,
            kind: symbol_kind(kind),
            truncated: false,
        };
    }

    let mut window_start = requested_line
        .saturating_sub(AUTO_PREVIEW_LEAD_LINES)
        .max(start_line);
    let max_start = end_line
        .saturating_add(1)
        .saturating_sub(MAX_AUTO_PREVIEW_LINES);
    window_start = window_start.min(max_start);
    PreviewContext {
        start_line: window_start,
        end_line: window_start + MAX_AUTO_PREVIEW_LINES - 1,
        kind: symbol_kind(kind),
        truncated: true,
    }
}

fn one_line_context(line: u32, kind: &'static str) -> PreviewContext {
    PreviewContext {
        start_line: line,
        end_line: line,
        kind,
        truncated: false,
    }
}

fn supports_auto_context(kind: SymbolKind) -> bool {
    !matches!(
        kind,
        SymbolKind::TypeParameter | SymbolKind::Parameter | SymbolKind::LocalVariable
    )
}

fn is_single_line_context(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::EnumMember
            | SymbolKind::Typedef
            | SymbolKind::GlobalField
            | SymbolKind::Field
            | SymbolKind::PreprocessorMacro
    )
}

fn symbol_kind(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "class",
        SymbolKind::TypeParameter => "typeParameter",
        SymbolKind::Enum => "enum",
        SymbolKind::EnumMember => "enumMember",
        SymbolKind::Typedef => "typedef",
        SymbolKind::Function => "function",
        SymbolKind::GlobalField => "globalField",
        SymbolKind::Field => "field",
        SymbolKind::Method => "method",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Destructor => "destructor",
        SymbolKind::Parameter => "parameter",
        SymbolKind::LocalVariable => "localVariable",
        SymbolKind::PreprocessorMacro => "preprocessorMacro",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::file_index_for_source;

    #[test]
    fn fields_use_one_line() {
        let source = "class Example\n{\n\tint m_Value;\n}\n";
        let context = preview_context(source, &file_index_for_source(source), 2);

        assert_eq!(context.start_line, 2);
        assert_eq!(context.end_line, 2);
        assert_eq!(context.kind, "field");
    }

    #[test]
    fn method_text_uses_the_complete_nearest_method() {
        let source = "class Example\n{\n\tvoid Run()\n\t{\n\t\tPrint(\"here\");\n\t}\n}\n";
        let context = preview_context(source, &file_index_for_source(source), 4);

        assert_eq!(context.start_line, 2);
        assert_eq!(context.end_line, 5);
        assert_eq!(context.kind, "method");
        assert!(!context.truncated);
    }

    #[test]
    fn class_declarations_use_the_class_scope() {
        let source = "class Example\n{\n\tvoid Run() {}\n}\n";
        let context = preview_context(source, &file_index_for_source(source), 0);

        assert_eq!(context.start_line, 0);
        assert_eq!(context.end_line, 3);
        assert_eq!(context.kind, "class");
    }

    #[test]
    fn large_scopes_are_bounded_and_keep_the_match_visible() {
        let body = (0..120)
            .map(|line| format!("\t\tint value{line};"))
            .collect::<Vec<_>>()
            .join("\n");
        let source = format!("class Example\n{{\n\tvoid Run()\n\t{{\n{body}\n\t}}\n}}\n");
        let requested_line = 100;
        let context = preview_context(&source, &file_index_for_source(&source), requested_line);

        assert_eq!(
            context.end_line - context.start_line + 1,
            MAX_AUTO_PREVIEW_LINES
        );
        assert!(context.start_line <= requested_line && requested_line <= context.end_line);
        assert_eq!(context.kind, "method");
        assert!(context.truncated);
    }
}
