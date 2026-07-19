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
#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Condvar, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

mod callable;
mod completion;
mod debug_hover;
mod definition;
mod diagnostics;
mod external_overlay;
mod hover;
mod hover_render;
mod open_documents;
mod semantic_tokens;
mod signature_help;

use completion::{
    completion_debug_markdown, completion_report_for_cached_analysis_with_external_indexes,
    empty_completion_list,
};
pub use completion::{
    completion_report_for_cached_analysis_with_external,
    completion_report_for_source_position_with_external, LspCompletionItem,
    LspCompletionItemLabelDetails, LspCompletionList, LspCompletionReport, LspCompletionTimings,
    LspTextEdit,
};
use debug_hover::debug_hover_report_for_cached_analysis_with_external_indexes;
pub use debug_hover::debug_hover_report_for_source_position;
pub(crate) use debug_hover::selected_label_from_debug_report;
use definition::definition_report_for_cached_analysis_with_external_indexes;
pub(crate) use definition::file_uri_for_path;
pub use definition::{
    definition_report_for_cached_analysis_with_external, definition_report_for_source_position,
    definition_report_for_source_position_with_external, LspDefinitionReport, LspLocation,
    LspLocationLink,
};
use diagnostics::{clear_diagnostics_message, publish_diagnostics_message};
pub use diagnostics::{parser_diagnostics_for_source, LspDiagnostic};
pub(crate) use external_overlay::ExternalIndexStatusSummary;
use external_overlay::{start_external_index, ExternalIndexHandle, ExternalIndexSnapshot};
use hover::hover_report_for_cached_analysis_with_external_indexes;
pub use hover::{
    hover_report_for_source_position, hover_report_for_source_position_with_external,
    hover_reports_for_source_positions, hover_reports_for_source_positions_with_external,
    HoverSelectionSource, LspHover, LspHoverReport,
};
pub use open_documents::{file_index_for_source, FileIndexAnalysis};
pub(crate) use open_documents::{
    file_index_for_source_with_timings, FileIndexAnalysisTimings, OpenDocument,
};
use semantic_tokens::{
    fast_semantic_tokens_for_cached_analysis,
    semantic_tokens_for_cached_analysis_with_external_indexes,
    semantic_tokens_for_cached_analysis_with_external_indexes_cancelled,
    LspSemanticTokenProjection, LspSemanticTokens, SEMANTIC_TOKEN_MODIFIERS, SEMANTIC_TOKEN_TYPES,
};
pub use semantic_tokens::{
    fast_semantic_tokens_for_source, fast_semantic_tokens_report_for_source,
    semantic_tokens_for_source_with_external, semantic_tokens_report_for_source,
    semantic_tokens_report_for_source_with_external, LspSemanticTokenReport,
    LspSemanticTokenTimings, SemanticTokenDebug,
};
use signature_help::{
    signature_help_debug_markdown, signature_help_report_for_cached_analysis_with_external_indexes,
};
pub use signature_help::{
    signature_help_report_for_source_position, LspParameterInformation, LspSignatureHelp,
    LspSignatureHelpReport, LspSignatureHelpTimings, LspSignatureInformation,
};

const SERVER_NAME: &str = "reforger-language-server";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const SIGNATURE_HELP_TRIGGER_CHARACTERS: &[&str] = &[
    "(", ",", ".", ":", "_", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n",
    "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z", "A", "B", "C", "D", "E", "F", "G",
    "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
];
const SIGNATURE_HELP_RETRIGGER_CHARACTERS: &[&str] = SIGNATURE_HELP_TRIGGER_CHARACTERS;
const DEBUG_HOVER_METHOD: &str = "reforger/debugHover";
const DEBUG_COMPLETION_METHOD: &str = "reforger/debugCompletion";
const WORKSPACE_FILE_CHANGED_METHOD: &str = "reforger/workspaceFileChanged";
const WORKSPACE_FILE_DELETED_METHOD: &str = "reforger/workspaceFileDeleted";
const MAX_LSP_HEADER_LINE_BYTES: usize = 8 * 1024;
const MAX_LSP_HEADER_BYTES: usize = 32 * 1024;
const MAX_LSP_BODY_BYTES: usize = 16 * 1024 * 1024;
const INCOMING_EVENT_QUEUE_CAPACITY: usize = 64;
const DOCUMENT_ANALYSIS_IDLE_DELAY_MS: u64 = 150;
const MAX_PENDING_DOCUMENT_REQUESTS_PER_URI: usize = 32;
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
    let (incoming_sender, incoming_receiver) = mpsc::sync_channel(INCOMING_EVENT_QUEUE_CAPACITY);
    let (internal_sender, internal_receiver) = mpsc::channel();
    let rich_scheduler = RichSemanticTokensScheduler::start(internal_sender.clone());
    let analysis_scheduler = OpenDocumentAnalysisScheduler::start(internal_sender.clone());
    let mut server = LspServer::new_with_runtime_senders(
        stdout.lock(),
        options,
        Some(rich_scheduler),
        Some(analysis_scheduler),
    );
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        loop {
            match read_message(&mut reader) {
                Ok(Some(message)) => {
                    if incoming_sender
                        .send(ServerEvent::Incoming {
                            received_at: Instant::now(),
                            result: Ok(message),
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = incoming_sender.send(ServerEvent::Incoming {
                        received_at: Instant::now(),
                        result: Err(error),
                    });
                    break;
                }
            }
        }
    });
    server.run_message_channels(incoming_receiver, internal_receiver)
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
    document_revisions: BTreeMap<String, u64>,
    logger: LspLogger,
    external_index: ExternalIndexHandle,
    rich_scheduler: Option<RichSemanticTokensScheduler>,
    analysis_scheduler: Option<OpenDocumentAnalysisScheduler>,
    deferred_document_requests: BTreeMap<String, Vec<DeferredDocumentRequest>>,
    next_server_request_id: u64,
    semantic_tokens_refresh_in_flight: Option<String>,
    semantic_tokens_refresh_dirty: bool,
    last_semantic_external_generation: u64,
    shutdown_requested: bool,
}

const RICH_SEMANTIC_TOKENS_IDLE_DELAY_MS: u64 = 250;
const MAX_PENDING_RICH_SEMANTIC_TOKEN_JOBS: usize = 16;

enum ServerEvent {
    Incoming {
        received_at: Instant,
        result: Result<Value, String>,
    },
    RichSemanticTokensReady {
        uri: String,
        revision: u64,
        external_generation: u64,
        external_status: &'static str,
        projection: LspSemanticTokenProjection,
        elapsed_ms: u128,
    },
    RichSemanticTokensSkipped {
        uri: String,
        revision: u64,
        external_generation: u64,
        reason: String,
        elapsed_ms: u128,
    },
    DocumentAnalysisReady {
        uri: String,
        revision: u64,
        analysis: FileIndexAnalysis,
        timings: FileIndexAnalysisTimings,
        elapsed_ms: u128,
    },
    DocumentAnalysisSkipped {
        uri: String,
        revision: u64,
        reason: String,
        elapsed_ms: u128,
    },
}

struct DeferredDocumentRequest {
    revision: u64,
    received_at: Instant,
    value: Value,
}

struct OpenDocumentAnalysisJob {
    uri: String,
    revision: u64,
    source: String,
    cancel: Arc<AtomicBool>,
    scheduled_at: Instant,
}

#[derive(Clone)]
struct OpenDocumentAnalysisScheduler {
    state: Arc<(Mutex<BTreeMap<String, OpenDocumentAnalysisJob>>, Condvar)>,
    sender: mpsc::Sender<ServerEvent>,
}

impl OpenDocumentAnalysisScheduler {
    fn start(sender: mpsc::Sender<ServerEvent>) -> Self {
        let scheduler = Self {
            state: Arc::new((Mutex::new(BTreeMap::new()), Condvar::new())),
            sender,
        };
        let worker = scheduler.clone();
        thread::spawn(move || worker.run());
        scheduler
    }

    fn schedule(&self, job: OpenDocumentAnalysisJob) {
        let (lock, wake) = &*self.state;
        let mut pending = lock.lock().unwrap();
        if let Some(previous) = pending.insert(job.uri.clone(), job) {
            previous.cancel.store(true, Ordering::Relaxed);
        }
        wake.notify_one();
    }

    fn run(self) {
        let (lock, wake) = &*self.state;
        loop {
            let mut pending = lock.lock().unwrap();
            while pending.is_empty() {
                pending = wake.wait(pending).unwrap();
            }
            let key = earliest_due_document_analysis_uri(&pending).unwrap();
            let due_at =
                pending[&key].scheduled_at + Duration::from_millis(DOCUMENT_ANALYSIS_IDLE_DELAY_MS);
            let now = Instant::now();
            if now < due_at {
                let (pending_after_wait, _) = wake.wait_timeout(pending, due_at - now).unwrap();
                pending = pending_after_wait;
                continue;
            }
            let Some(job) = pending.remove(&key) else {
                continue;
            };
            drop(pending);
            if job.cancel.load(Ordering::Relaxed) {
                continue;
            }
            let (analysis, timings) = file_index_for_source_with_timings(&job.source);
            let event = if job.cancel.load(Ordering::Relaxed) {
                ServerEvent::DocumentAnalysisSkipped {
                    uri: job.uri,
                    revision: job.revision,
                    reason: "superseded-during-analysis".to_string(),
                    elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                }
            } else {
                ServerEvent::DocumentAnalysisReady {
                    uri: job.uri,
                    revision: job.revision,
                    analysis,
                    timings,
                    elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                }
            };
            let _ = self.sender.send(event);
        }
    }
}

fn earliest_due_document_analysis_uri(
    pending: &BTreeMap<String, OpenDocumentAnalysisJob>,
) -> Option<String> {
    pending
        .iter()
        .min_by_key(|(uri, job)| (job.scheduled_at, *uri))
        .map(|(uri, _)| uri.clone())
}

fn source_backed_request_method(method: &str) -> bool {
    matches!(
        method,
        "textDocument/documentSymbol"
            | "textDocument/hover"
            | "textDocument/definition"
            | "textDocument/completion"
            | "textDocument/signatureHelp"
            | "textDocument/semanticTokens/full"
            | DEBUG_HOVER_METHOD
            | DEBUG_COMPLETION_METHOD
    )
}

fn request_document_uri(params: Option<&Value>) -> Option<String> {
    params?
        .get("textDocument")?
        .get("uri")?
        .as_str()
        .map(str::to_string)
}

struct RichSemanticTokensJob {
    uri: String,
    revision: u64,
    external_generation: u64,
    cancel: Arc<AtomicBool>,
    scheduled_at: Instant,
    source: String,
    analysis: FileIndexAnalysis,
    external_snapshot: ExternalIndexSnapshot,
}

fn oldest_pending_uri(pending: &BTreeMap<String, RichSemanticTokensJob>) -> Option<String> {
    pending
        .iter()
        .min_by_key(|(_, job)| job.scheduled_at)
        .map(|(uri, _)| uri.clone())
}

fn earliest_due_pending_uri(pending: &BTreeMap<String, RichSemanticTokensJob>) -> Option<String> {
    pending
        .iter()
        .min_by_key(|(uri, job)| (job.scheduled_at, *uri))
        .map(|(uri, _)| uri.clone())
}

#[derive(Clone)]
struct RichSemanticTokensScheduler {
    state: Arc<(Mutex<BTreeMap<String, RichSemanticTokensJob>>, Condvar)>,
    sender: mpsc::Sender<ServerEvent>,
}

impl RichSemanticTokensScheduler {
    fn start(sender: mpsc::Sender<ServerEvent>) -> Self {
        let scheduler = Self {
            state: Arc::new((Mutex::new(BTreeMap::new()), Condvar::new())),
            sender,
        };
        let worker = scheduler.clone();
        thread::spawn(move || worker.run());
        scheduler
    }

    fn schedule(&self, job: RichSemanticTokensJob) {
        let (lock, wake) = &*self.state;
        let mut pending = lock.lock().unwrap();
        if !pending.contains_key(&job.uri) && pending.len() >= MAX_PENDING_RICH_SEMANTIC_TOKEN_JOBS
        {
            let evicted_uri = oldest_pending_uri(&pending).unwrap();
            let evicted = pending.remove(&evicted_uri).unwrap();
            evicted.cancel.store(true, Ordering::Relaxed);
            let _ = self.sender.send(ServerEvent::RichSemanticTokensSkipped {
                uri: evicted.uri,
                revision: evicted.revision,
                external_generation: evicted.external_generation,
                reason: "scheduler-capacity-evicted".to_string(),
                elapsed_ms: evicted.scheduled_at.elapsed().as_millis(),
            });
        }
        if let Some(previous) = pending.insert(job.uri.clone(), job) {
            previous.cancel.store(true, Ordering::Relaxed);
        }
        wake.notify_one();
    }

    fn run(self) {
        let (lock, wake) = &*self.state;
        loop {
            let mut pending = lock.lock().unwrap();
            while pending.is_empty() {
                pending = wake.wait(pending).unwrap();
            }
            let key = earliest_due_pending_uri(&pending).unwrap();
            let due_at = pending[&key].scheduled_at
                + Duration::from_millis(RICH_SEMANTIC_TOKENS_IDLE_DELAY_MS);
            let now = Instant::now();
            if now < due_at {
                let (pending_after_wait, _) = wake.wait_timeout(pending, due_at - now).unwrap();
                pending = pending_after_wait;
                continue;
            }
            let Some(job) = pending.remove(&key) else {
                continue;
            };
            drop(pending);
            if job.cancel.load(Ordering::Relaxed) {
                continue;
            }
            let projection = semantic_tokens_for_cached_analysis_with_external_indexes_cancelled(
                &job.source,
                &job.analysis,
                job.external_snapshot.workspace.as_deref(),
                job.external_snapshot.game_data.as_deref(),
                &|| job.cancel.load(Ordering::Relaxed),
            );
            let event = match projection {
                Some(projection) => ServerEvent::RichSemanticTokensReady {
                    uri: job.uri,
                    revision: job.revision,
                    external_generation: job.external_generation,
                    external_status: job.external_snapshot.status,
                    projection,
                    elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                },
                None => ServerEvent::RichSemanticTokensSkipped {
                    uri: job.uri,
                    revision: job.revision,
                    external_generation: job.external_generation,
                    reason: "cancelled-during-work".to_string(),
                    elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                },
            };
            let _ = self.sender.send(event);
        }
    }
}

#[derive(Clone)]
struct LspLogger {
    path: Option<PathBuf>,
    lock: Arc<Mutex<()>>,
}

