use std::collections::{HashMap, HashSet};

use adjacency::rga_adjacency;
use clap::{Parser, ValueEnum};
use datalog::rga_datalog;
use datalog_agg::rga_datalog_agg;
use dfir_lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::util::collect_ready_async;
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

#[derive(Parser, Debug)]
struct Opts {
    #[clap(value_enum, long = "impl", short)]
    implementation: Option<Implementation>,
    #[clap(long)]
    graph: Option<WriteGraphType>,
    #[clap(flatten)]
    write_config: Option<WriteConfig>,
}

#[dfir_rs::main]
pub async fn main() {
    {
        // Set up tracing logger.
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }

    // An edge in the input data = a pair of `Token` vertex IDs.
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<(Token, Timestamp)>();
    let (rga_send, mut rga_recv) = dfir_rs::util::unbounded_channel::<(Token, Timestamp)>();
    let (list_send, mut list_recv) = dfir_rs::util::unbounded_channel::<(Timestamp, Timestamp)>();
    let opts = Opts::parse();

    let mut hf = match opts.implementation {
        Some(Implementation::Datalog) => rga_datalog(input_recv, rga_send, list_send),
        Some(Implementation::Adjacency) => rga_adjacency(input_recv, rga_send, list_send),
        Some(Implementation::Minimal) => rga_minimal(input_recv, rga_send, list_send),
        Some(Implementation::DatalogAgg) => rga_datalog_agg(input_recv, rga_send, list_send),
        None => rga_adjacency(input_recv, rga_send, list_send),
    };

    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
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

    hf.run_tick();

    let mut output = String::new();
    write_to_dot(&mut rga_recv, &mut list_recv, &mut output).await;
    println!("{}", output);
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

#[test]
fn test() {
    use example_test::run_current_example;

    {
        let mut example = run_current_example!("--impl adjacency");
        for line in EXPECTED_OUTPUT.split('\n') {
            example.read_string(line);
        }
    }

    {
        let mut example = run_current_example!("--impl datalog");
        for line in EXPECTED_OUTPUT.split('\n') {
            example.read_string(line);
        }
    }

    {
        let mut example = run_current_example!("--impl minimal");
        for line in EXPECTED_OUTPUT_MINIMAL.split('\n') {
            example.read_string(line);
        }
    }

    // TODO: This implementation appears to be broken.
    // {
    //     let mut example = run_current_example!("--impl datalog-agg");
    //     for line in EXPECTED_OUTPUT_MINIMAL.split('\n') {
    //         example.wait_for_output(&escape_regex(line));
    //     }
    // }
}

// Output can be re-ordered, so this will be tested line by line
#[cfg(test)]
const EXPECTED_OUTPUT: &str = r#"digraph G {
rankdir = TB
ts0_0 [label=<root<SUB><FONT COLOR="red" POINT-SIZE="8">ts0_0</FONT></SUB>>]
ts4_0 -> ts5_0 [color=gray, constraint=true]
ts5_0 [label=<e<SUB><FONT COLOR="red" POINT-SIZE="8">ts5_0</FONT></SUB>>]
ts9_0 -> ts10_0 [color=gray, constraint=true]
ts10_0 [label=<l<SUB><FONT COLOR="red" POINT-SIZE="8">ts10_0</FONT></SUB>>]
ts6_1 -> ts7_1 [color=gray, constraint=true]
ts7_1 [label=<r<SUB><FONT COLOR="red" POINT-SIZE="8">ts7_1</FONT></SUB>>]
ts0_0 -> ts1_0 [color=gray, constraint=true]
ts1_0 [label=<a<SUB><FONT COLOR="red" POINT-SIZE="8">ts1_0</FONT></SUB>>]
ts0_0 -> ts8_0 [color=gray, constraint=true]
ts8_0 [label=<C<SUB><FONT COLOR="red" POINT-SIZE="8">ts8_0</FONT></SUB>>]
ts1_0 -> ts2_0 [color=gray, constraint=true]
ts2_0 [label=<b<SUB><FONT COLOR="red" POINT-SIZE="8">ts2_0</FONT></SUB>>]
ts8_0 -> ts9_0 [color=gray, constraint=true]
ts9_0 [label=<o<SUB><FONT COLOR="red" POINT-SIZE="8">ts9_0</FONT></SUB>>]
ts2_0 -> ts3_0 [color=gray, constraint=true]
ts3_0 [label=<a<SUB><FONT COLOR="red" POINT-SIZE="8">ts3_0</FONT></SUB>>]
ts10_0 -> ts11_0 [color=gray, constraint=true]
ts11_0 [label=<l<SUB><FONT COLOR="red" POINT-SIZE="8">ts11_0</FONT></SUB>>]
ts2_0 -> ts6_1 [color=gray, constraint=true]
ts6_1 [label=<o<SUB><FONT COLOR="red" POINT-SIZE="8">ts6_1</FONT></SUB>>]
ts3_0 -> ts4_0 [color=gray, constraint=true]
ts4_0 [label=<t<SUB><FONT COLOR="red" POINT-SIZE="8">ts4_0</FONT></SUB>>]
ts0_0 -> ts8_0 [style=dashed, color=blue, constraint=false]
ts9_0 -> ts10_0 [style=dashed, color=blue, constraint=false]
ts6_1 -> ts7_1 [style=dashed, color=blue, constraint=false]
ts11_0 -> ts1_0 [style=dashed, color=blue, constraint=false]
ts7_1 -> ts3_0 [style=dashed, color=blue, constraint=false]
ts2_0 -> ts6_1 [style=dashed, color=blue, constraint=false]
ts4_0 -> ts5_0 [style=dashed, color=blue, constraint=false]
ts10_0 -> ts11_0 [style=dashed, color=blue, constraint=false]
ts3_0 -> ts4_0 [style=dashed, color=blue, constraint=false]
ts8_0 -> ts9_0 [style=dashed, color=blue, constraint=false]
ts1_0 -> ts2_0 [style=dashed, color=blue, constraint=false]
label=<<FONT COLOR="blue">Collaborate</FONT>>
}"#;

