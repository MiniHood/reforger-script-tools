use super::request_router::{RequestCommand, RoutedRequest};
#[cfg(test)]
use super::semantic_tokens::{
    fast_semantic_tokens_for_cached_analysis,
    semantic_tokens_for_cached_analysis_with_external_indexes, LspSemanticTokenProjection,
};
use super::{
    clear_diagnostics_message, document_symbol_count, document_symbols_from_cached_analysis,
    file_index_for_source_with_timings, generic_angle_offsets_for_delimiters, lex,
    lexical_semantic_tokens_for_source_with_bracket_coloring, parse_source,
    publish_diagnostics_message, request_document_uri,
    semantic_tokens_for_cached_analysis_with_external_indexes_and_bracket_coloring,
    AdmissionDisposition, AnalysisTask, BracketColoringMode, DebugRequestJob,
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, DocumentQuery, ExternalIndexSnapshot,
    FileIndexAnalysis, FileIndexAnalysisTimings, ForegroundDocumentJob, LspSemanticTokensFull,
    OpenDocument, OpenDocumentAnalysisJob, PositionIndex, RichSemanticTokensJob, RpcMessage,
    RuntimeEffect, RuntimeWorkExecutor, ServerEvent, TaskClass, TokenProjectionKind,
    TokenResultDisposition, MAX_PENDING_DOCUMENT_REQUESTS_PER_URI,
};
use crate::analysis_runtime::{AdmissionLimits, AnalysisRuntime, UpsertOutcome};
use serde_json::Value;
use std::collections::BTreeMap;
#[cfg(test)]
use std::sync::atomic::AtomicBool;
#[cfg(test)]
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Owns all mutable state whose lifetime is bounded by open documents and
/// their admitted analysis. Transport and external-index ownership stay in
/// the LSP composition root.
pub(super) struct DocumentRuntime {
    documents: BTreeMap<String, OpenDocument>,
    runtime: AnalysisRuntime,
    analysis_scheduler: Option<RuntimeWorkExecutor>,
    deferred_document_requests: BTreeMap<String, Vec<DeferredDocumentRequest>>,
    bracket_coloring: BracketColoringMode,
    next_server_request_id: u64,
    semantic_tokens_refresh_in_flight: Option<String>,
    semantic_tokens_refresh_dirty: bool,
    last_semantic_external_generation: u64,
}

/// The runtime-owned result of selecting a semantic-token projection for one
/// snapshot. Feature dispatch sees this immutable report; it never reaches
/// into document cache or admission state.
pub(super) struct SemanticTokensSelection {
    pub(super) tokens: LspSemanticTokensFull,
    pub(super) uri: String,
    pub(super) bytes: usize,
    pub(super) revision: u64,
    pub(super) token_count: usize,
    pub(super) parse_diagnostics: usize,
    pub(super) projection_mode: &'static str,
    pub(super) lex_ms: u128,
    pub(super) resolver_ms: u128,
    pub(super) resolver_calls: usize,
    pub(super) token_loop_ms: u128,
    pub(super) encode_ms: u128,
    pub(super) rich_work: Option<(String, u64, u64)>,
}

#[cfg(test)]
pub(super) struct DocumentRuntimeTestState {
    pub(super) revision: u64,
    pub(super) version: i32,
    pub(super) text: String,
    pub(super) foreground_ready: bool,
    pub(super) analysis_ready: bool,
    pub(super) rich_semantic_tokens: bool,
}

impl DocumentRuntime {
    #[cfg(test)]
    pub(super) fn new(analysis_scheduler: Option<RuntimeWorkExecutor>) -> Self {
        Self::new_with_bracket_coloring(analysis_scheduler, BracketColoringMode::Semantic)
    }

    pub(super) fn new_with_bracket_coloring(
        analysis_scheduler: Option<RuntimeWorkExecutor>,
        bracket_coloring: BracketColoringMode,
    ) -> Self {
        Self {
            documents: BTreeMap::new(),
            runtime: AnalysisRuntime::new(AdmissionLimits::new(64, 64 * 1024 * 1024)),
            analysis_scheduler,
            deferred_document_requests: BTreeMap::new(),
            bracket_coloring,
            next_server_request_id: 1,
            semantic_tokens_refresh_in_flight: None,
            semantic_tokens_refresh_dirty: false,
            last_semantic_external_generation: 0,
        }
    }

    #[cfg(test)]
    pub(super) fn test_document_state(&self, uri: &str) -> Option<DocumentRuntimeTestState> {
        self.documents
            .get(uri)
            .map(|document| DocumentRuntimeTestState {
                revision: document.revision,
                version: document.version,
                text: document.text.to_string(),
                foreground_ready: document.foreground_ready(),
                analysis_ready: document.analysis_ready(),
                rich_semantic_tokens: document
                    .semantic_tokens
                    .has_rich_for_revision(document.revision),
            })
    }

    #[cfg(test)]
    pub(super) fn test_has_any_foreground_document(&self) -> bool {
        self.documents.values().any(OpenDocument::foreground_ready)
    }

    #[cfg(test)]
    pub(super) fn test_admit_task(&mut self, uri: &str, class: TaskClass) -> AnalysisTask {
        let snapshot = self.runtime.latest(uri).expect("accepted snapshot");
        match self.runtime.admit(class, snapshot, 1, Instant::now()) {
            AdmissionDisposition::Enqueued { .. } => self.runtime.take_next().unwrap(),
            other => panic!("unexpected admission disposition: {other:?}"),
        }
    }

    #[cfg(test)]
    pub(super) fn test_prepare_rich_event(
        &mut self,
        uri: &str,
        external_generation: u64,
    ) -> (
        AnalysisTask,
        u64,
        LspSemanticTokenProjection,
        Arc<AtomicBool>,
    ) {
        let task = self.test_admit_task(uri, TaskClass::Rich);
        let document = self.documents.get_mut(uri).expect("open document");
        let cancel = task.cancellation_token();
        document.semantic_tokens.mark_pending(
            document.revision,
            external_generation,
            cancel.clone(),
        );
        (
            task,
            document.revision,
            fast_semantic_tokens_for_cached_analysis(&document.text, document.analysis()),
            cancel,
        )
    }

