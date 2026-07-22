use super::LspServer;
use serde_json::{json, Value};
use std::io::Write;

/// A transport action requested by runtime-owned work. Only `LspServer`
/// delivers these effects, keeping JSON-RPC framing out of the runtime.
pub(super) enum RuntimeEffect {
    Log(String),
    ReplayDeferred { value: Value, queue_ms: u128 },
    ScheduleRich { uri: String, revision: u64, external_generation: u64 },
    Notification(Value),
    Response {
        id: Value,
        result: Value,
    },
    Error {
        id: Value,
        code: i32,
        message: String,
    },
}

impl<W: Write> LspServer<W> {
    pub(super) fn deliver_effect(&mut self, effect: RuntimeEffect) -> Result<(), String> {
        match effect {
            RuntimeEffect::Log(message) => {
                self.log(&message);
                Ok(())
            }
            RuntimeEffect::ReplayDeferred { value, queue_ms } => {
                self.handle_message(value, Some(queue_ms), 0, 0).map(|_| ())
            }
            RuntimeEffect::ScheduleRich { uri, revision, external_generation } => {
                self.schedule_rich_semantic_tokens(&uri, revision, external_generation)
            }
            RuntimeEffect::Notification(message) => self.write_message(message),
            RuntimeEffect::Response { id, result } => self.respond(id, result),
            RuntimeEffect::Error { id, code, message } => self.respond_error(id, code, &message),
        }
    }

    /// Serializes and frames protocol responses; request routing owns only response choices.
    pub(super) fn respond(&mut self, id: Value, result: Value) -> Result<(), String> {
        self.write_message(json!({"jsonrpc": "2.0", "id": id, "result": result}))
    }

    pub(super) fn respond_error(
        &mut self,
        id: Value,
        code: i32,
        message: &str,
    ) -> Result<(), String> {
        self.write_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": code, "message": message}
        }))
    }

    pub(super) fn write_message(&mut self, value: Value) -> Result<(), String> {
        let body = serde_json::to_vec(&value)
            .map_err(|error| format!("Failed to serialize LSP response: {error}"))?;
        write!(self.writer, "Content-Length: {}\r\n\r\n", body.len())
            .map_err(|error| format!("Failed to write LSP header: {error}"))?;
        self.writer
            .write_all(&body)
            .map_err(|error| format!("Failed to write LSP body: {error}"))?;
        self.writer
            .flush()
            .map_err(|error| format!("Failed to flush LSP response: {error}"))
    }
}
