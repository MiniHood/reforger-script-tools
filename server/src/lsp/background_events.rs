use super::{LspServer, ServerEvent};
use std::io::Write;

impl<W: Write> LspServer<W> {
    pub(super) fn handle_background_event(&mut self, event: ServerEvent) -> Result<(), String> {
        let external_generation = self.external_index.status_summary().generation;
        let external_indexes = self.external_index.snapshot();
        let Some(result) =
            self.document_runtime
                .interpret_event(event, external_generation, external_indexes)
        else {
            return Ok(());
        };
        for effect in result? {
            self.deliver_effect(effect)?;
        }
        Ok(())
    }
}
