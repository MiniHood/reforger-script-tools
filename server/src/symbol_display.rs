use crate::index::{
    GlobalSymbolId, IndexedAttribute, IndexedConditionalBranch, IndexedDocComment, SymbolIndex,
};
use crate::lexer::TextSpan;
use crate::model::{CallableForm, SourceCategory, SourceKind, SymbolKind};
use std::path::PathBuf;

const DOC_PREVIEW_LIMIT: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDisplayInfo {
    pub id: GlobalSymbolId,
    pub label: String,
    pub kind: SymbolKind,
    pub detail: Option<String>,
    pub signature: Option<String>,
    pub doc_comments: Vec<IndexedDocComment>,
    pub documentation_preview: Option<String>,
    pub modifiers: Vec<String>,
    pub attributes: Vec<IndexedAttribute>,
    pub source_kind: SourceKind,
    pub source_category: SourceCategory,
    pub source_priority: u16,
    pub relative_path: Option<PathBuf>,
    pub absolute_path: Option<PathBuf>,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub conditional_context: Vec<IndexedConditionalBranch>,
    pub callable_form: Option<CallableForm>,
}

pub struct SymbolDisplay;

impl SymbolDisplay {
    pub fn for_symbol(index: &SymbolIndex, id: GlobalSymbolId) -> Option<SymbolDisplayInfo> {
        let symbol = index.symbol(id)?;
        let file = index.file(id.file_id)?;
        let signature = index.callable_signature(id);
        let detail = symbol_display_detail(index, id);

        Some(SymbolDisplayInfo {
            id,
            label: symbol
                .name
                .clone()
                .unwrap_or_else(|| "<unknown>".to_string()),
            kind: symbol.kind,
            detail,
            signature,
            doc_comments: symbol.doc_comments.clone(),
            documentation_preview: doc_preview(&symbol.doc_comments),
            modifiers: symbol.modifiers.clone(),
            attributes: symbol.attributes.clone(),
            source_kind: file.metadata.kind,
            source_category: file.metadata.category,
            source_priority: file.metadata.priority,
            relative_path: file.metadata.relative_path.clone(),
            absolute_path: file.metadata.absolute_path.clone(),
            span: symbol.span,
            selection_span: symbol.selection_span,
            conditional_context: symbol.conditional_context.clone(),
            callable_form: symbol.callable_form,
        })
    }
}

fn symbol_display_detail(index: &SymbolIndex, id: GlobalSymbolId) -> Option<String> {
    let symbol = index.symbol(id)?;

    if let Some(signature) = index.callable_signature(id) {
        return Some(signature);
    }

    match symbol.kind {
        SymbolKind::Class => prefixed_detail("base", symbol.detail.base_type.as_deref()),
        SymbolKind::EnumMember => {
            prefixed_detail("value", symbol.detail.enum_value_text.as_deref())
        }
        SymbolKind::Typedef
        | SymbolKind::GlobalField
        | SymbolKind::Field
        | SymbolKind::Parameter
        | SymbolKind::LocalVariable => {
            let mut parts = Vec::new();
            push_prefixed(&mut parts, "type", symbol.detail.type_text.as_deref());
            push_prefixed(&mut parts, "default", symbol.detail.default_text.as_deref());
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(" "))
            }
        }
        SymbolKind::Function
        | SymbolKind::Method
        | SymbolKind::Constructor
        | SymbolKind::Destructor => {
            prefixed_detail("return", symbol.detail.return_type_text.as_deref())
        }
        _ => None,
    }
}

fn prefixed_detail(label: &str, value: Option<&str>) -> Option<String> {
    value.map(|value| format!("{label} {value}"))
}

fn push_prefixed(parts: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        parts.push(format!("{label} {value}"));
    }
}

fn doc_preview(comments: &[IndexedDocComment]) -> Option<String> {
    let lines = comments
        .iter()
        .flat_map(|comment| readable_doc_lines(&comment.text))
        .collect::<Vec<_>>();

    lines
        .iter()
        .find_map(|line| display_brief_doc_line(line))
        .or_else(|| lines.into_iter().find_map(display_doc_line))
        .map(|line| truncate_preview(&line, DOC_PREVIEW_LIMIT))
}

fn readable_doc_lines(comment: &str) -> Vec<String> {
    comment.lines().map(readable_doc_line).collect()
}

fn readable_doc_line(line: &str) -> String {
    let mut value = line.trim();
    value = value.strip_prefix("//!").unwrap_or(value).trim_start();
    value = value.strip_prefix("/*!").unwrap_or(value).trim_start();
    value = value.strip_prefix("/*").unwrap_or(value).trim_start();
    value = value.strip_prefix('*').unwrap_or(value).trim_start();
    value = value.strip_suffix("*/").unwrap_or(value).trim_end();
    value.trim().to_string()
}

