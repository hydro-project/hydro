//! Various utilities for testing short Hydro programs, especially in doctests.

use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::compile::builder::FlowBuilder;
use crate::live_collections::boundedness::Unbounded;
use crate::live_collections::stream::{Ordering, Retries, Stream};
use crate::location::Process;

/// Sets up a test with multiple processes / clusters declared in the test logic (`thunk`). The test logic must return
/// a single streaming output, which can then be read in `check` (an async closure) to perform assertions.
///
/// Each declared process is deployed as a single local process, and each cluster is deployed as four local processes.
pub async fn multi_location_test<'a, T, C, O: Ordering, R: Retries>(
    thunk: impl FnOnce(&FlowBuilder<'a>, &Process<'a, ()>) -> Stream<T, Process<'a>, Unbounded, O, R>,
    check: impl FnOnce(Pin<Box<dyn futures::Stream<Item = T>>>) -> C,
) where
    T: Serialize + DeserializeOwned + 'static,
    C: Future<Output = ()>,
{
    let mut deployment = hydro_deploy::Deployment::new();
    let flow = FlowBuilder::new();
    let process = flow.process::<()>();
    let external = flow.external::<()>();
    let out = thunk(&flow, &process);
    let out_port = out.send_bincode_external(&external);
    let nodes = flow
        .with_remaining_processes(|| deployment.Localhost())
        .with_remaining_clusters(|| vec![deployment.Localhost(); 4])
        .with_external(&external, deployment.Localhost())
        .deploy(&(), &mut deployment);

    deployment.deploy().await.unwrap();

    let external_out = nodes.connect(out_port).await;
    deployment.start().await.unwrap();

    check(external_out).await;
}

/// Sets up a test declared in `thunk` that executes on a single [`Process`], returning a streaming output
/// that can be read in `check` (an async closure) to perform assertions.
pub async fn stream_transform_test<'a, T, C, O: Ordering, R: Retries>(
    thunk: impl FnOnce(&Process<'a>) -> Stream<T, Process<'a>, Unbounded, O, R>,
    check: impl FnOnce(Pin<Box<dyn futures::Stream<Item = T>>>) -> C,
) where
    T: Serialize + DeserializeOwned + 'static,
    C: Future<Output = ()>,
{
    let mut deployment = hydro_deploy::Deployment::new();
    let flow = FlowBuilder::new();
    let process = flow.process::<()>();
    let external = flow.external::<()>();
    let out = thunk(&process);
    let out_port = out.send_bincode_external(&external);
    let nodes = flow
        .with_process(&process, deployment.Localhost())
        .with_external(&external, deployment.Localhost())
        .deploy(&(), &mut deployment);

    deployment.deploy().await.unwrap();

    let external_out = nodes.connect(out_port).await;
    deployment.start().await.unwrap();

    check(external_out).await;
}

// from https://users.rust-lang.org/t/how-to-write-doctest-that-panic-with-an-expected-message/58650
/// Asserts that running the given closure results in a panic with a message containing `msg`.
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
