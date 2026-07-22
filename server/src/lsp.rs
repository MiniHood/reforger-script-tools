use crate::analysis_runtime::{
    AdmissionDisposition, AnalysisTask, PositionIndex, QueryQuality, TaskClass, TaskIdentity,
    UpsertOutcome,
};
#[cfg(test)]
use crate::analysis_runtime::{AdmissionLimits, AnalysisRuntime};
use crate::index::{GlobalSymbolId, SymbolIndex};
use crate::index_query::IndexQuery;
use crate::lexer::{lex, Keyword, TextSpan, Token, TokenKind};
use crate::model::SymbolKind;
use crate::parser::parse_source;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[cfg(test)]
use std::cell::Cell;
#[cfg(test)]
use std::collections::BTreeMap;
use std::io::{self, BufReader, Read, Write};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::mpsc;
#[cfg(test)]
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
#[cfg(test)]
use std::time::Duration;
use std::time::Instant;

mod background_events;
mod callable;
mod completion;
mod debug_hover;
mod definition;
mod diagnostics;
mod document_query;
mod document_runtime;
mod external_indexes;
mod external_overlay;
mod hover;
mod hover_render;
mod incoming_scheduler;
mod logging;
mod on_type_formatting;
mod open_documents;
mod request_router;
mod response_writer;
mod runtime_scheduler;
mod semantic_tokens;
mod signature_help;
mod transport;

