use crate::index_query::{EditorCompletionCandidate, EditorCompletionOrigin, IndexQuery};
use crate::lexer::{lex, TokenKind};
use crate::lsp::file_uri_for_path;
use crate::lsp::semantic_tokens::semantic_token_color_for_type;
use crate::lsp::symbol_kind_label;
use crate::model::SymbolKind;
use crate::symbol_display::{documentation_display, DocumentationDisplay, SymbolDisplayInfo};
use serde_json::json;

const OPEN_SYMBOL_LOCATION_COMMAND: &str = "reforger-sript-tools.openSymbolLocation";
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
    pub member_summary_query: Option<&'a IndexQuery<'index>>,
    pub links: Option<HoverLinkContext<'a, 'index>>,
}

#[derive(Clone, Copy)]
pub(crate) struct HoverLinkContext<'a, 'index> {
    pub current_uri: &'a str,
    pub external_query: Option<&'a IndexQuery<'index>>,
}

pub(crate) fn render_hover_markdown(
    display: &SymbolDisplayInfo,
    context: Option<HoverRenderContext<'_, '_>>,
) -> String {
    let mut markdown = String::new();
    markdown.push_str(&render_header(display, context.as_ref()));
    markdown.push('\n');
    markdown.push_str(&render_code(display, context.as_ref()));

    let mut sections = Vec::new();

    let docs = documentation_display(&display.doc_comments);
    let attribute_display = attribute_display(display);
    if let Some(documentation) = render_documentation(&docs, attribute_display.as_ref()) {
        sections.push(documentation);
    }

    if let Some(attribute_display) = &attribute_display {
        if let Some(params) = render_attribute_params(attribute_display) {
            sections.push(params);
        }
        sections.push(render_attribute_constructor(context.as_ref()));
    }

    if let Some(detail) = render_detail(display) {
        sections.push(detail);
    }

    if let Some(context) = context.as_ref() {
        if display.kind == SymbolKind::Class {
            let member_query = context.member_summary_query.unwrap_or(context.query);
            if let Some(summary) =
                render_class_members(display, context.query, member_query, Some(context))
            {
                sections.push(summary);
            }
        } else if display.kind == SymbolKind::Enum {
            if let Some(summary) = render_enum_members(display, context.query, Some(context)) {
                sections.push(summary);
            }
        }
    }

    if let Some(metadata) = render_metadata(display) {
        sections.push(metadata);
    }

    if !sections.is_empty() {
        markdown.push_str("\n\n");
        markdown.push_str(&sections.join("\n\n"));
    }

    markdown
}

fn render_header(
    display: &SymbolDisplayInfo,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let kind = hover_kind_label(display.kind);
    if let Some(container) = hover_container_name(display) {
        return format!(
            "<span style=\"font-size:1.12em;\"><strong>{}</strong> in <strong>{}</strong></span>",
            colored_text(kind, hover_header_token_type(display.kind)),
            render_type_identifier(&container, "class", context)
        );
    }
    let header = colored_text(kind, hover_header_token_type(display.kind));
    format!("<strong><span style=\"font-size:1.12em;\">{header}</span></strong>")
}

fn render_code(
    display: &SymbolDisplayInfo,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let declaration = hover_declaration_text(display);
    format!(
        "<div data-code=\"{}\">{}</div>",
        escape_html_attr(&declaration),
        render_colored_declaration(display, &declaration, context)
    )
}

fn render_detail(display: &SymbolDisplayInfo) -> Option<String> {
    let detail = display.detail.as_ref()?;
    if hover_declaration_text(display) != display.label {
        return None;
    }
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
        for parameter in &docs.parameters {
            let tag = parameter.direction.as_deref().unwrap_or("param");
            if parameter.description.is_empty() {
                lines.push(format!(
                    "{} {}",
                    render_doc_tag(&tag),
                    escape_html_text(&parameter.name)
                ));
            } else {
                lines.push(format!(
                    "{} {} {}",
                    render_doc_tag(&tag),
                    escape_html_text(&parameter.name),
                    parameter.description
                ));
            }
        }
    }
    if let Some(returns) = &docs.returns {
        lines.push(format!("{} {returns}", render_doc_tag("return")));
    }
    for warning in &docs.warnings {
        lines.push(format!("{} {warning}", render_doc_tag("warning")));
    }
    for note in &docs.notes {
        lines.push(format!("{} {note}", render_doc_tag("note")));
    }

    Some(lines.join("\n\n"))
}

