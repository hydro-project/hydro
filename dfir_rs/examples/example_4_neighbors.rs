use dfir_rs::dfir_syntax;

pub fn main() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (edges_send, edges_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();

    let mut flow = dfir_syntax! {
        // inputs: the origin vertex (vertex 0) and stream of input edges
        origin = source_iter(vec![0]);
        stream_of_edges = source_stream(edges_recv);

        // the join
        origin -> map(|v| (v, ())) -> [0]my_join;
        stream_of_edges -> [1]my_join;
        my_join = join() -> flat_map(|(src, (_, dst))| [src, dst]);

        // the output
        my_join -> unique() -> for_each(|n| println!("Reached: {}", n));
    };

    println!(
        "{}",
        flow.meta_graph()
            .expect("No graph found, maybe failed to parse.")
            .to_mermaid(&Default::default())
    );
    edges_send.send((0, 1)).unwrap();
    edges_send.send((2, 4)).unwrap();
    edges_send.send((3, 4)).unwrap();
    edges_send.send((1, 2)).unwrap();
    edges_send.send((0, 3)).unwrap();
    edges_send.send((0, 3)).unwrap();
    flow.run_available_sync();
}
