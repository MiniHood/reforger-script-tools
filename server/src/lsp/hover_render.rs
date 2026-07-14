use crate::index_query::{EditorCompletionCandidate, EditorCompletionOrigin, IndexQuery};
use crate::lsp::semantic_tokens::semantic_token_color_for_type;
use crate::lsp::symbol_kind_label;
use crate::model::{SourceCategory, SourceKind, SymbolKind};
use crate::symbol_display::{documentation_display, DocumentationDisplay, SymbolDisplayInfo};

const MEMBER_SAMPLE_LIMIT: usize = 4;
const ATTRIBUTE_CONSTRUCTOR_SIGNATURE: &str = r#"void Attribute(
	string defvalue = "",
	string uiwidget = "auto",
	string desc = "",
	string params = "",
	ParamEnumArray enums = NULL,
	string category = "",
	int precision = 3,
	typename enumType = void,
	bool prefabbed = false
)"#;

pub(crate) struct HoverRenderContext<'a, 'index> {
    pub query: &'a IndexQuery<'index>,
}

pub(crate) fn render_hover_markdown(
    display: &SymbolDisplayInfo,
    context: Option<HoverRenderContext<'_, '_>>,
) -> String {
    let mut sections = Vec::new();
    sections.push(render_header(display));
    sections.push(render_code(display));

    let docs = documentation_display(&display.doc_comments);
    let attribute_display = attribute_display(display);
    if let Some(documentation) = render_documentation(&docs, attribute_display.as_ref()) {
        sections.push(documentation);
    }

    if let Some(attribute_display) = &attribute_display {
        if let Some(params) = render_attribute_params(attribute_display) {
            sections.push(params);
        }
        sections.push(render_attribute_constructor());
    }

    if let Some(detail) = render_detail(display) {
        sections.push(detail);
    }

    if let Some(context) = context {
        if display.kind == SymbolKind::Class {
            if let Some(summary) = render_class_members(display, context.query) {
                sections.push(summary);
            }
        } else if display.kind == SymbolKind::Enum {
            if let Some(summary) = render_enum_members(display, context.query) {
                sections.push(summary);
            }
        }
    }

    if let Some(metadata) = render_metadata(display) {
        sections.push(metadata);
    }

    sections.join("\n\n")
}

fn render_header(display: &SymbolDisplayInfo) -> String {
    let kind = hover_kind_label(display.kind);
    let token_type = hover_token_type(display.kind);
    let color = semantic_token_color_for_type(token_type);
    let container = hover_container_name(display)
        .map(|container| format!(" in {container}"))
        .unwrap_or_default();
    if color == "<default>" {
        format!("**{kind}{container}**")
    } else {
        format!(
            "<span style=\"color:{color};\">{}{}</span>",
            escape_html_text(kind),
            escape_html_text(&container)
        )
    }
}

fn render_code(display: &SymbolDisplayInfo) -> String {
    let code = hover_declaration_text(display);
    format!("```enforce\n{}\n```", escape_fence_text(&code))
}

fn render_detail(display: &SymbolDisplayInfo) -> Option<String> {
    let detail = display.detail.as_ref()?;
    if display
        .signature
        .as_ref()
        .is_some_and(|signature| signature == detail)
    {
        return None;
    }
    Some(format!("**Detail:** `{}`", escape_inline_code(detail)))
}

