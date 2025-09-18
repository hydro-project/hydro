use bytes::BytesMut;
use hydro_lang::prelude::*;

use super::protocol::{WebSocketMessage, websocket_protocol};

pub fn websocket_chat<'a, P>(
    process: &Process<'a, P>,
    in_stream: KeyedStream<u64, BytesMut, Process<'a, P>, Unbounded>,
) -> KeyedStream<u64, BytesMut, Process<'a, P>, Unbounded> {
    websocket_protocol(in_stream, |messages, open_connections| {
        let messages = messages
            .filter_map(q!(|msg| {
                match msg {
                    WebSocketMessage::Text(text) => Some(text),
                    _ => None,
                }
            }))
            .inspect_with_key(q!(|(id, text)| println!(
                "Received text message from {}: {}",
                id, text
            )));

        let tick = process.tick();
        open_connections
            .snapshot(
                &tick,
                nondet!(
                    /** depending on when we take a snapshot, we will send to different members */
                ),
            )
            .with_identical_values(
                messages
                    .entries()
                    .batch(
                        &tick,
                        nondet!(
                            /** same timing effects as the snapshot */
                        ),
                    )
                    .assume_ordering(
                        nondet!(/** arbitrary interleaving of messages across clients */),
                    ),
            )
            .all_ticks()
            .map(q!(|(sender, text)| {
                let echo_response = format!("From {}: {}", sender, text);
                WebSocketMessage::Text(echo_response)
            }))
    })
}