fn display_doc_line(line: String) -> Option<String> {
    let line = line.trim();
    if line.is_empty() || is_doc_separator(line) {
        return None;
    }

    if line == "\\code" || line == "@code" || line == "\\endcode" || line == "@endcode" {
        return None;
    }

    if let Some(value) = strip_doc_tag(line, "\\brief").or_else(|| strip_doc_tag(line, "@brief")) {
        return non_empty(value);
    }
    if let Some(value) = strip_doc_tag(line, "\\return").or_else(|| strip_doc_tag(line, "@return"))
    {
        return non_empty(format!("Returns {value}"));
    }
    if let Some(value) =
        strip_doc_tag(line, "\\returns").or_else(|| strip_doc_tag(line, "@returns"))
    {
        return non_empty(format!("Returns {value}"));
    }
    if let Some(value) =
        strip_doc_tag(line, "\\warning").or_else(|| strip_doc_tag(line, "@warning"))
    {
        return non_empty(format!("Warning: {value}"));
    }
    if let Some(value) = strip_doc_tag(line, "\\note").or_else(|| strip_doc_tag(line, "@note")) {
        return non_empty(format!("Note: {value}"));
    }
    if let Some(value) = strip_param_doc_tag(line) {
        return non_empty(format!("Parameter {value}"));
    }
    if line.starts_with('\\') || line.starts_with('@') {
        return None;
    }

    Some(line.to_string())
}

fn display_brief_doc_line(line: &str) -> Option<String> {
    let line = line.trim();
    strip_doc_tag(line, "\\brief")
        .or_else(|| strip_doc_tag(line, "@brief"))
        .and_then(non_empty)
}

fn is_doc_separator(line: &str) -> bool {
    line.chars()
        .all(|ch| matches!(ch, '-' | '=' | '_' | '*' | '/'))
}

fn strip_doc_tag(line: &str, tag: &str) -> Option<String> {
    line.strip_prefix(tag).map(|value| {
        value
            .trim_start_matches([' ', '\t', ':', '-'])
            .trim()
            .to_string()
    })
}

