//! This implements the Maelstrom echo workload.
//!
//! See <https://fly.io/dist-sys/1/>

use hydro_lang::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EchoMessage {
    pub msg_id: usize,
    pub echo: String,
}

pub fn echo_server<'a, C>(
    input: KeyedStream<String, EchoMessage, Cluster<'a, C>>,
) -> KeyedStream<String, serde_json::Value, Cluster<'a, C>> {
    input.map(q!(|msg| {
        serde_json::json!({
            "type": "echo_ok",
            "echo": msg.echo,
            "in_reply_to": msg.msg_id
        })
    }))
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
        output_handle.complete(echo_server(input));

        let mut deployment = MaelstromDeployment::new("echo")
            .maelstrom_path(&PathBuf::from_str(&std::env::var("MAELSTROM_PATH").unwrap()).unwrap())
            .node_count(1)
            .time_limit(10);

        let _ = flow
            .with_cluster(&cluster, MaelstromClusterSpec)
            .deploy(&mut deployment);

        deployment.run().unwrap();
    }
}
