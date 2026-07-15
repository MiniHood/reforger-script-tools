use crate::index::SymbolIndex;
use crate::lsp::{
    file_index_for_source, offset_for_position, range_for_span, FileIndexAnalysis, LspPosition,
    LspRange,
};
use crate::model::SymbolKind;
use crate::resolver::{
    CandidateSource, IdentifierContext, ReferenceCandidate, ReferenceResolver, ResolutionReason,
};
use serde::Serialize;
use std::fs;
use std::path::Path;

fn layered_external_indexes<'a>(
    workspace_index: Option<&'a SymbolIndex>,
    game_data_index: Option<&'a SymbolIndex>,
) -> Vec<&'a SymbolIndex> {
    workspace_index.into_iter().chain(game_data_index).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspLocationLink {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_selection_range: Option<LspRange>,
    pub target_uri: String,
    pub target_range: LspRange,
    pub target_selection_range: LspRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDefinitionReport {
    pub locations: Vec<LspLocation>,
    pub links: Vec<LspLocationLink>,
    pub parse_diagnostics: usize,
    pub selected_label: Option<String>,
    pub selected_kind: Option<SymbolKind>,
    pub selected_source: Option<CandidateSource>,
    pub resolver_reason: Option<ResolutionReason>,
    pub identifier_context: Option<IdentifierContext>,
    pub resolver_candidate_count: usize,
}

impl LspDefinitionReport {
    pub fn is_hit(&self) -> bool {
        !self.links.is_empty()
    }
}

pub fn definition_report_for_source_position(
    source: &str,
    uri: &str,
    position: LspPosition,
) -> LspDefinitionReport {
    definition_report_for_source_position_with_external(source, uri, position, None)
}

pub fn definition_report_for_source_position_with_external(
    source: &str,
    uri: &str,
    position: LspPosition,
    external_index: Option<&SymbolIndex>,
) -> LspDefinitionReport {
    let analysis = file_index_for_source(source);
    definition_report_for_cached_analysis_with_external(
        source,
        &analysis,
        uri,
        position,
        external_index,
    )
}

pub fn definition_report_for_cached_analysis_with_external(
    source: &str,
    analysis: &FileIndexAnalysis,
    uri: &str,
    position: LspPosition,
    external_index: Option<&SymbolIndex>,
) -> LspDefinitionReport {
    definition_report_for_cached_analysis_with_external_indexes(
        source,
        analysis,
        uri,
        position,
        None,
        external_index,
    )
}

pub(crate) fn definition_report_for_cached_analysis_with_external_indexes(
    source: &str,
    analysis: &FileIndexAnalysis,
    uri: &str,
    position: LspPosition,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> LspDefinitionReport {
    let Some(offset) = offset_for_position(source, position) else {
        return empty_definition_report(analysis.parse_diagnostics);
    };
    definition_report_for_offset(
        source,
        analysis,
        uri,
        offset,
        workspace_index,
        game_data_index,
    )
}

fn definition_report_for_offset(
    source: &str,
    analysis: &FileIndexAnalysis,
    uri: &str,
    offset: usize,
    workspace_index: Option<&SymbolIndex>,
    game_data_index: Option<&SymbolIndex>,
) -> LspDefinitionReport {
    let resolver = ReferenceResolver::new_with_parse_scope_and_external_indexes(
        source,
        &analysis.index,
        &analysis.parse,
        &analysis.scope,
        layered_external_indexes(workspace_index, game_data_index),
    );
    let Some(resolution) = resolver.resolve_at_offset(offset) else {
        return empty_definition_report(analysis.parse_diagnostics);
    };
    let candidate_count = resolution.candidates.len();
    let reason = resolution.reason;
    let identifier_context = resolution.identifier_context;
    let Some(selected) = resolution.selected.as_ref() else {
        return LspDefinitionReport {
            locations: Vec::new(),
            links: Vec::new(),
            parse_diagnostics: analysis.parse_diagnostics,
            selected_label: None,
            selected_kind: None,
            selected_source: None,
            resolver_reason: Some(reason),
            identifier_context: Some(identifier_context),
            resolver_candidate_count: candidate_count,
        };
    };

    let link = definition_link_for_candidate(uri, source, resolution.token_span, selected);
    let location = link.as_ref().map(location_from_link);
    LspDefinitionReport {
        links: link.into_iter().collect(),
        locations: location.into_iter().collect(),
        parse_diagnostics: analysis.parse_diagnostics,
        selected_label: selected.name.clone(),
        selected_kind: Some(selected.kind),
        selected_source: Some(selected.source),
        resolver_reason: Some(reason),
        identifier_context: Some(identifier_context),
        resolver_candidate_count: candidate_count,
    }
}

fn empty_definition_report(parse_diagnostics: usize) -> LspDefinitionReport {
    LspDefinitionReport {
        locations: Vec::new(),
        links: Vec::new(),
        parse_diagnostics,
        selected_label: None,
        selected_kind: None,
        selected_source: None,
        resolver_reason: None,
        identifier_context: None,
        resolver_candidate_count: 0,
    }
}

fn definition_link_for_candidate(
    current_uri: &str,
    current_source: &str,
    origin_span: crate::lexer::TextSpan,
    candidate: &ReferenceCandidate,
) -> Option<LspLocationLink> {
    let origin_selection_range = Some(range_for_span(current_source, origin_span));
    match candidate.source {
        CandidateSource::FileLocal => Some(LspLocationLink {
            origin_selection_range,
            target_uri: current_uri.to_string(),
            target_range: range_for_span(current_source, candidate.span),
            target_selection_range: range_for_span(current_source, candidate.selection_span),
        }),
        CandidateSource::External => {
            let path = candidate.absolute_path.as_ref()?;
            let source = fs::read_to_string(path).ok()?;
            Some(LspLocationLink {
                origin_selection_range,
                target_uri: file_uri_for_path(path)?,
                target_range: range_for_span(&source, candidate.span),
                target_selection_range: range_for_span(&source, candidate.selection_span),
            })
        }
    }
}

fn location_from_link(link: &LspLocationLink) -> LspLocation {
    LspLocation {
        uri: link.target_uri.clone(),
        range: link.target_selection_range,
    }
}

pub(crate) fn file_uri_for_path(path: &Path) -> Option<String> {
    if !path.is_absolute() {
        return None;
    }
    let mut normalized = path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = normalized.strip_prefix("//?/") {
        normalized = stripped.to_string();
    }
    if normalized.starts_with('/') {
        Some(format!("file://{}", percent_encode_uri_path(&normalized)))
    } else {
        Some(format!("file:///{}", percent_encode_uri_path(&normalized)))
    }
}

fn percent_encode_uri_path(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        let keep =
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'_' | b'.' | b'~');
        if keep {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
