use super::{
    clear_diagnostics_message, completion, completion_debug_markdown,
    completion_report_for_cached_analysis_with_external_indexes,
    completion_report_for_current_argument_labels_at_offset_with_external_indexes,
    completion_report_for_current_local_scope_at_offset_with_external_indexes,
    completion_report_for_current_override_at_offset_with_external_indexes,
    completion_report_for_current_receiver_at_offset_with_external_indexes,
    completion_report_for_lexical_source_at_offset_with_external_indexes,
    completion_report_for_lexical_source_with_external_indexes,
    debug_hover_report_for_cached_analysis_with_external_indexes,
    definition_report_for_cached_analysis_with_external_indexes,
    definition_report_for_pending_snapshot, document_symbol_count,
    document_symbols_from_cached_analysis, empty_completion_list,
    hover_report_for_cached_analysis_with_external_indexes, hover_report_for_pending_snapshot,
    lexical_document_symbols_for_snapshot, lexical_semantic_tokens_for_source,
    on_type_formatting,
    offset_for_position, parse_params, parse_source, position_for_offset,
    selected_label_from_debug_report,
    signature_help_debug_markdown,
    signature_help_report_for_cached_analysis_with_external_indexes,
    signature_help_report_for_pending_snapshot, source_backed_request_method,
    symbol_kind_label, validate_message_params,
    AdmissionDisposition, BlockCommentPairParams, DebugCompletionJob, DebugHoverJob,
    DebugRequestJob, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentQuery, DocumentQueryState, DocumentSymbolParams,
    EnterTypingAssistParams, ForegroundDocumentJob, HoverParams, HoverSelectionSource,
    LspPositionIndex, LspSemanticTokensFull, LspServer, OpenDocument,
    PositionIndex,
    QueryQuality, RangeFormattingParams, RpcMessage, TaskClass, TextSpan,
    TokenProjectionKind, TokenResultDisposition, UpsertOutcome, WorkspaceFileChangedParams,
    WorkspaceFileDeletedParams, BLOCK_COMMENT_PAIR_METHOD, DEBUG_COMPLETION_METHOD,
    DEBUG_HOVER_METHOD, ENTER_TYPING_ASSIST_METHOD, RANGE_FORMATTING_METHOD,
    SEMANTIC_TOKEN_MODIFIERS, SEMANTIC_TOKEN_TYPES, SERVER_NAME, SERVER_VERSION,
    SIGNATURE_HELP_RETRIGGER_CHARACTERS, SIGNATURE_HELP_TRIGGER_CHARACTERS,
    WORKSPACE_FILE_CHANGED_METHOD, WORKSPACE_FILE_DELETED_METHOD,
    file_index_for_source_with_timings, lex, publish_diagnostics_message,
};
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

