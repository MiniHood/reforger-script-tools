use crate::ast::AstSourceFile;
use crate::index::{GlobalSymbolId, SymbolIndex};
use crate::index_query::IndexQuery;
use crate::lexer::TextSpan;
use crate::model::{SourceFileMetadata, SymbolCatalog, SymbolKind};
use crate::parser::parse_source;
use crate::symbol_display::SymbolDisplayInfo;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::time::Instant;

const SERVER_NAME: &str = "reforger-language-server";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LspServerOptions {
    pub log_path: Option<PathBuf>,
    pub game_data_scripts: Option<PathBuf>,
}

pub fn run_stdio(options: LspServerOptions) -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run(stdin.lock(), stdout.lock(), options)
}

pub fn run<R: Read, W: Write>(
    reader: R,
    writer: W,
    options: LspServerOptions,
) -> Result<(), String> {
    let mut server = LspServer::new(writer, options);
    server.run(reader)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspDocumentSymbol {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub kind: u32,
    pub range: LspRange,
    pub selection_range: LspRange,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<LspDocumentSymbol>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDocumentSymbolReport {
    pub symbols: Vec<LspDocumentSymbol>,
    pub parse_diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspHover {
    pub contents: LspMarkupContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<LspRange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LspMarkupContent {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspHoverReport {
    pub hover: Option<LspHover>,
    pub parse_diagnostics: usize,
    pub selected_label: Option<String>,
    pub selected_kind: Option<SymbolKind>,
}

impl LspDocumentSymbolReport {
    pub fn total_symbol_count(&self) -> usize {
        document_symbol_count(&self.symbols)
    }
}

impl LspHoverReport {
    pub fn is_hit(&self) -> bool {
        self.hover.is_some()
    }
}

struct LspServer<W: Write> {
    writer: W,
    documents: BTreeMap<String, String>,
    log_path: Option<PathBuf>,
    shutdown_requested: bool,
}

#[derive(Debug, Deserialize)]
struct RpcMessage {
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DidOpenTextDocumentParams {
    text_document: TextDocumentItem,
}

#[derive(Debug, Deserialize)]
struct TextDocumentItem {
    uri: String,
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DidChangeTextDocumentParams {
    text_document: VersionedTextDocumentIdentifier,
    content_changes: Vec<TextDocumentContentChangeEvent>,
}

#[derive(Debug, Deserialize)]
struct VersionedTextDocumentIdentifier {
    uri: String,
}

#[derive(Debug, Deserialize)]
struct TextDocumentContentChangeEvent {
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DidCloseTextDocumentParams {
    text_document: TextDocumentIdentifier,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentSymbolParams {
    text_document: TextDocumentIdentifier,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HoverParams {
    text_document: TextDocumentIdentifier,
    position: LspPosition,
}

#[derive(Debug, Deserialize)]
struct TextDocumentIdentifier {
    uri: String,
}

impl<W: Write> LspServer<W> {
    fn new(writer: W, options: LspServerOptions) -> Self {
        let log_path = options.log_path;
        let server = Self {
            writer,
            documents: BTreeMap::new(),
            log_path,
            shutdown_requested: false,
        };
        server.log(&format!(
            "startup server={SERVER_NAME} version={SERVER_VERSION} game_data_scripts={}",
            options
                .game_data_scripts
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<unset>".to_string())
        ));
        server
    }

    fn run<R: Read>(&mut self, reader: R) -> Result<(), String> {
        let mut reader = BufReader::new(reader);
        while let Some(message) = read_message(&mut reader)? {
            let should_exit = self.handle_message(message)?;
            if should_exit {
                break;
            }
        }
        self.log("exit");
        Ok(())
    }

    fn handle_message(&mut self, value: Value) -> Result<bool, String> {
        let message = serde_json::from_value::<RpcMessage>(value.clone())
            .map_err(|error| format!("Invalid JSON-RPC message: {error}"))?;
        let Some(method) = message.method.as_deref() else {
            return Ok(false);
        };

        match method {
            "initialize" => {
                self.log("request initialize");
                if let Some(id) = message.id {
                    self.respond(
                        id,
                        json!({
                            "capabilities": {
                                "textDocumentSync": 1,
                                "documentSymbolProvider": true,
                                "hoverProvider": true
                            },
                            "serverInfo": {
                                "name": SERVER_NAME,
                                "version": SERVER_VERSION
                            }
                        }),
                    )?;
                }
            }
            "initialized" => {
                self.log("notification initialized");
            }
            "shutdown" => {
                self.log("request shutdown");
                self.shutdown_requested = true;
                if let Some(id) = message.id {
                    self.respond(id, Value::Null)?;
                }
            }
            "exit" => {
                self.log("notification exit");
                return Ok(true);
            }
            "textDocument/didOpen" => {
                if let Some(params) =
                    parse_params::<DidOpenTextDocumentParams>(message.params, method)?
                {
                    let start = Instant::now();
                    let uri = params.text_document.uri;
                    let text = params.text_document.text;
                    if self.log_path.is_some() {
                        let bytes = text.len();
                        let report = document_symbol_report_for_source(&text);
                        self.log(&format!(
                            "notification didOpen uri={} bytes={} symbols={} parse_diagnostics={} elapsed_ms={}",
                            uri,
                            bytes,
                            report.total_symbol_count(),
                            report.parse_diagnostics,
                            start.elapsed().as_millis()
                        ));
                    }
                    self.documents.insert(uri, text);
                }
            }
            "textDocument/didChange" => {
                if let Some(params) =
                    parse_params::<DidChangeTextDocumentParams>(message.params, method)?
                {
                    if let Some(change) = params.content_changes.into_iter().last() {
                        let start = Instant::now();
                        let uri = params.text_document.uri;
                        let text = change.text;
                        if self.log_path.is_some() {
                            let bytes = text.len();
                            let report = document_symbol_report_for_source(&text);
                            self.log(&format!(
                                "notification didChange uri={} bytes={} symbols={} parse_diagnostics={} elapsed_ms={}",
                                uri,
                                bytes,
                                report.total_symbol_count(),
                                report.parse_diagnostics,
                                start.elapsed().as_millis()
                            ));
                        }
                        self.documents.insert(uri, text);
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(params) =
                    parse_params::<DidCloseTextDocumentParams>(message.params, method)?
                {
                    self.log(&format!(
                        "notification didClose uri={}",
                        params.text_document.uri
                    ));
                    self.documents.remove(&params.text_document.uri);
                }
            }
            "textDocument/documentSymbol" => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<DocumentSymbolParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut symbol_count = 0usize;
                    let mut parse_diagnostics = 0usize;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|source| {
                                bytes = source.len();
                                let report = document_symbol_report_for_source(source);
                                symbol_count = report.total_symbol_count();
                                parse_diagnostics = report.parse_diagnostics;
                                report.symbols
                            })
                        })
                        .map(|symbols| serde_json::to_value(symbols).unwrap_or(Value::Null))
                        .unwrap_or(Value::Null);
                    self.log(&format!(
                        "request documentSymbol uri={} bytes={} symbols={} parse_diagnostics={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        symbol_count,
                        parse_diagnostics,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                }
            }
            "textDocument/hover" => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<HoverParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut parse_diagnostics = 0usize;
                    let mut selected_label = "<none>".to_string();
                    let mut selected_kind = "None";
                    let mut hit = false;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|source| {
                                bytes = source.len();
                                let report =
                                    hover_report_for_source_position(source, params.position);
                                parse_diagnostics = report.parse_diagnostics;
                                hit = report.is_hit();
                                if let Some(label) = report.selected_label {
                                    selected_label = label;
                                }
                                if let Some(kind) = report.selected_kind {
                                    selected_kind = symbol_kind_label(kind);
                                }
                                report.hover
                            })
                        })
                        .flatten()
                        .map(|hover| serde_json::to_value(hover).unwrap_or(Value::Null))
                        .unwrap_or(Value::Null);
                    self.log(&format!(
                        "request hover uri={} bytes={} hit={} label={} kind={} parse_diagnostics={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        hit,
                        selected_label,
                        selected_kind,
                        parse_diagnostics,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                }
            }
            _ => {
                if let Some(id) = message.id {
                    self.respond_error(id, -32601, &format!("Method not found: {method}"))?;
                }
            }
        }

        Ok(self.shutdown_requested && method == "exit")
    }

    fn respond(&mut self, id: Value, result: Value) -> Result<(), String> {
        self.write_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }))
    }

    fn respond_error(&mut self, id: Value, code: i32, message: &str) -> Result<(), String> {
        self.write_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        }))
    }

    fn write_message(&mut self, value: Value) -> Result<(), String> {
        let body = serde_json::to_vec(&value)
            .map_err(|error| format!("Failed to serialize LSP response: {error}"))?;
        write!(self.writer, "Content-Length: {}\r\n\r\n", body.len())
            .map_err(|error| format!("Failed to write LSP header: {error}"))?;
        self.writer
            .write_all(&body)
            .map_err(|error| format!("Failed to write LSP body: {error}"))?;
        self.writer
            .flush()
            .map_err(|error| format!("Failed to flush LSP response: {error}"))
    }

    fn log(&self, message: &str) {
        let Some(log_path) = &self.log_path else {
            return;
        };
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(file, "[{}] {message}", timestamp_millis());
        }
    }
}

