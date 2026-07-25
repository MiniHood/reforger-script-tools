use super::request_router::{FeatureCommand, RequestCommand, RoutedRequest, WorkspaceIndexCommand};
use super::workspace_requests::{delete_workspace_file, update_workspace_file};
use super::{
    completion, completion_debug_markdown,
    completion_report_for_cached_analysis_with_external_indexes,
    completion_report_for_current_argument_labels_at_offset_with_external_indexes,
    completion_report_for_current_contextual_constructor_at_offset_with_external_indexes,
    completion_report_for_current_incomplete_callable_parameter_type_at_offset_with_external_indexes,
    completion_report_for_current_local_scope_at_offset_with_external_indexes,
    completion_report_for_current_override_at_offset_with_external_indexes,
    completion_report_for_current_preprocessor_at_offset_with_external_indexes,
    completion_report_for_current_receiver_at_offset_with_external_indexes,
    completion_report_for_current_super_at_offset_with_external_indexes,
    completion_report_for_lexical_source_at_offset_with_external_indexes,
    completion_report_for_lexical_source_with_external_indexes,
    debug_hover_report_for_cached_analysis_with_external_indexes,
    definition_report_for_cached_analysis_with_external_indexes,
    definition_report_for_pending_snapshot, document_symbol_count, document_symbol_range_repairs,
    empty_completion_list, hover_report_for_cached_analysis_with_external_indexes,
    hover_report_for_pending_snapshot, offset_for_position, on_type_formatting,
    position_for_offset, selected_label_from_debug_report, signature_help_debug_markdown,
    signature_help_report_for_cached_analysis_with_external_indexes,
    signature_help_report_for_pending_snapshot, source_backed_request_method, symbol_kind_label,
    DebugCompletionJob, DebugHoverJob, DebugRequestJob, DocumentQuery, DocumentQueryState,
    DocumentRuntime, ExternalIndexHandle, HoverSelectionSource, LspPositionIndex, QueryQuality,
    RuntimeEffect, TextSpan, ACTIVE_SCOPE_DELIMITERS_METHOD, BLOCK_COMMENT_PAIR_METHOD,
    CONTROL_HEADER_ENTER_METHOD, DEBUG_COMPLETION_METHOD, DEBUG_HOVER_METHOD,
    RANGE_FORMATTING_METHOD, WORKSPACE_FILE_CHANGED_METHOD, WORKSPACE_FILE_DELETED_METHOD,
};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Instant;

/// Executes feature and workspace commands from the explicit state they
/// require, returning effects for the composition root to deliver.
pub(super) struct FeatureDispatchOutcome {
    pub(super) should_exit: bool,
    pub(super) effects: Vec<RuntimeEffect>,
}

struct FeatureDispatcher<'a> {
    external_index: &'a mut ExternalIndexHandle,
    document_runtime: &'a mut DocumentRuntime,
    shutdown_requested: bool,
    operational_logging: bool,
    diagnostic_logging: bool,
    effects: Vec<RuntimeEffect>,
}

pub(super) fn execute_feature_or_workspace_message(
    external_index: &mut ExternalIndexHandle,
    document_runtime: &mut DocumentRuntime,
    shutdown_requested: bool,
    routed: RoutedRequest,
    queue_ms: Option<u128>,
    coalesced_changes: usize,
    superseded_changes: usize,
    operational_logging: bool,
    diagnostic_logging: bool,
) -> Result<FeatureDispatchOutcome, String> {
    FeatureDispatcher {
        external_index,
        document_runtime,
        shutdown_requested,
        operational_logging,
        diagnostic_logging,
        effects: Vec::new(),
    }
    .dispatch(routed, queue_ms, coalesced_changes, superseded_changes)
}

