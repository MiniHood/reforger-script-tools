use super::{
    coalescible_full_sync_did_change, LspServer, ServerEvent, INCOMING_EVENT_QUEUE_CAPACITY,
};
use serde_json::json;
use std::{collections::VecDeque, io::Write, sync::mpsc, time::Duration};

impl<W: Write> LspServer<W> {
    /// Owns channel polling and full-sync change coalescing before request routing.
    pub(super) fn run_message_channels(
        &mut self,
        incoming_receiver: mpsc::Receiver<ServerEvent>,
        internal_receiver: mpsc::Receiver<ServerEvent>,
    ) -> Result<(), String> {
        let mut deferred_incoming = VecDeque::new();
        loop {
            for _ in 0..INCOMING_EVENT_QUEUE_CAPACITY {
                match internal_receiver.try_recv() {
                    Ok(event) => self.handle_internal_event(event)?,
                    Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
                }
            }

            let next_event = deferred_incoming
                .pop_front()
                .map(Ok)
                .unwrap_or_else(|| incoming_receiver.recv_timeout(Duration::from_millis(100)));
            match next_event {
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
                        let Ok(next_event) = incoming_receiver.try_recv() else {
                            break;
                        };
                        let ServerEvent::Incoming {
                            received_at,
                            result: Ok(next_message),
                        } = next_event
                        else {
                            deferred_incoming.push_back(next_event);
                            break;
                        };
                        let Some(next_change) = coalescible_full_sync_did_change(&next_message)
                        else {
                            deferred_incoming.push_back(ServerEvent::Incoming {
                                received_at,
                                result: Ok(next_message),
                            });
                            break;
                        };
                        if next_change.uri != first_change.uri {
                            deferred_incoming.push_back(ServerEvent::Incoming {
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
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let external_status = self.external_index.status_summary();
                    for effect in self.document_runtime.observe_semantic_external_generation(
                        external_status.generation,
                        external_status.status,
                        None,
                    ) {
                        self.deliver_effect(effect)?;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        self.log("exit");
        self.logger
            .diagnostic("shutdown", json!({"outcome": "normal"}));
        self.logger.flush_diagnostics();
        Ok(())
    }
}