use completion::{
    completion_debug_markdown, completion_report_for_cached_analysis_with_external_indexes,
    completion_report_for_current_argument_labels_at_offset_with_external_indexes,
    completion_report_for_current_local_scope_at_offset_with_external_indexes,
    completion_report_for_current_override_at_offset_with_external_indexes,
    completion_report_for_current_receiver_at_offset_with_external_indexes,
    completion_report_for_lexical_source_at_offset_with_external_indexes,
    completion_report_for_lexical_source_with_external_indexes, empty_completion_list,
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
pub(crate) use definition::file_uri_for_path;
pub use definition::{
    definition_report_for_cached_analysis_with_external, definition_report_for_source_position,
    definition_report_for_source_position_with_external, LspDefinitionReport, LspLocation,
    LspLocationLink,
};
use definition::{
    definition_report_for_cached_analysis_with_external_indexes,
    definition_report_for_pending_snapshot,
};
use diagnostics::{clear_diagnostics_message, publish_diagnostics_message};
pub use diagnostics::{parser_diagnostics_for_source, LspDiagnostic};
use document_query::{DocumentQuery, DocumentQueryState};
use document_runtime::DocumentRuntime;
pub(crate) use external_overlay::ExternalIndexStatusSummary;
use external_overlay::{start_external_index, ExternalIndexHandle, ExternalIndexSnapshot};
use hover::{
    hover_report_for_cached_analysis_with_external_indexes, hover_report_for_pending_snapshot,
};
pub use hover::{
    hover_report_for_source_position, hover_report_for_source_position_with_external,
    hover_reports_for_source_positions, hover_reports_for_source_positions_with_external,
    HoverSelectionSource, LspHover, LspHoverReport,
};
use logging::LspLogger;
pub use open_documents::{file_index_for_source, FileIndexAnalysis};
pub(crate) use open_documents::{
    file_index_for_source_with_timings, FileIndexAnalysisTimings, OpenDocument,
    TokenProjectionKind, TokenResultDisposition,
};
use response_writer::RuntimeEffect;
use runtime_scheduler::{ForegroundDocumentJob, OpenDocumentAnalysisJob, RuntimeWorkExecutor};
pub use semantic_tokens::{
    fast_semantic_tokens_for_source, fast_semantic_tokens_report_for_source,
    semantic_tokens_for_source_with_external, semantic_tokens_report_for_source,
    semantic_tokens_report_for_source_with_external, LspSemanticTokenReport,
    LspSemanticTokenTimings, SemanticTokenDebug,
};
use semantic_tokens::{
    lexical_semantic_tokens_for_source, semantic_tokens_for_cached_analysis_with_external_indexes,
    semantic_tokens_for_cached_analysis_with_external_indexes_cancelled,
    LspSemanticTokenProjection, LspSemanticTokensFull, SEMANTIC_TOKEN_MODIFIERS,
    SEMANTIC_TOKEN_TYPES,
};
use signature_help::{
    signature_help_debug_markdown, signature_help_report_for_cached_analysis_with_external_indexes,
    signature_help_report_for_pending_snapshot,
};
pub use signature_help::{
    signature_help_report_for_source_position, LspParameterInformation, LspSignatureHelp,
    LspSignatureHelpReport, LspSignatureHelpTimings, LspSignatureInformation,
};
use transport::read_message;

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
const BLOCK_COMMENT_PAIR_METHOD: &str = "reforger/blockCommentPair";
const ENTER_TYPING_ASSIST_METHOD: &str = "reforger/enterTypingAssist";
const RANGE_FORMATTING_METHOD: &str = "textDocument/rangeFormatting";
const WORKSPACE_FILE_CHANGED_METHOD: &str = "reforger/workspaceFileChanged";
const WORKSPACE_FILE_DELETED_METHOD: &str = "reforger/workspaceFileDeleted";
const INCOMING_EVENT_QUEUE_CAPACITY: usize = 64;
const MAX_PENDING_DOCUMENT_ANALYSIS_JOBS: usize = 32;
const MAX_PENDING_DOCUMENT_REQUESTS_PER_URI: usize = 32;
// The executor retains a single foreground slot. A second CPU-bearing worker
// exists only when the host actually has another logical CPU available. This
// remains one executor and one admitted-work map, rather than a second
// scheduler.
const FOREGROUND_RUNTIME_WORKERS: usize = 1;
const MAX_BACKGROUND_RUNTIME_WORKERS: usize = 1;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LspServerOptions {
    pub log_path: Option<PathBuf>,
    pub diagnostic_log_path: Option<PathBuf>,
    pub game_data_scripts: Option<PathBuf>,
    pub game_data_metadata: Option<PathBuf>,
    pub index_cache: Option<PathBuf>,
    pub workspace_scripts: Vec<PathBuf>,
}

pub fn run_stdio(options: LspServerOptions) -> Result<(), String> {
    let stdout = io::stdout();
    let (incoming_sender, incoming_receiver) = mpsc::sync_channel(INCOMING_EVENT_QUEUE_CAPACITY);
    let (internal_sender, internal_receiver) = mpsc::channel();
    let analysis_scheduler = RuntimeWorkExecutor::start(internal_sender.clone());
    let mut server = LspServer::new_with_runtime_senders(
        stdout.lock(),
        options,
        None,
        Some(analysis_scheduler),
        Some(internal_sender),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    logger: LspLogger,
    external_index: ExternalIndexHandle,
    document_runtime: DocumentRuntime,
    shutdown_requested: bool,
}

// Transitional bridge while request routing and background publication move to
// typed Document Runtime commands and effects. It keeps the first ownership
// extraction behavior-preserving; no new code may rely on this forwarding.
impl<W: Write> Deref for LspServer<W> {
    type Target = DocumentRuntime;

    fn deref(&self) -> &Self::Target {
        &self.document_runtime
    }
}

impl<W: Write> DerefMut for LspServer<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.document_runtime
    }
}