    #[cfg(test)]
    pub(super) fn test_install_current_foreground(&mut self, uri: &str) -> bool {
        let snapshot = self.runtime.latest(uri).expect("accepted snapshot");
        let document = self.documents.get_mut(uri).expect("open document");
        document.replace(snapshot.clone());
        document.install_foreground(
            snapshot.revision(),
            PositionIndex::new(snapshot.text()),
            lex(snapshot.text()),
            parse_source(snapshot.text()),
        )
    }

    /// Captures the coherent input for one document-backed feature request.
    pub(super) fn capture_query<'a>(
        &'a self,
        uri: &str,
        external_indexes: ExternalIndexSnapshot,
    ) -> Option<DocumentQuery<'a>> {
        Some(DocumentQuery {
            document: self.documents.get(uri)?,
            external_indexes,
        })
    }

    pub(super) fn select_semantic_tokens(
        &mut self,
        uri: &str,
        external_generation: u64,
    ) -> SemanticTokensSelection {
        let mut selection = SemanticTokensSelection {
            tokens: LspSemanticTokensFull {
                result_id: "reforger:missing:lexical".to_string(),
                data: Vec::new(),
            },
            uri: uri.to_string(),
            bytes: 0,
            revision: 0,
            token_count: 0,
            parse_diagnostics: 0,
            projection_mode: "missing-document",
            lex_ms: 0,
            resolver_ms: 0,
            resolver_calls: 0,
            token_loop_ms: 0,
            encode_ms: 0,
            rich_work: None,
        };
        let Some(document) = self.documents.get_mut(uri) else {
            return selection;
        };
        selection.bytes = document.text.len();
        selection.revision = document.revision;
        let source = document.text.clone();
        let generic_angle_offsets = document
            .foreground()
            .map(|foreground| {
                generic_angle_offsets_for_delimiters(&source, foreground.scope_delimiters())
            })
            .unwrap_or_default();
        let (kind, result_id, disposition, projection) = {
            let selected = document.semantic_tokens.select_or_insert_lexical(
                document.revision,
                external_generation,
                || {
                    lexical_semantic_tokens_for_source_with_bracket_coloring(
                        &source,
                        self.bracket_coloring,
                        &generic_angle_offsets,
                    )
                },
            );
            (
                selected.kind,
                selected.result_id,
                selected.disposition,
                selected.projection.clone(),
            )
        };
        selection.tokens = LspSemanticTokensFull::from_tokens(result_id, &projection.tokens);
        selection.projection_mode = match kind {
            TokenProjectionKind::LexicalBaseline if document.analysis_ready() => "lexical-baseline",
            TokenProjectionKind::LexicalBaseline => "lexical-pending",
            TokenProjectionKind::RichOverlay => "rich-overlay",
        };
        debug_assert_eq!(disposition, TokenResultDisposition::Full);
        if kind == TokenProjectionKind::LexicalBaseline
            && document.analysis_ready()
            && !document
                .semantic_tokens
                .pending_for_revision_and_external_generation(
                    document.revision,
                    external_generation,
                )
        {
            selection.rich_work = Some((uri.to_string(), document.revision, external_generation));
        }
        selection.token_count = projection.token_count;
        selection.parse_diagnostics = projection.parse_diagnostics;
        selection.lex_ms = projection.timings.lex_ms;
        selection.resolver_ms = projection.timings.resolver_ms;
        selection.resolver_calls = projection.timings.identifier_resolver_calls;
        selection.token_loop_ms = projection.timings.token_loop_ms;
        selection.encode_ms = projection.timings.encode_ms;
        selection
    }

    /// Closes a document and turns every transport-visible consequence into
    /// effects for the composition root. No coordinator outside this runtime
    /// may retain deferred work for the closed snapshot.
    pub(super) fn close_document(&mut self, uri: &str) -> Vec<RuntimeEffect> {
        let mut effects = Vec::new();
        if let Some(mut document) = self.documents.remove(uri) {
            document.semantic_tokens.cancel_pending();
            self.runtime.close(uri, document.snapshot.revision());
        }
        if let Some(pending) = self.deferred_document_requests.remove(uri) {
            for request in pending {
                if let Some(id) = request.routed.message.id {
                    effects.push(RuntimeEffect::Error {
                        id,
                        code: -32801,
                        message: "Content modified".to_string(),
                    });
                }
            }
        }
        effects.push(RuntimeEffect::Notification(clear_diagnostics_message(uri)));
        effects.push(RuntimeEffect::Log(format!(
            "notification didClose uri={uri}"
        )));
        effects
    }

    pub(super) fn open_document(
        &mut self,
        params: DidOpenTextDocumentParams,
        queue_ms: u128,
    ) -> Result<Vec<RuntimeEffect>, String> {
        let start = Instant::now();
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let text = params.text_document.text;
        let bytes = text.len();
        let mut effects = Vec::new();
        if let Some(mut previous) = self.documents.remove(&uri) {
            previous.semantic_tokens.cancel_pending();
            self.runtime.close(&uri, previous.snapshot.revision());
        }
        let UpsertOutcome::Accepted = self.runtime.upsert(uri.clone(), version, text) else {
            return Err(format!(
                "didOpen could not install document snapshot for {uri}"
            ));
        };
        let snapshot = self.runtime.latest(&uri).expect("accepted snapshot");
        let revision = snapshot.revision();
        self.discard_deferred_document_requests_for_revision(&uri, revision, &mut effects)?;
        if let Some(scheduler) = self.analysis_scheduler.clone() {
            let mut document = OpenDocument::pending(snapshot);
            document.mark_analysis_pending();
            self.documents.insert(uri.clone(), document);
            self.admit_foreground(&uri, revision, scheduler);
            effects.push(RuntimeEffect::Log(format!(
                "notification didOpen uri={} bytes={} version={} revision={} foreground_state=pending analysis_state=waiting-foreground queue_ms={} analysis_elapsed_ms={}",
                uri, bytes, version, revision, queue_ms, start.elapsed().as_millis()
            )));
        } else {
            let mut document = OpenDocument::new(snapshot);
            let symbol_start = Instant::now();
            let symbols = document_symbols_from_cached_analysis(
                document.snapshot.text(),
                document.analysis(),
            );
            let document_symbol_ms = symbol_start.elapsed().as_millis();
            let symbol_count = document_symbol_count(&symbols);
            document.set_document_symbols(symbols);
            let parse_diagnostics = document.parse_diagnostic_count();
            let analysis_timings = document.analysis_timings();
            let diagnostics = document
                .syntax()
                .expect("ready test document has syntax")
                .diagnostics
                .clone();
            let source = document.snapshot.text().to_string();
            self.documents.insert(uri.clone(), document);
            effects.push(RuntimeEffect::Notification(publish_diagnostics_message(
                &uri,
                version,
                &source,
                &diagnostics,
            )));
            effects.push(RuntimeEffect::Log(format!(
                "notification didOpen uri={} bytes={} version={} revision={} cached_analysis=true document_symbols_cached=true symbols={} parse_diagnostics={} analysis_parse_ms={} analysis_catalog_ms={} analysis_index_ms={} analysis_scope_ms={} analysis_build_ms={} document_symbol_ms={} queue_ms={} analysis_elapsed_ms={}",
                uri, bytes, version, revision, symbol_count, parse_diagnostics,
                analysis_timings.parse_ms, analysis_timings.catalog_ms, analysis_timings.index_ms,
                analysis_timings.scope_ms, analysis_timings.total_ms, document_symbol_ms, queue_ms,
                start.elapsed().as_millis()
            )));
        }
        Ok(effects)
    }

    pub(super) fn change_document(
        &mut self,
        params: DidChangeTextDocumentParams,
        queue_ms: u128,
        coalesced_changes: usize,
        superseded_changes: usize,
    ) -> Result<Vec<RuntimeEffect>, String> {
        let Some(change) = params.content_changes.into_iter().last() else {
            return Ok(Vec::new());
        };
        let start = Instant::now();
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let text = change.text;
        let bytes = text.len();
        let mut effects = Vec::new();
        let Some(current_version) = self.runtime.latest(&uri).map(|snapshot| snapshot.version())
        else {
            effects.push(RuntimeEffect::Log(format!(
                "notification didChange ignored uri={} version={} reason=not_open",
                uri, version
            )));
            return Ok(effects);
        };
        if version <= current_version
            || !matches!(
                self.runtime.upsert(uri.clone(), version, text),
                UpsertOutcome::Accepted
            )
        {
            effects.push(RuntimeEffect::Log(format!(
                "notification didChange ignored uri={} version={} current_version={} reason=stale",
                uri, version, current_version
            )));
            return Ok(effects);
        }
        let snapshot = self.runtime.latest(&uri).expect("accepted snapshot");
        let revision = {
            let document = self
                .documents
                .get_mut(&uri)
                .expect("open document exists after version check");
            document.replace(snapshot);
            document.mark_analysis_pending();
            document.snapshot.revision()
        };
        if let Some(scheduler) = self.analysis_scheduler.clone() {
            self.discard_deferred_document_requests_for_revision(&uri, revision, &mut effects)?;
            self.admit_foreground(&uri, revision, scheduler);
            effects.push(RuntimeEffect::Log(format!(
                "notification didChange uri={} bytes={} version={} revision={} foreground_state=pending analysis_state=waiting-foreground queue_ms={} coalesced_changes={} superseded_changes={} analysis_elapsed_ms={}",
                uri, bytes, version, revision, queue_ms, coalesced_changes, superseded_changes, start.elapsed().as_millis()
            )));
        } else {
            let snapshot = self.runtime.latest(&uri).expect("accepted snapshot");
            let document = self.documents.get_mut(&uri).expect("open document exists");
            assert!(document.install_foreground(
                revision,
                PositionIndex::new(snapshot.text()),
                lex(snapshot.text()),
                parse_source(snapshot.text())
            ));
            let diagnostics = document
                .syntax()
                .expect("foreground installation supplies syntax")
                .diagnostics
                .clone();
            let source = snapshot.text().to_string();
            effects.push(RuntimeEffect::Notification(publish_diagnostics_message(
                &uri,
                version,
                &source,
                &diagnostics,
            )));
            let (analysis, timings) = file_index_for_source_with_timings(snapshot.text());
            effects.push(RuntimeEffect::Log(format!(
                "notification didChange uri={} bytes={} version={} revision={} cached_analysis=true document_symbols_cached=false symbols=pending parse_diagnostics={} analysis_parse_ms={} analysis_catalog_ms={} analysis_index_ms={} analysis_scope_ms={} analysis_build_ms={} queue_ms={} coalesced_changes={} superseded_changes={} analysis_elapsed_ms={}",
                uri, bytes, version, revision, analysis.parse_diagnostics, timings.parse_ms,
                timings.catalog_ms, timings.index_ms, timings.scope_ms, timings.total_ms,
                queue_ms, coalesced_changes, superseded_changes, start.elapsed().as_millis()
            )));
            self.install_analysis_synchronously(&uri, revision, analysis, timings);
        }
        Ok(effects)
    }

    fn install_analysis_synchronously(
        &mut self,
        uri: &str,
        revision: u64,
        analysis: FileIndexAnalysis,
        timings: FileIndexAnalysisTimings,
    ) {
        let Some(document) = self.documents.get_mut(uri) else {
            return;
        };
        let _ = document.install_analysis(revision, analysis, timings);
    }

    fn admit_foreground(&mut self, uri: &str, revision: u64, scheduler: RuntimeWorkExecutor) {
        let request_id = self.next_server_request_id;
        self.next_server_request_id += 1;
        let snapshot = self.runtime.latest(uri).expect("accepted snapshot");
        match self.runtime.admit(
            TaskClass::Foreground,
            snapshot,
            request_id,
            Instant::now() + Duration::from_secs(30),
        ) {
            AdmissionDisposition::Enqueued { .. } => {
                scheduler.schedule_foreground(ForegroundDocumentJob {
                    task: self
                        .runtime
                        .take_next()
                        .expect("admitted foreground task is runnable"),
                    scheduled_at: Instant::now(),
                })
            }
            AdmissionDisposition::DroppedOverload { .. } => self
                .documents
                .get_mut(uri)
                .expect("pending document exists")
                .reject_pending_analysis(),
        }
        let _ = revision;
    }

    fn discard_deferred_document_requests_for_revision(
        &mut self,
        uri: &str,
        current_revision: u64,
        effects: &mut Vec<RuntimeEffect>,
    ) -> Result<(), String> {
        let Some(pending) = self.deferred_document_requests.remove(uri) else {
            return Ok(());
        };
        for request in pending {
            if let Some(id) = request.routed.message.id {
                effects.push(RuntimeEffect::Error {
                    id,
                    code: -32801,
                    message: "Content modified".to_string(),
                });
            }
        }
        effects.push(RuntimeEffect::Log(format!(
            "request deferred discarded uri={} current_revision={} reason=superseded",
            uri, current_revision
        )));
        Ok(())
    }

    /// Interprets completion events whose only observable outcome is a
    /// request response. Freshness belongs to the runtime that admitted the
    /// task; the composition root only delivers the returned effects.
    pub(super) fn interpret_debug_event(
        &mut self,
        event: ServerEvent,
    ) -> Option<Vec<RuntimeEffect>> {
        let ServerEvent::DebugRequestReady {
            task,
            id,
            method,
            uri,
            revision,
            details,
            result,
            elapsed_ms,
        } = event
        else {
            return None;
        };
        if !self.runtime.complete(&task) {
            return Some(vec![
                RuntimeEffect::Log(format!(
                    "request {} discarded uri={} revision={} reason=runtime-stale async=true elapsed_ms={}",
                    method, uri, revision, elapsed_ms
                )),
                RuntimeEffect::Error { id, code: -32801, message: "Content modified".to_string() },
            ]);
        }
        Some(vec![
            RuntimeEffect::Log(format!(
                "request {} uri={} revision={} {} async=true elapsed_ms={}",
                method, uri, revision, details, elapsed_ms
            )),
            RuntimeEffect::Response { id, result },
        ])
    }

    pub(super) fn interpret_foreground_event(
        &mut self,
        event: ServerEvent,
    ) -> Option<Vec<RuntimeEffect>> {
        let ServerEvent::ForegroundDocumentReady {
            task,
            positions,
            lexer_tokens,
            syntax,
            elapsed_ms,
        } = event
        else {
            return None;
        };
        if !self.runtime.complete(&task) {
            return Some(vec![RuntimeEffect::Log(format!(
                "foreground discarded uri={} revision={} reason=runtime-stale elapsed_ms={}",
                task.uri(),
                task.revision(),
                elapsed_ms
            ))]);
        }
        let Some(document) = self.documents.get_mut(task.uri()) else {
            return Some(Vec::new());
        };
        if !document.install_foreground(task.revision(), positions, lexer_tokens, syntax) {
            return Some(vec![RuntimeEffect::Log(format!(
                "foreground discarded uri={} revision={} reason=stale-install elapsed_ms={}",
                task.uri(),
                task.revision(),
                elapsed_ms
            ))]);
        }
        let uri = task.uri().to_string();
        let version = document.version;
        let revision = document.revision;
        let diagnostics = document
            .syntax()
            .expect("foreground installation supplies syntax")
            .diagnostics
            .clone();
        let source = document.snapshot.text().to_string();
        let _ = document;
        self.admit_semantic_after_foreground_runtime(&uri, revision);
        Some(vec![
            RuntimeEffect::Notification(publish_diagnostics_message(&uri, version, &source, &diagnostics)),
            RuntimeEffect::Log(format!(
                "foreground ready uri={} version={} revision={} lexical_state=ready syntax_state=ready elapsed_ms={}",
                uri, version, revision, elapsed_ms
            )),
        ])
    }

    fn admit_semantic_after_foreground_runtime(&mut self, uri: &str, revision: u64) {
        let Some(scheduler) = self.analysis_scheduler.clone() else {
            return;
        };
        let Some(document) = self.documents.get(uri) else {
            return;
        };
        if document.revision != revision || !document.foreground_ready() {
            return;
        }
        let snapshot = document.snapshot.clone();
        let request_id = self.next_server_request_id;
        self.next_server_request_id += 1;
        match self.runtime.admit(
            TaskClass::Semantic,
            snapshot,
            request_id,
            Instant::now() + Duration::from_secs(30),
        ) {
            AdmissionDisposition::Enqueued { .. } => scheduler.schedule(OpenDocumentAnalysisJob {
                task: self
                    .runtime
                    .take_next()
                    .expect("foreground dependency admits a runnable semantic task"),
                scheduled_at: Instant::now(),
            }),
            AdmissionDisposition::DroppedOverload { .. } => {
                if let Some(document) = self.documents.get_mut(uri) {
                    if document.revision == revision {
                        document.reject_pending_analysis();
                    }
                }
            }
        }
    }

    pub(super) fn interpret_analysis_event(
        &mut self,
        event: ServerEvent,
        external_indexes: ExternalIndexSnapshot,
        external_generation: u64,
    ) -> Option<Result<Vec<RuntimeEffect>, String>> {
        match event {
            ServerEvent::DocumentAnalysisReady {
                task,
                analysis,
                timings,
                elapsed_ms,
            } => {
                if !self.runtime.complete(&task) {
                    return Some(Ok(vec![RuntimeEffect::Log(format!(
                        "documentAnalysis discarded uri={} revision={} reason=runtime-stale elapsed_ms={}",
                        task.uri(), task.revision(), elapsed_ms
                    ))]));
                }
                let uri = task.uri().to_string();
                let revision = task.revision();
                let Some(document) = self.documents.get_mut(&uri) else {
                    return Some(Ok(Vec::new()));
                };
                if !document.install_analysis(revision, analysis, timings) {
                    return Some(Ok(Vec::new()));
                }
                let pending = self
                    .deferred_document_requests
                    .remove(&uri)
                    .unwrap_or_default();
                let mut effects = Vec::new();
                for request in pending {
                    if request.revision == revision {
                        effects.push(RuntimeEffect::ReplayDeferred {
                            routed: request.routed,
                            queue_ms: request.received_at.elapsed().as_millis(),
                        });
                    } else if let Some(id) = request.routed.message.id {
                        effects.push(RuntimeEffect::Error {
                            id,
                            code: -32801,
                            message: "Content modified".to_string(),
                        });
                    }
                }
                effects.extend(self.admit_rich_semantic_tokens(
                    &uri,
                    revision,
                    external_indexes,
                    external_generation,
                ));
                effects.push(RuntimeEffect::Log(format!(
                    "documentAnalysis ready uri={} revision={} elapsed_ms={}",
                    uri, revision, elapsed_ms
                )));
                Some(Ok(effects))
            }
            ServerEvent::DocumentAnalysisSkipped {
                task,
                reason,
                elapsed_ms,
            } => {
                let current = self.runtime.complete(&task);
                let mut effects = Vec::new();
                if reason == "scheduler-capacity-evicted" && current {
                    if let Some(document) = self.documents.get_mut(task.uri()) {
                        if document.revision == task.revision() && !document.analysis_ready() {
                            document.reject_pending_analysis();
                            if let Err(error) = self
                                .discard_deferred_document_requests_for_revision(
                                    task.uri(),
                                    task.revision(),
                                    &mut effects,
                                )
                            {
                                return Some(Err(error));
                            }
                        }
                    }
                }
                effects.push(RuntimeEffect::Log(format!(
                    "documentAnalysis skipped uri={} revision={} reason={} elapsed_ms={}",
                    task.uri(),
                    task.revision(),
                    reason,
                    elapsed_ms
                )));
                Some(Ok(effects))
            }
            _ => None,
        }
    }

    pub(super) fn interpret_rich_skipped_event(
        &mut self,
        event: ServerEvent,
    ) -> Option<Vec<RuntimeEffect>> {
        let ServerEvent::RichSemanticTokensSkipped {
            task,
            uri,
            revision,
            external_generation,
            reason,
            elapsed_ms,
        } = event
        else {
            return None;
        };
        self.runtime.complete(&task);
        if let Some(document) = self.documents.get_mut(&uri) {
            document
                .semantic_tokens
                .cancel_pending_if_matches(revision, external_generation);
        }
        let effects = vec![RuntimeEffect::Log(format!(
            "semanticTokensRich skipped uri={} revision={} external_generation={} reason={} elapsed_ms={}",
            uri, revision, external_generation, reason, elapsed_ms
        ))];
        Some(effects)
    }

    /// Applies a completed rich-token projection. The caller supplies only
    /// the external generation it captured at the composition boundary; all
    /// document freshness and refresh coalescing remain owned here.
    pub(super) fn interpret_rich_ready_event(
        &mut self,
        event: ServerEvent,
        current_external_generation: u64,
    ) -> Option<Vec<RuntimeEffect>> {
        let ServerEvent::RichSemanticTokensReady {
            task,
            uri,
            revision,
            external_generation,
            external_status,
            projection,
            elapsed_ms,
        } = event
        else {
            return None;
        };
        if !self.runtime.complete(&task) {
            return Some(vec![RuntimeEffect::Log(format!(
                "semanticTokensRich discarded uri={} revision={} reason=runtime-stale elapsed_ms={}",
                uri, revision, elapsed_ms
            ))]);
        }
        let Some(document) = self.documents.get_mut(&uri) else {
            return Some(vec![RuntimeEffect::Log(format!(
                "semanticTokensRich discarded uri={} revision={} reason=missing-document elapsed_ms={}",
                uri, revision, elapsed_ms
            ))]);
        };
        if document.revision != revision {
            return Some(vec![RuntimeEffect::Log(format!(
                "semanticTokensRich discarded uri={} revision={} current_revision={} reason=stale-revision elapsed_ms={}",
                uri, revision, document.revision, elapsed_ms
            ))]);
        }
        if current_external_generation != external_generation {
            return Some(vec![RuntimeEffect::Log(format!(
                "semanticTokensRich discarded uri={} revision={} external_generation={} current_external_generation={} reason=stale-external-index elapsed_ms={}",
                uri, revision, external_generation, current_external_generation, elapsed_ms
            ))]);
        }
        let token_count = projection.token_count;
        let parse_diagnostics = projection.parse_diagnostics;
        let timings = projection.timings.clone();
        document
            .semantic_tokens
            .set_rich(revision, external_generation, projection);
        let mut effects = vec![RuntimeEffect::Log(format!(
            "semanticTokensRich ready uri={} revision={} external_generation={} tokens={} external_index_status={} parse_diagnostics={} lex_ms={} token_loop_ms={} resolver_ms={} resolver_calls={} encode_ms={} elapsed_ms={}",
            uri, revision, external_generation, token_count, external_status, parse_diagnostics,
            timings.lex_ms, timings.token_loop_ms, timings.resolver_ms,
            timings.identifier_resolver_calls, timings.encode_ms, elapsed_ms
        ))];
        self.request_semantic_tokens_refresh_effect(&mut effects);
        Some(effects)
    }

    pub(super) fn request_semantic_tokens_refresh_effect(
        &mut self,
        effects: &mut Vec<RuntimeEffect>,
    ) {
        if self.semantic_tokens_refresh_in_flight.is_some() {
            self.semantic_tokens_refresh_dirty = true;
            return;
        }
        let id = format!("server-{}", self.next_server_request_id);
        self.next_server_request_id += 1;
        self.semantic_tokens_refresh_in_flight = Some(id.clone());
        effects.push(RuntimeEffect::Log(format!(
            "request workspace/semanticTokens/refresh id={id}"
        )));
        effects.push(RuntimeEffect::RequestSemanticTokensRefresh { id });
    }

    pub(super) fn acknowledge_semantic_tokens_refresh(
        &mut self,
        message: &RpcMessage,
    ) -> Vec<RuntimeEffect> {
        let Some(id) = message.id.as_ref().and_then(Value::as_str) else {
            return Vec::new();
        };
        if self.semantic_tokens_refresh_in_flight.as_deref() != Some(id) {
            return Vec::new();
        }
        self.semantic_tokens_refresh_in_flight = None;
        let mut effects = Vec::new();
        if self.semantic_tokens_refresh_dirty {
            self.semantic_tokens_refresh_dirty = false;
            self.request_semantic_tokens_refresh_effect(&mut effects);
        }
        effects
    }

    pub(super) fn observe_semantic_external_generation(
        &mut self,
        generation: u64,
        status: &'static str,
    ) -> Vec<RuntimeEffect> {
        if self.documents.is_empty() {
            self.last_semantic_external_generation = generation;
            return Vec::new();
        }
        if generation == self.last_semantic_external_generation {
            return Vec::new();
        }
        self.last_semantic_external_generation = generation;
        for document in self.documents.values_mut() {
            document
                .semantic_tokens
                .cancel_pending_for_other_external_generation(generation);
            document
                .semantic_tokens
                .discard_rich_for_other_external_generation(generation);
        }
        let mut effects = vec![RuntimeEffect::Log(format!(
            "semanticTokens external overlay changed generation={} status={} documents={} requesting_refresh=true",
            generation, status, self.documents.len()
        ))];
        self.request_semantic_tokens_refresh_effect(&mut effects);
        effects
    }

    pub(super) fn admit_rich_semantic_tokens(
        &mut self,
        uri: &str,
        revision: u64,
        external_indexes: ExternalIndexSnapshot,
        generation: u64,
    ) -> Vec<RuntimeEffect> {
        let start = Instant::now();
        let Some(document) = self.documents.get(uri) else {
            return vec![RuntimeEffect::Log(format!(
                "semanticTokensRich skipped uri={} revision={} reason=missing-document-before-schedule elapsed_ms={}", uri, revision, start.elapsed().as_millis()
            ))];
        };
        if document.revision != revision
            || !document
                .semantic_tokens
                .needs_rich_projection(revision, generation)
        {
            return Vec::new();
        }
        let analysis = document.analysis().clone();
        let snapshot = self
            .runtime
            .latest(uri)
            .expect("open document has a runtime snapshot");
        let request_id = self.next_server_request_id;
        self.next_server_request_id += 1;
        let task = match self.runtime.admit(
            TaskClass::Rich,
            snapshot,
            request_id,
            Instant::now() + Duration::from_secs(30),
        ) {
            AdmissionDisposition::Enqueued { .. } => self
                .runtime
                .take_next()
                .expect("admitted rich task is runnable"),
            AdmissionDisposition::DroppedOverload {
                retained_jobs,
                retained_bytes,
                ..
            } => {
                return vec![RuntimeEffect::Log(format!(
                    "semanticTokensRich skipped uri={} revision={} external_generation={} reason=runtime-overload retained_jobs={} retained_bytes={}",
                    uri, revision, generation, retained_jobs, retained_bytes
                ))];
            }
        };
        self.documents
            .get_mut(uri)
            .expect("document remains present for admitted rich task")
            .semantic_tokens
            .mark_pending(revision, generation, task.cancellation_token());
        let Some(scheduler) = self.analysis_scheduler.as_ref() else {
            let projection =
                semantic_tokens_for_cached_analysis_with_external_indexes_and_bracket_coloring(
                    task.snapshot().text(),
                    &analysis,
                    external_indexes.workspace.as_deref(),
                    external_indexes.game_data.as_deref(),
                    self.bracket_coloring,
                );
            return self
                .interpret_rich_ready_event(
                    ServerEvent::RichSemanticTokensReady {
                        task: task.identity().clone(),
                        uri: uri.to_string(),
                        revision,
                        external_generation: generation,
                        external_status: external_indexes.status,
                        projection,
                        elapsed_ms: start.elapsed().as_millis(),
                    },
                    generation,
                )
                .expect("constructed rich event is handled");
        };
        scheduler.schedule_rich(RichSemanticTokensJob {
            task,
            uri: uri.to_string(),
            revision,
            external_generation: generation,
            scheduled_at: start,
            analysis,
            external_snapshot: external_indexes,
            bracket_coloring: self.bracket_coloring,
        });
        Vec::new()
    }

    pub(super) fn defer_document_request(
        &mut self,
        message: &super::RpcMessage,
        command: RequestCommand,
        parameter_error: Option<String>,
    ) -> Result<(bool, Vec<RuntimeEffect>), String> {
        let Some(uri) = request_document_uri(message.params.as_ref()) else {
            return Ok((false, Vec::new()));
        };
        let Some(document) = self.documents.get(&uri) else {
            return Ok((false, Vec::new()));
        };
        if document.analysis_ready() {
            return Ok((false, Vec::new()));
        }
        let revision = document.revision;
        let mut effects = Vec::new();
        if document.analysis_rejected() {
            if let Some(id) = message.id.clone() {
                effects.push(RuntimeEffect::Error {
                    id,
                    code: -32801,
                    message: "Content modified".to_string(),
                });
            }
            effects.push(RuntimeEffect::Log(format!(
                "request deferred rejected uri={} revision={} reason=analysis-overload",
                uri, revision
            )));
            return Ok((true, effects));
        }
        let pending = self
            .deferred_document_requests
            .entry(uri.clone())
            .or_default();
        if pending.len() >= MAX_PENDING_DOCUMENT_REQUESTS_PER_URI {
            if let Some(id) = message.id.clone() {
                effects.push(RuntimeEffect::Error {
                    id,
                    code: -32801,
                    message: "Content modified".to_string(),
                });
            }
            effects.push(RuntimeEffect::Log(format!(
                "request deferred rejected uri={} revision={} reason=capacity",
                uri, revision
            )));
            return Ok((true, effects));
        }
        pending.push(DeferredDocumentRequest {
            revision,
            received_at: Instant::now(),
            routed: RoutedRequest {
                command,
                message: message.clone(),
                parameter_error,
            },
        });
        effects.push(RuntimeEffect::Log(format!(
            "request deferred uri={} revision={} pending_requests={}",
            uri,
            revision,
            pending.len()
        )));
        Ok((true, effects))
    }

    /// Admits a debug capture on the runtime's rich lane. The returned task
    /// identity remains the sole authority for its worker result.
    pub(super) fn admit_debug_capture(
        &mut self,
        uri: &str,
    ) -> Result<AnalysisTask, (usize, usize)> {
        let request_id = self.next_server_request_id;
        self.next_server_request_id += 1;
        let snapshot = self.runtime.latest(uri).expect("current debug snapshot");
        match self.runtime.admit(
            TaskClass::Rich,
            snapshot,
            request_id,
            Instant::now() + Duration::from_secs(30),
        ) {
            AdmissionDisposition::Enqueued { .. } => Ok(self
                .runtime
                .take_next()
                .expect("admitted debug task is runnable")),
            AdmissionDisposition::DroppedOverload {
                retained_jobs,
                retained_bytes,
                ..
            } => Err((retained_jobs, retained_bytes)),
        }
    }

    pub(super) fn has_runtime_worker(&self) -> bool {
        self.analysis_scheduler.is_some()
    }

    pub(super) fn schedule_debug(&self, job: DebugRequestJob) {
        self.analysis_scheduler
            .as_ref()
            .expect("debug work is admitted only when the runtime worker exists")
            .schedule_debug(job);
    }

    /// The single owner for worker completion interpretation. The composition
    /// root supplies the external generation it captured, then only delivers
    /// the effects produced here.
    pub(super) fn interpret_event(
        &mut self,
        event: ServerEvent,
        current_external_generation: u64,
        external_indexes: ExternalIndexSnapshot,
    ) -> Option<Result<Vec<RuntimeEffect>, String>> {
        match event {
            ServerEvent::Incoming { .. } => None,
            event @ ServerEvent::RichSemanticTokensReady { .. } => Some(Ok(
                self.interpret_rich_ready_event(event, current_external_generation)?
            )),
            event @ ServerEvent::RichSemanticTokensSkipped { .. } => {
                Some(Ok(self.interpret_rich_skipped_event(event)?))
            }
            event @ ServerEvent::ForegroundDocumentReady { .. } => {
                Some(Ok(self.interpret_foreground_event(event)?))
            }
            event @ ServerEvent::DebugRequestReady { .. } => {
                Some(Ok(self.interpret_debug_event(event)?))
            }
            ServerEvent::ForegroundDocumentSkipped {
                task,
                reason,
                elapsed_ms,
            } => {
                let current = self.runtime.complete(&task);
                if current {
                    if let Some(document) = self.documents.get_mut(task.uri()) {
                        if document.revision == task.revision() {
                            document.reject_pending_analysis();
                        }
                    }
                }
                Some(Ok(vec![RuntimeEffect::Log(format!(
                    "foreground skipped uri={} revision={} reason={} elapsed_ms={}",
                    task.uri(),
                    task.revision(),
                    reason,
                    elapsed_ms
                ))]))
            }
            event @ ServerEvent::DocumentAnalysisReady { .. }
            | event @ ServerEvent::DocumentAnalysisSkipped { .. } => {
                self.interpret_analysis_event(event, external_indexes, current_external_generation)
            }
        }
    }
}

