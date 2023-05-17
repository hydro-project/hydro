#![allow(missing_docs)] // TODO(mingwei)

use std::collections::HashMap;

pub use hydroflow_cli_integration::*;
use hydroflow_cli_integration::{ServerBindConfig, ServerPort};

use crate::scheduled::graph::Hydroflow;

pub async fn launch_flow(mut flow: Hydroflow) {
    let stop = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        assert!(line.starts_with("stop"));
        stop.0.send(()).unwrap();
    });

    tokio::select! {
        _ = stop.1 => {},
        _ = flow.run_async() => {}
    }
}

pub struct HydroCLI {
    ports: HashMap<String, ServerOrBound>,
}

impl HydroCLI {
    pub fn port(&mut self, name: &str) -> ServerOrBound {
        self.ports.remove(name).unwrap()
    }
}

pub async fn init() -> HydroCLI {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();

    let bind_config = serde_json::from_str::<HashMap<String, ServerBindConfig>>(trimmed).unwrap();

    // config telling other services how to connect to me
    let mut bind_results: HashMap<String, ServerPort> = HashMap::new();
    let mut binds = HashMap::new();
    for (name, config) in bind_config {
        let bound = config.bind().await;
        bind_results.insert(name.clone(), bound.sink_port());
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

    let mut all_connected = HashMap::new();
    for (name, defn) in connection_defns {
        all_connected.insert(name, ServerOrBound::Server(defn));
    }

    for (name, defn) in binds {
        all_connected.insert(name, ServerOrBound::Bound(defn));
    }

    HydroCLI {
        ports: all_connected,
    }
}
