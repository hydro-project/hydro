use dfir_lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::scheduled::graph::Dfir;
use regex::Regex;

use crate::protocol::KvsMessage;

pub fn print_graph(flow: &Dfir, graph: WriteGraphType, write_config: Option<WriteConfig>) {
    let serde_graph = flow
        .meta_graph()
        .expect("No graph found, maybe failed to parse.");
    serde_graph.open_graph(graph, write_config).unwrap();
}

pub fn parse_command(line: String) -> Option<KvsMessage> {
    let re = Regex::new(r"([A-z]+)\s+(.+)").unwrap();
    let caps = re.captures(line.as_str())?;

    let cmd = caps.get(1).unwrap().as_str().to_uppercase();
    let args = caps.get(2).unwrap().as_str();
    match &*cmd {
        "PUT" => {
            let kv = args.split_once(',')?;
            Some(KvsMessage::ClientPut {
                key: kv.0.trim().to_owned(),
                value: kv.1.trim().to_owned(),
            })
        }
        "GET" => Some(KvsMessage::ClientGet {
            key: args.trim().to_owned(),
        }),
        _ => None,
    }
}