pub(super) struct DeferredDocumentRequest {
    pub(super) revision: u64,
    pub(super) received_at: Instant,
    pub(super) routed: RoutedRequest,
}

#[cfg(test)]
mod tests {
    use super::{
        semantic_tokens_for_cached_analysis_with_external_indexes, AdmissionDisposition,
        BracketColoringMode, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
        DocumentRuntime, ExternalIndexSnapshot, OpenDocument, RpcMessage, RuntimeEffect,
        ServerEvent, TaskClass,
    };
    use crate::analysis_runtime::UpsertOutcome;
    use serde_json::json;
    use std::time::{Duration, Instant};

    #[test]
    fn captured_query_pairs_the_open_snapshot_with_the_supplied_external_snapshot() {
        let mut runtime = DocumentRuntime::new(None);
        let uri = "file:///query.c";
        assert_eq!(
            runtime.runtime.upsert(uri, 7, "class Query {}".to_string()),
            UpsertOutcome::Accepted
        );
        let snapshot = runtime.runtime.latest(uri).expect("accepted snapshot");
        runtime
            .documents
            .insert(uri.to_string(), OpenDocument::new(snapshot));
        let query = runtime
            .capture_query(
                uri,
                ExternalIndexSnapshot {
                    status: "missing",
                    workspace: None,
                    game_data: None,
                },
            )
            .expect("open document query");

        assert_eq!(query.document.version, 7);
        assert_eq!(query.document.snapshot.text(), "class Query {}");
        assert_eq!(query.external_indexes.status, "missing");
    }

