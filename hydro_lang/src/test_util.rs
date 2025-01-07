use std::future::Future;
use std::pin::Pin;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{FlowBuilder, Process, Stream, Unbounded};

pub async fn stream_transform_test<
    'a,
    O: Serialize + DeserializeOwned + 'static,
    C: Future<Output = ()>,
>(
    thunk: impl FnOnce(&Process<'a>) -> Stream<O, Process<'a>, Unbounded>,
    check: impl FnOnce(Pin<Box<dyn dfir_rs::futures::Stream<Item = O>>>) -> C,
) {
    let mut deployment = hydro_deploy::Deployment::new();
    let flow = FlowBuilder::new();
    let process = flow.process::<()>();
    let external = flow.external_process::<()>();
    let out = thunk(&process);
    let out_port = out.send_bincode_external(&external);
    let nodes = flow
        .with_process(&process, deployment.Localhost())
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let external_out = nodes.connect_source_bincode(out_port).await;
    deployment.start().await.unwrap();

    check(external_out).await;
}
