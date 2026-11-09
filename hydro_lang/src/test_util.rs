//! Various utilities for testing short Hydro programs, especially in doctests.

use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::compile::builder::FlowBuilder;
use crate::live_collections::boundedness::{Boundedness, Unbounded};
use crate::live_collections::stream::{Ordering, Retries, Stream};
use crate::location::Process;
use crate::sim::flow::SimFlow;

/// Sets up a test with multiple processes / clusters declared in the test logic (`thunk`). The test logic must return
/// a single streaming output, which can then be read in `check` (an async closure) to perform assertions.
///
/// The program runs in the Hydro simulator, with each cluster having four members, and `check`
/// runs against every execution the simulator explores, so it must hold no matter how the
/// program's nondeterminism (batching, message order, retries, membership) plays out. The output
/// stream ends once the program can produce nothing more.
pub async fn multi_location_test<'a, T, C, O: Ordering + 'static, R: Retries + 'static>(
    thunk: impl FnOnce(
        &mut FlowBuilder<'a>,
        &Process<'a, ()>,
    ) -> Stream<T, Process<'a>, Unbounded, O, R>
    + Send,
    check: impl Fn(Pin<Box<dyn futures::Stream<Item = T>>>) -> C + Send,
) where
    T: Serialize + DeserializeOwned + 'static,
    C: Future<Output = ()>,
{
    multi_location_test_with_cluster_size(4, thunk, check).await;
}

/// Like [`multi_location_test`], but each cluster has `cluster_size` members. Fewer members make
/// the simulator's search smaller, since each member may join at any point in an execution.
#[expect(
    clippy::unused_async,
    reason = "async so that doctests can call it with `tokio_test::block_on`, like other async helpers"
)]
pub async fn multi_location_test_with_cluster_size<
    'a,
    T,
    C,
    O: Ordering + 'static,
    R: Retries + 'static,
>(
    cluster_size: usize,
    thunk: impl FnOnce(
        &mut FlowBuilder<'a>,
        &Process<'a, ()>,
    ) -> Stream<T, Process<'a>, Unbounded, O, R>
    + Send,
    check: impl Fn(Pin<Box<dyn futures::Stream<Item = T>>>) -> C + Send,
) where
    T: Serialize + DeserializeOwned + 'static,
    C: Future<Output = ()>,
{
    on_own_thread(move || {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();
        let out = thunk(&mut flow, &process).sim_output();
        let mut sim = flow.sim();
        #[expect(
            clippy::disallowed_methods,
            reason = "every cluster gets the same size, so iteration order does not matter"
        )]
        let clusters: Vec<_> = sim.clusters.keys().collect();
        for cluster in clusters {
            sim.cluster_max_sizes.insert(cluster, cluster_size);
        }
        run_check(sim, out, check);
    });
}

/// Sets up a test declared in `thunk` that executes on a single [`Process`], returning a streaming output
/// that can be read in `check` (an async closure) to perform assertions.
///
/// The program runs in the Hydro simulator, and `check` runs against every execution the
/// simulator explores. The output stream ends once the program can produce nothing more.
#[expect(
    clippy::unused_async,
    reason = "async so that doctests can call it with `tokio_test::block_on`, like other async helpers"
)]
pub async fn stream_transform_test<
    'a,
    T,
    C,
    B: Boundedness,
    O: Ordering + 'static,
    R: Retries + 'static,
>(
    thunk: impl FnOnce(&Process<'a>) -> Stream<T, Process<'a>, B, O, R> + Send,
    check: impl Fn(Pin<Box<dyn futures::Stream<Item = T>>>) -> C + Send,
) where
    T: Serialize + DeserializeOwned + 'static,
    C: Future<Output = ()>,
{
    on_own_thread(move || {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();
        let out = thunk(&process).sim_output();
        run_check(flow.sim(), out, check);
    });
}

/// Runs `f` on a thread of its own, since the simulator starts an async runtime for each
/// execution, which cannot run inside the caller's (doctests call these helpers from
/// `tokio_test::block_on`).
fn on_own_thread(f: impl FnOnce() + Send) {
    std::thread::scope(|scope| {
        if let Err(panic) = scope.spawn(f).join() {
            std::panic::resume_unwind(panic);
        }
    });
}

/// Runs `check` on the output `out` of the simulation `sim`, in every execution the simulator
/// explores.
fn run_check<T, C, O: Ordering + 'static, R: Retries + 'static>(
    sim: SimFlow<'_>,
    out: crate::sim::SimReceiver<T, O, R>,
    check: impl Fn(Pin<Box<dyn futures::Stream<Item = T>>>) -> C,
) where
    T: 'static,
    C: Future<Output = ()>,
{
    // The simulator requires an unwind-safe test body; a panic in `check` fails the test anyway.
    let (check, out) = (AssertUnwindSafe(check), AssertUnwindSafe(out));
    sim.exhaustive(async || {
        let stream = futures::stream::unfold(*out, |out| async move {
            out.try_next_impl().await.map(|item| (item, out))
        });
        (*check)(Box::pin(stream)).await;
    });
}

// from https://users.rust-lang.org/t/how-to-write-doctest-that-panic-with-an-expected-message/58650
/// Asserts that running the given closure results in a panic with a message containing `msg`.
pub fn assert_panics_with_message(func: impl FnOnce(), msg: &'static str) {
    let err = catch_unwind(AssertUnwindSafe(func)).expect_err("Didn't panic!");

    let chk = |panic_msg: &'_ str| {
        assert!(
            panic_msg.contains(msg),
            "Expected a panic message containing `{}`; got: `{}`.",
            msg,
            panic_msg
        );
    };

    err.downcast::<String>()
        .map(|s| chk(&s))
        .or_else(|err| err.downcast::<&'static str>().map(|s| chk(*s)))
        .expect("Unexpected panic type!");
}