fn strip_param_doc_tag(line: &str) -> Option<String> {
    for tag in ["\\param", "@param"] {
        let Some(rest) = line.strip_prefix(tag) else {
            continue;
        };
        let rest = rest.trim_start();
        let rest = if let Some(after_bracket) = rest.strip_prefix('[') {
            after_bracket
                .split_once(']')
                .map(|(_, value)| value.trim_start())
                .unwrap_or(rest)
        } else {
            rest
        };
        return Some(rest.to_string());
    }
    None
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn truncate_preview(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }

    let mut result = value
        .chars()
        .take(limit.saturating_sub(3))
        .collect::<String>();
    result.push_str("...");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::index::SymbolIndex;
    use crate::model::{
        SourceCategory, SourceFileMetadata, SymbolCatalog, SOURCE_PRIORITY_WORKSPACE,
    };
    use crate::parser::parse_source;

    #[test]
    fn displays_class_with_base_attributes_modifiers_docs_and_source() {
        let index = index(
            r#"//! Class documentation.
[BaseContainerProps()]
modded class Example : Base
{
}
"#,
        );
        let class_id = find(&index, SymbolKind::Class, "Example");

        let display = SymbolDisplay::for_symbol(&index, class_id).unwrap();

        assert_eq!(display.label, "Example");
        assert_eq!(display.kind, SymbolKind::Class);
        assert_eq!(display.detail.as_deref(), Some("base Base"));
        assert_eq!(
            display.documentation_preview.as_deref(),
            Some("Class documentation.")
        );
        assert_eq!(display.modifiers, vec!["modded"]);
        assert_eq!(display.attributes.len(), 1);
        assert_eq!(
            display.attributes[0].name.as_deref(),
            Some("BaseContainerProps")
        );
        assert_eq!(display.source_kind, SourceKind::Workspace);
        assert_eq!(display.source_category, SourceCategory::Workspace);
        assert_eq!(display.source_priority, SOURCE_PRIORITY_WORKSPACE);
    }

    #[test]
    fn displays_declaration_details_for_non_callable_symbols() {
        let index = index(
            r#"typedef string FactionKey;

enum Example
{
	Value = 4,
}

Game g_Game;

class Holder
{
	int m_Value;
	void Run(int value = 4);
	void Local()
	{
		int localValue = 5;
	}
}
"#,
        );

        let typedef =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Typedef, "FactionKey"))
                .unwrap();
        assert_eq!(typedef.detail.as_deref(), Some("type string"));

        let enum_member =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::EnumMember, "Value"))
                .unwrap();
        assert_eq!(enum_member.detail.as_deref(), Some("value 4"));

        let global =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::GlobalField, "g_Game"))
                .unwrap();
        assert_eq!(global.detail.as_deref(), Some("type Game"));

        let field =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Field, "m_Value")).unwrap();
        assert_eq!(field.detail.as_deref(), Some("type int"));

        let parameter =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Parameter, "value"))
                .unwrap();
        assert_eq!(parameter.detail.as_deref(), Some("type int default 4"));

        let local = SymbolDisplay::for_symbol(
            &index,
            find(&index, SymbolKind::LocalVariable, "localValue"),
        )
        .unwrap();
        assert_eq!(local.detail.as_deref(), Some("type int default 5"));
    }

    #[test]
    fn displays_callable_signatures() {
        let index = index(
            r#"void GlobalFn(int value = 4);

class Example
{
	void Example(int value);
	void ~Example();
	void Run(string name);
}
"#,
        );

        let function =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Function, "GlobalFn"))
                .unwrap();
        assert_eq!(
            function.signature.as_deref(),
            Some("GlobalFn(int value = 4) -> void")
        );
        assert_eq!(
            function.detail.as_deref(),
            Some("GlobalFn(int value = 4) -> void")
        );

        let method =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Method, "Run")).unwrap();
        assert_eq!(
            method.signature.as_deref(),
            Some("Example.Run(string name) -> void")
        );

        let constructor =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Constructor, "Example"))
                .unwrap();
        assert_eq!(constructor.signature.as_deref(), Some("Example(int value)"));

        let destructor =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Destructor, "Example"))
                .unwrap();
        assert_eq!(destructor.signature.as_deref(), Some("~Example()"));
    }

    #[test]
    fn displays_stable_output_without_optional_metadata() {
        let index = index("class Empty {}");
        let display =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Class, "Empty")).unwrap();

        assert_eq!(display.label, "Empty");
        assert_eq!(display.detail, None);
        assert_eq!(display.documentation_preview, None);
        assert!(display.doc_comments.is_empty());
        assert!(display.attributes.is_empty());
        assert!(display.modifiers.is_empty());
    }

    #[test]
    fn renders_clean_doc_previews_without_changing_raw_comments() {
        let index = index(
            r#"//! \brief Example class.
class Example
{
	/*!
	 * \param value raw input value.
	 * \return true when accepted.
	 */
	void Run(int value);
}
"#,
        );

        let class =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Class, "Example")).unwrap();
        assert_eq!(
            class.documentation_preview.as_deref(),
            Some("Example class.")
        );
        assert_eq!(class.doc_comments[0].text, "//! \\brief Example class.");

        let method =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Method, "Run")).unwrap();
        assert_eq!(
            method.documentation_preview.as_deref(),
            Some("Parameter value raw input value.")
        );
        assert!(method.doc_comments[0]
            .text
            .contains("\\return true when accepted."));
    }

    #[test]
    fn skips_empty_separator_and_code_only_doc_preview_lines() {
        let index = index(
            r#"/*!
 * -------------------------------------------------------------------------
 * \code
 * int value = 1;
 * \endcode
 */
class Example {}
"#,
        );
        let display =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Class, "Example")).unwrap();

        assert_eq!(
            display.documentation_preview.as_deref(),
            Some("int value = 1;")
        );
    }

    #[test]
    fn skips_unknown_doxygen_command_lines() {
        let index = index(
            r#"/*!
 * \addtogroup Attributes
 * Useful attribute docs.
 */
class Example {}
"#,
        );
        let display =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Class, "Example")).unwrap();

        assert_eq!(
            display.documentation_preview.as_deref(),
            Some("Useful attribute docs.")
        );
    }

    #[test]
    fn prefers_explicit_brief_preview_over_earlier_body_lines() {
        let index = index(
            r#"/*!
 * < values from generated documentation.
 * \brief Clear generated summary.
 */
class Example {}
"#,
        );
        let display =
            SymbolDisplay::for_symbol(&index, find(&index, SymbolKind::Class, "Example")).unwrap();

        assert_eq!(
            display.documentation_preview.as_deref(),
            Some("Clear generated summary.")
        );
    }

    fn index(source: &str) -> SymbolIndex {
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ast = AstSourceFile::new(source, &parse);
        let catalog = SymbolCatalog::from_ast_with_metadata(
            source,
            &ast,
            SourceFileMetadata {
                kind: SourceKind::Workspace,
                category: SourceCategory::Workspace,
                absolute_path: None,
                root_path: None,
                relative_path: None,
                priority: SOURCE_PRIORITY_WORKSPACE,
            },
        );
        SymbolIndex::from_catalogs([&catalog])
    }

    fn find(index: &SymbolIndex, kind: SymbolKind, name: &str) -> GlobalSymbolId {
        index
            .symbols()
            .iter()
            .find(|symbol| symbol.kind == kind && symbol.name.as_deref() == Some(name))
            .map(|symbol| symbol.id)
            .unwrap_or_else(|| panic!("missing {kind:?} {name}"))
    }
}
