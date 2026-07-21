use super::LspServer;
use serde_json::{json, Value};
use std::io::Write;

impl<W: Write> LspServer<W> {
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
