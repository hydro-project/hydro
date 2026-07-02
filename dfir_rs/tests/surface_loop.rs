use dfir_rs::dfir_syntax;
use multiplatform_test::multiplatform_test;

/// Test that a loop body only fires when its entry handoff (batch) has data.
/// The `for_each` inside the loop should only see data when elements are sent.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_loop_gating_basic() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        loop {
            inp -> batch() -> for_each(|x| out_send.send(x).unwrap());
        };
    };

    // First tick: no data sent, loop should not fire.
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, Vec::<i32>::new());

    // Send data, run another tick. Loop should fire.
    in_send.send(1).unwrap();
    in_send.send(2).unwrap();
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, vec![1, 2]);

    // Another tick with no data — loop should not fire again.
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, Vec::<i32>::new());
}

/// Test that two independent loops do not trigger each other.
/// Data arriving for loop A should not cause loop B to fire.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_loop_independence() {
    let (in_a_send, in_a_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (_in_b_send, in_b_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_a_send, mut out_a_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_b_send, mut out_b_recv) = dfir_rs::util::unbounded_channel::<&'static str>();

    let mut df = dfir_syntax! {
        inp_a = source_stream(in_a_recv);
        inp_b = source_stream(in_b_recv);
        loop {
            inp_a -> batch() -> for_each(|x| out_a_send.send(x).unwrap());
        };
        loop {
            inp_b -> batch() -> for_each(|_x: i32| out_b_send.send("fired").unwrap());
        };
    };

    // Send data only to loop A.
    in_a_send.send(42).unwrap();
    df.run_tick_sync();

    let out_a: Vec<i32> = dfir_rs::util::collect_ready(&mut out_a_recv);
    let out_b: Vec<&str> = dfir_rs::util::collect_ready(&mut out_b_recv);

    // Loop A should fire, loop B should NOT fire.
    assert_eq!(out_a, vec![42]);
    assert_eq!(out_b, Vec::<&str>::new());
}

/// Test defer_iteration (non-lazy): data deferred in one firing is available on the next,
/// and the non-empty defer_iteration buffer causes the loop to re-fire.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_defer_iteration_basic() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        loop {
            merged = union() -> tee();
            inp -> batch() -> merged;
            deferred -> merged;
            merged -> for_each(|x| out_send.send(x).unwrap());
            // Defer items < 100 back, multiplied by 10, to trigger re-fire.
            merged -> filter(|&x| x < 100) -> map(|x| x * 10) -> defer_iteration() -> deferred;
            deferred = identity();
        };
    };

    // Send 1. First firing: sees 1, defers 10.
    // Second firing (triggered by defer_iteration): sees 10, defers 100.
    // Third firing: sees 100 (>=100, no defer). Loop stops.
    in_send.send(1).unwrap();
    df.run_tick_sync();

    let mut out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![1, 10, 100]);
}

/// Test defer_iteration_lazy: data deferred is available next firing but does NOT
/// cause re-fire on its own.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_defer_iteration_lazy() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        loop {
            merged = union() -> tee();
            inp -> batch() -> merged;
            deferred -> merged;
            merged -> for_each(|x| out_send.send(x).unwrap());
            // Lazy defer: stash data but don't re-fire.
            merged -> map(|x| x * 10) -> defer_iteration_lazy() -> deferred;
            deferred = identity();
        };
    };

    // Send 1. Loop fires once (sees 1), defers 10 lazily. Loop does NOT re-fire.
    in_send.send(1).unwrap();
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, vec![1]);

    // Send 2. Loop fires again, sees both 2 (from input) and 10 (from lazy defer).
    // Defers 20 and 100 lazily for next time.
    in_send.send(2).unwrap();
    df.run_tick_sync();
    let mut out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![2, 10]);
}

/// Test batch_lazy: data enters the loop but does NOT trigger it to fire.
/// If the loop fires for another reason, the lazy data is available.
/// If the loop does not fire, the data is dropped.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_batch_lazy() {
    let (trigger_send, trigger_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (lazy_send, lazy_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        trigger_inp = source_stream(trigger_recv);
        lazy_inp = source_stream(lazy_recv);
        loop {
            merged = union();
            trigger_inp -> batch() -> merged;
            lazy_inp -> batch_lazy() -> merged;
            merged -> for_each(|x| out_send.send(x).unwrap());
        };
    };

    // Tick 1: only lazy data — loop should NOT fire.
    lazy_send.send(100).unwrap();
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, Vec::<i32>::new());

    // Tick 2: trigger data arrives — loop fires, sees both trigger and lazy data.
    trigger_send.send(1).unwrap();
    lazy_send.send(200).unwrap();
    df.run_tick_sync();
    let mut out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![1, 200]);

    // Tick 3: only trigger, no lazy data.
    trigger_send.send(2).unwrap();
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, vec![2]);

    // Tick 4: only lazy data again — loop should NOT fire, data dropped.
    lazy_send.send(300).unwrap();
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, Vec::<i32>::new());
}
