use std::collections::{HashMap, HashSet};

use adjacency::rga_adjacency;
use clap::{Parser, ValueEnum};
use datalog::rga_datalog;
use datalog_agg::rga_datalog_agg;
use hydroflow::util::collect_ready_async;
use minimal::rga_minimal;
use protocol::{Timestamp, Token};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

mod adjacency;
mod datalog;
mod datalog_agg;
mod minimal;
mod protocol;

#[derive(Parser, Debug, Clone, ValueEnum)]
enum Implementation {
    Datalog,
    Adjacency,
    DatalogAgg,
    Minimal,
}

#[derive(Clone, ValueEnum, Debug)]
enum GraphType {
    Mermaid,
    Dot,
    Json,
}

#[derive(Parser, Debug)]
struct Opts {
    #[clap(value_enum, long = "impl", short)]
    implementation: Option<Implementation>,
    #[clap(value_enum, long, short)]
    graph: Option<GraphType>,
    #[clap(value_enum, long, short)]
    output: String,
}

#[tokio::main]
pub async fn main() {
    // An edge in the input data = a pair of `Token` vertex IDs.
    let (input_send, input_recv) = hydroflow::util::unbounded_channel::<(Token, Timestamp)>();
    let (rga_send, mut rga_recv) = hydroflow::util::unbounded_channel::<(Token, Timestamp)>();
    let (list_send, mut list_recv) = hydroflow::util::unbounded_channel::<(Timestamp, Timestamp)>();
    let opts = Opts::parse();

    let mut df = match opts.implementation {
        Some(Implementation::Datalog) => rga_datalog(input_recv, rga_send, list_send),
        Some(Implementation::Adjacency) => rga_adjacency(input_recv, rga_send, list_send),
        Some(Implementation::Minimal) => rga_minimal(input_recv, rga_send, list_send),
        Some(Implementation::DatalogAgg) => rga_datalog_agg(input_recv, rga_send, list_send),
        None => rga_adjacency(input_recv, rga_send, list_send),
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
            }
        }
    }

    keystroke((1, 0, 'a'), (0, 0), &input_send).await;
    keystroke((2, 0, 'b'), (1, 0), &input_send).await;
    keystroke((3, 0, 'a'), (2, 0), &input_send).await;
    keystroke((4, 0, 't'), (3, 0), &input_send).await;
    keystroke((5, 0, 'e'), (4, 0), &input_send).await;

    keystroke((6, 1, 'o'), (2, 0), &input_send).await;
    keystroke((7, 1, 'r'), (6, 1), &input_send).await;
    keystroke((8, 0, 'C'), (0, 0), &input_send).await;
    keystroke((9, 0, 'o'), (8, 0), &input_send).await;
    keystroke((10, 0, 'l'), (9, 0), &input_send).await;
    keystroke((11, 0, 'l'), (10, 0), &input_send).await;

    df.run_tick();

    let mut output = String::new();
    write_to_dot(&mut rga_recv, &mut list_recv, &mut output).await;
    std::fs::write(opts.output, output).expect("write to output file failed");
}

async fn keystroke(
    (node_ts, node_id, c): (usize, usize, char),
    (parent_ts, parent_id): (usize, usize),
    input_send: &UnboundedSender<(Token, Timestamp)>,
) {
    input_send
        .send((
            Token {
                ts: Timestamp { node_ts, node_id },
                value: c,
            },
            Timestamp {
                node_ts: parent_ts,
                node_id: parent_id,
            },
        ))
        .unwrap();
}

async fn write_to_dot(
    rga_recv: &mut UnboundedReceiverStream<(Token, Timestamp)>,
    list_recv: &mut UnboundedReceiverStream<(Timestamp, Timestamp)>,
    w: &mut impl std::fmt::Write,
) {
    let tree_edges: HashSet<_> = collect_ready_async(rga_recv).await;
    let list_edges: HashMap<_, _> = collect_ready_async(list_recv).await;
    let node_names = tree_edges
        .iter()
        .map(|(c, _)| (c.ts, c.value))
        .collect::<HashMap<_, _>>();

    // print RGA tree in dot format
    writeln!(w, "digraph G {{\nrankdir = TB").unwrap();
    writeln!(
        w,
        "ts0_0 [label=<root<SUB><FONT COLOR=\"red\" POINT-SIZE=\"8\">ts0_0</FONT></SUB>>]"
    )
    .unwrap();
    for (c, p) in tree_edges {
        writeln!(w, "{} -> {} [color=gray, constraint=true]", p, c.ts).unwrap();
        writeln!(w, "{}", format_dot_node(c)).unwrap();
    }
    for (first, second) in list_edges.iter() {
        writeln!(
            w,
            "{} -> {} [style=dashed, color=blue, constraint=false]",
            first, second
        )
        .unwrap();
    }

    let mut x = Timestamp {
        node_ts: 0,
        node_id: 0,
    };
    let mut result = String::new();
    while let Some(y) = list_edges.get(&x) {
        result.push(*(node_names.get(y).unwrap()));
        x = *y;
    }
    if result.is_empty() {
        result = "Unknown".to_string()
    };
    writeln!(w, "label=<<FONT COLOR=\"blue\">{}</FONT>>", result).unwrap();
    writeln!(w, "}}").unwrap();
}

fn format_dot_node(n: Token) -> String {
    format!(
        "{} [label=<{}<SUB><FONT COLOR=\"red\" POINT-SIZE=\"8\">{}</FONT></SUB>>]",
        n.ts, n.value, n.ts
    )
}