enum ServerEvent {
    Incoming {
        received_at: Instant,
        result: Result<Value, String>,
    },
    RichSemanticTokensReady {
        task: TaskIdentity,
        uri: String,
        revision: u64,
        external_generation: u64,
        external_status: &'static str,
        projection: LspSemanticTokenProjection,
        elapsed_ms: u128,
    },
    RichSemanticTokensSkipped {
        task: TaskIdentity,
        uri: String,
        revision: u64,
        external_generation: u64,
        reason: String,
        elapsed_ms: u128,
    },
    DocumentAnalysisReady {
        task: TaskIdentity,
        analysis: FileIndexAnalysis,
        timings: FileIndexAnalysisTimings,
        elapsed_ms: u128,
    },
    ForegroundDocumentReady {
        task: TaskIdentity,
        positions: PositionIndex,
        lexer_tokens: Vec<Token>,
        syntax: crate::syntax::Parse,
        elapsed_ms: u128,
    },
    ForegroundDocumentSkipped {
        task: TaskIdentity,
        reason: String,
        elapsed_ms: u128,
    },
    DocumentAnalysisSkipped {
        task: TaskIdentity,
        reason: String,
        elapsed_ms: u128,
    },
    DebugRequestReady {
        task: TaskIdentity,
        id: Value,
        method: &'static str,
        uri: String,
        revision: u64,
        details: String,
        result: Value,
        elapsed_ms: u128,
    },
}

fn source_backed_request_method(method: &str) -> bool {
    matches!(method, DEBUG_HOVER_METHOD | DEBUG_COMPLETION_METHOD)
}

fn request_document_uri(params: Option<&Value>) -> Option<String> {
    params?
        .get("textDocument")?
        .get("uri")?
        .as_str()
        .map(str::to_string)
}

struct RichSemanticTokensJob {
    task: AnalysisTask,
    uri: String,
    revision: u64,
    external_generation: u64,
    scheduled_at: Instant,
    analysis: FileIndexAnalysis,
    external_snapshot: ExternalIndexSnapshot,
}

enum DebugRequestJob {
    Hover(DebugHoverJob),
    Completion(DebugCompletionJob),
}

struct DebugHoverJob {
    task: AnalysisTask,
    id: Value,
    uri: String,
    position: LspPosition,
    revision: u64,
    scheduled_at: Instant,
    analysis: FileIndexAnalysis,
    external_snapshot: ExternalIndexSnapshot,
    external_status: ExternalIndexStatusSummary,
}

struct DebugCompletionJob {
    task: AnalysisTask,
    id: Value,
    uri: String,
    position: LspPosition,
    revision: u64,
    scheduled_at: Instant,
    analysis: FileIndexAnalysis,
    external_snapshot: ExternalIndexSnapshot,
}

impl DebugRequestJob {
    fn task(&self) -> &AnalysisTask {
        match self {
            Self::Hover(job) => &job.task,
            Self::Completion(job) => &job.task,
        }
    }

    fn scheduled_at(&self) -> Instant {
        match self {
            Self::Hover(job) => job.scheduled_at,
            Self::Completion(job) => job.scheduled_at,
        }
    }

    fn execute(self) -> ServerEvent {
        match self {
            Self::Hover(job) => {
                let report = debug_hover_report_for_cached_analysis_with_external_indexes(
                    job.task.snapshot().text(),
                    &job.analysis,
                    &job.uri,
                    job.position,
                    job.external_snapshot.workspace.as_deref(),
                    job.external_snapshot.game_data.as_deref(),
                    Some(&job.external_status),
                );
                let hit = report.contains("Selected Symbol: yes");
                let label = selected_label_from_debug_report(&report)
                    .unwrap_or_else(|| "<none>".to_string());
                ServerEvent::DebugRequestReady {
                    task: job.task.identity().clone(),
                    id: job.id,
                    method: DEBUG_HOVER_METHOD,
                    uri: job.uri,
                    revision: job.revision,
                    details: format!("cached_analysis=true hit={} label={}", hit, label),
                    result: Value::String(report),
                    elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                }
            }
            Self::Completion(job) => {
                let report = completion_report_for_cached_analysis_with_external_indexes(
                    job.task.snapshot().text(),
                    &job.analysis,
                    job.position,
                    job.external_snapshot.workspace.as_deref(),
                    job.external_snapshot.game_data.as_deref(),
                );
                if job.task.is_cancelled() {
                    return ServerEvent::DebugRequestReady {
                        task: job.task.identity().clone(),
                        id: job.id,
                        method: DEBUG_COMPLETION_METHOD,
                        uri: job.uri,
                        revision: job.revision,
                        details: "cancelled-after-completion-report".to_string(),
                        result: Value::Null,
                        elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                    };
                }
                let signature_report =
                    signature_help_report_for_cached_analysis_with_external_indexes(
                        job.task.snapshot().text(),
                        &job.analysis,
                        job.position,
                        job.external_snapshot.workspace.as_deref(),
                        job.external_snapshot.game_data.as_deref(),
                    );
                let completion_context = report.completion_context.clone();
                let candidate_count = report.candidate_count;
                let signature_context = signature_report
                    .context
                    .clone()
                    .unwrap_or_else(|| "none".to_string());
                let signature_candidate_count = signature_report.candidate_count;
                let mut markdown = completion_debug_markdown(
                    &report,
                    &job.uri,
                    job.task.snapshot().text().len(),
                    job.revision,
                    job.external_snapshot.status,
                );
                markdown.push_str(&signature_help_debug_markdown(&signature_report));
                ServerEvent::DebugRequestReady {
                    task: job.task.identity().clone(), id: job.id, method: DEBUG_COMPLETION_METHOD,
                    uri: job.uri, revision: job.revision,
                    details: format!("cached_analysis=true context={} candidates={} signature_context={} signature_candidates={} external_index_status={} external_index_layers={}", completion_context, candidate_count, signature_context, signature_candidate_count, job.external_snapshot.status, job.external_snapshot.available_layers()),
                    result: Value::String(markdown), elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                }
            }
        }
    }
}

