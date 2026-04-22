use dfir_rs::lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::scheduled::context::{Dfir, TickClosure};

pub fn print_graph(
    flow: &Dfir<impl TickClosure>,
    graph: WriteGraphType,
    write_config: Option<WriteConfig>,
) {
    let serde_graph = flow
        .meta_graph()
        .expect("No graph found, maybe failed to parse.");
    serde_graph.open_graph(graph, write_config).unwrap();
}
