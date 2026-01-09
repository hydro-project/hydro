use stageleft::q;

use crate::live_collections::sliced::sliced;
use crate::live_collections::stream::{ExactlyOnce, TotalOrder};
use crate::location::{Location, Process};
use crate::nondet::nondet;
use crate::prelude::FlowBuilder;
use crate::sim::{SimReceiver, SimSender};

mod trophies;

// Test is currently broken in nightly.
#[cfg(not(nightly))]
#[test]
#[should_panic]
#[cfg_attr(not(target_os = "linux"), ignore)] // sim reproducer not yet reproducible on non-linux OSes
fn sim_crash_in_output() {
    use bytes::Bytes;

    // run as PATH="$PATH:." cargo sim -p hydro_lang --features sim -- sim_crash_in_output
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input();
    let out_recv: SimReceiver<Bytes, TotalOrder, ExactlyOnce> = input.sim_output();

    flow.sim().fuzz(async || {
        in_send.send(bolero::any::<Vec<u8>>().into());

        let x = out_recv.next().await.unwrap();
        if !x.is_empty() && x[0] == 42 && x.len() > 1 && x[1] == 43 && x.len() > 2 && x[2] == 44 {
            panic!("boom");
        }
    });
}

// Test is currently broken in nightly.
#[cfg(not(nightly))]
#[test]
#[should_panic]
#[cfg_attr(not(target_os = "linux"), ignore)] // sim reproducer not yet reproducible on non-linux OSes
fn sim_crash_in_output_with_filter() {
    use bytes::Bytes;

    // run as PATH="$PATH:." cargo sim -p hydro_lang --features sim -- sim_crash_in_output_with_filter
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<Bytes, _, _>();

    let out_recv = input
        .filter(q!(|x| x.len() > 1 && x[0] == 42 && x[1] == 43))
        .sim_output();

    flow.sim().fuzz(async || {
        in_send.send(bolero::any::<Vec<u8>>().into());

        if let Some(x) = out_recv.next().await
            && x.len() > 2
            && x[2] == 44
        {
            panic!("boom");
        }
    });
}

#[test]
fn sim_batch_preserves_order_fuzzed() {
    // uses RNG fuzzing in CI
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input();

    let tick = node.tick();
    let out_recv = input
        .batch(&tick, nondet!(/** test */))
        .all_ticks()
        .sim_output();

    flow.sim().fuzz(async || {
        in_send.send(1);
        in_send.send(2);
        in_send.send(3);

        assert_eq!(out_recv.next().await.unwrap(), 1);
        assert_eq!(out_recv.next().await.unwrap(), 2);
        assert_eq!(out_recv.next().await.unwrap(), 3);
        assert!(out_recv.next().await.is_none());
    });
}

fn fuzzed_batching_program<'a>(
    node: Process<'a>,
) -> (
    SimSender<i32, TotalOrder, ExactlyOnce>,
    SimReceiver<i32, TotalOrder, ExactlyOnce>,
) {
    let tick = node.tick();

    let (in_send, input) = node.sim_input();

    let out_recv = input
        .batch(&tick, nondet!(/** test */))
        .fold(q!(|| 0), q!(|acc, v| *acc += v))
        .all_ticks()
        .sim_output();
    (in_send, out_recv)
}

fn fuzzed_batching_program_sliced<'a>(
    node: Process<'a>,
) -> (
    SimSender<i32, TotalOrder, ExactlyOnce>,
    SimReceiver<i32, TotalOrder, ExactlyOnce>,
) {
    let (in_send, input) = node.sim_input();

    let out_recv = sliced! {
        let batch = use(input, nondet!(/** test */));
        batch.fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
    }
    .sim_output();
    (in_send, out_recv)
}

#[test]
#[should_panic]
fn sim_crash_with_fuzzed_batching() {
    // run as PATH="$PATH:." cargo sim -p hydro_lang --features sim -- sim_crash_with_fuzzed_batching
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, out_recv) = fuzzed_batching_program(node);

    // takes forever with exhaustive, but should complete quickly with fuzz
    flow.sim().fuzz(async || {
        for _ in 0..1000 {
            in_send.send(456); // the fuzzer should put these some batches
        }

        in_send.send(100);
        in_send.send(23); // the fuzzer must put these in one batch

        in_send.send(99); // the fuzzer must put this in a later batch

        while let Some(out) = out_recv.next().await {
            if out == 456 {
                // make sure exhaustive can't catch the bug by using trivial (size 1) batches
                return;
            } else if out == 123 {
                panic!("boom");
            }
        }
    });
}

#[test]
#[cfg_attr(target_os = "windows", ignore)] // trace locations don't work on Windows right now
fn trace_for_fuzzed_batching() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, out_recv) = fuzzed_batching_program(node);

    let repro_bytes = std::fs::read(
        "./src/sim/tests/sim-failures/hydro_lang__sim__tests__sim_crash_with_fuzzed_batching.bin",
    )
    .unwrap();

    let mut log_out = Vec::new();
    colored::control::set_override(false);

    flow.sim()
        .compiled()
        .fuzz_repro(repro_bytes, async |compiled| {
            let schedule = compiled.schedule_with_logger(&mut log_out);
            let rest = async move {
                for _ in 0..1000 {
                    in_send.send(456); // the fuzzer should put these some batches
                }

                in_send.send(100);
                in_send.send(23); // the fuzzer must put these in one batch

                in_send.send(99); // the fuzzer must put this in a later batch

                while let Some(out) = out_recv.next().await {
                    if out == 456 {
                        // make sure exhaustive can't catch the bug by using trivial (size 1) batches
                        return;
                    } else if out == 123 {
                        // don't actually panic so that we can get the trace
                        return;
                    }
                }
            };

            tokio::select! {
                biased;
                _ = rest => {},
                _ = schedule => {},
            };
        });

    let log_str = String::from_utf8(log_out).unwrap();
    hydro_build_utils::assert_snapshot!(log_str);
}

#[test]
#[cfg_attr(target_os = "windows", ignore)] // trace locations don't work on Windows right now
fn trace_for_fuzzed_batching_sliced() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, out_recv) = fuzzed_batching_program_sliced(node);

    let repro_bytes = std::fs::read(
        "./src/sim/tests/sim-failures/hydro_lang__sim__tests__sim_crash_with_fuzzed_batching.bin",
    )
    .unwrap();

    let mut log_out = Vec::new();
    colored::control::set_override(false);

    flow.sim()
        .compiled()
        .fuzz_repro(repro_bytes, async |compiled| {
            let schedule = compiled.schedule_with_logger(&mut log_out);
            let rest = async move {
                for _ in 0..1000 {
                    in_send.send(456); // the fuzzer should put these some batches
                }

                in_send.send(100);
                in_send.send(23); // the fuzzer must put these in one batch

                in_send.send(99); // the fuzzer must put this in a later batch

                while let Some(out) = out_recv.next().await {
                    if out == 456 {
                        // make sure exhaustive can't catch the bug by using trivial (size 1) batches
                        return;
                    } else if out == 123 {
                        // don't actually panic so that we can get the trace
                        return;
                    }
                }
            };

            tokio::select! {
                biased;
                _ = rest => {},
                _ = schedule => {},
            };
        });

    let log_str = String::from_utf8(log_out).unwrap();
    hydro_build_utils::assert_snapshot!(log_str);
}
