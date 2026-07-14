use crate::index::{GlobalSymbolId, SymbolIndex};
use crate::index_query::IndexQuery;
use crate::lexer::TextSpan;
use crate::model::SymbolKind;
#[cfg(test)]
use crate::parser::parse_source;
#[cfg(test)]
use crate::resolver::CandidateSource;
#[cfg(test)]
use crate::resolver::{IdentifierContext, ResolutionReason};
#[cfg(test)]
use crate::syntax::ParseDiagnostic;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

mod completion;
mod debug_hover;
mod definition;
mod diagnostics;
mod external_overlay;
mod hover;
mod open_documents;
mod semantic_tokens;

use completion::empty_completion_list;
pub use completion::{
    completion_report_for_cached_analysis_with_external,
    completion_report_for_source_position_with_external, LspCompletionItem,
    LspCompletionItemLabelDetails, LspCompletionList, LspCompletionReport, LspCompletionTimings,
    LspTextEdit,
};
use debug_hover::debug_hover_report_for_cached_analysis_with_external;
pub use debug_hover::debug_hover_report_for_source_position;
pub(crate) use debug_hover::selected_label_from_debug_report;
#[cfg(test)]
pub(crate) use definition::file_uri_for_path;
pub use definition::{
    definition_report_for_cached_analysis_with_external, definition_report_for_source_position,
    definition_report_for_source_position_with_external, LspDefinitionReport, LspLocation,
    LspLocationLink,
};
use diagnostics::{clear_diagnostics_message, publish_diagnostics_message};
pub use diagnostics::{parser_diagnostics_for_source, LspDiagnostic};
pub(crate) use external_overlay::ExternalIndexStatusSummary;
use external_overlay::{start_external_index, ExternalIndexHandle};
use hover::hover_report_for_cached_analysis_with_external;
pub use hover::{
    hover_report_for_source_position, hover_report_for_source_position_with_external,
    hover_reports_for_source_positions, hover_reports_for_source_positions_with_external,
    HoverSelectionSource, LspHover, LspHoverReport,
};
pub(crate) use open_documents::OpenDocument;
pub use open_documents::{file_index_for_source, FileIndexAnalysis};
use semantic_tokens::{
    fast_semantic_tokens_for_cached_analysis, semantic_tokens_for_cached_analysis_with_external,
    LspSemanticTokenProjection, LspSemanticTokens, SEMANTIC_TOKEN_MODIFIERS, SEMANTIC_TOKEN_TYPES,
};
pub use semantic_tokens::{
    fast_semantic_tokens_for_source, semantic_tokens_for_source_with_external,
    semantic_tokens_report_for_source, semantic_tokens_report_for_source_with_external,
    LspSemanticTokenReport, LspSemanticTokenTimings, SemanticTokenDebug,
};

const SERVER_NAME: &str = "reforger-language-server";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEBUG_HOVER_METHOD: &str = "reforger/debugHover";
const WORKSPACE_FILE_CHANGED_METHOD: &str = "reforger/workspaceFileChanged";
const WORKSPACE_FILE_DELETED_METHOD: &str = "reforger/workspaceFileDeleted";
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LspServerOptions {
    pub log_path: Option<PathBuf>,
    pub game_data_scripts: Option<PathBuf>,
    pub game_data_metadata: Option<PathBuf>,
    pub index_cache: Option<PathBuf>,
    pub workspace_scripts: Vec<PathBuf>,
}

pub fn run_stdio(options: LspServerOptions) -> Result<(), String> {
    let stdout = io::stdout();
    let mut server = LspServer::new(stdout.lock(), options);
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        loop {
            match read_message(&mut reader) {
                Ok(Some(message)) => {
                    if sender.send(Ok(message)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = sender.send(Err(error));
                    break;
                }
            }
        }
    });
    server.run_message_channel(receiver)
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
pub struct LspMarkupContent {
    pub kind: String,
    pub value: String,
}

impl LspDocumentSymbolReport {
    pub fn total_symbol_count(&self) -> usize {
        document_symbol_count(&self.symbols)
    }
}

