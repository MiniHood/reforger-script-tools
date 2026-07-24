//! Worker-event interpretation boundary.
//!
//! The composition root supplies its captured immutable external snapshot;
//! document runtime turns the event into transport-neutral effects.
use super::{DocumentRuntime, ExternalIndexSnapshot, RuntimeEffect, ServerEvent};

pub(super) fn interpret_background_event(
    runtime: &mut DocumentRuntime,
    event: ServerEvent,
    external_generation: u64,
    external_indexes: ExternalIndexSnapshot,
) -> Option<Result<Vec<RuntimeEffect>, String>> {
    runtime.interpret_event(event, external_generation, external_indexes)
}
