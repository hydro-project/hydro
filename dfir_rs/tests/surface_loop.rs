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

/// Regression test for a loop-contiguity toposort bug.
///
/// A single loop has two ingress points, but the sources feeding them are declared
/// *after* the loop (forward references). This causes the source that feeds the second
/// ingress to be inserted late, so the flat toposort interleaves as
/// `[source1, loopA, source2, loopB]`. Previously `make_loops_contiguous` would gather
/// the loop to `loopA`'s position (`[source1, loopA, loopB, source2]`), hoisting the
/// ingress *receiver* `loopB` ahead of its *sender* `source2` — producing a
/// handoff-buffer "use before declaration" compile error (and, if it compiled, an
/// empty/stale batch).
///
/// With the fix, ingress senders are ordered before the whole loop block, so this both
/// compiles and delivers all data.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_loop_ingress_forward_ref_sources() {
    let (in1_send, in1_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (in2_send, in2_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out1_send, mut out1_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out2_send, mut out2_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        loop {
            inp1 -> batch() -> for_each(|x| out1_send.send(x).unwrap());
            inp2 -> batch() -> for_each(|x| out2_send.send(x).unwrap());
        };
        // Sources declared *after* the loop, so their nodes are inserted late.
        inp1 = source_stream(in1_recv);
        inp2 = source_stream(in2_recv);
    };

    in1_send.send(1).unwrap();
    in1_send.send(2).unwrap();
    in2_send.send(10).unwrap();
    in2_send.send(20).unwrap();
    df.run_tick_sync();

    let mut out1: Vec<i32> = dfir_rs::util::collect_ready(&mut out1_recv);
    let mut out2: Vec<i32> = dfir_rs::util::collect_ready(&mut out2_recv);
    out1.sort_unstable();
    out2.sort_unstable();

    // Both ingress points must receive their data (no stale/empty batches).
    assert_eq!(out1, vec![1, 2]);
    assert_eq!(out2, vec![10, 20]);
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

/// Test defer_tick (non-lazy): data deferred in one firing is available on the next,
/// and the non-empty defer_tick buffer causes the loop to re-fire.
/// Note: defer_tick only works in nested loops (not root-level ones, which are fused with tick).
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_defer_tick_basic() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        // Root-level loop gates on input.
        loop {
            inp -> batch() -> root_data;
            root_data = identity();
            // Nested loop: iterates via defer_tick.
            loop {
                merged = union() -> tee();
                root_data -> batch() -> merged;
                deferred -> merged;
                merged -> for_each(|x| out_send.send(x).unwrap());
                // Defer items < 100 back, multiplied by 10, to trigger re-fire.
                merged -> filter(|&x| x < 100) -> map(|x| x * 10) -> defer_tick() -> deferred;
                deferred = identity();
            };
        };
    };

    // Send 1. First firing: sees 1, defers 10.
    // Second firing (triggered by defer_tick): sees 10, defers 100.
    // Third firing: sees 100 (>=100, no defer). Loop stops.
    in_send.send(1).unwrap();
    df.run_tick_sync();

    let mut out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![1, 10, 100]);
}

/// Test defer_tick_lazy: data deferred is available next firing but does NOT
/// cause re-fire on its own.
/// Note: defer_tick_lazy only works in nested loops.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_defer_tick_lazy() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        // Root-level loop gates on input.
        loop {
            inp -> batch() -> root_data;
            root_data = identity();
            // Nested loop: uses defer_tick_lazy.
            loop {
                merged = union() -> tee();
                root_data -> batch() -> merged;
                deferred -> merged;
                merged -> for_each(|x| out_send.send(x).unwrap());
                // Lazy defer: stash data but don't re-fire.
                merged -> map(|x| x * 10) -> defer_tick_lazy() -> deferred;
                deferred = identity();
            };
        };
    };

    // Send 1. Inner loop fires once (sees 1), defers 10 lazily. Inner loop does NOT re-fire.
    in_send.send(1).unwrap();
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, vec![1]);

    // Send 2. Inner loop fires again, sees both 2 (from input) and 10 (from lazy defer).
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

/// Test all_iterations: collects output from all loop iterations and emits
/// it outside the loop after the loop completes.
/// Note: defer_tick only works in nested loops, so we use a nested structure.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_all_iterations() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        // Root-level loop gates on input.
        loop {
            inp -> batch() -> root_data;
            root_data = identity();
            // Nested loop: iterates via defer_tick.
            loop {
                merged = union() -> tee();
                root_data -> batch() -> merged;
                deferred -> merged;
                // Defer items < 100 back, multiplied by 10.
                merged -> filter(|&x| x < 100) -> map(|x| x * 10) -> defer_tick() -> deferred;
                deferred = identity();
                // Send output outside the inner loop.
                merged -> output;
            };
            output = all_iterations() -> for_each(|x| out_send.send(x).unwrap());
        };
    };

    // Send 1. Iterations: sees 1 (defers 10), sees 10 (defers 100), sees 100 (stops).
    // all_iterations should collect output from all 3 iterations.
    in_send.send(1).unwrap();
    df.run_tick_sync();
    let mut out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![1, 10, 100]);
}