impl LspLogger {
    fn new(path: Option<PathBuf>) -> Self {
        if let Some(log_path) = path.as_ref() {
            if let Some(parent) = log_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
        }
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
    version: i32,
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
    version: i32,
}

#[derive(Debug, Deserialize)]
struct TextDocumentContentChangeEvent {
    #[serde(default)]
    range: Option<Value>,
    #[serde(rename = "rangeLength")]
    #[serde(default)]
    range_length: Option<u32>,
    text: String,
}

#[derive(Debug)]
struct CoalescibleDidChange {
    uri: String,
    version: i32,
}

fn coalescible_full_sync_did_change(value: &Value) -> Option<CoalescibleDidChange> {
    let message: RpcMessage = serde_json::from_value(value.clone()).ok()?;
    if message.id.is_some() || message.method.as_deref() != Some("textDocument/didChange") {
        return None;
    }
    let params: DidChangeTextDocumentParams = serde_json::from_value(message.params?).ok()?;
    (params.content_changes.len() == 1
        && params.content_changes[0].range.is_none()
        && params.content_changes[0].range_length.is_none())
    .then_some(CoalescibleDidChange {
        uri: params.text_document.uri,
        version: params.text_document.version,
    })
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
    sequence: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceFileDeletedParams {
    path: String,
    sequence: u64,
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
        Self::new_with_runtime_senders(writer, options, None, None)
    }

    fn new_with_runtime_senders(
        writer: W,
        options: LspServerOptions,
        rich_scheduler: Option<RichSemanticTokensScheduler>,
        analysis_scheduler: Option<OpenDocumentAnalysisScheduler>,
    ) -> Self {
        let logger = LspLogger::new(options.log_path.clone());
        let external_index = start_external_index(&options, logger.clone());
        let server = Self {
            writer,
            documents: BTreeMap::new(),
            document_revisions: BTreeMap::new(),
            logger,
            external_index,
            rich_scheduler,
            analysis_scheduler,
            deferred_document_requests: BTreeMap::new(),
            next_server_request_id: 1,
            semantic_tokens_refresh_in_flight: None,
            semantic_tokens_refresh_dirty: false,
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

    fn next_document_revision(&mut self, uri: &str) -> u64 {
        let revision = self
            .document_revisions
            .get(uri)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        self.document_revisions.insert(uri.to_string(), revision);
        revision
    }

    fn run<R: Read>(&mut self, reader: R) -> Result<(), String> {
        let mut reader = BufReader::new(reader);
        while let Some(message) = read_message(&mut reader)? {
            let should_exit = self.handle_message(message, None, 0, 0)?;
            if should_exit {
                break;
            }
        }
        self.log("exit");
        Ok(())
    }

    fn run_message_channels(
        &mut self,
        incoming_receiver: mpsc::Receiver<ServerEvent>,
        internal_receiver: mpsc::Receiver<ServerEvent>,
    ) -> Result<(), String> {
        let mut deferred_incoming = VecDeque::new();
        loop {
            for _ in 0..INCOMING_EVENT_QUEUE_CAPACITY {
                match internal_receiver.try_recv() {
                    Ok(event) => self.handle_internal_event(event)?,
                    Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
                }
            }

            let next_event = deferred_incoming
                .pop_front()
                .map(Ok)
                .unwrap_or_else(|| incoming_receiver.recv_timeout(Duration::from_millis(100)));
            match next_event {
                Ok(ServerEvent::Incoming {
                    received_at,
                    result: Ok(message),
                }) => {
                    let mut selected_message = message;
                    let mut selected_received_at = received_at;
                    let mut coalesced_changes = 1usize;
                    let mut superseded_changes = 0usize;
                    let Some(first_change) = coalescible_full_sync_did_change(&selected_message)
                    else {
                        let should_exit = self.handle_message(
                            selected_message,
                            Some(selected_received_at.elapsed().as_millis()),
                            0,
                            0,
                        )?;
                        if should_exit {
                            break;
                        }
                        continue;
                    };

                    while coalesced_changes < INCOMING_EVENT_QUEUE_CAPACITY {
                        let Ok(next_event) = incoming_receiver.try_recv() else {
                            break;
                        };
                        let ServerEvent::Incoming {
                            received_at,
                            result: Ok(next_message),
                        } = next_event
                        else {
                            deferred_incoming.push_back(next_event);
                            break;
                        };
                        let Some(next_change) = coalescible_full_sync_did_change(&next_message)
                        else {
                            deferred_incoming.push_back(ServerEvent::Incoming {
                                received_at,
                                result: Ok(next_message),
                            });
                            break;
                        };
                        if next_change.uri != first_change.uri {
                            deferred_incoming.push_back(ServerEvent::Incoming {
                                received_at,
                                result: Ok(next_message),
                            });
                            break;
                        }
                        coalesced_changes += 1;
                        if next_change.version
                            > coalescible_full_sync_did_change(&selected_message)
                                .expect("selected message remains coalescible")
                                .version
                        {
                            selected_message = next_message;
                            selected_received_at = received_at;
                            superseded_changes += 1;
                        } else {
                            superseded_changes += 1;
                        }
                    }
                    let should_exit = self.handle_message(
                        selected_message,
                        Some(selected_received_at.elapsed().as_millis()),
                        coalesced_changes,
                        superseded_changes,
                    )?;
                    if should_exit {
                        break;
                    }
                }
                Ok(ServerEvent::Incoming {
                    result: Err(error), ..
                }) => return Err(error),
                Ok(event) => self.handle_internal_event(event)?,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    self.request_semantic_tokens_refresh_if_external_generation_changed()?;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        self.log("exit");
        Ok(())
    }

    fn handle_message(
        &mut self,
        value: Value,
        queue_ms: Option<u128>,
        coalesced_changes: usize,
        superseded_changes: usize,
    ) -> Result<bool, String> {
        let message = serde_json::from_value::<RpcMessage>(value.clone())
            .map_err(|error| format!("Invalid JSON-RPC message: {error}"))?;
        let queue_ms = queue_ms.unwrap_or(0);
        let Some(method) = message.method.as_deref() else {
            self.handle_semantic_tokens_refresh_response(&message)?;
            return Ok(false);
        };

        if self.shutdown_requested && method != "exit" {
            let error = "Server has already received shutdown";
            if let Some(id) = message.id.clone() {
                self.respond_error(id, -32600, error)?;
            } else {
                self.log(&format!(
                    "notification ignored after shutdown method={method}"
                ));
            }
            return Ok(false);
        }

        if let Err(error) = validate_message_params(method, &message.params) {
            if let Some(id) = message.id.clone() {
                self.respond_error(id, -32602, &error)?;
            } else {
                self.log(&format!(
                    "notification ignored invalid_params method={method} error={error}"
                ));
            }
            return Ok(false);
        }

        if message.id.is_some()
            && source_backed_request_method(method)
            && self.defer_request_while_document_analysis_is_pending(&message, value.clone())?
        {
            return Ok(false);
        }

        match method {
            "initialize" => {
                self.log("request initialize");
                if let Some(id) = message.id {
                    self.respond(
                        id,
                        json!({
                            "capabilities": {
                                "textDocumentSync": {
                                    "openClose": true,
                                    "change": 1
                                },
                                "documentSymbolProvider": true,
                                "hoverProvider": true,
                                "definitionProvider": true,
                                "completionProvider": {
                                    "triggerCharacters": [".", "["]
                                },
                                "signatureHelpProvider": {
                                    "triggerCharacters": SIGNATURE_HELP_TRIGGER_CHARACTERS,
                                    "retriggerCharacters": SIGNATURE_HELP_RETRIGGER_CHARACTERS
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
                if !self.shutdown_requested {
                    return Err("LSP exit received before shutdown".to_string());
                }
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
                    let revision = self.next_document_revision(&uri);
                    if let Some(mut previous) = self.documents.remove(&uri) {
                        previous.semantic_tokens.cancel_pending();
                        previous.cancel_pending_analysis();
                    }
                    self.discard_deferred_document_requests(&uri, revision)?;
                    let mut document = OpenDocument::new(text, version, revision);
                    let symbol_start = Instant::now();
                    let symbols =
                        document_symbols_from_cached_analysis(&document.text, &document.analysis);
                    let document_symbol_ms = symbol_start.elapsed().as_millis();
                    let symbol_count = document_symbol_count(&symbols);
                    document.set_document_symbols(symbols);
                    let parse_diagnostics = document.analysis.parse_diagnostics;
                    let revision = document.revision;
                    let analysis_timings = document.analysis_timings;
                    let diagnostics_message = publish_diagnostics_message(
                        &uri,
                        version,
                        &document.text,
                        &document.analysis.diagnostics,
                    );
                    self.documents.insert(uri.clone(), document);
                    self.write_message(diagnostics_message)?;
                    self.log(&format!(
                        "notification didOpen uri={} bytes={} version={} revision={} cached_analysis=true document_symbols_cached=true symbols={} parse_diagnostics={} analysis_parse_ms={} analysis_catalog_ms={} analysis_index_ms={} analysis_scope_ms={} analysis_build_ms={} document_symbol_ms={} queue_ms={} analysis_elapsed_ms={}",
                        uri,
                        bytes,
                        version,
                        revision,
                        symbol_count,
                        parse_diagnostics,
                        analysis_timings.parse_ms,
                        analysis_timings.catalog_ms,
                        analysis_timings.index_ms,
                        analysis_timings.scope_ms,
                        analysis_timings.total_ms,
                        document_symbol_ms,
                        queue_ms,
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
                        let Some(current_version) =
                            self.documents.get(&uri).map(|document| document.version)
                        else {
                            self.log(&format!(
                                "notification didChange ignored uri={} version={} reason=not_open",
                                uri, version
                            ));
                            return Ok(false);
                        };
                        if version <= current_version {
                            self.log(&format!(
                                "notification didChange ignored uri={} version={} current_version={} reason=stale",
                                uri, version, current_version
                            ));
                            return Ok(false);
                        }
                        let document = self
                            .documents
                            .get_mut(&uri)
                            .expect("open document exists after version check");
                        document.replace(text, version);
                        let revision = document.revision;
                        self.document_revisions.insert(uri.clone(), revision);
                        let source = document.text.clone();
                        let analysis_cancel = document.mark_analysis_pending();
                        if let Some(scheduler) = self.analysis_scheduler.clone() {
                            self.discard_deferred_document_requests(&uri, revision)?;
                            scheduler.schedule(OpenDocumentAnalysisJob {
                                uri: uri.clone(),
                                revision,
                                source,
                                cancel: analysis_cancel,
                                scheduled_at: Instant::now(),
                            });
                            self.log(&format!(
                                "notification didChange uri={} bytes={} version={} revision={} cached_analysis=false analysis_state=pending queue_ms={} coalesced_changes={} superseded_changes={} analysis_elapsed_ms={}",
                                uri, bytes, version, revision, queue_ms, coalesced_changes, superseded_changes, start.elapsed().as_millis()
                            ));
                        } else {
                            let (analysis, timings) = file_index_for_source_with_timings(&source);
                            self.log(&format!(
                                "notification didChange uri={} bytes={} version={} revision={} cached_analysis=true document_symbols_cached=false symbols=pending parse_diagnostics={} analysis_parse_ms={} analysis_catalog_ms={} analysis_index_ms={} analysis_scope_ms={} analysis_build_ms={} queue_ms={} coalesced_changes={} superseded_changes={} analysis_elapsed_ms={}",
                                uri,
                                bytes,
                                version,
                                revision,
                                analysis.parse_diagnostics,
                                timings.parse_ms,
                                timings.catalog_ms,
                                timings.index_ms,
                                timings.scope_ms,
                                timings.total_ms,
                                queue_ms,
                                coalesced_changes,
                                superseded_changes,
                                start.elapsed().as_millis()
                            ));
                            self.install_document_analysis(
                                &uri,
                                revision,
                                analysis,
                                timings,
                                start.elapsed().as_millis(),
                            )?;
                        }
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
                    if let Some(mut document) = self.documents.remove(&params.text_document.uri) {
                        document.semantic_tokens.cancel_pending();
                        document.cancel_pending_analysis();
                    }
                    self.discard_deferred_document_requests(&params.text_document.uri, 0)?;
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
                    let result = self.external_index.update_workspace_file(
                        path.clone(),
                        params.text,
                        params.sequence,
                    );
                    match result {
                        Ok(Some((symbols, parse_diagnostics))) => {
                            let status = self.external_index.status_summary();
                            self.log(&format!(
                                "notification workspaceFileChanged path={} sequence={} bytes={} symbols={} parse_diagnostics={} overlay_status={} overlay_generation={} overlay_files={} overlay_symbols={} elapsed_ms={}",
                                path.display(),
                                params.sequence,
                                bytes,
                                symbols,
                                parse_diagnostics,
                                status.status,
                                status.generation,
                                status.files,
                                status.symbols,
                                start.elapsed().as_millis()
                            ));
                        }
                        Ok(None) => self.log(&format!(
                            "notification workspaceFileChanged ignored path={} sequence={} bytes={} elapsed_ms={}",
                            path.display(),
                            params.sequence,
                            bytes,
                            start.elapsed().as_millis()
                        )),
                        Err(error) => {
                            self.log(&format!(
                                "notification workspaceFileChanged path={} sequence={} bytes={} error={} elapsed_ms={}",
                                path.display(),
                                params.sequence,
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
                    let removed = self
                        .external_index
                        .delete_workspace_file(&path, params.sequence);
                    let status = self.external_index.status_summary();
                    match removed {
                        Some(removed) => {
                            self.log(&format!(
                                "notification workspaceFileDeleted path={} sequence={} removed={} overlay_status={} overlay_generation={} overlay_files={} overlay_symbols={} elapsed_ms={}",
                                path.display(),
                                params.sequence,
                                removed,
                                status.status,
                                status.generation,
                                status.files,
                                status.symbols,
                                start.elapsed().as_millis()
                            ));
                        }
                        None => self.log(&format!(
                            "notification workspaceFileDeleted ignored path={} sequence={} elapsed_ms={}",
                            path.display(),
                            params.sequence,
                            start.elapsed().as_millis()
                        )),
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
                    let mut cached_projection = false;
                    let mut projection_ms = 0u128;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get_mut(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                cached_projection = document.document_symbols_ready();
                                if !cached_projection {
                                    let projection_start = Instant::now();
                                    let symbols = document_symbols_from_cached_analysis(
                                        &document.text,
                                        &document.analysis,
                                    );
                                    projection_ms = projection_start.elapsed().as_millis();
                                    document.set_document_symbols(symbols);
                                }
                                let symbols = document.document_symbols();
                                symbol_count = document_symbol_count(&symbols);
                                parse_diagnostics = document.analysis.parse_diagnostics;
                                symbols.to_vec()
                            })
                        })
                        .map(|symbols| serde_json::to_value(symbols).unwrap_or(Value::Null))
                        .unwrap_or(Value::Null);
                    self.log(&format!(
                        "request documentSymbol uri={} bytes={} revision={} cached_analysis=true document_symbols_cached={} document_symbol_ms={} symbols={} parse_diagnostics={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        cached_projection,
                        projection_ms,
                        symbol_count,
                        parse_diagnostics,
                        queue_ms,
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
                    let mut external_index_layers = "none";
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let indexes = self.external_index.snapshot();
                                external_index_status = indexes.status;
                                external_index_layers = indexes.available_layers();
                                let report =
                                    completion_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        &document.analysis,
                                        params.position,
                                        indexes.workspace.as_deref(),
                                        indexes.game_data.as_deref(),
                                    );
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
                        "request completion uri={} bytes={} revision={} cached_analysis=true context={} receiver={} owner_type={} prefix={} candidates={} failure_reason={} external_index_status={} external_index_layers={} parse_diagnostics={} context_ms={} lookup_ms={} render_ms={} queue_ms={} elapsed_ms={}",
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
                        external_index_layers,
                        parse_diagnostics,
                        context_ms,
                        lookup_ms,
                        render_ms,
                        queue_ms,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                }
            }
            "textDocument/signatureHelp" => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<HoverParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut revision = 0u64;
                    let mut parse_diagnostics = 0usize;
                    let mut context = "<none>".to_string();
                    let mut active_parameter = "<none>".to_string();
                    let mut candidate_count = 0usize;
                    let mut selected_label = "<none>".to_string();
                    let mut failure_reason = "<none>".to_string();
                    let mut context_ms = 0u128;
                    let mut lookup_ms = 0u128;
                    let mut render_ms = 0u128;
                    let mut external_index_status = self.external_index.status_summary().status;
                    let mut external_index_layers = "none";
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let indexes = self.external_index.snapshot();
                                external_index_status = indexes.status;
                                external_index_layers = indexes.available_layers();
                                let report =
                                    signature_help_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        &document.analysis,
                                        params.position,
                                        indexes.workspace.as_deref(),
                                        indexes.game_data.as_deref(),
                                    );
                                parse_diagnostics = report.parse_diagnostics;
                                context = report.context.unwrap_or_else(|| "<none>".to_string());
                                active_parameter = report
                                    .active_parameter
                                    .map(|index| index.to_string())
                                    .unwrap_or_else(|| "<none>".to_string());
                                candidate_count = report.candidate_count;
                                selected_label = report
                                    .selected_label
                                    .unwrap_or_else(|| "<none>".to_string());
                                failure_reason = report
                                    .failure_reason
                                    .unwrap_or_else(|| "<none>".to_string());
                                context_ms = report.timings.context_detection.as_millis();
                                lookup_ms = report.timings.candidate_lookup.as_millis();
                                render_ms = report.timings.item_rendering.as_millis();
                                report.help
                            })
                        })
                        .flatten()
                        .map(|help| serde_json::to_value(help).unwrap_or(Value::Null))
                        .unwrap_or(Value::Null);
                    self.log(&format!(
                        "request signatureHelp uri={} bytes={} revision={} cached_analysis=true context={} active_parameter={} candidates={} selected={} failure_reason={} external_index_status={} external_index_layers={} parse_diagnostics={} context_ms={} lookup_ms={} render_ms={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        context,
                        active_parameter,
                        candidate_count,
                        selected_label,
                        failure_reason,
                        external_index_status,
                        external_index_layers,
                        parse_diagnostics,
                        context_ms,
                        lookup_ms,
                        render_ms,
                        queue_ms,
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
                    let mut rich_work: Option<(String, u64, u64, Arc<AtomicBool>)> = None;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get_mut(&log_uri).map(|document| {
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
                                    if !document
                                        .semantic_tokens
                                        .pending_for_revision_and_external_generation(
                                            document.revision,
                                            external_generation,
                                        )
                                    {
                                        let cancel = document
                                            .semantic_tokens
                                            .mark_pending(document.revision, external_generation);
                                        rich_work = Some((
                                            log_uri.clone(),
                                            document.revision,
                                            external_generation,
                                            cancel,
                                        ));
                                    }
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
                        "request semanticTokens uri={} bytes={} revision={} cached_analysis=true mode={} tokens={} external_index_status={} external_generation={} parse_diagnostics={} lex_ms={} token_loop_ms={} resolver_ms={} resolver_calls={} encode_ms={} queue_ms={} elapsed_ms={}",
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
                        queue_ms,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                    if let Some((uri, rich_revision, rich_external_generation, cancel)) = rich_work
                    {
                        self.schedule_rich_semantic_tokens(
                            &uri,
                            rich_revision,
                            rich_external_generation,
                            cancel,
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
                    let mut external_index_layers = "none";
                    let mut revision = 0u64;
                    let mut hit = false;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let indexes = self.external_index.snapshot();
                                external_index_status = indexes.status;
                                external_index_layers = indexes.available_layers();
                                let report = hover_report_for_cached_analysis_with_external_indexes(
                                    &document.text,
                                    &document.analysis,
                                    &log_uri,
                                    params.position,
                                    indexes.workspace.as_deref(),
                                    indexes.game_data.as_deref(),
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
                        "request hover uri={} bytes={} revision={} cached_analysis=true hit={} selection_source={} selected_source={} resolver_reason={} identifier_context={} resolver_candidates={} receiver_owner={} receiver_failure={} external_index_status={} external_index_layers={} label={} kind={} parse_diagnostics={} queue_ms={} elapsed_ms={}",
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
                        external_index_layers,
                        selected_label,
                        selected_kind,
                        parse_diagnostics,
                        queue_ms,
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
                    let mut external_index_layers = "none";
                    let mut revision = 0u64;
                    let mut hit = false;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let indexes = self.external_index.snapshot();
                                external_index_status = indexes.status;
                                external_index_layers = indexes.available_layers();
                                let report =
                                    definition_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        &document.analysis,
                                        &log_uri,
                                        params.position,
                                        indexes.workspace.as_deref(),
                                        indexes.game_data.as_deref(),
                                    );
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
                        "request definition uri={} bytes={} revision={} cached_analysis=true hit={} selected_source={} resolver_reason={} identifier_context={} resolver_candidates={} external_index_status={} external_index_layers={} label={} kind={} parse_diagnostics={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        hit,
                        selected_source,
                        resolver_reason,
                        identifier_context,
                        resolver_candidate_count,
                        external_index_status,
                        external_index_layers,
                        selected_label,
                        selected_kind,
                        parse_diagnostics,
                        queue_ms,
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
                                let indexes = self.external_index.snapshot();
                                let report =
                                    debug_hover_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        &document.analysis,
                                        &log_uri,
                                        params.position,
                                        indexes.workspace.as_deref(),
                                        indexes.game_data.as_deref(),
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
            DEBUG_COMPLETION_METHOD => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<HoverParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut revision = 0u64;
                    let mut completion_context = "none".to_string();
                    let mut candidate_count = 0usize;
                    let mut signature_context = "none".to_string();
                    let mut signature_candidate_count = 0usize;
                    let mut external_index_status = self.external_index.status_summary().status;
                    let mut external_index_layers = "none";
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                let indexes = self.external_index.snapshot();
                                external_index_status = indexes.status;
                                external_index_layers = indexes.available_layers();
                                let report = completion_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        &document.analysis,
                                        params.position,
                                        indexes.workspace.as_deref(),
                                        indexes.game_data.as_deref(),
                                    );
                                let signature_report = signature_help_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        &document.analysis,
                                        params.position,
                                        indexes.workspace.as_deref(),
                                        indexes.game_data.as_deref(),
                                    );
                                completion_context = report.completion_context.clone();
                                candidate_count = report.candidate_count;
                                signature_context = signature_report
                                    .context
                                    .clone()
                                    .unwrap_or_else(|| "none".to_string());
                                signature_candidate_count = signature_report.candidate_count;
                                let mut markdown = completion_debug_markdown(
                                    &report,
                                    &log_uri,
                                    bytes,
                                    revision,
                                    external_index_status,
                                );
                                markdown.push_str(&signature_help_debug_markdown(&signature_report));
                                Value::String(markdown)
                            })
                        })
                        .unwrap_or_else(|| {
                            Value::String(format!(
                                "# Reforger Completion Debug\n\nNo open document text found for `{}`.",
                                log_uri
                            ))
                        });
                    self.log(&format!(
                        "request debugCompletion uri={} bytes={} revision={} cached_analysis=true context={} candidates={} signature_context={} signature_candidates={} external_index_status={} external_index_layers={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        completion_context,
                        candidate_count,
                        signature_context,
                        signature_candidate_count,
                        external_index_status,
                        external_index_layers,
                        queue_ms,
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

    fn handle_internal_event(&mut self, event: ServerEvent) -> Result<(), String> {
        match event {
            ServerEvent::Incoming { .. } => Ok(()),
            ServerEvent::RichSemanticTokensReady {
                uri,
                revision,
                external_generation,
                external_status,
                projection,
                elapsed_ms,
            } => {
                let token_count = projection.token_count;
                let parse_diagnostics = projection.parse_diagnostics;
                let lex_ms = projection.timings.lex_ms;
                let token_loop_ms = projection.timings.token_loop_ms;
                let resolver_ms = projection.timings.resolver_ms;
                let resolver_calls = projection.timings.identifier_resolver_calls;
                let encode_ms = projection.timings.encode_ms;
                let Some(current_revision) =
                    self.documents.get(&uri).map(|document| document.revision)
                else {
                    self.log(&format!(
                        "semanticTokensRich discarded uri={} revision={} reason=missing-document elapsed_ms={}",
                        uri,
                        revision,
                        elapsed_ms
                    ));
                    return Ok(());
                };
                if current_revision != revision {
                    self.log(&format!(
                        "semanticTokensRich discarded uri={} revision={} current_revision={} reason=stale-revision elapsed_ms={}",
                        uri,
                        revision,
                        current_revision,
                        elapsed_ms
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
                        elapsed_ms
                    ));
                    return Ok(());
                }
                if let Some(document) = self.documents.get_mut(&uri) {
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
                    external_status,
                    parse_diagnostics,
                    lex_ms,
                    token_loop_ms,
                    resolver_ms,
                    resolver_calls,
                    encode_ms,
                    elapsed_ms
                ));
                self.request_semantic_tokens_refresh()
            }
            ServerEvent::RichSemanticTokensSkipped {
                uri,
                revision,
                external_generation,
                reason,
                elapsed_ms,
            } => {
                if let Some(document) = self.documents.get_mut(&uri) {
                    document
                        .semantic_tokens
                        .cancel_pending_if_matches(revision, external_generation);
                }
                self.log(&format!(
                    "semanticTokensRich skipped uri={} revision={} external_generation={} reason={} elapsed_ms={}",
                    uri, revision, external_generation, reason, elapsed_ms
                ));
                Ok(())
            }
            ServerEvent::DocumentAnalysisReady {
                uri,
                revision,
                analysis,
                timings,
                elapsed_ms,
            } => self.install_document_analysis(&uri, revision, analysis, timings, elapsed_ms),
            ServerEvent::DocumentAnalysisSkipped {
                uri,
                revision,
                reason,
                elapsed_ms,
            } => {
                self.log(&format!(
                    "documentAnalysis skipped uri={} revision={} reason={} elapsed_ms={}",
                    uri, revision, reason, elapsed_ms
                ));
                Ok(())
            }
        }
    }

    fn defer_request_while_document_analysis_is_pending(
        &mut self,
        message: &RpcMessage,
        value: Value,
    ) -> Result<bool, String> {
        let Some(uri) = request_document_uri(message.params.as_ref()) else {
            return Ok(false);
        };
        let Some(document) = self.documents.get(&uri) else {
            return Ok(false);
        };
        if document.analysis_ready() {
            return Ok(false);
        }
        let revision = document.revision;
        let pending = self
            .deferred_document_requests
            .entry(uri.clone())
            .or_default();
        if pending.len() >= MAX_PENDING_DOCUMENT_REQUESTS_PER_URI {
            if let Some(id) = message.id.clone() {
                self.respond_error(id, -32801, "Content modified")?;
            }
            self.log(&format!(
                "request deferred rejected uri={} revision={} reason=capacity",
                uri, revision
            ));
            return Ok(true);
        }
        pending.push(DeferredDocumentRequest {
            revision,
            received_at: Instant::now(),
            value,
        });
        let pending_count = pending.len();
        self.log(&format!(
            "request deferred uri={} revision={} pending_requests={}",
            uri, revision, pending_count
        ));
        Ok(true)
    }

    fn discard_deferred_document_requests(
        &mut self,
        uri: &str,
        current_revision: u64,
    ) -> Result<(), String> {
        let Some(pending) = self.deferred_document_requests.remove(uri) else {
            return Ok(());
        };
        for request in pending {
            let message: RpcMessage = serde_json::from_value(request.value)
                .map_err(|error| format!("Invalid deferred JSON-RPC message: {error}"))?;
            if let Some(id) = message.id {
                self.respond_error(id, -32801, "Content modified")?;
            }
        }
        self.log(&format!(
            "request deferred discarded uri={} current_revision={} reason=superseded",
            uri, current_revision
        ));
        Ok(())
    }

    fn install_document_analysis(
        &mut self,
        uri: &str,
        revision: u64,
        analysis: FileIndexAnalysis,
        timings: FileIndexAnalysisTimings,
        elapsed_ms: u128,
    ) -> Result<(), String> {
        let Some(document) = self.documents.get_mut(uri) else {
            self.log(&format!(
                "documentAnalysis discarded uri={} revision={} reason=missing-document elapsed_ms={}",
                uri, revision, elapsed_ms
            ));
            return Ok(());
        };
        if !document.install_analysis(revision, analysis, timings) {
            let current_revision = document.revision;
            let _ = document;
            self.log(&format!(
                "documentAnalysis discarded uri={} revision={} current_revision={} reason=stale elapsed_ms={}",
                uri, revision, current_revision, elapsed_ms
            ));
            return Ok(());
        }
        let version = document.version;
        let bytes = document.text.len();
        let parse_diagnostics = document.analysis.parse_diagnostics;
        let analysis_timings = document.analysis_timings;
        let diagnostics_message = publish_diagnostics_message(
            uri,
            version,
            &document.text,
            &document.analysis.diagnostics,
        );
        let _ = document;
        self.write_message(diagnostics_message)?;
        self.log(&format!(
            "documentAnalysis ready uri={} bytes={} version={} revision={} cached_analysis=true parse_diagnostics={} analysis_parse_ms={} analysis_catalog_ms={} analysis_index_ms={} analysis_scope_ms={} analysis_build_ms={} elapsed_ms={}",
            uri,
            bytes,
            version,
            revision,
            parse_diagnostics,
            analysis_timings.parse_ms,
            analysis_timings.catalog_ms,
            analysis_timings.index_ms,
            analysis_timings.scope_ms,
            analysis_timings.total_ms,
            elapsed_ms
        ));
        let pending = self
            .deferred_document_requests
            .remove(uri)
            .unwrap_or_default();
        for request in pending {
            if request.revision == revision {
                self.handle_message(
                    request.value,
                    Some(request.received_at.elapsed().as_millis()),
                    0,
                    0,
                )?;
            } else {
                let message: RpcMessage = serde_json::from_value(request.value)
                    .map_err(|error| format!("Invalid deferred JSON-RPC message: {error}"))?;
                if let Some(id) = message.id {
                    self.respond_error(id, -32801, "Content modified")?;
                }
            }
        }
        Ok(())
    }

    fn schedule_rich_semantic_tokens(
        &mut self,
        uri: &str,
        revision: u64,
        external_generation: u64,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), String> {
        if let Some(scheduler) = self.rich_scheduler.clone() {
            let start = Instant::now();
            let Some(document) = self.documents.get(uri) else {
                self.log(&format!(
                    "semanticTokensRich skipped uri={} revision={} reason=missing-document-before-schedule elapsed_ms={}",
                    uri,
                    revision,
                    start.elapsed().as_millis()
                ));
                return Ok(());
            };
            if document.revision != revision {
                self.log(&format!(
                    "semanticTokensRich skipped uri={} revision={} reason=stale-revision-before-schedule elapsed_ms={}",
                    uri,
                    revision,
                    start.elapsed().as_millis()
                ));
                return Ok(());
            }
            let job = RichSemanticTokensJob {
                uri: uri.to_string(),
                revision,
                external_generation,
                cancel,
                scheduled_at: start,
                source: document.text.clone(),
                analysis: document.analysis.clone(),
                external_snapshot: self.external_index.snapshot(),
            };
            scheduler.schedule(job);
            return Ok(());
        }

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
        let indexes = self.external_index.snapshot();
        *external_index_status = indexes.status;
        Some(semantic_tokens_for_cached_analysis_with_external_indexes(
            &document.text,
            &document.analysis,
            indexes.workspace.as_deref(),
            indexes.game_data.as_deref(),
        ))
    }

    fn request_semantic_tokens_refresh(&mut self) -> Result<(), String> {
        if self.semantic_tokens_refresh_in_flight.is_some() {
            self.semantic_tokens_refresh_dirty = true;
            return Ok(());
        }
        let request_id = self.next_server_request_id;
        self.next_server_request_id += 1;
        let id = format!("server-{request_id}");
        self.semantic_tokens_refresh_in_flight = Some(id.clone());
        self.log(&format!(
            "request workspace/semanticTokens/refresh id=server-{request_id}"
        ));
        self.write_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "workspace/semanticTokens/refresh",
            "params": null
        }))
    }

    fn handle_semantic_tokens_refresh_response(
        &mut self,
        message: &RpcMessage,
    ) -> Result<(), String> {
        let Some(id) = message.id.as_ref().and_then(Value::as_str) else {
            return Ok(());
        };
        if self.semantic_tokens_refresh_in_flight.as_deref() != Some(id) {
            return Ok(());
        }
        self.semantic_tokens_refresh_in_flight = None;
        if self.semantic_tokens_refresh_dirty {
            self.semantic_tokens_refresh_dirty = false;
            self.request_semantic_tokens_refresh()?;
        }
        Ok(())
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
        for document in self.documents.values_mut() {
            document
                .semantic_tokens
                .cancel_pending_for_other_external_generation(status.generation);
        }
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
    let positions = LspPositionIndex::new(source);
    LspDocumentSymbolReport {
        symbols: document_symbols_from_index(&positions, &analysis.index, &query),
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
    positions: &LspPositionIndex,
    index: &SymbolIndex,
    query: &IndexQuery<'_>,
) -> Vec<LspDocumentSymbol> {
    index
        .symbols()
        .iter()
        .filter(|symbol| symbol.parent.is_none())
        .filter(|symbol| !is_document_symbol_excluded_kind(symbol.kind))
        .filter_map(|symbol| document_symbol_for_id(positions, index, query, symbol.id))
        .collect()
}

fn document_symbol_for_id(
    positions: &LspPositionIndex,
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
        .filter_map(|child| document_symbol_for_id(positions, index, query, *child))
        .collect::<Vec<_>>();

    Some(LspDocumentSymbol {
        name: display.label,
        detail: display.detail.or(display.signature),
        kind: document_symbol_kind(symbol.kind),
        range: positions.range_for_span(symbol.span),
        selection_range: positions.range_for_span(symbol.selection_span),
        children,
    })
}

#[derive(Debug)]
pub(crate) struct LspPositionIndex {
    positions: Vec<LspPosition>,
}

#[cfg(test)]
thread_local! {
    static POSITION_INDEX_BUILD_COUNT: Cell<usize> = const { Cell::new(0) };
}

impl LspPositionIndex {
    pub(crate) fn new(source: &str) -> Self {
        Self::new_cancellable(source, None).expect("unconditional position index build")
    }

    pub(crate) fn new_cancellable(
        source: &str,
        should_cancel: Option<&dyn Fn() -> bool>,
    ) -> Option<Self> {
        #[cfg(test)]
        POSITION_INDEX_BUILD_COUNT.with(|count| count.set(count.get() + 1));
        let origin = LspPosition {
            line: 0,
            character: 0,
        };
        let mut positions = vec![origin; source.len().saturating_add(1)];
        let mut position = origin;
        for (character_index, (offset, character)) in source.char_indices().enumerate() {
            if character_index % 64 == 0
                && should_cancel.is_some_and(|should_cancel| should_cancel())
            {
                return None;
            }
            let next_offset = offset.saturating_add(character.len_utf8());
            for entry in &mut positions[offset..next_offset] {
                *entry = position;
            }
            match character {
                '\r' => {
                    position.line = position.line.saturating_add(1);
                    position.character = 0;
                }
                '\n' if offset == 0 || source.as_bytes()[offset - 1] != b'\r' => {
                    position.line = position.line.saturating_add(1);
                    position.character = 0;
                }
                '\n' => {}
                _ => {
                    position.character = position
                        .character
                        .saturating_add(character.len_utf16() as u32)
                }
            }
        }
        if let Some(end) = positions.last_mut() {
            *end = position;
        }
        Some(Self { positions })
    }

    pub(crate) fn position_for_offset(&self, offset: usize) -> LspPosition {
        self.positions
            .get(offset.min(self.positions.len().saturating_sub(1)))
            .copied()
            .unwrap_or(LspPosition {
                line: 0,
                character: 0,
            })
    }

    pub(crate) fn range_for_span(&self, span: crate::lexer::TextSpan) -> LspRange {
        LspRange {
            start: self.position_for_offset(span.start),
            end: self.position_for_offset(span.end),
        }
    }
}

pub(crate) fn range_for_span(source: &str, span: crate::lexer::TextSpan) -> LspRange {
    LspPositionIndex::new(source).range_for_span(span)
}

pub fn position_for_offset(source: &str, offset: usize) -> LspPosition {
    LspPositionIndex::new(source).position_for_offset(offset)
}

pub fn offset_for_position(source: &str, position: LspPosition) -> Option<usize> {
    let mut line = 0u32;
    let mut character = 0u32;

    for (index, value) in source.char_indices() {
        if value == '\n' && index > 0 && source.as_bytes()[index - 1] == b'\r' {
            continue;
        }
        if line == position.line {
            if character == position.character {
                return Some(index);
            }
            if value == '\r' || value == '\n' {
                return None;
            }
            let next_character = character + value.len_utf16() as u32;
            if position.character < next_character {
                return Some(index);
            }
            character = next_character;
        } else if value == '\r' {
            line += 1;
            character = 0;
        } else if value == '\n' && (index == 0 || source.as_bytes()[index - 1] != b'\r') {
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

fn validate_message_params(method: &str, params: &Option<Value>) -> Result<(), String> {
    match method {
        "textDocument/didOpen" => validate_params::<DidOpenTextDocumentParams>(params, method),
        "textDocument/didChange" => validate_params::<DidChangeTextDocumentParams>(params, method),
        "textDocument/didClose" => validate_params::<DidCloseTextDocumentParams>(params, method),
        "reforger/workspaceFileChanged" => {
            validate_params::<WorkspaceFileChangedParams>(params, method)
        }
        "reforger/workspaceFileDeleted" => {
            validate_params::<WorkspaceFileDeletedParams>(params, method)
        }
        "textDocument/documentSymbol" | "textDocument/semanticTokens/full" => {
            validate_params::<DocumentSymbolParams>(params, method)
        }
        "textDocument/hover"
        | "textDocument/definition"
        | "textDocument/completion"
        | "textDocument/signatureHelp"
        | DEBUG_HOVER_METHOD
        | DEBUG_COMPLETION_METHOD => validate_params::<HoverParams>(params, method),
        _ => Ok(()),
    }
}

fn validate_params<T: for<'de> Deserialize<'de>>(
    params: &Option<Value>,
    method: &str,
) -> Result<(), String> {
    let Some(params) = params else {
        return Err(format!("Invalid params for {method}: missing params"));
    };
    serde_json::from_value::<T>(params.clone())
        .map(|_| ())
        .map_err(|error| format!("Invalid params for {method}: {error}"))
}

fn read_message<R: BufRead>(reader: &mut R) -> Result<Option<Value>, String> {
    let mut content_length = None;
    let mut header_bytes = 0;
    loop {
        let Some(line) = read_lsp_header_line(reader)? else {
            return Ok(None);
        };
        header_bytes += line.len();
        if header_bytes > MAX_LSP_HEADER_BYTES {
            return Err("LSP headers exceed the configured limit".to_string());
        }
        let line = std::str::from_utf8(&line)
            .map_err(|error| format!("Invalid LSP header encoding: {error}"))?;
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
    if content_length > MAX_LSP_BODY_BYTES {
        return Err("LSP body exceeds the configured limit".to_string());
    }
    let mut body = vec![0u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("Failed to read LSP body: {error}"))?;
    serde_json::from_slice(&body).map_err(|error| format!("Invalid LSP JSON body: {error}"))
}

fn read_lsp_header_line<R: BufRead>(reader: &mut R) -> Result<Option<Vec<u8>>, String> {
    let mut line = Vec::with_capacity(128);
    loop {
        let (bytes_to_consume, line_complete) = {
            let available = reader
                .fill_buf()
                .map_err(|error| format!("Failed to read LSP header: {error}"))?;
            if available.is_empty() {
                if line.is_empty() {
                    return Ok(None);
                }
                return Err("LSP header ended before a line terminator".to_string());
            }
            let remaining = MAX_LSP_HEADER_LINE_BYTES.saturating_sub(line.len());
            if remaining == 0 {
                return Err("LSP header line exceeds the configured limit".to_string());
            }
            let available_len = available.len().min(remaining);
            let newline_index = available[..available_len]
                .iter()
                .position(|byte| *byte == b'\n');
            let bytes_to_consume = newline_index.map_or(available_len, |index| index + 1);
            line.extend_from_slice(&available[..bytes_to_consume]);
            (bytes_to_consume, newline_index.is_some())
        };
        reader.consume(bytes_to_consume);
        if line_complete {
            return Ok(Some(line));
        }
        if line.len() == MAX_LSP_HEADER_LINE_BYTES {
            return Err("LSP header line exceeds the configured limit".to_string());
        }
    }
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

    fn rich_semantic_tokens_job(uri: &str, scheduled_at: Instant) -> RichSemanticTokensJob {
        RichSemanticTokensJob {
            uri: uri.to_string(),
            revision: 1,
            external_generation: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            scheduled_at,
            source: String::new(),
            analysis: file_index_for_source(""),
            external_snapshot: ExternalIndexSnapshot {
                status: "missing",
                workspace: None,
                game_data: None,
            },
        }
    }

    #[test]
    fn rich_scheduler_evicts_the_oldest_pending_job() {
        let now = Instant::now();
        let mut pending = BTreeMap::new();
        for (uri, scheduled_at) in [
            ("file:///zeta.c", now - Duration::from_secs(1)),
            ("file:///alpha.c", now - Duration::from_secs(3)),
            ("file:///middle.c", now - Duration::from_secs(2)),
        ] {
            pending.insert(uri.to_string(), rich_semantic_tokens_job(uri, scheduled_at));
        }

        assert_eq!(
            oldest_pending_uri(&pending).as_deref(),
            Some("file:///alpha.c")
        );
    }

    #[test]
    fn rich_scheduler_selects_the_earliest_due_job_not_the_first_uri() {
        let now = Instant::now();
        let mut pending = BTreeMap::new();
        pending.insert(
            "file:///a.c".to_string(),
            rich_semantic_tokens_job("file:///a.c", now),
        );
        pending.insert(
            "file:///z.c".to_string(),
            rich_semantic_tokens_job(
                "file:///z.c",
                now - Duration::from_millis(RICH_SEMANTIC_TOKENS_IDLE_DELAY_MS + 1),
            ),
        );

        assert_eq!(
            earliest_due_pending_uri(&pending).as_deref(),
            Some("file:///z.c")
        );
    }

    #[test]
    fn semantic_token_refresh_coalesces_until_the_client_acknowledges_it() {
        let mut server = LspServer::new(Vec::new(), LspServerOptions::default());

        server.request_semantic_tokens_refresh().unwrap();
        server.request_semantic_tokens_refresh().unwrap();
        assert_eq!(
            server.semantic_tokens_refresh_in_flight.as_deref(),
            Some("server-1")
        );
        assert!(server.semantic_tokens_refresh_dirty);
        assert_eq!(
            String::from_utf8_lossy(&server.writer)
                .matches("workspace/semanticTokens/refresh")
                .count(),
            1
        );

        server
            .handle_message(
                json!({ "jsonrpc": "2.0", "id": "server-1", "result": null }),
                None,
                0,
                0,
            )
            .unwrap();

        assert_eq!(
            server.semantic_tokens_refresh_in_flight.as_deref(),
            Some("server-2")
        );
        assert!(!server.semantic_tokens_refresh_dirty);
        assert_eq!(
            String::from_utf8_lossy(&server.writer)
                .matches("workspace/semanticTokens/refresh")
                .count(),
            2
        );
    }

    #[test]
    fn duplicate_did_open_rejects_old_rich_semantic_tokens() {
        let uri = "file:///Scripts/Reopened.c";
        let mut server = LspServer::new(Vec::new(), LspServerOptions::default());
        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": uri,
                            "languageId": "enforce",
                            "version": 1,
                            "text": "class Old {}"
                        }
                    }
                }),
                None,
                0,
                0,
            )
            .unwrap();

        let external_generation = server.external_index.status_summary().generation;
        let (old_revision, projection, cancel) = {
            let document = server.documents.get_mut(uri).unwrap();
            let cancel = document
                .semantic_tokens
                .mark_pending(document.revision, external_generation);
            (
                document.revision,
                fast_semantic_tokens_for_cached_analysis(&document.text, &document.analysis),
                cancel,
            )
        };

        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": uri,
                            "languageId": "enforce",
                            "version": 2,
                            "text": "class New {}"
                        }
                    }
                }),
                None,
                0,
                0,
            )
            .unwrap();

        let current_revision = server.documents[uri].revision;
        assert!(cancel.load(Ordering::Relaxed));
        assert_ne!(old_revision, current_revision);

        server
            .handle_internal_event(ServerEvent::RichSemanticTokensReady {
                uri: uri.to_string(),
                revision: old_revision,
                external_generation,
                external_status: "missing",
                projection,
                elapsed_ms: 0,
            })
            .unwrap();

        assert!(server.documents[uri]
            .semantic_tokens
            .rich_for_revision_and_external_generation(current_revision, external_generation)
            .is_none());
    }

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
	void Example(int initialValue)
	{
	}
	void ~Example()
	{
	}
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

        let report = fast_semantic_tokens_report_for_source(source);

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
        assert!(report
            .decoded
            .iter()
            .any(|token| token.text == "Example" && token.token_type == "class"));
        assert!(
            !report
                .decoded
                .iter()
                .any(|token| token.text == "Example" && token.token_type == "method"),
            "{:?}",
            report.decoded
        );
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
                >= 3
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
            .any(|token| token.text == "COUNT" && token.token_type == "field"));
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

        let report = fast_semantic_tokens_report_for_source(source);

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
        assert_semantic_token(&report, "WB_GAME_MODE_CATEGORY", "field", Some("#cfcfcf"));
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
    fn semantic_tokens_keep_attribute_shape_after_invalid_previous_line() {
        let source = r#"class Example
{
	this

	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void RpcDo();
}
"#;

        let report = semantic_tokens_report_for_source(source);

        assert_semantic_token(&report, "RplRpc", "class", Some("#40b5ac"));
        assert!(
            !report
                .decoded
                .iter()
                .any(|token| token.text == "RplRpc" && token.token_type == "function"),
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
		int testnnn = GRAY_TEST2.testnum;
		stateComponent.GetDuration();
	}
}
"#;

        let report = semantic_tokens_report_for_source(source);

        assert_semantic_token(&report, "SCR_EGameModeState", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "GAME", "enumMember", Some("#cfcfcf"));
        assert_semantic_token(&report, "EHealthState", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "INJURED", "enumMember", Some("#cfcfcf"));
        assert_semantic_token(&report, "GRAY_TEST2", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "testnum", "enumMember", Some("#cfcfcf"));
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
    fn semantic_tokens_color_scope_references_before_rich_resolution() {
        let source = r#"class OwnerType
{
	void Run()
	{
	}
}

class Example
{
	OwnerType GetOwner();
	void Run(OwnerType owner)
	{
		OwnerType localOwner = GetOwner();
		if (owner == GetOwner())
			return owner;
		if (localOwner == owner)
		{
			OwnerType owner = localOwner;
			owner.Run();
		}
		int testnnn = GRAY_TEST2.testnum;
	}
}
"#;

        let report = fast_semantic_tokens_report_for_source(source);

        assert_semantic_token(&report, "owner", "parameter", Some("#cfcfcf"));
        assert_semantic_token(&report, "localOwner", "variable", Some("#cfcfcf"));
        assert_semantic_token_count_at_least(&report, "owner", "variable", 2);
        assert_semantic_token(&report, "GRAY_TEST2", "class", Some("#40b5ac"));
        assert_semantic_token(&report, "testnum", "enumMember", Some("#cfcfcf"));
    }

    #[test]
    fn semantic_tokens_keep_comment_contents_comment_colored() {
        let source = r#"class Example
{
	//! \param[in] enable{} Set() true to enable supplies, set false to disable
	/*!
		\return[] // True{} <> if the game is hosted by a player (i.e., not dedicated server)
	*/
	int testnnn = 1; /* testnnn {} Set() */
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
        assert!(
            report.decoded.iter().any(|token| {
                token.token_type == "comment"
                    && token.text == "/* testnnn {} Set() */"
                    && token.range.start.line == 6
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
    fn hover_uses_cursor_token_range_for_file_local_identifier() {
        let source = r#"class Example
{
	void Run()
	{
		string label = "é"; int localValue = 0; localValue = 1;
	}
}
"#;
        let position = position_for_needle(source, "localValue = 1", "localValue");

        let report = hover_report_for_source_position(source, position);
        let hover = report.hover.expect("local identifier should have hover");

        assert_eq!(position.character, 42, "position uses UTF-16 code units");
        assert_eq!(
            hover.range,
            Some(LspRange {
                start: position,
                end: LspPosition {
                    line: position.line,
                    character: position.character + 10,
                },
            })
        );
    }

    #[test]
    fn hover_uses_cursor_token_range_for_crlf_source() {
        let source = "class Example\r\n{\r\n\tvoid Run()\r\n\t{\r\n\t\tstring label = \"é\"; int localValue = 0; localValue = 1;\r\n\t}\r\n}\r\n";
        let position = position_for_needle(source, "localValue = 1", "localValue");

        let report = hover_report_for_source_position(source, position);
        let hover = report.hover.expect("local identifier should have hover");

        assert_eq!(position.line, 4, "CRLF advances one LSP line per break");
        assert_eq!(position.character, 42, "position uses UTF-16 code units");
        assert_eq!(
            hover.range,
            Some(LspRange {
                start: position,
                end: LspPosition {
                    line: position.line,
                    character: position.character + 10,
                },
            })
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
        let position = position_for_needle(source, "Widget widget", "Widget");
        let report =
            hover_report_for_source_position_with_external(source, position, Some(&external));

        assert!(report.is_hit());
        assert_eq!(report.selected_kind, Some(SymbolKind::Class));
        assert_eq!(report.selected_label.as_deref(), Some("Widget"));
        assert_eq!(report.selected_source, Some(CandidateSource::External));
        assert_eq!(
            report.hover.as_ref().and_then(|hover| hover.range),
            Some(LspRange {
                start: position,
                end: LspPosition {
                    line: position.line,
                    character: position.character + 6,
                },
            })
        );
        assert_eq!(
            report.identifier_context,
            Some(IdentifierContext::TypePosition)
        );
    }

    #[test]
    fn hover_type_usage_renders_same_class_display_as_class_declaration() {
        let source = r#"class Example
{
	void Run()
	{
		SCR_BaseGameModeStateComponent stateComponent;
	}
}
"#;
        let external = file_index_for_source(
            r#"//! Base component for handling game mode states.
class SCR_BaseGameModeStateComponent : SCR_BaseGameModeComponent
{
	bool GetAllowControls();
	float GetDuration();
}

class SCR_BaseGameModeComponent
{
	void InheritedRun();
}
"#,
        )
        .index;

        let report = hover_report_for_source_position_with_external(
            source,
            position_for_needle(
                source,
                "SCR_BaseGameModeStateComponent stateComponent",
                "SCR_BaseGameModeStateComponent",
            ),
            Some(&external),
        );
        let markdown = report.hover.as_ref().unwrap().contents.value.as_str();

        assert_eq!(report.selected_kind, Some(SymbolKind::Class));
        assert_eq!(
            report.selected_label.as_deref(),
            Some("SCR_BaseGameModeStateComponent")
        );
        assert_eq!(report.selected_source, Some(CandidateSource::External));
        assert_eq!(
            report.identifier_context,
            Some(IdentifierContext::TypePosition)
        );
        assert!(markdown.contains("<span style=\"color:#59A6E9;\">Class</span>"));
        assert!(markdown.contains(
            "data-code=\"class SCR_BaseGameModeStateComponent : SCR_BaseGameModeComponent\""
        ));
        assert!(markdown.contains("Base component for handling game mode states."));
        assert!(markdown.contains("### Functions"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">GetAllowControls</span>"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">GetDuration</span>"));
        assert!(!markdown.contains("### Inherited members"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">InheritedRun</span>"));
        assert!(!markdown.contains("inherited from"));
    }

    #[test]
    fn hover_class_declaration_uses_external_overlay_for_inherited_member_summary() {
        let source = r#"//! Base component for handling game mode states.
class SCR_BaseGameModeStateComponent : SCR_BaseGameModeComponent
{
	bool GetAllowControls();
	float GetDuration();
}
"#;
        let external = file_index_for_source(
            r#"//! Base component for handling game mode states.
class SCR_BaseGameModeStateComponent : SCR_BaseGameModeComponent
{
	bool GetAllowControls();
	float GetDuration();
}

class SCR_BaseGameModeComponent
{
	void InheritedRun();
}
"#,
        )
        .index;

        let report = hover_report_for_source_position_with_external(
            source,
            position_for_needle(
                source,
                "SCR_BaseGameModeStateComponent",
                "SCR_BaseGameModeStateComponent",
            ),
            Some(&external),
        );
        let markdown = report.hover.as_ref().unwrap().contents.value.as_str();

        assert_eq!(report.selected_source, Some(CandidateSource::FileLocal));
        assert_eq!(
            report.selected_label.as_deref(),
            Some("SCR_BaseGameModeStateComponent")
        );
        assert!(markdown.contains(
            "data-code=\"class SCR_BaseGameModeStateComponent : SCR_BaseGameModeComponent\""
        ));
        assert!(markdown.contains("### Functions"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">GetAllowControls</span>"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">GetDuration</span>"));
        assert!(!markdown.contains("### Inherited members"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">InheritedRun</span>"));
        assert!(!markdown.contains("inherited from"));
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
                && item.text_edit.new_text == "SetVisible(${1:visible})"
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
    fn completion_hides_restricted_members_for_external_receivers() {
        let source = r#"class GRAY_TEST2
{
	protected void proTestnum();
	private void proPrivate();
	void proPublic();
}

class Other
{
	void Run()
	{
		GRAY_TEST2 test33;
		test33.pro
	}
}
"#;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "test33.pro"),
            None,
        );

        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert!(labels.contains(&"proPublic"));
        assert!(!labels.contains(&"proTestnum"));
        assert!(!labels.contains(&"proPrivate"));
    }

    #[test]
    fn completion_keeps_restricted_members_for_self_receivers() {
        let source = r#"class GRAY_TEST2
{
	protected void proTestnum();
	private void proPrivate();
	void Run()
	{
		this.pro
	}
}
"#;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "this.pro"),
            None,
        );

        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert!(labels.contains(&"proTestnum"));
        assert!(labels.contains(&"proPrivate"));
    }

    #[test]
    fn completion_labels_overloads_and_uses_source_rank_as_tiebreaker() {
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
        assert_eq!(first.label, "SetText");
        assert!(first
            .sort_text
            .as_deref()
            .unwrap_or("")
            .starts_with("104:01:"));
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
            vec![("SCR_Mode", 13), ("SCR_Alias", 25), ("SCR_Widget", 7)]
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
    fn completion_returns_attribute_classes_in_attribute_name_position() {
        let source = r#"class Example
{
	[Attribu]
	int m_Value;
}
"#;
        let external = file_index_for_source(
            r#"class Attribute
{
	void Attribute(string defvalue = "");
}
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "Attribu"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "Attribu");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "Attribute"
                && item.kind == 7
                && item.text_edit.new_text == "Attribute($0)"
                && item.insert_text_format == Some(2)
                && item.optional_parameter_count == 1
                && item
                    .command
                    .as_ref()
                    .map(|command| command.command.as_str())
                    == Some("editor.action.triggerParameterHints")));
    }

    #[test]
    fn completion_wraps_attribute_shorthand_at_declaration_boundary() {
        let source = r#"class Example
{
	attribut
	int m_Value;
}
"#;
        let external = file_index_for_source(
            r#"class UniqueAttribute {}
class Attribute : UniqueAttribute
{
	void Attribute(string defvalue = "");
}
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "attribut"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "type");
        assert_eq!(report.prefix, "attribut");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "Attribute"
                && item.kind == 7
                && item.text_edit.new_text == "[Attribute($0)]"
                && item.insert_text_format == Some(2)
                && item.optional_parameter_count == 1));
    }

    #[test]
    fn completion_wraps_indirect_unique_attribute_shorthand() {
        let source = r#"class Example
{
	custom
	int m_Value;
}
"#;
        let external = file_index_for_source(
            r#"class UniqueAttribute {}
class SharedAttributeBase : UniqueAttribute {}
class CustomFlag : SharedAttributeBase
{
	void CustomFlag(string value = "");
}
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "custom"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "type");
        assert_eq!(report.prefix, "custom");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "CustomFlag"
                && item.kind == 7
                && item.text_edit.new_text == "[CustomFlag($0)]"
                && item.insert_text_format == Some(2)
                && item.optional_parameter_count == 1));
    }

    #[test]
    fn completion_returns_optional_parameter_labels_inside_attribute_args() {
        let source = r#"class Example
{
	[Attribute(defv)]
	int m_Value;
}
"#;
        let external = file_index_for_source(
            r#"class UniqueAttribute {}
class Attribute : UniqueAttribute
{
	void Attribute(string defvalue = "", string uiwidget = "auto", string desc = "");
}
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "defv"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "argument-label");
        assert_eq!(report.prefix, "defv");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "defvalue")
            .expect("expected defvalue parameter-label completion");
        assert_eq!(item.kind, 10);
        assert_eq!(item.text_edit.new_text, "defvalue");
        assert_eq!(item.insert_text_format, Some(2));
        assert_eq!(item.optional_parameter_count, 1);
        assert_eq!(
            item.command
                .as_ref()
                .map(|command| command.command.as_str()),
            Some("editor.action.triggerParameterHints")
        );
    }

    #[test]
    fn completion_returns_parameter_labels_inside_function_calls() {
        let source = r#"void SendToEveryone(ENotification notificationID, int param1 = 0, string label = "ok");

class Example
{
	void Run()
	{
		SendToEveryone(notif)
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "SendToEveryone(notif"),
            None,
        );

        assert_eq!(report.completion_context, "argument-label");
        assert_eq!(report.prefix, "notif");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "notificationID")
            .expect("expected function parameter-label completion");
        assert_eq!(item.text_edit.new_text, "notificationID");
        assert_eq!(item.required_parameter_count, 1);
        assert_eq!(
            item.command
                .as_ref()
                .map(|command| command.command.as_str()),
            Some("editor.action.triggerParameterHints")
        );
    }

    #[test]
    fn completion_prefers_positional_value_when_prefix_matches_active_parameter_name() {
        let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(int input, float num)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(input)
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "TestNumFun2(input"),
            None,
        );

        assert_eq!(report.completion_context, "argument-label");
        assert_eq!(report.prefix, "input");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "input")
            .expect("expected positional input value completion");
        assert_eq!(item.text_edit.new_text, "input");
        assert!(!report
            .list
            .items
            .iter()
            .any(|item| item.text_edit.new_text == "input: $0"));
    }

