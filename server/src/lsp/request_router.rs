//! JSON-RPC request classification boundary.
//!
//! This module validates one envelope and assigns it to a runtime contract.
//! It does not read document state or perform transport work.
use super::workspace_requests::{
    LoadedAddonGraphParams, WorkspaceFileChangedParams, WorkspaceFileDeletedParams,
};
use super::{
    validate_message_params, ActiveScopeDelimiterParams, BlockCommentPairParams, CompletionParams,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentSymbolParams, HoverParams, InputRouteParams, RangeFormattingParams, RpcMessage,
    ACTIVE_SCOPE_DELIMITERS_METHOD, BLOCK_COMMENT_PAIR_METHOD, CONTROL_HEADER_ENTER_METHOD,
    DEBUG_COMPLETION_METHOD, DEBUG_HOVER_METHOD, LOADED_ADDON_GRAPH_METHOD, PREVIEW_CONTEXT_METHOD,
    RANGE_FORMATTING_METHOD, READ_PACK_SOURCE_METHOD, WORKSPACE_FILE_CHANGED_METHOD,
    WORKSPACE_FILE_DELETED_METHOD,
};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RequestCommand {
    Lifecycle(LifecycleCommand),
    Document(DocumentCommand),
    Feature(FeatureCommand),
    WorkspaceIndex(WorkspaceIndexCommand),
    Cancellation,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LifecycleCommand {
    Initialize,
    Initialized,
    Shutdown,
    Exit,
    Response,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DocumentCommand {
    Open(Option<DidOpenTextDocumentParams>),
    Change(Option<DidChangeTextDocumentParams>),
    Close(Option<DidCloseTextDocumentParams>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WorkspaceIndexCommand {
    Changed(Option<WorkspaceFileChangedParams>),
    Deleted(Option<WorkspaceFileDeletedParams>),
    LoadedAddonGraph(Option<LoadedAddonGraphParams>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FeatureCommand {
    DocumentSymbols(Option<DocumentSymbolParams>),
    Completion(Option<CompletionParams>),
    SignatureHelp(Option<HoverParams>),
    SemanticTokensFull(Option<DocumentSymbolParams>),
    Hover(Option<HoverParams>),
    Definition(Option<HoverParams>),
    RangeFormatting(Option<RangeFormattingParams>),
    DebugHover(Option<HoverParams>),
    DebugCompletion(Option<HoverParams>),
    BlockCommentPair(Option<BlockCommentPairParams>),
    InputRoute(Option<InputRouteParams>),
    ActiveScopeDelimiters(Option<ActiveScopeDelimiterParams>),
    PreviewContext(Option<HoverParams>),
    ReadPackSource(Option<super::ReadPackSourceParams>),
    OtherTextDocument,
}

#[derive(Debug, Clone)]
pub(super) struct RoutedRequest {
    pub(super) command: RequestCommand,
    pub(super) message: RpcMessage,
    pub(super) parameter_error: Option<String>,
}

pub(super) fn classify_request(value: Value) -> Result<RoutedRequest, String> {
    let message = serde_json::from_value::<RpcMessage>(value.clone())
        .map_err(|error| format!("Invalid JSON-RPC message: {error}"))?;
    let command = match message.method.as_deref() {
        None => RequestCommand::Lifecycle(LifecycleCommand::Response),
        Some("$/cancelRequest") => RequestCommand::Cancellation,
        Some("initialize") => RequestCommand::Lifecycle(LifecycleCommand::Initialize),
        Some("initialized") => RequestCommand::Lifecycle(LifecycleCommand::Initialized),
        Some("shutdown") => RequestCommand::Lifecycle(LifecycleCommand::Shutdown),
        Some("exit") => RequestCommand::Lifecycle(LifecycleCommand::Exit),
        Some("textDocument/didOpen") => {
            RequestCommand::Document(DocumentCommand::Open(parse_typed_params(&message.params)))
        }
        Some("textDocument/didChange") => {
            RequestCommand::Document(DocumentCommand::Change(parse_typed_params(&message.params)))
        }
        Some("textDocument/didClose") => {
            RequestCommand::Document(DocumentCommand::Close(parse_typed_params(&message.params)))
        }
        Some(WORKSPACE_FILE_CHANGED_METHOD) => RequestCommand::WorkspaceIndex(
            WorkspaceIndexCommand::Changed(parse_typed_params(&message.params)),
        ),
        Some(WORKSPACE_FILE_DELETED_METHOD) => RequestCommand::WorkspaceIndex(
            WorkspaceIndexCommand::Deleted(parse_typed_params(&message.params)),
        ),
        Some(LOADED_ADDON_GRAPH_METHOD) => RequestCommand::WorkspaceIndex(
            WorkspaceIndexCommand::LoadedAddonGraph(parse_typed_params(&message.params)),
        ),
        Some("textDocument/documentSymbol") => RequestCommand::Feature(
            FeatureCommand::DocumentSymbols(parse_typed_params(&message.params)),
        ),
        Some("textDocument/completion") => RequestCommand::Feature(FeatureCommand::Completion(
            parse_typed_params(&message.params),
        )),
        Some("textDocument/signatureHelp") => RequestCommand::Feature(
            FeatureCommand::SignatureHelp(parse_typed_params(&message.params)),
        ),
        Some("textDocument/semanticTokens/full") => RequestCommand::Feature(
            FeatureCommand::SemanticTokensFull(parse_typed_params(&message.params)),
        ),
        Some("textDocument/hover") => {
            RequestCommand::Feature(FeatureCommand::Hover(parse_typed_params(&message.params)))
        }
        Some("textDocument/definition") => RequestCommand::Feature(FeatureCommand::Definition(
            parse_typed_params(&message.params),
        )),
        Some(RANGE_FORMATTING_METHOD) => RequestCommand::Feature(FeatureCommand::RangeFormatting(
            parse_typed_params(&message.params),
        )),
        Some(DEBUG_HOVER_METHOD) => RequestCommand::Feature(FeatureCommand::DebugHover(
            parse_typed_params(&message.params),
        )),
        Some(DEBUG_COMPLETION_METHOD) => RequestCommand::Feature(FeatureCommand::DebugCompletion(
            parse_typed_params(&message.params),
        )),
        Some(BLOCK_COMMENT_PAIR_METHOD) => RequestCommand::Feature(
            FeatureCommand::BlockCommentPair(parse_typed_params(&message.params)),
        ),
        Some(CONTROL_HEADER_ENTER_METHOD) => RequestCommand::Feature(FeatureCommand::InputRoute(
            parse_typed_params(&message.params),
        )),
        Some(ACTIVE_SCOPE_DELIMITERS_METHOD) => RequestCommand::Feature(
            FeatureCommand::ActiveScopeDelimiters(parse_typed_params(&message.params)),
        ),
        Some(PREVIEW_CONTEXT_METHOD) => RequestCommand::Feature(FeatureCommand::PreviewContext(
            parse_typed_params(&message.params),
        )),
        Some(READ_PACK_SOURCE_METHOD) => RequestCommand::Feature(FeatureCommand::ReadPackSource(
            parse_typed_params(&message.params),
        )),
        Some(method) if method.starts_with("textDocument/") => {
            RequestCommand::Feature(FeatureCommand::OtherTextDocument)
        }
        Some(_) => RequestCommand::Unknown,
    };
    let parameter_error = message
        .method
        .as_deref()
        .and_then(|method| validate_message_params(method, &message.params).err());
    Ok(RoutedRequest {
        command,
        message,
        parameter_error,
    })
}

fn parse_typed_params<T: for<'de> serde::Deserialize<'de>>(params: &Option<Value>) -> Option<T> {
    params
        .as_ref()
        .and_then(|params| serde_json::from_value(params.clone()).ok())
}

#[cfg(test)]
mod tests {
    use super::{
        classify_request, DocumentCommand, FeatureCommand, LifecycleCommand, RequestCommand,
        WorkspaceIndexCommand,
    };
    use serde_json::json;

    #[test]
    fn classifies_protocol_messages_without_document_runtime_state() {
        for (value, expected) in [
            (
                json!({"method": "initialize"}),
                RequestCommand::Lifecycle(LifecycleCommand::Initialize),
            ),
            (
                json!({"method": "textDocument/didOpen"}),
                RequestCommand::Document(DocumentCommand::Open(None)),
            ),
            (
                json!({"method": "textDocument/hover"}),
                RequestCommand::Feature(FeatureCommand::Hover(None)),
            ),
            (
                json!({"method": "reforger/workspaceFileChanged"}),
                RequestCommand::WorkspaceIndex(WorkspaceIndexCommand::Changed(None)),
            ),
            (
                json!({"method": "reforger/loadedAddonGraph"}),
                RequestCommand::WorkspaceIndex(WorkspaceIndexCommand::LoadedAddonGraph(None)),
            ),
            (
                json!({"method": "$/cancelRequest"}),
                RequestCommand::Cancellation,
            ),
        ] {
            assert_eq!(classify_request(value).unwrap().command, expected);
        }
    }

    #[test]
    fn captures_feature_payloads_and_reports_invalid_feature_parameters() {
        let routed = classify_request(json!({
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": "file:///script.c"},
                "position": {"line": 2, "character": 4}
            }
        }))
        .unwrap();
        let RequestCommand::Feature(FeatureCommand::Hover(Some(params))) = routed.command else {
            panic!("hover must retain its typed payload");
        };
        assert_eq!(params.text_document.uri, "file:///script.c");
        assert_eq!(params.position.line, 2);

        let routed = classify_request(json!({
            "method": "reforger/previewContext",
            "params": {
                "textDocument": {"uri": "file:///script.c"},
                "position": {"line": 8, "character": 0}
            }
        }))
        .unwrap();
        let RequestCommand::Feature(FeatureCommand::PreviewContext(Some(params))) = routed.command
        else {
            panic!("preview context must retain its document position");
        };
        assert_eq!(params.text_document.uri, "file:///script.c");
        assert_eq!(params.position.line, 8);

        let routed = classify_request(json!({
            "method": "reforger/readPackSource",
            "params": {"uri": "reforger-pak://58D0FB3206B6F859/scripts/Game/Example.c"}
        }))
        .unwrap();
        let RequestCommand::Feature(FeatureCommand::ReadPackSource(Some(params))) = routed.command
        else {
            panic!("pack source requests must retain their typed URI");
        };
        assert_eq!(
            params.uri,
            "reforger-pak://58D0FB3206B6F859/scripts/Game/Example.c"
        );

        let routed = classify_request(json!({
            "method": "textDocument/hover",
            "params": {"textDocument": {"uri": "file:///script.c"}}
        }))
        .unwrap();
        assert!(matches!(
            routed.command,
            RequestCommand::Feature(FeatureCommand::Hover(None))
        ));
        assert!(routed.parameter_error.is_some());
    }
}
