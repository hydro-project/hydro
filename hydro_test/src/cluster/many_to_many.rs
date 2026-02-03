use hydro_lang::prelude::*;

pub fn many_to_many<'a>(flow: &mut FlowBuilder<'a>) -> Cluster<'a, ()> {
    let cluster = flow.cluster();
    cluster
        .source_iter(q!(0..2))
        .broadcast(
            &cluster,
            TCP.bincode().name("m2m_broadcast"),
            nondet!(/** test */),
        )
        .entries()
        .assume_ordering(nondet!(/** intentionally unordered logs */))
        .for_each(q!(|n| println!("cluster received: {:?}", n)));

    cluster
}

#[cfg(test)]
mod tests {
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::DeployCrateWrapper;

    use crate::test_util::skip_tracing_logs;

    #[test]
    fn many_to_many_ir() {
        let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
        let _ = super::many_to_many(&mut builder);
        let built = builder.finalize();

        hydro_build_utils::assert_debug_snapshot!(built.ir());
    }

    #[tokio::test]
    async fn many_to_many() {
        let mut deployment = Deployment::new();

        let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
        let cluster = super::many_to_many(&mut builder);

        let nodes = builder
            .with_default_optimize()
            .with_cluster(&cluster, (0..2).map(|_| deployment.Localhost()))
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let cluster_stdouts = nodes
            .get_cluster(&cluster)
            .members()
            .iter()
            .map(|node| node.stdout())
            .collect::<Vec<_>>();

        deployment.start().await.unwrap();

        for mut node_stdout in cluster_stdouts {
            let mut node_outs = vec![];
            for _i in 0..4 {
                let actual_message = skip_tracing_logs(&mut node_stdout).await;
                node_outs.push(actual_message);
            }
            node_outs.sort();

            let mut node_outs = node_outs.into_iter();

            for sender in 0..2 {
                for value in 0..2 {
                    assert_eq!(
                        node_outs.next().unwrap(),
                        format!("cluster received: (MemberId::<()>({}), {})", sender, value)
                    );
                }
            }
        }
    }
}
