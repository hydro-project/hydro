use bytes::Bytes;
use futures::StreamExt;
use stageleft::q;

use crate::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
use crate::location::{External, Location, Process};
use crate::nondet::nondet;
use crate::prelude::FlowBuilder;

#[test]
#[should_panic]
fn sim_crash_in_output() {
    // run as PATH="$PATH:." cargo sim -p hydro_lang --features sim -- sim_crash_in_output
    let flow = FlowBuilder::new();
    let external = flow.external::<()>();
    let node = flow.process::<()>();

    let (port, input) = node.source_external_bincode(&external);
    let out_port = input.send_bincode_external(&external);

    flow.sim().fuzz(async |mut compiled| {
        let in_send = compiled.connect_sink_bincode(&port);
        let mut out_recv = compiled.connect_source_bincode::<Bytes>(&out_port);
        compiled.launch();

        in_send(bolero::any::<Vec<u8>>().into()).unwrap();

        let x = out_recv.next().await.unwrap();
        if !x.is_empty() && x[0] == 42 && x.len() > 1 && x[1] == 43 && x.len() > 2 && x[2] == 44 {
            panic!("boom");
        }
    });
}

#[test]
#[should_panic]
fn sim_crash_in_output_with_filter() {
    // run as PATH="$PATH:." cargo sim -p hydro_lang --features sim -- sim_crash_in_output_with_filter
    let flow = FlowBuilder::new();
    let external = flow.external::<()>();
    let node = flow.process::<()>();

    let (port, input) = node.source_external_bincode::<_, Bytes>(&external);

    let out_port = input
        .filter(q!(|x| x.len() > 1 && x[0] == 42 && x[1] == 43))
        .send_bincode_external(&external);

    flow.sim().fuzz(async |mut compiled| {
        let in_send = compiled.connect_sink_bincode(&port);
        let mut out_recv = compiled.connect_source_bincode::<Bytes>(&out_port);
        compiled.launch();

        in_send(bolero::any::<Vec<u8>>().into()).unwrap();

        if let Some(x) = out_recv.next().await
            && x.len() > 2
            && x[2] == 44
        {
            panic!("boom");
        }
    });
}

#[test]
#[should_panic]
fn sim_batch_nondet_size() {
    let flow = FlowBuilder::new();
    let external = flow.external::<()>();
    let node = flow.process::<()>();

    let (port, input) = node.source_external_bincode(&external);

    let tick = node.tick();
    let out_port = input
        .batch(&tick, nondet!(/** test */))
        .count()
        .all_ticks()
        .send_bincode_external(&external);

    flow.sim().exhaustive(async |mut compiled| {
        let in_send = compiled.connect_sink_bincode(&port);
        let mut out_recv = compiled.connect_source_bincode(&out_port);
        compiled.launch();

        in_send(()).unwrap();
        in_send(()).unwrap();
        in_send(()).unwrap();

        assert_eq!(out_recv.next().await.unwrap(), 3); // fails with nondet batching
    });
}

#[test]
fn sim_batch_preserves_order() {
    let flow = FlowBuilder::new();
    let external = flow.external::<()>();
    let node = flow.process::<()>();

    let (port, input) = node.source_external_bincode(&external);

    let tick = node.tick();
    let out_port = input
        .batch(&tick, nondet!(/** test */))
        .all_ticks()
        .send_bincode_external(&external);

    flow.sim().exhaustive(async |mut compiled| {
        let in_send = compiled.connect_sink_bincode(&port);
        let mut out_recv = compiled.connect_source_bincode(&out_port);
        compiled.launch();

        in_send(1).unwrap();
        in_send(2).unwrap();
        in_send(3).unwrap();

        assert_eq!(out_recv.next().await.unwrap(), 1);
        assert_eq!(out_recv.next().await.unwrap(), 2);
        assert_eq!(out_recv.next().await.unwrap(), 3);
        assert!(out_recv.next().await.is_none());
    });
}

#[test]
fn sim_batch_preserves_order_fuzzed() {
    // uses RNG fuzzing in CI
    let flow = FlowBuilder::new();
    let external = flow.external::<()>();
    let node = flow.process::<()>();

    let (port, input) = node.source_external_bincode(&external);

    let tick = node.tick();
    let out_port = input
        .batch(&tick, nondet!(/** test */))
        .all_ticks()
        .send_bincode_external(&external);

    flow.sim().fuzz(async |mut compiled| {
        let in_send = compiled.connect_sink_bincode(&port);
        let mut out_recv = compiled.connect_source_bincode(&out_port);
        compiled.launch();

        in_send(1).unwrap();
        in_send(2).unwrap();
        in_send(3).unwrap();

        assert_eq!(out_recv.next().await.unwrap(), 1);
        assert_eq!(out_recv.next().await.unwrap(), 2);
        assert_eq!(out_recv.next().await.unwrap(), 3);
        assert!(out_recv.next().await.is_none());
    });
}

fn fuzzed_batching_program<'a>(
    external: External<'a, ()>,
    node: Process<'a>,
) -> (ExternalBincodeSink<i32>, ExternalBincodeStream<i32>) {
    let tick = node.tick();

    let (port, input) = node.source_external_bincode(&external);

    let out_port = input
        .batch(&tick, nondet!(/** test */))
        .fold(q!(|| 0), q!(|acc, v| *acc += v))
        .all_ticks()
        .send_bincode_external(&external);
    (port, out_port)
}

#[test]
#[should_panic]
fn sim_crash_with_fuzzed_batching() {
    // run as PATH="$PATH:." cargo sim -p hydro_lang --features sim -- sim_crash_with_fuzzed_batching
    let flow = FlowBuilder::new();
    let external = flow.external::<()>();
    let node = flow.process::<()>();
    let (port, out_port) = fuzzed_batching_program(external, node);

    // takes forever with exhaustive, but should complete quickly with fuzz
    flow.sim().fuzz(async |mut compiled| {
        let in_send = compiled.connect_sink_bincode(&port);
        let mut out_recv = compiled.connect_source_bincode(&out_port);
        compiled.launch();

        for _ in 0..1000 {
            in_send(456).unwrap(); // the fuzzer should put these some batches
        }

        in_send(100).unwrap();
        in_send(23).unwrap(); // the fuzzer must put these in one batch

        in_send(99).unwrap(); // the fuzzer must put this in a later batch

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
fn trace_for_fuzzed_batching() {
    let flow = FlowBuilder::new();
    let external = flow.external::<()>();
    let node = flow.process::<()>();

    let (port, out_port) = fuzzed_batching_program(external, node);

    let repro_bytes = std::fs::read(
        "./src/sim/tests/sim-failures/hydro_lang__sim__tests__sim_crash_with_fuzzed_batching.bin",
    )
    .unwrap();

    let mut log_out = Vec::new();
    colored::control::set_override(false);

    flow.sim()
        .compiled()
        .fuzz_repro(repro_bytes, async |mut compiled| {
            let in_send = compiled.connect_sink_bincode(&port);
            let mut out_recv = compiled.connect_source_bincode(&out_port);

            let schedule = compiled.schedule_with_logger(&mut log_out);
            let rest = async move {
                for _ in 0..1000 {
                    in_send(456).unwrap(); // the fuzzer should put these some batches
                }

                in_send(100).unwrap();
                in_send(23).unwrap(); // the fuzzer must put these in one batch

                in_send(99).unwrap(); // the fuzzer must put this in a later batch

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
