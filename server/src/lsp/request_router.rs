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
    Lifecycle,
    Document,
    Feature,
    WorkspaceIndex,
    Cancellation,
    Unknown,
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
        None => RequestCommand::Lifecycle,
        Some("$/cancelRequest") => RequestCommand::Cancellation,
        Some("initialize" | "initialized" | "shutdown" | "exit") => RequestCommand::Lifecycle,
        Some("textDocument/didOpen" | "textDocument/didChange" | "textDocument/didClose") => {
            RequestCommand::Document
        }
        Some(WORKSPACE_FILE_CHANGED_METHOD | WORKSPACE_FILE_DELETED_METHOD) => {
            RequestCommand::WorkspaceIndex
        }
        Some(method)
            if method.starts_with("textDocument/")
                || matches!(
                    method,
                    DEBUG_HOVER_METHOD
                        | DEBUG_COMPLETION_METHOD
                        | BLOCK_COMMENT_PAIR_METHOD
                        | ENTER_TYPING_ASSIST_METHOD
                        | RANGE_FORMATTING_METHOD
                ) =>
        {
            RequestCommand::Feature
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
    use super::{classify_request, RequestCommand};
    use serde_json::json;

    #[test]
    fn classifies_protocol_messages_without_document_runtime_state() {
        for (value, expected) in [
            (json!({"method": "initialize"}), RequestCommand::Lifecycle),
            (
                json!({"method": "textDocument/didOpen"}),
                RequestCommand::Document,
            ),
            (
                json!({"method": "textDocument/hover"}),
                RequestCommand::Feature,
            ),
            (
                json!({"method": "reforger/workspaceFileChanged"}),
                RequestCommand::WorkspaceIndex,
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