pub fn document_symbols_for_source(source: &str) -> Vec<LspDocumentSymbol> {
    document_symbol_report_for_source(source).symbols
}

pub fn document_symbol_report_for_source(source: &str) -> LspDocumentSymbolReport {
    let analysis = file_index_for_source(source);
    let query = IndexQuery::new(&analysis.index);
    LspDocumentSymbolReport {
        symbols: document_symbols_from_index(source, &analysis.index, &query),
        parse_diagnostics: analysis.parse_diagnostics,
    }
}

pub fn hover_report_for_source_position(source: &str, position: LspPosition) -> LspHoverReport {
    let analysis = file_index_for_source(source);
    let Some(offset) = offset_for_position(source, position) else {
        return LspHoverReport {
            hover: None,
            parse_diagnostics: analysis.parse_diagnostics,
            selected_label: None,
            selected_kind: None,
        };
    };
    let Some(id) = hover_symbol_at_offset(&analysis.index, offset) else {
        return LspHoverReport {
            hover: None,
            parse_diagnostics: analysis.parse_diagnostics,
            selected_label: None,
            selected_kind: None,
        };
    };
    let query = IndexQuery::new(&analysis.index);
    let Some(display) = query.symbol_display(id) else {
        return LspHoverReport {
            hover: None,
            parse_diagnostics: analysis.parse_diagnostics,
            selected_label: None,
            selected_kind: None,
        };
    };
    let selected_kind = display.kind;
    let selected_label = display.label.clone();
    let symbol = analysis.index.symbol(id);
    let range = symbol.map(|symbol| range_for_span(source, symbol.selection_span));
    LspHoverReport {
        hover: Some(LspHover {
            contents: LspMarkupContent {
                kind: "markdown".to_string(),
                value: render_hover_markdown(&display),
            },
            range,
        }),
        parse_diagnostics: analysis.parse_diagnostics,
        selected_label: Some(selected_label),
        selected_kind: Some(selected_kind),
    }
}