    #[test]
    fn rich_ready_emits_refresh_effect_and_coalesces_a_follow_up_until_acknowledged() {
        let mut runtime = DocumentRuntime::new(None);
        let uri = "file:///rich-ready.c";
        assert_eq!(
            runtime.runtime.upsert(uri, 1, "class Ready {}".to_string()),
            UpsertOutcome::Accepted
        );
        let snapshot = runtime.runtime.latest(uri).expect("accepted snapshot");
        runtime
            .documents
            .insert(uri.to_string(), OpenDocument::new(snapshot.clone()));
        let task = match runtime.runtime.admit(
            TaskClass::Rich,
            snapshot,
            1,
            Instant::now() + Duration::from_secs(1),
        ) {
            AdmissionDisposition::Enqueued { .. } => runtime.runtime.take_next().unwrap(),
            other => panic!("unexpected admission disposition: {other:?}"),
        };
        let projection = {
            let document = runtime.documents.get(uri).unwrap();
            semantic_tokens_for_cached_analysis_with_external_indexes(
                &document.text,
                document.analysis(),
                None,
                None,
            )
        };
        let effects = runtime
            .interpret_rich_ready_event(
                ServerEvent::RichSemanticTokensReady {
                    task: task.identity().clone(),
                    uri: uri.to_string(),
                    revision: 1,
                    external_generation: 0,
                    external_status: "missing",
                    projection,
                    elapsed_ms: 0,
                },
                0,
            )
            .expect("rich event belongs to the document runtime");
        assert!(effects.iter().any(|effect| matches!(
            effect,
            RuntimeEffect::RequestSemanticTokensRefresh { id } if id == "server-1"
        )));

        let mut follow_up = Vec::new();
        runtime.request_semantic_tokens_refresh_effect(&mut follow_up);
        assert!(follow_up.is_empty());
        assert!(runtime.semantic_tokens_refresh_dirty);

        let ack: RpcMessage = serde_json::from_value(json!({
            "jsonrpc": "2.0", "id": "server-1", "result": null
        }))
        .unwrap();
        let effects = runtime.acknowledge_semantic_tokens_refresh(&ack);
        assert!(effects.iter().any(|effect| matches!(
            effect,
            RuntimeEffect::RequestSemanticTokensRefresh { id } if id == "server-2"
        )));
    }

