use dfir_rs::scheduled::ticks::{TickDuration, TickInstant};
use dfir_rs::dfir_syntax;
use multiplatform_test::multiplatform_test;

// TODO(inline): uses next_stratum(), not supported in inline codegen
// #[multiplatform_test(test, wasm, env_tracing)]
// pub fn test_stratum_loop() { ... }

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_tick_loop() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<TickInstant>();

    let mut df = dfir_syntax! {
        source_iter([TickInstant::new(0)]) -> union_tee;
        union_tee = union() -> tee();
        union_tee -> map(|n| n + TickDuration::SINGLE_TICK) -> filter(|&n| n < TickInstant::new(10)) -> defer_tick() -> union_tee;
        union_tee -> for_each(|v| out_send.send(v).unwrap());
    };
    df.run_available_sync();

    assert_eq!(
        &[
            TickInstant::new(0),
            TickInstant::new(1),
            TickInstant::new(2),
            TickInstant::new(3),
            TickInstant::new(4),
            TickInstant::new(5),
            TickInstant::new(6),
            TickInstant::new(7),
            TickInstant::new(8),
            TickInstant::new(9)
        ],
        &*dfir_rs::util::collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}

// TODO(inline): remaining tests use next_stratum() or intra-tick cycles, not supported in inline codegen
// test_stratum_loop: uses next_stratum()
// test_persist_stratum_run_available: uses next_stratum()
// test_persist_stratum_run_async: uses next_stratum()
// test_issue_800_1050_persist: intra-tick cycle (my_union_tee -> filter -> my_union_tee)
// test_issue_800_1050_fold_keyed: intra-tick cycle
// test_issue_800_1050_reduce_keyed: intra-tick cycle
// test_nospin_issue_961: uses next_stratum()
// test_nospin_issue_961_complicated: intra-tick cycle (double -> items)