fn render_class_members(
    display: &SymbolDisplayInfo,
    direct_query: &IndexQuery<'_>,
    member_query: &IndexQuery<'_>,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> Option<String> {
    let direct_lookup = direct_query.completion_members_for_class(&display.label);
    let member_lookup = member_query.completion_members_for_class(&display.label);
    if direct_lookup.candidates.is_empty() && member_lookup.candidates.is_empty() {
        return None;
    }
    let direct_members = direct_lookup
        .candidates
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.origin,
                EditorCompletionOrigin::Direct | EditorCompletionOrigin::Overlay
            ) && is_public_facing_class_summary_member(candidate)
        })
        .cloned()
        .collect::<Vec<_>>();
    let inherited_members = member_lookup
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.origin == EditorCompletionOrigin::Inherited
                && is_public_facing_class_summary_member(candidate)
        })
        .cloned()
        .collect::<Vec<_>>();
    let constructors = members_for_kind(&direct_members, SymbolKind::Constructor);
    let methods = direct_members
        .iter()
        .chain(inherited_members.iter())
        .filter(|candidate| candidate.kind == SymbolKind::Method)
        .collect::<Vec<_>>();
    let fields = direct_members
        .iter()
        .chain(inherited_members.iter())
        .filter(|candidate| candidate.kind == SymbolKind::Field)
        .collect::<Vec<_>>();

    let mut lines = Vec::new();
    push_member_group(&mut lines, "Constructors", &constructors, context);
    push_member_group(&mut lines, "Functions", &methods, context);
    push_member_group(&mut lines, "Fields", &fields, context);
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

fn is_public_facing_class_summary_member(candidate: &EditorCompletionCandidate) -> bool {
    matches!(
        candidate.kind,
        SymbolKind::Constructor | SymbolKind::Method | SymbolKind::Field
    ) && !candidate
        .display
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier.as_str(), "private" | "protected"))
}

fn push_member_group(
    lines: &mut Vec<String>,
    label: &str,
    members: &[&EditorCompletionCandidate],
    context: Option<&HoverRenderContext<'_, '_>>,
) {
    if members.is_empty() {
        return;
    }
    let rendered_members = members
        .iter()
        .map(|candidate| render_member_line(candidate, context))
        .collect::<Vec<_>>()
        .join("<br>");
    let block = format!("{}\n\n{rendered_members}", render_section_header(label));
    lines.push(block);
}

fn render_enum_members(
    display: &SymbolDisplayInfo,
    query: &IndexQuery<'_>,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> Option<String> {
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
        .map(|candidate| {
            let mut rendered = linked_symbol_text(
                &candidate.display.label,
                hover_token_type(candidate.display.kind),
                &candidate.display,
                context,
            );
            if let Some(detail) = &candidate.display.detail {
                rendered.push_str(" // ");
                rendered.push_str(&escape_html_text(detail));
            }
            rendered
        })
        .collect::<Vec<_>>()
        .join("<br>");
    Some(format!(
        "{}\n\n{sample}",
        render_section_header("Enum Values")
    ))
}

fn render_metadata(display: &SymbolDisplayInfo) -> Option<String> {
    let mut parts = Vec::new();

    let attribute_names = display
        .attributes
        .iter()
        .filter(|attribute| attribute.name.as_deref() != Some("Attribute"))
        .map(|attribute| {
            attribute
                .name
                .as_deref()
                .unwrap_or(attribute.text.as_str())
                .to_string()
        })
        .collect::<Vec<_>>();
    if !attribute_names.is_empty() {
        parts.push(format!("Attributes: {}", attribute_names.join(", ")));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

fn hover_token_type(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class | SymbolKind::Typedef | SymbolKind::TypeParameter => "class",
        SymbolKind::Enum => "enum",
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Constructor | SymbolKind::Destructor => "class",
        SymbolKind::GlobalField | SymbolKind::Field => "field",
        SymbolKind::Parameter => "parameter",
        SymbolKind::EnumMember | SymbolKind::LocalVariable | SymbolKind::PreprocessorMacro => {
            "variable"
        }
    }
}

fn hover_header_token_type(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class | SymbolKind::Enum | SymbolKind::Constructor => "keyword",
        _ => hover_token_type(kind),
    }
}

fn hover_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Field | SymbolKind::GlobalField => "field",
        SymbolKind::Method | SymbolKind::Function => "Function",
        SymbolKind::LocalVariable => "variable",
        SymbolKind::EnumMember => "Enum Value",
        SymbolKind::PreprocessorMacro => "Preprocessor Macro",
        _ => symbol_kind_label(kind),
    }
}