/// Test batch_lazy in a nested loop: verify lazy data doesn't persist across
/// outer loop iterations or across ticks.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_batch_lazy_nested_no_stale_data() {
    let (trigger_send, trigger_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (lazy_send, lazy_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        trigger_inp = source_stream(trigger_recv);
        lazy_inp = source_stream(lazy_recv);
        // Root-level loop gates on trigger_inp.
        loop {
            trigger_inp -> batch() -> trigger_in_root;
            lazy_inp -> batch_lazy() -> lazy_in_root;
            trigger_in_root = identity();
            lazy_in_root = identity();
            // Middle loop: iterates via defer_tick (1 -> 10 -> 100, stops).
            loop {
                outer_merged = union() -> tee();
                trigger_in_root -> batch() -> outer_merged;
                lazy_in_root -> batch_lazy() -> lazy_in_outer;
                lazy_in_outer = identity();
                outer_deferred -> outer_merged;
                outer_merged
                    -> filter(|&x: &i32| x < 100)
                    -> map(|x: i32| x * 10)
                    -> defer_tick()
                    -> outer_deferred;
                outer_deferred = identity();

                // Inner loop: always fires when outer_merged has data (via batch).
                // Lazy data from outer also enters.
                loop {
                    inner_out = union();
                    outer_merged -> batch() -> inner_out;
                    lazy_in_outer -> batch_lazy() -> inner_out;
                    inner_out -> inner_results;
                };
                inner_results = all_iterations() -> outer_results;
            };
            outer_results = all_iterations() -> for_each(|x: i32| out_send.send(x).unwrap());
        };
    };

    // Tick 1: trigger=1, lazy=999.
    // Outer iteration 1: outer_merged=1, lazy_in_outer=999.
    //   Inner fires: sees 1 and 999.
    // Outer iteration 2: outer_merged=10, lazy_in_outer buffer re-created (empty).
    //   Inner fires: sees only 10 (no stale 999).
    // Outer iteration 3: outer_merged=100, lazy_in_outer buffer re-created (empty).
    //   Inner fires: sees only 100.
    trigger_send.send(1).unwrap();
    lazy_send.send(999).unwrap();
    df.run_tick_sync();
    let mut out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![1, 10, 100, 999]);

    // Tick 2: trigger=1, no lazy data.
    // No stale 999 from previous tick.
    trigger_send.send(1).unwrap();
    df.run_tick_sync();
    let mut out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![1, 10, 100]);
}

/// Test that a root-level loop emits `if` (fires at most once per tick, not `while`).
/// This is the key semantic difference from issue #2902: root loops are fused with the tick.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_root_loop_fires_once_per_tick() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        loop {
            inp -> batch() -> for_each(|x| out_send.send(x).unwrap());
        };
    };

    // Send data and run a tick. Loop should fire exactly once.
    in_send.send(1).unwrap();
    in_send.send(2).unwrap();
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, vec![1, 2]);

    // No data — loop should not fire.
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, Vec::<i32>::new());
}

/// Test that `defer_tick` works inside a root-level loop.
/// Data deferred in one tick should be available in the next tick's firing.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_root_loop_defer_tick() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        loop {
            merged = union() -> tee();
            inp -> batch() -> merged;
            deferred -> merged;
            merged -> for_each(|x| out_send.send(x).unwrap());
            // Defer items < 100 back for the next tick.
            merged -> filter(|&x| x < 100) -> map(|x| x * 10) -> defer_tick() -> deferred;
            deferred = identity();
        };
    };

    // Tick 1: Send 1. Sees 1, defers 10 to next tick.
    in_send.send(1).unwrap();
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, vec![1]);

    // Tick 2: Sees 10 (from defer_tick), defers 100 to next tick.
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, vec![10]);

    // Tick 3: Sees 100 (from defer_tick). 100 >= 100, so nothing deferred.
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, vec![100]);

    // Tick 4: Nothing. Loop should not fire.
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, Vec::<i32>::new());
}

/// Test that `defer_tick_lazy` works inside a root-level loop.
/// Data deferred lazily should be available when the loop fires for another reason
/// on the next tick, but should NOT cause the loop to fire on its own.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_root_loop_defer_tick_lazy() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        loop {
            merged = union() -> tee();
            inp -> batch() -> merged;
            deferred -> merged;
            merged -> for_each(|x| out_send.send(x).unwrap());
            // Lazy defer: stash data but don't trigger next tick.
            merged -> map(|x| x * 10) -> defer_tick_lazy() -> deferred;
            deferred = identity();
        };
    };

    // Tick 1: Send 1. Sees 1, lazily defers 10.
    in_send.send(1).unwrap();
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, vec![1]);

    // Tick 2: No new input; lazy defer does NOT trigger the loop.
    df.run_tick_sync();
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, Vec::<i32>::new());

    // Tick 3: Send 2. Loop fires; sees 2 and the lazily-deferred 10.
    in_send.send(2).unwrap();
    df.run_tick_sync();
    let mut out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![2, 10]);
}

/// Test that root-level loop independence is maintained.
/// Data arriving for loop A does not trigger loop B (even though both use `if`).
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_root_loop_independence() {
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

    // Loop A should fire, loop B should NOT fire (issue #2902 fix).
    assert_eq!(out_a, vec![42]);
    assert_eq!(out_b, Vec::<&str>::new());
}

/// Test that `defer_tick` works in a nested loop (iteration semantics).
/// Previously required `defer_tick()` but now `defer_tick()` adapts to context.
#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_nested_loop_defer_tick() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        loop {
            inp -> batch() -> root_data;
            root_data = identity();
            // Nested loop: defer_tick() acts as iteration-boundary here.
            loop {
                merged = union() -> tee();
                root_data -> batch() -> merged;
                deferred -> merged;
                merged -> for_each(|x| out_send.send(x).unwrap());
                merged -> filter(|&x| x < 100) -> map(|x| x * 10) -> defer_tick() -> deferred;
                deferred = identity();
            };
        };
    };

    in_send.send(1).unwrap();
    df.run_tick_sync();

    // Should iterate: 1 -> 10 -> 100 (stops at 100 because filter blocks >= 100).
    let out: Vec<i32> = dfir_rs::util::collect_ready(&mut out_recv);
    assert_eq!(out, vec![1, 10, 100]);
}
