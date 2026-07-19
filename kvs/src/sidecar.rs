//! A **gRPC** sidecar that bridges external clients to the dataflow, used to
//! expose the store to real clients under a deployment (see
//! [`crate::kvs_deploy`]).
//!
//! This is the [`sidecar_bidi`](hydro_lang::prelude::Location::sidecar_bidi)
//! escape hatch: it runs a background task on the deployed machine that owns a
//! [`tonic`] gRPC server. A router member calls [`create`] to get back a
//! `(Stream<ClientRequest>, Sink<ClientResponse>)` pair — client RPCs decoded by
//! the server flow into the dataflow as [`ClientRequest`]s, and the
//! [`ClientResponse`]s the dataflow produces are matched back to the awaiting
//! RPC handlers. Because each router member runs its own server on its own
//! container, a client connects to exactly one router.
//!
//! The client-facing interface is the `KvStore` gRPC service defined in
//! `proto/kvs.proto` (unary `Put`/`Get`). The internal `req_id` used to
//! correlate a dataflow response back to its originating RPC is generated here
//! and never appears on the client wire.
//!
//! Everything that touches the generated protobuf types is gated behind
//! `#[cfg(stageleft_runtime)]`: [`create`] is referenced from a `q!` closure in
//! [`crate::kvs_deploy`], so stageleft re-exports it into the generated
//! `__staged` module. That generated crate is compiled without our `build.rs`
//! proto output on its include path, so any module-level item referencing the
//! `pb` types would fail to resolve there. `stageleft_runtime` is set for every
//! real build but stripped from staged codegen, so the gated items exist exactly
//! where they compile — and [`create`]'s signature (the only thing `__staged`
//! actually re-exports) stays free of protobuf types.

use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{ClientRequest, ClientResponse};

/// The generated gRPC bindings for `proto/kvs.proto`.
#[cfg(stageleft_runtime)]
pub mod pb {
    tonic::include_proto!("kvs");
}

/// Create a sidecar that serves the `KvStore` gRPC interface on `port` and
/// bridges it to the dataflow.
///
/// Spawns a background [`tonic`] server that, for each `Put`/`Get` RPC:
/// 1. mints an internal `req_id` and registers a one-shot channel for the reply,
/// 2. forwards a [`ClientRequest`] into the dataflow (the returned `Stream`),
/// 3. awaits the matching [`ClientResponse`] delivered via the returned `Sink`
///    (a background task routes each response to the waiting RPC by `req_id`),
/// 4. translates that response into the RPC's protobuf reply.
///
/// The returned `(Stream, Sink)` pair is what `sidecar_bidi` wires into the
/// dataflow; the concrete types match its expectations (no boxing needed).
///
/// This function is deliberately **not** `#[cfg(stageleft_runtime)]`-gated even
/// though its body uses the gated [`pb`] types: stageleft re-exports it into the
/// generated `__staged` module (copying any cfg attrs onto the re-export), and
/// the trybuild crate has no `stageleft_runtime` cfg, so gating it would strip
/// the re-export and break the `q!` reference in [`crate::kvs_deploy`]. The body
/// is only ever compiled in the real crate (where `stageleft_runtime` is set and
/// [`pb`] exists), never copied into `__staged`.
pub fn create(port: u16) -> (ReceiverStream<ClientRequest>, PollSender<ClientResponse>) {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, Ordering};

    use tokio::sync::mpsc;
    use tokio::sync::oneshot;
    use tonic::{Request, Response, Status};

    use pb::kv_store_server::{KvStore, KvStoreServer};
    use pb::{GetReply, GetRequest, PutReply, PutRequest};
    use crate::Op;

    // TCP-side RPC handlers → dataflow (backs the returned Stream).
    let (to_df_tx, to_df_rx) = mpsc::channel::<ClientRequest>(1024);
    // dataflow → response router (backs the returned Sink).
    let (from_df_tx, mut from_df_rx) = mpsc::channel::<ClientResponse>(1024);

    // Correlates each in-flight RPC's `req_id` with the one-shot channel its
    // handler is blocked on. A single response stream comes back from the
    // dataflow (unordered), so we route each one to its awaiting handler here.
    type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<ClientResponse>>>>;
    let pending: Pending = Arc::new(Mutex::new(HashMap::new()));

    // Route dataflow responses back to the RPC handler waiting on each `req_id`.
    {
        let pending = pending.clone();
        tokio::spawn(async move {
            while let Some(resp) = from_df_rx.recv().await {
                let req_id = match &resp {
                    ClientResponse::PutAck { req_id } => *req_id,
                    ClientResponse::GetResult { req_id, .. } => *req_id,
                };
                if let Some(tx) = pending.lock().unwrap().remove(&req_id) {
                    // The receiver is gone only if the client hung up; ignore.
                    let _ = tx.send(resp);
                }
            }
        });
    }

    /// The `tonic` service: turns each RPC into a dataflow request and awaits
    /// its correlated response.
    struct KvStoreService {
        to_df: mpsc::Sender<ClientRequest>,
        pending: Pending,
        next_req_id: AtomicU64,
    }

    impl KvStoreService {
        /// Submit `op` to the dataflow under a fresh `req_id` and await its
        /// response.
        async fn submit(&self, op: Op) -> Result<ClientResponse, Status> {
            let req_id = self.next_req_id.fetch_add(1, Ordering::Relaxed);
            let (tx, rx) = oneshot::channel();
            self.pending.lock().unwrap().insert(req_id, tx);

            if self
                .to_df
                .send(ClientRequest { req_id, op })
                .await
                .is_err()
            {
                self.pending.lock().unwrap().remove(&req_id);
                return Err(Status::unavailable("dataflow closed"));
            }

            rx.await
                .map_err(|_| Status::unavailable("dataflow dropped the response"))
        }
    }

    #[tonic::async_trait]
    impl KvStore for KvStoreService {
        async fn put(
            &self,
            request: Request<PutRequest>,
        ) -> Result<Response<PutReply>, Status> {
            let PutRequest { key, value } = request.into_inner();
            match self.submit(Op::Put { key, value }).await? {
                ClientResponse::PutAck { .. } => Ok(Response::new(PutReply {})),
                ClientResponse::GetResult { .. } => {
                    Err(Status::internal("expected a put ack, got a get result"))
                }
            }
        }

        async fn get(
            &self,
            request: Request<GetRequest>,
        ) -> Result<Response<GetReply>, Status> {
            let GetRequest { key } = request.into_inner();
            match self.submit(Op::Get { key }).await? {
                ClientResponse::GetResult { values, .. } => Ok(Response::new(GetReply {
                    values: values.into_iter().collect(),
                })),
                ClientResponse::PutAck { .. } => {
                    Err(Status::internal("expected a get result, got a put ack"))
                }
            }
        }
    }

    let service = KvStoreService {
        to_df: to_df_tx,
        pending,
        next_req_id: AtomicU64::new(0),
    };

    tokio::spawn(async move {
        let addr: SocketAddr = ([0, 0, 0, 0], port).into();
        tonic::transport::Server::builder()
            .add_service(KvStoreServer::new(service))
            .serve(addr)
            .await
            .unwrap();
    });

    let stream = ReceiverStream::new(to_df_rx);
    let sink = PollSender::new(from_df_tx);

    (stream, sink)
}