// Output can be re-ordered, so this will be tested line by line
#[cfg(test)]
const EXPECTED_OUTPUT_MINIMAL: &str = r#"digraph G {
rankdir = TB
ts0_0 [label=<root<SUB><FONT COLOR="red" POINT-SIZE="8">ts0_0</FONT></SUB>>]
ts3_0 -> ts4_0 [color=gray, constraint=true]
ts4_0 [label=<t<SUB><FONT COLOR="red" POINT-SIZE="8">ts4_0</FONT></SUB>>]
ts0_0 -> ts8_0 [color=gray, constraint=true]
ts8_0 [label=<C<SUB><FONT COLOR="red" POINT-SIZE="8">ts8_0</FONT></SUB>>]
ts2_0 -> ts3_0 [color=gray, constraint=true]
ts3_0 [label=<a<SUB><FONT COLOR="red" POINT-SIZE="8">ts3_0</FONT></SUB>>]
ts6_1 -> ts7_1 [color=gray, constraint=true]
ts7_1 [label=<r<SUB><FONT COLOR="red" POINT-SIZE="8">ts7_1</FONT></SUB>>]
ts0_0 -> ts1_0 [color=gray, constraint=true]
ts1_0 [label=<a<SUB><FONT COLOR="red" POINT-SIZE="8">ts1_0</FONT></SUB>>]
ts10_0 -> ts11_0 [color=gray, constraint=true]
ts11_0 [label=<l<SUB><FONT COLOR="red" POINT-SIZE="8">ts11_0</FONT></SUB>>]
ts8_0 -> ts9_0 [color=gray, constraint=true]
ts9_0 [label=<o<SUB><FONT COLOR="red" POINT-SIZE="8">ts9_0</FONT></SUB>>]
ts2_0 -> ts6_1 [color=gray, constraint=true]
ts6_1 [label=<o<SUB><FONT COLOR="red" POINT-SIZE="8">ts6_1</FONT></SUB>>]
ts9_0 -> ts10_0 [color=gray, constraint=true]
ts10_0 [label=<l<SUB><FONT COLOR="red" POINT-SIZE="8">ts10_0</FONT></SUB>>]
ts1_0 -> ts2_0 [color=gray, constraint=true]
ts2_0 [label=<b<SUB><FONT COLOR="red" POINT-SIZE="8">ts2_0</FONT></SUB>>]
ts4_0 -> ts5_0 [color=gray, constraint=true]
ts5_0 [label=<e<SUB><FONT COLOR="red" POINT-SIZE="8">ts5_0</FONT></SUB>>]
label=<<FONT COLOR="blue">Unknown</FONT>>
}"#;
