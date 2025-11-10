use std::hash::{Hash, Hasher};

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::MemberId;
use hydro_lang::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
use hydro_lang::prelude::*;
use hydro_std::membership::track_membership;
use serde::{Deserialize, Serialize};

pub struct P1 {}
pub struct P2 {}
pub struct P3 {}

pub struct C2 {}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum Command {
    Put(String, String),
    Get(String),
}

fn hash<Tag>(key: &str, node: &MemberId<Tag>) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    key.hash(&mut hasher);
    node.hash(&mut hasher);
    hasher.finish()
}

#[expect(clippy::type_complexity, reason = "example code")]
pub fn distributed_rendezvous_partitioning<'a>(
    external: &External<'a, ()>,
    p1: &Process<'a, P1>,
    c2: &Cluster<'a, C2>,
    p3: &Process<'a, P3>,
) -> (
    ExternalBincodeSink<Command>,
    ExternalBincodeStream<(MemberId<C2>, (String, Option<String>)), NoOrder>,
) {
    let (tx, rx) = p1.source_external_bincode::<_, Command, _, _>(external);

    let req = rx
        .map(q!(|n| {
            match n.clone() {
                Command::Put(k, _) => (k.clone(), n),
                Command::Get(k) => (k, n),
            }
        }))
        .inspect(q!(|v| println!("req: {v:?}")));

    let tick = p1.tick();

    let members = track_membership(p1.source_cluster_members(c2))
        .snapshot(&tick, nondet!(/** */))
        .map(q!(|_| ()))
        .entries()
        .map(q!(|(k, _)| k))
        .inspect(q!(|v| println!("members: {v:?}")))
        .assume_ordering(nondet!(/** */))
        .collect_vec();

    let max_hash_member = req
        .batch(&tick, nondet!(/** */))
        .cross_singleton(members)
        .filter_map(q!(|((key, command), member_ids)| {
            member_ids
                .into_iter()
                .map(|member_id| (member_id.clone(), self::hash(&key, &member_id)))
                .max_by_key(|v| v.1)
                .map(|(node_id, max_hash)| (key.clone(), command.clone(), node_id, max_hash))

            // ).first()(
            //         (key.clone(), command),
            //         self::hash(key.as_str(), &node_id),
            //     )
        }));

    // .inspect(q!(|v| println!("members_crossed_reqs: {v:?}")))
    // .into_keyed()
    // .fold_commutative(
    //     q!(|| None),
    //     q!(|acc, hash| if let Some((acc_data, acc_hash)) = acc {
    //         if *acc_hash > hash {
    //             *acc_data = hash;
    //             *acc_hash = hash;
    //         }
    //     } else {
    //         *acc = Some(hash);
    //     }),
    // );

    let stream = max_hash_member.all_ticks().inspect(q!(|v| println!("stream: {v:?}"))) // ????
        .map(q!(|(_, command, node_id, _)| (node_id, (command)))).into_keyed().demux_bincode(c2);

    let gets = stream
        .clone()
        .filter_map(q!(|v| {
            if let Command::Get(v) = v {
                Some((v, ()))
            } else {
                None
            }
        }))
        .inspect(q!(|v| println!("get: {v:?}")));

    let puts = stream
        .clone()
        .filter_map(q!(|v| {
            if let Command::Put(k, v) = v {
                Some((k, v))
            } else {
                None
            }
        }))
        .inspect(q!(|v| println!("put: {v:?}")))
        .into_keyed();

    let storage = puts
        .assume_ordering(nondet!(/** */))
        .fold(q!(|| None), q!(|acc, v| *acc = Some(v)));

    let new_tick = c2.tick();

    let ret = storage
        .snapshot(&new_tick, nondet!(/** */))
        .entries()
        .join(gets.batch(&new_tick, nondet!(/** */)))
        .map(q!(|(k, (v1, _))| { (k, v1) }))
        .all_ticks();

    let rx = ret.send_bincode(p3).entries();

    let rx = rx.send_bincode_external(external);

    // .send_bincode_external(external);

    // let rx = members_crossed_reqs.send_bincode_external(&tx);

    (tx, rx)
}

// let z = p1
//     .source_cluster_members_docker(c2)
//     .entries()
//     .cross_singleton(x);

// let gets = rx.clone().filter_map(q!(|v| {
//     if let Command::GET(v) = v {
//         Some(v)
//     } else {
//         None
//     }
// }));

// let puts = rx.clone().filter_map(q!(|v| {
//     if let Command::PUT(k, v) = v {
//         Some((k, v))
//     } else {
//         None
//     }
// }));

// let z = rx
//     .assume_ordering(nondet!(/** test */))
//     .map(q!(|n: Command| {
//         println!("processed element: {n:?}");
//     }))
//     .broadcast_bincode_docker(c2, nondet!(/** test */))
//     .map(q!(|n| {
//         println!("processed element: {n}");
//         format!("{n} - c2")
//     }))
//     .send_bincode_docker(p3)
//     .entries()
//     .assume_ordering(nondet!(/** test */))
//     .map(q!(|(from, v)| {
//         println!("processed element: from: {}, v: {v}", from.container_name);
//         (from, format!("{v} - p3"))
//     }))
//     .send_bincode_external(external);

