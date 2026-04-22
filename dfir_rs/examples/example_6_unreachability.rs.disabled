use dfir_rs::dfir_syntax;

pub fn main() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pairs_send, pairs_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();

    let mut flow = dfir_syntax! {
        // inputs: the origin vertex (vertex 0) and stream of input edges
        origin = source_iter(vec![0]);
        stream_of_edges = source_stream(pairs_recv) -> tee();

        // the join for reachable vertices
        reached_vertices[0] -> map(|v| (v, ())) -> [0]my_join;
        stream_of_edges[1] -> [1]my_join;
        my_join = join() -> flat_map(|(src, ((), dst))| [src, dst]);

        // the cycle: my_join gets data from reached_vertices
        // and provides data back to reached_vertices!
        origin -> [base]reached_vertices;
        my_join -> [cycle]reached_vertices;
        reached_vertices = union()->tee();

        // the difference: all_vertices - reached_vertices
        all_vertices = stream_of_edges[0]
          -> flat_map(|(src, dst)| [src, dst]) -> tee();
        all_vertices[0] -> [pos]unreached_vertices;
        reached_vertices[1] -> [neg]unreached_vertices;
        unreached_vertices = difference();

        // the output
        all_vertices[1] -> unique() -> for_each(|v| println!("Received vertex: {}", v));
        unreached_vertices -> for_each(|v| println!("unreached_vertices vertex: {}", v));
    };

    println!(
        "{}",
        flow.meta_graph()
            .expect("No graph found, maybe failed to parse.")
            .to_mermaid(&Default::default())
    );

    pairs_send.send((5, 10)).unwrap();
    pairs_send.send((0, 3)).unwrap();
    pairs_send.send((3, 6)).unwrap();
    pairs_send.send((6, 5)).unwrap();
    pairs_send.send((11, 12)).unwrap();
    flow.run_available_sync();
}