fn hover_container_name(display: &SymbolDisplayInfo) -> Option<String> {
    if display.kind == SymbolKind::EnumMember {
        return display.container_name.clone();
    }
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
    if display.signature.is_some() {
        return callable_declaration_text(display);
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

fn render_colored_declaration(
    display: &SymbolDisplayInfo,
    declaration: &str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    if display.signature.is_some() {
        return render_colored_callable_declaration(display, declaration, context);
    }
    render_colored_plain_declaration(display, declaration, context)
}

fn render_colored_callable_declaration(
    display: &SymbolDisplayInfo,
    declaration: &str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let Some(name_start) = find_identifier_start(declaration, &display.label) else {
        return render_colored_plain_declaration(display, declaration, context);
    };
    let Some(open_offset) = declaration[name_start..].find('(') else {
        return render_colored_plain_declaration(display, declaration, context);
    };
    let open = name_start + open_offset;
    let prefix = declaration[..name_start].trim_end();
    let params_and_suffix = &declaration[open..];
    let return_type_start = prefix
        .rfind(char::is_whitespace)
        .map_or(0, |index| index + 1);
    let modifiers = prefix[..return_type_start].trim();
    let return_type = prefix[return_type_start..].trim();

    let mut parts = Vec::new();
    if !modifiers.is_empty() {
        parts.push(color_words(modifiers, "keyword"));
    }
    if !return_type.is_empty() {
        parts.push(render_type_text(return_type, context));
    }
    parts.push(linked_symbol_text(
        &display.label,
        hover_token_type(display.kind),
        display,
        context,
    ));
    let mut rendered = parts.join(" ");
    rendered.push_str(&render_callable_params(params_and_suffix, context));
    rendered
}

fn render_callable_params(
    params_and_suffix: &str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let Some(open) = params_and_suffix.find('(') else {
        return escape_html_text(params_and_suffix);
    };
    let Some(close) = params_and_suffix.rfind(')') else {
        return escape_html_text(params_and_suffix);
    };
    let before = &params_and_suffix[..=open];
    let params = &params_and_suffix[open + 1..close];
    let after = &params_and_suffix[close..];
    let mut rendered = String::new();
    rendered.push_str(&colored_text(before, "operator"));
    let rendered_params = split_top_level_arguments(params)
        .into_iter()
        .map(|param| render_parameter_declaration(&param, context))
        .collect::<Vec<_>>()
        .join(&format!("{} ", colored_text(",", "operator")));
    rendered.push_str(&rendered_params);
    rendered.push_str(&colored_text(after, "operator"));
    rendered
}

fn render_parameter_declaration(
    param: &str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let trimmed = param.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let (head, default) = match trimmed.split_once(" = ") {
        Some((head, default)) => (head.trim(), Some(default.trim())),
        None => (trimmed, None),
    };
    let tokens = head.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return escape_html_text(trimmed);
    }
    if tokens.len() == 1 {
        return render_type_text(tokens[0], context);
    }
    let name = tokens[tokens.len() - 1];
    let mut rendered = Vec::new();
    for token in &tokens[..tokens.len() - 1] {
        let token_type = if is_parameter_modifier(token) {
            "keyword"
        } else {
            hover_type_token_type(token)
        };
        if is_parameter_modifier(token) {
            rendered.push(colored_text(token, token_type));
        } else {
            rendered.push(render_type_text(token, context));
        }
    }
    rendered.push(colored_text(name, "parameter"));
    let mut value = rendered.join(" ");
    if let Some(default) = default {
        value.push(' ');
        value.push_str(&colored_text("=", "operator"));
        value.push(' ');
        value.push_str(&escape_html_text(default));
    }
    value
}

fn render_type_text(type_text: &str, context: Option<&HoverRenderContext<'_, '_>>) -> String {
    let mut rendered = String::new();
    let mut cursor = 0usize;
    for token in lex(type_text) {
        if token.kind == TokenKind::Eof {
            continue;
        }
        if token.span.start > cursor {
            rendered.push_str(&escape_html_text(&type_text[cursor..token.span.start]));
        }
        let text = &type_text[token.span.start..token.span.end];
        match token.kind {
            TokenKind::Identifier => {
                rendered.push_str(&render_type_identifier(text, "class", context));
            }
            TokenKind::Keyword(_) => {
                let token_type = hover_type_token_type(text);
                if token_type == "class" {
                    rendered.push_str(&render_type_identifier(text, token_type, context));
                } else {
                    rendered.push_str(&colored_text(text, token_type));
                }
            }
            TokenKind::Operator(_) | TokenKind::LeftBracket | TokenKind::RightBracket => {
                rendered.push_str(&colored_text(text, "operator"));
            }
            TokenKind::Comma | TokenKind::Colon | TokenKind::Dot => {
                rendered.push_str(&colored_text(text, "operator"));
            }
            _ => rendered.push_str(&escape_html_text(text)),
        }
        cursor = token.span.end;
    }
    if cursor < type_text.len() {
        rendered.push_str(&escape_html_text(&type_text[cursor..]));
    }
    rendered
}

fn render_type_identifier(
    text: &str,
    token_type: &str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    if let Some(display) = context.and_then(|context| find_type_display(context, text)) {
        return linked_symbol_text(text, token_type, &display, context);
    }
    colored_text(text, token_type)
}

fn find_type_display(
    context: &HoverRenderContext<'_, '_>,
    name: &str,
) -> Option<SymbolDisplayInfo> {
    find_type_display_in_query(context.query, name).or_else(|| {
        context
            .links
            .and_then(|links| links.external_query)
            .and_then(|query| find_type_display_in_query(query, name))
    })
}

fn find_type_display_in_query(query: &IndexQuery<'_>, name: &str) -> Option<SymbolDisplayInfo> {
    query
        .top_level_conflicts(name)
        .into_iter()
        .filter_map(|id| query.symbol_display(id))
        .find(|display| {
            matches!(
                display.kind,
                SymbolKind::Class
                    | SymbolKind::Enum
                    | SymbolKind::Typedef
                    | SymbolKind::TypeParameter
            )
        })
}

fn linked_symbol_text(
    text: &str,
    token_type: &str,
    display: &SymbolDisplayInfo,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let label = colored_text(text, token_type);
    let Some(command_uri) = context
        .and_then(|context| context.links)
        .and_then(|links| hover_command_uri_for_display(display, links))
    else {
        return label;
    };
    format!("<a href=\"{}\">{label}</a>", escape_html_attr(&command_uri))
}

fn hover_command_uri_for_display(
    display: &SymbolDisplayInfo,
    links: HoverLinkContext<'_, '_>,
) -> Option<String> {
    if display.selection_span.is_empty() {
        return None;
    }
    let target_uri = display
        .absolute_path
        .as_deref()
        .and_then(file_uri_for_path)
        .unwrap_or_else(|| links.current_uri.to_string());
    let args = json!([{
        "uri": target_uri,
        "startByte": display.selection_span.start,
        "endByte": display.selection_span.end,
    }]);
    Some(format!(
        "command:{}?{}",
        OPEN_SYMBOL_LOCATION_COMMAND,
        percent_encode_command_arg(&args.to_string())
    ))
}

fn render_colored_plain_declaration(
    display: &SymbolDisplayInfo,
    declaration: &str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    match display.kind {
        SymbolKind::Class => {
            render_keyword_name_declaration(display, declaration, "class", "class", context)
        }
        SymbolKind::Enum => {
            render_keyword_name_declaration(display, declaration, "enum", "class", context)
        }
        SymbolKind::Typedef => render_typedef_declaration(display, declaration, context),
        SymbolKind::Field
        | SymbolKind::GlobalField
        | SymbolKind::LocalVariable
        | SymbolKind::Parameter => render_typed_name_declaration(display, declaration, context),
        SymbolKind::EnumMember => render_name_first_declaration(display, declaration, context),
        SymbolKind::PreprocessorMacro => {
            render_preprocessor_macro_declaration(display, declaration, context)
        }
        _ => escape_html_text(declaration),
    }
}

fn render_preprocessor_macro_declaration(
    display: &SymbolDisplayInfo,
    declaration: &str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let Some(name) = declaration.strip_prefix("#define ") else {
        return escape_html_text(declaration);
    };
    format!(
        "{} {}",
        colored_text("#define", "preprocessor"),
        linked_symbol_text(name, hover_token_type(display.kind), display, context)
    )
}

fn render_keyword_name_declaration(
    display: &SymbolDisplayInfo,
    declaration: &str,
    keyword: &'static str,
    name_token_type: &'static str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let Some(rest) = declaration.strip_prefix(keyword) else {
        return escape_html_text(declaration);
    };
    let rest = rest.trim_start();
    let mut split = rest.splitn(2, char::is_whitespace);
    let name_and_suffix = split.next().unwrap_or_default();
    let suffix = split.next().unwrap_or_default();
    let (name, inline_suffix) = name_and_suffix
        .split_once(':')
        .map_or((name_and_suffix, ""), |(name, suffix)| (name, suffix));
    let mut rendered = format!(
        "{} {}",
        colored_text(keyword, "keyword"),
        linked_symbol_text(name, name_token_type, display, context)
    );
    if !inline_suffix.is_empty() {
        rendered.push_str(&colored_text(":", "operator"));
        rendered.push(' ');
        rendered.push_str(&render_type_identifier(
            inline_suffix.trim(),
            "class",
            context,
        ));
    } else if name_and_suffix.ends_with(':') {
        rendered.push_str(&colored_text(":", "operator"));
    }
    if !suffix.is_empty() {
        let suffix = suffix.trim();
        if let Some(base) = suffix.strip_prefix(':') {
            rendered.push(' ');
            rendered.push_str(&colored_text(":", "operator"));
            let base = base.trim();
            if !base.is_empty() {
                rendered.push(' ');
                rendered.push_str(&render_type_identifier(base, "class", context));
            }
        } else {
            rendered.push(' ');
            rendered.push_str(&render_type_identifier(suffix, "class", context));
        }
    }
    rendered
}

