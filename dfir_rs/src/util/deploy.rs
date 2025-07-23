//! Hydro Deploy integration for DFIR.
#![allow(clippy::allow_attributes, missing_docs, reason = "// TODO(mingwei)")]

use std::cell::RefCell;
use std::collections::HashMap;

use futures::StreamExt;
use futures::stream::FuturesUnordered;
pub use hydro_deploy_integration::*;
use serde::de::DeserializeOwned;

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

pub async fn launch_flow(mut flow: Dfir<'_>) {
    let stop = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        if line.starts_with("stop") {
            stop.0.send(()).unwrap();
        } else {
            eprintln!("Unexpected stdin input: {:?}", line);
        }
    });

    let local_set = tokio::task::LocalSet::new();
    let flow = local_set.run_until(flow.run_async());

    tokio::select! {
        _ = stop.1 => {},
        _ = flow => {}
    }
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
