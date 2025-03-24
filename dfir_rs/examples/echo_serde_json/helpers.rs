use std::net::SocketAddr;

use dfir_rs::lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::scheduled::graph::Dfir;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::codec::LinesCodecError;

pub fn print_graph(flow: &Dfir, graph: WriteGraphType, write_config: Option<WriteConfig>) {
    let serde_graph = flow
        .meta_graph()
        .expect("No graph found, maybe failed to parse.");
    serde_graph.open_graph(graph, write_config).unwrap();
}

pub fn serialize_json<T>(msg: T) -> String
where
    T: Serialize + for<'a> Deserialize<'a> + Clone,
{
    json!(msg).to_string()
}

pub fn deserialize_json<T>(msg: Result<(String, SocketAddr), LinesCodecError>) -> (T, SocketAddr)
where
    T: Serialize + for<'a> Deserialize<'a> + Clone,
{
    let (m, a) = msg.unwrap();
    (serde_json::from_str(&(m)).unwrap(), a)
}
