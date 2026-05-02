//! Sidecar that runs a tonic-generated gRPC server alongside the Hydro
//! dataflow on each router. Handlers bridge client requests into the
//! dataflow via an mpsc channel and await responses via a `oneshot` map
//! indexed by request id.
//!
//! Wired up in `complete_distributed_kvs` via the same
//! [`hydro_lang::Location::bidi_external_sidecar`] primitive.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::net::TcpListener;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use crate::{KvsCommand, KvsResponse};

// The tonic-generated module (output of build.rs's tonic_build call).
pub mod pb {
    tonic::include_proto!("kvs");
}

use pb::kvs_server::{Kvs, KvsServer};
use pb::{GetRequest, GetResponse, PutRequest, PutResponse, ValueSetMsg, VectorClockMsg};

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> u64 {
    NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<KvsResponse>>>>;

struct KvsGrpcService {
    cmds_tx: Sender<(u64, KvsCommand)>,
    pending: PendingMap,
}

#[tonic::async_trait]
impl Kvs for KvsGrpcService {
    async fn put(&self, req: Request<PutRequest>) -> Result<Response<PutResponse>, Status> {
        let PutRequest {
            trace_id,
            key,
            value,
        } = req.into_inner();
        let id = next_request_id();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);

        tracing::info!(name: "request_start", %trace_id, method = "PUT", %key, body_len = value.len());

        self.cmds_tx
            .send((
                id,
                KvsCommand::Put {
                    trace_id: trace_id.clone(),
                    key: key.clone(),
                    value: value.clone(),
                },
            ))
            .await
            .map_err(|_| Status::internal("dataflow command channel closed"))?;

        let resp = rx
            .await
            .map_err(|_| Status::internal("response oneshot dropped"))?;
        tracing::info!(name: "request_end", %trace_id, request_id = id, ?resp);

        match resp {
            KvsResponse::PutOk {
                trace_id,
                key,
                existing_vc,
                node_ids,
            } => Ok(Response::new(PutResponse {
                trace_id,
                key,
                existing_vc: existing_vc.map(|vc| VectorClockMsg {
                    entries: vc.into_iter().collect(),
                }),
                node_ids: node_ids.into_iter().collect(),
            })),
            other => {
                tracing::error!(name: "grpc_put_wrong_response_kind", ?other);
                Err(Status::internal("unexpected response kind"))
            }
        }
    }

    async fn get(&self, req: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let GetRequest { trace_id, key } = req.into_inner();
        let id = next_request_id();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);

        tracing::info!(name: "request_start", %trace_id, method = "GET", %key);

        self.cmds_tx
            .send((
                id,
                KvsCommand::Get {
                    trace_id: trace_id.clone(),
                    key: key.clone(),
                },
            ))
            .await
            .map_err(|_| Status::internal("dataflow command channel closed"))?;

        let resp = rx
            .await
            .map_err(|_| Status::internal("response oneshot dropped"))?;
        tracing::info!(name: "request_end", %trace_id, request_id = id, ?resp);

        match resp {
            KvsResponse::GetResult {
                trace_id,
                key,
                value,
                existing_vc,
                node_ids,
            } => Ok(Response::new(GetResponse {
                trace_id,
                key,
                value: value.map(|vs| ValueSetMsg {
                    values: vs.into_iter().collect(),
                }),
                existing_vc: existing_vc.map(|vc| VectorClockMsg {
                    entries: vc.into_iter().collect(),
                }),
                node_ids: node_ids.into_iter().collect(),
            })),
            other => {
                tracing::error!(name: "grpc_get_wrong_response_kind", ?other);
                Err(Status::internal("unexpected response kind"))
            }
        }
    }
}

/// Entry point for the Hydro `bidi_external_sidecar` sidecar.
pub async fn kvs_grpc_sidecar(
    listener: TcpListener,
    cmds_tx: Sender<(u64, KvsCommand)>,
    mut resp_rx: Receiver<(u64, KvsResponse)>,
) {
    let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));

    // Drain task: pump dataflow responses into the pending oneshots.
    let drain_pending = pending.clone();
    tokio::spawn(async move {
        while let Some((id, resp)) = resp_rx.recv().await {
            let sender = drain_pending.lock().unwrap().remove(&id);
            if let Some(tx) = sender {
                let _ = tx.send(resp);
            } else {
                tracing::warn!(name: "grpc_sidecar_response_no_pending", request_id = id);
            }
        }
    });

    let addr = listener.local_addr().ok();
    tracing::info!(name: "grpc_sidecar_listen", ?addr);

    let service = KvsGrpcService { cmds_tx, pending };
    Server::builder()
        .add_service(KvsServer::new(service))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await
        .expect("grpc_sidecar tonic server");
}