struct FileIndexAnalysis {
    index: SymbolIndex,
    parse_diagnostics: usize,
}

fn file_index_for_source(source: &str) -> FileIndexAnalysis {
    let parse = parse_source(source);
    let parse_diagnostics = parse.diagnostics.len();
    let ast = AstSourceFile::new(source, &parse);
    let catalog = SymbolCatalog::from_ast_with_metadata(
        source,
        &ast,
        SourceFileMetadata {
            kind: crate::model::SourceKind::Workspace,
            category: crate::model::SourceCategory::Workspace,
            absolute_path: None,
            root_path: None,
            relative_path: None,
            priority: crate::model::SOURCE_PRIORITY_WORKSPACE,
        },
    );
    let mut index = SymbolIndex::default();
    index.add_catalog(&catalog);
    FileIndexAnalysis {
        index,
        parse_diagnostics,
    }
}

pub fn document_symbol_count(symbols: &[LspDocumentSymbol]) -> usize {
    symbols
        .iter()
        .map(|symbol| 1 + document_symbol_count(&symbol.children))
        .sum()
}

fn document_symbols_from_index(
    source: &str,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
) -> Vec<LspDocumentSymbol> {
    index
        .symbols()
        .iter()
        .filter(|symbol| symbol.parent.is_none())
        .filter(|symbol| symbol.kind != SymbolKind::Parameter)
        .filter_map(|symbol| document_symbol_for_id(source, index, query, symbol.id))
        .collect()
}

