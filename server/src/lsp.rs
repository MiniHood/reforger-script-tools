use crate::ast::AstSourceFile;
use crate::index::{GlobalSymbolId, SymbolIndex};
use crate::index_cache::{
    load_or_build_game_data_index, GameDataIndexCacheConfig, RuntimeIndexSummary,
};
use crate::index_query::IndexQuery;
use crate::lexer::{lex, Keyword, TextSpan, Token, TokenKind};
use crate::model::{SourceFileMetadata, SymbolCatalog, SymbolKind};
use crate::parser::parse_source;
use crate::resolver::{
    CandidateSource, HoverResolution, IdentifierContext, ReceiverResolution, ReferenceResolver,
    ResolutionReason,
};
use crate::symbol_display::SymbolDisplayInfo;
use crate::syntax::ParseDiagnostic;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

const SERVER_NAME: &str = "reforger-language-server";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEBUG_HOVER_METHOD: &str = "reforger/debugHover";
const DEBUG_TOKEN_CONTEXT: usize = 8;
const DEBUG_CANDIDATE_LIMIT: usize = 20;
const DEBUG_CHILD_LIMIT: usize = 20;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LspServerOptions {
    pub log_path: Option<PathBuf>,
    pub game_data_scripts: Option<PathBuf>,
    pub game_data_metadata: Option<PathBuf>,
    pub index_cache: Option<PathBuf>,
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
    pub selected_source: Option<CandidateSource>,
    pub selection_source: HoverSelectionSource,
    pub resolver_reason: Option<ResolutionReason>,
    pub identifier_context: Option<IdentifierContext>,
    pub resolver_candidate_count: usize,
    pub receiver_resolution: Option<ReceiverResolution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverSelectionSource {
    ResolverIdentifier,
    ResolverSyntaxSpan,
    None,
}

impl HoverSelectionSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ResolverIdentifier => "resolver-identifier",
            Self::ResolverSyntaxSpan => "resolver-syntax-span",
            Self::None => "none",
        }
    }
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
    documents: BTreeMap<String, OpenDocument>,
    logger: LspLogger,
    external_index: ExternalIndexHandle,
    shutdown_requested: bool,
}

struct OpenDocument {
    text: String,
    version: Option<i32>,
    revision: u64,
    analysis: FileIndexAnalysis,
}

impl OpenDocument {
    fn new(text: String, version: Option<i32>, revision: u64) -> Self {
        let analysis = file_index_for_source(&text);
        Self {
            text,
            version,
            revision,
            analysis,
        }
    }

    fn replace(&mut self, text: String, version: Option<i32>) {
        self.text = text;
        self.version = version;
        self.revision += 1;
        self.analysis = file_index_for_source(&self.text);
    }
}

#[derive(Clone)]
struct LspLogger {
    path: Option<PathBuf>,
    lock: Arc<Mutex<()>>,
}