fn render_documentation(
    docs: &DocumentationDisplay,
    attribute_display: Option<&AttributeDisplay>,
) -> Option<String> {
    if docs.summary.is_none()
        && docs.parameters.is_empty()
        && docs.returns.is_none()
        && docs.warnings.is_empty()
        && docs.notes.is_empty()
        && attribute_display
            .and_then(|display| display.description.as_ref())
            .is_none()
    {
        return None;
    }

    let mut lines = Vec::new();
    if let Some(summary) = docs
        .summary
        .as_ref()
        .or_else(|| attribute_display.and_then(|display| display.description.as_ref()))
    {
        lines.push(summary.clone());
    }
    if !docs.parameters.is_empty() {
        lines.push("### Parameters".to_string());
        for parameter in &docs.parameters {
            let direction = parameter
                .direction
                .as_deref()
                .map(|direction| format!(" [{direction}]"))
                .unwrap_or_default();
            if parameter.description.is_empty() {
                lines.push(format!(
                    "- `{}`{}",
                    escape_inline_code(&parameter.name),
                    direction
                ));
            } else {
                lines.push(format!(
                    "- `{}`{}: {}",
                    escape_inline_code(&parameter.name),
                    direction,
                    parameter.description
                ));
            }
        }
    }
    if let Some(returns) = &docs.returns {
        lines.push("### Returns".to_string());
        lines.push(returns.clone());
    }
    for warning in &docs.warnings {
        lines.push("### Warning".to_string());
        lines.push(warning.clone());
    }
    for note in &docs.notes {
        lines.push("### Note".to_string());
        lines.push(note.clone());
    }

    Some(lines.join("\n\n"))
}

