use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::location::{MemberId, MembershipEvent};
use hydro_lang::prelude::*;
use hydro_std::compartmentalize::{DecoupleClusterStream, DecoupleProcessStream, PartitionStream};
use stageleft::IntoQuotedMut;

pub fn partition<'a, F: Fn((MemberId<()>, String)) -> (MemberId<()>, String) + 'a>(
    cluster1: Cluster<'a, ()>,
    cluster2: Cluster<'a, ()>,
    dist_policy: impl IntoQuotedMut<'a, F, Cluster<'a, ()>>,
) -> (Cluster<'a, ()>, Cluster<'a, ()>) {
    cluster1
        .source_iter(q!(vec!(CLUSTER_SELF_ID)))
        .map(q!(move |id| (
            MemberId::<()>::from_raw(id.raw_id),
            format!("Hello from {}", id.raw_id)
        )))
        .send_partitioned(&cluster2, dist_policy)
        .for_each(q!(move |message| println!(
            "My self id is {}, my message is {:?}",
            CLUSTER_SELF_ID.raw_id, message
        )));
    (cluster1, cluster2)
}

pub fn decouple_cluster<'a>(flow: &FlowBuilder<'a>) -> (Cluster<'a, ()>, Cluster<'a, ()>) {
    let cluster1 = flow.cluster();
    let cluster2 = flow.cluster();
    cluster1
        .source_iter(q!(vec!(CLUSTER_SELF_ID)))
        // .for_each(q!(|message| println!("hey, {}", message)))
        .inspect(q!(|message| println!("Cluster1 node sending message: {}", message)))
        .decouple_cluster(&cluster2)
        .for_each(q!(move |message| println!(
            "My self id is {}, my message is {}",
            CLUSTER_SELF_ID, message
        )));
    (cluster1, cluster2)
}

pub fn decouple_process<'a>(flow: &FlowBuilder<'a>) -> (Process<'a, ()>, Process<'a, ()>) {
    let process1 = flow.process();
    let process2 = flow.process();
    process1
        .source_iter(q!(0..3))
        .decouple_process(&process2)
        .for_each(q!(|message| println!("I received message is {}", message)));
    (process1, process2)
}

pub fn simple_cluster<'a>(flow: &FlowBuilder<'a>) -> (Process<'a, ()>, Cluster<'a, ()>) {
    let process = flow.process();
    let cluster = flow.cluster();

    let numbers = process.source_iter(q!(0..5));
    let ids = process
        .source_cluster_members(&cluster)
        .entries()
        .filter_map(q!(|(i, e)| match e {
            MembershipEvent::Joined => Some(i),
            MembershipEvent::Left => None,
        }));

    ids.cross_product(numbers)
        .map(q!(|(id, n)| (id, (id, n))))
        .demux_bincode(&cluster)
        .inspect(q!(move |n| println!(
            "cluster received: {:?} (self cluster id: {})",
            n, CLUSTER_SELF_ID
        )))
        .send_bincode(&process)
        .entries()
        .for_each(q!(|(id, d)| println!("node received: ({}, {:?})", id, d)));

    (process, cluster)
}

#[cfg(test)]
mod tests {
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::DeployCrateWrapper;
    use hydro_lang::location::MemberId;
    use stageleft::q;

    #[test]
    fn simple_cluster_ir() {
        let builder = hydro_lang::compile::builder::FlowBuilder::new();
        let _ = super::simple_cluster(&builder);
        let built = builder.finalize();

        hydro_build_utils::assert_debug_snapshot!(built.ir());
    }

    #[tokio::test]
    async fn simple_cluster() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::compile::builder::FlowBuilder::new();
        let (node, cluster) = super::simple_cluster(&builder);

        let nodes = builder
            .with_default_optimize()
            .with_process(&node, deployment.Localhost())
            .with_cluster(&cluster, (0..2).map(|_| deployment.Localhost()))
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut node_stdout = nodes.get_process(&node).stdout().await;
        let cluster_stdouts = futures::future::join_all(
            nodes
                .get_cluster(&cluster)
                .members()
                .iter()
                .map(|node| node.stdout()),
        )
        .await;

        deployment.start().await.unwrap();

        for (i, mut stdout) in cluster_stdouts.into_iter().enumerate() {
            for j in 0..5 {
                assert_eq!(
                    stdout.recv().await.unwrap(),
                    format!(
                        "cluster received: (MemberId::<()>({}), {}) (self cluster id: MemberId::<()>({}))",
                        i, j, i
                    )
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
                format!(
                    "node received: (MemberId::<()>({}), (MemberId::<()>({}), {}))",
                    i / 5,
                    i / 5,
                    i % 5
                )
            );
        }
    }

    #[tokio::test]
    async fn decouple_process() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::compile::builder::FlowBuilder::new();
        let (process1, process2) = super::decouple_process(&builder);
        let built = builder.with_default_optimize();

        let nodes = built
            .with_process(&process1, deployment.Localhost())
            .with_process(&process2, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();
        let mut process2_stdout = nodes.get_process(&process2).stdout().await;
        deployment.start().await.unwrap();
        for i in 0..3 {
            let expected_message = format!("I received message is {}", i);
            assert_eq!(process2_stdout.recv().await.unwrap(), expected_message);
        }
    }

    #[tokio::test]
    async fn decouple_cluster() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::compile::builder::FlowBuilder::new();
        let (cluster1, cluster2) = super::decouple_cluster(&builder);
        let built = builder.with_default_optimize();

        let nodes = built
            .with_cluster(&cluster1, (0..3).map(|_| deployment.Localhost()))
            .with_cluster(&cluster2, (0..3).map(|_| deployment.Localhost()))
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let cluster2_stdouts = futures::future::join_all(
            nodes
                .get_cluster(&cluster2)
                .members()
                .iter()
                .map(|node| node.stdout()),
        )
        .await;

        deployment.start().await.unwrap();

        for (i, mut stdout) in cluster2_stdouts.into_iter().enumerate() {
            for _j in 0..1 {
                let expected_message = format!(
                    "My self id is MemberId::<()>({}), my message is MemberId::<()>({})",
                    i, i
                );
                assert_eq!(stdout.recv().await.unwrap(), expected_message);
            }
        }
    }

    #[tokio::test]
    async fn partition() {
        let mut deployment = Deployment::new();

        let num_nodes = 3;
        let num_partitions = 2;
        let builder = hydro_lang::compile::builder::FlowBuilder::new();
        let (cluster1, cluster2) = super::partition(
            builder.cluster::<()>(),
            builder.cluster::<()>(),
            q!(move |(id, msg)| (
                MemberId::<()>::from_raw(id.raw_id * num_partitions as u32),
                msg
            )),
        );
        let built = builder.with_default_optimize();

        let nodes = built
            .with_cluster(&cluster1, (0..num_nodes).map(|_| deployment.Localhost()))
            .with_cluster(
                &cluster2,
                (0..num_nodes * num_partitions).map(|_| deployment.Localhost()),
            )
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let cluster2_stdouts = futures::future::join_all(
            nodes
                .get_cluster(&cluster2)
                .members()
                .iter()
                .map(|node| node.stdout()),
        )
        .await;

        deployment.start().await.unwrap();

        for (cluster2_id, mut stdout) in cluster2_stdouts.into_iter().enumerate() {
            if cluster2_id % num_partitions == 0 {
                let expected_message = format!(
                    r#"My self id is {}, my message is "Hello from {}""#,
                    cluster2_id,
                    cluster2_id / num_partitions
                );
                assert_eq!(stdout.recv().await.unwrap(), expected_message);
            }
        }
    }
}