struct LspServer<W: Write> {
    writer: W,
    documents: BTreeMap<String, OpenDocument>,
    logger: LspLogger,
    external_index: ExternalIndexHandle,
    next_server_request_id: u64,
    last_semantic_external_generation: u64,
    shutdown_requested: bool,
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

fn format_paths(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return "<none>".to_string();
    }
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(";")
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
struct WorkspaceFileChangedParams {
    path: String,
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceFileDeletedParams {
    path: String,
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
            next_server_request_id: 1,
            last_semantic_external_generation: 0,
            shutdown_requested: false,
        };
        server.log(&format!(
            "startup server={SERVER_NAME} version={SERVER_VERSION} game_data_scripts={} index_cache={} workspace_roots={} external_index_status={}",
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
            format_paths(&options.workspace_scripts),
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

    fn run_message_channel(
        &mut self,
        receiver: mpsc::Receiver<Result<Value, String>>,
    ) -> Result<(), String> {
        loop {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(message)) => {
                    let should_exit = self.handle_message(message)?;
                    if should_exit {
                        break;
                    }
                }
                Ok(Err(error)) => return Err(error),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    self.request_semantic_tokens_refresh_if_external_generation_changed()?;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
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
                                "hoverProvider": true,
                                "definitionProvider": true,
                                "completionProvider": {
                                    "triggerCharacters": ["."]
                                },
                                "semanticTokensProvider": {
                                    "legend": {
                                        "tokenTypes": SEMANTIC_TOKEN_TYPES,
                                        "tokenModifiers": SEMANTIC_TOKEN_MODIFIERS
                                    },
                                    "full": true,
                                    "range": false
                                }
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
                    let mut document = OpenDocument::new(text, version, 1);
                    let symbols =
                        document_symbols_from_cached_analysis(&document.text, &document.analysis);
                    let symbol_count = document_symbol_count(&symbols);
                    document.set_document_symbols(symbols);
                    let parse_diagnostics = document.analysis.parse_diagnostics;
                    let revision = document.revision;
                    let diagnostics_message = publish_diagnostics_message(
                        &uri,
                        &document.text,
                        &document.analysis.diagnostics,
                    );
                    self.documents.insert(uri.clone(), document);
                    self.write_message(diagnostics_message)?;
                    self.log(&format!(
                        "notification didOpen uri={} bytes={} version={} revision={} cached_analysis=true document_symbols_cached=true symbols={} parse_diagnostics={} analysis_elapsed_ms={}",
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
                        document.set_document_symbols(symbols);
                        let parse_diagnostics = document.analysis.parse_diagnostics;
                        let revision = document.revision;
                        let diagnostics_message = publish_diagnostics_message(
                            &uri,
                            &document.text,
                            &document.analysis.diagnostics,
                        );
                        self.write_message(diagnostics_message)?;
                        self.log(&format!(
                            "notification didChange uri={} bytes={} version={} revision={} cached_analysis=true document_symbols_cached=true symbols={} parse_diagnostics={} analysis_elapsed_ms={}",
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
                    self.write_message(clear_diagnostics_message(&params.text_document.uri))?;
                }
            }
            WORKSPACE_FILE_CHANGED_METHOD => {
                if let Some(params) =
                    parse_params::<WorkspaceFileChangedParams>(message.params, method)?
                {
                    let start = Instant::now();
                    let path = PathBuf::from(params.path);
                    let bytes = params.text.len();
                    let result = self
                        .external_index
                        .update_workspace_file(path.clone(), params.text);
                    match result {
                        Ok((symbols, parse_diagnostics)) => {
                            let status = self.external_index.status_summary();
                            self.log(&format!(
                                "notification workspaceFileChanged path={} bytes={} symbols={} parse_diagnostics={} overlay_status={} overlay_generation={} overlay_files={} overlay_symbols={} elapsed_ms={}",
                                path.display(),
                                bytes,
                                symbols,
                                parse_diagnostics,
                                status.status,
                                status.generation,
                                status.files,
                                status.symbols,
                                start.elapsed().as_millis()
                            ));
                            self.request_semantic_tokens_refresh()?;
                        }
                        Err(error) => {
                            self.log(&format!(
                                "notification workspaceFileChanged path={} bytes={} error={} elapsed_ms={}",
                                path.display(),
                                bytes,
                                error,
                                start.elapsed().as_millis()
                            ));
                        }
                    }
                }
            }
            WORKSPACE_FILE_DELETED_METHOD => {
                if let Some(params) =
                    parse_params::<WorkspaceFileDeletedParams>(message.params, method)?
                {
                    let start = Instant::now();
                    let path = PathBuf::from(params.path);
                    let removed = self.external_index.delete_workspace_file(&path);
                    let status = self.external_index.status_summary();
                    self.log(&format!(
                        "notification workspaceFileDeleted path={} removed={} overlay_status={} overlay_generation={} overlay_files={} overlay_symbols={} elapsed_ms={}",
                        path.display(),
                        removed,
                        status.status,
                        status.generation,
                        status.files,
                        status.symbols,
                        start.elapsed().as_millis()
                    ));
                    if removed {
                        self.request_semantic_tokens_refresh()?;
                    }
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
                                let symbols = document.document_symbols();
                                symbol_count = document_symbol_count(&symbols);
                                parse_diagnostics = document.analysis.parse_diagnostics;
                                symbols.to_vec()
                            })
                        })
                        .map(|symbols| serde_json::to_value(symbols).unwrap_or(Value::Null))
                        .unwrap_or(Value::Null);
                    self.log(&format!(
                        "request documentSymbol uri={} bytes={} revision={} cached_analysis=true document_symbols_cached=true symbols={} parse_diagnostics={} elapsed_ms={}",
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
            "textDocument/completion" => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<HoverParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut parse_diagnostics = 0usize;
                    let mut revision = 0u64;
                    let mut receiver = "<none>".to_string();
                    let mut owner_type = "<none>".to_string();
                    let mut completion_context = "none".to_string();
                    let mut prefix = String::new();
                    let mut candidate_count = 0usize;
                    let mut failure_reason = "<none>".to_string();
                    let mut context_ms = 0u128;
                    let mut lookup_ms = 0u128;
                    let mut render_ms = 0u128;
                    let mut external_index_status = self.external_index.status_summary().status;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let report = self.external_index.with_index(|status, index| {
                                    external_index_status = status;
                                    completion_report_for_cached_analysis_with_external(
                                        &document.text,
                                        &document.analysis,
                                        params.position,
                                        index,
                                    )
                                });
                                parse_diagnostics = report.parse_diagnostics;
                                completion_context = report.completion_context.clone();
                                receiver = report
                                    .receiver_text
                                    .clone()
                                    .unwrap_or_else(|| "<none>".to_string());
                                owner_type = report
                                    .owner_type
                                    .clone()
                                    .unwrap_or_else(|| "<none>".to_string());
                                prefix = report.prefix.clone();
                                candidate_count = report.candidate_count;
                                failure_reason = report
                                    .failure_reason
                                    .clone()
                                    .unwrap_or_else(|| "<none>".to_string());
                                context_ms = report.timings.context_detection.as_millis();
                                lookup_ms = report.timings.candidate_lookup.as_millis();
                                render_ms = report.timings.item_rendering.as_millis();
                                report.list
                            })
                        })
                        .map(|list| serde_json::to_value(list).unwrap_or(Value::Null))
                        .unwrap_or_else(|| {
                            serde_json::to_value(empty_completion_list()).unwrap_or(Value::Null)
                        });
                    self.log(&format!(
                        "request completion uri={} bytes={} revision={} cached_analysis=true context={} receiver={} owner_type={} prefix={} candidates={} failure_reason={} external_index_status={} parse_diagnostics={} context_ms={} lookup_ms={} render_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        completion_context,
                        receiver,
                        owner_type,
                        prefix,
                        candidate_count,
                        failure_reason,
                        external_index_status,
                        parse_diagnostics,
                        context_ms,
                        lookup_ms,
                        render_ms,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                }
            }
            "textDocument/semanticTokens/full" => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<DocumentSymbolParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut revision = 0u64;
                    let mut token_count = 0usize;
                    let mut parse_diagnostics = 0usize;
                    let external_index_summary = self.external_index.status_summary();
                    let external_index_status = external_index_summary.status;
                    let external_generation = external_index_summary.generation;
                    let mut projection_mode = "missing-document";
                    let mut lex_ms = 0u128;
                    let mut resolver_ms = 0u128;
                    let mut resolver_calls = 0usize;
                    let mut token_loop_ms = 0u128;
                    let mut encode_ms = 0u128;
                    let mut rich_work: Option<(String, u64, u64)> = None;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let projection = if let Some(projection) = document
                                    .semantic_tokens
                                    .rich_for_revision_and_external_generation(
                                        document.revision,
                                        external_generation,
                                    ) {
                                    projection_mode = "rich-cache";
                                    projection.clone()
                                } else {
                                    projection_mode = "fast-compute";
                                    rich_work = Some((
                                        log_uri.clone(),
                                        document.revision,
                                        external_generation,
                                    ));
                                    fast_semantic_tokens_for_cached_analysis(
                                        &document.text,
                                        &document.analysis,
                                    )
                                };
                                token_count = projection.token_count;
                                parse_diagnostics = projection.parse_diagnostics;
                                lex_ms = projection.timings.lex_ms;
                                resolver_ms = projection.timings.resolver_ms;
                                resolver_calls = projection.timings.identifier_resolver_calls;
                                token_loop_ms = projection.timings.token_loop_ms;
                                encode_ms = projection.timings.encode_ms;
                                projection.tokens
                            })
                        })
                        .map(|tokens| serde_json::to_value(tokens).unwrap_or(Value::Null))
                        .unwrap_or_else(|| {
                            serde_json::to_value(LspSemanticTokens { data: Vec::new() })
                                .unwrap_or(Value::Null)
                        });
                    self.log(&format!(
                        "request semanticTokens uri={} bytes={} revision={} cached_analysis=true mode={} tokens={} external_index_status={} external_generation={} parse_diagnostics={} lex_ms={} token_loop_ms={} resolver_ms={} resolver_calls={} encode_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        projection_mode,
                        token_count,
                        external_index_status,
                        external_generation,
                        parse_diagnostics,
                        lex_ms,
                        token_loop_ms,
                        resolver_ms,
                        resolver_calls,
                        encode_ms,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                    if let Some((uri, rich_revision, rich_external_generation)) = rich_work {
                        self.prepare_rich_semantic_tokens(
                            &uri,
                            rich_revision,
                            rich_external_generation,
                        )?;
                    }
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
                                let report = self.external_index.with_index(|status, index| {
                                    external_index_status = status;
                                    hover_report_for_cached_analysis_with_external(
                                        &document.text,
                                        &document.analysis,
                                        params.position,
                                        index,
                                    )
                                });
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
            "textDocument/definition" => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<HoverParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut parse_diagnostics = 0usize;
                    let mut selected_label = "<none>".to_string();
                    let mut selected_kind = "None";
                    let mut selected_source = "<none>";
                    let mut resolver_reason = "<none>";
                    let mut identifier_context = "<none>";
                    let mut resolver_candidate_count = 0usize;
                    let mut external_index_status = self.external_index.status_summary().status;
                    let mut revision = 0u64;
                    let mut hit = false;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let report = self.external_index.with_index(|status, index| {
                                    external_index_status = status;
                                    definition_report_for_cached_analysis_with_external(
                                        &document.text,
                                        &document.analysis,
                                        &log_uri,
                                        params.position,
                                        index,
                                    )
                                });
                                parse_diagnostics = report.parse_diagnostics;
                                hit = report.is_hit();
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
                                if let Some(label) = report.selected_label {
                                    selected_label = label;
                                }
                                if let Some(kind) = report.selected_kind {
                                    selected_kind = symbol_kind_label(kind);
                                }
                                report.links
                            })
                        })
                        .map(|links| serde_json::to_value(links).unwrap_or(Value::Null))
                        .unwrap_or(Value::Null);
                    self.log(&format!(
                        "request definition uri={} bytes={} revision={} cached_analysis=true hit={} selected_source={} resolver_reason={} identifier_context={} resolver_candidates={} external_index_status={} label={} kind={} parse_diagnostics={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        hit,
                        selected_source,
                        resolver_reason,
                        identifier_context,
                        resolver_candidate_count,
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
                                let report = self.external_index.with_index(|_, index| {
                                    debug_hover_report_for_cached_analysis_with_external(
                                        &document.text,
                                        &document.analysis,
                                        params.position,
                                        index,
                                        Some(&external_status),
                                    )
                                });
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

        self.request_semantic_tokens_refresh_if_external_generation_changed()?;

        Ok(self.shutdown_requested && method == "exit")
    }

    fn prepare_rich_semantic_tokens(
        &mut self,
        uri: &str,
        revision: u64,
        external_generation: u64,
    ) -> Result<(), String> {
        let start = Instant::now();
        let mut external_index_status = self.external_index.status_summary().status;
        let Some(projection) =
            self.rich_semantic_tokens_for_revision(uri, revision, &mut external_index_status)
        else {
            self.log(&format!(
                "semanticTokensRich skipped uri={} revision={} reason=stale-or-missing-document elapsed_ms={}",
                uri,
                revision,
                start.elapsed().as_millis()
            ));
            return Ok(());
        };
        let token_count = projection.token_count;
        let parse_diagnostics = projection.parse_diagnostics;
        let lex_ms = projection.timings.lex_ms;
        let token_loop_ms = projection.timings.token_loop_ms;
        let resolver_ms = projection.timings.resolver_ms;
        let resolver_calls = projection.timings.identifier_resolver_calls;
        let encode_ms = projection.timings.encode_ms;
        let Some(current_revision) = self.documents.get(uri).map(|document| document.revision)
        else {
            self.log(&format!(
                "semanticTokensRich discarded uri={} revision={} reason=missing-document elapsed_ms={}",
                uri,
                revision,
                start.elapsed().as_millis()
            ));
            return Ok(());
        };
        if current_revision != revision {
            self.log(&format!(
                "semanticTokensRich discarded uri={} revision={} current_revision={} reason=stale-revision elapsed_ms={}",
                uri,
                revision,
                current_revision,
                start.elapsed().as_millis()
            ));
            return Ok(());
        }
        let current_external_generation = self.external_index.status_summary().generation;
        if current_external_generation != external_generation {
            self.log(&format!(
                "semanticTokensRich discarded uri={} revision={} external_generation={} current_external_generation={} reason=stale-external-index elapsed_ms={}",
                uri,
                revision,
                external_generation,
                current_external_generation,
                start.elapsed().as_millis()
            ));
            return Ok(());
        }
        if let Some(document) = self.documents.get_mut(uri) {
            document
                .semantic_tokens
                .set_rich(revision, external_generation, projection);
        }
        self.log(&format!(
            "semanticTokensRich ready uri={} revision={} external_generation={} tokens={} external_index_status={} parse_diagnostics={} lex_ms={} token_loop_ms={} resolver_ms={} resolver_calls={} encode_ms={} elapsed_ms={}",
            uri,
            revision,
            external_generation,
            token_count,
            external_index_status,
            parse_diagnostics,
            lex_ms,
            token_loop_ms,
            resolver_ms,
            resolver_calls,
            encode_ms,
            start.elapsed().as_millis()
        ));
        self.request_semantic_tokens_refresh()
    }

    fn rich_semantic_tokens_for_revision(
        &self,
        uri: &str,
        revision: u64,
        external_index_status: &mut &'static str,
    ) -> Option<LspSemanticTokenProjection> {
        let document = self.documents.get(uri)?;
        if document.revision != revision {
            return None;
        }
        Some(self.external_index.with_index(|status, index| {
            *external_index_status = status;
            semantic_tokens_for_cached_analysis_with_external(
                &document.text,
                &document.analysis,
                index,
            )
        }))
    }

    fn request_semantic_tokens_refresh(&mut self) -> Result<(), String> {
        let request_id = self.next_server_request_id;
        self.next_server_request_id += 1;
        self.log(&format!(
            "request workspace/semanticTokens/refresh id=server-{request_id}"
        ));
        self.write_message(json!({
            "jsonrpc": "2.0",
            "id": format!("server-{request_id}"),
            "method": "workspace/semanticTokens/refresh",
            "params": null
        }))
    }

    fn request_semantic_tokens_refresh_if_external_generation_changed(
        &mut self,
    ) -> Result<(), String> {
        if self.documents.is_empty() {
            self.last_semantic_external_generation =
                self.external_index.status_summary().generation;
            return Ok(());
        }
        let status = self.external_index.status_summary();
        if status.generation == self.last_semantic_external_generation {
            return Ok(());
        }
        self.last_semantic_external_generation = status.generation;
        self.log(&format!(
            "semanticTokens external overlay changed generation={} status={} documents={} requesting_refresh=true",
            status.generation,
            status.status,
            self.documents.len()
        ));
        self.request_semantic_tokens_refresh()
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

pub(crate) fn range_for_span(source: &str, span: crate::lexer::TextSpan) -> LspRange {
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

fn document_symbol_kind(kind: SymbolKind) -> u32 {
    match kind {
        SymbolKind::Class => 5,
        SymbolKind::TypeParameter => 26,
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
        SymbolKind::PreprocessorMacro => 13,
    }
}

pub fn symbol_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "Class",
        SymbolKind::TypeParameter => "TypeParameter",
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
        SymbolKind::PreprocessorMacro => "PreprocessorMacro",
    }
}

fn is_document_symbol_excluded_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::TypeParameter
            | SymbolKind::Parameter
            | SymbolKind::LocalVariable
            | SymbolKind::PreprocessorMacro
    )
}

