//! hydro_lang/deploy integration tests

use futures::{SinkExt, StreamExt};
use hydro_deploy::{AwsNetwork, Deployment};

#[cfg(stageleft_runtime)]
use crate::deploy::{DockerDeploy, DockerNetwork};
use crate::live_collections::stream::NoOrder;
use crate::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
use crate::location::{MemberId, MembershipEvent};
use crate::nondet::nondet;
use crate::prelude::*;
use crate::telemetry;

struct P1 {}
struct C2 {}
struct C3 {}
struct P4 {}
struct P5 {}

#[expect(clippy::type_complexity, reason = "test code")]
fn distributed_echo<'a>(
    external: &External<'a, ()>,
    p1: &Process<'a, P1>,
    c2: &Cluster<'a, C2>,
    c3: &Cluster<'a, C3>,
    p4: &Process<'a, P4>,
    p5: &Process<'a, P5>,
) -> (
    ExternalBincodeSink<u32>,
    ExternalBincodeStream<u32, NoOrder>,
    ExternalBincodeStream<(MemberId<C2>, MembershipEvent), NoOrder>,
    ExternalBincodeStream<(MemberId<C3>, MembershipEvent), NoOrder>,
) {
    let (tx, rx) = p1.source_external_bincode(external);

    let rx = rx
        .map(q!(|n| n + 1))
        .round_robin_bincode(c2, nondet!(/** test */))
        .map(q!(|n| n + 1))
        .round_robin_bincode(c3, nondet!(/** test */))
        .values()
        .map(q!(|n| n + 1))
        .send_bincode(p4)
        .values()
        .map(q!(|n| n + 1))
        .send(p5, TCP.bincode())
        .map(q!(|n| n + 1))
        .send_bincode_external(external);

    let ems_c2 = p1
        .source_cluster_members(c2)
        .entries()
        .send_bincode_external(external);

    let ems_c3 = p1
        .source_cluster_members(c3)
        .entries()
        .send_bincode_external(external);

    (tx, rx, ems_c2, ems_c3)
}

const CLUSTER_SIZE: usize = 3;

#[tokio::test]
async fn docker() {
    telemetry::initialize_tracing_with_directive("trace");

    let network = DockerNetwork::new("distributed_echo_test".to_string());

    let mut deployment = DockerDeploy::new(network);

    let builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let p5 = builder.process();
    let (external_sink, external_stream, ems_c2, ems_c3) =
        distributed_echo(&external, &p1, &c2, &c3, &p4, &p5);

    let config = vec![r#"profile.dev.strip="symbols""#.to_string()];

    let nodes = builder
        .with_process(&p1, deployment.add_localhost_docker(None, config.clone()))
        .with_cluster(
            &c2,
            deployment.add_localhost_docker_cluster(None, config.clone(), CLUSTER_SIZE),
        )
        .with_cluster(
            &c3,
            deployment.add_localhost_docker_cluster(None, config.clone(), CLUSTER_SIZE),
        )
        .with_process(&p4, deployment.add_localhost_docker(None, config.clone()))
        .with_process(&p5, deployment.add_localhost_docker(None, config.clone()))
        .with_external(&external, deployment.add_external("external".to_string()))
        .deploy(&mut deployment);

    deployment.provision(&nodes).await.unwrap();
    deployment.start(&nodes).await.unwrap();

    let mut external_sink = nodes.connect(external_sink).await;
    let mut external_stream = nodes.connect(external_stream).await;

    let mut ems_c2_stream = nodes.connect(ems_c2).await;
    let mut ems_c3_stream = nodes.connect(ems_c3).await;

    println!("waiting for c2 events");
    {
        let mut events = Vec::new();
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 1 c2 events");
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 2 c2 events");
    }

    println!("waiting for c3 events");
    {
        let mut events = Vec::new();
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 1 c3 events");
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 2 c3 events");
    }

    println!("sending 0");
    external_sink.send(0).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 5);

    println!("sending 1");

    external_sink.send(1).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 6);

    external_sink.send(2).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 7);

    external_sink.send(3).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 8);

    deployment.stop(&nodes).await.unwrap();

    deployment.cleanup(&nodes).await.unwrap();

    println!("success");
}

