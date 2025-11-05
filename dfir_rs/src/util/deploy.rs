//! Hydro Deploy integration for DFIR.
#![allow(clippy::allow_attributes, missing_docs, reason = "// TODO(mingwei)")]

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::SystemTime;

use futures::StreamExt;
use futures::stream::FuturesUnordered;
pub use hydro_deploy_integration::*;
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::scheduled::graph::Dfir;

#[macro_export]
macro_rules! launch {
    ($f:expr) => {
        async {
            let ports = $crate::util::deploy::init_no_ack_start().await;
            let flow = $f(&ports);

            println!("ack start");

            $crate::util::deploy::launch_flow(flow).await
        }
    };
}

pub use crate::launch;
use crate::scheduled::metrics::DfirMetrics;

pub async fn launch_flow(flow: Dfir<'_>) {
    let read_stop = async {
        use tokio::io::AsyncBufReadExt;

        let stdin = tokio::io::BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.starts_with("stop") {
                return;
            } else {
                eprintln!("Unexpected stdin input: {:?}", line);
            }
        }
        // Yield forever.
        let _ = std::future::pending::<std::convert::Infallible>().await;
    };

    let local_set = tokio::task::LocalSet::new();
    let flow = local_set.run_until(async move {
        use tokio::fs::OpenOptions;
        use tokio::io::{AsyncWriteExt, BufWriter};

        let mut flow = flow;
        let mut metrics = flow.metrics();

        // TODO!!! MAKE THIS GENERIC.

        for i in 0.. {
            let _work_done = flow.run_tick().await;

            // Metrics
            if 0 == i % 100 {
                let () = do_metrics(&flow, &mut metrics).await;
            }

            while !flow.can_start_tick() {
                let _ = flow.recv_events_async().await;
            }
        }

        async fn do_metrics(_flow: &Dfir<'_>, metrics: &mut DfirMetrics) {
            // Create an output file write to `/var/log/hydro.log`
            let file_result = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .append(true)
                .open("/var/log/hydro/metrics.log")
                .await;
            let file = match file_result {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Failed to create metrics file: {}", e);
                    return;
                }
            };
            let mut writer = BufWriter::new(file);

            // Get updated metrics.
            // Emit metrics.
            // TODO(mingwei)!!! MAKE THIS GENERIC.
            let ts = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis();

            // Handoffs
            for handoff_id in metrics.handoff_ids() {
                let handoff_metrics = metrics.handoff_metrics(handoff_id);
                let emf = json!({
                    "_aws": {
                        "Timestamp": ts,
                        "CloudWatchMetrics": [
                            {
                                "Namespace": "Hydro/HandoffMetrics",
                                "Dimensions": [[], ["HandoffId"]],
                                "Metrics": [
                                    {"Name": "CurrItemsCount", "Unit": "Count"},
                                    {"Name": "TotalItemsCount", "Unit": "Count"},
                                ]
                            }
                        ]
                    },
                    "HandoffId": handoff_id.to_string(),
                    "CurrItemsCount": handoff_metrics.curr_items_count(),
                    "TotalItemsCount": handoff_metrics.total_items_count(),
                })
                .to_string();
                writer.write_all(emf.as_bytes()).await.unwrap();
                writer.write_u8(b'\n').await.unwrap();
            }

            // Subgraphs
            for sg_id in metrics.subgraph_ids() {
                let sg_metrics = metrics.subgraph_metrics(sg_id);
                let emf = json!({
                    "_aws": {
                        "Timestamp": ts,
                        "CloudWatchMetrics": [
                            {
                                "Namespace": "Hydro/SubgraphMetrics",
                                "Dimensions": [[], ["SubgraphId"]],
                                "Metrics": [
                                    {"Name": "TotalRunCount", "Unit": "Count"},
                                    {"Name": "TotalPollDuration", "Unit": "Microseconds"},
                                    {"Name": "TotalPollCount", "Unit": "Count"},
                                    {"Name": "TotalIdleDuration", "Unit": "Microseconds"},
                                    {"Name": "TotalIdleCount", "Unit": "Count"},
                                ]
                            }
                        ]
                    },
                    "SubgraphId": sg_id.to_string(),
                    "TotalRunCount": sg_metrics.total_run_count(),
                    "TotalPollDuration": sg_metrics.total_poll_duration().as_micros(),
                    "TotalPollCount": sg_metrics.total_poll_count(),
                    "TotalIdleDuration": sg_metrics.total_idle_duration().as_micros(),
                    "TotalIdleCount": sg_metrics.total_idle_count(),
                })
                .to_string();
                writer.write_all(emf.as_bytes()).await.unwrap();
                writer.write_u8(b'\n').await.unwrap();

                writer.flush().await.unwrap();
            }
            // Reset metrics.
            metrics.reset();

            // Tokio RuntimeMetrics
            {
                let rt_metrics = tokio::runtime::Handle::current().metrics();
                let emf = json!({
                    "_aws": {
                        "Timestamp": ts,
                        "CloudWatchMetrics": [
                            {
                                "Namespace": "Hydro/TokioRuntimeMetrics",
                                "Dimensions": [[]],
                                "Metrics": [
                                    // TODO(mingwei): for example for now
                                    {"Name": "NumAliveTasks", "Unit": "Count"},
                                    {"Name": "GlobalQueueDepth", "Unit": "Count"},
                                ]
                            }
                        ]
                    },
                    "NumAliveTasks": rt_metrics.num_alive_tasks(),
                    "GlobalQueueDepth": rt_metrics.global_queue_depth(),
                })
                .to_string();
                writer.write_all(emf.as_bytes()).await.unwrap();
                writer.write_u8(b'\n').await.unwrap();
            }

            writer.flush().await.unwrap();
            writer.shutdown().await.unwrap();
        }
    });

    tokio::select! {
        _ = flow => {
              // Should be unreachable.
        },
        () = read_stop => {
            // Exit triggered.
            return;
        },
    }
}