impl FeatureDispatcher<'_> {
    fn dispatch(
        &mut self,
        routed: RoutedRequest,
        queue_ms: Option<u128>,
        coalesced_changes: usize,
        superseded_changes: usize,
    ) -> Result<FeatureDispatchOutcome, String> {
        let started_at = Instant::now();
        let RoutedRequest {
            command,
            message,
            parameter_error,
        } = routed;
        let queue_ms = queue_ms.unwrap_or(0);
        let Some(method) = message.method.as_deref() else {
            for effect in self
                .document_runtime
                .acknowledge_semantic_tokens_refresh(&message)
            {
                self.deliver_effect(effect)?;
            }
            return Ok(self.finish(false));
        };
        self.effects.push(RuntimeEffect::diagnostic_lazy(
            self.diagnostic_logging,
            "rpc.received",
            || {
                json!({
                    "method": method,
                    "command": format!("{command:?}"),
                    "request": message.id.is_some(),
                    "queueMs": queue_ms,
                    "coalescedChanges": coalesced_changes,
                    "supersededChanges": superseded_changes,
                })
            },
        ));

        if self.shutdown_requested && method != "exit" {
            let error = "Server has already received shutdown";
            if let Some(id) = message.id.clone() {
                self.respond_error(id, -32600, error)?;
            } else {
                self.log(|| format!("notification ignored after shutdown method={method}"));
            }
            return Ok(self.finish(false));
        }

        if let Some(error) = parameter_error {
            if let Some(id) = message.id.clone() {
                self.respond_error(id, -32602, &error)?;
            } else {
                self.log(|| {
                    format!("notification ignored invalid_params method={method} error={error}")
                });
            }
            return Ok(self.finish(false));
        }

        if message.id.is_some() && source_backed_request_method(method) {
            let (deferred, effects) = self.document_runtime.defer_document_request(
                &message,
                command.clone(),
                parameter_error.clone(),
            )?;
            for effect in effects {
                self.deliver_effect(effect)?;
            }
            if deferred {
                return Ok(self.finish(false));
            }
        }

        let mut semantic_generation_preservation = None;
        match method {
            "$/cancelRequest" => {}
            WORKSPACE_FILE_CHANGED_METHOD => {
                let RequestCommand::WorkspaceIndex(WorkspaceIndexCommand::Changed(params)) =
                    &command
                else {
                    unreachable!("workspace change method has a workspace command");
                };
                let previous_generation = self.external_index.status_summary().generation;
                let preservation = params.as_ref().and_then(|params| {
                    self.document_runtime.self_save_generation_preservation(
                        Path::new(&params.path),
                        &params.text,
                        previous_generation,
                    )
                });
                for effect in update_workspace_file(
                    &mut self.external_index,
                    params.clone(),
                    self.operational_logging,
                ) {
                    self.deliver_effect(effect)?;
                }
                let generation = self.external_index.status_summary().generation;
                if generation == previous_generation.saturating_add(1) {
                    semantic_generation_preservation = preservation;
                }
            }
            WORKSPACE_FILE_DELETED_METHOD => {
                let RequestCommand::WorkspaceIndex(WorkspaceIndexCommand::Deleted(params)) =
                    &command
                else {
                    unreachable!("workspace deletion method has a workspace command");
                };
                for effect in delete_workspace_file(
                    &mut self.external_index,
                    params.clone(),
                    self.operational_logging,
                ) {
                    self.deliver_effect(effect)?;
                }
            }
            "textDocument/documentSymbol" => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let RequestCommand::Feature(FeatureCommand::DocumentSymbols(params)) = &command
                    else {
                        unreachable!("document symbol method has a typed command");
                    };
                    let params = params.clone();
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut symbol_count = 0usize;
                    let mut parse_diagnostics = 0usize;
                    let mut revision = 0u64;
                    let mut cached_projection = false;
                    let mut outline_quality = "Exact";
                    let mut projection_ms = 0u128;
                    let mut range_repair_count = 0usize;
                    let mut range_repair_samples = Vec::new();
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.document_runtime
                                .capture_query(&log_uri, self.external_index.snapshot())
                                .map(|query| {
                                    let projection = query.document_symbols();
                                    bytes = projection.bytes;
                                    revision = projection.revision;
                                    cached_projection = projection.cached;
                                    outline_quality = projection.quality;
                                    projection_ms = projection.projection_ms;
                                    parse_diagnostics = projection.parse_diagnostics;
                                    symbol_count = document_symbol_count(&projection.symbols);
                                    if self.operational_logging || self.diagnostic_logging {
                                        (range_repair_count, range_repair_samples) =
                                            document_symbol_range_repairs(&projection.symbols, 8);
                                    }
                                    projection.symbols
                                })
                        })
                        .map(|symbols| serde_json::to_value(symbols).unwrap_or(Value::Null))
                        .unwrap_or(Value::Null);
                    self.log(|| format!(
                        "request documentSymbol uri={} bytes={} revision={} query_quality={} document_symbols_cached={} document_symbol_ms={} symbols={} parse_diagnostics={} range_repairs={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        outline_quality,
                        cached_projection,
                        projection_ms,
                        symbol_count,
                        parse_diagnostics,
                        range_repair_count,
                        queue_ms,
                        start.elapsed().as_millis()
                    ));
                    if range_repair_count > 0 {
                        self.deliver_effect(RuntimeEffect::diagnostic_lazy(
                            self.diagnostic_logging,
                            "documentSymbol.rangeRepaired",
                            || {
                                json!({
                                    "revision": revision,
                                    "bytes": bytes,
                                    "repairCount": range_repair_count,
                                    "samples": range_repair_samples,
                                })
                            },
                        ))?;
                    }
                    self.respond(id, result)?;
                }
            }
            "textDocument/completion" => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let RequestCommand::Feature(FeatureCommand::Completion(params)) = &command
                    else {
                        unreachable!("completion method has a typed command");
                    };
                    let params = params.clone();
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
                            let trigger_character = params
                                .context
                                .as_ref()
                                .and_then(|context| context.trigger_character.as_deref())
                                .map(str::to_string);
                            log_uri = params.text_document.uri;
                            self.document_runtime
                                .capture_query(&log_uri, self.external_index.snapshot())
                                .map(|query| {
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
                                        completion_report_for_current_preprocessor_at_offset_with_external_indexes(
                                            &document.text,
                                            offset,
                                            indexes.workspace.as_deref(),
                                            indexes.game_data.as_deref(),
                                        ).unwrap_or_else(|| {
                                            let completion_start = std::time::Instant::now();
                                            if let Some(report) = completion_report_for_current_contextual_constructor_at_offset_with_external_indexes(
                                                &document.text,
                                                offset,
                                                indexes.workspace.as_deref(),
                                                indexes.game_data.as_deref(),
                                            ) {
                                                return report;
                                            }
                                            if let Some(report) = completion_report_for_current_incomplete_callable_parameter_type_at_offset_with_external_indexes(
                                                &document.text,
                                                offset,
                                                indexes.workspace.as_deref(),
                                                indexes.game_data.as_deref(),
                                            ) {
                                                return report;
                                            }
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
                                            .or_else(|| completion_report_for_current_super_at_offset_with_external_indexes(
                                                &document.text,
                                                offset,
                                                indexes.workspace.as_deref(),
                                                indexes.game_data.as_deref(),
                                            ))
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
                                        })
                                    } else {
                                        completion_report_for_lexical_source_with_external_indexes(
                                            &document.text,
                                            params.position,
                                            indexes.workspace.as_deref(),
                                            indexes.game_data.as_deref(),
                                        )
                                    }
                                };
                                let report = completion::apply_automatic_trigger_policy(
                                    report,
                                    trigger_character.as_deref(),
                                );
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
                    self.log(|| format!(
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
            CONTROL_HEADER_ENTER_METHOD => {
                if let Some(id) = message.id {
                    let started_at = Instant::now();
                    let RequestCommand::Feature(FeatureCommand::InputRoute(params)) = &command
                    else {
                        unreachable!("input route has typed parameters");
                    };
                    let mut trace = None;
                    let result = params
                        .clone()
                        .and_then(|params| {
                            if !matches!(params.operation.as_str(), "insertNewline" | "indent" | "insertSpace") {
                                trace = params.trace.then_some(("declined", "none", true, "unsupportedOperation"));
                                return None;
                            }
                            if params.selections.len() != 1 || params.selections[0].start != params.selections[0].end {
                                trace = params.trace.then_some(("declined", "none", true, "singleCaretRequired"));
                                return None;
                            }
                            trace = params.trace.then_some(("declined", "none", true, "documentUnavailable"));
                            let query = self
                                .document_runtime
                                .capture_query(&params.text_document.uri, self.external_index.snapshot())?;
                            let document = query.document;
                            if document.version != params.version {
                                trace = params.trace.then_some(("stale", "none", false, "staleVersion"));
                                return None;
                            }
                            let cursor = offset_for_position(&document.text, params.selections[0].end)?;
                            if params.operation == "insertSpace" {
                                crate::lsp::collection_declaration::collection_declaration_before_cursor(
                                    &document.text,
                                    cursor,
                                    false,
                                )?;
                                trace = params.trace.then_some(("applied", "collectionDeclarationTail", true, "eligible"));
                                let position = position_for_offset(&document.text, cursor);
                                return Some(json!({
                                    "edits": [{
                                        "range": { "start": position, "end": position },
                                        "newText": " ",
                                    }],
                                    "owner": "collectionDeclarationTail",
                                    "selection": { "line": position.line, "character": position.character + 1 },
                                    "triggerSuggest": true,
                                }));
                            }
                            let plan = if params.operation == "indent" {
                                on_type_formatting::unbraced_if_body_indent_plan(&document.text, cursor)
                                    .map(|plan| (plan, "unbracedIfBody"))
                            } else {
                                on_type_formatting::auto_block_class_declaration_enter_plan(
                                    &document.text,
                                    cursor,
                                    params.options.tab_size,
                                    params.options.insert_spaces,
                                )
                                .map(|plan| (plan, "classDeclaration"))
                                .or_else(|| on_type_formatting::auto_block_protected_method_enter_plan(
                                    &document.text,
                                    cursor,
                                    params.options.tab_size,
                                    params.options.insert_spaces,
                                )
                                .map(|plan| (plan, "protectedMethod")))
                                .or_else(|| on_type_formatting::control_header_block_before_enter_plan(
                                    &document.text,
                                    cursor,
                                    params.options.tab_size,
                                    params.options.insert_spaces,
                                )
                                .map(|plan| (plan, "controlHeader")))
                            .or_else(|| on_type_formatting::if_header_body_before_enter_plan(
                                &document.text,
                                cursor,
                                params.options.tab_size,
                                params.options.insert_spaces,
                            ).map(|plan| (plan, "ifHeader")))
                            .or_else(|| on_type_formatting::semicolon_before_enter_plan(
                                &document.text,
                                cursor,
                            ).map(|plan| (plan, "semicolon")))
                            };
                            let Some((plan, owner)) = plan else {
                                return None;
                            };
                            let use_snippet = owner == "pairedBraceBody";
                            trace = params.trace.then_some(("applied", owner, true, "eligible"));
                            let start = position_for_offset(&document.text, plan.span.start);
                            Some(json!({
                                "edits": [{
                                    "range": {
                                        "start": start,
                                        "end": position_for_offset(&document.text, plan.span.end),
                                    },
                                    "newText": if use_snippet { "" } else { &plan.replacement },
                                }],
                                "snippet": use_snippet.then_some(&plan.replacement),
                                "snippetRange": use_snippet.then(|| json!({ "start": start, "end": position_for_offset(&document.text, plan.span.end) })),
                                "owner": owner,
                                "selectionRange": plan.switch_arm_selection_end.map(|end| json!({ "start": { "line": plan.selection_line, "character": plan.selection_character }, "end": { "line": plan.selection_line, "character": end } })),
                                "selection": { "line": plan.selection_line, "character": plan.selection_character },
                                "triggerSuggest": plan.switch_arm_selection_end.is_some(),
                            }))
                        })
                        .unwrap_or_else(|| json!({ "edits": [], "reason": "declined" }));
                    if let Some((outcome, owner, version_match, reason)) = trace {
                        self.log(|| format!(
                            "inputRoute operation={} outcome={outcome} reason={reason} owner={owner} version_match={version_match} elapsed_ms={}",
                            params.as_ref().map(|params| params.operation.as_str()).unwrap_or("unknown"),
                            started_at.elapsed().as_millis()
                        ));
                    }
                    self.respond(id, result)?;
                }
            }
            BLOCK_COMMENT_PAIR_METHOD => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let RequestCommand::Feature(FeatureCommand::BlockCommentPair(params)) =
                        &command
                    else {
                        unreachable!("block comment pair method has a typed command");
                    };
                    let params = params.clone();
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut version = -1i32;
                    let mut outcome = "no_edit";
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            version = params.version;
                            let query = self
                                .document_runtime
                                .capture_query(&log_uri, self.external_index.snapshot())?;
                            let document = query.document;
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
                    self.log(|| format!(
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
            ACTIVE_SCOPE_DELIMITERS_METHOD => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let RequestCommand::Feature(FeatureCommand::ActiveScopeDelimiters(params)) =
                        &command
                    else {
                        unreachable!("active scope delimiter method has typed parameters");
                    };
                    let params = params.clone();
                    let mut log_uri = "<missing>".to_string();
                    let mut requested_version = -1i32;
                    let mut response_version = -1i32;
                    let mut position_count = 0usize;
                    let mut pair_count = 0usize;
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            requested_version = params.version;
                            position_count = params.positions.len();
                            let query = self
                                .document_runtime
                                .capture_query(&log_uri, self.external_index.snapshot())?;
                            let document = query.document;
                            response_version = document.version;
                            if document.version != params.version {
                                return Some(json!({
                                    "version": document.version,
                                    "pairs": [],
                                }));
                            }
                            if document.text.len()
                                > crate::lsp::scope_delimiters::MAX_ACTIVE_SCOPE_DELIMITER_SOURCE_BYTES
                            {
                                return Some(json!({
                                    "version": document.version,
                                    "pairs": [],
                                }));
                            }
                            let Some(delimiters) = document
                                .foreground()
                                .map(|foreground| foreground.scope_delimiters())
                            else {
                                return Some(json!({
                                    "version": document.version,
                                    "pending": !document.analysis_rejected(),
                                    "pairs": [],
                                }));
                            };
                            let offsets = params
                                .positions
                                .iter()
                                .take(64)
                                .filter_map(|position| {
                                    offset_for_position(&document.text, *position)
                                })
                                .collect::<Vec<_>>();
                            let pairs = crate::lsp::scope_delimiters::active_scope_delimiters(
                                delimiters,
                                &offsets,
                            )
                            .into_iter()
                            .filter_map(|delimiter| {
                                let closer = delimiter.closer?;
                                Some(json!({
                                    "opener": {
                                        "start": position_for_offset(
                                            &document.text,
                                            delimiter.opener.start,
                                        ),
                                        "end": position_for_offset(
                                            &document.text,
                                            delimiter.opener.end,
                                        ),
                                    },
                                    "closer": {
                                        "start": position_for_offset(
                                            &document.text,
                                            closer.start,
                                        ),
                                        "end": position_for_offset(
                                            &document.text,
                                            closer.end,
                                        ),
                                    },
                                }))
                            })
                            .collect::<Vec<_>>();
                            pair_count = pairs.len();
                            Some(json!({
                                "version": document.version,
                                "pairs": pairs,
                            }))
                        })
                        .unwrap_or_else(|| json!({ "version": -1, "pairs": [] }));
                    self.log(|| format!(
                        "request activeScopeDelimiters uri={} requested_version={} response_version={} positions={} pairs={} elapsed_ms={}",
                        log_uri,
                        requested_version,
                        response_version,
                        position_count,
                        pair_count,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                }
            }
            RANGE_FORMATTING_METHOD => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let RequestCommand::Feature(FeatureCommand::RangeFormatting(params)) = &command
                    else {
                        unreachable!("range formatting method has a typed command");
                    };
                    let params = params.clone();
                    let mut log_uri = "<missing>".to_string();
                    let mut bytes = 0usize;
                    let mut version = -1i32;
                    let mut edit_count = 0usize;
                    let mut outcome = "no_edit";
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            let query = self
                                .document_runtime
                                .capture_query(&log_uri, self.external_index.snapshot())?;
                            let document = query.document;
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
                    self.log(|| format!(
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
                    let RequestCommand::Feature(FeatureCommand::SignatureHelp(params)) = &command
                    else {
                        unreachable!("signature help method has a typed command");
                    };
                    let params = params.clone();
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
                    let mut cached_analysis = false;
                    let mut external_index_status = self.external_index.status_summary().status;
                    let mut external_index_layers = "none";
                    let result = params
                        .and_then(|params| {
                            log_uri = params.text_document.uri;
                            self.document_runtime
                                .capture_query(&log_uri, self.external_index.snapshot())
                                .map(|query| {
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
                                        cached_analysis = true;
                                        signature_help_report_for_cached_analysis_with_external_indexes(
                                        &document.text,
                                        analysis,
                                        params.position,
                                        indexes.workspace.as_deref(),
                                        indexes.game_data.as_deref(),
                                    )
                                    }
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
                    self.log(|| format!(
                        "request signatureHelp uri={} bytes={} revision={} cached_analysis={} context={} active_parameter={} candidates={} selected={} failure_reason={} external_index_status={} external_index_layers={} parse_diagnostics={} context_ms={} lookup_ms={} render_ms={} queue_ms={} elapsed_ms={}",
                        log_uri,
                        bytes,
                        revision,
                        cached_analysis,
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
                if let Some(id) = message.id.clone() {
                    let start = Instant::now();
                    let RequestCommand::Feature(FeatureCommand::SemanticTokensFull(params)) =
                        &command
                    else {
                        unreachable!("semantic tokens method has a typed command");
                    };
                    let params = params.clone();
                    let external_index_summary = self.external_index.status_summary();
                    let external_index_status = external_index_summary.status;
                    let external_generation = external_index_summary.generation;
                    let mut selection = params
                        .as_ref()
                        .map(|params| {
                            self.document_runtime.select_semantic_tokens(
                                &params.text_document.uri,
                                external_generation,
                            )
                        })
                        .unwrap_or_else(|| {
                            self.document_runtime
                                .select_semantic_tokens("<missing>", external_generation)
                        });
                    if let Some((uri, rich_revision, rich_external_generation)) =
                        selection.rich_work.clone()
                    {
                        let external_indexes = self.external_index.snapshot_for_document_identity(
                            self.document_runtime.document_path_identity(&uri),
                        );
                        let effects = self.document_runtime.admit_rich_semantic_tokens(
                            &uri,
                            rich_revision,
                            external_indexes,
                            rich_external_generation,
                        );
                        for effect in effects {
                            self.deliver_effect(effect)?;
                        }
                        selection = self
                            .document_runtime
                            .select_semantic_tokens(&uri, external_generation);
                    }
                    if !selection.ready_to_publish && selection.rich_work.is_some() {
                        self.log(|| format!(
                            "request semanticTokens uri={} bytes={} revision={} cached_analysis=true mode={} outcome=rejected-rich-overload tokens={} external_index_status={} external_generation={} parse_diagnostics={} lex_ms={} token_loop_ms={} resolver_ms={} resolver_calls={} encode_ms={} queue_ms={} elapsed_ms={}",
                            selection.uri,
                            selection.bytes,
                            selection.revision,
                            selection.projection_mode,
                            selection.token_count,
                            external_index_status,
                            external_generation,
                            selection.parse_diagnostics,
                            selection.lex_ms,
                            selection.token_loop_ms,
                            selection.resolver_ms,
                            selection.resolver_calls,
                            selection.encode_ms,
                            queue_ms,
                            start.elapsed().as_millis()
                        ));
                        self.respond_error(id, -32801, "Content modified")?;
                        return Ok(self.finish(false));
                    }
                    if !selection.ready_to_publish {
                        let effects = self.document_runtime.defer_semantic_token_request(
                            &message,
                            command.clone(),
                            external_generation,
                        )?;
                        for effect in effects {
                            self.deliver_effect(effect)?;
                        }
                        self.log(|| format!(
                            "request semanticTokens uri={} bytes={} revision={} cached_analysis={} mode={} outcome=deferred tokens={} external_index_status={} external_generation={} parse_diagnostics={} lex_ms={} token_loop_ms={} resolver_ms={} resolver_calls={} encode_ms={} queue_ms={} elapsed_ms={}",
                            selection.uri,
                            selection.bytes,
                            selection.revision,
                            selection.projection_mode != "lexical-pending",
                            selection.projection_mode,
                            selection.token_count,
                            external_index_status,
                            external_generation,
                            selection.parse_diagnostics,
                            selection.lex_ms,
                            selection.token_loop_ms,
                            selection.resolver_ms,
                            selection.resolver_calls,
                            selection.encode_ms,
                            queue_ms,
                            start.elapsed().as_millis()
                        ));
                        return Ok(self.finish(false));
                    }
                    let result = serde_json::to_value(&selection.tokens).unwrap_or(Value::Null);
                    self.log(|| format!(
                        "request semanticTokens uri={} bytes={} revision={} cached_analysis=true mode={} outcome={} tokens={} external_index_status={} external_generation={} parse_diagnostics={} lex_ms={} token_loop_ms={} resolver_ms={} resolver_calls={} encode_ms={} queue_ms={} elapsed_ms={}",
                        selection.uri,
                        selection.bytes,
                        selection.revision,
                        selection.projection_mode,
                        "responded",
                        selection.token_count,
                        external_index_status,
                        external_generation,
                        selection.parse_diagnostics,
                        selection.lex_ms,
                        selection.token_loop_ms,
                        selection.resolver_ms,
                        selection.resolver_calls,
                        selection.encode_ms,
                        queue_ms,
                        start.elapsed().as_millis()
                    ));
                    self.respond(id, result)?;
                }
            }
            "textDocument/hover" => {
                if let Some(id) = message.id {
                    let start = Instant::now();
                    let RequestCommand::Feature(FeatureCommand::Hover(params)) = &command else {
                        unreachable!("hover method has a typed command");
                    };
                    let params = params.clone();
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
                            self.document_runtime
                                .capture_query(&log_uri, self.external_index.snapshot())
                                .map(|query| {
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
                    self.log(|| format!(
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
                    let RequestCommand::Feature(FeatureCommand::Definition(params)) = &command
                    else {
                        unreachable!("definition method has a typed command");
                    };
                    let params = params.clone();
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
                            self.document_runtime
                                .capture_query(&log_uri, self.external_index.snapshot())
                                .map(|query| {
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
                    self.log(|| format!(
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
                    let RequestCommand::Feature(FeatureCommand::DebugHover(params)) = &command
                    else {
                        unreachable!("debug hover method has a typed command");
                    };
                    let params = params.clone();
                    if let Some(ref params) = params {
                        if let Some(query) = self.document_runtime.capture_query(
                            &params.text_document.uri,
                            self.external_index.snapshot(),
                        ) {
                            let DocumentQuery {
                                document,
                                external_indexes: indexes,
                            } = query;
                            if let DocumentQueryState::Cached(analysis) =
                                DocumentQuery::state_for(document)
                            {
                                if self.document_runtime.has_runtime_worker() {
                                    let uri = params.text_document.uri.clone();
                                    let position = params.position;
                                    let revision = document.revision;
                                    let analysis = analysis.clone();
                                    let external_status = self.external_index.status_summary();
                                    let task = match self.document_runtime.admit_debug_capture(&uri)
                                    {
                                        Ok(task) => task,
                                        Err((retained_jobs, retained_bytes)) => {
                                            self.log(|| format!(
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
                                            return Ok(self.finish(false));
                                        }
                                    };
                                    self.document_runtime.schedule_debug(DebugRequestJob::Hover(
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
                                    return Ok(self.finish(false));
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
                            self.document_runtime
                                .capture_query(&log_uri, self.external_index.snapshot())
                                .and_then(|query| {
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
                    self.log(|| format!(
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
                    let RequestCommand::Feature(FeatureCommand::DebugCompletion(params)) = &command
                    else {
                        unreachable!("debug completion method has a typed command");
                    };
                    let params = params.clone();
                    if let Some(ref params) = params {
                        if let Some(query) = self.document_runtime.capture_query(
                            &params.text_document.uri,
                            self.external_index.snapshot(),
                        ) {
                            let DocumentQuery {
                                document,
                                external_indexes: indexes,
                            } = query;
                            if let DocumentQueryState::Cached(analysis) =
                                DocumentQuery::state_for(document)
                            {
                                if self.document_runtime.has_runtime_worker() {
                                    let uri = params.text_document.uri.clone();
                                    let position = params.position;
                                    let revision = document.revision;
                                    let analysis = analysis.clone();
                                    let task = match self.document_runtime.admit_debug_capture(&uri)
                                    {
                                        Ok(task) => task,
                                        Err((retained_jobs, retained_bytes)) => {
                                            self.log(|| format!(
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
                                            return Ok(self.finish(false));
                                        }
                                    };
                                    self.document_runtime.schedule_debug(
                                        DebugRequestJob::Completion(DebugCompletionJob {
                                            task,
                                            id,
                                            uri,
                                            position,
                                            revision,
                                            scheduled_at: start,
                                            analysis,
                                            external_snapshot: indexes,
                                        }),
                                    );
                                    return Ok(self.finish(false));
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
                            self.document_runtime
                                .capture_query(&log_uri, self.external_index.snapshot())
                                .and_then(|query| {
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
                    self.log(|| format!(
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
        let external_status = self.external_index.status_summary();
        for effect in self.document_runtime.observe_semantic_external_generation(
            external_status.generation,
            external_status.status,
            semantic_generation_preservation,
        ) {
            self.deliver_effect(effect)?;
        }
        let should_exit = self.shutdown_requested && method == "exit";
        self.effects.push(RuntimeEffect::diagnostic_lazy(
            self.diagnostic_logging,
            "rpc.completed",
            || {
                json!({
                    "method": method,
                    "outcome": if should_exit { "exit" } else { "complete" },
                    "elapsedMs": started_at.elapsed().as_millis(),
                })
            },
        ));
        Ok(self.finish(should_exit))
    }

    fn finish(&mut self, should_exit: bool) -> FeatureDispatchOutcome {
        FeatureDispatchOutcome {
            should_exit,
            effects: std::mem::take(&mut self.effects),
        }
    }

    fn deliver_effect(&mut self, effect: RuntimeEffect) -> Result<(), String> {
        self.effects.push(effect);
        Ok(())
    }

    fn log(&mut self, message: impl FnOnce() -> String) {
        if self.operational_logging {
            self.effects.push(RuntimeEffect::Log(message()));
        }
    }

    fn respond(&mut self, id: Value, result: Value) -> Result<(), String> {
        self.effects.push(RuntimeEffect::Response { id, result });
        Ok(())
    }

    fn respond_error(&mut self, id: Value, code: i32, message: &str) -> Result<(), String> {
        self.effects.push(RuntimeEffect::Error {
            id,
            code,
            message: message.to_string(),
        });
        Ok(())
    }
}
