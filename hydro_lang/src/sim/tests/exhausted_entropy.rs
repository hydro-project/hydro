//! Once the fuzzer's entropy is exhausted, unhooked at-least-once operators must stop
//! generating retries, so the run terminates with every element delivered exactly once.
//! Each scenario is run against both drivers that can run out: the byte-slice driver
//! (libfuzzer, reproducers) and the random driver (the `cargo test` fallback of `fuzz`).

use std::panic::RefUnwindSafe;

use crate::live_collections::sliced::sliced;
use crate::live_collections::stream::{AtLeastOnce, ExactlyOnce, NoOrder, TotalOrder};
use crate::location::Location;
use crate::nondet::nondet;
use crate::prelude::FlowBuilder;
use crate::sim_hooks::BatchHook;

#[derive(Clone, Copy)]
enum ExhaustedDriver {
    Bytes,
    Rng,
}

fn run_exhausted(
    flow: FlowBuilder<'_>,
    driver: ExhaustedDriver,
    thunk: impl AsyncFn() + RefUnwindSafe,
) {
    match driver {
        // an empty input, so every decision reads zeroed bytes
        ExhaustedDriver::Bytes => flow
            .sim()
            .compiled()
            .fuzz_repro(vec![], async move |instance| {
                instance
                    .run_with_scheduler_and_logger(std::io::sink(), thunk())
                    .await
            }),
        ExhaustedDriver::Rng => flow
            .sim()
            .unit_test_fuzz_iterations(1)
            .unit_test_fuzz_max_len(0)
            .fuzz(thunk),
    }
}

fn top_level_unordered_retries(driver: ExhaustedDriver) {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, NoOrder, ExactlyOnce>();
    let output = input
        .weaken_retries::<AtLeastOnce>()
        .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
        .sim_output();

    run_exhausted(flow, driver, async || {
        in_send.send_many_unordered([1, 2, 3]);
        let all: Vec<u32> = output.collect_sorted().await;
        assert_eq!(all, [1, 2, 3]);
    });
}

fn top_level_ordered_retries(driver: ExhaustedDriver) {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let output = input
        .weaken_retries::<AtLeastOnce>()
        .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
        .sim_output();

    run_exhausted(flow, driver, async || {
        in_send.send_many([1, 2, 3]);
        output.assert_yields_only([1, 2, 3]).await;
    });
}

fn top_level_alo_ordering(driver: ExhaustedDriver) {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, NoOrder, ExactlyOnce>();
    let output = input
        .weaken_retries::<AtLeastOnce>()
        .assume_ordering::<TotalOrder>(nondet!(/** fuzzed */))
        .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
        .sim_output();

    run_exhausted(flow, driver, async || {
        in_send.send_many_unordered([1, 2, 3]);
        let mut all: Vec<u32> = output.collect().await;
        all.sort();
        assert_eq!(all, [1, 2, 3]);
    });
}

fn in_tick_retries(driver: ExhaustedDriver) {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    // scripted so all three elements land in the same tick
    let batch_hook: BatchHook<u32, NoOrder> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<u32, NoOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input, nondet!(/** scripted */ hook = batch_hook));
        b.weaken_retries::<AtLeastOnce>()
            .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
    }
    .sim_output();

    run_exhausted(flow, driver, async || {
        in_send.send_many_unordered([1, 2, 3]);
        batch_hook.release_values([1, 2, 3]).await;
        let all: Vec<u32> = output.collect_sorted().await;
        assert_eq!(all, [1, 2, 3]);
    });
}

fn in_tick_alo_ordering(driver: ExhaustedDriver) {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    // scripted so all three elements land in the same tick
    let batch_hook: BatchHook<u32, NoOrder> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<u32, NoOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input, nondet!(/** scripted */ hook = batch_hook));
        b.weaken_retries::<AtLeastOnce>()
            .assume_ordering::<TotalOrder>(nondet!(/** fuzzed */))
            .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
    }
    .sim_output();

    run_exhausted(flow, driver, async || {
        in_send.send_many_unordered([1, 2, 3]);
        batch_hook.release_values([1, 2, 3]).await;
        let mut all: Vec<u32> = output.collect().await;
        all.sort();
        assert_eq!(all, [1, 2, 3]);
    });
}

fn ordered_alo_batch(driver: ExhaustedDriver) {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input.weaken_retries::<AtLeastOnce>(), nondet!(/** fuzzed */));
        b.assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
    }
    .sim_output();

    run_exhausted(flow, driver, async || {
        in_send.send_many([1, 2, 3]);
        output.assert_yields_only([1, 2, 3]).await;
    });
}

fn unordered_alo_batch(driver: ExhaustedDriver) {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, NoOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input.weaken_retries::<AtLeastOnce>(), nondet!(/** fuzzed */));
        b.assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
    }
    .sim_output();

    run_exhausted(flow, driver, async || {
        in_send.send_many_unordered([1, 2, 3]);
        let all: Vec<u32> = output.collect_sorted().await;
        assert_eq!(all, [1, 2, 3]);
    });
}

macro_rules! for_each_driver {
    ($($scenario:ident),* $(,)?) => {
        $(
            mod $scenario {
                use super::ExhaustedDriver;

                #[test]
                fn bytes() {
                    super::$scenario(ExhaustedDriver::Bytes);
                }

                #[test]
                fn rng() {
                    super::$scenario(ExhaustedDriver::Rng);
                }
            }
        )*
    };
}

for_each_driver!(
    top_level_unordered_retries,
    top_level_ordered_retries,
    top_level_alo_ordering,
    in_tick_retries,
    in_tick_alo_ordering,
    ordered_alo_batch,
    unordered_alo_batch,
);
