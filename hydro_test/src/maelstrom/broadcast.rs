//! This implements the Maelstrom broadcast workload.
//!
//! See <https://fly.io/dist-sys/3a/> and <https://fly.io/dist-sys/3b/>

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Broadcast {
    pub msg_id: usize,
    pub message: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Read {
    pub msg_id: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Topology {
    pub msg_id: usize,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Request {
    #[serde(alias = "broadcast")]
    Broadcast(Broadcast),
    #[serde(alias = "read")]
    Read(Read),
    #[serde(alias = "topology")]
    Topology(Topology),
}

fn broadcast_core<'a, C: 'a>(
    cluster: &Cluster<'a, C>,
    writes: Stream<u32, Cluster<'a, C>, Unbounded, NoOrder>,
) -> Stream<u32, Cluster<'a, C>, Unbounded, NoOrder> {
    writes
        .broadcast(cluster, TCP.fail_stop().bincode(), nondet!(/** TODO */))
        .values()
}

pub fn broadcast_server<'a, C: 'a>(
    cluster: &Cluster<'a, C>,
    input: KeyedStream<String, Request, Cluster<'a, C>>,
) -> KeyedStream<String, serde_json::Value, Cluster<'a, C>, Unbounded, NoOrder> {
    let broadcast_requests = input.clone().filter_map(q!(|body| {
        if let Request::Broadcast(b) = body {
            Some(b)
        } else {
            None
        }
    }));

    let broadcast_response = broadcast_requests.clone().map(q!(|req| {
        serde_json::json!({
            "type": "broadcast_ok",
            "in_reply_to": req.msg_id
        })
    }));

    let written_data = broadcast_requests.values().map(q!(|t| t.message));
    let current_state = broadcast_core(cluster, written_data);

    let read_requests = input.clone().filter_map(q!(|body| {
        if let Request::Read(r) = body {
            Some(r)
        } else {
            None
        }
    }));

    let read_response = sliced! {
        let req = use(read_requests, nondet!(/** batching of requests does not matter */));
        let data = use(
            current_state
                .assume_ordering(nondet!(/** client ignores order */))
                .fold(q!(|| vec![]), q!(|v, d| v.push(d))),
            nondet!(/** we only guarantee eventual consistency */)
        );

        req.cross_singleton(data).map(q!(|(req, data)| {
            serde_json::json!({
                "type": "read_ok",
                "messages": data,
                "in_reply_to": req.msg_id
            })
        }))
    };

    let topology_requests = input.filter_map(q!(|body| {
        if let Request::Topology(t) = body {
            Some(t)
        } else {
            None
        }
    }));

    let topology_response = topology_requests.map(q!(|req| {
        serde_json::json!({
            "type": "topology_ok",
            "in_reply_to": req.msg_id
        })
    }));

    broadcast_response
        .interleave(read_response)
        .interleave(topology_response)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::str::FromStr;

    use hydro_lang::deploy::maelstrom::deploy_maelstrom::{
        MaelstromClusterSpec, MaelstromDeployment,
    };
    use hydro_lang::deploy::maelstrom::maelstrom_bidi_clients;

    use super::*;

    #[tokio::test]
    #[cfg_attr(not(maelstrom_available), ignore)]
    async fn broadcast_3a_maelstrom() {
        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();

        let (input, output_handle) = maelstrom_bidi_clients(&cluster);
        output_handle
            .complete(broadcast_server(&cluster, input).assume_ordering(nondet!(/** test */)));

        let mut deployment = MaelstromDeployment::new("broadcast")
            .maelstrom_path(PathBuf::from_str(&std::env::var("MAELSTROM_PATH").unwrap()).unwrap())
            .node_count(1)
            .time_limit(20)
            .rate(10);

        let _ = flow
            .with_cluster(&cluster, MaelstromClusterSpec)
            .deploy(&mut deployment);

        deployment.run().unwrap();
    }

    #[tokio::test]
    #[cfg_attr(not(maelstrom_available), ignore)]
    async fn broadcast_3b_maelstrom() {
        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();

        let (input, output_handle) = maelstrom_bidi_clients(&cluster);
        output_handle
            .complete(broadcast_server(&cluster, input).assume_ordering(nondet!(/** test */)));

        let mut deployment = MaelstromDeployment::new("broadcast")
            .maelstrom_path(PathBuf::from_str(&std::env::var("MAELSTROM_PATH").unwrap()).unwrap())
            .node_count(5)
            .time_limit(20)
            .rate(10);

        let _ = flow
            .with_cluster(&cluster, MaelstromClusterSpec)
            .deploy(&mut deployment);

        deployment.run().unwrap();
    }
}
