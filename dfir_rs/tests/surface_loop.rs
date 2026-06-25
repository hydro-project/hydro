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
