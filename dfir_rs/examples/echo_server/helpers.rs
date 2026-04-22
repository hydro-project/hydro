use dfir_rs::lang::graph::{WriteConfig, WriteGraphType};


pub fn print_graph(serde_graph: &dfir_lang::graph::DfirGraph, graph: WriteGraphType, write_config: Option<WriteConfig>) {
    serde_graph.open_graph(graph, write_config).unwrap();
}
