use dfir_rs::dfir_syntax;

pub fn main() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (edges_send, edges_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();

    let mut flow = dfir_syntax! {
        // inputs: the origin vertex (vertex 0) and stream of input edges
        origin = source_iter(vec![0]);
        stream_of_edges = source_stream(edges_recv);

        // the join
        reached_vertices -> map(|v| (v, ())) -> [0]my_join_tee;
        stream_of_edges -> [1]my_join_tee;
        my_join_tee = join() -> flat_map(|(src, ((), dst))| [src, dst]) -> tee();

        // the cycle: my_join_tee gets data from reached_vertices
        // and provides data back to reached_vertices!
        origin -> [base]reached_vertices;
        my_join_tee -> [cycle]reached_vertices;
        reached_vertices = union();

        // the output
        my_join_tee[print] -> unique() -> for_each(|x| println!("Reached: {}", x));
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
    edges_send.send((4, 0)).unwrap();
    flow.run_available_sync();
}