#[cfg(test)]
fn earliest_due_pending_uri(pending: &BTreeMap<String, RichSemanticTokensJob>) -> Option<String> {
    pending
        .iter()
        .min_by_key(|(uri, job)| (job.scheduled_at, *uri))
        .map(|(uri, _)| uri.clone())
}

#[cfg(test)]
fn coalesce_rich_job(
    pending: &mut BTreeMap<String, RichSemanticTokensJob>,
    job: RichSemanticTokensJob,
) {
    if let Some(previous) = pending.insert(job.uri.clone(), job) {
        previous.task.cancel();
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
#[serde(rename_all = "camelCase")]
struct EnterTypingAssistParams {
    text_document: TextDocumentIdentifier,
    position: LspPosition,
    ch: String,
    /// VS Code captures this at the Enter edit. It is used solely to reject a
    /// stale editor result before any typing-assist edit is planned.
    version: i32,
    options: BlockCommentPairOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockCommentPairParams {
    text_document: TextDocumentIdentifier,
    position: LspPosition,
    version: i32,
    options: BlockCommentPairOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockCommentPairOptions {
    tab_size: usize,
    insert_spaces: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RangeFormattingParams {
    text_document: TextDocumentIdentifier,
    range: LspRange,
    #[serde(rename = "options")]
    _options: Value,
}

#[derive(Debug, Deserialize)]
struct TextDocumentIdentifier {
    uri: String,
}

impl<W: Write> LspServer<W> {
    fn new(writer: W, options: LspServerOptions) -> Self {
        Self::new_with_runtime_senders(writer, options, None, None, None)
    }

    fn new_with_runtime_senders(
        writer: W,
        options: LspServerOptions,
        _removed_rich_scheduler: Option<()>,
        analysis_scheduler: Option<RuntimeWorkExecutor>,
        _internal_sender: Option<mpsc::Sender<ServerEvent>>,
    ) -> Self {
        let logger = LspLogger::new(
            options.log_path.clone(),
            options.diagnostic_log_path.clone(),
        );
        let external_index = start_external_index(&options, logger.clone());
        let server = Self {
            writer,
            logger,
            external_index,
            document_runtime: DocumentRuntime::new(analysis_scheduler),
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
        server.logger.diagnostic(
            "startup",
            json!({
                "gameDataConfigured": options.game_data_scripts.is_some(),
                "workspaceRoots": options.workspace_scripts.len(),
                "indexCacheConfigured": options.index_cache.is_some(),
            }),
        );
        server
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
        self.logger
            .diagnostic("shutdown", json!({"outcome": "normal"}));
        self.logger.flush_diagnostics();
        Ok(())
    }

    fn log(&self, message: &str) {
        self.logger.log(message);
    }
}

pub fn document_symbols_for_source(source: &str) -> Vec<LspDocumentSymbol> {
    document_symbol_report_for_source(source).symbols
}

/// Returns only declarations whose identity is proven by the current lexer
/// snapshot. This is the pending-analysis outline contract: it is exact for
/// the returned top-level class, enum, and typedef names, but deliberately
/// omits members and declarations that need syntax recovery or semantic facts.
/// It must never reuse an earlier revision's cached projection.
fn lexical_document_symbols_for_snapshot(
    snapshot: &crate::analysis_runtime::DocumentSnapshot,
) -> Vec<LspDocumentSymbol> {
    snapshot.positions().map_or_else(Vec::new, |positions| {
        lexical_document_symbols(snapshot.text(), &positions)
    })
}

fn lexical_document_symbols(source: &str, positions: &PositionIndex) -> Vec<LspDocumentSymbol> {
    let tokens = lex(source);
    let mut symbols = Vec::new();
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
                    lexical_outline_declaration(&tokens, index, declaration_kind, source)
                {
                    let kind = match declaration_kind {
                        TokenKind::Keyword(Keyword::Class) => 5,
                        TokenKind::Keyword(Keyword::Enum) => 10,
                        TokenKind::Keyword(Keyword::Typedef) => 26,
                        _ => unreachable!("only outline declaration keywords reach this branch"),
                    };
                    let range = lsp_range_for_span(
                        positions,
                        TextSpan::new(token.span.start, name_token.span.end),
                    );
                    let selection_range = lsp_range_for_span(positions, name_token.span);
                    symbols.push(LspDocumentSymbol {
                        name,
                        detail: None,
                        kind,
                        range,
                        selection_range,
                        children: Vec::new(),
                    });
                    index = next_index;
                    continue;
                }
            }
            _ => {}
        }
        index += 1;
    }

    symbols
}

fn lexical_outline_declaration(
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

pub fn document_symbol_report_for_source(source: &str) -> LspDocumentSymbolReport {
    let analysis = file_index_for_source(source);
    document_symbol_report_for_cached_analysis(source, &analysis)
}

fn document_symbol_report_for_cached_analysis(
    source: &str,
    analysis: &FileIndexAnalysis,
) -> LspDocumentSymbolReport {
    debug_assert_eq!(
        analysis.semantic.declarations().len(),
        analysis.index.symbols().len(),
        "the local index must remain a complete projection of SemanticFile"
    );
    let query = IndexQuery::new(&analysis.index);
    let positions = LspPositionIndex::new(source);
    LspDocumentSymbolReport {
        symbols: document_symbols_from_index(&positions, &analysis.index, &query),
        parse_diagnostics: analysis.parse_diagnostics,
    }
}

fn lsp_range_for_span(positions: &PositionIndex, span: TextSpan) -> LspRange {
    let start = positions.position_for_offset(span.start);
    let end = positions.position_for_offset(span.end);
    LspRange {
        start: LspPosition {
            line: start.line,
            character: start.character,
        },
        end: LspPosition {
            line: end.line,
            character: end.character,
        },
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
    positions: PositionIndex,
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
        PositionIndex::new_cancellable(source, should_cancel).map(|positions| Self { positions })
    }

    pub(crate) fn position_for_offset(&self, offset: usize) -> LspPosition {
        let position = self.positions.position_for_offset(offset);
        LspPosition {
            line: position.line,
            character: position.character,
        }
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
        ENTER_TYPING_ASSIST_METHOD => validate_params::<EnterTypingAssistParams>(params, method),
        BLOCK_COMMENT_PAIR_METHOD => validate_params::<BlockCommentPairParams>(params, method),
        RANGE_FORMATTING_METHOD => validate_params::<RangeFormattingParams>(params, method),
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

#[cfg(test)]
mod tests;
