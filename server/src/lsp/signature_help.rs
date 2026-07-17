use crate::index::SymbolIndex;
use crate::index_query::{
    EditorCompletionCandidate, EditorCompletionOrigin, EditorTopLevelCompletionMode, IndexQuery,
};
use crate::lsp::callable::{
    callable_argument_context_at_offset, callable_signature_parts, CallableArgumentContext,
    CallableParameter, CallableSignatureParts, CallableTarget,
};
use crate::lsp::{
    file_index_for_source, offset_for_position, FileIndexAnalysis, LspMarkupContent, LspPosition,
};
use crate::resolver::{CandidateSource, ReferenceCandidate, ReferenceResolver};
use crate::symbol_display::documentation_display;
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspSignatureHelp {
    pub signatures: Vec<LspSignatureInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_signature: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_parameter: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspSignatureInformation {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<LspMarkupContent>,
    pub parameters: Vec<LspParameterInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_parameter: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspParameterInformation {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<LspMarkupContent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspSignatureHelpReport {
    pub help: Option<LspSignatureHelp>,
    pub parse_diagnostics: usize,
    pub context: Option<String>,
    pub active_parameter: Option<usize>,
    pub candidate_count: usize,
    pub selected_label: Option<String>,
    pub failure_reason: Option<String>,
    pub timings: LspSignatureHelpTimings,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LspSignatureHelpTimings {
    pub context_detection: Duration,
    pub candidate_lookup: Duration,
    pub item_rendering: Duration,
    pub total: Duration,
}

pub fn signature_help_report_for_source_position(
    source: &str,
    position: LspPosition,
) -> LspSignatureHelpReport {
    let analysis = file_index_for_source(source);
    signature_help_report_for_cached_analysis_with_external_indexes(
        source, &analysis, position, None, None,
    )
}

pub(crate) fn signature_help_report_for_cached_analysis_with_external_indexes(
    source: &str,
    analysis: &FileIndexAnalysis,
    position: LspPosition,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> LspSignatureHelpReport {
    let total_start = Instant::now();
    let Some(offset) = offset_for_position(source, position) else {
        return empty_signature_help_report(analysis.parse_diagnostics, total_start);
    };

    let context_start = Instant::now();
    let Some(context) = callable_argument_context_at_offset(source, &analysis.parse.root, offset)
    else {
        let mut report = empty_signature_help_report(analysis.parse_diagnostics, total_start);
        report.timings.context_detection = context_start.elapsed();
        report.failure_reason = Some("not in callable argument list".to_string());
        return report;
    };
    let context_elapsed = context_start.elapsed();

    let resolver = ReferenceResolver::new_with_parse_scope_and_external_indexes(
        source,
        &analysis.index,
        &analysis.parse,
        &analysis.scope,
        workspace_index.into_iter().chain(game_data_index),
    );

    let lookup_start = Instant::now();
    let candidates = callable_candidates_for_context(
        &context,
        &resolver,
        &analysis.index,
        workspace_index,
        game_data_index,
    );
    let lookup_elapsed = lookup_start.elapsed();
    if candidates.is_empty() {
        let mut report = empty_signature_help_report(analysis.parse_diagnostics, total_start);
        report.timings.context_detection = context_elapsed;
        report.timings.candidate_lookup = lookup_elapsed;
        report.context = Some(context_label(&context));
        report.active_parameter = Some(context.argument_index);
        report.failure_reason = Some("callable target unresolved".to_string());
        return report;
    }

    let render_start = Instant::now();
    let mut signatures = Vec::new();
    let mut selected_label = None;
    let mut active_parameter = context.argument_index;
    for candidate in &candidates {
        let label = candidate
            .name
            .as_deref()
            .unwrap_or(candidate.display.label.as_str());
        let signature = candidate
            .signature
            .as_deref()
            .or(candidate.constructor_signature.as_deref());
        let Some(signature) = signature else {
            continue;
        };
        let Some(parts) = callable_signature_parts(label, signature) else {
            continue;
        };
        if let Some(label) = context.active_label.as_ref() {
            if let Some(index) = parts
                .parameters_info
                .iter()
                .position(|parameter| parameter.name.eq_ignore_ascii_case(label))
            {
                active_parameter = index;
            }
        }
        if selected_label.is_none() {
            selected_label = Some(label.to_string());
        }
        signatures.push(signature_information_for_candidate(
            candidate,
            label,
            &parts,
            active_parameter,
        ));
    }
    let render_elapsed = render_start.elapsed();

    if signatures.is_empty() {
        let mut report = empty_signature_help_report(analysis.parse_diagnostics, total_start);
        report.timings.context_detection = context_elapsed;
        report.timings.candidate_lookup = lookup_elapsed;
        report.timings.item_rendering = render_elapsed;
        report.context = Some(context_label(&context));
        report.active_parameter = Some(context.argument_index);
        report.failure_reason = Some("callable signature unavailable".to_string());
        return report;
    }

    LspSignatureHelpReport {
        help: Some(LspSignatureHelp {
            signatures,
            active_signature: Some(0),
            active_parameter: Some(active_parameter as u32),
        }),
        parse_diagnostics: analysis.parse_diagnostics,
        context: Some(context_label(&context)),
        active_parameter: Some(active_parameter),
        candidate_count: candidates.len(),
        selected_label,
        failure_reason: None,
        timings: LspSignatureHelpTimings {
            context_detection: context_elapsed,
            candidate_lookup: lookup_elapsed,
            item_rendering: render_elapsed,
            total: total_start.elapsed(),
        },
    }
}

fn empty_signature_help_report(
    parse_diagnostics: usize,
    total_start: Instant,
) -> LspSignatureHelpReport {
    LspSignatureHelpReport {
        help: None,
        parse_diagnostics,
        context: None,
        active_parameter: None,
        candidate_count: 0,
        selected_label: None,
        failure_reason: None,
        timings: LspSignatureHelpTimings {
            total: total_start.elapsed(),
            ..Default::default()
        },
    }
}

fn callable_candidates_for_context(
    context: &CallableArgumentContext,
    resolver: &ReferenceResolver<'_, '_>,
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> Vec<EditorCompletionCandidate> {
    match &context.target {
        CallableTarget::Attribute { name } | CallableTarget::New { type_name: name } => {
            callable_type_candidates(name, local_index, workspace_index, game_data_index)
        }
        CallableTarget::Call { callee_span } => resolver
            .resolve_at_offset(callee_span.start)
            .map(|resolution| {
                let mut references = Vec::new();
                if let Some(selected) = resolution.selected {
                    references.push(selected.clone());
                    references.extend(
                        resolution
                            .candidates
                            .into_iter()
                            .filter(|candidate| candidate.id != selected.id),
                    );
                } else {
                    references = resolution.candidates;
                }
                references
                    .into_iter()
                    .filter_map(|reference| {
                        completion_candidate_for_reference(
                            &reference,
                            local_index,
                            workspace_index,
                            game_data_index,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn callable_type_candidates(
    name: &str,
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> Vec<EditorCompletionCandidate> {
    let mut candidates = Vec::new();
    candidates.extend(exact_type_candidates(name, local_index));
    if let Some(index) = workspace_index {
        candidates.extend(exact_type_candidates(name, index));
    }
    if let Some(index) = game_data_index {
        candidates.extend(exact_type_candidates(name, index));
    }
    candidates
        .into_iter()
        .filter(|candidate| candidate.constructor_signature.is_some())
        .collect()
}

fn exact_type_candidates(name: &str, index: &SymbolIndex) -> Vec<EditorCompletionCandidate> {
    IndexQuery::new(index)
        .completion_top_level_limited(name, EditorTopLevelCompletionMode::Type, 32)
        .into_iter()
        .filter(|candidate| {
            candidate
                .name
                .as_deref()
                .unwrap_or(candidate.display.label.as_str())
                == name
        })
        .collect()
}

fn completion_candidate_for_reference(
    reference: &ReferenceCandidate,
    local_index: &SymbolIndex,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> Option<EditorCompletionCandidate> {
    let indexes = match reference.source {
        CandidateSource::FileLocal => vec![local_index],
        CandidateSource::External => workspace_index.into_iter().chain(game_data_index).collect(),
    };

    for index in indexes {
        let Some(symbol) = index.symbol(reference.id) else {
            continue;
        };
        if symbol.kind != reference.kind || symbol.name != reference.name {
            continue;
        }
        if let Some(expected_path) = reference.absolute_path.as_ref() {
            let Some(actual_path) = index
                .file(reference.id.file_id)
                .and_then(|file| file.metadata.absolute_path.as_ref())
            else {
                continue;
            };
            if actual_path != expected_path {
                continue;
            }
        }
        return IndexQuery::new(index)
            .completion_symbols([reference.id], EditorCompletionOrigin::Direct)
            .into_iter()
            .next();
    }

    None
}

fn signature_information_for_candidate(
    candidate: &EditorCompletionCandidate,
    label: &str,
    parts: &CallableSignatureParts,
    active_parameter: usize,
) -> LspSignatureInformation {
    let signature = candidate
        .signature
        .as_deref()
        .or(candidate.constructor_signature.as_deref())
        .unwrap_or(label);
    let documentation = callable_documentation(candidate, parts);
    let parameters = parts
        .parameters_info
        .iter()
        .map(|parameter| parameter_information(parameter, candidate))
        .collect();
    LspSignatureInformation {
        label: signature.to_string(),
        documentation,
        parameters,
        active_parameter: Some(active_parameter as u32),
    }
}

fn callable_documentation(
    candidate: &EditorCompletionCandidate,
    parts: &CallableSignatureParts,
) -> Option<LspMarkupContent> {
    let mut sections = Vec::new();
    let parameter_summary = signature_parameter_summary(parts, candidate);
    if !parameter_summary.is_empty() {
        sections.push(parameter_summary);
    }
    if let Some(preview) = candidate.display.documentation_preview.as_ref() {
        if !preview.trim().is_empty() {
            sections.push(preview.clone());
        }
    }
    if sections.is_empty() {
        None
    } else {
        Some(LspMarkupContent {
            kind: "markdown".to_string(),
            value: sections.join("\n\n"),
        })
    }
}

fn signature_parameter_summary(
    parts: &CallableSignatureParts,
    candidate: &EditorCompletionCandidate,
) -> String {
    let docs = documentation_display(&candidate.display.doc_comments);
    let mut output = String::new();
    for parameter in &parts.parameters_info {
        let optional = parameter.default_text.is_some();
        let direction = parameter_direction(parameter);
        let doc = docs
            .parameters
            .iter()
            .find(|doc| doc.name == parameter.name)
            .map(|doc| doc.description.as_str())
            .unwrap_or("");
        output.push_str(&format!(
            "- `{}`{}{}{}\n",
            parameter.name,
            parameter_type_suffix(parameter),
            if optional { " optional" } else { "" },
            parameter
                .default_text
                .as_ref()
                .map(|default| format!(" = `{}`", escape_markdown_inline_code(default)))
                .unwrap_or_default()
        ));
        if let Some(direction) = direction {
            output.push_str(&format!("  - direction: `{direction}`\n"));
        }
        if !doc.is_empty() {
            output.push_str(&format!("  - {}\n", escape_markdown_text(doc)));
        }
    }
    if let Some(returns) = docs.returns {
        if !returns.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("Returns: {}\n", escape_markdown_text(&returns)));
        }
    }
    output
}

fn parameter_information(
    parameter: &CallableParameter,
    candidate: &EditorCompletionCandidate,
) -> LspParameterInformation {
    let docs = documentation_display(&candidate.display.doc_comments);
    let doc = docs
        .parameters
        .iter()
        .find(|doc| doc.name == parameter.name)
        .map(|doc| doc.description.clone());
    let mut documentation = Vec::new();
    if let Some(doc) = doc {
        if !doc.trim().is_empty() {
            documentation.push(doc);
        }
    }
    if let Some(default) = parameter.default_text.as_ref() {
        documentation.push(format!(
            "Optional. Default: `{}`",
            escape_markdown_inline_code(default)
        ));
    }
    LspParameterInformation {
        label: parameter.raw.clone(),
        documentation: (!documentation.is_empty()).then(|| LspMarkupContent {
            kind: "markdown".to_string(),
            value: documentation.join("\n\n"),
        }),
    }
}

fn parameter_direction(parameter: &CallableParameter) -> Option<&'static str> {
    let text = parameter.type_and_modifiers.as_str();
    if text.split_whitespace().any(|part| part == "inout") {
        Some("inout")
    } else if text.split_whitespace().any(|part| part == "out") {
        Some("out")
    } else {
        None
    }
}

fn parameter_type_suffix(parameter: &CallableParameter) -> String {
    if parameter.type_and_modifiers.is_empty() {
        String::new()
    } else {
        format!(
            ": `{}`",
            escape_markdown_inline_code(&parameter.type_and_modifiers)
        )
    }
}

pub(crate) fn signature_help_debug_markdown(report: &LspSignatureHelpReport) -> String {
    let mut output = String::new();
    output.push_str("\n---\n\n");
    output.push_str("## Signature Help Context\n\n");
    output.push_str(&format!(
        "- Context: `{}`\n",
        escape_markdown_cell(report.context.as_deref().unwrap_or("<none>"))
    ));
    output.push_str(&format!(
        "- Active Parameter: `{}`\n",
        report
            .active_parameter
            .map(|index| index.to_string())
            .unwrap_or_else(|| "<none>".to_string())
    ));
    output.push_str(&format!(
        "- Candidate Count: `{}`\n",
        report.candidate_count
    ));
    output.push_str(&format!(
        "- Selected Callable: `{}`\n",
        escape_markdown_cell(report.selected_label.as_deref().unwrap_or("<none>"))
    ));
    output.push_str(&format!(
        "- Failure Reason: `{}`\n",
        escape_markdown_cell(report.failure_reason.as_deref().unwrap_or("<none>"))
    ));
    output.push_str(&format!(
        "- Parse Diagnostics: `{}`\n\n",
        report.parse_diagnostics
    ));

    output.push_str("## Signature Help Timings\n\n");
    output.push_str("| Phase | Milliseconds |\n");
    output.push_str("| --- | ---: |\n");
    output.push_str(&format!(
        "| Context detection | {} |\n",
        report.timings.context_detection.as_millis()
    ));
    output.push_str(&format!(
        "| Candidate lookup | {} |\n",
        report.timings.candidate_lookup.as_millis()
    ));
    output.push_str(&format!(
        "| Item rendering | {} |\n",
        report.timings.item_rendering.as_millis()
    ));
    output.push_str(&format!(
        "| Total | {} |\n\n",
        report.timings.total.as_millis()
    ));

    output.push_str("## Signature Candidates\n\n");
    let Some(help) = report.help.as_ref() else {
        output.push_str("None.\n");
        return output;
    };
    if help.signatures.is_empty() {
        output.push_str("None.\n");
        return output;
    }

    output.push_str("| # | Signature | Active Param | Parameters | Documentation |\n");
    output.push_str("| ---: | --- | ---: | --- | --- |\n");
    for (index, signature) in help.signatures.iter().take(20).enumerate() {
        output.push_str(&format!(
            "| {} | `{}` | {} | {} | {} |\n",
            index + 1,
            escape_markdown_cell(&signature.label),
            signature
                .active_parameter
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            signature_parameters_cell(signature),
            markdown_table_text(
                signature
                    .documentation
                    .as_ref()
                    .map(|documentation| documentation.value.as_str())
                    .unwrap_or("")
            )
        ));
    }
    if help.signatures.len() > 20 {
        output.push_str(&format!(
            "|  |  |  |  | +{} more signatures |\n",
            help.signatures.len() - 20
        ));
    }
    output.push('\n');

    if let Some(signature) = help.signatures.first() {
        output.push_str("## Active Signature Parameters\n\n");
        output.push_str("| # | Parameter | Active | Documentation |\n");
        output.push_str("| ---: | --- | --- | --- |\n");
        let active = help
            .active_parameter
            .or(signature.active_parameter)
            .map(|value| value as usize);
        for (index, parameter) in signature.parameters.iter().enumerate() {
            output.push_str(&format!(
                "| {} | `{}` | {} | {} |\n",
                index + 1,
                escape_markdown_cell(&parameter.label),
                if Some(index) == active { "yes" } else { "no" },
                markdown_table_text(
                    parameter
                        .documentation
                        .as_ref()
                        .map(|documentation| documentation.value.as_str())
                        .unwrap_or("")
                )
            ));
        }
    }

    output
}

fn context_label(context: &CallableArgumentContext) -> String {
    match &context.target {
        CallableTarget::Attribute { name } => format!("attribute {name}"),
        CallableTarget::Call { .. } => "call".to_string(),
        CallableTarget::New { type_name } => format!("new {type_name}"),
    }
}

fn escape_markdown_inline_code(value: &str) -> String {
    value.replace('`', "\\`")
}

fn escape_markdown_text(value: &str) -> String {
    value.replace('\\', "\\\\")
}

fn signature_parameters_cell(signature: &LspSignatureInformation) -> String {
    let parameters = signature
        .parameters
        .iter()
        .map(|parameter| format!("`{}`", escape_markdown_cell(&parameter.label)))
        .collect::<Vec<_>>()
        .join("<br>");
    if parameters.is_empty() {
        String::new()
    } else {
        parameters
    }
}

fn escape_markdown_cell(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('|', "\\|")
        .replace('\r', "")
        .replace('\n', " ")
}

fn markdown_table_text(value: &str) -> String {
    escape_markdown_cell(value.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::LspPosition;

    #[test]
    fn function_call_reports_active_parameter_after_comma() {
        let source = r#"class Example
{
	void SendToEveryone(ENotification notificationID, int param1 = 0);
	void Run()
	{
		SendToEveryone(ENotification.PLAYER_JOINED, );
	}
}
enum ENotification
{
	PLAYER_JOINED
}"#;
        let position = position_after(source, "PLAYER_JOINED, ");
        let report = signature_help_report_for_source_position(source, position);
        assert_eq!(report.active_parameter, Some(1));
        let help = report.help.unwrap();
        assert_eq!(help.signatures[0].parameters.len(), 2);
    }

    #[test]
    fn named_argument_selects_matching_parameter() {
        let source = r#"class Attribute
{
	void Attribute(string defvalue = "", string uiwidget = "auto", string desc = "");
}
class Example
{
	[Attribute(desc: )]
	int m_Value;
}"#;
        let position = position_after(source, "desc: ");
        let report = signature_help_report_for_source_position(source, position);
        assert_eq!(report.active_parameter, Some(2));
    }

    #[test]
    fn non_call_position_returns_no_help() {
        let source = "class Example {}";
        let report = signature_help_report_for_source_position(
            source,
            LspPosition {
                line: 0,
                character: 2,
            },
        );
        assert!(report.help.is_none());
    }

    fn position_after(source: &str, needle: &str) -> LspPosition {
        let offset = source.find(needle).unwrap() + needle.len();
        let mut line = 0u32;
        let mut character = 0u32;
        for ch in source[..offset].chars() {
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }
        LspPosition { line, character }
    }
}