fn document_symbol_for_id(
    source: &str,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    id: GlobalSymbolId,
) -> Option<LspDocumentSymbol> {
    let symbol = index.symbol(id)?;
    if symbol.kind == SymbolKind::Parameter {
        return None;
    }
    let display = query.symbol_display(id)?;
    let children = index
        .children(id)
        .iter()
        .filter_map(|child| document_symbol_for_id(source, index, query, *child))
        .collect::<Vec<_>>();

    Some(LspDocumentSymbol {
        name: display.label,
        detail: display.detail.or(display.signature),
        kind: document_symbol_kind(symbol.kind),
        range: range_for_span(source, symbol.span),
        selection_range: range_for_span(source, symbol.selection_span),
        children,
    })
}

fn range_for_span(source: &str, span: crate::lexer::TextSpan) -> LspRange {
    LspRange {
        start: position_for_offset(source, span.start),
        end: position_for_offset(source, span.end),
    }
}

pub fn position_for_offset(source: &str, offset: usize) -> LspPosition {
    let mut line = 0u32;
    let mut character = 0u32;

    for (index, value) in source.char_indices() {
        if index >= offset {
            break;
        }
        if value == '\n' {
            line += 1;
            character = 0;
        } else {
            character += value.len_utf16() as u32;
        }
    }

    LspPosition { line, character }
}

pub fn offset_for_position(source: &str, position: LspPosition) -> Option<usize> {
    let mut line = 0u32;
    let mut character = 0u32;

    for (index, value) in source.char_indices() {
        if line == position.line {
            if character == position.character {
                return Some(index);
            }
            if value == '\n' {
                return None;
            }
            let next_character = character + value.len_utf16() as u32;
            if position.character < next_character {
                return Some(index);
            }
            character = next_character;
        } else if value == '\n' {
            line += 1;
            character = 0;
        }
    }

    if line == position.line && character == position.character {
        Some(source.len())
    } else {
        None
    }
}

fn hover_symbol_at_offset(index: &SymbolIndex, offset: usize) -> Option<GlobalSymbolId> {
    index
        .symbols()
        .iter()
        .filter_map(|symbol| {
            let selection_hit = span_contains_offset(symbol.selection_span, offset);
            let span_hit = span_contains_offset(symbol.span, offset);
            if !selection_hit && !span_hit {
                return None;
            }
            let matched_span = if selection_hit {
                symbol.selection_span
            } else {
                symbol.span
            };
            Some((
                !selection_hit,
                matched_span.end.saturating_sub(matched_span.start),
                symbol.id,
            ))
        })
        .min_by_key(|(span_rank, span_len, id)| (*span_rank, *span_len, id.file_id, id.symbol_id))
        .map(|(_, _, id)| id)
}

fn span_contains_offset(span: TextSpan, offset: usize) -> bool {
    span.start <= offset && offset < span.end
}

