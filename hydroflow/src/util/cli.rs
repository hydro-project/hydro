#![allow(missing_docs)] // TODO(mingwei)

use std::cell::RefCell;
use std::collections::HashMap;

pub use hydroflow_cli_integration::*;
use serde::de::DeserializeOwned;

use crate::scheduled::graph::Hydroflow;

pub async fn launch<T: DeserializeOwned + Default>(
    flow: impl FnOnce(&HydroCLI<T>) -> Hydroflow<'_>,
) {
    let ports = init_no_ack_start::<T>().await;
    let flow = flow(&ports);

    println!("ack start");

    launch_flow(flow).await;
}

pub async fn launch_flow(mut flow: Hydroflow<'_>) {
    let stop = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        assert!(line.starts_with("stop"));
        stop.0.send(()).unwrap();
    });

    let local_set = tokio::task::LocalSet::new();
    let flow = local_set.run_until(async move {
        flow.run_async().await;
    });

    tokio::select! {
        _ = stop.1 => {},
        _ = flow => {}
    }
}

pub struct HydroCLI<T = Option<()>> {
    ports: RefCell<HashMap<String, ServerOrBound>>,
    pub meta: T,
}

impl<T> HydroCLI<T> {
    pub fn port(&self, name: &str) -> ServerOrBound {
        self.ports
            .try_borrow_mut()
            .unwrap()
            .remove(name)
            .unwrap_or_else(|| panic!("port {} not found", name))
    }
}

pub async fn init_no_ack_start<T: DeserializeOwned + Default>() -> HydroCLI<T> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();

    let bind_config = serde_json::from_str::<InitConfig>(trimmed).unwrap();

    // config telling other services how to connect to me
    let mut bind_results: HashMap<String, ServerPort> = HashMap::new();
    let mut binds = HashMap::new();
    for (name, config) in bind_config.0 {
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
        all_connected.insert(name, ServerOrBound::Server((&defn).into()));
    }

    for (name, defn) in binds {
        all_connected.insert(name, ServerOrBound::Bound(defn));
    }

    HydroCLI {
        ports: RefCell::new(all_connected),
        meta: bind_config
            .1
            .map(|b| serde_json::from_str(&b).unwrap())
            .unwrap_or_default(),
    }
}

pub async fn init<T: DeserializeOwned + Default>() -> HydroCLI<T> {
    let ret = init_no_ack_start::<T>().await;

    println!("ack start");

    ret
}