impl LspLogger {
    fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            lock: Arc::new(Mutex::new(())),
        }
    }

    fn log(&self, message: &str) {
        let Some(log_path) = self.path.as_ref() else {
            return;
        };
        let Ok(_guard) = self.lock.lock() else {
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

#[derive(Clone)]
struct ExternalIndexHandle {
    state: Arc<Mutex<ExternalIndexState>>,
}

#[derive(Debug)]
struct ExternalIndexState {
    status: ExternalIndexStatus,
    index: Option<SymbolIndex>,
    summary: Option<RuntimeIndexSummary>,
    cache_status: Option<String>,
    cache_detail: Option<String>,
    fingerprint: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExternalIndexStatus {
    Missing,
    Building,
    Ready,
    Failed,
}

impl ExternalIndexStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Building => "building",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
struct ExternalIndexStatusSummary {
    status: &'static str,
    files: usize,
    symbols: usize,
    parse_diagnostics: usize,
    cache_status: Option<String>,
    cache_detail: Option<String>,
    fingerprint: Option<String>,
    error: Option<String>,
}

impl ExternalIndexHandle {
    fn missing() -> Self {
        Self {
            state: Arc::new(Mutex::new(ExternalIndexState {
                status: ExternalIndexStatus::Missing,
                index: None,
                summary: None,
                cache_status: None,
                cache_detail: None,
                fingerprint: None,
                error: None,
            })),
        }
    }

    fn status_summary(&self) -> ExternalIndexStatusSummary {
        let state = self.state.lock().unwrap();
        let summary = state.summary.as_ref();
        ExternalIndexStatusSummary {
            status: state.status.as_str(),
            files: summary.map(|summary| summary.files).unwrap_or(0),
            symbols: summary.map(|summary| summary.indexed_symbols).unwrap_or(0),
            parse_diagnostics: summary
                .map(|summary| summary.parse_diagnostics)
                .unwrap_or(0),
            cache_status: state.cache_status.clone(),
            cache_detail: state.cache_detail.clone(),
            fingerprint: state.fingerprint.clone(),
            error: state.error.clone(),
        }
    }
}

fn start_external_index(options: &LspServerOptions, logger: LspLogger) -> ExternalIndexHandle {
    let Some(scripts_root) = options.game_data_scripts.clone() else {
        return ExternalIndexHandle::missing();
    };
    let Some(cache_path) = options.index_cache.clone() else {
        return ExternalIndexHandle::missing();
    };

    let handle = ExternalIndexHandle {
        state: Arc::new(Mutex::new(ExternalIndexState {
            status: ExternalIndexStatus::Building,
            index: None,
            summary: None,
            cache_status: None,
            cache_detail: None,
            fingerprint: None,
            error: None,
        })),
    };

    let state = handle.state.clone();
    let metadata_path = options.game_data_metadata.clone();
    thread::spawn(move || {
        let start = Instant::now();
        logger.log(&format!(
            "externalIndex start scripts={} cache={}",
            scripts_root.display(),
            cache_path.display()
        ));
        let result = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root,
            cache_path,
            metadata_path,
        });

        let mut state = state.lock().unwrap();
        match result {
            Ok(result) => {
                let cache_status = result.cache_status.as_str().to_string();
                let cache_detail = result.cache_status.detail().map(str::to_string);
                let fingerprint = result.fingerprint.summary();
                logger.log(&format!(
                    "externalIndex ready cache_status={} cache_detail={} files={} symbols={} parse_diagnostics={} elapsed_ms={}",
                    cache_status,
                    cache_detail.as_deref().unwrap_or("<none>"),
                    result.summary.files,
                    result.summary.indexed_symbols,
                    result.summary.parse_diagnostics,
                    start.elapsed().as_millis()
                ));
                state.status = ExternalIndexStatus::Ready;
                state.index = Some(result.index);
                state.summary = Some(result.summary);
                state.cache_status = Some(cache_status);
                state.cache_detail = cache_detail;
                state.fingerprint = Some(fingerprint);
                state.error = None;
            }
            Err(error) => {
                logger.log(&format!(
                    "externalIndex failed error={} elapsed_ms={}",
                    error,
                    start.elapsed().as_millis()
                ));
                state.status = ExternalIndexStatus::Failed;
                state.index = None;
                state.summary = None;
                state.cache_status = None;
                state.cache_detail = None;
                state.fingerprint = None;
                state.error = Some(error);
            }
        }
    });

    handle
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
    version: Option<i32>,
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
    version: Option<i32>,
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
        let logger = LspLogger::new(options.log_path.clone());
        let external_index = start_external_index(&options, logger.clone());
        let server = Self {
            writer,
            documents: BTreeMap::new(),
            logger,
            external_index,
            shutdown_requested: false,
        };
        server.log(&format!(
            "startup server={SERVER_NAME} version={SERVER_VERSION} game_data_scripts={} index_cache={} external_index_status={}",
            options
                .game_data_scripts
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<unset>".to_string()),
            options
                .index_cache
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<unset>".to_string()),
            server.external_index.status_summary().status
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
                    let version = params.text_document.version;
                    let text = params.text_document.text;
                    let bytes = text.len();
                    let document = OpenDocument::new(text, version, 1);
                    let symbols =
                        document_symbols_from_cached_analysis(&document.text, &document.analysis);
                    let symbol_count = document_symbol_count(&symbols);
                    let parse_diagnostics = document.analysis.parse_diagnostics;
                    let revision = document.revision;
                    self.documents.insert(uri.clone(), document);
                    self.log(&format!(
                        "notification didOpen uri={} bytes={} version={} revision={} cached_analysis=true symbols={} parse_diagnostics={} analysis_elapsed_ms={}",
                        uri,
                        bytes,
                        format_optional_i32(version),
                        revision,
                        symbol_count,
                        parse_diagnostics,
                        start.elapsed().as_millis()
                    ));
                }
            }
            "textDocument/didChange" => {
                if let Some(params) =
                    parse_params::<DidChangeTextDocumentParams>(message.params, method)?
                {
                    if let Some(change) = params.content_changes.into_iter().last() {
                        let start = Instant::now();
                        let uri = params.text_document.uri;
                        let version = params.text_document.version;
                        let text = change.text;
                        let bytes = text.len();
                        let document = self
                            .documents
                            .entry(uri.clone())
                            .or_insert_with(|| OpenDocument::new(String::new(), None, 0));
                        document.replace(text, version);
                        let symbols = document_symbols_from_cached_analysis(
                            &document.text,
                            &document.analysis,
                        );
                        let symbol_count = document_symbol_count(&symbols);
                        let parse_diagnostics = document.analysis.parse_diagnostics;
                        let revision = document.revision;
                        self.log(&format!(
                            "notification didChange uri={} bytes={} version={} revision={} cached_analysis=true symbols={} parse_diagnostics={} analysis_elapsed_ms={}",
                            uri,
                            bytes,
                            format_optional_i32(version),
                            revision,
                            symbol_count,
                            parse_diagnostics,
                            start.elapsed().as_millis()
                        ));
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
                    let mut revision = 0u64;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let symbols = document_symbols_from_cached_analysis(
                                    &document.text,
                                    &document.analysis,
                                );
                                symbol_count = document_symbol_count(&symbols);
                                parse_diagnostics = document.analysis.parse_diagnostics;
                                symbols
                            })
                        })
                        .map(|symbols| serde_json::to_value(symbols).unwrap_or(Value::Null))
                        .unwrap_or(Value::Null);
                    self.log(&format!(
                        "request documentSymbol uri={} bytes={} revision={} cached_analysis=true symbols={} parse_diagnostics={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
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
                    let mut selected_source = "<none>";
                    let mut selection_source = HoverSelectionSource::None;
                    let mut resolver_reason = "<none>";
                    let mut identifier_context = "<none>";
                    let mut resolver_candidate_count = 0usize;
                    let mut receiver_owner = "<none>".to_string();
                    let mut receiver_failure = "<none>".to_string();
                    let mut external_index_status = self.external_index.status_summary().status;
                    let mut revision = 0u64;
                    let mut hit = false;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let external_index = self.external_index.state.lock().unwrap();
                                external_index_status = external_index.status.as_str();
                                let report = hover_report_for_cached_analysis_with_external(
                                    &document.text,
                                    &document.analysis,
                                    params.position,
                                    external_index.index.as_ref(),
                                );
                                parse_diagnostics = report.parse_diagnostics;
                                hit = report.is_hit();
                                selection_source = report.selection_source;
                                selected_source = report
                                    .selected_source
                                    .map(|source| source.as_str())
                                    .unwrap_or("<none>");
                                resolver_reason = report
                                    .resolver_reason
                                    .map(|reason| reason.as_str())
                                    .unwrap_or("<none>");
                                identifier_context = report
                                    .identifier_context
                                    .map(|context| context.as_str())
                                    .unwrap_or("<none>");
                                resolver_candidate_count = report.resolver_candidate_count;
                                if let Some(receiver) = report.receiver_resolution.as_ref() {
                                    receiver_owner = receiver
                                        .owner_type
                                        .as_deref()
                                        .unwrap_or("<none>")
                                        .to_string();
                                    receiver_failure = receiver
                                        .failure_reason
                                        .as_deref()
                                        .unwrap_or("<none>")
                                        .to_string();
                                }
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
                        "request hover uri={} bytes={} revision={} cached_analysis=true hit={} selection_source={} selected_source={} resolver_reason={} identifier_context={} resolver_candidates={} receiver_owner={} receiver_failure={} external_index_status={} label={} kind={} parse_diagnostics={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        hit,
                        selection_source.as_str(),
                        selected_source,
                        resolver_reason,
                        identifier_context,
                        resolver_candidate_count,
                        receiver_owner,
                        receiver_failure,
                        external_index_status,
                        selected_label,
                        selected_kind,
                        parse_diagnostics,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                }
            }
            DEBUG_HOVER_METHOD => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<HoverParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut revision = 0u64;
                    let mut hit = false;
                    let mut selected_label = "<none>".to_string();
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let external_status = self.external_index.status_summary();
                                let external_index = self.external_index.state.lock().unwrap();
                                let report = debug_hover_report_for_cached_analysis_with_external(
                                    &document.text,
                                    &document.analysis,
                                    params.position,
                                    external_index.index.as_ref(),
                                    Some(&external_status),
                                );
                                hit = report.contains("Selected Symbol: yes");
                                if let Some(label) = selected_label_from_debug_report(&report) {
                                    selected_label = label;
                                }
                                Value::String(report)
                            })
                        })
                        .unwrap_or_else(|| {
                            Value::String(format!(
                                "# Reforger Hover Debug\n\nNo open document text found for `{}`.",
                                log_uri
                            ))
                        });
                    self.log(&format!(
                        "request debugHover uri={} bytes={} revision={} cached_analysis=true hit={} label={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        hit,
                        selected_label,
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
        self.logger.log(message);
    }
}

pub fn document_symbols_for_source(source: &str) -> Vec<LspDocumentSymbol> {
    document_symbol_report_for_source(source).symbols
}

pub fn document_symbol_report_for_source(source: &str) -> LspDocumentSymbolReport {
    let analysis = file_index_for_source(source);
    document_symbol_report_for_cached_analysis(source, &analysis)
}

fn document_symbol_report_for_cached_analysis(
    source: &str,
    analysis: &FileIndexAnalysis,
) -> LspDocumentSymbolReport {
    let query = IndexQuery::new(&analysis.index);
    LspDocumentSymbolReport {
        symbols: document_symbols_from_index(source, &analysis.index, &query),
        parse_diagnostics: analysis.parse_diagnostics,
    }
}

fn document_symbols_from_cached_analysis(
    source: &str,
    analysis: &FileIndexAnalysis,
) -> Vec<LspDocumentSymbol> {
    document_symbol_report_for_cached_analysis(source, analysis).symbols
}

pub fn hover_report_for_source_position(source: &str, position: LspPosition) -> LspHoverReport {
    hover_report_for_source_position_with_external(source, position, None)
}

pub fn hover_report_for_source_position_with_external(
    source: &str,
    position: LspPosition,
    external_index: Option<&SymbolIndex>,
) -> LspHoverReport {
    let analysis = file_index_for_source(source);
    hover_report_for_cached_analysis_with_external(source, &analysis, position, external_index)
}

fn hover_report_for_cached_analysis_with_external(
    source: &str,
    analysis: &FileIndexAnalysis,
    position: LspPosition,
    external_index: Option<&SymbolIndex>,
) -> LspHoverReport {
    let Some(offset) = offset_for_position(source, position) else {
        return empty_hover_report(analysis.parse_diagnostics);
    };
    hover_report_for_offset(source, &analysis, offset, external_index)
}

pub fn hover_reports_for_source_positions(
    source: &str,
    positions: &[LspPosition],
) -> Vec<LspHoverReport> {
    hover_reports_for_source_positions_with_external(source, positions, None)
}

pub fn hover_reports_for_source_positions_with_external(
    source: &str,
    positions: &[LspPosition],
    external_index: Option<&SymbolIndex>,
) -> Vec<LspHoverReport> {
    let analysis = file_index_for_source(source);
    positions
        .iter()
        .map(|position| {
            offset_for_position(source, *position)
                .map(|offset| hover_report_for_offset(source, &analysis, offset, external_index))
                .unwrap_or_else(|| empty_hover_report(analysis.parse_diagnostics))
        })
        .collect()
}

