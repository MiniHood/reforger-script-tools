use crate::game_data_search::{
    compact_signature, documentation_summary, encode_symbol_ref, kind_name, logical_path,
    owner_name, qualify, SourceLineStarts,
};
use crate::index::{SourceFileId, SymbolIndex};
use crate::index_build::IndexBuildControl;
use crate::index_cache::source_content_digest;
use crate::symbol_display::documentation_display;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const MAX_SYMBOL_REF_BYTES: usize = 2048;
const MAX_MEMBERS: usize = 50;
const MAX_RAW_DOCUMENTATION_BYTES: usize = 16 * 1024;
const DEFAULT_LINES: usize = 200;
const MAX_LINES: usize = 500;
const MAX_SOURCE_BYTES: usize = 128 * 1024;

#[derive(Debug)]
pub enum GameDataInspectionError {
    Initialization(String),
    Unavailable,
    InvalidSymbolRef,
    StaleSymbolRef,
    InvalidSource(String),
    GameDataChanged,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct GameDataSourceReadRequest {
    pub catalogue_revision: String,
    pub relative_path: String,
    pub start_line: Option<usize>,
    pub line_count: Option<usize>,
}

pub fn inspect(
    index: &SymbolIndex,
    starts: &BTreeMap<SourceFileId, SourceLineStarts>,
    control: &IndexBuildControl,
    revision: &str,
    symbol_ref: &str,
) -> Result<Value, GameDataInspectionError> {
    if symbol_ref.len() > MAX_SYMBOL_REF_BYTES || !symbol_ref.starts_with("sr1:") {
        return Err(GameDataInspectionError::InvalidSymbolRef);
    }
    let mut found = None;
    for symbol in index.symbols() {
        control
            .check()
            .map_err(|_| GameDataInspectionError::Cancelled)?;
        let Some(file) = index.file(symbol.id.file_id) else {
            continue;
        };
        let Some(name) = symbol.name.as_deref() else {
            continue;
        };
        let owner = owner_name(index, symbol);
        let qualified = qualify(owner.as_deref(), name);
        let expected = encode_symbol_ref(
            revision,
            &logical_path(file),
            kind_name(symbol.kind),
            &qualified,
            symbol.selection_span.start,
        );
        if expected == symbol_ref {
            found = Some((symbol.id, qualified));
            break;
        }
    }
    let Some((id, qualified_name)) = found else {
        return Err(GameDataInspectionError::StaleSymbolRef);
    };
    let symbol = index.symbol(id).expect("located symbol");
    let file = index.file(id.file_id).expect("located file");
    let lines = starts.get(&id.file_id).cloned().unwrap_or_default();
    let range = |span: crate::lexer::TextSpan| {
        let range = lines.range(span.start, span.end);
        json!({"startLine": range.start_line, "endLine": range.end_line})
    };
    let raw = symbol
        .doc_comments
        .iter()
        .map(|comment| comment.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let (raw, raw_truncated) = bounded(raw, MAX_RAW_DOCUMENTATION_BYTES);
    let members = index
        .children(id)
        .iter()
        .filter_map(|child| index.symbol(*child).map(|member| (*child, member)))
        .filter(|(_, member)| member.kind != crate::model::SymbolKind::Parameter)
        .collect::<Vec<_>>();
    let member_values = members.iter().take(MAX_MEMBERS).map(|(member_id, member)| {
        let name = member.name.clone().unwrap_or_default(); let owner = owner_name(index, member); let qualified = qualify(owner.as_deref(), &name);
        json!({"symbolRef": encode_symbol_ref(revision, &logical_path(file), kind_name(member.kind), &qualified, member.selection_span.start), "name": name, "kind": kind_name(member.kind), "signature": index.callable_signature(*member_id).unwrap_or_else(|| compact_signature(member, &qualified)), "documentationSummary": documentation_summary(member), "selectionRange": range(member.selection_span)})
    }).collect::<Vec<_>>();
    let parent_ref = symbol
        .parent
        .and_then(|parent| index.symbol(parent))
        .and_then(|parent| {
            let parent_file = index.file(parent.id.file_id)?;
            let name = parent.name.as_deref()?;
            let owner = owner_name(index, parent);
            let q = qualify(owner.as_deref(), name);
            Some(encode_symbol_ref(
                revision,
                &logical_path(parent_file),
                kind_name(parent.kind),
                &q,
                parent.selection_span.start,
            ))
        });
    let members_returned = member_values.len();
    let documentation = documentation_display(&symbol.doc_comments);
    Ok(json!({"catalogueRevision": revision, "symbolRef": symbol_ref, "name": symbol.name, "kind": kind_name(symbol.kind), "qualifiedName": qualified_name, "container": owner_name(index, symbol), "signature": index.callable_signature(id).unwrap_or_else(|| compact_signature(symbol, &qualified_name)), "type": symbol.detail.type_text, "returnType": symbol.detail.return_type_text, "baseType": symbol.detail.base_type, "defaultValue": symbol.detail.default_text, "enumValue": symbol.detail.enum_value_text, "modifiers": symbol.modifiers, "attributes": symbol.attributes.iter().map(|a| &a.text).collect::<Vec<_>>(), "callableForm": symbol.callable_form.map(|f| f.as_str()), "documentation": {"summary": documentation.summary, "parameters": documentation.parameters.into_iter().map(|parameter| json!({"name": parameter.name, "direction": parameter.direction, "description": parameter.description})).collect::<Vec<_>>(), "returns": documentation.returns, "warnings": documentation.warnings, "notes": documentation.notes}, "rawDocumentation": raw, "rawTruncated": raw_truncated, "conditionalContext": symbol.conditional_context.iter().map(|c| json!({"kind": c.kind.as_str(), "condition": c.condition})).collect::<Vec<_>>(), "sourceCategory": file.metadata.category.as_str(), "relativePath": logical_path(file), "declarationRange": range(symbol.span), "selectionRange": range(symbol.selection_span), "parentSymbolRef": parent_ref, "readSourceInput": {"catalogueRevision": revision, "relativePath": logical_path(file), "startLine": lines.range(symbol.span.start, symbol.span.end).start_line}, "members": member_values, "membersReturned": members_returned, "membersTotal": members.len(), "membersTruncated": members.len() > MAX_MEMBERS, "membersTruncationGuidance": if members.len() > MAX_MEMBERS { Some(format!("Call search_game_data_symbols with owner '{qualified_name}'.")) } else { None::<String> }}))
}

pub fn read_source(
    index: &SymbolIndex,
    control: &IndexBuildControl,
    revision: &str,
    root: &Option<PathBuf>,
    expected_digest: &str,
    request: GameDataSourceReadRequest,
) -> Result<Value, GameDataInspectionError> {
    if request.catalogue_revision != revision {
        return Err(GameDataInspectionError::StaleSymbolRef);
    }
    if request.relative_path.is_empty()
        || request.relative_path.contains("\\")
        || request.relative_path.starts_with('/')
        || request.relative_path.contains("..")
    {
        return Err(GameDataInspectionError::InvalidSource(
            "relativePath must be an exact logical catalogue path".to_string(),
        ));
    }
    let root = root.as_ref().ok_or(GameDataInspectionError::Unavailable)?;
    if source_content_digest(root, control).map_err(GameDataInspectionError::Initialization)?
        != expected_digest
    {
        return Err(GameDataInspectionError::GameDataChanged);
    }
    let file = index
        .files()
        .iter()
        .find(|file| logical_path(file) == request.relative_path)
        .ok_or_else(|| {
            GameDataInspectionError::InvalidSource(
                "relativePath is not in the catalogue".to_string(),
            )
        })?;
    let path = file.metadata.absolute_path.as_ref().ok_or_else(|| {
        GameDataInspectionError::InvalidSource("catalogue source is unavailable".to_string())
    })?;
    let canonical_root = root
        .canonicalize()
        .map_err(|_| GameDataInspectionError::GameDataChanged)?;
    let canonical_path = path
        .canonicalize()
        .map_err(|_| GameDataInspectionError::GameDataChanged)?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(GameDataInspectionError::InvalidSource(
            "relativePath escapes the Game Data root".to_string(),
        ));
    }
    let source = std::fs::read_to_string(canonical_path)
        .map_err(|_| GameDataInspectionError::GameDataChanged)?;
    let start = request.start_line.unwrap_or(1);
    if start == 0 {
        return Err(GameDataInspectionError::InvalidSource(
            "startLine must be one-based".to_string(),
        ));
    }
    let count = request
        .line_count
        .unwrap_or(DEFAULT_LINES)
        .clamp(1, MAX_LINES);
    let all = source.split_inclusive('\n').collect::<Vec<_>>();
    if start > all.len().saturating_add(1) {
        return Err(GameDataInspectionError::InvalidSource(
            "startLine is outside the source file".to_string(),
        ));
    }
    let mut content = String::new();
    let mut taken = 0;
    for text in all.iter().skip(start - 1).take(count) {
        if content.len() + text.len() > MAX_SOURCE_BYTES {
            break;
        }
        content.push_str(text);
        taken += 1;
    }
    if taken == 0 && start <= all.len() {
        return Err(GameDataInspectionError::InvalidSource(
            "The requested source line exceeds the 128 KiB response limit.".to_string(),
        ));
    }
    let end = if taken == 0 {
        start.saturating_sub(1)
    } else {
        start + taken - 1
    };
    let truncated = end < all.len();
    Ok(
        json!({"catalogueRevision": revision, "relativePath": request.relative_path, "startLine": start, "endLine": end, "content": content, "truncated": truncated, "nextStartLine": truncated.then_some(end + 1)}),
    )
}

fn bounded(value: String, max: usize) -> (String, bool) {
    if value.len() <= max {
        return (value, false);
    }
    let mut end = max;
    while !value.is_char_boundary(end) {
        end -= 1
    }
    (value[..end].to_string(), true)
}
