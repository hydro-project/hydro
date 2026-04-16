use dfir_rs::lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::lang::graph::DfirGraph;

pub fn print_graph(serde_graph: &DfirGraph, graph: WriteGraphType, write_config: Option<WriteConfig>) {
    serde_graph.open_graph(graph, write_config).unwrap();
}
