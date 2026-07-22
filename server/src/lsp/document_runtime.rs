use super::{
    clear_diagnostics_message, document_symbol_count, document_symbols_from_cached_analysis,
    file_index_for_source_with_timings, lex, parse_source, publish_diagnostics_message,
    request_document_uri, semantic_tokens_for_cached_analysis_with_external_indexes,
    AdmissionDisposition, AnalysisTask, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DocumentQuery, ExternalIndexSnapshot, FileIndexAnalysis, FileIndexAnalysisTimings,
    ForegroundDocumentJob, LspSemanticTokenProjection, LspSemanticTokensFull, LspServer,
    OpenDocument, OpenDocumentAnalysisJob, PositionIndex, RichSemanticTokensJob, RpcMessage,
    RuntimeEffect, RuntimeWorkExecutor, ServerEvent, TaskClass,
    MAX_PENDING_DOCUMENT_REQUESTS_PER_URI,
};
use crate::analysis_runtime::{AdmissionLimits, AnalysisRuntime, UpsertOutcome};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::time::{Duration, Instant};

/// Owns all mutable state whose lifetime is bounded by open documents and
/// their admitted analysis. Transport and external-index ownership stay in
/// the LSP composition root.
pub(super) struct DocumentRuntime {
    pub(super) documents: BTreeMap<String, OpenDocument>,
    pub(super) runtime: AnalysisRuntime,
    pub(super) analysis_scheduler: Option<RuntimeWorkExecutor>,
    pub(super) deferred_document_requests: BTreeMap<String, Vec<DeferredDocumentRequest>>,
    pub(super) deferred_semantic_token_requests:
        BTreeMap<String, Vec<DeferredSemanticTokenRequest>>,
    pub(super) next_server_request_id: u64,
    pub(super) semantic_tokens_refresh_in_flight: Option<String>,
    pub(super) semantic_tokens_refresh_dirty: bool,
    pub(super) last_semantic_external_generation: u64,
}

