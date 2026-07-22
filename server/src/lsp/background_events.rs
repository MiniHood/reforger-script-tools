use super::{LspServer, ServerEvent};
use std::io::Write;
/// Compatibility executor for worker completions. The composition root owns
/// this transport-facing remainder; document state transitions are being
/// moved into `DocumentRuntime` as their typed event contracts land.
impl<W: Write> LspServer<W> {
    pub(super) fn handle_background_event(&mut self, event: ServerEvent) -> Result<(), String> {
        let external_generation = self.external_index.status_summary().generation;
        if matches!(
            &event,
            ServerEvent::DocumentAnalysisReady { .. } | ServerEvent::DocumentAnalysisSkipped { .. }
        ) {
            let effects = self
                .document_runtime
                .interpret_analysis_event(event, external_generation)
                .expect("analysis event is handled by the document runtime")?;
            for effect in effects { self.deliver_effect(effect)?; }
            return Ok(());
        }
        if matches!(&event, ServerEvent::RichSemanticTokensSkipped { .. }) {
            let effects = self.document_runtime.interpret_rich_skipped_event(event)
                .expect("rich skipped event is handled by the document runtime");
            for effect in effects { self.deliver_effect(effect)?; }
            return Ok(());
        }
        match event {
            ServerEvent::Incoming { .. } => Ok(()),
            ServerEvent::RichSemanticTokensReady {
                task,
                uri,
                revision,
                external_generation,
                external_status,
                projection,
                elapsed_ms,
            } => {
                if !self.runtime.complete(&task) {
                    self.log(&format!(
                        "semanticTokensRich discarded uri={} revision={} reason=runtime-stale elapsed_ms={}",
                        uri, revision, elapsed_ms
                    ));
                    return Ok(());
                }
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
                let published = self.publish_deferred_semantic_token_requests(
                    &uri,
                    revision,
                    external_generation,
                )?;
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
            ServerEvent::RichSemanticTokensSkipped {
                task,
                uri,
                revision,
                external_generation,
                reason,
                elapsed_ms,
            } => {
                self.runtime.complete(&task);
                if let Some(document) = self.documents.get_mut(&uri) {
                    document
                        .semantic_tokens
                        .cancel_pending_if_matches(revision, external_generation);
                }
                self.discard_deferred_semantic_token_requests(&uri, revision, "rich-skipped")?;
                self.log(&format!(
                    "semanticTokensRich skipped uri={} revision={} external_generation={} reason={} elapsed_ms={}",
                    uri, revision, external_generation, reason, elapsed_ms
                ));
                Ok(())
            }
            ServerEvent::ForegroundDocumentReady {
                task,
                positions,
                lexer_tokens,
                syntax,
                elapsed_ms,
            } => {
                let effects = self.document_runtime.interpret_foreground_event(
                    ServerEvent::ForegroundDocumentReady { task, positions, lexer_tokens, syntax, elapsed_ms },
                ).expect("foreground event is handled by the document runtime");
                for effect in effects { self.deliver_effect(effect)?; }
                Ok(())
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
                self.log(&format!(
                    "foreground skipped uri={} revision={} reason={} elapsed_ms={}",
                    task.uri(),
                    task.revision(),
                    reason,
                    elapsed_ms
                ));
                Ok(())
            }
            ServerEvent::DocumentAnalysisReady {
                task,
                analysis,
                timings,
                elapsed_ms,
            } => {
                if self.runtime.complete(&task) {
                    let uri = task.uri().to_string();
                    let revision = task.revision();
                    self.install_document_analysis(
                        task.uri(),
                        revision,
                        analysis,
                        timings,
                        elapsed_ms,
                    )?;
                    let external_generation = self.external_index.status_summary().generation;
                    let should_schedule_rich = self.documents.get(&uri).is_some_and(|document| {
                        document
                            .semantic_tokens
                            .needs_rich_projection(revision, external_generation)
                    });
                    if should_schedule_rich {
                        self.schedule_rich_semantic_tokens(&uri, revision, external_generation)?;
                    }
                    Ok(())
                } else {
                    self.log(&format!(
                        "documentAnalysis discarded uri={} revision={} reason=runtime-stale elapsed_ms={}",
                        task.uri(), task.revision(), elapsed_ms
                    ));
                    Ok(())
                }
            }
            ServerEvent::DocumentAnalysisSkipped {
                task,
                reason,
                elapsed_ms,
            } => {
                let current = self.runtime.complete(&task);
                if reason == "scheduler-capacity-evicted" && current {
                    if let Some(document) = self.documents.get_mut(task.uri()) {
                        if document.revision == task.revision() && !document.analysis_ready() {
                            document.reject_pending_analysis();
                            self.discard_deferred_document_requests(task.uri(), task.revision())?;
                            self.discard_deferred_semantic_token_requests(
                                task.uri(),
                                task.revision(),
                                "analysis-skipped",
                            )?;
                        }
                    }
                }
                self.log(&format!(
                    "documentAnalysis skipped uri={} revision={} reason={} elapsed_ms={}",
                    task.uri(),
                    task.revision(),
                    reason,
                    elapsed_ms
                ));
                Ok(())
            }
            ServerEvent::DebugRequestReady {
                task,
                id,
                method,
                uri,
                revision,
                details,
                result,
                elapsed_ms,
            } => {
                let effects = self
                    .document_runtime
                    .interpret_debug_event(ServerEvent::DebugRequestReady {
                        task,
                        id,
                        method,
                        uri,
                        revision,
                        details,
                        result,
                        elapsed_ms,
                    })
                    .expect("debug event is handled by the document runtime");
                for effect in effects {
                    self.deliver_effect(effect)?;
                }
                Ok(())
            }
        }
    }
}