fn hover_report_for_offset(
    source: &str,
    analysis: &FileIndexAnalysis,
    offset: usize,
    external_index: Option<&SymbolIndex>,
) -> LspHoverReport {
    let query = IndexQuery::new(&analysis.index);
    let resolver = ReferenceResolver::new(source, &analysis.index, external_index);
    match resolver.resolve_hover_at_offset(offset) {
        Some(HoverResolution::Identifier(resolution)) => {
            let candidate_count = resolution.candidates.len();
            let reason = resolution.reason;
            let identifier_context = resolution.identifier_context;
            if let Some(selected) = resolution.selected.as_ref() {
                match selected.source {
                    CandidateSource::FileLocal => {
                        if let Some(mut report) = hover_report_for_symbol(
                            source,
                            &analysis.index,
                            &query,
                            selected.id,
                            None,
                            HoverSelectionSource::ResolverIdentifier,
                            Some(CandidateSource::FileLocal),
                            Some(reason),
                            Some(identifier_context),
                            candidate_count,
                        ) {
                            report.receiver_resolution = resolution.receiver.clone();
                            return report.with_parse_diagnostics(analysis.parse_diagnostics);
                        }
                    }
                    CandidateSource::External => {
                        if let Some(external_index) = external_index {
                            let external_query = IndexQuery::new(external_index);
                            if let Some(mut report) = hover_report_for_symbol(
                                source,
                                external_index,
                                &external_query,
                                selected.id,
                                Some(range_for_span(source, resolution.token_span)),
                                HoverSelectionSource::ResolverIdentifier,
                                Some(CandidateSource::External),
                                Some(reason),
                                Some(identifier_context),
                                candidate_count,
                            ) {
                                report.receiver_resolution = resolution.receiver.clone();
                                return report.with_parse_diagnostics(analysis.parse_diagnostics);
                            }
                        }
                    }
                }
            }
            LspHoverReport {
                hover: None,
                parse_diagnostics: analysis.parse_diagnostics,
                selected_label: None,
                selected_kind: None,
                selected_source: None,
                selection_source: HoverSelectionSource::None,
                resolver_reason: Some(reason),
                identifier_context: Some(identifier_context),
                resolver_candidate_count: candidate_count,
                receiver_resolution: resolution.receiver.clone(),
            }
        }
        Some(HoverResolution::SyntaxSpan(resolution)) => {
            let Some(selected) = resolution.selected.as_ref() else {
                return empty_hover_report(analysis.parse_diagnostics);
            };
            hover_report_for_symbol(
                source,
                &analysis.index,
                &query,
                selected.id,
                None,
                HoverSelectionSource::ResolverSyntaxSpan,
                Some(CandidateSource::FileLocal),
                Some(resolution.reason),
                None,
                resolution.candidates.len(),
            )
            .map(|report| report.with_parse_diagnostics(analysis.parse_diagnostics))
            .unwrap_or_else(|| empty_hover_report(analysis.parse_diagnostics))
        }
        None => empty_hover_report(analysis.parse_diagnostics),
    }
}

fn hover_report_for_symbol(
    source: &str,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    id: GlobalSymbolId,
    range_override: Option<LspRange>,
    selection_source: HoverSelectionSource,
    selected_source: Option<CandidateSource>,
    resolver_reason: Option<ResolutionReason>,
    identifier_context: Option<IdentifierContext>,
    resolver_candidate_count: usize,
) -> Option<LspHoverReport> {
    let display = query.symbol_display(id)?;
    let selected_kind = display.kind;
    let selected_label = display.label.clone();
    let symbol = index.symbol(id);
    let range = range_override
        .or_else(|| symbol.map(|symbol| range_for_span(source, symbol.selection_span)));
    Some(LspHoverReport {
        hover: Some(LspHover {
            contents: LspMarkupContent {
                kind: "markdown".to_string(),
                value: render_hover_markdown(&display),
            },
            range,
        }),
        parse_diagnostics: 0,
        selected_label: Some(selected_label),
        selected_kind: Some(selected_kind),
        selected_source,
        selection_source,
        resolver_reason,
        identifier_context,
        resolver_candidate_count,
        receiver_resolution: None,
    })
}

impl LspHoverReport {
    fn with_parse_diagnostics(mut self, parse_diagnostics: usize) -> Self {
        self.parse_diagnostics = parse_diagnostics;
        self
    }
}

fn empty_hover_report(parse_diagnostics: usize) -> LspHoverReport {
    LspHoverReport {
        hover: None,
        parse_diagnostics,
        selected_label: None,
        selected_kind: None,
        selected_source: None,
        selection_source: HoverSelectionSource::None,
        resolver_reason: None,
        identifier_context: None,
        resolver_candidate_count: 0,
        receiver_resolution: None,
    }
}

pub fn debug_hover_report_for_source_position(source: &str, position: LspPosition) -> String {
    debug_hover_report_for_source_position_with_external(source, position, None, None)
}

fn debug_hover_report_for_source_position_with_external(
    source: &str,
    position: LspPosition,
    external_index: Option<&SymbolIndex>,
    external_status: Option<&ExternalIndexStatusSummary>,
) -> String {
    let analysis = file_index_for_source(source);
    debug_hover_report_for_cached_analysis_with_external(
        source,
        &analysis,
        position,
        external_index,
        external_status,
    )
}

fn debug_hover_report_for_cached_analysis_with_external(
    source: &str,
    analysis: &FileIndexAnalysis,
    position: LspPosition,
    external_index: Option<&SymbolIndex>,
    external_status: Option<&ExternalIndexStatusSummary>,
) -> String {
    let start = Instant::now();
    let index = &analysis.index;
    let query = IndexQuery::new(index);
    let offset = offset_for_position(source, position);
    let tokens = lex(source);
    let resolver = ReferenceResolver::new(source, index, external_index);
    let resolver_resolution = offset.and_then(|offset| resolver.resolve_at_offset(offset));
    let candidates = offset
        .map(|offset| resolver.syntax_span_candidates_at_offset(offset))
        .unwrap_or_default();
    let selected_id = resolver_resolution
        .as_ref()
        .and_then(|resolution| resolution.selected.as_ref())
        .filter(|candidate| candidate.source == CandidateSource::FileLocal)
        .map(|candidate| candidate.id)
        .or_else(|| {
            if resolver_resolution.is_none() {
                candidates.first().map(|candidate| candidate.id)
            } else {
                None
            }
        });
    let selected_external_id = resolver_resolution
        .as_ref()
        .and_then(|resolution| resolution.selected.as_ref())
        .filter(|candidate| candidate.source == CandidateSource::External)
        .map(|candidate| candidate.id);

    let mut report = String::new();
    report.push_str("# Reforger Hover Debug\n\n");
    report.push_str(&format!(
        "- Position: line {} character {} (UTF-16, zero-based)\n",
        position.line, position.character
    ));
    report.push_str(&format!(
        "- Byte offset: {}\n",
        format_optional_usize(offset)
    ));
    report.push_str(&format!("- Source bytes: {}\n", source.len()));
    report.push_str("- Pipeline: lexer -> parser -> AST -> model -> index -> display\n");
    report.push_str(&format!(
        "- Parse diagnostics: {}\n",
        analysis.parse_diagnostics
    ));
    report.push_str(&format!("- Indexed symbols: {}\n", index.symbols().len()));
    report.push_str(&format!(
        "- Selected Symbol: {}\n",
        if selected_id.is_some() || selected_external_id.is_some() {
            "yes"
        } else {
            "no"
        }
    ));
    report.push_str(&format!("- Elapsed: {} ms\n", start.elapsed().as_millis()));

    report.push_str("\n## Source Line\n\n");
    append_source_line(&mut report, source, position);

    report.push_str("\n## Tokens Around Cursor\n\n");
    append_token_context(&mut report, source, &tokens, offset);

    report.push_str("\n## Theme / Token Coloring Context\n\n");
    append_theme_token_context(&mut report, source, &tokens, offset);

    report.push_str("\n## Parse Diagnostics\n\n");
    append_parse_diagnostics(&mut report, source, &analysis.diagnostics);

    report.push_str("\n## Resolver Resolution\n\n");
    append_resolver_resolution(
        &mut report,
        &query,
        external_index,
        resolver_resolution.as_ref(),
    );

    report.push_str("\n## External Index\n\n");
    append_external_index_status(&mut report, external_status);

    report.push_str("\n## Hover Selection\n\n");
    if let Some(id) = selected_id {
        append_display_details(&mut report, source, index, &query, id);
        if let Some(display) = query.symbol_display(id) {
            report.push_str("\n### Hover Markdown\n\n");
            report.push_str("```markdown\n");
            report.push_str(&escape_fence_text(&render_hover_markdown(&display)));
            report.push_str("\n```\n");
        }
    } else if let (Some(id), Some(external_index)) = (selected_external_id, external_index) {
        let external_query = IndexQuery::new(external_index);
        append_external_display_details(&mut report, external_index, &external_query, id);
        if let Some(display) = external_query.symbol_display(id) {
            report.push_str("\n### Hover Markdown\n\n");
            report.push_str("```markdown\n");
            report.push_str(&escape_fence_text(&render_hover_markdown(&display)));
            report.push_str("\n```\n");
        }
    } else {
        report.push_str("No symbol matched the cursor position.\n");
    }

    report.push_str("\n## Candidate Symbols At Cursor\n\n");
    append_hover_candidates(&mut report, index, &query, &candidates);

    if let Some(id) = selected_id {
        report.push_str("\n## Parent Chain\n\n");
        append_parent_chain(&mut report, source, index, &query, id);

        report.push_str("\n## Immediate Children\n\n");
        append_children(&mut report, source, index, &query, id);
    } else if let (Some(id), Some(external_index)) = (selected_external_id, external_index) {
        let external_query = IndexQuery::new(external_index);
        report.push_str("\n## Parent Chain\n\n");
        append_external_parent_chain(&mut report, external_index, &external_query, id);

        report.push_str("\n## Immediate Children\n\n");
        append_external_children(&mut report, external_index, &external_query, id);
    }

    report.push_str("\n## Symbol Kind Counts\n\n");
    append_symbol_kind_counts(&mut report, index);

    report
}