impl<W: Write> LspServer<W> {
    pub(super) fn handle_message(
        &mut self,
        value: Value,
        queue_ms: Option<u128>,
        coalesced_changes: usize,
        superseded_changes: usize,
    ) -> Result<bool, String> {
        let started_at = Instant::now();
        let message = serde_json::from_value::<RpcMessage>(value.clone())
            .map_err(|error| format!("Invalid JSON-RPC message: {error}"))?;
        let queue_ms = queue_ms.unwrap_or(0);
        let Some(method) = message.method.as_deref() else {
            self.handle_semantic_tokens_refresh_response(&message)?;
            return Ok(false);
        };
        self.logger.diagnostic(
            "rpc.received",
            json!({
                "method": method,
                "request": message.id.is_some(),
                "queueMs": queue_ms,
                "coalescedChanges": coalesced_changes,
                "supersededChanges": superseded_changes,
            }),
        );

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
            "$/cancelRequest" => {
                if let Some(id) = message.params.as_ref().and_then(|params| params.get("id")) {
                    self.cancel_deferred_semantic_token_request(id);
                }
            }
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
                                "documentRangeFormattingProvider": true,
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
                    if let Some(mut previous) = self.documents.remove(&uri) {
                        previous.semantic_tokens.cancel_pending();
                        self.runtime.close(&uri, previous.snapshot.revision());
                    }
                    let UpsertOutcome::Accepted = self.runtime.upsert(uri.clone(), version, text)
                    else {
                        return Err(format!(
                            "didOpen could not install document snapshot for {uri}"
                        ));
                    };
                    let snapshot = self
                        .runtime
                        .latest(&uri)
                        .expect("accepted document snapshot is immediately readable");
                    let revision = snapshot.revision();
                    self.discard_deferred_document_requests(&uri, revision)?;
                    self.discard_deferred_semantic_token_requests(&uri, revision, "opened")?;
                    let revision = snapshot.revision();
                    if let Some(scheduler) = self.analysis_scheduler.clone() {
                        let mut document = OpenDocument::pending(snapshot);
                        document.mark_analysis_pending();
                        self.documents.insert(uri.clone(), document);
                        let request_id = self.next_server_request_id;
                        self.next_server_request_id += 1;
                        match self.runtime.admit(
                            TaskClass::Foreground,
                            self.runtime.latest(&uri).expect("accepted snapshot"),
                            request_id,
                            Instant::now() + Duration::from_secs(30),
                        ) {
                            AdmissionDisposition::Enqueued { .. } => {
                                let task = self
                                    .runtime
                                    .take_next()
                                    .expect("admitted foreground task is runnable");
                                scheduler.schedule_foreground(ForegroundDocumentJob {
                                    task,
                                    scheduled_at: Instant::now(),
                                });
                            }
                            AdmissionDisposition::DroppedOverload { .. } => {
                                self.documents
                                    .get_mut(&uri)
                                    .expect("pending document exists")
                                    .reject_pending_analysis();
                            }
                        }
                        self.log(&format!(
                            "notification didOpen uri={} bytes={} version={} revision={} foreground_state=pending analysis_state=waiting-foreground queue_ms={} analysis_elapsed_ms={}",
                            uri,
                            bytes,
                            version,
                            revision,
                            queue_ms,
                            start.elapsed().as_millis()
                        ));
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
                        let diagnostics_message = publish_diagnostics_message(
                            &uri,
                            version,
                            document.snapshot.text(),
                            &document
                                .syntax()
                                .expect("ready test document has syntax")
                                .diagnostics,
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
                            self.runtime.latest(&uri).map(|snapshot| snapshot.version())
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
                        let UpsertOutcome::Accepted =
                            self.runtime.upsert(uri.clone(), version, text)
                        else {
                            self.log(&format!(
                                "notification didChange ignored uri={} version={} current_version={} reason=stale",
                                uri, version, current_version
                            ));
                            return Ok(false);
                        };
                        let snapshot = self
                            .runtime
                            .latest(&uri)
                            .expect("accepted document snapshot is immediately readable");
                        let revision = {
                            let document = self
                                .documents
                                .get_mut(&uri)
                                .expect("open document exists after version check");
                            document.replace(snapshot);
                            document.mark_analysis_pending();
                            document.snapshot.revision()
                        };
                        self.discard_deferred_semantic_token_requests(
                            &uri,
                            revision,
                            "superseded",
                        )?;
                        if let Some(scheduler) = self.analysis_scheduler.clone() {
                            self.discard_deferred_document_requests(&uri, revision)?;
                            let request_id = self.next_server_request_id;
                            self.next_server_request_id += 1;
                            match self.runtime.admit(
                                TaskClass::Foreground,
                                self.runtime.latest(&uri).expect("accepted snapshot"),
                                request_id,
                                Instant::now() + Duration::from_secs(30),
                            ) {
                                AdmissionDisposition::Enqueued { .. } => {
                                    let task = self
                                        .runtime
                                        .take_next()
                                        .expect("admitted foreground task is runnable");
                                    scheduler.schedule_foreground(ForegroundDocumentJob {
                                        task,
                                        scheduled_at: Instant::now(),
                                    });
                                }
                                AdmissionDisposition::DroppedOverload { .. } => self
                                    .documents
                                    .get_mut(&uri)
                                    .expect("pending document exists")
                                    .reject_pending_analysis(),
                            }
                            self.log(&format!(
                                "notification didChange uri={} bytes={} version={} revision={} foreground_state=pending analysis_state=waiting-foreground queue_ms={} coalesced_changes={} superseded_changes={} analysis_elapsed_ms={}",
                                uri, bytes, version, revision, queue_ms, coalesced_changes, superseded_changes, start.elapsed().as_millis()
                            ));
                        } else {
                            // This in-process path exists for deterministic
                            // unit fixtures only. The stdio runtime always
                            // reaches this state through a foreground worker.
                            let snapshot = self.runtime.latest(&uri).expect("accepted snapshot");
                            let document =
                                self.documents.get_mut(&uri).expect("open document exists");
                            assert!(document.install_foreground(
                                revision,
                                PositionIndex::new(snapshot.text()),
                                lex(snapshot.text()),
                                parse_source(snapshot.text()),
                            ));
                            let diagnostics = document
                                .syntax()
                                .expect("fixture foreground installation supplies syntax")
                                .diagnostics
                                .clone();
                            self.write_message(publish_diagnostics_message(
                                &uri,
                                version,
                                snapshot.text(),
                                &diagnostics,
                            ))?;
                            let (analysis, timings) = file_index_for_source_with_timings(
                                self.runtime.latest(&uri).expect("accepted snapshot").text(),
                            );
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
                        self.runtime
                            .close(&params.text_document.uri, document.snapshot.revision());
                    }
                    self.discard_deferred_document_requests(&params.text_document.uri, 0)?;
                    self.discard_deferred_semantic_token_requests(
                        &params.text_document.uri,
                        0,
                        "closed",
                    )?;
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
                    let mut outline_quality = "Exact";
                    let mut projection_ms = 0u128;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get_mut(&log_uri).map(|document| {
                                bytes = document.text.len();
                                revision = document.revision;
                                if document.analysis_ready() {
                                    cached_projection = document.document_symbols_ready();
                                    if !cached_projection {
                                        let projection_start = Instant::now();
                                        let symbols = document_symbols_from_cached_analysis(
                                            &document.text,
                                            document.analysis(),
                                        );
                                        projection_ms = projection_start.elapsed().as_millis();
                                        document.set_document_symbols(symbols);
                                    }
                                    let symbols = document.document_symbols();
                                    symbol_count = document_symbol_count(&symbols);
                                    parse_diagnostics = document.analysis().parse_diagnostics;
                                    symbols.to_vec()
                                } else {
                                    outline_quality = "Unavailable";
                                    let projection_start = Instant::now();
                                    let symbols =
                                        lexical_document_symbols_for_snapshot(&document.snapshot);
                                    projection_ms = projection_start.elapsed().as_millis();
                                    symbol_count = document_symbol_count(&symbols);
                                    symbols
                                }
                            })
                        })
                        .map(|symbols| serde_json::to_value(symbols).unwrap_or(Value::Null))
                        .unwrap_or(Value::Null);
                    self.log(&format!(
                        "request documentSymbol uri={} bytes={} revision={} query_quality={} document_symbols_cached={} document_symbol_ms={} symbols={} parse_diagnostics={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        outline_quality,
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
                    let mut response_labels = String::new();
                    let mut failure_reason = "<none>".to_string();
                    let mut query_quality = QueryQuality::Exact;
                    let mut recovery_reason = "<none>".to_string();
                    let mut context_ms = 0u128;
                    let mut lookup_ms = 0u128;
                    let mut render_ms = 0u128;
                    let mut cached_analysis = false;
                    let mut foreground_ready = false;
                    let mut external_index_status = self.external_index.status_summary().status;
                    let mut external_index_layers = "none";
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.document_query(&log_uri).map(|query| {
                                let DocumentQuery {
                                    document,
                                    external_indexes: indexes,
                                } = query;
                                bytes = document.text.len();
                                revision = document.revision;
                                foreground_ready = document.foreground_ready();
                                external_index_status = indexes.status;
                                external_index_layers = indexes.available_layers();
                                let report = if let DocumentQueryState::Cached(analysis) =
                                    DocumentQuery::state_for(document)
                                {
                                    cached_analysis = true;
                                    completion_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        analysis,
                                        params.position,
                                        indexes.workspace.as_deref(),
                                        indexes.game_data.as_deref(),
                                    )
                                } else {
                                    let offset = document
                                        .snapshot
                                        .positions()
                                        .and_then(|positions| {
                                            positions.offset_for_position(
                                                crate::analysis_runtime::Position {
                                                    line: params.position.line,
                                                    character: params.position.character,
                                                },
                                            )
                                        })
                                        // A completion may arrive in the short interval before
                                        // foreground publication. Derive the offset from the same
                                        // immutable snapshot rather than falling back to broad
                                        // position-based lexical completion.
                                        .or_else(|| {
                                            offset_for_position(&document.text, params.position)
                                        });
                                    if let Some(offset) = offset {
                                        let completion_start = std::time::Instant::now();
                                        let argument_label_report = completion_report_for_current_argument_labels_at_offset_with_external_indexes(
                                            &document.text,
                                            offset,
                                            indexes.workspace.as_deref(),
                                            indexes.game_data.as_deref(),
                                        );
                                        let value_report = completion_report_for_current_receiver_at_offset_with_external_indexes(
                                            &document.text,
                                            offset,
                                            indexes.workspace.as_deref(),
                                            indexes.game_data.as_deref(),
                                        )
                                        .or_else(|| completion_report_for_current_override_at_offset_with_external_indexes(
                                            &document.text,
                                            offset,
                                            indexes.workspace.as_deref(),
                                            indexes.game_data.as_deref(),
                                        ))
                                        .or_else(|| completion_report_for_current_local_scope_at_offset_with_external_indexes(
                                            &document.text,
                                            offset,
                                            indexes.workspace.as_deref(),
                                            indexes.game_data.as_deref(),
                                        ));
                                        match (argument_label_report, value_report) {
                                            (Some(argument_report), Some(value_report)) => {
                                                completion::merge_argument_label_and_value_reports(
                                                    argument_report,
                                                    value_report,
                                                    completion_start,
                                                )
                                            }
                                            (Some(argument_report), None) => argument_report,
                                            (None, Some(value_report)) => value_report,
                                            (None, None) => {
                                            completion_report_for_lexical_source_at_offset_with_external_indexes(
                                                &document.text,
                                                offset,
                                                indexes.workspace.as_deref(),
                                                indexes.game_data.as_deref(),
                                            )
                                            }
                                        }
                                    } else {
                                        completion_report_for_lexical_source_with_external_indexes(
                                            &document.text,
                                            params.position,
                                            indexes.workspace.as_deref(),
                                            indexes.game_data.as_deref(),
                                        )
                                    }
                                };
                                parse_diagnostics = report.parse_diagnostics;
                                query_quality = report.query_quality;
                                recovery_reason = report
                                    .recovery_reason
                                    .clone()
                                    .unwrap_or_else(|| "<none>".to_string());
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
                                response_labels = report
                                    .list
                                    .items
                                    .iter()
                                    .take(3)
                                    .map(|item| item.label.as_str())
                                    .collect::<Vec<_>>()
                                    .join(",");
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
                        "request completion uri={} bytes={} revision={} foreground_ready={} cached_analysis={} query_quality={:?} recovery_reason={} context={} receiver={} owner_type={} prefix={} candidates={} response_labels={} failure_reason={} external_index_status={} external_index_layers={} parse_diagnostics={} context_ms={} lookup_ms={} render_ms={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        foreground_ready,
                        cached_analysis,
                        query_quality,
                        recovery_reason,
                        completion_context,
                        receiver,
                        owner_type,
                        prefix,
                        candidate_count,
                        response_labels,
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
            ENTER_TYPING_ASSIST_METHOD => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<EnterTypingAssistParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut version = -1i32;
                    let mut line = -1i64;
                    let mut character = -1i64;
                    let mut outcome = "no_edit";
                    let mut trigger = "unknown";
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            version = params.version;
                            line = params.position.line as i64;
                            character = params.position.character as i64;
                            trigger = if params.ch == "\n" { "enter" } else if params.ch == "\t" { "tab" } else { "unsupported" };
                            if !matches!(params.ch.as_str(), "\n" | "\t")
                                || (params.ch == "\n" && params.position.line == 0)
                            {
                                outcome = "unsupported_trigger";
                                return None;
                            }
                            let document = self.documents.get(&log_uri)?;
                            bytes = document.text.len();
                            if document.version != params.version {
                                outcome = "stale_version";
                                return None;
                            }
                            let cursor = offset_for_position(&document.text, params.position)?;
                            if params.ch == "\n" {
                                if let Some(plan) = on_type_formatting::incomplete_if_header_enter_plan(
                                    &document.text,
                                    cursor,
                                    params.options.tab_size,
                                    params.options.insert_spaces,
                                ) {
                                    outcome = "if_header";
                                    let start = position_for_offset(&document.text, plan.span.start);
                                    return Some(json!({
                                        "edits": [{
                                            "range": {
                                                "start": start,
                                                "end": position_for_offset(&document.text, plan.span.end),
                                            },
                                            "newText": plan.replacement,
                                        }],
                                        "selection": {
                                            "line": start.line + 1,
                                            "character": plan.selection_character,
                                        },
                                    }));
                                }
                            }
                            let mut edits = Vec::new();
                            let outdent = on_type_formatting::unbraced_if_body_outdent_plan(
                                &document.text,
                                cursor,
                                params.options.tab_size,
                                params.options.insert_spaces,
                                params.ch == "\t",
                            );
                            if let Some(plan) = &outdent {
                                edits.push(json!({
                                    "range": {
                                        "start": position_for_offset(&document.text, plan.span.start),
                                        "end": position_for_offset(&document.text, plan.span.end),
                                    },
                                    "newText": plan.replacement,
                                }));
                            }
                            if params.ch == "\n" {
                                if let Some(insertion) = on_type_formatting::semicolon_insertion_offset(
                                    &document.text,
                                    cursor,
                                ) {
                                    let position = position_for_offset(&document.text, insertion);
                                    edits.push(json!({
                                        "range": { "start": position, "end": position },
                                        "newText": ";",
                                    }));
                                }
                            }
                            outcome = match (outdent.is_some(), edits.is_empty()) {
                                (_, true) => "no_edit",
                                (true, _) => "if_body_outdent",
                                (false, _) => "semicolon",
                            };
                            let mut result = json!({ "edits": edits });
                            if let Some(plan) = outdent {
                                result["selection"] = json!({
                                    "line": params.position.line,
                                    "character": plan.selection_character,
                                });
                            }
                            Some(result)
                        })
                        .unwrap_or_else(|| json!({ "edits": [] }));
                    self.log(&format!(
                        "request enterTypingAssist uri={} bytes={} version={} line={} character={} trigger={} outcome={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        version,
                        line,
                        character,
                        trigger,
                        outcome,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                }
            }
            BLOCK_COMMENT_PAIR_METHOD => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<BlockCommentPairParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut version = -1i32;
                    let mut outcome = "no_edit";
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            version = params.version;
                            let document = self.documents.get(&log_uri)?;
                            bytes = document.text.len();
                            if document.version != params.version {
                                outcome = "stale_version";
                                return None;
                            }
                            let cursor = offset_for_position(&document.text, params.position)?;
                            let plan = on_type_formatting::block_comment_pair_plan(
                                &document.text,
                                cursor,
                                params.options.tab_size,
                                params.options.insert_spaces,
                            )?;
                            outcome = "paired";
                            let start = position_for_offset(&document.text, plan.span.start);
                            Some(json!({
                                "edits": [{
                                    "range": {
                                        "start": start,
                                        "end": position_for_offset(&document.text, plan.span.end),
                                    },
                                    "newText": plan.replacement,
                                }],
                                "selection": {
                                    "line": start.line + 1,
                                    "character": plan.selection_character,
                                },
                            }))
                        })
                        .unwrap_or_else(|| json!({ "edits": [] }));
                    self.log(&format!(
                        "request blockCommentPair uri={} bytes={} version={} outcome={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        version,
                        outcome,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                }
            }
            RANGE_FORMATTING_METHOD => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let params = parse_params::<RangeFormattingParams>(message.params, method)?;
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut version = -1i32;
                    let mut edit_count = 0usize;
                    let mut outcome = "no_edit";
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            let document = self.documents.get(&log_uri)?;
                            bytes = document.text.len();
                            version = document.version;
                            let start = offset_for_position(&document.text, params.range.start)?;
                            let end = offset_for_position(&document.text, params.range.end)?;
                            if start > end {
                                outcome = "invalid_range";
                                return None;
                            }
                            let edits = crate::formatting::format_comment_region(
                                &document.text,
                                TextSpan::new(start, end),
                            );
                            edit_count = edits.len();
                            if edits.is_empty() {
                                return None;
                            }
                            let positions = LspPositionIndex::new(&document.text);
                            outcome = "comment_region";
                            Some(Value::Array(
                                edits
                                    .into_iter()
                                    .map(|edit| {
                                        json!({
                                            "range": positions.range_for_span(edit.span),
                                            "newText": edit.replacement
                                        })
                                    })
                                    .collect(),
                            ))
                        })
                        .unwrap_or_else(|| json!([]));
                    self.log(&format!(
                        "request rangeFormatting uri={} bytes={} version={} edits={} outcome={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        version,
                        edit_count,
                        outcome,
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
                            self.document_query(&log_uri).map(|query| {
                                let DocumentQuery {
                                    document,
                                    external_indexes: indexes,
                                } = query;
                                bytes = document.text.len();
                                revision = document.revision;
                                external_index_status = indexes.status;
                                external_index_layers = indexes.available_layers();
                                let report = match DocumentQuery::state_for(document) {
                                    DocumentQueryState::Cached(analysis) =>
                                        signature_help_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        analysis,
                                        params.position,
                                        indexes.workspace.as_deref(),
                                        indexes.game_data.as_deref(),
                                    ),
                                    DocumentQueryState::Foreground(foreground) =>
                                        signature_help_report_for_pending_snapshot(
                                        &document.snapshot,
                                        foreground,
                                        document.parse_diagnostic_count(),
                                        params.position,
                                    ),
                                    DocumentQueryState::Pending => return None,
                                };
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
                        "request signatureHelp uri={} bytes={} revision={} cached_analysis={} context={} active_parameter={} candidates={} selected={} failure_reason={} external_index_status={} external_index_layers={} parse_diagnostics={} context_ms={} lookup_ms={} render_ms={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        self.documents.get(&log_uri).is_some_and(OpenDocument::analysis_ready),
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
                    let mut rich_work: Option<(String, u64, u64)> = None;
                    let mut result_id = "reforger:missing:lexical".to_string();
                    let mut defer_current_request = false;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.documents.get_mut(&log_uri).map(|document| {
                                    bytes = document.text.len();
                                    revision = document.revision;
                                    let had_rich_display =
                                        document.semantic_tokens.has_rich_display();
                                    let source = document.text.clone();
                                    let (
                                        selection_kind,
                                        selection_result_id,
                                        disposition,
                                        projection,
                                    ) = {
                                        let selection =
                                            document.semantic_tokens.select_or_insert_lexical(
                                                document.revision,
                                                external_generation,
                                                || lexical_semantic_tokens_for_source(&source),
                                            );
                                        (
                                            selection.kind,
                                            selection.result_id,
                                            selection.disposition,
                                            selection.projection.clone(),
                                        )
                                    };
                                    result_id = selection_result_id;
                                    projection_mode = match selection_kind {
                                        TokenProjectionKind::LexicalBaseline
                                            if document.analysis_ready() =>
                                        {
                                            "lexical-baseline"
                                        }
                                        TokenProjectionKind::LexicalBaseline => "lexical-pending",
                                        TokenProjectionKind::RichOverlay => "rich-overlay",
                                    };
                                    debug_assert_eq!(disposition, TokenResultDisposition::Full);
                                    if selection_kind == TokenProjectionKind::LexicalBaseline
                                        && document.analysis_ready()
                                    {
                                        if !document
                                            .semantic_tokens
                                            .pending_for_revision_and_external_generation(
                                                document.revision,
                                                external_generation,
                                            )
                                        {
                                            rich_work = Some((
                                                log_uri.clone(),
                                                document.revision,
                                                external_generation,
                                            ));
                                        }
                                    }
                                    defer_current_request = selection_kind
                                        == TokenProjectionKind::LexicalBaseline
                                        && had_rich_display;
                                    token_count = projection.token_count;
                                    parse_diagnostics = projection.parse_diagnostics;
                                    lex_ms = projection.timings.lex_ms;
                                    resolver_ms = projection.timings.resolver_ms;
                                    resolver_calls = projection.timings.identifier_resolver_calls;
                                    token_loop_ms = projection.timings.token_loop_ms;
                                    encode_ms = projection.timings.encode_ms;
                                    LspSemanticTokensFull::from_tokens(
                                        result_id.clone(),
                                        &projection.tokens,
                                    )
                                })
                        })
                        .map(|tokens| serde_json::to_value(tokens).unwrap_or(Value::Null))
                        .unwrap_or_else(|| {
                            serde_json::to_value(LspSemanticTokensFull {
                                result_id,
                                data: Vec::new(),
                            })
                            .unwrap_or(Value::Null)
                        });
                    self.log(&format!(
                        "request semanticTokens uri={} bytes={} revision={} cached_analysis=true mode={} outcome={} tokens={} external_index_status={} external_generation={} parse_diagnostics={} lex_ms={} token_loop_ms={} resolver_ms={} resolver_calls={} encode_ms={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        projection_mode,
                        if defer_current_request { "deferred-rich" } else { "responded" },
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
                    if defer_current_request {
                        self.defer_semantic_token_request(
                            &log_uri,
                            revision,
                            external_generation,
                            id,
                        )?;
                    } else {
                        self.respond(id, result)?;
                    }
                    if let Some((uri, rich_revision, rich_external_generation)) = rich_work {
                        self.schedule_rich_semantic_tokens(
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
                    let mut external_index_layers = "none";
                    let mut revision = 0u64;
                    let mut hit = false;
                    let mut query_quality = QueryQuality::Exact;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.document_query(&log_uri).map(|query| {
                                query_quality = query.quality();
                                let DocumentQuery {
                                    document,
                                    external_indexes: indexes,
                                } = query;
                                bytes = document.text.len();
                                revision = document.revision;
                                external_index_status = indexes.status;
                                external_index_layers = indexes.available_layers();
                                let report = match DocumentQuery::state_for(document) {
                                    DocumentQueryState::Cached(analysis) => {
                                        hover_report_for_cached_analysis_with_external_indexes(
                                            &document.text,
                                            analysis,
                                            &log_uri,
                                            params.position,
                                            indexes.workspace.as_deref(),
                                            indexes.game_data.as_deref(),
                                        )
                                    }
                                    DocumentQueryState::Foreground(foreground) => {
                                        hover_report_for_pending_snapshot(
                                            &document.snapshot,
                                            foreground,
                                            params.position,
                                            document.parse_diagnostic_count(),
                                        )
                                    }
                                    DocumentQueryState::Pending => return None,
                                };
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
                        "request hover uri={} bytes={} revision={} query_quality={:?} cached_analysis={} hit={} selection_source={} selected_source={} resolver_reason={} identifier_context={} resolver_candidates={} receiver_owner={} receiver_failure={} external_index_status={} external_index_layers={} label={} kind={} parse_diagnostics={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        query_quality,
                        query_quality.permits_local_facts(),
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
                    let mut query_quality = QueryQuality::Exact;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.document_query(&log_uri).map(|query| {
                                query_quality = query.quality();
                                let DocumentQuery {
                                    document,
                                    external_indexes: indexes,
                                } = query;
                                bytes = document.text.len();
                                revision = document.revision;
                                external_index_status = indexes.status;
                                external_index_layers = indexes.available_layers();
                                let report = match DocumentQuery::state_for(document) {
                                    DocumentQueryState::Cached(analysis) => {
                                        definition_report_for_cached_analysis_with_external_indexes(
                                            &document.text,
                                            analysis,
                                            &log_uri,
                                            params.position,
                                            indexes.workspace.as_deref(),
                                            indexes.game_data.as_deref(),
                                        )
                                    }
                                    DocumentQueryState::Foreground(foreground) => {
                                        definition_report_for_pending_snapshot(
                                            &document.snapshot,
                                            foreground,
                                            &log_uri,
                                            params.position,
                                            document.parse_diagnostic_count(),
                                        )
                                    }
                                    DocumentQueryState::Pending => return Vec::new(),
                                };
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
                        "request definition uri={} bytes={} revision={} query_quality={:?} cached_analysis={} hit={} selected_source={} resolver_reason={} identifier_context={} resolver_candidates={} external_index_status={} external_index_layers={} label={} kind={} parse_diagnostics={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        query_quality,
                        query_quality.permits_local_facts(),
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
                    if let Some(ref params) = params {
                        if let Some(query) = self.document_query(&params.text_document.uri) {
                            let DocumentQuery {
                                document,
                                external_indexes: indexes,
                            } = query;
                            if let DocumentQueryState::Cached(analysis) =
                                DocumentQuery::state_for(document)
                            {
                                if let Some(scheduler) = self.analysis_scheduler.clone() {
                                    let uri = params.text_document.uri.clone();
                                    let position = params.position;
                                    let revision = document.revision;
                                    let analysis = analysis.clone();
                                    let external_status = self.external_index.status_summary();
                                    let task = match self.admit_debug_capture(&uri) {
                                        Ok(task) => task,
                                        Err((retained_jobs, retained_bytes)) => {
                                            self.log(&format!(
                                            "request debugHover skipped uri={} revision={} reason=runtime-overload retained_jobs={} retained_bytes={} elapsed_ms={}",
                                            uri,
                                            revision,
                                            retained_jobs,
                                            retained_bytes,
                                            start.elapsed().as_millis()
                                        ));
                                            self.respond_error(
                                                id,
                                                -32801,
                                                "Debug capture unavailable",
                                            )?;
                                            return Ok(false);
                                        }
                                    };
                                    scheduler.schedule_debug(DebugRequestJob::Hover(
                                        DebugHoverJob {
                                            task,
                                            id,
                                            uri,
                                            position,
                                            revision,
                                            scheduled_at: start,
                                            analysis,
                                            external_snapshot: indexes,
                                            external_status,
                                        },
                                    ));
                                    return Ok(false);
                                }
                            }
                        }
                    }
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut revision = 0u64;
                    let mut hit = false;
                    let mut selected_label = "<none>".to_string();
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.document_query(&log_uri).and_then(|query| {
                                let DocumentQuery {
                                    document,
                                    external_indexes: indexes,
                                } = query;
                                let DocumentQueryState::Cached(analysis) =
                                    DocumentQuery::state_for(document)
                                else {
                                    return None;
                                };
                                bytes = document.text.len();
                                revision = document.revision;
                                let external_status = self.external_index.status_summary();
                                let report =
                                    debug_hover_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        analysis,
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
                                Some(Value::String(report))
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
                    if let Some(ref params) = params {
                        if let Some(query) = self.document_query(&params.text_document.uri) {
                            let DocumentQuery {
                                document,
                                external_indexes: indexes,
                            } = query;
                            if let DocumentQueryState::Cached(analysis) =
                                DocumentQuery::state_for(document)
                            {
                                if let Some(scheduler) = self.analysis_scheduler.clone() {
                                    let uri = params.text_document.uri.clone();
                                    let position = params.position;
                                    let revision = document.revision;
                                    let analysis = analysis.clone();
                                    let task = match self.admit_debug_capture(&uri) {
                                        Ok(task) => task,
                                        Err((retained_jobs, retained_bytes)) => {
                                            self.log(&format!(
                                            "request debugCompletion skipped uri={} revision={} reason=runtime-overload retained_jobs={} retained_bytes={} elapsed_ms={}",
                                            uri,
                                            revision,
                                            retained_jobs,
                                            retained_bytes,
                                            start.elapsed().as_millis()
                                        ));
                                            self.respond_error(
                                                id,
                                                -32801,
                                                "Debug capture unavailable",
                                            )?;
                                            return Ok(false);
                                        }
                                    };
                                    scheduler.schedule_debug(DebugRequestJob::Completion(
                                        DebugCompletionJob {
                                            task,
                                            id,
                                            uri,
                                            position,
                                            revision,
                                            scheduled_at: start,
                                            analysis,
                                            external_snapshot: indexes,
                                        },
                                    ));
                                    return Ok(false);
                                }
                            }
                        }
                    }
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
                            self.document_query(&log_uri).and_then(|query| {
                                let DocumentQuery {
                                    document,
                                    external_indexes: indexes,
                                } = query;
                                let DocumentQueryState::Cached(analysis) =
                                    DocumentQuery::state_for(document)
                                else {
                                    return None;
                                };
                                bytes = document.text.len();
                                revision = document.revision;
                                external_index_status = indexes.status;
                                external_index_layers = indexes.available_layers();
                                let report = completion_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        analysis,
                                        params.position,
                                        indexes.workspace.as_deref(),
                                        indexes.game_data.as_deref(),
                                    );
                                let signature_report = signature_help_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        analysis,
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
                                Some(Value::String(markdown))
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
        };
        self.request_semantic_tokens_refresh_if_external_generation_changed()?;
        let should_exit = self.shutdown_requested && method == "exit";
        self.logger.diagnostic(
            "rpc.completed",
            json!({
                "method": method,
                "outcome": if should_exit { "exit" } else { "complete" },
                "elapsedMs": started_at.elapsed().as_millis(),
            }),
        );
        Ok(should_exit)
    }
}
