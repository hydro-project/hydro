//! Runtime support for Maelstrom deployment backend.
//!
//! This module provides the runtime code that runs inside Maelstrom nodes,
//! handling stdin/stdout JSON message passing according to the Maelstrom protocol.

#![allow(
    unused,
    reason = "unused in trybuild but the __staged version is needed"
)]
#![allow(missing_docs, reason = "used internally")]

use std::io::{BufRead, Write};

use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use stageleft::{QuotedWithContext, RuntimeData, q};

use crate::forward_handle::ForwardHandle;
use crate::live_collections::boundedness::Unbounded;
use crate::live_collections::keyed_stream::KeyedStream;
use crate::live_collections::stream::{ExactlyOnce, NoOrder, TotalOrder};
use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{Cluster, LocationKey, MembershipEvent, NoTick};
use crate::nondet::nondet;

/// Maelstrom message envelope structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaelstromMessage<T> {
    pub src: String,
    pub dest: String,
    pub body: T,
}

/// Maelstrom init message body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitBody {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub msg_id: u64,
    pub node_id: String,
    pub node_ids: Vec<String>,
}

/// Maelstrom init_ok response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitOkBody {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub in_reply_to: u64,
}

/// Metadata for a Maelstrom node, populated from the init message.
/// Also manages a shared stdin reader that broadcasts lines to multiple subscribers.
pub struct MaelstromMeta {
    pub node_id: String,
    pub node_ids: Vec<String>,
    stdin_tx: tokio::sync::broadcast::Sender<String>,
}

impl MaelstromMeta {
    /// Subscribe to stdin lines. Each subscriber receives all lines read from stdin.
    /// Multiple subscribers can be created and each will receive a copy of every line.
    pub fn subscribe_stdin(&self) -> tokio_stream::wrappers::BroadcastStream<String> {
        tokio_stream::wrappers::BroadcastStream::new(self.stdin_tx.subscribe())
    }

    /// Start receiving incoming messages from clients and other nodes, after launching the DFIR.
    pub fn start_receiving(&self) {
        let tx_clone = self.stdin_tx.clone();

        // Spawn thread to read stdin and broadcast lines
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                match line {
                    Ok(l) => {
                        // Ignore send errors (no receivers)
                        let _ = tx_clone.send(l);
                    }
                    Err(_) => break,
                }
            }
        });
    }
}

/// Initialize a Maelstrom node by reading the init message from stdin.
/// Returns the node metadata and sends init_ok response.
/// Also spawns a background thread to read stdin and broadcast lines to subscribers.
pub fn maelstrom_init() -> MaelstromMeta {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    // Read the init message
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .expect("Failed to read init message");

    let msg: MaelstromMessage<InitBody> =
        serde_json::from_str(&line).expect("Failed to parse init message");

    assert_eq!(msg.body.msg_type, "init", "First message must be init");

    // Set up broadcast channel for stdin lines
    let (stdin_tx, _) = tokio::sync::broadcast::channel::<String>(1024);

    let meta = MaelstromMeta {
        node_id: msg.body.node_id.clone(),
        node_ids: msg.body.node_ids.clone(),
        stdin_tx,
    };

    // Send init_ok response
    let response = MaelstromMessage {
        src: msg.body.node_id,
        dest: msg.src,
        body: InitOkBody {
            msg_type: "init_ok".to_string(),
            in_reply_to: msg.body.msg_id,
        },
    };

    let response_json = serde_json::to_string(&response).expect("Failed to serialize init_ok");
    writeln!(stdout, "{}", response_json).expect("Failed to write init_ok");
    stdout.flush().expect("Failed to flush stdout");

    meta
}

/// Get the cluster member IDs from the Maelstrom metadata.
/// The `meta` parameter is a RuntimeData reference to the MaelstromMeta that will be
/// available at runtime as `__hydro_lang_maelstrom_meta`.
pub(super) fn cluster_members<'a>(
    meta: RuntimeData<&'a MaelstromMeta>,
    _of_cluster: LocationKey,
) -> impl QuotedWithContext<'a, &'a [TaglessMemberId], ()> + Clone + 'a {
    q!({
        let members: &'static [TaglessMemberId] = Box::leak(
            meta.node_ids
                .iter()
                .map(|id| TaglessMemberId::from_maelstrom_node_id(id.clone()))
                .collect::<Vec<TaglessMemberId>>()
                .into_boxed_slice(),
        );
        members
    })
}

/// Get the self ID for this cluster member.
pub(super) fn cluster_self_id<'a>(
    meta: RuntimeData<&'a MaelstromMeta>,
) -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
    q!(TaglessMemberId::from_maelstrom_node_id(
        meta.node_id.clone()
    ))
}

