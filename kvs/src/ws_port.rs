//! WebSocket sidecar — a per-connection streaming bridge between
//! the Hydro dataflow and WebSocket clients.
//!
//! Unlike [`crate::grpc_port`], whose reply targets are per-request
//! oneshots, this sidecar's reply targets are **per-connection writer
//! channels**: one mpsc per connection that drives that connection's WS
//! sink.
//!
//! ## Envelope and correlation
//!
//! The dataflow-side envelope is plain `(u64, KvsCommand)` /
//! `(u64, KvsResponse)` — same shape as gRPC. The `u64`
//! is a sidecar-assigned globally-unique monotonic `SeqId`.
//!
//! The sidecar keeps two side tables:
//! * `conns: HashMap<ConnId, Sender<Message>>` — each connection's
//!   outbound frame channel, which feeds a writer task owning that
//!   connection's WS sink.
//! * `seq_to_conn: HashMap<SeqId, ConnId>` — records which connection
//!   a given in-flight command came in on, so the response can be
//!   routed back when it arrives.
//!
//! On an inbound frame: assign a fresh `SeqId`, insert `seq_id →
//! conn_id`, push `(seq_id, cmd)` into the dataflow.
//! On an inbound response: look up `seq_id → conn_id`, then `conn_id →
//! writer`, then send the serialized response on the writer.
//! On connection close: evict all `seq_id` entries pointing at the
//! closed `conn_id`, plus the `conn_id → writer` entry.
//!
//! This keeps the framework envelope the same shape as gRPC
//! (`(u64, T)`) while the sidecar's *internal* correlation differs:
//! per-connection mpsc instead of per-request oneshot. The framework
//! primitive is now type-parametric, so the envelope shape is the
//! *sidecar's* choice, not imposed by the framework.
//!
//! ## Wire format
//!
//! JSON on top of WebSocket text frames. Each inbound frame is one
//! `serde_json`-encoded [`KvsCommand`]; each outbound frame is one
//! `serde_json`-encoded [`KvsResponse`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_tungstenite::tungstenite::Message;

use crate::{KvsCommand, KvsResponse};

/// Sidecar-local connection identifier. Assigned at accept time.
pub type ConnId = u64;
/// Sidecar-local command sequence identifier, globally unique and
/// monotonic across the sidecar's lifetime. This is the `u64` that
/// rides the dataflow envelope.
pub type SeqId = u64;

static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_SEQ_ID: AtomicU64 = AtomicU64::new(1);

fn next_conn_id() -> ConnId {
    NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed)
}

fn next_seq_id() -> SeqId {
    NEXT_SEQ_ID.fetch_add(1, Ordering::Relaxed)
}

/// State shared between the accept loop, the per-connection readers,
/// and the response dispatcher.
struct Shared {
    /// Per-connection outbound frame channels. Drained by each
    /// connection's writer task.
    conns: HashMap<ConnId, Sender<Message>>,
    /// Records which connection each in-flight command came in on so
    /// responses can be routed back.
    seq_to_conn: HashMap<SeqId, ConnId>,
}

/// Entry point for the Hydro `bidi_external_sidecar` sidecar.
pub async fn kvs_ws_sidecar(
    listener: TcpListener,
    cmds_tx: Sender<(SeqId, KvsCommand)>,
    mut resp_rx: Receiver<(SeqId, KvsResponse)>,
) {
    let shared: Arc<Mutex<Shared>> = Arc::new(Mutex::new(Shared {
        conns: HashMap::new(),
        seq_to_conn: HashMap::new(),
    }));

    // Response dispatcher: route each dataflow response back to the
    // originating connection.
    let dispatch_shared = shared.clone();
    tokio::spawn(async move {
        while let Some((seq_id, resp)) = resp_rx.recv().await {
            let (writer, conn_id_for_log) = {
                let mut st = dispatch_shared.lock().unwrap();
                let conn_id = st.seq_to_conn.remove(&seq_id);
                let writer = conn_id.and_then(|c| st.conns.get(&c).cloned());
                (writer, conn_id)
            };
            if let Some(tx) = writer {
                match serde_json::to_string(&resp) {
                    Ok(body) => {
                        if tx.send(Message::Text(body)).await.is_err() {
                            tracing::debug!(
                                name: "ws_dispatch_writer_closed",
                                %seq_id, conn_id = ?conn_id_for_log
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            name: "ws_serialize_error",
                            %seq_id, conn_id = ?conn_id_for_log, error = %e
                        );
                    }
                }
            } else {
                tracing::debug!(
                    name: "ws_response_for_closed_conn",
                    %seq_id, conn_id = ?conn_id_for_log
                );
            }
        }
    });

    let addr = listener.local_addr().ok();
    tracing::info!(name: "ws_sidecar_listen", ?addr);

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let conn_id = next_conn_id();
                let cmds_tx = cmds_tx.clone();
                let shared = shared.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(conn_id, stream, cmds_tx, shared).await {
                        tracing::warn!(name: "ws_connection_error", %conn_id, %peer, error = %e);
                    }
                });
            }
            Err(e) => {
                tracing::warn!(name: "ws_accept_error", error = %e);
            }
        }
    }
}

async fn handle_connection(
    conn_id: ConnId,
    stream: TcpStream,
    cmds_tx: Sender<(SeqId, KvsCommand)>,
    shared: Arc<Mutex<Shared>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    let (mut ws_sink, mut ws_source) = ws.split();

    // Per-connection outbound channel. The writer task drains it into
    // the WS sink; the response dispatcher pushes into it via the
    // `Sender` cloned into `shared.conns`.
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(64);
    shared.lock().unwrap().conns.insert(conn_id, out_tx);

    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
        let _ = ws_sink.close().await;
    });

    tracing::info!(name: "ws_connection_open", %conn_id);

    // Reader loop: deserialize inbound frames, stamp with a fresh
    // `SeqId`, record the `seq_id → conn_id` mapping, push the command
    // into the dataflow.
    while let Some(msg) = ws_source.next().await {
        let msg = msg?;
        let text = match msg {
            Message::Text(t) => t,
            Message::Binary(b) => String::from_utf8(b)?,
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            Message::Close(_) => break,
        };
        let cmd: KvsCommand = serde_json::from_str(&text)?;
        let seq_id = next_seq_id();
        let trace_id = match &cmd {
            KvsCommand::Put { trace_id, .. } | KvsCommand::Get { trace_id, .. } => trace_id.clone(),
        };
        shared.lock().unwrap().seq_to_conn.insert(seq_id, conn_id);
        tracing::info!(name: "ws_request_in", %conn_id, %seq_id, %trace_id);
        if cmds_tx.send((seq_id, cmd)).await.is_err() {
            return Err("dataflow command channel closed".into());
        }
    }

    // Connection is closing: evict all its state from `shared` so
    // future responses for orphaned seq_ids are dropped cleanly.
    {
        let mut st = shared.lock().unwrap();
        st.conns.remove(&conn_id);
        st.seq_to_conn.retain(|_, cid| *cid != conn_id);
    }
    let _ = writer.await;
    tracing::info!(name: "ws_connection_close", %conn_id);

    Ok(())
}
