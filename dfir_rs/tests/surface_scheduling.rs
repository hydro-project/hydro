use std::error::Error;

use dfir_rs::scheduled::ticks::{TickDuration, TickInstant};
use dfir_rs::{dfir_syntax, rassert_eq};
use multiplatform_test::multiplatform_test;
use tokio::time::timeout;
use web_time::Duration;

// Note: next_stratum() removed from inline codegen tests — it is a no-op in inline mode
// since all operators run in a single stratum.

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_stratum_loop() {
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

#[multiplatform_test(dfir, env_tracing)]
async fn test_persist_stratum_run_available() -> Result<(), Box<dyn Error>> {
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel();

    let mut df = dfir_syntax! {
        a = source_iter([0])
            -> persist::<'static>()
            -> for_each(|x| out_send.send(x).unwrap());
    };
    df.run_available().await;

    let seen: Vec<_> = dfir_rs::util::collect_ready_async(out_recv).await;
    rassert_eq!(
        &[0],
        &*seen,
        "Only one tick should have run, actually ran {}",
        seen.len()
    )?;

    Ok(())
}

#[multiplatform_test(dfir, env_tracing)]
async fn test_persist_stratum_run_async() -> Result<(), Box<dyn Error>> {
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel();

    let mut df = dfir_syntax! {
        source_iter([0])
            -> persist::<'static>()
            -> for_each(|x| out_send.send(x).unwrap());
    };

    timeout(Duration::from_millis(200), df.run())
        .await
        .expect_err("Expected time out");

    let seen: Vec<_> = dfir_rs::util::collect_ready_async(out_recv).await;
    rassert_eq!(
        &[0],
        &*seen,
        "Only one tick should have run, actually ran {}",
        seen.len()
    )?;

    Ok(())
}

// TODO(inline): intra-tick cycle (my_union_tee -> filter -> my_union_tee), not supported
// test_issue_800_1050_persist
// test_issue_800_1050_fold_keyed
// test_issue_800_1050_reduce_keyed

#[multiplatform_test(dfir, env_tracing)]
async fn test_nospin_issue_961() {
    let mut df = dfir_syntax! {
        source_iter([1])
            -> persist::<'static>()
            -> defer_tick_lazy()
            -> null();
    };

    timeout(Duration::from_millis(100), df.run_available())
        .await
        .expect("Should not spin.");
}

// TODO(inline): intra-tick cycle (double -> items), not supported
// test_nospin_issue_961_complicated
