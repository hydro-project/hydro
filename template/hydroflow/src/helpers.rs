use hydroflow::lang::graph::{WriteConfig, WriteGraphType};
use hydroflow::scheduled::graph::Hydroflow;

pub fn print_graph(flow: &Hydroflow, graph: WriteGraphType, write_config: Option<WriteConfig>) {
    let serde_graph = flow
        .meta_graph()
        .expect("No graph found, maybe failed to parse.");
    serde_graph.open_graph(graph, write_config).unwrap();
}
