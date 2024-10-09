use hydroflow_plus::*;
use stageleft::*;

pub fn many_to_many<'a>(flow: &FlowBuilder<'a>) -> Cluster<'a, ()> {
    let cluster = flow.cluster();
    cluster
        .source_iter(q!(0..2))
        .broadcast_bincode(&cluster)
        .for_each(q!(|n| println!("cluster received: {:?}", n)));

    cluster
}

#[cfg(test)]
mod tests {
    use hydro_deploy::Deployment;
    use hydroflow_plus::deploy::{DeployCrateWrapper, TrybuildHost};

    #[tokio::test]
    async fn many_to_many() {
        let mut deployment = Deployment::new();

        let builder = hydroflow_plus::FlowBuilder::new();
        let cluster = super::many_to_many(&builder);
        let built = builder.with_default_optimize();

        insta::assert_debug_snapshot!(built.ir());

        let nodes = built
            .with_cluster(
                &cluster,
                (0..2)
                    .map(|_| TrybuildHost::new(deployment.Localhost()))
                    .collect::<Vec<_>>(),
            )
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let cluster_stdouts = futures::future::join_all(
            nodes
                .get_cluster(&cluster)
                .members()
                .iter()
                .map(|node| node.stdout()),
        )
        .await;

        deployment.start().await.unwrap();

        for mut node_stdout in cluster_stdouts {
            let mut node_outs = vec![];
            for _i in 0..4 {
                node_outs.push(node_stdout.recv().await.unwrap());
            }
            node_outs.sort();

            let mut node_outs = node_outs.into_iter();

            for sender in 0..2 {
                for value in 0..2 {
                    assert_eq!(
                        node_outs.next().unwrap(),
                        format!("cluster received: (ClusterId::<()>({}), {})", sender, value)
                    );
                }
            }
        }
    }
}
