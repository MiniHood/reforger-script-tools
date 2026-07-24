use super::{
    coalescible_full_sync_did_change, LspServer, ServerEvent, INCOMING_EVENT_QUEUE_CAPACITY,
};
use serde_json::json;
use std::{collections::VecDeque, io::Write, sync::mpsc};

impl<W: Write> LspServer<W> {
    /// Owns event delivery and full-sync change coalescing before request routing.
    pub(super) fn run_message_channels(
        &mut self,
        event_receiver: mpsc::Receiver<ServerEvent>,
    ) -> Result<(), String> {
        let mut deferred_events = VecDeque::new();
        loop {
            let next_event = deferred_events
                .pop_front()
                .map(Ok)
                .unwrap_or_else(|| event_receiver.recv());
            match next_event {
                Ok(ServerEvent::TransportClosed) => break,
                Ok(ServerEvent::Incoming {
                    received_at,
                    result: Ok(message),
                }) => {
                    let mut selected_message = message;
                    let mut selected_received_at = received_at;
                    let mut coalesced_changes = 1usize;
                    let mut superseded_changes = 0usize;
                    let Some(first_change) = coalescible_full_sync_did_change(&selected_message)
                    else {
                        if self.handle_message(
                            selected_message,
                            Some(selected_received_at.elapsed().as_millis()),
                            0,
                            0,
                        )? {
                            break;
                        }
                        continue;
                    };
                    while coalesced_changes < INCOMING_EVENT_QUEUE_CAPACITY {
                        let Ok(next_event) = event_receiver.try_recv() else {
                            break;
                        };
                        let ServerEvent::Incoming {
                            received_at,
                            result: Ok(next_message),
                        } = next_event
                        else {
                            deferred_events.push_back(next_event);
                            break;
                        };
                        let Some(next_change) = coalescible_full_sync_did_change(&next_message)
                        else {
                            deferred_events.push_back(ServerEvent::Incoming {
                                received_at,
                                result: Ok(next_message),
                            });
                            break;
                        };
                        if next_change.uri != first_change.uri {
                            deferred_events.push_back(ServerEvent::Incoming {
                                received_at,
                                result: Ok(next_message),
                            });
                            break;
                        }
                        coalesced_changes += 1;
                        if next_change.version
                            > coalescible_full_sync_did_change(&selected_message)
                                .expect("selected message remains coalescible")
                                .version
                        {
                            selected_message = next_message;
                            selected_received_at = received_at;
                        }
                        superseded_changes += 1;
                    }
                    if self.handle_message(
                        selected_message,
                        Some(selected_received_at.elapsed().as_millis()),
                        coalesced_changes,
                        superseded_changes,
                    )? {
                        break;
                    }
                }
                Ok(ServerEvent::Incoming {
                    result: Err(error), ..
                }) => return Err(error),
                Ok(event) => self.handle_internal_event(event)?,
                Err(mpsc::RecvError) => break,
            }
        }
        self.log("exit");
        self.logger
            .diagnostic("shutdown", json!({"outcome": "normal"}));
        self.logger.flush_diagnostics();
        Ok(())
    }
}