    #[test]
    fn closing_a_document_cancels_its_snapshot_and_emits_only_transport_effects() {
        let mut runtime = DocumentRuntime::new(None);
        let uri = "file:///closed.c";
        assert_eq!(
            runtime
                .runtime
                .upsert(uri, 1, "class Closed {}".to_string()),
            UpsertOutcome::Accepted
        );
        let snapshot = runtime.runtime.latest(uri).expect("accepted snapshot");
        runtime
            .documents
            .insert(uri.to_string(), OpenDocument::new(snapshot));

        let effects = runtime.close_document(uri);

        assert!(!runtime.documents.contains_key(uri));
        assert!(runtime.runtime.latest(uri).is_none());
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, RuntimeEffect::Notification(_))));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, RuntimeEffect::Log(_))));
    }

    #[test]
    fn open_and_change_own_snapshot_replacement_and_emit_delivery_effects() {
        let mut runtime = DocumentRuntime::new(None);
        let uri = "file:///runtime.c".to_string();
        let opened = runtime
            .open_document(
                DidOpenTextDocumentParams {
                    text_document: super::super::TextDocumentItem {
                        uri: uri.clone(),
                        version: 1,
                        text: "class Before {}".to_string(),
                    },
                },
                0,
            )
            .expect("open succeeds");
        assert!(opened
            .iter()
            .any(|effect| matches!(effect, RuntimeEffect::Notification(_))));

        let changed = runtime
            .change_document(
                DidChangeTextDocumentParams {
                    text_document: super::super::VersionedTextDocumentIdentifier {
                        uri: uri.clone(),
                        version: 2,
                    },
                    content_changes: vec![super::super::TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: "class After {}".to_string(),
                    }],
                },
                0,
                0,
                0,
            )
            .expect("change succeeds");

        assert_eq!(runtime.documents[&uri].version, 2);
        assert_eq!(runtime.documents[&uri].snapshot.text(), "class After {}");
        assert!(runtime.documents[&uri].analysis_ready());
        assert!(changed
            .iter()
            .any(|effect| matches!(effect, RuntimeEffect::Notification(_))));
    }

    #[test]
    fn changed_document_returns_current_lexical_tokens_while_rich_tokens_refresh() {
        let mut runtime = DocumentRuntime::new(None);
        let uri = "file:///delimiter-typing.c".to_string();
        runtime
            .open_document(
                DidOpenTextDocumentParams {
                    text_document: super::super::TextDocumentItem {
                        uri: uri.clone(),
                        version: 1,
                        text: "class Example { void Run() { Invoke(); } }".to_string(),
                    },
                },
                0,
            )
            .expect("open succeeds");

        let initial = runtime.select_semantic_tokens(&uri, 0);
        let (_, revision, generation) = initial.rich_work.expect("rich work");
        runtime.admit_rich_semantic_tokens(
            &uri,
            revision,
            ExternalIndexSnapshot {
                status: "missing",
                workspace: None,
                game_data: None,
            },
            generation,
        );
        assert!(runtime
            .select_semantic_tokens(&uri, 0)
            .tokens
            .result_id
            .contains(":rich:"));

        runtime
            .change_document(
                DidChangeTextDocumentParams {
                    text_document: super::super::VersionedTextDocumentIdentifier {
                        uri: uri.clone(),
                        version: 2,
                    },
                    content_changes: vec![super::super::TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: "class Example { void Run() { Invoke(worl); } }".to_string(),
                    }],
                },
                0,
                0,
                0,
            )
            .expect("change succeeds");

        let changed = runtime.select_semantic_tokens(&uri, 0);
        assert_eq!(changed.tokens.result_id, "reforger:2:lexical");
        assert!(changed.rich_work.is_some());
    }

    #[test]
    fn current_foreground_generic_facts_drive_the_first_punctuation_projection() {
        let mut runtime =
            DocumentRuntime::new_with_bracket_coloring(None, BracketColoringMode::Punctuation);
        let uri = "file:///punctuation-baseline.c".to_string();
        let source = "class Example { array<int>> value; }";
        runtime
            .open_document(
                DidOpenTextDocumentParams {
                    text_document: super::super::TextDocumentItem {
                        uri: uri.clone(),
                        version: 1,
                        text: source.to_string(),
                    },
                },
                0,
            )
            .expect("open succeeds");

        let selection = runtime.select_semantic_tokens(&uri, 0);
        let closers = source.find(">>").unwrap();
        let punctuation_type = super::super::SEMANTIC_TOKEN_TYPES
            .iter()
            .position(|token_type| *token_type == "reforgerPunctuation")
            .unwrap() as u32;
        let operator_type = super::super::SEMANTIC_TOKEN_TYPES
            .iter()
            .position(|token_type| *token_type == "operator")
            .unwrap() as u32;

        assert_eq!(selection.tokens.result_id, "reforger:1:lexical");
        assert_eq!(
            semantic_token_type_at_offset(&selection.tokens.data, closers),
            Some(punctuation_type),
        );
        assert_eq!(
            semantic_token_type_at_offset(&selection.tokens.data, closers + 1),
            Some(operator_type),
        );
    }

    #[test]
    fn current_foreground_generic_facts_do_not_remove_semantic_baseline_operators() {
        let mut runtime =
            DocumentRuntime::new_with_bracket_coloring(None, BracketColoringMode::Semantic);
        let uri = "file:///semantic-baseline.c".to_string();
        let source = "class Example { array<int> value; }";
        runtime
            .open_document(
                DidOpenTextDocumentParams {
                    text_document: super::super::TextDocumentItem {
                        uri: uri.clone(),
                        version: 1,
                        text: source.to_string(),
                    },
                },
                0,
            )
            .expect("open succeeds");

        let selection = runtime.select_semantic_tokens(&uri, 0);
        let generic_open = source.find('<').unwrap();
        let operator_type = super::super::SEMANTIC_TOKEN_TYPES
            .iter()
            .position(|token_type| *token_type == "operator")
            .unwrap() as u32;

        assert_eq!(selection.tokens.result_id, "reforger:1:lexical");
        assert_eq!(
            semantic_token_type_at_offset(&selection.tokens.data, generic_open),
            Some(operator_type),
        );
    }

    fn semantic_token_type_at_offset(data: &[u32], offset: usize) -> Option<u32> {
        let mut line = 0usize;
        let mut character = 0usize;
        for token in data.chunks_exact(5) {
            line += token[0] as usize;
            character = if token[0] == 0 {
                character + token[1] as usize
            } else {
                token[1] as usize
            };
            if line == 0 && character <= offset && offset < character + token[2] as usize {
                return Some(token[3]);
            }
        }
        None
    }
}
