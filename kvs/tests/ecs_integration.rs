//! ECS integration tests — run against a live HydroKvsPublic CloudFormation stack.
//!
//! These tests discover the NLB endpoint automatically from the stack outputs.
//! They require AWS credentials with cloudformation:DescribeStacks permission
//! and network access to the NLB.
//!
//! Run with: cargo nextest run -p kvs --test ecs_integration
//!
//! The stack name and region can be overridden via environment variables:
//!   KVS_STACK_NAME  (default: HydroKvsPublic)
//!   AWS_REGION      (default: us-east-1)

use kvs::testing::{
    new_trace_id, run_kvs_test, send_and_check_get, send_and_check_put, send_recv, send_recv_ws,
};
use kvs::{KvsCommand, KvsResponse, REPLICATION_FACTOR};
use tokio::sync::OnceCell;

const CLUSTER_SIZE: usize = 6;

struct EcsEndpoints {
    grpc: String,
    ws: String,
}

static ENDPOINTS: OnceCell<EcsEndpoints> = OnceCell::const_new();

async fn get_endpoints() -> &'static EcsEndpoints {
    ENDPOINTS
        .get_or_init(|| async {
            let stack_name =
                std::env::var("KVS_STACK_NAME").unwrap_or_else(|_| "HydroKvsPublic".to_string());
            let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

            let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(region))
                .load()
                .await;
            let cfn = aws_sdk_cloudformation::Client::new(&config);

            let resp = cfn
                .describe_stacks()
                .stack_name(&stack_name)
                .send()
                .await
                .unwrap_or_else(|e| panic!("Failed to describe stack {stack_name}: {e}"));

            let stack = resp
                .stacks()
                .first()
                .unwrap_or_else(|| panic!("Stack {stack_name} not found"));

            let mut grpc = None;
            let mut ws = None;

            for output in stack.outputs() {
                let key = output.output_key().unwrap_or_default();
                let value = output.output_value().unwrap_or_default();
                if key.contains("loc1v10") {
                    grpc = Some(value.to_string());
                } else if key.contains("loc1v11") {
                    ws = Some(value.to_string());
                }
            }

            let grpc = grpc
                .unwrap_or_else(|| panic!("gRPC endpoint not found in stack {stack_name} outputs"));
            let ws = ws.unwrap_or_else(|| {
                panic!("WebSocket endpoint not found in stack {stack_name} outputs")
            });

            eprintln!("Discovered endpoints — gRPC: {grpc}, WebSocket: {ws}");
            EcsEndpoints { grpc, ws }
        })
        .await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ecs_put_and_get() {
    let ep = get_endpoints().await;
    send_and_check_put(&ep.grpc, "ecs_test_key", "ecs_test_value").await;
    send_and_check_get(&ep.grpc, "ecs_test_key").await;
}

#[tokio::test]
async fn test_ecs_missing_key() {
    let ep = get_endpoints().await;
    let resp = send_recv(
        &ep.grpc,
        &KvsCommand::Get {
            trace_id: new_trace_id(),
            key: format!("nonexistent_{}", new_trace_id()),
        },
    )
    .await;
    assert!(
        matches!(&resp, KvsResponse::GetResult { value: None, .. }),
        "expected None for missing key, got {resp:?}"
    );
}

#[tokio::test]
async fn test_ecs_overwrite() {
    let ep = get_endpoints().await;
    let key = format!("overwrite_{}", new_trace_id());

    send_and_check_put(&ep.grpc, &key, "first").await;
    send_and_check_put(&ep.grpc, &key, "second").await;

    let resp = send_recv(
        &ep.grpc,
        &KvsCommand::Get {
            trace_id: new_trace_id(),
            key: key.clone(),
        },
    )
    .await;
    match &resp {
        KvsResponse::GetResult { value: Some(v), .. } => {
            assert!(
                v.contains("second"),
                "expected overwritten value 'second', got {v:?}"
            );
            assert!(
                !v.contains("first"),
                "old value 'first' should be overwritten, got {v:?}"
            );
        }
        other => panic!("expected GetResult with value, got {other:?}"),
    }
}

#[tokio::test]
async fn test_ecs_replication_factor() {
    let ep = get_endpoints().await;
    let key = format!("repfactor_{}", new_trace_id());

    let resp = send_recv(
        &ep.grpc,
        &KvsCommand::Put {
            trace_id: new_trace_id(),
            key: key.clone(),
            value: "check_rep".into(),
        },
    )
    .await;

    let node_ids = match &resp {
        KvsResponse::PutOk { node_ids, .. } => node_ids,
        other => panic!("expected PutOk, got {other:?}"),
    };
    assert_eq!(
        node_ids.len(),
        REPLICATION_FACTOR,
        "expected {REPLICATION_FACTOR} replicas, got {node_ids:?}"
    );
}

#[tokio::test]
async fn test_ecs_full_suite() {
    let ep = get_endpoints().await;
    run_kvs_test(&ep.grpc, CLUSTER_SIZE).await;
}

#[tokio::test]
#[ignore = "requires outbound port 81 — WebSocket NLB listener"]
async fn test_ecs_websocket_put_get() {
    let ep = get_endpoints().await;
    let key = format!("ws_test_{}", new_trace_id());
    let value = "ws_value";

    let put_resp = send_recv_ws(
        &ep.ws,
        &KvsCommand::Put {
            trace_id: new_trace_id(),
            key: key.clone(),
            value: value.into(),
        },
    )
    .await;
    assert!(
        matches!(&put_resp, KvsResponse::PutOk { key: k, .. } if k == &key),
        "expected PutOk, got {put_resp:?}"
    );

    let get_resp = send_recv_ws(
        &ep.ws,
        &KvsCommand::Get {
            trace_id: new_trace_id(),
            key: key.clone(),
        },
    )
    .await;
    match &get_resp {
        KvsResponse::GetResult {
            value: Some(vs), ..
        } if vs.contains(&value.to_string()) => {}
        other => panic!("WS read-back failed for {key}: {other:?}"),
    }

    send_and_check_get(&ep.grpc, &key).await;
}