    #[test]
    fn completion_does_not_offer_active_parameter_label_for_positional_slot_prefix() {
        let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(int input, float num)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(inp)
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "TestNumFun2(inp"),
            None,
        );

        assert_eq!(report.completion_context, "argument-label");
        assert_eq!(report.prefix, "inp");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "input" && item.text_edit.new_text == "input"));
        assert!(!report
            .list
            .items
            .iter()
            .any(|item| item.text_edit.new_text == "input: $0"));
    }

    #[test]
    fn completion_keeps_value_candidates_after_parameter_labels() {
        let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(int input, float num, string testValue)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(input, tes)
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "TestNumFun2(input, tes"),
            None,
        );

        assert_eq!(report.completion_context, "argument-label");
        assert_eq!(report.prefix, "tes");
        let first = report
            .list
            .items
            .first()
            .expect("expected parameter label completion");
        assert_eq!(first.label, "test");
        assert_eq!(first.text_edit.new_text, "test: $0");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "testValue" && item.text_edit.new_text == "testValue"));
    }

    #[test]
    fn completion_uses_active_parameter_when_no_matching_value_exists() {
        let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(float num)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(inpu, num,)
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "TestNumFun2(inpu"),
            None,
        );

        assert_eq!(report.completion_context, "argument-label");
        assert_eq!(report.prefix, "inpu");
        let first = report
            .list
            .items
            .first()
            .expect("expected active parameter completion");
        assert_eq!(first.label, "input");
        assert_eq!(first.text_edit.new_text, "input");
    }

    #[test]
    fn completion_parameter_labels_default_enum_arguments_to_enum_owner() {
        let source = r#"enum ENotification
{
	PLAYER_JOINED
}

void SendToEveryone(ENotification notificationID, int param1 = 0);

class Example
{
	void Run()
	{
		SendToEveryone(notif)
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "SendToEveryone(notif"),
            None,
        );

        assert_eq!(report.completion_context, "argument-label");
        assert_eq!(report.prefix, "notif");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "notificationID")
            .expect("expected enum-backed function parameter-label completion");
        assert_eq!(item.text_edit.new_text, "${0:ENotification.}");
        assert_eq!(
            item.command
                .as_ref()
                .map(|command| command.command.as_str()),
            Some("reforger-sript-tools.completion.triggerSuggestAtSnippetPlaceholderEnd")
        );
        assert_eq!(item.required_parameter_count, 1);
    }

    #[test]
    fn completion_uses_named_parameter_when_parameter_is_out_of_order() {
        let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(float num)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(num, inp)
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "TestNumFun2(num, inp"),
            None,
        );

        assert_eq!(report.completion_context, "argument-label");
        assert_eq!(report.prefix, "inp");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "input")
            .expect("expected out-of-order input named-parameter completion");
        assert_eq!(item.text_edit.new_text, "input: $0");
    }

    #[test]
    fn completion_offers_active_parameter_for_empty_trailing_argument_slot() {
        let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(int input, float num)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(input, num,)
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "TestNumFun2(input, num,"),
            None,
        );

        assert_eq!(report.completion_context, "argument-label");
        assert_eq!(report.prefix, "");
        let first = report
            .list
            .items
            .first()
            .expect("expected parameter completions for trailing argument slot");
        assert_eq!(first.label, "test");
        assert_eq!(first.text_edit.new_text, "test");
    }

    #[test]
    fn completion_attribute_shorthand_defaults_required_enum_parameters_to_enum_owners() {
        let source = r#"class Example
{
	rplr int m_Value;
}
"#;
        let external = file_index_for_source(
            r#"enum RplChannel
{
	Reliable
}
enum RplRcver
{
	Server
	Owner
}
enum RplCondition
{
	None
}
class UniqueAttribute {}
class RplRpc : UniqueAttribute
{
	void RplRpc(RplChannel channel, RplRcver rcver, RplCondition condition = RplCondition.None);
}
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "rplr"),
            Some(&external),
        );

        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "RplRpc")
            .expect("expected RplRpc attribute shorthand completion");
        assert_eq!(item.text_edit.new_text, "[RplRpc(${1:RplChannel.})]");
        assert_eq!(
            item.command
                .as_ref()
                .map(|command| command.command.as_str()),
            Some("reforger-sript-tools.completion.triggerSuggestAtSnippetPlaceholderEnd")
        );
        assert_eq!(item.required_parameter_count, 2);
        assert_eq!(item.optional_parameter_count, 1);
    }

    #[test]
    fn completion_enum_member_advances_attribute_snippet_to_next_parameter() {
        let source = r#"class Example
{
	[RplRpc(RplChannel.)]
	int m_Value;
}
"#;
        let external = file_index_for_source(
            r#"enum RplChannel
{
	Reliable
}
enum RplRcver
{
	Server
	Owner
}
class UniqueAttribute {}
class RplRpc : UniqueAttribute
{
	void RplRpc(RplChannel channel, RplRcver rcver);
}
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "RplChannel."),
            Some(&external),
        );

        assert_eq!(report.completion_context, "member");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "Reliable")
            .expect("expected enum member completion");
        assert_eq!(
            item.text_edit.new_text,
            "RplChannel.Reliable, ${1:RplRcver.}"
        );
        assert_eq!(item.insert_text_format, Some(2));
        assert_eq!(item.filter_text.as_deref(), Some("RplChannel.Reliable"));
        assert_eq!(
            item.command
                .as_ref()
                .map(|command| command.command.as_str()),
            Some("reforger-sript-tools.completion.triggerSuggestAtSnippetPlaceholderEnd")
        );

        let source = r#"class Example
{
	[RplRpc(RplChannel.Reliable, RplRcver.)]
	int m_Value;
}
"#;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "RplRcver."),
            Some(&external),
        );
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "Server")
            .expect("expected final enum member completion");
        assert_eq!(item.text_edit.new_text, "RplRcver.Server");
        assert!(item.command.is_none());
    }

    #[test]
    fn completion_falls_back_to_value_candidates_when_argument_label_prefix_has_no_match() {
        let source = r#"int testChannel;

class Example
{
	[RplRpc(tes, RplRcver.Server)]
	int m_Value;
}
"#;
        let external = file_index_for_source(
            r#"enum RplChannel
{
	Reliable
}
enum RplRcver
{
	Server
}
class UniqueAttribute {}
class RplRpc : UniqueAttribute
{
	void RplRpc(RplChannel channel, RplRcver rcver);
}
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "RplRpc(tes"),
            Some(&external),
        );

        assert_eq!(report.prefix, "tes");
        assert_ne!(report.completion_context, "argument-label");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "testChannel"));
    }

    #[test]
    fn completion_returns_parameter_labels_inside_constructor_calls() {
        let source = r#"class Widget
{
	void Widget(string name = "", int value = 0);
}

class Example
{
	void Run()
	{
		Widget widget = new Widget(na);
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "new Widget(na"),
            None,
        );

        assert_eq!(report.completion_context, "argument-label");
        assert_eq!(report.prefix, "na");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "name")
            .expect("expected constructor parameter-label completion");
        assert_eq!(item.text_edit.new_text, "name");
        assert_eq!(
            item.command
                .as_ref()
                .map(|command| command.command.as_str()),
            Some("editor.action.triggerParameterHints")
        );
        assert_eq!(item.optional_parameter_count, 1);
    }

    #[test]
    fn callable_completion_triggers_signature_help_after_insert() {
        let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run()
	{
		GRAY_TEST2 test44;
		test44.TestNum
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "test44.TestNum"),
            None,
        );

        assert_eq!(report.completion_context, "member");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "TestNumFun2")
            .expect("expected callable member completion");
        assert_eq!(item.text_edit.new_text, "TestNumFun2(${1:input}, ${2:num})");
        assert_eq!(item.required_parameter_count, 2);
        assert_eq!(item.optional_parameter_count, 1);
        assert_eq!(
            item.command
                .as_ref()
                .map(|command| command.command.as_str()),
            Some("editor.action.triggerParameterHints")
        );
    }

    #[test]
    fn completion_hides_already_supplied_parameter_labels() {
        let source = r#"class Example
{
	[Attribute(DEFVALUE: "", defv)]
	int m_Value;
}
"#;
        let external = file_index_for_source(
            r#"class UniqueAttribute {}
class Attribute : UniqueAttribute
{
	void Attribute(string defvalue = "", string uiwidget = "auto", string desc = "");
}
"#,
        )
        .index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "DEFVALUE: \"\", defv"),
            Some(&external),
        );

        assert!(!report
            .list
            .items
            .iter()
            .any(|item| item.label == "defvalue"));
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
        assert!(!labels.contains(&"SCR_Value"));
    }

    #[test]
    fn completion_caps_broad_top_level_prefixes() {
        let source = "class Example { void Run() { s } }";
        let mut external_source = String::new();
        for index in 0..400 {
            external_source.push_str(&format!("class sGenerated{index} {{}}\n"));
        }
        let external = file_index_for_source(&external_source).index;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "{ s"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "s");
        assert_eq!(report.list.items.len(), 250);
        assert_eq!(report.candidate_count, 250);
        assert!(report.list.is_incomplete);
    }

    #[test]
    fn completion_returns_visible_locals_for_unqualified_value_prefix() {
        let source = r#"class Example
{
	void Run(IEntity owner)
	{
		ow
	}
}
"#;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "ow"),
            None,
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "ow");
        assert!(report.list.items.iter().any(|item| item.label == "owner"
            && item.kind == 6
            && item.text_edit.new_text == "owner"));
    }

    #[test]
    fn completion_returns_current_class_members_for_unqualified_value_prefix() {
        let source = r#"class Example
{
	IEntity GetOwner();
	void Run()
	{
		GetO
	}
}
"#;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "GetO"),
            None,
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "GetO");
        assert!(report.list.items.iter().any(|item| item.label == "GetOwner"
            && item.kind == 2
            && item.text_edit.new_text == "GetOwner()"));
    }

    #[test]
    fn completion_matches_unqualified_prefix_case_insensitively() {
        let source = r#"class Example
{
	IEntity GetOwner();
	void Run(IEntity owner)
	{
		if (owner == get)
		{
		}
	}
}
"#;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "get"),
            None,
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "get");
        assert!(report.list.items.iter().any(|item| item.label == "GetOwner"
            && item.kind == 2
            && item.text_edit.new_text == "GetOwner()"));
    }

    #[test]
    fn completion_returns_cross_layer_inherited_members_for_unqualified_prefix() {
        let source = r#"class Example : ScriptComponent
{
	void Run(IEntity owner)
	{
		if (owner == getow)
		{
		}
	}
}
"#;
        let external = file_index_for_source(
            r#"class ScriptComponent
{
	IEntity GetOwner();
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "getow"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "getow");
        assert!(report.list.items.iter().any(|item| item.label == "GetOwner"
            && item.kind == 2
            && item.text_edit.new_text == "GetOwner()"));
    }

    #[test]
    fn completion_keeps_value_context_for_incomplete_statement_before_declaration() {
        let source = r#"class Game
{
}

Game GetGame();

class Example
{
	void Run()
	{
		getgam

		int testnum = 44;

		GetGame().GetPlayerController().GetControlledEntity();
	}
}
"#;

        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "getgam"),
            None,
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "getgam");
        assert!(report.list.items.iter().any(|item| item.label == "GetGame"
            && item.kind == 3
            && item.text_edit.new_text == "GetGame()"));
    }

    #[test]
    fn completion_returns_language_keywords_for_value_prefixes() {
        let source = r#"class Example
{
	void Run()
	{
		retur
	}
}
"#;
        let external = file_index_for_source(
            r#"enum EOrder
{
	RETURN_FIRE,
	RETURN_TO_PREVIOUS_STATE
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "retur"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "retur");
        let first = report.list.items.first().unwrap();
        assert_eq!(first.label, "return");
        assert_eq!(first.kind, 14);
        assert_eq!(first.text_edit.new_text, "return");
        assert!(!report
            .list
            .items
            .iter()
            .any(|item| item.label == "RETURN_FIRE"));
    }

    #[test]
    fn completion_ranks_closest_keyword_before_matching_source_symbols() {
        let source = r#"class Example
{
	void Run()
	{
		stati
	}
}
"#;
        let external = file_index_for_source(
            r#"enum EStaticKind
{
	STATIC,
	STATIC_ARLAND_AIRBASE
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "stati"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "stati");
        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels.first().copied(), Some("static"));
        assert!(!labels.contains(&"STATIC"));
        assert!(!labels.contains(&"Static"));
    }

    #[test]
    fn completion_ranks_primitive_type_keyword_before_modifier_prefix() {
        let source = r#"class Example
{
	void Run()
	{
		in
	}
}
"#;
        let external = file_index_for_source(
            r#"class int
{
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "\t\tin"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "in");
        let labels = report
            .list
            .items
            .iter()
            .map(|item| (item.label.as_str(), item.kind))
            .collect::<Vec<_>>();
        assert_eq!(labels.first().copied(), Some(("int", 14)));
        assert!(labels.contains(&("inout", 14)));
        assert_eq!(
            labels.iter().filter(|(label, _)| *label == "int").count(),
            1
        );
    }

    #[test]
    fn completion_keeps_declaration_keywords_out_of_expression_contexts() {
        let source = r#"class Example
{
	void Run(bool enabled)
	{
		if (enabled == stati)
		{
		}
	}
}
"#;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "stati"),
            None,
        );

        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert!(!labels.contains(&"static"));
    }

    #[test]
    fn completion_returns_declaration_keywords_at_declaration_boundaries() {
        let source = r#"class Example
{
	sta
}
"#;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "sta"),
            None,
        );

        let first = report.list.items.first().unwrap();
        assert_eq!(first.label, "static");
        assert_eq!(first.kind, 14);
    }

    #[test]
    fn completion_returns_modifier_keywords_when_prefix_is_type_context() {
        let boundary_source = r#"class Example
{
	overr
}
"#;
        let boundary_report = completion_report_for_source_position_with_external(
            boundary_source,
            position_after_needle(boundary_source, "overr"),
            None,
        );
        let first = boundary_report.list.items.first().unwrap();
        assert_eq!(first.label, "override");
        assert_eq!(first.kind, 14);
        assert_eq!(first.text_edit.new_text, "override");

        let type_context_source = r#"class Example
{
	override overr
}
"#;
        let report = completion_report_for_source_position_with_external(
            type_context_source,
            position_after_needle(type_context_source, "override overr"),
            None,
        );
        let first = report.list.items.first().unwrap();
        assert_eq!(first.label, "override");
        assert_eq!(first.kind, 14);
        assert_eq!(first.text_edit.new_text, "override");
    }

    #[test]
    fn completion_returns_inherited_override_method_skeletons() {
        let source = r#"class Child : Parent
{
	OnPostIn
}
"#;
        let external = file_index_for_source(
            r#"class Parent
{
	protected void OnPostInit(IEntity owner);
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "OnPostIn"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "override");
        assert_eq!(report.prefix, "OnPostIn");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "OnPostInit")
            .expect("expected inherited override completion");
        assert_eq!(item.kind, 2);
        assert_eq!(item.detail.as_deref(), Some("override protected void"));
        assert_eq!(item.insert_text_format, Some(2));
        assert_eq!(
            item.text_edit.new_text,
            "override protected void OnPostInit(IEntity owner)\n{\n\t$0\n}"
        );
    }

    #[test]
    fn completion_returns_override_skeletons_for_event_base_methods() {
        let source = r#"class Child : ScriptComponent
{
	onpostin
}
"#;
        let external = file_index_for_source(
            r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "onpostin"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "override");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "OnPostInit")
            .expect("expected event base method override completion");
        assert_eq!(
            item.text_edit.new_text,
            "override protected void OnPostInit(IEntity owner)\n{\n\t$0\n}"
        );
    }

    #[test]
    fn completion_keeps_override_keyword_when_override_skeletons_are_available() {
        let source = r#"class Child : ScriptComponent
{
	o
}
"#;
        let external = file_index_for_source(
            r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "\to"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "override");
        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels.first().copied(), Some("override"));
        assert!(labels.contains(&"OnPostInit"));
    }

    #[test]
    fn completion_keeps_source_symbols_when_override_skeletons_are_available() {
        let source = r#"class Child : Parent
{
	rp
}
"#;
        let external = file_index_for_source(
            r#"class UniqueAttribute {}
class RplProp : UniqueAttribute
{
}

class Parent
{
	protected bool RplLoad(ScriptBitReader reader);
	protected bool RplSave(ScriptBitWriter writer);
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "rp"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "override");
        assert_eq!(report.prefix, "rp");
        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels.first().copied(), Some("RplProp"));
        assert!(labels.contains(&"RplLoad"));
        assert!(labels.contains(&"RplSave"));
        assert!(labels.contains(&"RplProp"));
    }

    #[test]
    fn completion_ranks_closest_source_symbol_before_capping() {
        let source = r#"class Example
{
	rp
}
"#;
        let mut external_source = String::new();
        for index in 0..400 {
            external_source.push_str(&format!("class RplGenerated{index} {{}}\n"));
        }
        external_source.push_str("class RplProp {}\n");
        let external = file_index_for_source(&external_source).index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "rp"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "rp");
        assert_eq!(report.list.items.len(), 250);
        assert!(report.list.is_incomplete);
        assert_eq!(report.list.items.first().unwrap().label, "RplProp");
        assert!(report.list.items.iter().any(|item| item.label == "RplProp"));
    }

    #[test]
    fn completion_match_quality_beats_source_rank_for_top_level_symbols() {
        let source = r#"typedef func SCR_BaseGameMode_PlayerId;
typedef func SCR_BaseGameMode_PlayerIdAndEntity;
typedef func SCR_BaseGameMode_OnPlayerRoleChanged;

class Example
{
	rplr
}
"#;
        let external = file_index_for_source(
            r#"enum RplRcver {}
class UniqueAttribute {}
class RplRpc : UniqueAttribute
{
	void RplRpc(RplChannel channel, RplRcver rcver, RplCondition condition = RplCondition.None);
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "rplr"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "top-level");
        assert_eq!(report.prefix, "rplr");
        let labels = report
            .list
            .items
            .iter()
            .take(5)
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels.first().copied(), Some("RplRpc"));
        assert!(labels.contains(&"RplRcver"));
        assert!(
            labels.iter().position(|label| *label == "RplRpc").unwrap()
                < labels
                    .iter()
                    .position(|label| *label == "SCR_BaseGameMode_PlayerId")
                    .unwrap_or(usize::MAX)
        );
    }

    #[test]
    fn completion_override_skeleton_omits_already_typed_modifiers() {
        let source = r#"class Child : ScriptComponent
{
	override protected onpostin
}
"#;
        let external = file_index_for_source(
            r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "onpostin"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "override");
        let item = report
            .list
            .items
            .iter()
            .find(|item| item.label == "OnPostInit")
            .expect("expected inherited override completion");
        assert_eq!(item.detail.as_deref(), Some("void"));
        assert_eq!(
            item.text_edit.new_text,
            "void OnPostInit(IEntity owner)\n{\n\t$0\n}"
        );
    }

    #[test]
    fn completion_returns_override_skeletons_before_inline_comment_at_class_scope() {
        let source = r#"class GRAY_TEST : ScriptComponent
{
	int testnnn;
	onpostin//Nothing appearing

	override protected void OnPostInit(IEntity owner)
	{
	}
}
"#;
        let external = file_index_for_source(
            r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "onpostin"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "override");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "OnPostInit"));
    }

    #[test]
    fn completion_returns_override_skeletons_before_following_method_without_comment() {
        let source = r#"class GRAY_TEST : ScriptComponent
{
	int testnnn;
	onpostin

	override protected void OnPostInit(IEntity owner)
	{
	}
}
"#;
        let external = file_index_for_source(
            r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "onpostin"),
            Some(&external),
        );

        assert_eq!(report.completion_context, "override");
        assert!(report
            .list
            .items
            .iter()
            .any(|item| item.label == "OnPostInit"));
    }

    #[test]
    fn completion_excludes_private_and_static_methods_from_override_skeletons() {
        let source = r#"class Child : Parent
{
	On
}
"#;
        let external = file_index_for_source(
            r#"class Parent
{
	private void OnPrivate();
	static void OnStatic();
	protected void OnAllowed();
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "On"),
            Some(&external),
        );
        let labels = report
            .list
            .items
            .iter()
            .filter(|item| item.text_edit.new_text.starts_with("override "))
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();

        assert!(labels.contains(&"OnAllowed"));
        assert!(!labels.contains(&"OnPrivate"));
        assert!(!labels.contains(&"OnStatic"));
        assert!(!report
            .list
            .items
            .iter()
            .any(|item| item.text_edit.new_text.starts_with("override static")));
    }

    #[test]
    fn completion_does_not_return_override_skeletons_inside_method_bodies() {
        let source = r#"class Child : Parent
{
	void Run()
	{
		OnPostIn
	}
}
"#;
        let external = file_index_for_source(
            r#"class Parent
{
	protected void OnPostInit(IEntity owner);
}
"#,
        )
        .index;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "OnPostIn"),
            Some(&external),
        );

        assert_ne!(report.completion_context, "override");
        assert!(!report
            .list
            .items
            .iter()
            .any(|item| item.text_edit.new_text.contains("override protected void")));
    }

    #[test]
    fn completion_returns_empty_inside_comments() {
        let source = r#"class Example
{
	void Run()
	{
		// get
	}
}
"#;
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "get"),
            None,
        );

        assert_eq!(report.completion_context, "none");
        assert!(report.list.items.is_empty());
    }

    #[test]
    fn completion_returns_empty_inside_block_comments_after_code() {
        let source = r#"class Example
{
	void Run()
	{
		int testnnn = 1; /* testnnn */
	}
}
"#;
        let report = completion_report_for_source_position_with_external(
            source,
            position_for_needle(source, "/* testnnn", "test"),
            None,
        );

        assert_eq!(report.completion_context, "none");
        assert!(report.list.items.is_empty());
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
        assert!(report.list.items.iter().all(|item| item.command.is_none()));
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
        assert_eq!(
            report.hover.as_ref().and_then(|hover| hover.range),
            Some(LspRange {
                start: LspPosition {
                    line: 2,
                    character: 6,
                },
                end: LspPosition {
                    line: 2,
                    character: 9,
                },
            })
        );
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
        assert_eq!(file_uri_for_path(Path::new("relative/File.c")), None);
        if cfg!(windows) {
            let uri = file_uri_for_path(Path::new("C:\\Game Data\\Scripts\\File Name.c")).unwrap();
            assert_eq!(uri, "file:///C:/Game%20Data/Scripts/File%20Name.c");
        } else {
            let uri = file_uri_for_path(Path::new("/tmp/Game Data/File Name.c")).unwrap();
            assert_eq!(uri, "file:///tmp/Game%20Data/File%20Name.c");
        }
    }

    #[test]
    fn file_uri_for_path_encodes_windows_unc_authority() {
        if cfg!(windows) {
            assert_eq!(
                file_uri_for_path(Path::new(r"\\server\share\File Name.c")).unwrap(),
                "file://server/share/File%20Name.c"
            );
            assert_eq!(
                file_uri_for_path(Path::new(r"\\?\UNC\server\share\File.c")).unwrap(),
                "file://server/share/File.c"
            );
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

        assert!(markdown.contains("data-code=\"protected void Run(int value = 4)\""));
        assert!(markdown.contains("<span style=\"color:#59A6E9;\">protected</span>"));
        assert!(markdown.contains("<span style=\"color:#59A6E9;\">void</span>"));
        assert!(markdown.contains("<span style=\"color:#f3ad58;\">Run</span>"));
        assert!(markdown.contains("Runs the example."));
        assert!(!markdown.contains("Modifiers: protected"));
        assert!(!markdown.contains("Attributes: Attribute"));
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
    fn offset_conversion_treats_cr_and_crlf_as_single_line_endings() {
        for source in ["class A {}\rclass B {}", "class A {}\r\nclass B {}"] {
            let offset = source.find("class B").expect("second class");
            let position = position_for_offset(source, offset);
            assert_eq!(
                position,
                LspPosition {
                    line: 1,
                    character: 0
                }
            );
            assert_eq!(offset_for_position(source, position), Some(offset));
        }
    }

    #[test]
    fn position_index_preserves_utf16_and_crlf_boundaries() {
        let source = "ab😀\r\nclass Marker {}";
        let index = LspPositionIndex::new(source);

        assert_eq!(
            index.position_for_offset(source.find('😀').expect("emoji")),
            LspPosition {
                line: 0,
                character: 2
            }
        );
        assert_eq!(
            index.position_for_offset(source.find("class").expect("second line")),
            LspPosition {
                line: 1,
                character: 0
            }
        );
    }

    #[test]
    fn document_symbol_projection_builds_one_position_index() {
        POSITION_INDEX_BUILD_COUNT.with(|count| count.set(0));

        let report = document_symbol_report_for_source(
            "class First { void Run() {} }\nclass Second { int value; }\n",
        );

        assert_eq!(report.symbols.len(), 2);
        assert_eq!(POSITION_INDEX_BUILD_COUNT.with(|count| count.get()), 1);
    }

    #[test]
    fn position_index_stops_when_cancellation_arrives_mid_build() {
        let source = "field ".repeat(256);
        let checks = Cell::new(0usize);

        let index = LspPositionIndex::new_cancellable(
            &source,
            Some(&|| {
                checks.set(checks.get() + 1);
                checks.get() >= 2
            }),
        );

        assert!(index.is_none());
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
    fn framed_lsp_contains_invalid_request_params_and_continues() {
        let mut input = Vec::new();
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/hover",
                "params": {}
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "initialize",
                "params": {}
            }),
        );
        write_test_message(
            &mut input,
            json!({"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null}),
        );
        write_test_message(
            &mut input,
            json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
        );

        let mut output = Vec::new();
        run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\"id\":1"));
        assert!(output.contains("\"code\":-32602"));
        assert!(output.contains("\"id\":2"));
        assert!(output.contains("\"serverInfo\""));
    }

    #[test]
    fn framed_lsp_ignores_invalid_notification_params_and_continues() {
        let mut input = Vec::new();
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {}
            }),
        );
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
            json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null}),
        );
        write_test_message(
            &mut input,
            json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
        );

        let mut output = Vec::new();
        run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(!output.contains("\"error\""));
        assert!(output.contains("\"id\":1"));
        assert!(output.contains("\"serverInfo\""));
    }

    #[test]
    fn framed_lsp_rejects_requests_after_shutdown() {
        let mut input = Vec::new();
        write_test_message(
            &mut input,
            json!({"jsonrpc": "2.0", "id": 1, "method": "shutdown", "params": null}),
        );
        write_test_message(
            &mut input,
            json!({"jsonrpc": "2.0", "id": 2, "method": "initialize", "params": {}}),
        );
        write_test_message(
            &mut input,
            json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
        );

        let mut output = Vec::new();
        run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\"id\":2"));
        assert!(output.contains("\"code\":-32600"));
    }

    #[test]
    fn framed_lsp_exit_before_shutdown_is_an_error() {
        let mut input = Vec::new();
        write_test_message(
            &mut input,
            json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
        );

        let error = run(input.as_slice(), Vec::new(), LspServerOptions::default()).unwrap_err();

        assert!(error.contains("before shutdown"));
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
    fn framed_lsp_did_change_defers_document_symbol_projection_until_requested() {
        let old_source = "class Old\n{\n\tvoid OldRun();\n}\n";
        let new_source = "class New\n{\n\tvoid NewRun();\n}\n";
        let log_path = test_log_path("lazy_document_symbols_after_change");
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
                        "uri": "file:///Scripts/LazySymbols.c",
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
                        "uri": "file:///Scripts/LazySymbols.c",
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
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/LazySymbols.c"
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
                log_path: Some(log_path.clone()),
                game_data_scripts: None,
                game_data_metadata: None,
                index_cache: None,
                workspace_scripts: Vec::new(),
            },
        )
        .unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"name\":\"New\""));
        assert!(output_text.contains("\"name\":\"NewRun\""));
        assert!(!output_text.contains("\"name\":\"Old\""));
        assert!(!output_text.contains("\"name\":\"OldRun\""));

        let log = std::fs::read_to_string(&log_path).unwrap();
        assert!(log.contains("notification didChange"));
        assert!(log.contains("notification didChange uri=file:///Scripts/LazySymbols.c"));
        assert!(log.contains("document_symbols_cached=false symbols=pending"));
        assert!(log.contains("request documentSymbol uri=file:///Scripts/LazySymbols.c"));
        assert!(log.contains("document_symbols_cached=false document_symbol_ms="));

        cleanup_log(&log_path);
    }

    #[test]
    fn channel_runtime_coalesces_contiguous_full_sync_changes_before_outline_request() {
        let uri = "file:///Scripts/Coalesced.c";
        let log_path = test_log_path("coalesced_channel_changes");
        let (incoming_sender, incoming_receiver) = mpsc::channel();
        let (internal_sender, internal_receiver) = mpsc::channel();
        let mut server = LspServer::new(
            Vec::new(),
            LspServerOptions {
                log_path: Some(log_path.clone()),
                ..LspServerOptions::default()
            },
        );
        let send = |value| {
            incoming_sender
                .send(ServerEvent::Incoming {
                    received_at: Instant::now(),
                    result: Ok(value),
                })
                .unwrap();
        };
        send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": { "textDocument": {
                "uri": uri,
                "languageId": "enforce",
                "version": 1,
                "text": "class Initial {}"
            }}
        }));
        for (version, name) in [(2, "Second"), (3, "Third")] {
            send(json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri, "version": version },
                    "contentChanges": [{ "text": format!("class {name} {{}}") }]
                }
            }));
        }
        send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": { "textDocument": { "uri": uri } }
        }));
        send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 4 },
                "contentChanges": [{ "text": "class Current {}" }]
            }
        }));
        send(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentSymbol",
            "params": { "textDocument": { "uri": uri } }
        }));
        drop(incoming_sender);
        drop(internal_sender);

        server
            .run_message_channels(incoming_receiver, internal_receiver)
            .unwrap();

        let output = String::from_utf8(server.writer).unwrap();
        assert!(output.contains("\"id\":1"));
        assert!(output.contains("\"id\":2"));
        assert!(output.contains("\"name\":\"Third\""));
        assert!(output.contains("\"name\":\"Current\""));
        assert!(!output.contains("\"name\":\"Second\""));

        let log = std::fs::read_to_string(&log_path).unwrap();
        assert_eq!(log.matches("notification didChange uri=").count(), 2);
        assert!(log.contains("version=3"));
        assert!(log.contains("version=4"));
        assert!(log.contains("coalesced_changes=2 superseded_changes=1"));
        cleanup_log(&log_path);
    }

    #[test]
    fn only_single_full_text_changes_are_coalescible() {
        let full_text = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": "file:///Scripts/Full.c", "version": 2 },
                "contentChanges": [{ "text": "class Full {}" }]
            }
        });
        let ranged = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": "file:///Scripts/Range.c", "version": 2 },
                "contentChanges": [{
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } },
                    "text": "x"
                }]
            }
        });
        let multiple = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": "file:///Scripts/Multiple.c", "version": 2 },
                "contentChanges": [{ "text": "class A {}" }, { "text": "class B {}" }]
            }
        });

        assert_eq!(
            coalescible_full_sync_did_change(&full_text).map(|change| (change.uri, change.version)),
            Some(("file:///Scripts/Full.c".to_string(), 2))
        );
        assert!(coalescible_full_sync_did_change(&ranged).is_none());
        assert!(coalescible_full_sync_did_change(&multiple).is_none());
    }

    #[test]
    fn document_analysis_scheduler_keeps_only_latest_pending_revision() {
        let (sender, receiver) = mpsc::channel();
        let scheduler = OpenDocumentAnalysisScheduler::start(sender);
        scheduler.schedule(OpenDocumentAnalysisJob {
            uri: "file:///Scripts/Pending.c".to_string(),
            revision: 2,
            source: "class Old {}".to_string(),
            cancel: Arc::new(AtomicBool::new(false)),
            scheduled_at: Instant::now(),
        });
        scheduler.schedule(OpenDocumentAnalysisJob {
            uri: "file:///Scripts/Pending.c".to_string(),
            revision: 3,
            source: "class Current {}".to_string(),
            cancel: Arc::new(AtomicBool::new(false)),
            scheduled_at: Instant::now(),
        });

        let event = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("latest analysis result");

        assert!(matches!(
            event,
            ServerEvent::DocumentAnalysisReady { revision: 3, .. }
        ));
    }

    #[test]
    fn pending_document_symbol_request_replays_after_current_analysis_installs() {
        let (sender, receiver) = mpsc::channel();
        let scheduler = OpenDocumentAnalysisScheduler::start(sender);
        let mut server = LspServer::new_with_runtime_senders(
            Vec::new(),
            LspServerOptions::default(),
            None,
            Some(scheduler),
        );
        let uri = "file:///Scripts/PendingOutline.c";

        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": { "textDocument": {
                        "uri": uri,
                        "languageId": "enforce",
                        "version": 1,
                        "text": "class Initial {}"
                    }}
                }),
                None,
                0,
                0,
            )
            .unwrap();
        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didChange",
                    "params": {
                        "textDocument": { "uri": uri, "version": 2 },
                        "contentChanges": [{ "text": "class Current {}" }]
                    }
                }),
                None,
                0,
                0,
            )
            .unwrap();
        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "textDocument/documentSymbol",
                    "params": { "textDocument": { "uri": uri } }
                }),
                None,
                0,
                0,
            )
            .unwrap();
        assert!(!String::from_utf8_lossy(&server.writer).contains("\"id\":1"));

        server
            .handle_internal_event(
                receiver
                    .recv_timeout(Duration::from_secs(2))
                    .expect("document analysis result"),
            )
            .unwrap();

        let output = String::from_utf8(server.writer).unwrap();
        assert!(output.contains("\"id\":1"));
        assert!(output.contains("\"name\":\"Current\""));
        assert!(!output.contains("\"name\":\"Initial\""));
    }

    #[test]
    fn pending_request_receives_content_modified_when_a_new_edit_supersedes_it() {
        let (sender, _receiver) = mpsc::channel();
        let scheduler = OpenDocumentAnalysisScheduler::start(sender);
        let mut server = LspServer::new_with_runtime_senders(
            Vec::new(),
            LspServerOptions::default(),
            None,
            Some(scheduler),
        );
        let uri = "file:///Scripts/SupersededRequest.c";

        for (method, params) in [
            (
                "textDocument/didOpen",
                json!({ "textDocument": {
                    "uri": uri, "languageId": "enforce", "version": 1, "text": "class Initial {}"
                }}),
            ),
            (
                "textDocument/didChange",
                json!({
                    "textDocument": { "uri": uri, "version": 2 },
                    "contentChanges": [{ "text": "class Pending {}" }]
                }),
            ),
        ] {
            server
                .handle_message(
                    json!({ "jsonrpc": "2.0", "method": method, "params": params }),
                    None,
                    0,
                    0,
                )
                .unwrap();
        }
        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0", "id": 1,
                    "method": "textDocument/documentSymbol",
                    "params": { "textDocument": { "uri": uri } }
                }),
                None,
                0,
                0,
            )
            .unwrap();
        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0", "method": "textDocument/didChange",
                    "params": {
                        "textDocument": { "uri": uri, "version": 3 },
                        "contentChanges": [{ "text": "class Current {}" }]
                    }
                }),
                None,
                0,
                0,
            )
            .unwrap();

        let output = String::from_utf8(server.writer).unwrap();
        assert!(output.contains("\"id\":1"));
        assert!(output.contains("\"code\":-32801"));
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
        assert!(output_text.contains("\"signatureHelpProvider\""));
        assert!(
            output_text.contains("\"completionProvider\":{\"triggerCharacters\":[\".\",\"[\"]}")
        );
        assert!(output_text.contains("void Run(int value)"));
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
        assert!(
            output_text.contains("\"completionProvider\":{\"triggerCharacters\":[\".\",\"[\"]}")
        );
        assert!(output_text.contains("\"isIncomplete\":false"));
        assert!(output_text.contains("\"label\":\"SetVisible\""));
        assert!(output_text.contains("\"newText\":\"SetVisible(${1:visible})\""));
    }

    #[test]
    fn framed_lsp_smoke_test_handles_signature_help() {
        let source = "class Smoke\n{\n\tvoid Run(int value, string label = \"ok\");\n\tvoid Test(int input)\n\t{\n\t\tRun(1, );\n\t\tRun(inp);\n\t}\n}\n";
        let second_parameter_position = position_after_needle(source, "Run(1, ");
        let typed_argument_position = position_after_needle(source, "Run(inp");
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
                "method": "textDocument/signatureHelp",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Smoke.c"
                    },
                    "position": {
                        "line": second_parameter_position.line,
                        "character": second_parameter_position.character
                    }
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/signatureHelp",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Smoke.c"
                    },
                    "position": {
                        "line": typed_argument_position.line,
                        "character": typed_argument_position.character
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
        run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

        let output_text = String::from_utf8(output).unwrap();
        assert!(output_text.contains("\"signatureHelpProvider\""));
        assert!(output_text.contains("\"triggerCharacters\":[\"(\",\",\",\".\",\":\",\"_\",\"a\""));
        assert!(
            output_text.contains("\"retriggerCharacters\":[\"(\",\",\",\".\",\":\",\"_\",\"a\"")
        );
        assert!(output_text.contains("\"activeParameter\":1"));
        assert!(output_text.contains("\"activeParameter\":0"));
        assert!(output_text
            .contains("\"label\":\"Smoke.Run(int value, string label = \\\"ok\\\") -> void\""));
        assert!(output_text.contains("\"label\":\"int value\""));
        assert!(output_text.contains("\"label\":\"string label = \\\"ok\\\"\""));
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
                    "text": workspace_source,
                    "sequence": 1
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
                    "path": workspace_file.display().to_string(),
                    "sequence": 2
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
                    "text": workspace_source,
                    "sequence": 1
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
        assert_eq!(output_text.matches("void Run(int value)").count(), 2);

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
        assert!(output_text.contains("\"version\":1"));
        assert!(output_text.contains("\"version\":2"));
        assert!(
            clear_diagnostics_message("file:///Scripts/Diagnostics.c")["params"]
                .get("version")
                .is_none()
        );
        assert!(output_text.contains("\"diagnostics\":[]"));
    }

    #[test]
    fn framed_lsp_ignores_stale_changes_without_regressing_diagnostics_or_symbols() {
        let initial_source = "class Initial\n{\n\tvoid InitialRun(\n}\n";
        let current_source = "class Current\n{\n\tvoid CurrentRun();\n}\n";
        let stale_source = "class Stale\n{\n\tvoid StaleRun(\n}\n";
        let uri = "file:///Scripts/VersionedDiagnostics.c";
        let log_path = test_log_path("stale_diagnostic_change");
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
                        "uri": uri,
                        "languageId": "enforce",
                        "version": 1,
                        "text": initial_source
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
                    "textDocument": { "uri": uri, "version": 3 },
                    "contentChanges": [{ "text": current_source }]
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri, "version": 2 },
                    "contentChanges": [{ "text": stale_source }]
                }
            }),
        );
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/documentSymbol",
                "params": { "textDocument": { "uri": uri } }
            }),
        );
        write_test_message(
            &mut input,
            json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null }),
        );
        write_test_message(
            &mut input,
            json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
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
            output_text
                .matches("textDocument/publishDiagnostics")
                .count(),
            2
        );
        assert!(output_text.contains("\"version\":1"));
        assert!(output_text.contains("\"version\":3"));
        assert!(!output_text.contains("\"version\":2"));
        assert!(output_text.contains("\"name\":\"Current\""));
        assert!(output_text.contains("\"name\":\"CurrentRun\""));
        assert!(!output_text.contains("\"name\":\"Stale\""));
        assert!(!output_text.contains("\"name\":\"StaleRun\""));

        let log = std::fs::read_to_string(&log_path).unwrap();
        assert!(log.contains(
            "notification didChange ignored uri=file:///Scripts/VersionedDiagnostics.c version=2 current_version=3 reason=stale"
        ));
        cleanup_log(&log_path);
    }

    #[test]
    fn document_open_and_change_require_versions() {
        let uri = "file:///Scripts/RequiredVersions.c";
        let mut server = LspServer::new(Vec::new(), LspServerOptions::default());

        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didChange",
                    "params": {
                        "textDocument": { "uri": uri, "version": 1 },
                        "contentChanges": [{ "text": "class ChangedBeforeOpen {}" }]
                    }
                }),
                None,
                0,
                0,
            )
            .unwrap();
        assert!(!server.documents.contains_key(uri));

        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": uri,
                            "languageId": "enforce",
                            "text": "class MissingOpenVersion {}"
                        }
                    }
                }),
                None,
                0,
                0,
            )
            .unwrap();
        assert!(!server.documents.contains_key(uri));

        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": uri,
                            "languageId": "enforce",
                            "version": 1,
                            "text": "class Current {}"
                        }
                    }
                }),
                None,
                0,
                0,
            )
            .unwrap();
        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didChange",
                    "params": {
                        "textDocument": { "uri": uri },
                        "contentChanges": [{ "text": "class MissingChangeVersion {}" }]
                    }
                }),
                None,
                0,
                0,
            )
            .unwrap();

        let document = &server.documents[uri];
        assert_eq!(document.version, 1);
        assert_eq!(document.text, "class Current {}");

        server
            .handle_message(
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didChange",
                    "params": {
                        "textDocument": { "uri": uri, "version": 1 },
                        "contentChanges": [{ "text": "class SameVersionReplay {}" }]
                    }
                }),
                None,
                0,
                0,
            )
            .unwrap();

        let document = &server.documents[uri];
        assert_eq!(document.version, 1);
        assert_eq!(document.text, "class Current {}");
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

    #[test]
    fn framed_lsp_smoke_test_handles_debug_completion_request() {
        let source = "class Smoke\n{\n\tvoid Run()\n\t{\n\t\tSmoke value;\n\t\tvalue.\n\t}\n}\n";
        let completion_position = position_after_needle(source, "value.");
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
                        "uri": "file:///Scripts/SmokeCompletion.c",
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
                "method": "reforger/debugCompletion",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/SmokeCompletion.c"
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
        assert!(output_text.contains("# Reforger Completion Debug"));
        assert!(output_text.contains("## Completion Context"));
        assert!(output_text.contains("## Signature Help Context"));
        assert!(output_text.contains("not in callable argument list"));
        assert!(output_text.contains("value"));
        assert!(output_text.contains("Run"));
        assert!(!output_text.contains("Method not found"));
    }

    #[test]
    fn framed_lsp_debug_completion_includes_signature_help_when_inside_call() {
        let source = "class Smoke\n{\n\tvoid Run(int value, string label = \"ok\");\n\tvoid Test()\n\t{\n\t\tRun(1, );\n\t}\n}\n";
        let completion_position = position_after_needle(source, "Run(1, ");
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
                        "uri": "file:///Scripts/SmokeSignatureDebug.c",
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
                "method": "reforger/debugCompletion",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/SmokeSignatureDebug.c"
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
        assert!(output_text.contains("# Reforger Completion Debug"));
        assert!(output_text.contains("## Signature Help Context"));
        assert!(output_text.contains("- Active Parameter: `1`"));
        assert!(output_text.contains("Smoke.Run(int value, string label = \\\"ok\\\") -> void"));
        assert!(output_text.contains("string label = \\\"ok\\\""));
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

    #[test]
    fn read_message_rejects_an_oversized_header_before_parsing() {
        let input = format!("X-Long: {}\r\n\r\n", "x".repeat(16 * 1024));
        let error = read_message(&mut BufReader::new(input.as_bytes())).unwrap_err();

        assert_eq!(error, "LSP header line exceeds the configured limit");
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
                    && ((matches!(token.token_type, "class" | "type" | "typeParameter")
                        && token.color == "#40b5ac")
                        || (token.token_type == "enum" && token.color == "#40b5ac"))
            })
            .count();
        assert!(
            actual >= expected,
            "expected at least {expected} type-family semantic tokens text={text:?}, found {actual}: {:?}",
            report.decoded
        );
    }
}
