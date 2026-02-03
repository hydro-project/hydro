//! Echo server for Maelstrom.
//!
//! This implements the Maelstrom echo workload, which simply echoes back
//! any message it receives from clients.

use hydro_lang::prelude::*;

/// Creates an echo server flow for Maelstrom.
///
/// Takes a keyed input stream of (client_id, message_body) and returns
/// a keyed output stream of (client_id, response_body).
pub fn echo_server<'a, C>(
    input: KeyedStream<String, serde_json::Value, Cluster<'a, C>>,
) -> KeyedStream<String, serde_json::Value, Cluster<'a, C>> {
    input.map(q!(|body| {
        // Extract the msg_id from the request
        let msg_id = body.get("msg_id").and_then(|v| v.as_u64());

        // Build the echo response
        let mut response = serde_json::json!({
            "type": "echo_ok"
        });

        // Copy the echo field from request to response
        if let Some(echo) = body.get("echo") {
            response["echo"] = echo.clone();
        }

        // Add in_reply_to if msg_id was present
        if let Some(id) = msg_id {
            response["in_reply_to"] = serde_json::json!(id);
        }

        response
    }))
}
