// TODO: Reachability example uses within-tick fixpoint cycles that require
// `loop {}` support in inline codegen.

pub fn main() {
    eprintln!("Reachability example is temporarily disabled during inline codegen migration.");
}

// Original code:
// use dfir_rs::dfir_syntax;
//
// pub fn main() {
//     let (edges_send, edges_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();
//     let mut flow = dfir_syntax! {
//         origin = source_iter(vec![0]);
//         stream_of_edges = source_stream(edges_recv);
//         reached_vertices -> map(|v| (v, ())) -> [0]my_join_tee;
//         stream_of_edges -> [1]my_join_tee;
//         my_join_tee = join() -> flat_map(|(src, ((), dst))| [src, dst]) -> tee();
//         origin -> [base]reached_vertices;
//         my_join_tee -> [cycle]reached_vertices;
//         reached_vertices = union();
//         my_join_tee[print] -> unique() -> for_each(|x| println!("Reached: {}", x));
//     };
//     // ...
// }
