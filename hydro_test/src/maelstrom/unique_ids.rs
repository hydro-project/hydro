//! This implements the Maelstrom unique-ids workload.
//!
//! See <https://fly.io/dist-sys/2/>

use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GenerateMessage {
    pub msg_id: usize,
}

pub fn unique_id_server<'a, C: 'a>(
    input: KeyedStream<String, GenerateMessage, Cluster<'a, C>>,
) -> KeyedStream<String, serde_json::Value, Cluster<'a, C>> {
    input
        .entries()
        .assume_ordering(nondet!(/** ids can be nondeterministic */))
        .enumerate()
        .map(q!(move |(idx, (sender, msg))| {
            let self_id = &CLUSTER_SELF_ID;
            (
                sender,
                serde_json::json!({
                    "type": "generate_ok",
                    "id": format!("{}-{}", self_id, idx),
                    "in_reply_to": msg.msg_id
                }),
            )
        }))
        .into_keyed()
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
    async fn test_with_maelstrom() {
        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();

        let (input, output_handle) = maelstrom_bidi_clients(&cluster);
        output_handle.complete(unique_id_server(input));

        let mut deployment = MaelstromDeployment::new("unique-ids")
            .maelstrom_path(PathBuf::from_str(&std::env::var("MAELSTROM_PATH").unwrap()).unwrap())
            .node_count(3)
            .time_limit(30)
            .rate(1000)
            .availability("total")
            .nemesis("partition");

        let _ = flow
            .with_cluster(&cluster, MaelstromClusterSpec)
            .deploy(&mut deployment);

        deployment.run().unwrap();
    }
}
