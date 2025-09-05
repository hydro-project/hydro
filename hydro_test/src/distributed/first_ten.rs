use hydro_lang::location::external_process::ExternalBincodeSink;
use hydro_lang::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SendOverNetwork {
    pub n: u32,
}

pub struct P1 {}
pub struct P2 {}

pub fn first_ten_distributed<'a>(
    external: &External<'a, ()>,
    process: &Process<'a, P1>,
    second_process: &Process<'a, P2>,
) -> ExternalBincodeSink<String> {
    let (numbers_external_port, numbers_external) = process.source_external_bincode(external);
    numbers_external.for_each(q!(|n| println!("hi: {:?}", n)));

    let numbers = process.source_iter(q!(0..10));
    numbers
        .map(q!(|n| SendOverNetwork { n }))
        .send_bincode(second_process)
        .for_each(q!(|n| println!("{}", n.n)));

    numbers_external_port
}

#[cfg(test)]
mod tests {
    use futures::SinkExt;
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::DeployCrateWrapper;

    #[test]
    fn first_ten_distributed_ir() {
        let builder = hydro_lang::builder::FlowBuilder::new();
        let external = builder.external();
        let p1 = builder.process();
        let p2 = builder.process();
        super::first_ten_distributed(&external, &p1, &p2);

        hydro_build_utils::assert_debug_snapshot!(builder.finalize().ir());
    }

    #[tokio::test]
    async fn first_ten_distributed() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::builder::FlowBuilder::new();
        let external = builder.external();
        let p1 = builder.process();
        let p2 = builder.process();
        let external_port = super::first_ten_distributed(&external, &p1, &p2);

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_process(&p2, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_port = nodes.connect_sink_bincode(external_port).await;

        let mut first_node_stdout = nodes.get_process(&p1).stdout().await;
        let mut second_node_stdout = nodes.get_process(&p2).stdout().await;

        deployment.start().await.unwrap();

        external_port
            .send("this is some string".to_string())
            .await
            .unwrap();
        assert_eq!(
            first_node_stdout.recv().await.unwrap(),
            "hi: \"this is some string\""
        );

        for i in 0..10 {
            assert_eq!(second_node_stdout.recv().await.unwrap(), i.to_string());
        }
    }
}
