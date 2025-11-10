//! Hydro Deploy integration for DFIR.
#![allow(clippy::allow_attributes, missing_docs, reason = "// TODO(mingwei)")]

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;

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
    let flow = local_set.run_until(flow.run());

    tokio::select! {
        _ = stop.1 => {},
        _ = flow => {}
    }
}

pub async fn launch_flow_containerized(mut flow: Dfir<'_>) {
    let local_set = tokio::task::LocalSet::new();
    local_set.run_until(flow.run()).await;
}

pub async fn init_no_ack_start<T: DeserializeOwned + Default>() -> DeployPorts<u32, T> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    eprintln!("INPUT: {input}");
    let trimmed = input.trim();

    let bind_config = serde_json::from_str::<InitConfig<u32>>(trimmed).unwrap();

    // config telling other services how to connect to me
    let mut bind_results: HashMap<String, ServerPort<u32>> = HashMap::new();
    let mut binds = HashMap::new();
    for (name, config) in bind_config.0 {
        let bound = config.bind().await;
        bind_results.insert(name.clone(), bound.server_port());
        binds.insert(name.clone(), bound);
    }

    let bind_serialized = serde_json::to_string(&bind_results).unwrap();
    println!("ready: {bind_serialized}");
    eprintln!("ready: {bind_serialized}");

    let mut start_buf = String::new();
    std::io::stdin().read_line(&mut start_buf).unwrap();
    eprintln!("START COMMAND: {start_buf}");
    let connection_defns = if start_buf.starts_with("start: ") {
        serde_json::from_str::<HashMap<String, ServerPort<u32>>>(
            start_buf.trim_start_matches("start: ").trim(),
        )
        .unwrap()
    } else {
        panic!("expected start");
    };

    eprintln!("PRE CONNECT/ACCEPT: connections_defn: {connection_defns:?}, binds: {binds:?}",);

    let (client_conns, server_conns) = futures::join!(
        connection_defns
            .into_iter()
            .map(|(name, defn)| async move {
                eprintln!("connecting to {name} {defn:?}");
                let r = (name, Connection::AsClient(defn.connect().await));
                eprintln!("connected to {} {:?}", r.0, r.1);
                r
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>(),
        binds
            .into_iter()
            .map(|(name, defn)| async move {
                eprintln!("accepting on {name} {defn:?}");
                let r = (name, Connection::AsServer(accept_bound(defn).await));
                eprintln!("accepted on {} {:?}", r.0, r.1);
                r
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
    );

    eprintln!(
        "POST CONNECT/ACCEPT. client_conns: {client_conns:?}, server_conns: {server_conns:?}"
    );

    let all_connected = client_conns
        .into_iter()
        .chain(server_conns.into_iter())
        .collect();

    eprintln!("POST CONNECT/ACCEPT 2: {all_connected:?}");

    DeployPorts {
        ports: RefCell::new(all_connected),
        meta: bind_config
            .1
            .map(|b| serde_json::from_str(&b).unwrap())
            .unwrap_or_default(),
    }
}

pub async fn init<T: DeserializeOwned + Default>() -> DeployPorts<u32, T> {
    let ret = init_no_ack_start::<T>().await;

    println!("ack start");
    eprintln!("ack start");

    ret
}
