use std::net::SocketAddr;

use dfir_rs::lang::graph::{DfirGraph, WriteConfig, WriteGraphType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::codec::LinesCodecError;

pub fn print_graph(
    serde_graph: &DfirGraph,
    graph: WriteGraphType,
    write_config: Option<WriteConfig>,
) {
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
