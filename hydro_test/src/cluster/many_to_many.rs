use hydro_lang::*;

pub fn many_to_many<'a>(flow: &FlowBuilder<'a>) -> Cluster<'a, ()> {
    let cluster = flow.cluster();
    cluster
        .source_iter(q!(0..2))
        .broadcast_bincode(&cluster, nondet!(/** test */))
        .entries()
        .for_each(q!(|n| println!("cluster received: {:?}", n)));

    cluster
}

#[cfg(test)]
mod tests {
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::DeployCrateWrapper;

    #[test]
    fn many_to_many_ir() {
        let builder = hydro_lang::FlowBuilder::new();
        let _ = super::many_to_many(&builder);
        let built = builder.finalize();

        hydro_build_utils::assert_debug_snapshot!(built.ir());
    }

    #[tokio::test]
    async fn many_to_many() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let cluster = super::many_to_many(&builder);

        let nodes = builder
            .with_default_optimize()
            .with_cluster(&cluster, (0..2).map(|_| deployment.Localhost()))
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
