use crate::analysis_runtime::{DocumentSnapshot, Position};
use crate::index::SymbolIndex;
use crate::lexer::{lex, Keyword, Token, TokenKind};
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

/// The pending-analysis definition contract is intentionally narrower than
/// resolver-backed navigation: only a cursor already on a syntactically proven
/// top-level declaration may navigate to that declaration's current-snapshot
/// name.  References, members, locals, and recovered declarations return no
/// target rather than combining current text with an older analysis.
pub(crate) fn definition_report_for_pending_snapshot(
    snapshot: &DocumentSnapshot,
    uri: &str,
    position: LspPosition,
    parse_diagnostics: usize,
) -> LspDefinitionReport {
    let Some(offset) = snapshot.positions().offset_for_position(Position {
        line: position.line,
        character: position.character,
    }) else {
        return empty_definition_report(parse_diagnostics);
    };

    let source = snapshot.text();
    let Some((name, name_span, kind)) = lexical_top_level_declaration_at_offset(source, offset)
    else {
        return empty_definition_report(parse_diagnostics);
    };
    let range = range_for_span(source, name_span);
    let link = LspLocationLink {
        origin_selection_range: Some(range.clone()),
        target_uri: uri.to_string(),
        target_range: range.clone(),
        target_selection_range: range,
    };
    LspDefinitionReport {
        locations: vec![location_from_link(&link)],
        links: vec![link],
        parse_diagnostics,
        selected_label: Some(name),
        selected_kind: Some(kind),
        selected_source: Some(CandidateSource::FileLocal),
        resolver_reason: None,
        identifier_context: None,
        resolver_candidate_count: 1,
    }
}

fn lexical_top_level_declaration_at_offset(
    source: &str,
    offset: usize,
) -> Option<(String, crate::lexer::TextSpan, SymbolKind)> {
    let tokens = lex(source);
    let mut brace_depth = 0usize;
    let mut index = 0usize;
    while index < tokens.len() {
        let token = tokens[index];
        match token.kind {
            TokenKind::LeftBrace => brace_depth += 1,
            TokenKind::RightBrace => brace_depth = brace_depth.saturating_sub(1),
            TokenKind::Keyword(Keyword::Class | Keyword::Enum | Keyword::Typedef)
                if brace_depth == 0 =>
            {
                let declaration_kind = token.kind;
                if let Some((name, name_token, next_index)) =
                    lexical_top_level_declaration(&tokens, index, declaration_kind, source)
                {
                    if name_token.span.start <= offset && offset <= name_token.span.end {
                        let kind = match declaration_kind {
                            TokenKind::Keyword(Keyword::Class) => SymbolKind::Class,
                            TokenKind::Keyword(Keyword::Enum) => SymbolKind::Enum,
                            TokenKind::Keyword(Keyword::Typedef) => SymbolKind::Typedef,
                            _ => unreachable!("only declaration keywords reach this branch"),
                        };
                        return Some((name, name_token.span, kind));
                    }
                    index = next_index;
                    continue;
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn lexical_top_level_declaration(
    tokens: &[Token],
    keyword_index: usize,
    declaration_kind: TokenKind,
    source: &str,
) -> Option<(String, Token, usize)> {
    let mut index = keyword_index + 1;
    let mut typedef_name = None;
    while let Some(token) = tokens.get(index).copied() {
        if token.kind.is_trivia() {
            index += 1;
            continue;
        }
        match declaration_kind {
            TokenKind::Keyword(Keyword::Class | Keyword::Enum) => {
                return (token.kind == TokenKind::Identifier).then(|| {
                    (
                        source[token.span.start..token.span.end].to_string(),
                        token,
                        index + 1,
                    )
                });
            }
            TokenKind::Keyword(Keyword::Typedef) => match token.kind {
                TokenKind::Identifier => typedef_name = Some(token),
                TokenKind::Semicolon | TokenKind::Eof => {
                    return typedef_name.map(|name| {
                        (
                            source[name.span.start..name.span.end].to_string(),
                            name,
                            index + 1,
                        )
                    });
                }
                TokenKind::LeftBrace | TokenKind::RightBrace => return None,
                _ => {}
            },
            _ => return None,
        }
        index += 1;
    }
    None
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
    if let Some(stripped) = normalized.strip_prefix("//?/UNC/") {
        normalized = format!("//{stripped}");
    }
    if let Some(stripped) = normalized.strip_prefix("//?/") {
        normalized = stripped.to_string();
    }
    if let Some(unc) = normalized.strip_prefix("//") {
        let (host, share_path) = unc.split_once('/')?;
        return Some(format!(
            "file://{}/{}",
            percent_encode_uri_path(host),
            percent_encode_uri_path(share_path)
        ));
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
