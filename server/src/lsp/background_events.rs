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
            for effect in effects {
                self.deliver_effect(effect)?;
            }
            return Ok(());
        }
        if matches!(&event, ServerEvent::RichSemanticTokensSkipped { .. }) {
            let effects = self
                .document_runtime
                .interpret_rich_skipped_event(event)
                .expect("rich skipped event is handled by the document runtime");
            for effect in effects {
                self.deliver_effect(effect)?;
            }
            return Ok(());
        }
        match event {
            ServerEvent::Incoming { .. } => Ok(()),
            event @ ServerEvent::RichSemanticTokensReady { .. } => {
                let current_external_generation = self.external_index.status_summary().generation;
                let effects = self
                    .document_runtime
                    .interpret_rich_ready_event(event, current_external_generation)
                    .expect("rich ready event is handled by the document runtime");
                for effect in effects {
                    self.deliver_effect(effect)?;
                }
                Ok(())
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
                let effects = self
                    .document_runtime
                    .interpret_foreground_event(ServerEvent::ForegroundDocumentReady {
                        task,
                        positions,
                        lexer_tokens,
                        syntax,
                        elapsed_ms,
                    })
                    .expect("foreground event is handled by the document runtime");
                for effect in effects {
                    self.deliver_effect(effect)?;
                }
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