/// Get the cluster membership stream (static for Maelstrom - all members join at start).
/// This references `__hydro_lang_maelstrom_meta` which will be in scope at runtime.
pub(super) fn cluster_membership_stream<'a>(
    _location_id: &LocationId,
) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
{
    let meta: RuntimeData<&MaelstromMeta> = RuntimeData::new("__hydro_lang_maelstrom_meta");
    q!(Box::new(futures::stream::iter(
        meta.node_ids
            .iter()
            .map(|id| (
                TaglessMemberId::from_maelstrom_node_id(id.clone()),
                MembershipEvent::Joined
            ))
            .collect::<Vec<_>>()
    ))
        as Box<
            dyn futures::Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin,
        >)
}

/// Create sink and source for m2m (cluster member to cluster member) communication.
/// Messages are routed through Maelstrom's network via stdin/stdout.
pub(super) fn deploy_maelstrom_m2m(meta: RuntimeData<&MaelstromMeta>) -> (syn::Expr, syn::Expr) {
    // Sink: serialize and write to stdout with Maelstrom message envelope
    let sink_expr = q!({
        let node_id = meta.node_id.clone();
        sinktools::map(
            move |(dest_id, payload): (TaglessMemberId, bytes::Bytes)| {
                let msg = serde_json::json!({
                    "src": node_id,
                    "dest": dest_id.get_maelstrom_node_id(),
                    "body": {
                        "type": "hydro_data",
                        "data": payload.to_vec()
                    }
                });
                serde_json::to_string(&msg).unwrap() + "\n"
            },
            futures::sink::unfold((), |(), line: String| {
                Box::pin(async move {
                    print!("{}", line);
                    std::io::stdout().flush().ok();
                    Ok::<_, std::io::Error>(())
                })
            }),
        )
    })
    .splice_untyped_ctx(&());

    // Source: subscribe to the shared stdin broadcast stream
    let source_expr = q!({
        let node_ids: std::collections::HashSet<String> = meta.node_ids.iter().cloned().collect();
        let lines = meta.subscribe_stdin();
        futures::StreamExt::filter_map(lines, move |line_result| {
            let node_ids = node_ids.clone();
            Box::pin(async move {
                let line = line_result.ok()?;
                let mut msg =
                    serde_json::from_str::<MaelstromMessage<serde_json::Value>>(&line).ok()?;
                // Only process messages from other nodes (not clients)
                if msg
                    .body
                    .get("type")
                    .is_some_and(|t| t.as_str() == Some("hydro_data"))
                {
                    let deser: Vec<u8> =
                        serde_json::from_value(msg.body.get_mut("data").unwrap().take()).unwrap();
                    Some(Ok::<_, std::io::Error>((
                        TaglessMemberId::from_maelstrom_node_id(msg.src),
                        bytes::BytesMut::from(&deser[..]),
                    )))
                } else {
                    None
                }
            })
        })
    })
    .splice_untyped_ctx(&());

    (sink_expr, source_expr)
}

/// Creates a stream of client messages from Maelstrom stdin.
/// Returns tuples of (client_id, message_body) where client_id is the source client
/// and message_body is the JSON value of the message body.
///
/// This function is meant to be used with `source_stream` on a Cluster location.
pub fn maelstrom_client_source(
    meta: &MaelstromMeta,
) -> impl Stream<Item = (String, serde_json::Value)> + Unpin {
    use std::collections::HashSet;

    let node_ids: HashSet<String> = meta.node_ids.iter().cloned().collect();
    let lines = meta.subscribe_stdin();

    Box::pin(lines.filter_map(move |line_result| {
        let node_ids = node_ids.clone();
        async move {
            let line = line_result.ok()?;
            let msg: MaelstromMessage<serde_json::Value> = serde_json::from_str(&line).ok()?;
            // Only process messages from clients (not other nodes)
            if !node_ids.contains(&msg.src) {
                Some((msg.src, msg.body))
            } else {
                None
            }
        }
    }))
}

/// Sends a response to a Maelstrom client via stdout.
///
/// This function is meant to be used with `for_each` on a stream of responses.
pub fn maelstrom_send_response(node_id: &str, client_id: &str, body: serde_json::Value) {
    use std::io::Write;

    let msg = MaelstromMessage {
        src: node_id.to_string(),
        dest: client_id.to_string(),
        body,
    };

    let json = serde_json::to_string(&msg).expect("Failed to serialize response");
    println!("{}", json);
    std::io::stdout().flush().ok();
}