fn render_hover_markdown(display: &SymbolDisplayInfo) -> String {
    let code = display.signature.as_ref().unwrap_or(&display.label);
    let mut sections = Vec::new();
    sections.push(format!("```enforce\n{code}\n```"));

    if let Some(detail) = &display.detail {
        if detail != code {
            sections.push(detail.clone());
        }
    }
    if let Some(preview) = &display.documentation_preview {
        sections.push(preview.clone());
    }
    if !display.modifiers.is_empty() {
        sections.push(format!("**Modifiers:** {}", display.modifiers.join(", ")));
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
        sections.push(format!("**Attributes:** {}", attribute_names.join(", ")));
    }

    sections.join("\n\n")
}

fn document_symbol_kind(kind: SymbolKind) -> u32 {
    match kind {
        SymbolKind::Class => 5,
        SymbolKind::Enum => 10,
        SymbolKind::EnumMember => 22,
        SymbolKind::Typedef => 26,
        SymbolKind::Function => 12,
        SymbolKind::GlobalField => 13,
        SymbolKind::Field => 8,
        SymbolKind::Method => 6,
        SymbolKind::Constructor => 9,
        SymbolKind::Destructor => 6,
        SymbolKind::Parameter => 13,
    }
}

pub fn symbol_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "Class",
        SymbolKind::Enum => "Enum",
        SymbolKind::EnumMember => "EnumMember",
        SymbolKind::Typedef => "Typedef",
        SymbolKind::Function => "Function",
        SymbolKind::GlobalField => "GlobalField",
        SymbolKind::Field => "Field",
        SymbolKind::Method => "Method",
        SymbolKind::Constructor => "Constructor",
        SymbolKind::Destructor => "Destructor",
        SymbolKind::Parameter => "Parameter",
    }
}

fn parse_params<T: for<'de> Deserialize<'de>>(
    params: Option<Value>,
    method: &str,
) -> Result<Option<T>, String> {
    let Some(params) = params else {
        return Ok(None);
    };
    serde_json::from_value(params)
        .map(Some)
        .map_err(|error| format!("Invalid params for {method}: {error}"))
}

fn read_message<R: BufRead>(reader: &mut R) -> Result<Option<Value>, String> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("Failed to read LSP header: {error}"))?;
        if bytes == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|error| format!("Invalid Content-Length: {error}"))?,
            );
        }
    }

    let Some(content_length) = content_length else {
        return Err("Missing Content-Length header".to_string());
    };
    let mut body = vec![0u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("Failed to read LSP body: {error}"))?;
    serde_json::from_slice(&body).map_err(|error| format!("Invalid LSP JSON body: {error}"))
}

