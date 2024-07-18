use hydroflow_plus::*;
use stageleft::*;

pub fn simple_cluster<'a, D: Deploy<'a, ClusterId = u32>>(
    flow: &FlowBuilder<'a, D>,
    process_spec: &impl ProcessSpec<'a, D>,
    cluster_spec: &impl ClusterSpec<'a, D>,
) -> (D::Process, D::Cluster) {
    let process = flow.process(process_spec);
    let cluster = flow.cluster(cluster_spec);

    let numbers = flow.source_iter(&process, q!(0..5));
    let ids = flow.source_iter(&process, cluster.ids()).map(q!(|&id| id));

    let cluster_self_id = cluster.self_id();

    ids.cross_product(numbers)
        .map(q!(|(id, n)| (id, (id, n))))
        .send_bincode(&cluster)
        .inspect(q!(move |n| println!(
            "cluster received: {:?} (self cluster id: {})",
            n, cluster_self_id
        )))
        .send_bincode(&process)
        .for_each(q!(|(id, d)| println!("node received: ({}, {:?})", id, d)));

    (process, cluster)
}

use hydroflow_plus::util::cli::HydroCLI;
use hydroflow_plus_cli_integration::{CLIRuntime, HydroflowPlusMeta};

#[stageleft::entry]
pub fn simple_cluster_runtime<'a>(
    flow: FlowBuilder<'a, CLIRuntime>,
    cli: RuntimeData<&'a HydroCLI<HydroflowPlusMeta>>,
) -> impl Quoted<'a, Hydroflow<'a>> {
    let _ = simple_cluster(&flow, &cli, &cli);
    flow.extract()
        .optimize_default()
        .with_dynamic_id(q!(cli.meta.subgraph_id))
}

#[stageleft::runtime]
#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use hydro_deploy::{Deployment, HydroflowCrate};
    use hydroflow_plus_cli_integration::{
        DeployClusterSpec, DeployCrateWrapper, DeployProcessSpec,
    };

    #[tokio::test]
    async fn simple_cluster() {
        let deployment = RefCell::new(Deployment::new());
        let localhost = deployment.borrow_mut().Localhost();

        let builder = hydroflow_plus::FlowBuilder::new();
        let (node, cluster) = super::simple_cluster(
            &builder,
            &DeployProcessSpec::new(|| {
                deployment.borrow_mut().add_service(
                    HydroflowCrate::new(".", localhost.clone())
                        .bin("simple_cluster")
                        .profile("dev"),
                )
            }),
            &DeployClusterSpec::new(|| {
                (0..2)
                    .map(|_| {
                        deployment.borrow_mut().add_service(
                            HydroflowCrate::new(".", localhost.clone())
                                .bin("simple_cluster")
                                .profile("dev"),
                        )
                    })
                    .collect()
            }),
        );

        insta::assert_debug_snapshot!(builder.extract().ir());

        let mut deployment = deployment.into_inner();

        deployment.deploy().await.unwrap();

        let mut node_stdout = node.stdout().await;
        let cluster_stdouts =
            futures::future::join_all(cluster.members.iter().map(|node| node.stdout())).await;

        deployment.start().await.unwrap();

        for (i, mut stdout) in cluster_stdouts.into_iter().enumerate() {
            for j in 0..5 {
                assert_eq!(
                    stdout.recv().await.unwrap(),
                    format!("cluster received: ({}, {}) (self cluster id: {})", i, j, i)
                );
            }
        }

        let mut node_outs = vec![];
        for _i in 0..10 {
            node_outs.push(node_stdout.recv().await.unwrap());
        }
        node_outs.sort();

        for (i, n) in node_outs.into_iter().enumerate() {
            assert_eq!(
                n,
                format!("node received: ({}, ({}, {}))", i / 5, i / 5, i % 5)
            );
        }
    }
}