// #[cfg(test)]
// mod tests {
//     use std::sync::Arc;

//     use dfir_rs::tokio_stream::StreamExt;
//     use futures::SinkExt;
//     use hydro_deploy::{AwsNetwork, Deployment};
//     use hydro_lang::deploy::{DockerDeploy, DockerNetwork};
//     // use hydro_lang::{compile::deploy_provider::Deploy, location::NetworkHint};
//     use tokio::sync::RwLock;

//     #[tokio::test]
//     async fn distributed_echo_localhost() {
//         let mut deployment = Deployment::new();

//         let builder = hydro_lang::compile::builder::FlowBuilder::new();
//         let external = builder.external();
//         let p1 = builder.process();
//         let p2 = builder.process();
//         let p3 = builder.process();
//         let (external_sink, external_stream) = super::distributed_echo(&external, &p1, &p2, &p3);

//         let nodes = builder
//             .with_process(&p1, deployment.Localhost())
//             .with_process(&p2, deployment.Localhost())
//             .with_process(&p3, deployment.Localhost())
//             .with_external(&external, deployment.Localhost())
//             .deploy(&mut deployment);

//         deployment.deploy().await.unwrap();

//         let mut external_sink = nodes.connect(external_sink).await;
//         let mut external_stream = nodes.connect(external_stream).await;

//         deployment.start().await.unwrap();

//         external_sink.send(7).await.unwrap();
//         assert_eq!(external_stream.next().await.unwrap(), 13);

//         deployment.stop().await.unwrap();
//     }

//     #[ignore]
//     #[tokio::test(flavor = "multi_thread")]
//     async fn distributed_echo_aws() {
//         let mut deployment: Deployment = Deployment::new();

//         let builder = hydro_lang::compile::builder::FlowBuilder::new();
//         let external = builder.external();
//         let p1 = builder.process();
//         let p2 = builder.process();
//         let p3 = builder.process();
//         let (external_sink, external_stream) = super::distributed_echo(&external, &p1, &p2, &p3);

//         let network = Arc::new(RwLock::new(AwsNetwork::new("us-east-1", None)));

//         let nodes = builder
//             .with_process(
//                 &p1,
//                 deployment.AwsEc2Host()
//                     .region("us-east-1")
//                     .instance_type("t3.micro")
//                     .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
//                     .network(network.clone())
//                     .add(),
//             )
//             .with_process(
//                 &p2,
//                 deployment.AwsEc2Host()
//                     .region("us-east-1")
//                     .instance_type("t3.micro")
//                     .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
//                     .network(network.clone())
//                     .add(),
//             )
//             .with_process(
//                 &p3,
//                 deployment.AwsEc2Host()
//                     .region("us-east-1")
//                     .instance_type("t3.micro")
//                     .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
//                     .network(network.clone())
//                     .add(),
//             )
//             .with_external(&external, deployment.Localhost())
//             .deploy(&mut deployment);

//         deployment.deploy().await.unwrap();

//         let mut external_sink = nodes.connect(external_sink).await;
//         let mut external_stream = nodes.connect(external_stream).await;

//         deployment.start().await.unwrap();

//         external_sink.send(7).await.unwrap();
//         assert_eq!(external_stream.next().await.unwrap(), 13);

//         deployment.stop().await.unwrap();
//     }

//     #[tokio::test]
//     async fn distributed_echo_containerized() {
//         let mut deployment = DockerDeploy::new();

//         let builder = hydro_lang::compile::builder::FlowBuilder::new();
//         let external = builder.external();
//         let p1 = builder.process();
//         let p2 = builder.process();
//         let p3 = builder.process();
//         let (external_sink, external_stream) = super::distributed_echo(&external, &p1, &p2, &p3);

//         let network = DockerNetwork::new("distributed_echo_test".to_string());

//         let nodes = builder
//             .with_process(
//                 &p1,
//                 deployment.add_docker("p1".to_string(), network.clone()),
//             )
//             .with_process(
//                 &p2,
//                 deployment.add_docker("p2".to_string(), network.clone()),
//             )
//             .with_process(
//                 &p3,
//                 deployment.add_docker("p3".to_string(), network.clone()),
//             )
//             .with_external(&external, deployment.add_external("external".to_string()))
//             .deploy(&mut deployment);

//         deployment.provision().await.unwrap();
//         deployment.start().await.unwrap();

//         let mut external_stream = nodes.connect(external_stream).await;

//         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

//         let mut external_sink = nodes.connect(external_sink).await;

//         external_sink.send(7).await.unwrap();
//         assert_eq!(external_stream.next().await.unwrap(), 13);

//         deployment.stop().await.unwrap();
//     }
// }