fn timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_symbols_include_nested_declarations() {
        let source = r#"class Example
{
	int m_Value;
	void Run(int value);
}
"#;

        let symbols = document_symbols_for_source(source);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Example");
        assert_eq!(symbols[0].kind, 5);
        assert!(symbols[0]
            .children
            .iter()
            .any(|child| child.name == "m_Value" && child.kind == 8));
        assert!(symbols[0]
            .children
            .iter()
            .any(|child| child.name == "Run" && child.kind == 6));
        let run = symbols[0]
            .children
            .iter()
            .find(|child| child.name == "Run")
            .unwrap();
        assert!(run.children.is_empty());
    }

    #[test]
    fn positions_are_zero_based_utf16() {
        let source = "class A\n{\n\tstring Name;\n}\n";
        let symbols = document_symbols_for_source(source);

        assert_eq!(
            symbols[0].range.start,
            LspPosition {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            symbols[0].selection_range.start,
            LspPosition {
                line: 0,
                character: 6
            }
        );
    }

    #[test]
    fn document_symbols_cover_declared_kinds_and_sane_ranges() {
        let source = r#"//! Global typedef docs
typedef string FactionKey;

Game g_Game;

[EnumBitFlag()]
enum ExampleFlags
{
	None = 0,
	Enabled = 1
}

class Example
{
	int m_Value;
	void Example(int value);
	void ~Example();
	void Run(string name);
}
"#;

        let report = document_symbol_report_for_source(source);

        assert_eq!(report.parse_diagnostics, 0);
        assert!(report
            .symbols
            .iter()
            .any(|symbol| symbol.name == "FactionKey" && symbol.kind == 26));
        assert!(report
            .symbols
            .iter()
            .any(|symbol| symbol.name == "g_Game" && symbol.kind == 13));
        let enum_symbol = report
            .symbols
            .iter()
            .find(|symbol| symbol.name == "ExampleFlags")
            .unwrap();
        assert_eq!(enum_symbol.kind, 10);
        assert!(enum_symbol
            .children
            .iter()
            .any(|child| child.name == "Enabled" && child.kind == 22));

        let class_symbol = report
            .symbols
            .iter()
            .find(|symbol| symbol.name == "Example")
            .unwrap();
        assert_eq!(class_symbol.kind, 5);
        assert!(class_symbol
            .children
            .iter()
            .any(|child| child.name == "m_Value" && child.kind == 8));
        assert!(class_symbol
            .children
            .iter()
            .any(|child| child.name == "Example" && child.kind == 9));
        assert!(class_symbol
            .children
            .iter()
            .any(|child| child.name == "Example" && child.kind == 6));
        assert!(class_symbol
            .children
            .iter()
            .any(|child| child.name == "Run" && child.kind == 6));

        assert_ranges_are_sane(&report.symbols);
    }

    #[test]
    fn hover_selects_class_method_field_parameter_typedef_enum_member_and_global() {
        let source = r#"//! Global typedef docs
typedef string FactionKey;

Game g_Game;

[EnumBitFlag()]
enum ExampleFlags
{
	None = 0,
	Enabled = 1
}

//! Class docs.
class Example : Base
{
	[Attribute("0")]
	protected int m_Value;
	void Run(string name);
}
"#;

        assert_hover(
            source,
            "Example : Base",
            "Example",
            SymbolKind::Class,
            "Example",
        );
        assert_hover(source, "m_Value", "m_Value", SymbolKind::Field, "m_Value");
        assert_hover(source, "Run(string", "Run", SymbolKind::Method, "Run");
        assert_hover(source, "string name", "name", SymbolKind::Parameter, "name");
        assert_hover(
            source,
            "FactionKey;",
            "FactionKey",
            SymbolKind::Typedef,
            "FactionKey",
        );
        assert_hover(
            source,
            "Enabled = 1",
            "Enabled",
            SymbolKind::EnumMember,
            "Enabled",
        );
        assert_hover(
            source,
            "g_Game",
            "g_Game",
            SymbolKind::GlobalField,
            "g_Game",
        );
    }

    #[test]
    fn hover_returns_none_for_whitespace_outside_symbols() {
        let source = "\nclass Example {}\n";

        let report = hover_report_for_source_position(
            source,
            LspPosition {
                line: 0,
                character: 0,
            },
        );

        assert!(!report.is_hit());
        assert_eq!(report.parse_diagnostics, 0);
    }

    #[test]
    fn hover_markdown_uses_signature_detail_docs_modifiers_and_attributes() {
        let source = r#"//! Runs the example.
class Example
{
	//! Runs the example.
	[Attribute("0")]
	protected void Run(int value = 4);
}
"#;

        let report = hover_at(source, "Run(int", "Run");
        let hover = report.hover.unwrap();
        let markdown = hover.contents.value;

        assert!(markdown.contains("```enforce\nExample.Run(int value = 4) -> void\n```"));
        assert!(markdown.contains("Runs the example."));
        assert!(markdown.contains("**Modifiers:** protected"));
        assert!(markdown.contains("**Attributes:** Attribute"));
    }

    #[test]
    fn offset_conversion_uses_utf16_positions() {
        let source = "class Sm😀ke {}\n";
        let offset = source.find("ke").unwrap();

        let position = position_for_offset(source, offset);

        assert_eq!(
            position,
            LspPosition {
                line: 0,
                character: 10
            }
        );
        assert_eq!(offset_for_position(source, position), Some(offset));
        assert_eq!(
            offset_for_position(
                source,
                LspPosition {
                    line: 0,
                    character: 9
                }
            ),
            Some(source.find('😀').unwrap())
        );
    }

    #[test]
    fn framed_lsp_smoke_test_handles_open_and_document_symbol() {
        let source = "class Smoke\n{\n\tvoid Run();\n}\n";
        let mut input = Vec::new();
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Smoke.c",
                        "languageId": "enforce",
                        "version": 1,
                        "text": source
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Smoke.c"
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "shutdown",
                "params": null
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null
            }),
        );

        let mut output = Vec::new();
        run(
            input.as_slice(),
            &mut output,
            LspServerOptions {
                log_path: None,
                game_data_scripts: None,
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"documentSymbolProvider\":true"));
        assert!(output_text.contains("\"name\":\"Smoke\""));
        assert!(output_text.contains("\"name\":\"Run\""));
    }

    #[test]
    fn framed_lsp_smoke_test_handles_hover() {
        let source = "class Smoke\n{\n\tvoid Run(int value);\n}\n";
        let hover_position = position_for_needle(source, "Run(int", "Run");
        let mut input = Vec::new();
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Smoke.c",
                        "languageId": "enforce",
                        "version": 1,
                        "text": source
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Smoke.c"
                    },
                    "position": {
                        "line": hover_position.line,
                        "character": hover_position.character
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "shutdown",
                "params": null
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null
            }),
        );

        let mut output = Vec::new();
        run(
            input.as_slice(),
            &mut output,
            LspServerOptions {
                log_path: None,
                game_data_scripts: None,
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"hoverProvider\":true"));
        assert!(output_text.contains("Smoke.Run(int value) -> void"));
        assert!(output_text.contains("\"kind\":\"markdown\""));
    }

    fn assert_ranges_are_sane(symbols: &[LspDocumentSymbol]) {
        for symbol in symbols {
            assert!(
                range_contains(symbol.range, symbol.selection_range),
                "selection range must be inside declaration range for {}",
                symbol.name
            );
            assert_ranges_are_sane(&symbol.children);
        }
    }

    fn range_contains(outer: LspRange, inner: LspRange) -> bool {
        position_le(outer.start, inner.start) && position_le(inner.end, outer.end)
    }

    fn position_le(left: LspPosition, right: LspPosition) -> bool {
        (left.line, left.character) <= (right.line, right.character)
    }

    fn write_test_message(output: &mut Vec<u8>, value: Value) {
        let body = serde_json::to_vec(&value).unwrap();
        write!(output, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        output.extend_from_slice(&body);
    }

    fn assert_hover(
        source: &str,
        needle: &str,
        cursor: &str,
        expected_kind: SymbolKind,
        expected_label: &str,
    ) {
        let report = hover_at(source, needle, cursor);

        assert_eq!(report.parse_diagnostics, 0);
        assert_eq!(report.selected_kind, Some(expected_kind));
        assert_eq!(report.selected_label.as_deref(), Some(expected_label));
        assert!(report.hover.is_some());
    }

    fn hover_at(source: &str, needle: &str, cursor: &str) -> LspHoverReport {
        hover_report_for_source_position(source, position_for_needle(source, needle, cursor))
    }

    fn position_for_needle(source: &str, needle: &str, cursor: &str) -> LspPosition {
        let start = source
            .find(needle)
            .unwrap_or_else(|| panic!("missing needle {needle}"));
        let cursor_start = needle
            .find(cursor)
            .unwrap_or_else(|| panic!("missing cursor {cursor} in {needle}"));
        position_for_offset(source, start + cursor_start)
    }
}