fn render_typedef_declaration(
    display: &SymbolDisplayInfo,
    declaration: &str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let Some(rest) = declaration.strip_prefix("typedef") else {
        return escape_html_text(declaration);
    };
    let rest = rest.trim();
    let Some((target, name)) = rest.rsplit_once(char::is_whitespace) else {
        return escape_html_text(declaration);
    };
    format!(
        "{} {} {}",
        colored_text("typedef", "keyword"),
        render_type_text(target.trim(), context),
        linked_symbol_text(name.trim(), "class", display, context)
    )
}

fn render_typed_name_declaration(
    display: &SymbolDisplayInfo,
    declaration: &str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let Some(name_start) = find_identifier_start(declaration, &display.label) else {
        return escape_html_text(declaration);
    };
    let prefix = declaration[..name_start].trim_end();
    let suffix = declaration[name_start + display.label.len()..].trim_start();
    let return_type_start = prefix
        .rfind(char::is_whitespace)
        .map_or(0, |index| index + 1);
    let modifiers = prefix[..return_type_start].trim();
    let type_text = prefix[return_type_start..].trim();
    let mut parts = Vec::new();
    if !modifiers.is_empty() {
        parts.push(color_words(modifiers, "keyword"));
    }
    if !type_text.is_empty() {
        parts.push(render_type_text(type_text, context));
    }
    parts.push(linked_symbol_text(
        &display.label,
        hover_token_type(display.kind),
        display,
        context,
    ));
    let mut rendered = parts.join(" ");
    if !suffix.is_empty() {
        rendered.push(' ');
        rendered.push_str(&escape_html_text(suffix));
    }
    rendered
}

