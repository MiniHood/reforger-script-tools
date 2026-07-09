use crate::ast::AstSourceFile;
use crate::index::{GlobalSymbolId, SymbolIndex};
use crate::index_query::IndexQuery;
use crate::model::{SourceFileMetadata, SymbolCatalog, SymbolKind};
use crate::parser::parse_source;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDocumentSymbolReport {
    pub symbols: Vec<LspDocumentSymbol>,
    pub parse_diagnostics: usize,
}

impl LspDocumentSymbolReport {
    pub fn total_symbol_count(&self) -> usize {
        document_symbol_count(&self.symbols)
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
                                "documentSymbolProvider": true
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
    let query = IndexQuery::new(&index);
    LspDocumentSymbolReport {
        symbols: document_symbols_from_index(source, &index, &query),
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

fn position_for_offset(source: &str, offset: usize) -> LspPosition {
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
}