impl DocumentRuntime {
    pub(super) fn new(analysis_scheduler: Option<RuntimeWorkExecutor>) -> Self {
        Self {
            documents: BTreeMap::new(),
            runtime: AnalysisRuntime::new(AdmissionLimits::new(64, 64 * 1024 * 1024)),
            analysis_scheduler,
            deferred_document_requests: BTreeMap::new(),
            deferred_semantic_token_requests: BTreeMap::new(),
            next_server_request_id: 1,
            semantic_tokens_refresh_in_flight: None,
            semantic_tokens_refresh_dirty: false,
            last_semantic_external_generation: 0,
        }
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
                if let Ok(message) = serde_json::from_value::<RpcMessage>(request.value) {
                    if let Some(id) = message.id {
                        effects.push(RuntimeEffect::Error {
                            id,
                            code: -32801,
                            message: "Content modified".to_string(),
                        });
                    }
                }
            }
        }
        if let Some(pending) = self.deferred_semantic_token_requests.remove(uri) {
            for request in pending {
                effects.push(RuntimeEffect::Error {
                    id: request.id,
                    code: -32802,
                    message: "Semantic tokens superseded".to_string(),
                });
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
        self.discard_deferred_semantic_requests_for_revision(
            &uri,
            revision,
            "opened",
            &mut effects,
        );

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
        self.discard_deferred_semantic_requests_for_revision(
            &uri,
            revision,
            "superseded",
            &mut effects,
        );
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
            let message: RpcMessage = serde_json::from_value(request.value)
                .map_err(|error| format!("Invalid deferred JSON-RPC message: {error}"))?;
            if let Some(id) = message.id {
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

    fn discard_deferred_semantic_requests_for_revision(
        &mut self,
        uri: &str,
        current_revision: u64,
        reason: &str,
        effects: &mut Vec<RuntimeEffect>,
    ) {
        let Some(pending) = self.deferred_semantic_token_requests.remove(uri) else {
            return;
        };
        for request in pending {
            effects.push(RuntimeEffect::Error {
                id: request.id,
                code: -32802,
                message: "Semantic tokens superseded".to_string(),
            });
        }
        effects.push(RuntimeEffect::Log(format!("semanticTokens deferred discarded uri={} current_revision={} reason={} outcome=server-cancelled", uri, current_revision, reason)));
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
                            value: request.value,
                            queue_ms: request.received_at.elapsed().as_millis(),
                        });
                    } else if let Ok(message) = serde_json::from_value::<RpcMessage>(request.value)
                    {
                        if let Some(id) = message.id {
                            effects.push(RuntimeEffect::Error {
                                id,
                                code: -32801,
                                message: "Content modified".to_string(),
                            });
                        }
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
                            self.discard_deferred_semantic_requests_for_revision(
                                task.uri(),
                                task.revision(),
                                "analysis-skipped",
                                &mut effects,
                            );
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
        let mut effects = Vec::new();
        self.discard_deferred_semantic_requests_for_revision(
            &uri,
            revision,
            "rich-skipped",
            &mut effects,
        );
        effects.push(RuntimeEffect::Log(format!(
            "semanticTokensRich skipped uri={} revision={} external_generation={} reason={} elapsed_ms={}",
            uri, revision, external_generation, reason, elapsed_ms
        )));
        Some(effects)
    }

    /// Applies a completed rich-token projection. The caller supplies only
    /// the external generation it captured at the composition boundary; all
    /// document freshness, deferred response policy, and refresh coalescing
    /// remain owned here.
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
        let published = self.publish_deferred_semantic_token_effects(
            &uri,
            revision,
            external_generation,
            &mut effects,
        );
        if published == 0 {
            self.request_semantic_tokens_refresh_effect(&mut effects);
        } else {
            effects.push(RuntimeEffect::Log(format!(
                "semanticTokensRich delivered uri={} revision={} external_generation={} deferred_requests={} refresh=false",
                uri, revision, external_generation, published
            )));
        }
        Some(effects)
    }

    pub(super) fn publish_deferred_semantic_token_effects(
        &mut self,
        uri: &str,
        revision: u64,
        external_generation: u64,
        effects: &mut Vec<RuntimeEffect>,
    ) -> usize {
        let Some(projection) = self
            .documents
            .get(uri)
            .and_then(|document| {
                document
                    .semantic_tokens
                    .rich_for_revision_and_external_generation(revision, external_generation)
            })
            .cloned()
        else {
            return 0;
        };
        let pending = self
            .deferred_semantic_token_requests
            .remove(uri)
            .unwrap_or_default();
        let mut published = 0;
        for request in pending {
            if request.revision == revision && request.external_generation == external_generation {
                let result = serde_json::to_value(LspSemanticTokensFull::from_tokens(
                    format!("reforger:{}:rich:{}", revision, external_generation),
                    &projection.tokens,
                ))
                .unwrap_or(Value::Null);
                effects.push(RuntimeEffect::Response {
                    id: request.id,
                    result,
                });
                effects.push(RuntimeEffect::Log(format!(
                    "semanticTokens deferred published uri={} revision={} external_generation={} wait_ms={}",
                    uri, revision, external_generation, request.received_at.elapsed().as_millis()
                )));
                published += 1;
            } else {
                effects.push(RuntimeEffect::Error {
                    id: request.id,
                    code: -32802,
                    message: "Semantic tokens superseded".to_string(),
                });
            }
        }
        published
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
        let Some(scheduler) = self.analysis_scheduler.clone() else {
            let projection = semantic_tokens_for_cached_analysis_with_external_indexes(
                task.snapshot().text(),
                &analysis,
                external_indexes.workspace.as_deref(),
                external_indexes.game_data.as_deref(),
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
        vec![RuntimeEffect::ScheduleRich {
            scheduler,
            job: RichSemanticTokensJob {
                task,
                uri: uri.to_string(),
                revision,
                external_generation: generation,
                scheduled_at: start,
                analysis,
                external_snapshot: external_indexes,
            },
        }]
    }

    pub(super) fn defer_document_request(
        &mut self,
        message: &RpcMessage,
        value: Value,
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
            value,
        });
        effects.push(RuntimeEffect::Log(format!(
            "request deferred uri={} revision={} pending_requests={}",
            uri,
            revision,
            pending.len()
        )));
        Ok((true, effects))
    }

    pub(super) fn defer_semantic_token_request(
        &mut self,
        uri: &str,
        revision: u64,
        external_generation: u64,
        id: Value,
    ) -> Vec<RuntimeEffect> {
        let pending = self
            .deferred_semantic_token_requests
            .entry(uri.to_string())
            .or_default();
        if pending.len() >= MAX_PENDING_DOCUMENT_REQUESTS_PER_URI {
            return vec![
                RuntimeEffect::Error { id, code: -32802, message: "Semantic tokens superseded".to_string() },
                RuntimeEffect::Log(format!(
                    "semanticTokens deferred uri={} revision={} external_generation={} outcome=server-cancelled reason=capacity pending_requests={}",
                    uri, revision, external_generation, pending.len()
                )),
            ];
        }
        pending.push(DeferredSemanticTokenRequest {
            id,
            revision,
            external_generation,
            received_at: Instant::now(),
        });
        vec![RuntimeEffect::Log(format!(
            "semanticTokens deferred uri={} revision={} external_generation={} pending_requests={}",
            uri,
            revision,
            external_generation,
            pending.len()
        ))]
    }

    pub(super) fn discard_deferred_semantic_token_requests(
        &mut self,
        uri: &str,
        current_revision: u64,
        reason: &str,
    ) -> Vec<RuntimeEffect> {
        let mut effects = Vec::new();
        self.discard_deferred_semantic_requests_for_revision(
            uri,
            current_revision,
            reason,
            &mut effects,
        );
        effects
    }

    pub(super) fn cancel_deferred_semantic_token_request(
        &mut self,
        id: &Value,
    ) -> Vec<RuntimeEffect> {
        let mut cancellations = Vec::new();
        self.deferred_semantic_token_requests
            .retain(|uri, pending| {
                let before = pending.len();
                pending.retain(|request| &request.id != id);
                let removed = before - pending.len();
                if removed > 0 {
                    cancellations.push((uri.clone(), removed));
                }
                !pending.is_empty()
            });
        if cancellations.is_empty() {
            return vec![RuntimeEffect::Log(
                "semanticTokens deferred cancellation ignored reason=not-pending".to_string(),
            )];
        }
        cancellations
            .into_iter()
            .map(|(uri, removed)| {
                RuntimeEffect::Log(format!(
                    "semanticTokens deferred cancelled uri={} requests={}",
                    uri, removed
                ))
            })
            .collect()
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
    pub(super) value: Value,
}

