use bytes::BytesMut;
use hydro_lang::prelude::*;

use super::protocol::{WebSocketMessage, websocket_protocol};

pub fn websocket_echo<'a, P>(
    in_stream: KeyedStream<u64, BytesMut, Process<'a, P>, Unbounded>,
) -> KeyedStream<u64, BytesMut, Process<'a, P>, Unbounded> {
    websocket_protocol(in_stream, |messages, _| {
        messages
            .filter_map(q!(|msg| {
                match msg {
                    WebSocketMessage::Text(text) => Some(text),
                    _ => None,
                }
            }))
            .inspect_with_key(q!(|(id, text)| println!(
                "Received text message from {}: {}",
                id, text
            )))
            .map(q!(|text| {
                let echo_response = format!("Echo: {}", text.to_uppercase());
                WebSocketMessage::Text(echo_response)
            }))
    })
}
