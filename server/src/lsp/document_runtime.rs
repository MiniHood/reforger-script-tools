use super::{
    clear_diagnostics_message,
    request_document_uri, semantic_tokens_for_cached_analysis_with_external_indexes,
    AdmissionDisposition, AnalysisTask, DocumentQuery, ExternalIndexSnapshot, FileIndexAnalysis,
    FileIndexAnalysisTimings, LspSemanticTokenProjection, LspSemanticTokensFull, LspServer,
    OpenDocument, OpenDocumentAnalysisJob, RichSemanticTokensJob, RpcMessage, RuntimeWorkExecutor,
    RuntimeEffect, TaskClass, MAX_PENDING_DOCUMENT_REQUESTS_PER_URI,
};
use crate::analysis_runtime::{AdmissionLimits, AnalysisRuntime};
use serde_json::{json, Value};
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
        effects.push(RuntimeEffect::Log(format!("notification didClose uri={uri}")));
        effects
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

    pub(super) fn publish_deferred_semantic_token_requests(
        &mut self,
        uri: &str,
        revision: u64,
        external_generation: u64,
    ) -> Result<usize, String> {
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
            return Ok(0);
        };
        let pending = self
            .deferred_semantic_token_requests
            .remove(uri)
            .unwrap_or_default();
        let mut published = 0usize;
        for request in pending {
            if request.revision == revision && request.external_generation == external_generation {
                let result = serde_json::to_value(LspSemanticTokensFull::from_tokens(
                    format!("reforger:{}:rich:{}", revision, external_generation),
                    &projection.tokens,
                ))
                .unwrap_or(Value::Null);
                self.respond(request.id, result)?;
                published += 1;
                self.log(&format!(
                    "semanticTokens deferred published uri={} revision={} external_generation={} wait_ms={}",
                    uri,
                    revision,
                    external_generation,
                    request.received_at.elapsed().as_millis()
                ));
            } else {
                self.respond_error(request.id, -32802, "Semantic tokens superseded")?;
            }
        }
        Ok(published)
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

    /// Foreground publication is the only dependency edge into whole-file
    /// semantic work.  In particular, an accepted edit never admits semantic
    /// construction while its current syntax/position state is absent.
    pub(super) fn admit_semantic_after_foreground(&mut self, uri: &str, revision: u64) {
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
        let _ = document;
        let request_id = self.next_server_request_id;
        self.next_server_request_id += 1;
        match self.runtime.admit(
            TaskClass::Semantic,
            snapshot,
            request_id,
            Instant::now() + Duration::from_secs(30),
        ) {
            AdmissionDisposition::Enqueued { .. } => {
                let task = self
                    .runtime
                    .take_next()
                    .expect("foreground dependency admits a runnable semantic task");
                scheduler.schedule(OpenDocumentAnalysisJob {
                    task,
                    scheduled_at: Instant::now(),
                });
            }
            AdmissionDisposition::DroppedOverload { .. } => {
                if let Some(document) = self.documents.get_mut(uri) {
                    if document.revision == revision {
                        document.reject_pending_analysis();
                    }
                }
            }
        }
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
        let published =
            self.publish_deferred_semantic_token_requests(uri, revision, external_generation)?;
        if published == 0 {
            self.request_semantic_tokens_refresh()
        } else {
            self.log(&format!(
                "semanticTokensRich delivered uri={} revision={} external_generation={} deferred_requests={} refresh=false",
                uri, revision, external_generation, published
            ));
            Ok(())
        }
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

    pub(super) fn request_semantic_tokens_refresh(&mut self) -> Result<(), String> {
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

    pub(super) fn handle_semantic_tokens_refresh_response(
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

    pub(super) fn request_semantic_tokens_refresh_if_external_generation_changed(
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
            document
                .semantic_tokens
                .discard_rich_for_other_external_generation(status.generation);
        }
        self.log(&format!(
            "semanticTokens external overlay changed generation={} status={} documents={} requesting_refresh=true",
            status.generation,
            status.status,
            self.documents.len()
        ));
        self.request_semantic_tokens_refresh()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis_runtime::UpsertOutcome;

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
    fn closing_a_document_cancels_its_snapshot_and_emits_only_transport_effects() {
        let mut runtime = DocumentRuntime::new(None);
        let uri = "file:///closed.c";
        assert_eq!(
            runtime.runtime.upsert(uri, 1, "class Closed {}".to_string()),
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
        assert!(effects.iter().any(|effect| matches!(effect, RuntimeEffect::Log(_))));
    }
}