struct FileIndexAnalysis {
    index: SymbolIndex,
    parse_diagnostics: usize,
    diagnostics: Vec<ParseDiagnostic>,
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
        diagnostics: parse.diagnostics,
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
        .filter(|symbol| !is_document_symbol_excluded_kind(symbol.kind))
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
    if is_document_symbol_excluded_kind(symbol.kind) {
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
        SymbolKind::LocalVariable => 13,
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
        SymbolKind::LocalVariable => "LocalVariable",
    }
}

fn is_document_symbol_excluded_kind(kind: SymbolKind) -> bool {
    matches!(kind, SymbolKind::Parameter | SymbolKind::LocalVariable)
}

fn append_source_line(report: &mut String, source: &str, position: LspPosition) {
    let Some((start, end)) = line_bounds(source, position.line) else {
        report.push_str("Position is outside the document.\n");
        return;
    };
    let line = &source[start..end];
    report.push_str("```text\n");
    report.push_str(&escape_debug_text(line));
    report.push('\n');
    report.push_str(&" ".repeat(position.character as usize));
    report.push_str("^\n");
    report.push_str("```\n");
}

fn append_token_context(
    report: &mut String,
    source: &str,
    tokens: &[Token],
    offset: Option<usize>,
) {
    if tokens.is_empty() {
        report.push_str("No tokens.\n");
        return;
    }
    let center = offset
        .and_then(|offset| {
            tokens
                .iter()
                .position(|token| span_contains_or_touches_offset(token.span, offset))
        })
        .unwrap_or(0);
    let start = center.saturating_sub(DEBUG_TOKEN_CONTEXT);
    let end = (center + DEBUG_TOKEN_CONTEXT + 1).min(tokens.len());
    report.push_str("| Hit | Kind | Span | Text |\n");
    report.push_str("| --- | --- | --- | --- |\n");
    for token in &tokens[start..end] {
        let hit = offset.is_some_and(|offset| span_contains_or_touches_offset(token.span, offset));
        report.push_str(&format!(
            "| {} | `{:?}` | `{}` | `{}` |\n",
            if hit { "*" } else { "" },
            token.kind,
            format_span(token.span),
            escape_table_text(span_text(source, token.span))
        ));
    }
}

fn append_theme_token_context(
    report: &mut String,
    source: &str,
    tokens: &[Token],
    offset: Option<usize>,
) {
    report.push_str("Expected scopes/colors are derived from the Enforce lexer plus the bundled TextMate grammar/theme palette. VS Code does not expose the active TextMate color at a cursor position through the extension API.\n\n");
    if tokens.is_empty() {
        report.push_str("No tokens.\n");
        return;
    }

    let center = offset
        .and_then(|offset| {
            tokens
                .iter()
                .position(|token| span_contains_or_touches_offset(token.span, offset))
        })
        .unwrap_or(0);
    let start = center.saturating_sub(DEBUG_TOKEN_CONTEXT);
    let end = (center + DEBUG_TOKEN_CONTEXT + 1).min(tokens.len());

    report.push_str("| Hit | Text | Lexer kind | Expected scope | Theme role | Color |\n");
    report.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for index in start..end {
        let token = tokens[index];
        let hit = offset.is_some_and(|offset| span_contains_or_touches_offset(token.span, offset));
        let theme = token_theme_classification(source, tokens, index);
        report.push_str(&format!(
            "| {} | `{}` | `{:?}` | `{}` | `{}` | `{}` |\n",
            if hit { "*" } else { "" },
            escape_table_text(span_text(source, token.span)),
            token.kind,
            theme.scope,
            theme.role,
            theme.color,
        ));
    }
}

fn append_parse_diagnostics(report: &mut String, source: &str, diagnostics: &[ParseDiagnostic]) {
    if diagnostics.is_empty() {
        report.push_str("None.\n");
        return;
    }
    for diagnostic in diagnostics.iter().take(10) {
        report.push_str(&format!(
            "- {} at `{}` {}\n",
            diagnostic.message,
            format_span(diagnostic.span),
            format_range(source, diagnostic.span)
        ));
    }
    if diagnostics.len() > 10 {
        report.push_str(&format!(
            "- ... {} more diagnostics\n",
            diagnostics.len() - 10
        ));
    }
}

