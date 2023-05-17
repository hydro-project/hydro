use clap::{Parser, ValueEnum};
use hydroflow::hydroflow_syntax;

// This example detects size three cliques in a graph. Size three cliques are also known as triangles.
// The equivalent datalog program would be Triangle(x,y,z) := Edge(x,y), Edge(y,z), Edge(z,x)
#[derive(Parser, Debug, Clone, ValueEnum)]
enum GraphType {
    Mermaid,
    Dot,
    Json,
}
#[derive(Parser, Debug)]
struct Opts {
    #[clap(value_enum, long)]
    graph: Option<GraphType>,
}
pub fn main() {
    let opts = Opts::parse();
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (edges_send, edges_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut df = hydroflow_syntax! {
        edges = source_stream(edges_recv) -> tee();

        // set up the two joins
        // edge_pairs((z,x), y) :- edges(x,y), edges(y,z)
        edge_pairs = join() -> map(|(y, (x,z))| ((z,x), y)); //Here we have found all paths from x to z that go through y. Now we need to find edges that connect z back to x.
        // triangle(x,y,z) :- edge_pairs((z,x), y), edges(z, x)
        triangle = join() -> map(|((z,x), (y, ()))| (x, y, z));

        // wire the inputs to the joins
        edges[0] -> map(|(y,z)| (z,y)) -> [0]edge_pairs;
        edges[1] -> [1]edge_pairs;
        edge_pairs -> map(|((z,x), y)| ((z, x), y)) -> [0]triangle;
        edges[2] -> map(|(z,x)| ((z,x), ())) -> [1]triangle;

        // post-process: sort fields of each tuple by node ID
        triangle -> map(|(x, y, z)| {
            let mut v = vec![x, y, z];
            v.sort();
            (v[0], v[1], v[2])
        }) -> for_each(|e| println!("three_clique found: {:?}", e));
    };

    if let Some(graph) = opts.graph {
        let serde_graph = df
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        match graph {
            GraphType::Mermaid => {
                println!("{}", serde_graph.to_mermaid());
            }
            GraphType::Dot => {
                println!("{}", serde_graph.to_dot())
            }
            GraphType::Json => {
                unimplemented!();
                // println!("{}", serde_graph.to_json())
            }
        }
    }

    df.run_available();

    println!("A");

    edges_send.send((5, 10)).unwrap();
    edges_send.send((0, 3)).unwrap();
    edges_send.send((3, 6)).unwrap();
    df.run_available();

    println!("B");

    edges_send.send((6, 5)).unwrap();
    edges_send.send((6, 0)).unwrap();
    edges_send.send((10, 6)).unwrap();
    df.run_available();

    // A
    // B
    // three_clique found: (0, 3, 6)
    // three_clique found: (5, 6, 10)
    // three_clique found: (0, 3, 6)
    // three_clique found: (5, 6, 10)
    // three_clique found: (0, 3, 6)
    // three_clique found: (5, 6, 10)
}