fn render_name_first_declaration(
    display: &SymbolDisplayInfo,
    declaration: &str,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let Some(name_start) = find_identifier_start(declaration, &display.label) else {
        return escape_html_text(declaration);
    };
    let mut rendered = String::new();
    rendered.push_str(&escape_html_text(&declaration[..name_start]));
    rendered.push_str(&linked_symbol_text(
        &display.label,
        hover_token_type(display.kind),
        display,
        context,
    ));
    rendered.push_str(&escape_html_text(
        &declaration[name_start + display.label.len()..],
    ));
    rendered
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

fn render_member_line(
    candidate: &&EditorCompletionCandidate,
    context: Option<&HoverRenderContext<'_, '_>>,
) -> String {
    let declaration = hover_declaration_text(&candidate.display);
    render_colored_declaration(&candidate.display, &declaration, context)
}

fn callable_declaration_text(display: &SymbolDisplayInfo) -> String {
    let Some(signature) = display.signature.as_deref() else {
        return display.label.clone();
    };
    let modifiers = hover_visible_modifiers(&display.modifiers);
    let modifiers = modifiers
        .is_empty()
        .then(String::new)
        .unwrap_or_else(|| format!("{} ", modifiers.join(" ")));
    match display.kind {
        SymbolKind::Method => method_declaration_text(display, signature, &modifiers),
        SymbolKind::Function => function_declaration_text(display, signature, &modifiers),
        SymbolKind::Constructor => constructor_declaration_text(display, signature, &modifiers),
        SymbolKind::Destructor => destructor_declaration_text(display, signature, &modifiers),
        _ => signature.trim_end_matches(';').to_string(),
    }
}

fn hover_visible_modifiers(modifiers: &[String]) -> Vec<&str> {
    modifiers
        .iter()
        .map(String::as_str)
        .filter(|modifier| !matches!(*modifier, "override" | "proto" | "external" | "event"))
        .collect()
}

fn method_declaration_text(
    display: &SymbolDisplayInfo,
    signature: &str,
    modifiers: &str,
) -> String {
    let Some((head, return_type)) = signature.split_once(" -> ") else {
        return signature.trim_end_matches(';').to_string();
    };
    let Some(open) = head.find('(') else {
        return signature.trim_end_matches(';').to_string();
    };
    let Some((_, name)) = head[..open].rsplit_once('.') else {
        return signature.trim_end_matches(';').to_string();
    };
    if name != display.label {
        return signature.trim_end_matches(';').to_string();
    }
    format!("{modifiers}{return_type} {name}{}", &head[open..])
}

fn function_declaration_text(
    display: &SymbolDisplayInfo,
    signature: &str,
    modifiers: &str,
) -> String {
    let Some((head, return_type)) = signature.split_once(" -> ") else {
        return signature.trim_end_matches(';').to_string();
    };
    if !head.starts_with(&display.label) {
        return signature.trim_end_matches(';').to_string();
    }
    format!("{modifiers}{return_type} {head}")
}

fn constructor_declaration_text(
    display: &SymbolDisplayInfo,
    signature: &str,
    modifiers: &str,
) -> String {
    let Some(open) = signature.find('(') else {
        return signature.trim_end_matches(';').to_string();
    };
    let params = &signature[open..];
    format!("{modifiers}void {}{params}", display.label)
}

fn destructor_declaration_text(
    display: &SymbolDisplayInfo,
    signature: &str,
    modifiers: &str,
) -> String {
    let Some(open) = signature.find('(') else {
        return signature.trim_end_matches(';').to_string();
    };
    let params = &signature[open..];
    format!("{modifiers}void ~{}{}", display.label, params)
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
    Some(format!(
        "{}\n\n```enforce\n{lines}\n```",
        render_section_header("Params")
    ))
}

fn render_attribute_constructor(context: Option<&HoverRenderContext<'_, '_>>) -> String {
    let rendered = render_attribute_constructor_signature(context);
    format!(
        "{}\n\n{}",
        render_colored_section_header("Constructor", "keyword"),
        rendered
    )
}

fn render_attribute_constructor_signature(context: Option<&HoverRenderContext<'_, '_>>) -> String {
    let Some(open) = ATTRIBUTE_CONSTRUCTOR_SIGNATURE.find('(') else {
        return escape_html_text(ATTRIBUTE_CONSTRUCTOR_SIGNATURE);
    };
    let prefix = ATTRIBUTE_CONSTRUCTOR_SIGNATURE[..open].trim();
    let params_and_suffix = &ATTRIBUTE_CONSTRUCTOR_SIGNATURE[open..];
    let mut tokens = prefix.split_whitespace();
    let return_type = tokens.next().unwrap_or_default();
    let name = tokens.next().unwrap_or_default();
    if return_type.is_empty() || name.is_empty() {
        return escape_html_text(ATTRIBUTE_CONSTRUCTOR_SIGNATURE);
    }
    format!(
        "{} {}{}",
        render_type_text(return_type, context),
        render_type_identifier(name, "class", context),
        render_callable_params(params_and_suffix, context)
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

fn render_doc_tag(tag: &str) -> String {
    format!("`{}`", escape_inline_code(tag))
}

fn render_section_header(label: &str) -> String {
    format!("### {}", escape_html_text(label))
}

fn render_colored_section_header(label: &str, token_type: &str) -> String {
    format!("### {}", colored_text(label, token_type))
}

fn find_identifier_start(value: &str, ident: &str) -> Option<usize> {
    if ident.is_empty() {
        return None;
    }
    value.match_indices(ident).find_map(|(index, _)| {
        let before = value[..index].chars().next_back();
        let after = value[index + ident.len()..].chars().next();
        if before.is_none_or(|ch| !is_identifier_char(ch))
            && after.is_none_or(|ch| !is_identifier_char(ch))
        {
            Some(index)
        } else {
            None
        }
    })
}

fn is_identifier_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn color_words(value: &str, token_type: &str) -> String {
    value
        .split_whitespace()
        .map(|word| colored_text(word, token_type))
        .collect::<Vec<_>>()
        .join(" ")
}

fn hover_type_token_type(type_text: &str) -> &'static str {
    let head = type_text
        .trim()
        .split(|ch: char| ch == '<' || ch == '[' || ch.is_whitespace())
        .next()
        .unwrap_or_default();
    if matches!(head, "void" | "bool" | "int" | "float" | "typename") {
        "keyword"
    } else {
        "class"
    }
}

fn is_parameter_modifier(value: &str) -> bool {
    matches!(value, "out" | "inout" | "notnull" | "const")
}

fn colored_text(value: &str, token_type: &str) -> String {
    let color = semantic_token_color_for_type(token_type);
    if color == "<default>" {
        escape_html_text(value)
    } else {
        format!(
            "<span style=\"color:{color};\">{}</span>",
            escape_html_text(value)
        )
    }
}

fn escape_inline_code(value: &str) -> String {
    value.replace('`', "\\`")
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_attr(value: &str) -> String {
    escape_html_text(value).replace('"', "&quot;")
}

fn percent_encode_command_arg(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
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

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: None,
            }),
        );

        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">field</span>"));
        assert!(
            markdown.contains("<strong><span style=\"font-size:1.12em;\"><span style=\"color:#cfcfcf;\">field</span></span></strong>")
        );
        assert!(markdown.contains("data-code=\"protected ref array&lt;int&gt; m_Values\""));
        assert!(markdown.contains("<span style=\"color:#59A6E9;\">protected</span>"));
        assert!(markdown.contains("<span style=\"color:#59A6E9;\">ref</span>"));
        assert!(markdown.contains("<span style=\"color:#40b5ac;\">array</span>"));
        assert!(markdown.contains("<span style=\"color:#bfbfbf;\">&lt;</span>"));
        assert!(markdown.contains("<span style=\"color:#59A6E9;\">int</span>"));
        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">m_Values</span>"));
        assert!(!markdown.contains("**Detail:** `type ref array<int>`"));
        assert!(!markdown.contains("Modifiers: protected"));
        assert!(!markdown.contains("Attributes: Attribute"));
    }

    #[test]
    fn renders_preprocessor_macro_with_directive_coloring() {
        let index = index("#define ENABLE_BASE_DESTRUCTION\n");
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(
                &index,
                SymbolKind::PreprocessorMacro,
                "ENABLE_BASE_DESTRUCTION",
            ))
            .unwrap();

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: None,
            }),
        );

        assert!(markdown.contains("Preprocessor Macro"));
        assert!(markdown.contains("data-code=\"#define ENABLE_BASE_DESTRUCTION\""));
        assert!(markdown.contains("<span style=\"color:#d4fd95;\">#define</span>"));
        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">ENABLE_BASE_DESTRUCTION</span>"));
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

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: None,
            }),
        );

        assert!(markdown.contains(
            "<span style=\"font-size:1.12em;\"><strong><span style=\"color:#f3ad58;\">Function</span></strong> in <strong><span style=\"color:#40b5ac;\">Example</span></strong></span>\n<div"
        ));
        assert!(markdown.contains("data-code=\"bool Run(int value)\""));
        assert!(markdown.contains("<span style=\"color:#59A6E9;\">bool</span>"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">Run</span>"));
        assert!(markdown.contains("Runs the example."));
        assert!(!markdown.contains("### Parameters"));
        assert!(markdown.contains("`in` value Input value."));
        assert!(!markdown.contains("`in` `value`"));
        assert!(!markdown.contains("`param[in]`"));
        assert!(markdown.contains("`return` true when accepted."));
        assert!(markdown.contains("`warning` Can fail."));
        assert!(markdown.contains("`note` Use from tests."));
        assert!(!markdown.contains("<span style=\"color:#59A6E9;\">return</span>"));
    }

    #[test]
    fn renders_class_with_base_and_bounded_member_summary() {
        let index = index(
            r#"class Example : Base
{
	void Example();
	void ~Example();
	void Run();
	protected void InternalRun();
	private void PrivateRun();
	void Stop();
	override void OnGameStateChanged(Example state);
	proto external event void EngineEvent();
	Example GetAffiliatedState();
	int m_Value;
	protected int m_InternalValue;
	private int m_PrivateValue;
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::Class, "Example"))
            .unwrap();

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: None,
            }),
        );

        assert!(markdown.contains("<span style=\"color:#59A6E9;\">Class</span>"));
        assert!(markdown.contains("data-code=\"class Example : Base\""));
        assert!(markdown.contains("<span style=\"color:#59A6E9;\">class</span>"));
        assert!(markdown.contains("<span style=\"color:#40b5ac;\">Example</span>"));
        assert!(markdown.contains("<span style=\"color:#bfbfbf;\">:</span>"));
        assert!(markdown.contains("<span style=\"color:#40b5ac;\">Base</span>"));
        assert!(!markdown.contains("**Detail:** `base Base`"));
        assert!(markdown.contains("### Constructors"));
        assert!(!markdown.contains("### Destructors"));
        assert!(!markdown.contains("~Example"));
        assert!(markdown.contains("### Functions"));
        assert!(markdown.contains("### Fields"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">Run</span>"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">Stop</span>"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">OnGameStateChanged</span>"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">EngineEvent</span>"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">GetAffiliatedState</span>"));
        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">m_Value</span>"));
        assert!(!markdown.contains("InternalRun"));
        assert!(!markdown.contains("PrivateRun"));
        assert!(!markdown.contains("m_InternalValue"));
        assert!(!markdown.contains("m_PrivateValue"));
        assert!(!markdown.contains("override"));
        assert!(!markdown.contains("proto"));
        assert!(!markdown.contains("external"));
        assert!(!markdown.contains("event"));
        assert!(!markdown.contains("+"));
    }

    #[test]
    fn renders_source_backed_command_links_for_class_and_type_targets() {
        let index = index(
            r#"class Base {}

class Example : Base
{
	Base Make(Base value);
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::Class, "Example"))
            .unwrap();

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: Some(HoverLinkContext {
                    current_uri: "file:///current.c",
                    external_query: None,
                }),
            }),
        );

        assert!(markdown.contains("command:reforger-sript-tools.openSymbolLocation?"));
        assert!(markdown.contains("<a href=\"command:reforger-sript-tools.openSymbolLocation?"));
        assert!(!markdown.contains("[<span"));
        assert!(!markdown.contains("](command:reforger-sript-tools.openSymbolLocation?"));
        assert!(markdown.contains("%22uri%22"));
        assert!(markdown.contains("%22startByte%22"));
        assert!(markdown.contains("%22endByte%22"));
        assert!(markdown.contains("<span style=\"color:#40b5ac;\">Base</span>"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">Make</span>"));
    }

    #[test]
    fn class_summary_shows_only_public_facing_direct_and_inherited_members() {
        let index = index(
            r#"class Base
{
	void BaseRun();
	protected void ProtectedRun();
	private void PrivateRun();
	int m_BaseValue;
	protected int m_ProtectedBaseValue;
}

class Child : Base
{
	void ChildRun();
	protected void ProtectedChildRun();
	private void PrivateChildRun();
	int m_ChildValue;
	protected int m_ProtectedChildValue;
	private int m_PrivateChildValue;
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::Class, "Child"))
            .unwrap();

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: None,
            }),
        );
        let functions = markdown.find("### Functions").unwrap();
        let child_run = markdown.find("ChildRun").unwrap();
        let base_run = markdown.find("BaseRun").unwrap();
        let fields = markdown.find("### Fields").unwrap();

        assert!(child_run > functions);
        assert!(base_run > child_run);
        assert!(fields > base_run);
        assert!(!markdown.contains("### Inherited members"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">ChildRun</span>"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">BaseRun</span>"));
        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">m_ChildValue</span>"));
        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">m_BaseValue</span>"));
        assert!(!markdown.contains("inherited from"));
        assert!(!markdown.contains("ProtectedRun"));
        assert!(!markdown.contains("PrivateRun"));
        assert!(!markdown.contains("m_ProtectedBaseValue"));
        assert!(!markdown.contains("ProtectedChildRun"));
        assert!(!markdown.contains("PrivateChildRun"));
        assert!(!markdown.contains("m_ProtectedChildValue"));
        assert!(!markdown.contains("m_PrivateChildValue"));
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

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: None,
            }),
        );

        assert!(markdown.contains("Test flags."));
        assert!(markdown.contains("### Params"));
        assert!(markdown.contains("string defvalue = \"0\""));
        assert!(markdown.contains("string uiwidget = UIWidgets.Flags"));
        assert!(markdown.contains("ParamEnumArray enums = ParamEnumArray.FromEnum(EGameFlags)"));
        assert!(markdown.contains("string category = WB_GAME_MODE_CATEGORY"));
        assert!(markdown.contains("### <span style=\"color:#59A6E9;\">Constructor</span>"));
        assert!(markdown.contains(
            "<span style=\"color:#59A6E9;\">void</span> <span style=\"color:#40b5ac;\">Attribute</span><span style=\"color:#bfbfbf;\">(</span>"
        ));
        assert!(!markdown.contains("string desc = \"Test flags.\""));
    }

    #[test]
    fn renders_enum_members_section() {
        let index = index(
            r#"enum ExampleEnum
{
	First = 1,
	Second,
	Third,
	Fourth,
	Fifth
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::Enum, "ExampleEnum"))
            .unwrap();

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: None,
            }),
        );

        assert!(markdown.contains(
            "<strong><span style=\"font-size:1.12em;\"><span style=\"color:#59A6E9;\">Enum</span></span></strong>"
        ));
        assert!(markdown.contains("data-code=\"enum ExampleEnum\""));
        assert!(markdown.contains("<span style=\"color:#59A6E9;\">enum</span>"));
        assert!(markdown.contains("<span style=\"color:#40b5ac;\">ExampleEnum</span>"));
        assert!(markdown.contains("### Enum Values"));
        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">First</span> // value 1"));
        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">Second</span>"));
        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">Fifth</span>"));
        assert!(!markdown.contains("// +"));
    }

    #[test]
    fn renders_source_backed_command_links_for_enum_values() {
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

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: Some(HoverLinkContext {
                    current_uri: "file:///current.c",
                    external_query: None,
                }),
            }),
        );

        assert!(markdown.contains("### Enum Values"));
        assert!(markdown.contains("<a href=\"command:reforger-sript-tools.openSymbolLocation?"));
        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">First</span>"));
        assert!(markdown.contains(" // value 1"));
        assert!(!markdown.contains("```enforce"));
        assert!(!markdown.contains("[<span"));
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
            (
                SymbolKind::Constructor,
                "Example",
                "data-code=\"void Example(int value)\"",
            ),
            (
                SymbolKind::Destructor,
                "Example",
                "data-code=\"void ~Example()\"",
            ),
            (
                SymbolKind::Typedef,
                "Name",
                "data-code=\"typedef string Name\"",
            ),
            (SymbolKind::EnumMember, "Value", "data-code=\"Value = 1\""),
            (
                SymbolKind::Parameter,
                "parameter",
                "data-code=\"int parameter\"",
            ),
            (
                SymbolKind::LocalVariable,
                "localValue",
                "data-code=\"int localValue = parameter\"",
            ),
            (
                SymbolKind::GlobalField,
                "g_Value",
                "data-code=\"int g_Value\"",
            ),
        ] {
            let display = query.symbol_display(find(&index, kind, name)).unwrap();
            let markdown = render_hover_markdown(
                &display,
                Some(HoverRenderContext {
                    query: &query,
                    member_summary_query: None,
                    links: None,
                }),
            );
            assert!(
                markdown.contains(expected),
                "missing {expected:?} in {markdown}"
            );
            if kind == SymbolKind::Destructor {
                assert!(
                    markdown.contains("<span style=\"color:#40b5ac;\">Destructor</span>"),
                    "{markdown}"
                );
                assert!(
                    markdown.contains("<span style=\"color:#40b5ac;\">Example</span>"),
                    "{markdown}"
                );
            }
        }
    }

    #[test]
    fn renders_enum_member_with_owner_header() {
        let index = index(
            r#"enum SCR_EGameModeState
{
	PREGAME = 0,
	GAME,
	POSTGAME
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::EnumMember, "PREGAME"))
            .unwrap();

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: None,
            }),
        );

        assert!(markdown.contains("<span style=\"font-size:1.12em;\"><strong><span style=\"color:#cfcfcf;\">Enum Value</span></strong> in <strong><span style=\"color:#40b5ac;\">SCR_EGameModeState</span></strong></span>"));
        assert!(markdown.contains("data-code=\"PREGAME = 0\""));
    }

    #[test]
    fn renders_single_letter_names_without_matching_type_text() {
        let index = index(
            r#"class Example
{
	void Run()
	{
		int i = 0;
	}
}
"#,
        );
        let query = IndexQuery::new(&index);
        let display = query
            .symbol_display(find(&index, SymbolKind::LocalVariable, "i"))
            .unwrap();

        let markdown = render_hover_markdown(
            &display,
            Some(HoverRenderContext {
                query: &query,
                member_summary_query: None,
                links: None,
            }),
        );

        assert!(markdown.contains("data-code=\"int i = 0\""));
        assert!(markdown.contains("<span style=\"color:#59A6E9;\">int</span>"));
        assert!(markdown.contains("<span style=\"color:#cfcfcf;\">i</span> = 0"));
        assert!(!markdown.contains(">i</span>nt"));
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