fn append_display_details(
    report: &mut String,
    source: &str,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    id: GlobalSymbolId,
) {
    let Some(symbol) = index.symbol(id) else {
        report.push_str("Selected symbol is missing from the index.\n");
        return;
    };
    let Some(display) = query.symbol_display(id) else {
        report.push_str("Selected symbol has no display information.\n");
        return;
    };

    report.push_str(&format!("- Kind: `{}`\n", symbol_kind_label(display.kind)));
    report.push_str(&format!(
        "- Label: `{}`\n",
        escape_debug_text(&display.label)
    ));
    if let Some(signature) = &display.signature {
        report.push_str(&format!(
            "- Signature: `{}`\n",
            escape_debug_text(signature)
        ));
    }
    if let Some(detail) = &display.detail {
        report.push_str(&format!("- Detail: `{}`\n", escape_debug_text(detail)));
    }
    report.push_str(&format!(
        "- Span: `{}` {}\n",
        format_span(symbol.span),
        format_range(source, symbol.span)
    ));
    report.push_str(&format!(
        "- Selection span: `{}` {}\n",
        format_span(symbol.selection_span),
        format_range(source, symbol.selection_span)
    ));
    if !display.modifiers.is_empty() {
        report.push_str(&format!(
            "- Modifiers: `{}`\n",
            display.modifiers.join(", ")
        ));
    }
    if !display.attributes.is_empty() {
        report.push_str(&format!(
            "- Attributes: `{}`\n",
            display
                .attributes
                .iter()
                .map(|attribute| attribute.name.as_deref().unwrap_or(attribute.text.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(preview) = &display.documentation_preview {
        report.push_str(&format!(
            "- Doc preview: `{}`\n",
            escape_debug_text(preview)
        ));
    }
    if !display.doc_comments.is_empty() {
        report.push_str(&format!(
            "- Raw doc comments: `{}`\n",
            display.doc_comments.len()
        ));
    }
    if let Some(form) = display.callable_form {
        report.push_str(&format!("- Callable form: `{}`\n", form.as_str()));
    }
    if !display.conditional_context.is_empty() {
        report.push_str(&format!(
            "- Conditional context: `{}`\n",
            display
                .conditional_context
                .iter()
                .map(|branch| format!(
                    "{} {}",
                    branch.kind.as_str(),
                    branch.condition.as_deref().unwrap_or("")
                ))
                .collect::<Vec<_>>()
                .join(" > ")
        ));
    }
}

fn append_external_display_details(
    report: &mut String,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    id: GlobalSymbolId,
) {
    let Some(symbol) = index.symbol(id) else {
        report.push_str("Selected external symbol is missing from the index.\n");
        return;
    };
    let Some(display) = query.symbol_display(id) else {
        report.push_str("Selected external symbol has no display information.\n");
        return;
    };
    let file = index.file(id.file_id);

    report.push_str("- Source: `external`\n");
    report.push_str(&format!("- Kind: `{}`\n", symbol_kind_label(display.kind)));
    report.push_str(&format!(
        "- Label: `{}`\n",
        escape_debug_text(&display.label)
    ));
    if let Some(signature) = &display.signature {
        report.push_str(&format!(
            "- Signature: `{}`\n",
            escape_debug_text(signature)
        ));
    }
    if let Some(detail) = &display.detail {
        report.push_str(&format!("- Detail: `{}`\n", escape_debug_text(detail)));
    }
    report.push_str(&format!("- Span: `{}`\n", format_span(symbol.span)));
    report.push_str(&format!(
        "- Selection span: `{}`\n",
        format_span(symbol.selection_span)
    ));
    if let Some(file) = file {
        report.push_str(&format!(
            "- Source kind: `{}`\n",
            file.metadata.kind.as_str()
        ));
        report.push_str(&format!(
            "- Source category: `{}`\n",
            file.metadata.category.as_str()
        ));
        report.push_str(&format!("- Priority: `{}`\n", file.metadata.priority));
        if let Some(path) = &file.metadata.relative_path {
            report.push_str(&format!(
                "- Relative path: `{}`\n",
                escape_debug_text(&path.display().to_string())
            ));
        }
        if let Some(path) = &file.metadata.absolute_path {
            report.push_str(&format!(
                "- Absolute path: `{}`\n",
                escape_debug_text(&path.display().to_string())
            ));
        }
    }
    if !display.modifiers.is_empty() {
        report.push_str(&format!(
            "- Modifiers: `{}`\n",
            display.modifiers.join(", ")
        ));
    }
    if !display.attributes.is_empty() {
        report.push_str(&format!(
            "- Attributes: `{}`\n",
            display
                .attributes
                .iter()
                .map(|attribute| attribute.name.as_deref().unwrap_or(attribute.text.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(preview) = &display.documentation_preview {
        report.push_str(&format!(
            "- Doc preview: `{}`\n",
            escape_debug_text(preview)
        ));
    }
    if let Some(form) = display.callable_form {
        report.push_str(&format!("- Callable form: `{}`\n", form.as_str()));
    }
}

fn append_hover_candidates(
    report: &mut String,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    candidates: &[crate::resolver::ReferenceCandidate],
) {
    if candidates.is_empty() {
        report.push_str("None.\n");
        return;
    }
    report.push_str("| Rank | Match | Kind | Label | Span | Detail |\n");
    report.push_str("| ---: | --- | --- | --- | --- | --- |\n");
    for (index_in_list, candidate) in candidates.iter().take(DEBUG_CANDIDATE_LIMIT).enumerate() {
        let display = query.symbol_display(candidate.id);
        let symbol = index.symbol(candidate.id);
        report.push_str(&format!(
            "| {} | {} | `{}` | `{}` | `{}` | `{}` |\n",
            index_in_list + 1,
            candidate.reason.as_str(),
            display
                .as_ref()
                .map(|display| symbol_kind_label(display.kind))
                .unwrap_or("<missing>"),
            display
                .as_ref()
                .map(|display| escape_table_text(&display.label))
                .unwrap_or_else(|| "<missing>".to_string()),
            symbol
                .map(|symbol| format_span(symbol.selection_span))
                .unwrap_or_else(|| "<missing>".to_string()),
            display
                .as_ref()
                .and_then(|display| display.signature.as_ref().or(display.detail.as_ref()))
                .map(|value| escape_table_text(value))
                .unwrap_or_default()
        ));
    }
    if candidates.len() > DEBUG_CANDIDATE_LIMIT {
        report.push_str(&format!(
            "\n{} additional candidates omitted.\n",
            candidates.len() - DEBUG_CANDIDATE_LIMIT
        ));
    }
}

fn append_resolver_resolution(
    report: &mut String,
    query: &IndexQuery<'_>,
    external_index: Option<&SymbolIndex>,
    resolution: Option<&crate::resolver::ReferenceResolution>,
) {
    let Some(resolution) = resolution else {
        report.push_str("Cursor is not on an identifier token; resolver syntax-span hover will be used if a symbol span contains the cursor.\n");
        return;
    };

    report.push_str(&format!(
        "- Token: `{}` at `{}`\n",
        escape_debug_text(&resolution.token_text),
        format_span(resolution.token_span)
    ));
    report.push_str(&format!("- Reason: `{}`\n", resolution.reason.as_str()));
    report.push_str(&format!(
        "- Identifier context: `{}`\n",
        resolution.identifier_context.as_str()
    ));
    if let Some(receiver) = &resolution.receiver {
        report.push_str("\n### Receiver\n\n");
        report.push_str(&format!(
            "- Receiver text: `{}` at `{}`\n",
            escape_debug_text(&receiver.receiver_text),
            format_span(receiver.receiver_span)
        ));
        report.push_str(&format!(
            "- Inferred owner type: `{}`\n",
            receiver
                .owner_type
                .as_deref()
                .map(escape_debug_text)
                .unwrap_or_else(|| "<none>".to_string())
        ));
        report.push_str(&format!(
            "- Static-looking receiver: `{}`\n",
            receiver.is_static
        ));
        if let Some(failure) = &receiver.failure_reason {
            report.push_str(&format!("- Failure: `{}`\n", escape_debug_text(failure)));
        }
        if !receiver.lookup_path.is_empty() {
            report.push_str("- Lookup path:\n");
            for step in &receiver.lookup_path {
                report.push_str(&format!("  - `{}`\n", escape_debug_text(step)));
            }
        }
        report.push('\n');
    }
    report.push_str(&format!("- Candidates: {}\n", resolution.candidates.len()));
    if let Some(selected) = &resolution.selected {
        let selected_label = selected
            .name
            .as_deref()
            .map(escape_debug_text)
            .unwrap_or_else(|| "<unknown>".to_string());
        report.push_str(&format!(
            "- Selected: `{}` `{}` from `{}`\n",
            symbol_kind_label(selected.kind),
            selected_label,
            selected.source.as_str()
        ));
    } else {
        report.push_str("- Selected: none\n");
    }

    if resolution.candidates.is_empty() {
        return;
    }

    report.push_str("\n| Rank | Source | Reason | Kind | Label | Span | Detail |\n");
    report.push_str("| ---: | --- | --- | --- | --- | --- | --- |\n");
    for (index_in_list, candidate) in resolution
        .candidates
        .iter()
        .take(DEBUG_CANDIDATE_LIMIT)
        .enumerate()
    {
        let display = match candidate.source {
            CandidateSource::FileLocal => query.symbol_display(candidate.id),
            CandidateSource::External => external_index
                .map(IndexQuery::new)
                .and_then(|query| query.symbol_display(candidate.id)),
        };
        report.push_str(&format!(
            "| {} | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |\n",
            index_in_list + 1,
            candidate.source.as_str(),
            candidate.reason.as_str(),
            symbol_kind_label(candidate.kind),
            candidate
                .name
                .as_deref()
                .map(escape_table_text)
                .unwrap_or_else(|| "<unknown>".to_string()),
            format_span(candidate.selection_span),
            display
                .as_ref()
                .and_then(|display| display.signature.as_ref().or(display.detail.as_ref()))
                .map(|value| escape_table_text(value))
                .unwrap_or_default()
        ));
    }
    if resolution.candidates.len() > DEBUG_CANDIDATE_LIMIT {
        report.push_str(&format!(
            "\n{} additional resolver candidates omitted.\n",
            resolution.candidates.len() - DEBUG_CANDIDATE_LIMIT
        ));
    }
}

fn append_external_index_status(report: &mut String, status: Option<&ExternalIndexStatusSummary>) {
    let Some(status) = status else {
        report.push_str("No runtime external index status was provided.\n");
        return;
    };

    report.push_str(&format!("- Status: `{}`\n", status.status));
    report.push_str(&format!("- Files: `{}`\n", status.files));
    report.push_str(&format!("- Symbols: `{}`\n", status.symbols));
    report.push_str(&format!(
        "- Parse diagnostics: `{}`\n",
        status.parse_diagnostics
    ));
    if let Some(cache_status) = &status.cache_status {
        report.push_str(&format!(
            "- Cache status: `{}`\n",
            escape_debug_text(cache_status)
        ));
    }
    if let Some(detail) = &status.cache_detail {
        report.push_str(&format!(
            "- Cache detail: `{}`\n",
            escape_debug_text(detail)
        ));
    }
    if let Some(fingerprint) = &status.fingerprint {
        report.push_str(&format!(
            "- Fingerprint: `{}`\n",
            escape_debug_text(fingerprint)
        ));
    }
    if let Some(error) = &status.error {
        report.push_str(&format!("- Error: `{}`\n", escape_debug_text(error)));
    }
}

fn append_parent_chain(
    report: &mut String,
    source: &str,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    id: GlobalSymbolId,
) {
    let mut current = index.symbol(id).and_then(|symbol| symbol.parent);
    if current.is_none() {
        report.push_str("None.\n");
        return;
    }
    while let Some(parent_id) = current {
        append_symbol_bullet(report, source, index, query, parent_id);
        current = index.symbol(parent_id).and_then(|symbol| symbol.parent);
    }
}

fn append_children(
    report: &mut String,
    source: &str,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    id: GlobalSymbolId,
) {
    let children = index.children(id);
    if children.is_empty() {
        report.push_str("None.\n");
        return;
    }
    for child in children.iter().take(DEBUG_CHILD_LIMIT) {
        append_symbol_bullet(report, source, index, query, *child);
    }
    if children.len() > DEBUG_CHILD_LIMIT {
        report.push_str(&format!(
            "- ... {} more children\n",
            children.len() - DEBUG_CHILD_LIMIT
        ));
    }
}

fn append_external_parent_chain(
    report: &mut String,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    id: GlobalSymbolId,
) {
    let mut current = index.symbol(id).and_then(|symbol| symbol.parent);
    if current.is_none() {
        report.push_str("None.\n");
        return;
    }
    while let Some(parent_id) = current {
        append_external_symbol_bullet(report, index, query, parent_id);
        current = index.symbol(parent_id).and_then(|symbol| symbol.parent);
    }
}

fn append_external_children(
    report: &mut String,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    id: GlobalSymbolId,
) {
    let children = index.children(id);
    if children.is_empty() {
        report.push_str("None.\n");
        return;
    }
    for child in children.iter().take(DEBUG_CHILD_LIMIT) {
        append_external_symbol_bullet(report, index, query, *child);
    }
    if children.len() > DEBUG_CHILD_LIMIT {
        report.push_str(&format!(
            "- ... {} more children\n",
            children.len() - DEBUG_CHILD_LIMIT
        ));
    }
}

fn append_external_symbol_bullet(
    report: &mut String,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    id: GlobalSymbolId,
) {
    let Some(symbol) = index.symbol(id) else {
        return;
    };
    let Some(display) = query.symbol_display(id) else {
        return;
    };
    let detail = display
        .signature
        .as_ref()
        .or(display.detail.as_ref())
        .map(|value| format!(" - `{}`", escape_debug_text(value)))
        .unwrap_or_default();
    let path = index
        .file(id.file_id)
        .and_then(|file| file.metadata.relative_path.as_ref())
        .map(|path| format!(" `{}`", escape_debug_text(&path.display().to_string())))
        .unwrap_or_default();
    report.push_str(&format!(
        "- `{}` `{}`{} at `{}`{}\n",
        symbol_kind_label(display.kind),
        escape_debug_text(&display.label),
        detail,
        format_span(symbol.selection_span),
        path
    ));
}

fn append_symbol_bullet(
    report: &mut String,
    source: &str,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
    id: GlobalSymbolId,
) {
    let Some(symbol) = index.symbol(id) else {
        return;
    };
    let Some(display) = query.symbol_display(id) else {
        return;
    };
    let detail = display
        .signature
        .as_ref()
        .or(display.detail.as_ref())
        .map(|value| format!(" - `{}`", escape_debug_text(value)))
        .unwrap_or_default();
    report.push_str(&format!(
        "- `{}` `{}`{} at `{}` {}\n",
        symbol_kind_label(display.kind),
        escape_debug_text(&display.label),
        detail,
        format_span(symbol.selection_span),
        format_range(source, symbol.selection_span)
    ));
}

fn append_symbol_kind_counts(report: &mut String, index: &SymbolIndex) {
    let mut counts = BTreeMap::<SymbolKind, usize>::new();
    for symbol in index.symbols() {
        *counts.entry(symbol.kind).or_default() += 1;
    }
    report.push_str("| Kind | Count |\n");
    report.push_str("| --- | ---: |\n");
    for (kind, count) in counts {
        report.push_str(&format!("| `{}` | {} |\n", symbol_kind_label(kind), count));
    }
}

fn line_bounds(source: &str, target_line: u32) -> Option<(usize, usize)> {
    let mut current_line = 0u32;
    let mut start = 0usize;
    for (index, value) in source.char_indices() {
        if current_line == target_line && value == '\n' {
            return Some((start, index));
        }
        if value == '\n' {
            current_line += 1;
            start = index + value.len_utf8();
        }
    }
    (current_line == target_line).then_some((start, source.len()))
}

fn format_optional_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<invalid>".to_string())
}

fn format_optional_i32(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<none>".to_string())
}

fn format_span(span: TextSpan) -> String {
    format!("{}..{}", span.start, span.end)
}

fn format_range(source: &str, span: TextSpan) -> String {
    let start = position_for_offset(source, span.start);
    let end = position_for_offset(source, span.end);
    format!(
        "L{}:C{}-L{}:C{}",
        start.line, start.character, end.line, end.character
    )
}

fn span_text(source: &str, span: TextSpan) -> &str {
    source.get(span.start..span.end).unwrap_or("")
}

fn span_contains_or_touches_offset(span: TextSpan, offset: usize) -> bool {
    if span.is_empty() {
        return span.start == offset;
    }
    span.start <= offset && offset < span.end
}

#[derive(Debug, Clone, Copy)]
struct ThemeClassification {
    scope: &'static str,
    role: &'static str,
    color: &'static str,
}

fn token_theme_classification(source: &str, tokens: &[Token], index: usize) -> ThemeClassification {
    let token = tokens[index];
    if is_preprocessor_line_token(source, token) {
        return theme("meta.preprocessor.enforce", "preprocessor", "#d4fd95");
    }

    match token.kind {
        TokenKind::LineComment | TokenKind::BlockComment => {
            theme("comment.enforce", "comment", "#59aa59")
        }
        TokenKind::DocLineComment | TokenKind::DocBlockComment => {
            theme("comment.documentation.enforce", "comment", "#59aa59")
        }
        TokenKind::String | TokenKind::UnterminatedString => {
            theme("string.quoted.double.enforce", "string", "#c178dd")
        }
        TokenKind::Number | TokenKind::InvalidNumber => {
            theme("constant.numeric.enforce", "variable/number", "#cfcfcf")
        }
        TokenKind::Hash => theme("meta.preprocessor.enforce", "preprocessor", "#d4fd95"),
        TokenKind::Identifier => identifier_theme_classification(source, tokens, index),
        TokenKind::Keyword(keyword) => keyword_theme_classification(keyword),
        TokenKind::LeftBrace
        | TokenKind::RightBrace
        | TokenKind::LeftParen
        | TokenKind::RightParen
        | TokenKind::LeftBracket
        | TokenKind::RightBracket
        | TokenKind::Semicolon
        | TokenKind::Colon
        | TokenKind::Comma
        | TokenKind::Dot
        | TokenKind::Question
        | TokenKind::Operator(_) => theme("punctuation.enforce", "punctuation", "#bfbfbf"),
        TokenKind::Whitespace | TokenKind::Eof => theme("source.enforce", "plain", "<default>"),
        TokenKind::Unknown | TokenKind::UnterminatedBlockComment => {
            theme("source.enforce", "unknown", "<default>")
        }
    }
}

fn identifier_theme_classification(
    source: &str,
    tokens: &[Token],
    index: usize,
) -> ThemeClassification {
    if previous_non_trivia(tokens, index)
        .is_some_and(|token| token.kind == TokenKind::Keyword(Keyword::Class))
    {
        return theme("entity.name.type.class.enforce", "class/type", "#40b5ac");
    }
    if previous_non_trivia(tokens, index)
        .is_some_and(|token| token.kind == TokenKind::Keyword(Keyword::Enum))
    {
        return theme("entity.name.type.enum.enforce", "enum/type", "#40b5ac");
    }
    if previous_non_trivia(tokens, index)
        .is_some_and(|token| token.kind == TokenKind::Keyword(Keyword::Typedef))
    {
        return theme("support.type.enforce", "type", "#40b5ac");
    }
    if next_non_trivia(tokens, index).is_some_and(|token| token.kind == TokenKind::LeftParen) {
        return theme("entity.name.function.enforce", "function", "#f3ad58");
    }

    let text = span_text(source, tokens[index].span);
    if text.starts_with(|value: char| value.is_ascii_uppercase()) {
        theme("support.type.enforce", "type", "#40b5ac")
    } else {
        theme("variable.other.enforce", "variable", "#cfcfcf")
    }
}

fn keyword_theme_classification(keyword: Keyword) -> ThemeClassification {
    match keyword {
        Keyword::Void
        | Keyword::Int
        | Keyword::Float
        | Keyword::Bool
        | Keyword::String
        | Keyword::Vector
        | Keyword::Typename => theme("support.type.primitive.enforce", "type", "#40b5ac"),
        Keyword::Class => theme("keyword.declaration.class.enforce", "keyword", "#59A6E9"),
        Keyword::Enum => theme("keyword.declaration.enum.enforce", "keyword", "#59A6E9"),
        Keyword::Typedef => theme("keyword.declaration.typedef.enforce", "keyword", "#59A6E9"),
        _ => theme("keyword.enforce", "keyword", "#59A6E9"),
    }
}

fn theme(scope: &'static str, role: &'static str, color: &'static str) -> ThemeClassification {
    ThemeClassification { scope, role, color }
}

fn previous_non_trivia(tokens: &[Token], index: usize) -> Option<Token> {
    tokens[..index]
        .iter()
        .rev()
        .copied()
        .find(|token| !token.kind.is_trivia())
}

fn next_non_trivia(tokens: &[Token], index: usize) -> Option<Token> {
    tokens
        .get(index + 1..)
        .unwrap_or_default()
        .iter()
        .copied()
        .find(|token| !token.kind.is_trivia())
}

fn is_preprocessor_line_token(source: &str, token: Token) -> bool {
    let line_start = source[..token.span.start]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let before_token = &source[line_start..token.span.start];
    let from_token = &source[token.span.start..];
    before_token.trim_start().starts_with('#')
        || before_token.trim().is_empty() && from_token.trim_start().starts_with('#')
}

fn escape_table_text(value: &str) -> String {
    escape_debug_text(value).replace('|', "\\|")
}

fn escape_debug_text(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

fn escape_fence_text(value: &str) -> String {
    value.replace("```", "`\u{200b}``")
}

fn selected_label_from_debug_report(report: &str) -> Option<String> {
    report
        .lines()
        .find_map(|line| line.strip_prefix("- Label: `"))
        .and_then(|line| line.strip_suffix('`'))
        .map(str::to_string)
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
	void Local()
	{
		int localValue = 5;
	}
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
        assert!(!symbols[0]
            .children
            .iter()
            .any(|child| child.name == "localValue"));
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
	void Run(string name)
	{
		int localValue = 5;
		localValue = localValue + 1;
		Print(name);
		m_Value = localValue;
		foreach (int index, auto item : m_aItems)
		{
			string itemName = item.ToString();
		}
		for (int i = 0, count = 4; i < count; i++)
		{
		}
		FactionKey key;
		g_Game = null;
	}
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
            "localValue = 5",
            "localValue",
            SymbolKind::LocalVariable,
            "localValue",
        );
        assert_hover(
            source,
            "localValue + 1",
            "localValue",
            SymbolKind::LocalVariable,
            "localValue",
        );
        assert_hover(source, "Print(name)", "name", SymbolKind::Parameter, "name");
        assert_hover(
            source,
            "m_Value = localValue",
            "m_Value",
            SymbolKind::Field,
            "m_Value",
        );
        assert_hover(
            source,
            "int index, auto item",
            "index",
            SymbolKind::LocalVariable,
            "index",
        );
        assert_hover(
            source,
            "auto item :",
            "item",
            SymbolKind::LocalVariable,
            "item",
        );
        assert_hover(source, "int i = 0", "i =", SymbolKind::LocalVariable, "i");
        assert_hover(
            source,
            "count = 4",
            "count",
            SymbolKind::LocalVariable,
            "count",
        );
        assert_hover(
            source,
            "typedef string FactionKey",
            "FactionKey",
            SymbolKind::Typedef,
            "FactionKey",
        );
        assert_hover(
            source,
            "FactionKey key",
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
            "Game g_Game",
            "g_Game",
            SymbolKind::GlobalField,
            "g_Game",
        );
        assert_hover(
            source,
            "g_Game = null",
            "g_Game",
            SymbolKind::GlobalField,
            "g_Game",
        );
    }

    #[test]
    fn hover_type_position_selects_class_instead_of_constructor() {
        let source = r#"class Example
{
	void Example();
	static Example Make()
	{
		Example value = new Example();
		return value;
	}
}
"#;

        let return_type = hover_at(source, "static Example Make", "Example");
        let local_type = hover_at(source, "Example value", "Example");
        let constructor_call = hover_at(source, "new Example()", "Example");

        assert_eq!(return_type.selected_kind, Some(SymbolKind::Class));
        assert_eq!(return_type.selected_label.as_deref(), Some("Example"));
        assert_eq!(
            return_type.identifier_context,
            Some(IdentifierContext::TypePosition)
        );

        assert_eq!(local_type.selected_kind, Some(SymbolKind::Class));
        assert_eq!(
            local_type.identifier_context,
            Some(IdentifierContext::TypePosition)
        );

        assert_eq!(
            constructor_call.selected_kind,
            Some(SymbolKind::Constructor)
        );
        assert_eq!(
            constructor_call.identifier_context,
            Some(IdentifierContext::ValueOrCallable)
        );
    }

    #[test]
    fn hover_resolves_member_access_through_receiver_type() {
        let source = r#"class Entity
{
	vector GetOrigin();
}

class Example
{
	void Run(Entity ent)
	{
		ent.GetOrigin();
	}
}
"#;

        let report = hover_at(source, "ent.GetOrigin", "GetOrigin");

        assert_eq!(report.selected_kind, Some(SymbolKind::Method));
        assert_eq!(report.selected_label.as_deref(), Some("GetOrigin"));
        assert_eq!(
            report.identifier_context,
            Some(IdentifierContext::MemberAccess)
        );
        assert_eq!(
            report.resolver_reason,
            Some(ResolutionReason::ReceiverMember)
        );
        assert_eq!(report.resolver_candidate_count, 1);
        assert_eq!(
            report
                .receiver_resolution
                .as_ref()
                .and_then(|receiver| receiver.owner_type.as_deref()),
            Some("Entity")
        );
    }

    #[test]
    fn hover_uses_external_index_for_type_position_symbols() {
        let source = r#"class Example
{
	void Run()
	{
		Widget widget;
	}
}
"#;
        let external = file_index_for_source("class Widget {}").index;
        let report = hover_report_for_source_position_with_external(
            source,
            position_for_needle(source, "Widget widget", "Widget"),
            Some(&external),
        );

        assert!(report.is_hit());
        assert_eq!(report.selected_kind, Some(SymbolKind::Class));
        assert_eq!(report.selected_label.as_deref(), Some("Widget"));
        assert_eq!(report.selected_source, Some(CandidateSource::External));
        assert_eq!(
            report.identifier_context,
            Some(IdentifierContext::TypePosition)
        );
    }

    #[test]
    fn file_local_symbols_beat_external_symbols() {
        let source = r#"class Widget {}
class Example
{
	void Run()
	{
		Widget widget;
	}
}
"#;
        let external = file_index_for_source("class Widget {}").index;
        let report = hover_report_for_source_position_with_external(
            source,
            position_for_needle(source, "Widget widget", "Widget"),
            Some(&external),
        );

        assert!(report.is_hit());
        assert_eq!(report.selected_kind, Some(SymbolKind::Class));
        assert_eq!(report.selected_label.as_deref(), Some("Widget"));
        assert_eq!(report.selected_source, Some(CandidateSource::FileLocal));
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
        assert_eq!(report.selection_source, HoverSelectionSource::None);
        assert_eq!(report.resolver_reason, None);
        assert_eq!(report.resolver_candidate_count, 0);
    }

    #[test]
    fn hover_uses_resolver_syntax_span_for_non_identifier_inside_symbol_span() {
        let source = r#"class Example
{
	void Run(int value);
}
"#;

        let report = hover_at(source, "void Run", "void");

        assert!(report.is_hit());
        assert_eq!(
            report.selection_source,
            HoverSelectionSource::ResolverSyntaxSpan
        );
        assert_eq!(report.resolver_reason, Some(ResolutionReason::SyntaxSpan));
        assert!(report.resolver_candidate_count > 0);
        assert_eq!(report.selected_kind, Some(SymbolKind::Method));
        assert_eq!(report.selected_label.as_deref(), Some("Run"));
    }

    #[test]
    fn hover_returns_none_for_unresolved_identifier_without_syntax_span_selection() {
        let source = r#"class Example
{
	void Run()
	{
		MissingThing();
	}
}
"#;

        let report = hover_at(source, "MissingThing();", "MissingThing");

        assert!(!report.is_hit());
        assert_eq!(report.selection_source, HoverSelectionSource::None);
        assert_eq!(report.resolver_reason, Some(ResolutionReason::Unresolved));
        assert_eq!(report.resolver_candidate_count, 0);
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
                game_data_metadata: None,
                index_cache: None,
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
                game_data_metadata: None,
                index_cache: None,
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"hoverProvider\":true"));
        assert!(output_text.contains("Smoke.Run(int value) -> void"));
        assert!(output_text.contains("\"kind\":\"markdown\""));
    }

    #[test]
    fn framed_lsp_uses_cached_analysis_for_repeated_hover() {
        let source = "class Smoke\n{\n\tvoid Run(int value);\n}\n";
        let hover_position = position_for_needle(source, "Run(int", "Run");
        let log_path = test_log_path("cached_hover");
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
        for id in [2, 3] {
            write_test_message(
                &mut input,
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
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
        }
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 4,
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
                log_path: Some(log_path.clone()),
                game_data_scripts: None,
                game_data_metadata: None,
                index_cache: None,
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert_eq!(
            output_text.matches("Smoke.Run(int value) -> void").count(),
            2
        );

        let log = std::fs::read_to_string(&log_path).unwrap();
        assert_eq!(log.matches("notification didOpen").count(), 1);
        assert_eq!(log.matches("analysis_elapsed_ms=").count(), 1);
        assert_eq!(log.matches("request hover").count(), 2);
        assert_eq!(
            log.matches("request hover").count(),
            log.matches("cached_analysis=true").count() - 1
        );

        cleanup_log(&log_path);
    }

    #[test]
    fn framed_lsp_did_change_replaces_cached_analysis() {
        let old_source = "class Old\n{\n\tvoid OldRun();\n}\n";
        let new_source = "class New\n{\n\tvoid NewRun();\n}\n";
        let hover_position = position_for_needle(new_source, "NewRun", "NewRun");
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
                        "uri": "file:///Scripts/Changed.c",
                        "languageId": "enforce",
                        "version": 1,
                        "text": old_source
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Changed.c",
                        "version": 2
                    },
                    "contentChanges": [
                        {
                            "text": new_source
                        }
                    ]
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
                        "uri": "file:///Scripts/Changed.c"
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
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Changed.c"
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 4,
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
                game_data_metadata: None,
                index_cache: None,
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("New.NewRun() -> void"));
        assert!(output_text.contains("\"name\":\"New\""));
        assert!(output_text.contains("\"name\":\"NewRun\""));
        assert!(!output_text.contains("\"name\":\"Old\""));
        assert!(!output_text.contains("\"name\":\"OldRun\""));
    }

    #[test]
    fn framed_lsp_did_close_removes_cached_document() {
        let source = "class Closed {}\n";
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
                        "uri": "file:///Scripts/Closed.c",
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
                "method": "textDocument/didClose",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Closed.c"
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
                        "uri": "file:///Scripts/Closed.c"
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
                game_data_metadata: None,
                index_cache: None,
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"result\":null"));
        assert!(!output_text.contains("\"name\":\"Closed\""));
    }

    #[test]
    fn debug_hover_report_includes_language_engine_context() {
        let source = "class Smoke\n{\n\tvoid Run(int value);\n}\n";
        let hover_position = position_for_needle(source, "Run(int", "Run");

        let report = debug_hover_report_for_source_position(source, hover_position);

        assert!(report.contains("# Reforger Hover Debug"));
        assert!(report.contains("## Resolver Resolution"));
        assert!(report.contains("## Tokens Around Cursor"));
        assert!(report.contains("## Theme / Token Coloring Context"));
        assert!(report.contains("## Candidate Symbols At Cursor"));
        assert!(report.contains("- Selected Symbol: yes"));
        assert!(report.contains("- Label: `Run`"));
        assert!(report.contains("Smoke.Run(int value) -> void"));
        assert!(report.contains("`Method`"));
        assert!(report.contains("entity.name.function.enforce"));
        assert!(report.contains("#f3ad58"));
    }

    #[test]
    fn framed_lsp_smoke_test_handles_debug_hover_request() {
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
                "method": "reforger/debugHover",
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
                game_data_metadata: None,
                index_cache: None,
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("# Reforger Hover Debug"));
        assert!(output_text.contains("Smoke.Run(int value) -> void"));
        assert!(output_text.contains("Candidate Symbols At Cursor"));
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

    fn test_log_path(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "reforger_lsp_{name}_{}_{}.log",
            std::process::id(),
            timestamp_millis()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    fn cleanup_log(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
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
        assert_eq!(
            report.selected_kind,
            Some(expected_kind),
            "hover kind mismatch for needle `{needle}` cursor `{cursor}`"
        );
        assert_eq!(
            report.selected_label.as_deref(),
            Some(expected_label),
            "hover label mismatch for needle `{needle}` cursor `{cursor}`"
        );
        assert!(report.hover.is_some());
        assert_eq!(
            report.selection_source,
            HoverSelectionSource::ResolverIdentifier
        );
        assert!(report.resolver_candidate_count > 0);
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