fn render_class_members(display: &SymbolDisplayInfo, query: &IndexQuery<'_>) -> Option<String> {
    let members = query.completion_members_for_class(&display.label);
    if members.candidates.is_empty() {
        return None;
    }
    let direct_members = members
        .candidates
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.origin,
                EditorCompletionOrigin::Direct | EditorCompletionOrigin::Overlay
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let inherited_count = members
        .candidates
        .iter()
        .filter(|candidate| candidate.origin == EditorCompletionOrigin::Inherited)
        .count();

    let constructors = members_for_kind(&direct_members, SymbolKind::Constructor);
    let destructors = members_for_kind(&direct_members, SymbolKind::Destructor);
    let methods = members_for_kind(&direct_members, SymbolKind::Method);
    let fields = direct_members
        .iter()
        .filter(|candidate| candidate.kind == SymbolKind::Field)
        .collect::<Vec<_>>();

    let mut lines = Vec::new();
    push_member_group(&mut lines, "Constructors", &constructors);
    push_member_group(&mut lines, "Destructors", &destructors);
    push_member_group(&mut lines, "Functions", &methods);
    push_member_group(&mut lines, "Properties", &fields);
    if inherited_count > 0 {
        lines.push(format!("**Inherited members:** {inherited_count}"));
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn members_for_kind(
    candidates: &[EditorCompletionCandidate],
    kind: SymbolKind,
) -> Vec<&EditorCompletionCandidate> {
    candidates
        .iter()
        .filter(|candidate| candidate.kind == kind)
        .collect()
}

fn push_member_group(lines: &mut Vec<String>, label: &str, members: &[&EditorCompletionCandidate]) {
    if members.is_empty() {
        return;
    }
    let sample = members
        .iter()
        .take(MEMBER_SAMPLE_LIMIT)
        .map(format_member_line)
        .collect::<Vec<_>>()
        .join("\n");
    let omitted = members.len().saturating_sub(MEMBER_SAMPLE_LIMIT);
    let mut block = format!("### {label}\n\n```enforce\n{sample}");
    if omitted == 0 {
        block.push_str("\n```");
    } else {
        block.push_str(&format!("\n// +{omitted} more\n```"));
    }
    lines.push(block);
}

fn render_enum_members(display: &SymbolDisplayInfo, query: &IndexQuery<'_>) -> Option<String> {
    let members = query
        .completion_static_members_for_type(&display.label)
        .into_iter()
        .filter(|candidate| candidate.kind == SymbolKind::EnumMember)
        .collect::<Vec<_>>();
    if members.is_empty() {
        return None;
    }
    let sample = members
        .iter()
        .take(MEMBER_SAMPLE_LIMIT)
        .map(|candidate| {
            candidate.display.detail.as_ref().map_or_else(
                || candidate.display.label.clone(),
                |detail| format!("{} // {}", candidate.display.label, detail),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let omitted = members.len().saturating_sub(MEMBER_SAMPLE_LIMIT);
    let mut block = format!("### Members\n\n```enforce\n{sample}");
    if omitted > 0 {
        block.push_str(&format!("\n// +{omitted} more"));
    }
    block.push_str("\n```");
    Some(block)
}

fn render_metadata(display: &SymbolDisplayInfo) -> Option<String> {
    let mut parts = Vec::new();
    if !display.modifiers.is_empty() {
        parts.push(format!("**Modifiers:** {}", display.modifiers.join(", ")));
    }

    let attribute_names = display
        .attributes
        .iter()
        .map(|attribute| {
            attribute
                .name
                .as_deref()
                .unwrap_or(attribute.text.as_str())
                .to_string()
        })
        .collect::<Vec<_>>();
    if !attribute_names.is_empty() {
        parts.push(format!("**Attributes:** {}", attribute_names.join(", ")));
    }

    if let Some(source) = render_source(display) {
        parts.push(source);
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

fn render_source(display: &SymbolDisplayInfo) -> Option<String> {
    let path = display
        .relative_path
        .as_ref()
        .or(display.absolute_path.as_ref())?
        .display()
        .to_string();
    Some(format!(
        "**Source:** {} / {} `{}`",
        source_kind_label(display.source_kind),
        source_category_label(display.source_category),
        escape_inline_code(&path)
    ))
}

fn hover_token_type(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class | SymbolKind::Typedef | SymbolKind::TypeParameter => "class",
        SymbolKind::Enum => "enum",
        SymbolKind::Function => "function",
        SymbolKind::Method | SymbolKind::Constructor | SymbolKind::Destructor => "method",
        SymbolKind::GlobalField | SymbolKind::Field => "property",
        SymbolKind::Parameter => "parameter",
        SymbolKind::EnumMember | SymbolKind::LocalVariable | SymbolKind::PreprocessorMacro => {
            "variable"
        }
    }
}

fn hover_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Field | SymbolKind::GlobalField => "property",
        SymbolKind::LocalVariable => "variable",
        SymbolKind::EnumMember => "enum value",
        _ => symbol_kind_label(kind),
    }
}

fn hover_container_name(display: &SymbolDisplayInfo) -> Option<String> {
    let signature = display.signature.as_deref()?;
    match display.kind {
        SymbolKind::Method => {
            let prefix = signature.split('(').next()?;
            let (owner, name) = prefix.rsplit_once('.')?;
            if name == display.label {
                Some(owner.to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn hover_declaration_text(display: &SymbolDisplayInfo) -> String {
    if let Some(signature) = &display.signature {
        return signature.trim_end_matches(';').to_string();
    }

    let detail = display.detail.as_deref();
    let mut text = match display.kind {
        SymbolKind::Class => match detail.and_then(|detail| detail.strip_prefix("base ")) {
            Some(base) => format!("class {} : {base}", display.label),
            None => format!("class {}", display.label),
        },
        SymbolKind::Enum => format!("enum {}", display.label),
        SymbolKind::Typedef => match detail.and_then(|detail| detail.strip_prefix("type ")) {
            Some(target) => format!("typedef {target} {}", display.label),
            None => display.label.clone(),
        },
        SymbolKind::Field | SymbolKind::GlobalField | SymbolKind::LocalVariable => {
            declaration_with_type_default(display, detail)
        }
        SymbolKind::Parameter => declaration_with_type_default(display, detail),
        SymbolKind::EnumMember => match detail.and_then(|detail| detail.strip_prefix("value ")) {
            Some(value) => format!("{} = {value}", display.label),
            None => display.label.clone(),
        },
        SymbolKind::PreprocessorMacro => format!("#define {}", display.label),
        _ => display.label.clone(),
    };
    if !display.modifiers.is_empty()
        && matches!(
            display.kind,
            SymbolKind::Field | SymbolKind::GlobalField | SymbolKind::LocalVariable
        )
    {
        text = format!("{} {text}", display.modifiers.join(" "));
    }
    text.trim_end_matches(';').to_string()
}

fn declaration_with_type_default(display: &SymbolDisplayInfo, detail: Option<&str>) -> String {
    let Some(detail) = detail else {
        return display.label.clone();
    };
    let Some(rest) = detail.strip_prefix("type ") else {
        return display.label.clone();
    };
    let (type_text, default_text) = match rest.split_once(" default ") {
        Some((type_text, default_text)) => (type_text, Some(default_text)),
        None => (rest, None),
    };
    match default_text {
        Some(default_text) => format!("{type_text} {} = {default_text}", display.label),
        None => format!("{type_text} {}", display.label),
    }
}

fn format_member_line(candidate: &&EditorCompletionCandidate) -> String {
    candidate
        .signature
        .as_deref()
        .or(candidate.detail.as_deref())
        .or(candidate.name.as_deref())
        .unwrap_or("<unknown>")
        .trim_end_matches(';')
        .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttributeDisplay {
    description: Option<String>,
    params: Vec<AttributeParamDisplay>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttributeParamDisplay {
    name: String,
    type_text: &'static str,
    value: String,
}

fn attribute_display(display: &SymbolDisplayInfo) -> Option<AttributeDisplay> {
    let attribute = display
        .attributes
        .iter()
        .find(|attribute| attribute.name.as_deref() == Some("Attribute"))?;
    let args = attribute_argument_values(&attribute.text);
    if args.is_empty() {
        return None;
    }

    let specs = attribute_param_specs();
    let mut params = Vec::new();
    let mut description = None;
    for (index, argument) in args.into_iter().enumerate() {
        let (name, value) = match argument.split_once(':') {
            Some((name, value)) if is_named_attribute_arg(name.trim()) => {
                (name.trim().to_string(), value.trim().to_string())
            }
            _ => {
                let Some((name, _type_text)) = specs.get(index) else {
                    continue;
                };
                ((*name).to_string(), argument.trim().to_string())
            }
        };
        let Some((_, type_text)) = specs.iter().find(|(spec_name, _)| *spec_name == name) else {
            continue;
        };
        if name == "desc" {
            description = Some(unquote_string(&value));
            continue;
        }
        params.push(AttributeParamDisplay {
            name,
            type_text,
            value,
        });
    }

    if params.is_empty() && description.is_none() {
        None
    } else {
        Some(AttributeDisplay {
            description,
            params,
        })
    }
}

fn render_attribute_params(display: &AttributeDisplay) -> Option<String> {
    if display.params.is_empty() {
        return None;
    }
    let lines = display
        .params
        .iter()
        .map(|param| format!("{} {} = {}", param.type_text, param.name, param.value))
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!("### Params\n\n```enforce\n{lines}\n```"))
}

fn render_attribute_constructor() -> String {
    format!(
        "### Constructor\n\n```enforce\n{}\n```",
        ATTRIBUTE_CONSTRUCTOR_SIGNATURE
    )
}

fn attribute_param_specs() -> &'static [(&'static str, &'static str)] {
    &[
        ("defvalue", "string"),
        ("uiwidget", "string"),
        ("desc", "string"),
        ("params", "string"),
        ("enums", "ParamEnumArray"),
        ("category", "string"),
        ("precision", "int"),
        ("enumType", "typename"),
        ("prefabbed", "bool"),
    ]
}

fn is_named_attribute_arg(value: &str) -> bool {
    attribute_param_specs()
        .iter()
        .any(|(name, _)| *name == value)
}

fn attribute_argument_values(attribute_text: &str) -> Vec<String> {
    let Some(open) = attribute_text.find('(') else {
        return Vec::new();
    };
    let Some(close) = attribute_text.rfind(')') else {
        return Vec::new();
    };
    if close <= open {
        return Vec::new();
    }
    split_top_level_arguments(&attribute_text[open + 1..close])
}

fn split_top_level_arguments(value: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut start = 0;
    let mut angle = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            '[' => bracket += 1,
            ']' => bracket = bracket.saturating_sub(1),
            '{' => brace += 1,
            '}' => brace = brace.saturating_sub(1),
            ',' if angle == 0 && paren == 0 && bracket == 0 && brace == 0 => {
                push_trimmed_arg(&mut args, &value[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    push_trimmed_arg(&mut args, &value[start..]);
    args
}

fn push_trimmed_arg(args: &mut Vec<String>, value: &str) {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
        args.push(trimmed.to_string());
    }
}

fn unquote_string(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

fn source_kind_label(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Unknown => "Unknown",
        SourceKind::Workspace => "Workspace",
        SourceKind::GameData => "GameData",
        SourceKind::Fixture => "Fixture",
    }
}

fn source_category_label(category: SourceCategory) -> &'static str {
    match category {
        SourceCategory::Workspace => "Workspace",
        SourceCategory::Game => "Game",
        SourceCategory::GameCode => "GameCode",
        SourceCategory::GameLib => "GameLib",
        SourceCategory::Core => "Core",
        SourceCategory::Generated => "Generated",
        SourceCategory::Workbench => "Workbench",
        SourceCategory::DocsDoxygen => "DocsDoxygen",
        SourceCategory::TestAutotest => "TestAutotest",
        SourceCategory::Unknown => "Unknown",
    }
}

fn escape_inline_code(value: &str) -> String {
    value.replace('`', "\\`")
}

fn escape_fence_text(value: &str) -> String {
    value.replace("```", "`\u{200b}``")
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::index::SymbolIndex;
    use crate::model::{
        SourceCategory, SourceFileMetadata, SourceKind, SymbolCatalog, SOURCE_PRIORITY_WORKSPACE,
    };
    use crate::parser::parse_source;

    #[test]
    fn renders_field_with_type_modifiers_and_attributes() {
        let index = index(
            r#"class Example
{
	[Attribute()]
	protected ref array<int> m_Values;
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::Field, "m_Values"))
            .unwrap();

        let markdown = render_hover_markdown(&display, Some(HoverRenderContext { query: &query }));

        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">property</span>"));
        assert!(markdown.contains("```enforce\nprotected ref array<int> m_Values\n```"));
        assert!(markdown.contains("**Detail:** `type ref array<int>`"));
        assert!(markdown.contains("**Modifiers:** protected"));
        assert!(markdown.contains("**Attributes:** Attribute"));
    }

    #[test]
    fn renders_callable_docs_with_params_returns_warnings_and_notes() {
        let index = index(
            r#"class Example
{
	/*!
	 * \brief Runs the example.
	 * \param[in] value Input value.
	 * \return true when accepted.
	 * \warning Can fail.
	 * \note Use from tests.
	 */
	bool Run(int value);
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::Method, "Run"))
            .unwrap();

        let markdown = render_hover_markdown(&display, Some(HoverRenderContext { query: &query }));

        assert!(markdown.contains("Example.Run(int value) -> bool"));
        assert!(markdown.contains("Runs the example."));
        assert!(markdown.contains("### Parameters"));
        assert!(markdown.contains("`value` [in]: Input value."));
        assert!(markdown.contains("### Returns"));
        assert!(markdown.contains("true when accepted."));
        assert!(markdown.contains("### Warning"));
        assert!(markdown.contains("Can fail."));
        assert!(markdown.contains("### Note"));
        assert!(markdown.contains("Use from tests."));
    }

    #[test]
    fn renders_class_with_base_and_bounded_member_summary() {
        let index = index(
            r#"class Example : Base
{
	void Example();
	void ~Example();
	void Run();
	void Stop();
	int m_Value;
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::Class, "Example"))
            .unwrap();

        let markdown = render_hover_markdown(&display, Some(HoverRenderContext { query: &query }));

        assert!(markdown.contains("<span style=\"color:#40b5ac;\">Class</span>"));
        assert!(markdown.contains("```enforce\nclass Example : Base\n```"));
        assert!(markdown.contains("**Detail:** `base Base`"));
        assert!(markdown.contains("### Constructors"));
        assert!(markdown.contains("### Destructors"));
        assert!(markdown.contains("### Functions"));
        assert!(markdown.contains("### Properties"));
    }

    #[test]
    fn renders_attribute_hover_params_and_constructor() {
        let index = index(
            r#"class Example
{
	[Attribute("0", uiwidget: UIWidgets.Flags, "Test flags.", "", ParamEnumArray.FromEnum(EGameFlags), WB_GAME_MODE_CATEGORY)]
	protected EGameFlags m_eTestGameFlags;
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::Field, "m_eTestGameFlags"))
            .unwrap();

        let markdown = render_hover_markdown(&display, Some(HoverRenderContext { query: &query }));

        assert!(markdown.contains("Test flags."));
        assert!(markdown.contains("### Params"));
        assert!(markdown.contains("string defvalue = \"0\""));
        assert!(markdown.contains("string uiwidget = UIWidgets.Flags"));
        assert!(markdown.contains("ParamEnumArray enums = ParamEnumArray.FromEnum(EGameFlags)"));
        assert!(markdown.contains("string category = WB_GAME_MODE_CATEGORY"));
        assert!(markdown.contains("### Constructor"));
        assert!(markdown.contains("void Attribute("));
        assert!(!markdown.contains("string desc = \"Test flags.\""));
    }

    #[test]
    fn renders_enum_members_section() {
        let index = index(
            r#"enum ExampleEnum
{
	First = 1,
	Second
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::Enum, "ExampleEnum"))
            .unwrap();

        let markdown = render_hover_markdown(&display, Some(HoverRenderContext { query: &query }));

        assert!(markdown.contains("### Members"));
        assert!(markdown.contains("First // value 1"));
        assert!(markdown.contains("Second"));
    }

    #[test]
    fn renders_constructor_destructor_typedef_enum_parameter_local_and_global() {
        let index = index(
            r#"typedef string Name;
int g_Value;
enum ExampleEnum
{
	Value = 1
}
class Example
{
	void Example(int value);
	void ~Example();
	void Run(int parameter)
	{
		int localValue = parameter;
	}
}
"#,
        );
        let query = IndexQuery::new(&index);
        for (kind, name, expected) in [
            (SymbolKind::Constructor, "Example", "Example(int value)"),
            (SymbolKind::Destructor, "Example", "~Example()"),
            (SymbolKind::Typedef, "Name", "**Detail:** `type string`"),
            (SymbolKind::EnumMember, "Value", "**Detail:** `value 1`"),
            (SymbolKind::Parameter, "parameter", "**Detail:** `type int`"),
            (
                SymbolKind::LocalVariable,
                "localValue",
                "**Detail:** `type int default parameter`",
            ),
            (SymbolKind::GlobalField, "g_Value", "**Detail:** `type int`"),
        ] {
            let display = query.symbol_display(find(&index, kind, name)).unwrap();
            let markdown =
                render_hover_markdown(&display, Some(HoverRenderContext { query: &query }));
            assert!(
                markdown.contains(expected),
                "missing {expected:?} in {markdown}"
            );
        }
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
                absolute_path: Some("C:/workspace/Example.c".into()),
                root_path: Some("C:/workspace".into()),
                relative_path: Some("Example.c".into()),
                priority: SOURCE_PRIORITY_WORKSPACE,
            },
        );
        SymbolIndex::from_catalogs([&catalog])
    }

    fn find(index: &SymbolIndex, kind: SymbolKind, name: &str) -> crate::index::GlobalSymbolId {
        index
            .symbols()
            .iter()
            .find(|symbol| symbol.kind == kind && symbol.name.as_deref() == Some(name))
            .map(|symbol| symbol.id)
            .unwrap_or_else(|| panic!("missing {kind:?} {name}"))
    }
}