#[tokio::test]
async fn localhost() {
    let mut deployment = Deployment::new();

    let builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let p5 = builder.process();
    let (external_sink, external_stream, ems_c2, ems_c3) =
        distributed_echo(&external, &p1, &c2, &c3, &p4, &p5);

    let nodes = builder
        .with_process(&p1, deployment.Localhost())
        .with_cluster(&c2, (0..CLUSTER_SIZE).map(|_| deployment.Localhost()))
        .with_cluster(&c3, (0..CLUSTER_SIZE).map(|_| deployment.Localhost()))
        .with_process(&p4, deployment.Localhost())
        .with_process(&p5, deployment.Localhost())
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();
    deployment.start().await.unwrap();

    let mut external_sink = nodes.connect(external_sink).await;
    let mut external_stream = nodes.connect(external_stream).await;

    let mut ems_c2_stream = nodes.connect(ems_c2).await;
    let mut ems_c3_stream = nodes.connect(ems_c3).await;

    println!("waiting for c2 events");
    {
        let mut events = Vec::new();
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 1 c2 events");
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 2 c2 events");
    }

    println!("waiting for c3 events");
    {
        let mut events = Vec::new();
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 1 c3 events");
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 2 c3 events");
    }

    println!("sending 0");
    external_sink.send(0).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 5);

    println!("sending 1");

    external_sink.send(1).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 6);

    external_sink.send(2).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 7);

    external_sink.send(3).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 8);

    deployment.stop().await.unwrap();

    println!("success");
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn distributed_echo_aws() {
    let mut deployment: Deployment = Deployment::new();

    let builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let p5 = builder.process();
    let (external_sink, external_stream, ems_c2, ems_c3) =
        distributed_echo(&external, &p1, &c2, &c3, &p4, &p5);

    let network = AwsNetwork::new("us-east-1", None);

    let nodes = builder
        .with_process(
            &p1,
            deployment.AwsEc2Host()
                        .region("us-east-1")
                        .instance_type("t3.micro")
                        .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                        .network(network.clone())
                        .add(),
        )
        .with_cluster(
            &c2,
            (0..CLUSTER_SIZE).map(|_| {
                deployment.AwsEc2Host()
                        .region("us-east-1")
                        .instance_type("t3.micro")
                        .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                        .network(network.clone())
                        .add()
            }),
        )
        .with_cluster(
            &c3,
            (0..CLUSTER_SIZE).map(|_| {
                deployment.AwsEc2Host()
                        .region("us-east-1")
                        .instance_type("t3.micro")
                        .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                        .network(network.clone())
                        .add()
            }),
        )
        .with_process(
            &p4,
            deployment.AwsEc2Host()
                        .region("us-east-1")
                        .instance_type("t3.micro")
                        .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                        .network(network.clone())
                        .add(),
        )
        .with_process(
            &p5,
            deployment.AwsEc2Host()
                        .region("us-east-1")
                        .instance_type("t3.micro")
                        .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                        .network(network.clone())
                        .add(),
        )
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut external_sink = nodes.connect(external_sink).await;
    let mut external_stream = nodes.connect(external_stream).await;

    let mut ems_c2_stream = nodes.connect(ems_c2).await;
    let mut ems_c3_stream = nodes.connect(ems_c3).await;

    println!("waiting for c2 events");
    {
        let mut events = Vec::new();
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 1 c2 events");
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 2 c2 events");
    }

    println!("waiting for c3 events");
    {
        let mut events = Vec::new();
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 1 c3 events");
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 2 c3 events");
    }

    println!("sending 0");
    external_sink.send(0).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 5);

    println!("sending 1");

    external_sink.send(1).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 6);

    external_sink.send(2).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 7);

    external_sink.send(3).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 8);

    deployment.stop().await.unwrap();
}