fn format_optional_i32(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<none>".to_string())
}

pub(crate) fn span_text(source: &str, span: TextSpan) -> &str {
    source.get(span.start..span.end).unwrap_or("")
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
    fn semantic_tokens_classify_lexer_and_symbol_facts() {
        let source = r#"// docs
[Attribute()]
class Base
{
}

class Example
	: Base
{
	static const int COUNT = 4;
	void Run(int value)
	{
		string name = "x";
		Example other;
		other.Run(value);
	}
}
#ifdef DEBUG
#define GAME_MODE_DEBUG
#endif
"#;

        let report = semantic_tokens_report_for_source(source);

        assert_eq!(report.parse_diagnostics, 0);
        assert!(!report.tokens.data.is_empty());
        assert_eq!(report.tokens.data.len() % 5, 0);
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "Example" && token.token_type == "class"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "Run" && token.token_type == "method"));
        assert!(
            report
                .decoded
                .iter()
                .filter(|token| token.text == "Run" && token.token_type == "method")
                .count()
                >= 2
        );
        assert!(
            report
                .decoded
                .iter()
                .filter(|token| token.text == "Example" && token.token_type == "class")
                .count()
                >= 2
        );
        assert!(
            report
                .decoded
                .iter()
                .filter(|token| token.text == "Base" && token.token_type == "class")
                .count()
                >= 2,
            "{:?}",
            report.decoded
        );
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "COUNT" && token.token_type == "property"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "value" && token.token_type == "parameter"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "name" && token.token_type == "variable"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "Attribute" && token.token_type == "class"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "void" && token.token_type == "keyword"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "int" && token.token_type == "keyword"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "string" && token.token_type == "class"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "\"x\"" && token.token_type == "string"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "4" && token.token_type == "number"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "// docs" && token.token_type == "comment"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "ifdef" && token.token_type == "preprocessor"));
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "define" && token.token_type == "preprocessor"));
        assert_semantic_token(&report, "DEBUG", "variable", Some("#cfcfcf"));
        assert_semantic_token(&report, "GAME_MODE_DEBUG", "variable", Some("#cfcfcf"));
    }

    #[test]
    fn semantic_tokens_color_external_enum_member_references() {
        let root = temp_test_dir("semantic_tokens_external_enum");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("EWeaponType.c"),
            "enum EWeaponType\n{\n\tWT_NONE,\n\tWT_FRAGGRENADE,\n}\n",
        )
        .unwrap();
        let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
            roots: vec![crate::index_build::IndexSourceRoot::new(
                &root,
                crate::model::SourceKind::GameData,
                crate::model::SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap()
        .index;
        let source = r#"class Example
{
	void Run()
	{
		EWeaponType value = EWeaponType.WT_FRAGGRENADE;
	}
}
"#;

        let report = semantic_tokens_report_for_source_with_external(source, Some(&external));

        assert!(
            report
                .decoded
                .iter()
                .any(|token| token.text == "WT_FRAGGRENADE" && token.token_type == "enumMember"),
            "{:?}",
            report.decoded
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn semantic_tokens_color_primitive_keywords_and_external_class_types_separately() {
        let root = temp_test_dir("semantic_tokens_external_class_type");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("KickCauseCode.c"),
            "class KickCauseCode : handle64\n{\n\tstatic KickCauseCode NONE;\n}\n",
        )
        .unwrap();
        fs::write(
            root.join("SCR_InstigatorContextData.c"),
            "class SCR_InstigatorContextData {}\n",
        )
        .unwrap();
        fs::write(root.join("IEntity.c"), "class IEntity {}\n").unwrap();
        fs::write(root.join("array.c"), "class array {}\n").unwrap();
        fs::write(
            root.join("EResourceType.c"),
            "enum EResourceType\n{\n\tSUPPLIES,\n}\n",
        )
        .unwrap();
        fs::write(
            root.join("ScriptInvokerBase.c"),
            "class ScriptInvokerBase {}\n",
        )
        .unwrap();
        let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
            roots: vec![crate::index_build::IndexSourceRoot::new(
                &root,
                crate::model::SourceKind::GameData,
                crate::model::SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap()
        .index;
        let source = "\
void SCR_BaseGameMode_OnPlayerDisconnected(int playerId, KickCauseCode cause = KickCauseCode.NONE, int timeout = -1);
void SCR_BaseGameMode_OnControllableDestroyed(notnull SCR_InstigatorContextData instigatorContextData);
void SCR_BaseGameMode_PlayerIdAndEntity(int playerId, IEntity player);
void SCR_BaseGameMode_OnResourceEnabledChanged(array<EResourceType> disabledResourceTypes);
typedef ScriptInvokerBase<OnPreloadFinished> OnPreloadFinishedInvoker;
class Example { protected ref ScriptInvoker m_OnGameEnd = new ScriptInvoker(); }
";

        let report = semantic_tokens_report_for_source_with_external(source, Some(&external));

        assert!(
            report
                .decoded
                .iter()
                .any(|token| token.text == "void" && token.token_type == "keyword"),
            "{:?}",
            report.decoded
        );
        assert!(
            report
                .decoded
                .iter()
                .filter(|token| token.text == "int" && token.token_type == "keyword")
                .count()
                >= 2,
            "{:?}",
            report.decoded
        );
        assert!(
            report
                .decoded
                .iter()
                .filter(|token| token.text == "KickCauseCode" && token.token_type == "class")
                .count()
                >= 2,
            "{:?}",
            report.decoded
        );
        assert_semantic_token(&report, "SCR_InstigatorContextData", "class", None);
        assert_semantic_token(&report, "IEntity", "class", None);
        assert_semantic_token(&report, "array", "class", None);
        assert_semantic_token(&report, "EResourceType", "enum", None);
        assert_semantic_token(&report, "ScriptInvokerBase", "class", None);
        assert!(
            report
                .decoded
                .iter()
                .any(|token| token.text == "NONE" && token.token_type == "enumMember"),
            "{:?}",
            report.decoded
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn semantic_tokens_color_source_backed_type_spans_before_external_index_is_ready() {
        let source = "\
void SCR_BaseGameMode_OnPlayerDisconnected(int playerId, KickCauseCode cause = KickCauseCode.NONE, int timeout = -1);
void SCR_BaseGameMode_OnControllableDestroyed(notnull SCR_InstigatorContextData instigatorContextData);
void SCR_BaseGameMode_PlayerIdAndEntity(int playerId, IEntity player);
void SCR_BaseGameMode_OnResourceEnabledChanged(array<EResourceType> disabledResourceTypes);
typedef ScriptInvokerBase<OnPreloadFinished> OnPreloadFinishedInvoker;
class Example { protected ref ScriptInvoker m_OnGameEnd = new ScriptInvoker(); }
";

        let report = semantic_tokens_report_for_source(source);

        assert_semantic_token(&report, "void", "keyword", Some("#59A6E9"));
        assert_semantic_token(&report, "int", "keyword", Some("#59A6E9"));
        assert_semantic_token(&report, "KickCauseCode", "class", Some("#40b5ac"));
        assert_semantic_token(
            &report,
            "SCR_InstigatorContextData",
            "class",
            Some("#40b5ac"),
        );
        assert_semantic_token(&report, "IEntity", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "array", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "EResourceType", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "ScriptInvokerBase", "class", Some("#40b5ac"));
        assert_semantic_token_count_at_least(&report, "ScriptInvoker", "class", 2);
    }

    #[test]
    fn semantic_tokens_apply_type_keyword_color_policy_in_declarations() {
        let source = r#"class SCR_Class {}
enum SCR_EEnum { VALUE, }
class ResourceName {}
class LocalizedString {}
class Curve {}
class Color {}
class array<Class T> {}
class map<Class TKey, Class TValue> {}
class set<Class T> {}

class Example
{
	bool m_bValue;
	int m_iValue;
	float m_fValue;
	string m_sValue;
	SCR_EEnum m_eValue;
	vector m_vValue;
	array<SCR_Class> m_aValue;
	map<string, SCR_Class> m_mValue;
	ResourceName m_sResourceName;
	LocalizedString m_sLocalisedString;
	Curve m_aCurve;
	SCR_Class m_ClassInstance;
	typename m_ClassTypename;
	set<SCR_Class> m_Set;
	Color m_Color;
	void Run()
	{
		bool b = true;
		bool c = false;
	}
}
"#;

        let report = semantic_tokens_report_for_source(source);

        for text in ["bool", "int", "float", "typename", "true", "false"] {
            assert_semantic_token(&report, text, "keyword", Some("#59A6E9"));
        }
        for text in [
            "string",
            "SCR_EEnum",
            "vector",
            "array",
            "map",
            "ResourceName",
            "LocalizedString",
            "Curve",
            "SCR_Class",
            "set",
            "Color",
        ] {
            assert_semantic_type_family_token_count_at_least(&report, text, 1);
        }
    }

    #[test]
    fn semantic_tokens_color_enum_static_member_values_as_variables() {
        let source = r#"enum EHealthState
{
	INJURED,
}

class Example
{
	void Run()
	{
		EHealthState state = EHealthState.INJURED;
	}
}
"#;

        let report = semantic_tokens_report_for_source(source);

        assert_semantic_token(&report, "EHealthState", "enum", Some("#40b5ac"));
        assert_semantic_token(&report, "INJURED", "enumMember", Some("#cfcfcf"));
    }

    #[test]
    fn semantic_tokens_keep_generic_callback_type_arguments_type_colored() {
        let source = "\
void SCR_BaseGameMode_PlayerId(int playerId);
typedef func SCR_BaseGameMode_PlayerId;
class Example
{
\tprotected ref ScriptInvokerBase<SCR_BaseGameMode_PlayerId> m_OnPlayerAuditSuccess = new ScriptInvokerBase<SCR_BaseGameMode_PlayerId>();
}
";

        let report = semantic_tokens_report_for_source(source);

        assert_semantic_type_family_token_count_at_least(&report, "SCR_BaseGameMode_PlayerId", 2);
        assert_semantic_token_count_at_least(&report, "ScriptInvokerBase", "class", 2);
        assert_eq!(
            report
                .decoded
                .iter()
                .filter(|token| {
                    token.text == "SCR_BaseGameMode_PlayerId" && token.token_type == "function"
                })
                .count(),
            1,
            "{:?}",
            report.decoded
        );
    }

    #[test]
    fn semantic_tokens_resolve_attribute_argument_expressions() {
        let root = temp_test_dir("semantic_tokens_attribute_arguments");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("Attribute.c"),
            r#"class Attribute {}
class UIWidgets
{
	static const string Flags = "flags";
}
class ParamEnumArray
{
	static ParamEnumArray FromEnum(typename value);
}
enum EGameFlags
{
	TEST,
}
"#,
        )
        .unwrap();
        let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
            roots: vec![crate::index_build::IndexSourceRoot::new(
                &root,
                crate::model::SourceKind::GameData,
                crate::model::SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap()
        .index;
        let source = r#"class Example
{
	static const string WB_GAME_MODE_CATEGORY = "Game";
	[Attribute("0", uiwidget: UIWidgets.Flags, "Test Game Flags for when you run mission via WE.", "", ParamEnumArray.FromEnum(EGameFlags), WB_GAME_MODE_CATEGORY)]
	protected EGameFlags m_eTestGameFlags;
}
"#;

        let report = semantic_tokens_report_for_source_with_external(source, Some(&external));

        assert_semantic_token(&report, "Attribute", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "uiwidget", "variable", Some("#cfcfcf"));
        assert_semantic_token(&report, "UIWidgets", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "Flags", "enumMember", Some("#cfcfcf"));
        assert_semantic_token(&report, "ParamEnumArray", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "FromEnum", "method", Some("#f3ad58"));
        assert_semantic_token(&report, "EGameFlags", "enum", Some("#40b5ac"));
        assert_semantic_token(
            &report,
            "WB_GAME_MODE_CATEGORY",
            "property",
            Some("#cfcfcf"),
        );
        assert!(
            !report.decoded.iter().any(|token| {
                matches!(
                    token.text.as_str(),
                    "Attribute"
                        | "uiwidget"
                        | "UIWidgets"
                        | "Flags"
                        | "ParamEnumArray"
                        | "FromEnum"
                        | "EGameFlags"
                        | "WB_GAME_MODE_CATEGORY"
                ) && token.token_type == "decorator"
            }),
            "{:?}",
            report.decoded
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn semantic_tokens_refine_unqualified_attribute_arguments_with_external_facts() {
        let root = temp_test_dir("semantic_tokens_attribute_argument_enum");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Attribute.c"), "class Attribute {}\n").unwrap();
        fs::write(root.join("EGameFlags.c"), "enum EGameFlags { Test, }\n").unwrap();
        let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
            roots: vec![crate::index_build::IndexSourceRoot::new(
                root.clone(),
                crate::model::SourceKind::GameData,
                crate::model::SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap()
        .index;
        let source = r#"class Example
{
	[Attribute(ParamEnumArray.FromEnum(EGameFlags))]
	void Run();
}
"#;

        let fast_report = semantic_tokens_report_for_source(source);
        assert_semantic_token(&fast_report, "EGameFlags", "class", Some("#40b5ac"));

        let rich_report = semantic_tokens_report_for_source_with_external(source, Some(&external));
        assert_semantic_token(&rich_report, "EGameFlags", "enum", Some("#40b5ac"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn semantic_tokens_color_attribute_expression_shape_before_external_index_is_ready() {
        let source = r#"class Example
{
	[Attribute("0", uiwidget: UIWidgets.Flags, "Test", "", ParamEnumArray.FromEnum(EGameFlags), WB_GAME_MODE_CATEGORY)]
	protected EGameFlags m_eTestGameFlags;
}
"#;

        let report = semantic_tokens_report_for_source(source);

        assert_semantic_token(&report, "Attribute", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "uiwidget", "variable", Some("#cfcfcf"));
        assert_semantic_token(&report, "UIWidgets", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "Flags", "enumMember", Some("#cfcfcf"));
        assert_semantic_token(&report, "ParamEnumArray", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "FromEnum", "method", Some("#f3ad58"));
        assert_semantic_token(&report, "EGameFlags", "class", Some("#40b5ac"));
        assert_semantic_token(
            &report,
            "WB_GAME_MODE_CATEGORY",
            "variable",
            Some("#cfcfcf"),
        );
        assert!(
            report
                .decoded
                .iter()
                .filter(|token| matches!(
                    token.text.as_str(),
                    "Attribute"
                        | "uiwidget"
                        | "UIWidgets"
                        | "Flags"
                        | "ParamEnumArray"
                        | "FromEnum"
                        | "WB_GAME_MODE_CATEGORY"
                ))
                .all(|token| token.token_type != "decorator"),
            "{:?}",
            report.decoded
        );
    }

    #[test]
    fn semantic_tokens_color_call_shapes_before_rich_resolution() {
        let source = r#"class Example
{
	void Run()
	{
		RunTimer();
		stateComponent.GetDuration();
	}
}
"#;

        let report = semantic_tokens_report_for_source(source);

        assert_semantic_token(&report, "RunTimer", "function", Some("#f3ad58"));
        assert_semantic_token(&report, "GetDuration", "method", Some("#f3ad58"));
    }

    #[test]
    fn semantic_tokens_color_static_member_shapes_before_rich_resolution() {
        let source = r#"class Example
{
	void Run()
	{
		SCR_BaseGameModeStateComponent stateComponent = GetStateComponent(SCR_EGameModeState.GAME);
		EHealthState.INJURED;
		stateComponent.GetDuration();
	}
}
"#;

        let report = semantic_tokens_report_for_source(source);

        assert_semantic_token(&report, "SCR_EGameModeState", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "GAME", "enumMember", Some("#cfcfcf"));
        assert_semantic_token(&report, "EHealthState", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "INJURED", "enumMember", Some("#cfcfcf"));
        assert!(
            !report
                .decoded
                .iter()
                .any(|token| token.text == "stateComponent" && token.token_type == "class"),
            "{:?}",
            report.decoded
        );
    }

    #[test]
    fn semantic_tokens_keep_comment_contents_comment_colored() {
        let source = r#"class Example
{
	//! \param[in] enable{} Set() true to enable supplies, set false to disable
	/*!
		\return[] // True{} <> if the game is hosted by a player (i.e., not dedicated server)
	*/
	void Run();
}
"#;

        let report = semantic_tokens_report_for_source(source);

        assert!(
            report.decoded.iter().any(|token| {
                token.token_type == "comment"
                    && token.text.contains("\\return[]")
                    && token.text.contains("True{} <> if")
            }),
            "{:?}",
            report.decoded
        );
        assert!(
            !report.decoded.iter().any(|token| {
                matches!(
                    token.text.as_str(),
                    "[" | "]" | "{" | "}" | "(" | ")" | "<" | ">" | "if" | "Set"
                ) && token.range.start.line >= 2
                    && token.range.end.line <= 5
                    && token.token_type != "comment"
            }),
            "{:?}",
            report.decoded
        );
    }

    #[test]
    fn semantic_token_cache_is_keyed_by_external_generation() {
        let mut cache = open_documents::SemanticTokenCache::default();
        let projection = LspSemanticTokenProjection {
            tokens: LspSemanticTokens {
                data: vec![1, 2, 3],
            },
            token_count: 1,
            parse_diagnostics: 0,
            timings: LspSemanticTokenTimings::default(),
        };

        cache.set_rich(7, 1, projection);

        assert!(cache
            .rich_for_revision_and_external_generation(7, 1)
            .is_some());
        assert!(cache
            .rich_for_revision_and_external_generation(7, 2)
            .is_none());
        assert!(cache
            .rich_for_revision_and_external_generation(8, 1)
            .is_none());
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
        assert_hover(source, "i++)", "i++", SymbolKind::LocalVariable, "i");
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
    fn completion_returns_members_for_receiver_and_replaces_prefix() {
        let source = r#"class Example
{
	void Run()
	{
		Widget widget;
		widget.Set
	}
}
"#;
        let external = file_index_for_source(
            r#"class Widget
{
	void SetVisible(bool visible);
	void SetText(string text);
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "widget.Set"),
            Some(&external),
        );

        assert_eq!(report.receiver_text.as_deref(), Some("widget"));
        assert_eq!(report.owner_type.as_deref(), Some("Widget"));
        assert_eq!(report.prefix, "Set");
        assert!(report.candidate_count >= 2);
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "SetVisible"
                && item.kind == 2
                && item.text_edit.new_text == "SetVisible(visible)"
                && item.insert_text_format == Some(2)
                && item
                    .label_details
                    .as_ref()
                    .and_then(|details| details.detail.as_deref())
                    == Some("(bool visible)")
                && item
                    .label_details
                    .as_ref()
                    .and_then(|details| details.description.as_deref())
                    == Some("-> void")
                && item.text_edit.range.start.character == 9
                && item.text_edit.range.end.character == 12));
    }

    #[test]
    fn completion_labels_overloads_and_sorts_workspace_before_game_data() {
        let source = r#"class Widget
{
	void SetVisible(bool visible);
	void SetVisible(bool visible, bool animate);
}

class Example
{
	void Run()
	{
		Widget widget;
		widget.Set
	}
}
"#;
        let external = file_index_for_source(
            r#"class Widget
{
	void SetText(string text);
}
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "widget.Set"),
            Some(&external),
        );

        let overload_details = report
            .list
            .items
            .iter()
            .filter(|item| item.label == "SetVisible")
            .filter_map(|item| {
                item.label_details
                    .as_ref()
                    .and_then(|details| details.detail.as_deref())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            overload_details,
            vec!["(bool visible)", "(bool visible, bool animate)"]
        );
        let first = report.list.items.first().unwrap();
        assert_eq!(first.label, "SetVisible");
        assert!(first.sort_text.as_deref().unwrap_or("").starts_with("01:"));
    }

    #[test]
    fn completion_returns_type_candidates_in_type_position() {
        let source = "class Example { void Run(SCR_ value) {} }";
        let external = file_index_for_source(
            r#"class SCR_Widget {}
enum SCR_Mode {}
typedef int SCR_Alias;
void SCR_Function();
int SCR_Global;
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "SCR_"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "type");
        assert_eq!(report.prefix, "SCR_");
        assert_eq!(
            report
                .list
                .items
                .iter()
                .map(|item| (item.label.as_str(), item.kind))
                .collect::<Vec<_>>(),
            vec![("SCR_Alias", 25), ("SCR_Mode", 13), ("SCR_Widget", 7)]
        );
        assert!(report
            .list
            .items
            .iter()
            .all(|item| item.text_edit.range.start.character == 25
                && item.text_edit.range.end.character == 29));
    }

    #[test]
    fn completion_uses_identifier_prefix_inside_existing_token() {
        let source = "class Example { void Run(SCR_Widget value) { GetGame(); } }";
        let external = file_index_for_source(
            r#"class SCR_Widget {}
class SCR_Other {}
void GetGame();
void GetGameMode();
"#,
        )
        .index;

        let type_report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "SCR_"),
            Some(&external),
        );
        let value_report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "GetG"),
            Some(&external),
        );

        assert_eq!(type_report.completion_context, "type");
        assert_eq!(type_report.prefix, "SCR_");
        assert!(type_report
            .list
            .items
            .iter()
            .any(|item| item.label == "SCR_Widget"));
        assert_eq!(value_report.completion_context, "top-level");
        assert_eq!(value_report.prefix, "GetG");
        assert!(value_report
            .list
            .items
            .iter()
            .any(|item| item.label == "GetGame"));
    }

    #[test]
    fn completion_returns_type_candidates_in_generic_type_argument() {
        let source = "class Example { void Run() { array<SCR_> values; } }";
        let external = file_index_for_source(
            r#"class SCR_Widget {}
void SCR_Function();
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "SCR_"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "type");
        assert_eq!(
            report
                .list
                .items
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            vec!["SCR_Widget"]
        );
    }

    #[test]
    fn completion_returns_top_level_value_candidates_for_prefix() {
        let source = "class Example { void Run() { SCR_ } }";
        let external = file_index_for_source(
            r#"class SCR_Widget {}
enum SCR_Mode
{
	SCR_Value
}
typedef int SCR_Alias;
void SCR_Function();
int SCR_Global;
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "SCR_"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "SCR_");
        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert!(labels.contains(&"SCR_Function"));
        assert!(labels.contains(&"SCR_Global"));
        assert!(labels.contains(&"SCR_Value"));
    }

    #[test]
    fn completion_returns_enum_members_for_static_enum_owner() {
        let source = r#"enum LogLevel
{
	DEBUG,
	NORMAL
}

class Example
{
	void Run()
	{
		LogLevel.
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "LogLevel."),
            None,
        );

        assert_eq!(report.receiver_text.as_deref(), Some("LogLevel"));
        assert_eq!(report.owner_type.as_deref(), Some("LogLevel"));
        assert_eq!(
            report
                .list
                .items
                .iter()
                .map(|item| (item.label.as_str(), item.kind))
                .collect::<Vec<_>>(),
            vec![("DEBUG", 20), ("NORMAL", 20)]
        );
    }

    #[test]
    fn completion_returns_static_class_members_for_static_class_owner() {
        let source = r#"class Example
{
	static int s_Value;
	static void StaticRun();
	void InstanceRun();
	int m_Value;
}

class User
{
	void Run()
	{
		Example.
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "Example."),
            None,
        );

        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(report.receiver_text.as_deref(), Some("Example"));
        assert_eq!(labels, vec!["s_Value", "StaticRun"]);
    }

    #[test]
    fn completion_returns_engine_class_cast_for_static_class_owner() {
        let source = r#"class Example
{
}

class User
{
	void Run()
	{
		Example.
	}
}
"#;
        let external = file_index_for_source(
            r#"class Class
{
	static Class Cast(Class from);
}

class Example
{
}
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "Example."),
            Some(&external),
        );

        assert_eq!(
            report
                .list
                .items
                .iter()
                .map(|item| (item.label.as_str(), item.kind))
                .collect::<Vec<_>>(),
            vec![("Cast", 2)]
        );
    }

    #[test]
    fn completion_expands_typedef_receiver_owner() {
        let source = r#"class Example
{
	void Run(TIntArray values)
	{
		values.
	}
}
"#;
        let external = file_index_for_source(
            r#"class array<Class T>
{
	void Insert(T value);
	void Remove(T value);
}

typedef array<int> TIntArray;
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "values."),
            Some(&external),
        );

        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(report.owner_type.as_deref(), Some("TIntArray"));
        assert_eq!(labels, vec!["Insert", "Remove"]);
    }

    #[test]
    fn completion_infers_direct_new_expression_receiver() {
        let source = r#"class SCR_AIAnimateBehavior
{
	array<string> GetPortNames();
}

class Example
{
	void Run()
	{
		(new SCR_AIAnimateBehavior()).
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "())."),
            None,
        );

        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            report.receiver_text.as_deref(),
            Some("(new SCR_AIAnimateBehavior())")
        );
        assert_eq!(report.owner_type.as_deref(), Some("SCR_AIAnimateBehavior"));
        assert_eq!(labels, vec!["GetPortNames"]);
    }

    #[test]
    fn completion_uses_full_receiver_chain_before_dot() {
        let source = r#"class AIWaypoint
{
	string ToString();
}

class SCR_BTParam<Class T>
{
	T m_Value;
}

class SCR_AIDefendBehavior
{
	ref SCR_BTParam<AIWaypoint> m_RelatedWaypoint;

	void Run()
	{
		m_RelatedWaypoint.m_Value.
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "m_Value."),
            None,
        );

        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            report.receiver_text.as_deref(),
            Some("m_RelatedWaypoint.m_Value")
        );
        assert_eq!(report.owner_type.as_deref(), Some("AIWaypoint"));
        assert_eq!(labels, vec!["ToString"]);
    }

    #[test]
    fn completion_returns_empty_for_non_member_positions_and_unresolved_receivers() {
        let non_member = "class Example {}";
        let non_member_report = completion_report_for_source_position_with_external(
            non_member,
            LspPosition {
                line: 0,
                character: 5,
            },
            None,
        );
        assert!(non_member_report.list.items.is_empty());

        let unresolved = "class Example { void Run() { missing. } }";
        let unresolved_report = completion_report_for_source_position_with_external(
            unresolved,
            position_after_needle(unresolved, "missing."),
            None,
        );
        assert_eq!(unresolved_report.receiver_text.as_deref(), Some("missing"));
        assert_eq!(unresolved_report.owner_type, None);
        assert!(unresolved_report.list.items.is_empty());
        assert_eq!(
            unresolved_report.failure_reason.as_deref(),
            Some("receiver type was not inferred")
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
    fn hover_does_not_use_broad_class_span_for_modifiers() {
        let source = r#"class Example : Base
{
	protected RplComponent m_RplComponent;
	private static const int COUNT = 4;
}
"#;

        for (needle, cursor) in [
            ("protected RplComponent", "protected"),
            ("private static", "private"),
            ("static const", "static"),
            ("const int", "const"),
        ] {
            let report = hover_at(source, needle, cursor);
            assert!(
                !report.is_hit(),
                "modifier `{cursor}` should not select enclosing symbol: {report:?}"
            );
        }
    }

    #[test]
    fn hover_returns_none_for_comments_inside_symbol_span() {
        let source = r#"class ExampleClass
{
	/*
		ExampleClass comment text should not select the class.
	*/
}
"#;

        let report = hover_at(source, "ExampleClass comment", "ExampleClass");

        assert!(!report.is_hit());
        assert_eq!(report.selection_source, HoverSelectionSource::None);
        assert_eq!(report.resolver_reason, None);
        assert_eq!(report.resolver_candidate_count, 0);
    }

    #[test]
    fn debug_hover_does_not_select_symbol_for_comments_inside_symbol_span() {
        let source = r#"class ExampleClass
{
	/*
		ExampleClass comment text should not select the class.
	*/
}
"#;
        let position = position_for_needle(source, "ExampleClass comment", "ExampleClass");

        let report = debug_hover_report_for_source_position(source, position);

        assert!(report.contains("- Selected Symbol: no"));
        assert!(report.contains("Cursor is not on an identifier token"));
        assert!(report.contains("No symbol matched the cursor position."));
        assert!(!report.contains("| 1 | syntax-span | `Class` | `ExampleClass`"));
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
    fn definition_selects_declarations_and_usages() {
        let source = r#"typedef string FactionKey;
Game g_Game;
enum ExampleFlags
{
	Enabled = 1
}
class Example
{
	protected int m_Value;
	void Run(string name)
	{
		int localValue = 5;
		localValue = localValue + 1;
		Print(name);
		m_Value = localValue;
		FactionKey key;
		ExampleFlags flag = ExampleFlags.Enabled;
		g_Game = null;
	}
}
"#;
        let uri = "file:///Scripts/Definition.c";

        assert_definition(
            source,
            uri,
            "class Example",
            "Example",
            SymbolKind::Class,
            "Example",
            "file:///Scripts/Definition.c",
        );
        assert_definition(
            source,
            uri,
            "localValue + 1",
            "localValue",
            SymbolKind::LocalVariable,
            "localValue",
            "file:///Scripts/Definition.c",
        );
        assert_definition(
            source,
            uri,
            "Print(name)",
            "name",
            SymbolKind::Parameter,
            "name",
            "file:///Scripts/Definition.c",
        );
        assert_definition(
            source,
            uri,
            "m_Value = localValue",
            "m_Value",
            SymbolKind::Field,
            "m_Value",
            "file:///Scripts/Definition.c",
        );
        assert_definition(
            source,
            uri,
            "FactionKey key",
            "FactionKey",
            SymbolKind::Typedef,
            "FactionKey",
            "file:///Scripts/Definition.c",
        );
        assert_definition(
            source,
            uri,
            "ExampleFlags.Enabled",
            "Enabled",
            SymbolKind::EnumMember,
            "Enabled",
            "file:///Scripts/Definition.c",
        );
        assert_definition(
            source,
            uri,
            "g_Game = null",
            "g_Game",
            SymbolKind::GlobalField,
            "g_Game",
            "file:///Scripts/Definition.c",
        );
    }

    #[test]
    fn definition_returns_null_for_non_targets() {
        let source = r#"class LogLevel {}
class Example
{
	void Run()
	{
		Print("hello", level: LogLevel);
		MissingThing();
	}
}
"#;
        let whitespace = definition_report_for_source_position(
            source,
            "file:///Scripts/Definition.c",
            LspPosition {
                line: 0,
                character: 0,
            },
        );
        assert!(!whitespace.is_hit());
        assert_eq!(whitespace.resolver_reason, None);

        let named_arg = definition_at(source, "level: LogLevel", "level");
        assert!(!named_arg.is_hit());
        assert_eq!(
            named_arg.resolver_reason,
            Some(ResolutionReason::NamedArgumentLabel)
        );

        let unresolved = definition_at(source, "MissingThing();", "MissingThing");
        assert!(!unresolved.is_hit());
        assert_eq!(
            unresolved.resolver_reason,
            Some(ResolutionReason::Unresolved)
        );
    }

    #[test]
    fn definition_resolves_preprocessor_macro_references_when_defined() {
        let source = r#"#define ENABLE_DIAG
#ifdef ENABLE_DIAG
#define GAME_MODE_DEBUG
#endif
"#;

        let report = definition_report_for_source_position(
            source,
            "file:///Scripts/Preprocessor.c",
            position_for_needle(source, "#ifdef ENABLE_DIAG", "ENABLE_DIAG"),
        );

        assert!(report.is_hit(), "{report:?}");
        assert_eq!(report.selected_kind, Some(SymbolKind::PreprocessorMacro));
        assert_eq!(report.selected_label.as_deref(), Some("ENABLE_DIAG"));
        assert_eq!(
            report.resolver_reason,
            Some(ResolutionReason::PreprocessorMacro)
        );
        assert_eq!(
            report.locations[0].range.start,
            LspPosition {
                line: 0,
                character: 8
            }
        );

        let missing = definition_report_for_source_position(
            "#ifdef MISSING_DIAG\n#endif\n",
            "file:///Scripts/Preprocessor.c",
            LspPosition {
                line: 0,
                character: 8,
            },
        );
        assert!(!missing.is_hit());
        assert_eq!(
            missing.resolver_reason,
            Some(ResolutionReason::PreprocessorMacro)
        );
    }

    #[test]
    fn definition_uses_external_file_uri_when_available() {
        let root = temp_test_dir("external_definition");
        fs::create_dir_all(&root).unwrap();
        let external_path = root.join("External Type.c");
        fs::write(&external_path, "class ExternalType\n{\n\tvoid Run();\n}\n").unwrap();
        let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
            roots: vec![crate::index_build::IndexSourceRoot::new(
                &root,
                crate::model::SourceKind::GameData,
                crate::model::SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap()
        .index;
        let source = r#"class Example
{
	void Run()
	{
		ExternalType value;
	}
}
"#;

        let report = definition_report_for_source_position_with_external(
            source,
            "file:///Scripts/Definition.c",
            position_for_needle(source, "ExternalType value", "ExternalType"),
            Some(&external),
        );

        assert!(report.is_hit());
        assert_eq!(report.selected_source, Some(CandidateSource::External));
        assert_eq!(report.selected_kind, Some(SymbolKind::Class));
        assert_eq!(report.selected_label.as_deref(), Some("ExternalType"));
        assert_eq!(report.locations.len(), 1);
        assert!(report.locations[0].uri.ends_with("/External%20Type.c"));
        assert_eq!(
            report.locations[0].range.start,
            LspPosition {
                line: 0,
                character: 6
            }
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn definition_resolves_keyword_type_positions_to_external_generated_types() {
        let root = temp_test_dir("external_keyword_type_definition");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("string.c"), "sealed class string\n{\n}\n").unwrap();
        fs::write(root.join("vector.c"), "sealed class vector\n{\n}\n").unwrap();
        fs::write(root.join("bool.c"), "sealed class bool\n{\n}\n").unwrap();
        let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
            roots: vec![crate::index_build::IndexSourceRoot::new(
                &root,
                crate::model::SourceKind::GameData,
                crate::model::SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap()
        .index;
        let source = r#"class Example
{
	string m_sValue;
	vector m_vValue;
	bool m_bValue;
	void Run()
	{
		bool value = true;
	}
}
"#;

        for (needle, cursor, expected) in [
            ("string m_sValue", "string", "string.c"),
            ("vector m_vValue", "vector", "vector.c"),
            ("bool m_bValue", "bool", "bool.c"),
        ] {
            let report = definition_report_for_source_position_with_external(
                source,
                "file:///Scripts/KeywordTypes.c",
                position_for_needle(source, needle, cursor),
                Some(&external),
            );
            assert!(report.is_hit(), "{cursor}: {report:?}");
            assert_eq!(report.selected_source, Some(CandidateSource::External));
            assert_eq!(report.selected_kind, Some(SymbolKind::Class));
            assert!(
                report.locations[0].uri.ends_with(expected),
                "{:?}",
                report.locations
            );
        }

        let literal = definition_report_for_source_position_with_external(
            source,
            "file:///Scripts/KeywordTypes.c",
            position_for_needle(source, "true;", "true"),
            Some(&external),
        );
        assert!(!literal.is_hit());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn definition_resolves_receiver_member_with_external_index() {
        let root = temp_test_dir("receiver_definition");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("Entity.c"),
            "class IEntity\n{\n\tvector GetOrigin();\n}\n",
        )
        .unwrap();
        let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
            roots: vec![crate::index_build::IndexSourceRoot::new(
                &root,
                crate::model::SourceKind::GameData,
                crate::model::SOURCE_PRIORITY_GAME_DATA,
            )],
        })
        .unwrap()
        .index;
        let source = r#"class Example
{
	void Run(IEntity ent)
	{
		ent.GetOrigin();
	}
}
"#;

        let report = definition_report_for_source_position_with_external(
            source,
            "file:///Scripts/Definition.c",
            position_for_needle(source, "ent.GetOrigin", "GetOrigin"),
            Some(&external),
        );

        assert!(report.is_hit());
        assert_eq!(report.selected_source, Some(CandidateSource::External));
        assert_eq!(report.selected_kind, Some(SymbolKind::Method));
        assert_eq!(report.selected_label.as_deref(), Some("GetOrigin"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_uri_for_path_encodes_windows_style_paths_and_spaces() {
        if cfg!(windows) {
            let uri = file_uri_for_path(Path::new("C:\\Game Data\\Scripts\\File Name.c")).unwrap();
            assert_eq!(uri, "file:///C:/Game%20Data/Scripts/File%20Name.c");
        } else {
            let uri = file_uri_for_path(Path::new("/tmp/Game Data/File Name.c")).unwrap();
            assert_eq!(uri, "file:///tmp/Game%20Data/File%20Name.c");
        }
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
                workspace_scripts: Vec::new(),
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"documentSymbolProvider\":true"));
        assert!(output_text.contains("\"name\":\"Smoke\""));
        assert!(output_text.contains("\"name\":\"Run\""));
    }

    #[test]
    fn framed_lsp_reuses_cached_document_symbols_for_repeated_requests() {
        let source = "class Smoke\n{\n\tvoid Run();\n}\n";
        let log_path = test_log_path("cached_document_symbols");
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
                    "method": "textDocument/documentSymbol",
                    "params": {
                        "textDocument": {
                            "uri": "file:///Scripts/Smoke.c"
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
                workspace_scripts: Vec::new(),
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert_eq!(output_text.matches("\"name\":\"Smoke\"").count(), 2);
        assert_eq!(output_text.matches("\"name\":\"Run\"").count(), 2);

        let log = std::fs::read_to_string(&log_path).unwrap();
        assert_eq!(log.matches("notification didOpen").count(), 1);
        assert_eq!(log.matches("analysis_elapsed_ms=").count(), 1);
        assert_eq!(log.matches("request documentSymbol").count(), 2);
        assert_eq!(log.matches("document_symbols_cached=true").count(), 3);

        cleanup_log(&log_path);
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
                workspace_scripts: Vec::new(),
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"hoverProvider\":true"));
        assert!(output_text.contains("\"completionProvider\":{\"triggerCharacters\":[\".\"]}"));
        assert!(output_text.contains("Smoke.Run(int value) -> void"));
        assert!(output_text.contains("\"kind\":\"markdown\""));
    }

    #[test]
    fn framed_lsp_smoke_test_handles_definition() {
        let source = "class Smoke\n{\n\tvoid Run(int value)\n\t{\n\t\tPrint(value);\n\t}\n}\n";
        let definition_position = position_for_needle(source, "Print(value)", "value");
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
                "method": "textDocument/definition",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Smoke.c"
                    },
                    "position": {
                        "line": definition_position.line,
                        "character": definition_position.character
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
                workspace_scripts: Vec::new(),
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"definitionProvider\":true"));
        assert!(output_text.contains("\"targetUri\":\"file:///Scripts/Smoke.c\""));
        assert!(output_text.contains("\"originSelectionRange\""));
        assert!(output_text.contains("\"targetRange\""));
        assert!(output_text.contains("\"targetSelectionRange\""));
        assert!(output_text.contains("\"line\":2"));
        assert!(output_text.contains("\"character\":14"));
    }

    #[test]
    fn framed_lsp_smoke_test_handles_member_completion() {
        let source = "class Widget\n{\n\tvoid SetVisible(bool visible);\n}\nclass Smoke\n{\n\tvoid Run()\n\t{\n\t\tWidget widget;\n\t\twidget.\n\t}\n}\n";
        let completion_position = position_after_needle(source, "widget.");
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
                "method": "textDocument/completion",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Smoke.c"
                    },
                    "position": {
                        "line": completion_position.line,
                        "character": completion_position.character
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
        run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"completionProvider\":{\"triggerCharacters\":[\".\"]}"));
        assert!(output_text.contains("\"isIncomplete\":false"));
        assert!(output_text.contains("\"label\":\"SetVisible\""));
        assert!(output_text.contains("\"newText\":\"SetVisible(visible)\""));
    }

    #[test]
    fn framed_lsp_smoke_test_handles_semantic_tokens() {
        let source =
            "class Smoke\n{\n\tvoid Run(int value)\n\t{\n\t\tstring name = \"x\";\n\t}\n}\n";
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
                "method": "textDocument/semanticTokens/full",
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
                "id": "server-1",
                "result": null
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/semanticTokens/full",
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
                "id": 5,
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
        run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"semanticTokensProvider\""));
        assert!(output_text.contains("\"tokenTypes\":[\"class\",\"enum\",\"type\""));
        assert!(output_text.contains("\"id\":2"));
        assert!(output_text.contains("\"method\":\"workspace/semanticTokens/refresh\""));
        assert!(output_text.contains("\"id\":4"));
        assert!(output_text.contains("\"data\":["));
    }

    #[test]
    fn framed_lsp_workspace_overlay_updates_hover_and_definition() {
        let root = temp_test_dir("workspace_overlay");
        let scripts = root.join("Scripts");
        std::fs::create_dir_all(&scripts).unwrap();
        let workspace_file = scripts.join("WorkspaceThing.c");
        let user_file = scripts.join("User.c");
        let workspace_source = "class WorkspaceThing\n{\n\tvoid WorkspaceMethod();\n}\n";
        std::fs::write(&workspace_file, workspace_source).unwrap();

        let user_source = "class User\n{\n\tvoid Run()\n\t{\n\t\tWorkspaceThing thing;\n\t\tthing.WorkspaceMethod();\n\t}\n}\n";
        let hover_position =
            position_for_needle(user_source, "thing.WorkspaceMethod", "WorkspaceMethod");
        let completion_position = position_after_needle(user_source, "thing.");
        let definition_position =
            position_for_needle(user_source, "WorkspaceThing thing", "WorkspaceThing");
        let user_uri = file_uri_for_path(&user_file).unwrap();
        let target_uri = file_uri_for_path(&workspace_file).unwrap();
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
                        "uri": user_uri,
                        "languageId": "enforce",
                        "version": 1,
                        "text": user_source
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "method": WORKSPACE_FILE_CHANGED_METHOD,
                "params": {
                    "path": workspace_file.display().to_string(),
                    "text": workspace_source
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
                        "uri": user_uri
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
                "method": "textDocument/completion",
                "params": {
                    "textDocument": {
                        "uri": user_uri
                    },
                    "position": {
                        "line": completion_position.line,
                        "character": completion_position.character
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/definition",
                "params": {
                    "textDocument": {
                        "uri": user_uri
                    },
                    "position": {
                        "line": definition_position.line,
                        "character": definition_position.character
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "method": WORKSPACE_FILE_DELETED_METHOD,
                "params": {
                    "path": workspace_file.display().to_string()
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": {
                        "uri": user_uri
                    },
                    "position": {
                        "line": completion_position.line,
                        "character": completion_position.character
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": {
                        "uri": user_uri
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
                "id": 7,
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
        run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("WorkspaceThing.WorkspaceMethod() -> void"));
        assert!(output_text.contains("\"label\":\"WorkspaceMethod\""));
        assert!(output_text.contains(&target_uri));
        assert!(output_text.contains(
            "{\"id\":5,\"jsonrpc\":\"2.0\",\"result\":{\"isIncomplete\":false,\"items\":[]}}"
        ));
        assert!(output_text.contains("{\"id\":6,\"jsonrpc\":\"2.0\",\"result\":null}"));

        let _ = std::fs::remove_dir_all(root);
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
                workspace_scripts: Vec::new(),
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
        let definition_position = position_for_needle(new_source, "NewRun", "NewRun");
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
                "method": "textDocument/definition",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Changed.c"
                    },
                    "position": {
                        "line": definition_position.line,
                        "character": definition_position.character
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 4,
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
                "id": 5,
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
                workspace_scripts: Vec::new(),
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("New.NewRun() -> void"));
        assert!(output_text.contains("\"name\":\"New\""));
        assert!(output_text.contains("\"name\":\"NewRun\""));
        assert!(output_text.contains("\"uri\":\"file:///Scripts/Changed.c\""));
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
                workspace_scripts: Vec::new(),
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"result\":null"));
        assert!(!output_text.contains("\"name\":\"Closed\""));
    }

    #[test]
    fn framed_lsp_publishes_and_clears_parser_diagnostics() {
        let broken_source = "class Broken\n{\n\tvoid Run(\n}\n";
        let fixed_source = "class Fixed\n{\n\tvoid Run();\n}\n";
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
                        "uri": "file:///Scripts/Diagnostics.c",
                        "languageId": "enforce",
                        "version": 1,
                        "text": broken_source
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
                        "uri": "file:///Scripts/Diagnostics.c",
                        "version": 2
                    },
                    "contentChanges": [
                        {
                            "text": fixed_source
                        }
                    ]
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
                        "uri": "file:///Scripts/Diagnostics.c"
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 2,
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
                workspace_scripts: Vec::new(),
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert_eq!(
            output_text
                .matches("textDocument/publishDiagnostics")
                .count(),
            3
        );
        assert!(output_text.contains("Reforger Script Tools parser"));
        assert!(output_text.contains("reforger.parser.syntax"));
        assert!(output_text.contains("\"severity\":1"));
        assert!(output_text.contains("\"diagnostics\":[]"));
    }

    #[test]
    fn parser_diagnostic_projection_adds_stable_source_and_code() {
        let source = "class Broken\n{\n\tvoid Run(\n}\n";
        let parse = parse_source(source);

        let diagnostics = parser_diagnostics_for_source(source, &parse.diagnostics);

        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.source == diagnostics::PARSER_DIAGNOSTIC_SOURCE));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == diagnostics::PARSER_DIAGNOSTIC_CODE));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == 1));
    }

    #[test]
    fn parser_diagnostic_projection_expands_zero_width_ranges() {
        let source = "class Broken\n";
        let diagnostics = parser_diagnostics_for_source(
            source,
            &[ParseDiagnostic {
                message: "Expected declaration".to_string(),
                span: TextSpan::new(source.len(), source.len()),
            }],
        );

        let range = diagnostics[0].range;
        assert_ne!(
            range.start, range.end,
            "zero-width parser diagnostics should project to a visible editor range"
        );
    }

    #[test]
    fn debug_hover_report_includes_language_engine_context() {
        let source = "class Smoke\n{\n\tvoid Run(int value);\n}\n";
        let hover_position = position_for_needle(source, "Run(int", "Run");

        let report = debug_hover_report_for_source_position(source, hover_position);

        assert!(report.contains("# Reforger Hover Debug"));
        assert!(report.contains("## Resolver Resolution"));
        assert!(report.contains("## Tokens Around Cursor"));
        assert!(report.contains("## Semantic Token Coloring Context"));
        assert!(report.contains("## Candidate Symbols At Cursor"));
        assert!(report.contains("- Selected Symbol: yes"));
        assert!(report.contains("- Label: `Run`"));
        assert!(report.contains("Smoke.Run(int value) -> void"));
        assert!(report.contains("`Method`"));
        assert!(report.contains("`method`"));
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
                workspace_scripts: Vec::new(),
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

    fn temp_test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "reforger_lsp_{name}_{}_{}",
            std::process::id(),
            timestamp_millis()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
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

    fn assert_definition(
        source: &str,
        uri: &str,
        needle: &str,
        cursor: &str,
        expected_kind: SymbolKind,
        expected_label: &str,
        expected_uri: &str,
    ) {
        let report = definition_report_for_source_position(
            source,
            uri,
            position_for_needle(source, needle, cursor),
        );
        assert!(
            report.is_hit(),
            "definition miss for needle `{needle}` cursor `{cursor}`"
        );
        assert_eq!(report.parse_diagnostics, 0);
        assert_eq!(report.selected_kind, Some(expected_kind));
        assert_eq!(report.selected_label.as_deref(), Some(expected_label));
        assert_eq!(report.locations.len(), 1);
        assert_eq!(report.locations[0].uri, expected_uri);
    }

    fn definition_at(source: &str, needle: &str, cursor: &str) -> LspDefinitionReport {
        definition_report_for_source_position(
            source,
            "file:///Scripts/Definition.c",
            position_for_needle(source, needle, cursor),
        )
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

    fn position_after_needle(source: &str, needle: &str) -> LspPosition {
        let start = source
            .find(needle)
            .unwrap_or_else(|| panic!("missing needle {needle}"));
        position_for_offset(source, start + needle.len())
    }

    fn assert_semantic_token(
        report: &LspSemanticTokenReport,
        text: &str,
        token_type: &str,
        color: Option<&str>,
    ) {
        assert!(
            report.decoded.iter().any(|token| {
                token.text == text
                    && token.token_type == token_type
                    && color.is_none_or(|color| token.color == color)
            }),
            "missing semantic token text={text:?} type={token_type:?} color={color:?}: {:?}",
            report.decoded
        );
    }

    fn assert_semantic_token_count_at_least(
        report: &LspSemanticTokenReport,
        text: &str,
        token_type: &str,
        expected: usize,
    ) {
        let actual = report
            .decoded
            .iter()
            .filter(|token| token.text == text && token.token_type == token_type)
            .count();
        assert!(
            actual >= expected,
            "expected at least {expected} semantic tokens text={text:?} type={token_type:?}, found {actual}: {:?}",
            report.decoded
        );
    }

    fn assert_semantic_type_family_token_count_at_least(
        report: &LspSemanticTokenReport,
        text: &str,
        expected: usize,
    ) {
        let actual = report
            .decoded
            .iter()
            .filter(|token| {
                token.text == text
                    && matches!(
                        token.token_type,
                        "class" | "enum" | "type" | "typeParameter"
                    )
                    && token.color == "#40b5ac"
            })
            .count();
        assert!(
            actual >= expected,
            "expected at least {expected} green type-family semantic tokens text={text:?}, found {actual}: {:?}",
            report.decoded
        );
    }
}
