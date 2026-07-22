//! JSON-RPC request classification boundary.
//!
//! This module validates one envelope and assigns it to a runtime contract.
//! It does not read document state or perform transport work.
use super::{
    validate_message_params, RpcMessage, BLOCK_COMMENT_PAIR_METHOD, DEBUG_COMPLETION_METHOD,
    DEBUG_HOVER_METHOD, ENTER_TYPING_ASSIST_METHOD, RANGE_FORMATTING_METHOD,
    WORKSPACE_FILE_CHANGED_METHOD, WORKSPACE_FILE_DELETED_METHOD,
};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DocumentCommand {
    Open,
    Change,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceIndexCommand {
    Changed,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FeatureCommand {
    DocumentSymbols,
    Completion,
    SignatureHelp,
    SemanticTokensFull,
    Hover,
    Definition,
    RangeFormatting,
    DebugHover,
    DebugCompletion,
    BlockCommentPair,
    EnterTypingAssist,
    OtherTextDocument,
}

#[derive(Debug)]
pub(super) struct RoutedRequest {
    pub(super) command: RequestCommand,
    pub(super) message: RpcMessage,
    pub(super) value: Value,
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
        Some("textDocument/didOpen") => RequestCommand::Document(DocumentCommand::Open),
        Some("textDocument/didChange") => RequestCommand::Document(DocumentCommand::Change),
        Some("textDocument/didClose") => RequestCommand::Document(DocumentCommand::Close),
        Some(WORKSPACE_FILE_CHANGED_METHOD) => {
            RequestCommand::WorkspaceIndex(WorkspaceIndexCommand::Changed)
        }
        Some(WORKSPACE_FILE_DELETED_METHOD) => {
            RequestCommand::WorkspaceIndex(WorkspaceIndexCommand::Deleted)
        }
        Some("textDocument/documentSymbol") => {
            RequestCommand::Feature(FeatureCommand::DocumentSymbols)
        }
        Some("textDocument/completion") => RequestCommand::Feature(FeatureCommand::Completion),
        Some("textDocument/signatureHelp") => {
            RequestCommand::Feature(FeatureCommand::SignatureHelp)
        }
        Some("textDocument/semanticTokens/full") => {
            RequestCommand::Feature(FeatureCommand::SemanticTokensFull)
        }
        Some("textDocument/hover") => RequestCommand::Feature(FeatureCommand::Hover),
        Some("textDocument/definition") => RequestCommand::Feature(FeatureCommand::Definition),
        Some(RANGE_FORMATTING_METHOD) => RequestCommand::Feature(FeatureCommand::RangeFormatting),
        Some(DEBUG_HOVER_METHOD) => RequestCommand::Feature(FeatureCommand::DebugHover),
        Some(DEBUG_COMPLETION_METHOD) => RequestCommand::Feature(FeatureCommand::DebugCompletion),
        Some(BLOCK_COMMENT_PAIR_METHOD) => {
            RequestCommand::Feature(FeatureCommand::BlockCommentPair)
        }
        Some(ENTER_TYPING_ASSIST_METHOD) => {
            RequestCommand::Feature(FeatureCommand::EnterTypingAssist)
        }
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
        value,
        parameter_error,
    })
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
                RequestCommand::Document(DocumentCommand::Open),
            ),
            (
                json!({"method": "textDocument/hover"}),
                RequestCommand::Feature(FeatureCommand::Hover),
            ),
            (
                json!({"method": "reforger/workspaceFileChanged"}),
                RequestCommand::WorkspaceIndex(WorkspaceIndexCommand::Changed),
            ),
            (
                json!({"method": "$/cancelRequest"}),
                RequestCommand::Cancellation,
            ),
        ] {
            assert_eq!(classify_request(value).unwrap().command, expected);
        }
    }
}