/// Full semantic-token responses replace the editor's entire token display.
/// Keep these requests apart from source-backed feature deferral so a newer
/// revision can wait for a matching rich projection without publishing a
/// lexical downgrade.
pub(super) struct DeferredSemanticTokenRequest {
    pub(super) id: Value,
    pub(super) revision: u64,
    pub(super) external_generation: u64,
    pub(super) received_at: Instant,
}

impl<W: Write> LspServer<W> {
    pub(super) fn defer_request_while_document_analysis_is_pending(
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
        let analysis_rejected = document.analysis_rejected();
        if analysis_rejected {
            if let Some(id) = message.id.clone() {
                self.respond_error(id, -32801, "Content modified")?;
            }
            self.log(&format!(
                "request deferred rejected uri={} revision={} reason=analysis-overload",
                uri, revision
            ));
            return Ok(true);
        }
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

    pub(super) fn discard_deferred_document_requests(
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

    pub(super) fn defer_semantic_token_request(
        &mut self,
        uri: &str,
        revision: u64,
        external_generation: u64,
        id: Value,
    ) -> Result<(), String> {
        let pending_count = self
            .deferred_semantic_token_requests
            .get(uri)
            .map_or(0, Vec::len);
        if pending_count >= MAX_PENDING_DOCUMENT_REQUESTS_PER_URI {
            self.respond_error(id, -32802, "Semantic tokens superseded")?;
            self.log(&format!(
                "semanticTokens deferred uri={} revision={} external_generation={} outcome=server-cancelled reason=capacity pending_requests={}",
                uri, revision, external_generation, pending_count
            ));
            return Ok(());
        }
        let pending_count = {
            let pending = self
                .deferred_semantic_token_requests
                .entry(uri.to_string())
                .or_default();
            pending.push(DeferredSemanticTokenRequest {
                id,
                revision,
                external_generation,
                received_at: Instant::now(),
            });
            pending.len()
        };
        self.log(&format!(
            "semanticTokens deferred uri={} revision={} external_generation={} pending_requests={}",
            uri, revision, external_generation, pending_count
        ));
        Ok(())
    }

    pub(super) fn discard_deferred_semantic_token_requests(
        &mut self,
        uri: &str,
        current_revision: u64,
        reason: &str,
    ) -> Result<(), String> {
        let Some(pending) = self.deferred_semantic_token_requests.remove(uri) else {
            return Ok(());
        };
        for request in pending {
            self.respond_error(request.id, -32802, "Semantic tokens superseded")?;
        }
        self.log(&format!(
            "semanticTokens deferred discarded uri={} current_revision={} reason={} outcome=server-cancelled",
            uri, current_revision, reason
        ));
        Ok(())
    }

    pub(super) fn cancel_deferred_semantic_token_request(&mut self, id: &Value) {
        let mut cancellations = Vec::new();
        self.deferred_semantic_token_requests
            .retain(|uri, pending| {
                let before = pending.len();
                pending.retain(|request| &request.id != id);
                let removed = before - pending.len();
                if removed > 0 {
                    cancellations.push((uri.clone(), removed));
                }
                !pending.is_empty()
            });
        if cancellations.is_empty() {
            self.log("semanticTokens deferred cancellation ignored reason=not-pending");
        } else {
            for (uri, removed) in cancellations {
                self.log(&format!(
                    "semanticTokens deferred cancelled uri={} requests={}",
                    uri, removed
                ));
            }
        }
    }

    pub(super) fn install_document_analysis(
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
        let parse_diagnostics = document.analysis().parse_diagnostics;
        let analysis_timings = document.analysis_timings();
        let _ = document;
        self.log(&format!(
            "documentAnalysis ready uri={} bytes={} version={} revision={} cached_analysis=true semantic_idle_delay_ms=0 parse_diagnostics={} analysis_parse_ms={} analysis_catalog_ms={} analysis_index_ms={} analysis_scope_ms={} analysis_build_ms={} elapsed_ms={}",
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

    pub(super) fn schedule_rich_semantic_tokens(
        &mut self,
        uri: &str,
        revision: u64,
        external_generation: u64,
    ) -> Result<(), String> {
        if let Some(scheduler) = self.analysis_scheduler.clone() {
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
            let analysis = document.analysis().clone();
            let _ = document;
            let snapshot = self.runtime.latest(uri).expect("current rich snapshot");
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
                    self.log(&format!(
                        "semanticTokensRich skipped uri={} revision={} external_generation={} reason=runtime-overload retained_jobs={} retained_bytes={}",
                        uri, revision, external_generation, retained_jobs, retained_bytes
                    ));
                    return Ok(());
                }
            };
            self.documents
                .get_mut(uri)
                .expect("document remains present for admitted rich task")
                .semantic_tokens
                .mark_pending(revision, external_generation, task.cancellation_token());
            let job = RichSemanticTokensJob {
                task,
                uri: uri.to_string(),
                revision,
                external_generation,
                scheduled_at: start,
                analysis,
                external_snapshot: self.external_index.snapshot(),
            };
            scheduler.schedule_rich(job);
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
        let mut effects = Vec::new();
        let published = self
            .document_runtime
            .publish_deferred_semantic_token_effects(
                uri,
                revision,
                external_generation,
                &mut effects,
            );
        if published == 0 {
            self.document_runtime
                .request_semantic_tokens_refresh_effect(&mut effects);
        } else {
            effects.push(RuntimeEffect::Log(format!(
                "semanticTokensRich delivered uri={} revision={} external_generation={} deferred_requests={} refresh=false",
                uri, revision, external_generation, published
            )));
        }
        for effect in effects {
            self.deliver_effect(effect)?;
        }
        Ok(())
    }

    /// Debug captures share the optional rich lane rather than creating a
    /// second background owner. The returned identity remains the only
    /// authority for responding to the request after worker execution.
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

    pub(super) fn rich_semantic_tokens_for_revision(
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
            document.analysis(),
            indexes.workspace.as_deref(),
            indexes.game_data.as_deref(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis_runtime::UpsertOutcome;
    use serde_json::json;

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
}
