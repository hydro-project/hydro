use std::error::Error;

use dfir_rs::scheduled::ticks::{TickDuration, TickInstant};
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax, rassert_eq};
use multiplatform_test::multiplatform_test;
use tokio::time::timeout;
use web_time::Duration;

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_stratum_loop() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<TickInstant>();

    let mut df = dfir_syntax! {
        source_iter([TickInstant::new(0)]) -> union_tee;
        union_tee = union() -> tee();
        union_tee -> map(|n| n + TickDuration::SINGLE_TICK) -> filter(|&n| n < TickInstant::new(10)) -> next_stratum() -> defer_tick() -> union_tee;
        union_tee -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
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
    assert_eq!(
        (TickInstant::new(10), 0),
        (df.current_tick(), df.current_stratum())
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
    assert_graphvis_snapshots!(df);
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
    assert_eq!(
        (TickInstant::new(10), 0),
        (df.current_tick(), df.current_stratum())
    );
}

#[multiplatform_test(dfir, env_tracing)]
async fn test_persist_stratum_run_available() -> Result<(), Box<dyn Error>> {
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel();

    let mut df = dfir_syntax! {
        a = source_iter([0])
            -> persist::<'static>()
            -> next_stratum()
            -> for_each(|x| out_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(df);
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
            -> next_stratum()
            -> for_each(|x| out_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(df);

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

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_issue_800_1050_persist() {
    let mut df = dfir_syntax! {
        in1 = source_iter(0..10) -> map(|i| (i, i));
        in1 -> persist::<'static>() -> my_union_tee;

        my_union_tee = union() -> tee();
        my_union_tee -> filter(|_| false) -> my_union_tee;
        my_union_tee -> for_each(|x| println!("A {} {} {:?}", context.current_tick(), context.current_stratum(), x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();
}

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_issue_800_1050_fold_keyed() {
    let mut df = dfir_syntax! {
        in1 = source_iter(0..10) -> map(|i| (i, i));
        in1 -> fold_keyed::<'static>(Vec::new, Vec::push) -> my_union_tee;

        my_union_tee = union() -> tee();
        my_union_tee -> filter(|_| false) -> my_union_tee;
        my_union_tee -> for_each(|x| println!("A {} {} {:?}", context.current_tick(), context.current_stratum(), x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();
}

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_issue_800_1050_reduce_keyed() {
    let mut df = dfir_syntax! {
        in1 = source_iter(0..10) -> map(|i| (i, i));
        in1 -> reduce_keyed::<'static>(std::ops::AddAssign::add_assign) -> my_union_tee;

        my_union_tee = union() -> tee();
        my_union_tee -> filter(|_| false) -> my_union_tee;
        my_union_tee -> for_each(|x| println!("A {} {} {:?}", context.current_tick(), context.current_stratum(), x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();
}

#[multiplatform_test(dfir, env_tracing)]
async fn test_nospin_issue_961() {
    let mut df = dfir_syntax! {
        source_iter([1])
            -> next_stratum()
            -> persist::<'static>()
            -> defer_tick_lazy()
            -> null();
    };
    assert_graphvis_snapshots!(df);

    timeout(Duration::from_millis(100), df.run_available())
        .await
        .expect("Should not spin.");
}

#[multiplatform_test(dfir, env_tracing)]
async fn test_nospin_issue_961_complicated() {
    let mut df = dfir_syntax! {
        source_iter([1]) -> items;
        items = union();

        double = items
            -> persist::<'static>()
            -> fold(|| 0, |accum, x| *accum += x)
            -> defer_tick_lazy()
            -> filter(|_| false)
            -> tee();

        double -> null();

        double -> items;
    };
    assert_graphvis_snapshots!(df);

    timeout(Duration::from_millis(100), df.run_available())
        .await
        .expect("Should not spin.");
}
