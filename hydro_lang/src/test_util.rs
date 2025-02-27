use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{FlowBuilder, Process, Stream, Unbounded};

pub async fn multi_location_test<
    'a,
    O: Serialize + DeserializeOwned + 'static,
    C: Future<Output = ()>,
    OutOrder,
>(
    thunk: impl FnOnce(
        &FlowBuilder<'a>,
        &Process<'a, ()>,
    ) -> Stream<O, Process<'a>, Unbounded, OutOrder>,
    check: impl FnOnce(Pin<Box<dyn dfir_rs::futures::Stream<Item = O>>>) -> C,
) {
    let mut deployment = hydro_deploy::Deployment::new();
    let flow = FlowBuilder::new();
    let process = flow.process::<()>();
    let external = flow.external_process::<()>();
    let out = thunk(&flow, &process);
    let out_port = out.send_bincode_external(&external);
    let nodes = flow
        .with_remaining_processes(|| deployment.Localhost())
        .with_remaining_clusters(|| vec![deployment.Localhost(); 4])
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let external_out = nodes.connect_source_bincode(out_port).await;
    deployment.start().await.unwrap();

    check(external_out).await;
}

pub async fn stream_transform_test<
    'a,
    O: Serialize + DeserializeOwned + 'static,
    C: Future<Output = ()>,
    OutOrder,
>(
    thunk: impl FnOnce(&Process<'a>) -> Stream<O, Process<'a>, Unbounded, OutOrder>,
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

// from https://users.rust-lang.org/t/how-to-write-doctest-that-panic-with-an-expected-message/58650
pub fn assert_panics_with_message(func: impl FnOnce(), msg: &'static str) {
    let err = catch_unwind(AssertUnwindSafe(func)).expect_err("Didn't panic!");

    let chk = |panic_msg: &'_ str| {
        if !panic_msg.contains(msg) {
            panic!(
                "Expected a panic message containing `{}`; got: `{}`.",
                msg, panic_msg
            );
        }
    };

    err.downcast::<String>()
        .map(|s| chk(&s))
        .or_else(|err| err.downcast::<&'static str>().map(|s| chk(*s)))
        .expect("Unexpected panic type!");
}