pub async fn launch_flow_containerized(mut flow: Dfir<'_>) {
    let local_set = tokio::task::LocalSet::new();
    local_set.run_until(flow.run()).await;
}

pub async fn init_no_ack_start<T: DeserializeOwned + Default>() -> DeployPorts<T> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();

    let bind_config = serde_json::from_str::<InitConfig>(trimmed).unwrap();

    // config telling other services how to connect to me
    let mut bind_results: HashMap<String, ServerPort> = HashMap::new();
    let mut binds = HashMap::new();
    for (name, config) in bind_config.0 {
        let bound = config.bind().await;
        bind_results.insert(name.clone(), bound.server_port());
        binds.insert(name.clone(), bound);
    }

    let bind_serialized = serde_json::to_string(&bind_results).unwrap();
    println!("ready: {bind_serialized}");

    let mut start_buf = String::new();
    std::io::stdin().read_line(&mut start_buf).unwrap();
    let connection_defns = if start_buf.starts_with("start: ") {
        serde_json::from_str::<HashMap<String, ServerPort>>(
            start_buf.trim_start_matches("start: ").trim(),
        )
        .unwrap()
    } else {
        panic!("expected start");
    };

    let (client_conns, server_conns) = futures::join!(
        connection_defns
            .into_iter()
            .map(|(name, defn)| async move { (name, Connection::AsClient(defn.connect().await)) })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>(),
        binds
            .into_iter()
            .map(
                |(name, defn)| async move { (name, Connection::AsServer(accept_bound(defn).await)) }
            )
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
    );

    let all_connected = client_conns
        .into_iter()
        .chain(server_conns.into_iter())
        .collect();

    DeployPorts {
        ports: RefCell::new(all_connected),
        meta: bind_config
            .1
            .map(|b| serde_json::from_str(&b).unwrap())
            .unwrap_or_default(),
    }
}

pub async fn init<T: DeserializeOwned + Default>() -> DeployPorts<T> {
    let ret = init_no_ack_start::<T>().await;

    println!("ack start");

    ret
}
