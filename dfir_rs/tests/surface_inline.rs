//! Tests for the experimental `dfir_syntax!` macro.
//! This runs the dataflow inline using local Vec buffers instead of the Dfir scheduler.

/// Test 1: Simple linear pipeline
#[dfir_rs::test]
pub async fn test_inline_linear() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(0..5_i32) -> map(|x: i32| x * 10) -> for_each(|v: i32| out.push(v));
    };
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![0, 10, 20, 30, 40], output);
}

/// Test 2: Fold (crosses stratum boundary, uses state API)
#[dfir_rs::test]
pub async fn test_inline_fold() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(0..5_i32)
            -> fold(|| 0_i32, |acc: &mut i32, x: i32| *acc += x)
            -> for_each(|v: i32| out.push(v));
    };
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![10], output);
}

/// Test 3: Diamond DAG
#[dfir_rs::test]
pub async fn test_inline_diamond() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        my_tee = source_iter(1..=3_i32) -> tee();
        my_tee -> map(|x: i32| x * 10) -> my_union;
        my_tee -> map(|x: i32| x * 100) -> my_union;
        my_union = union() -> for_each(|v: i32| out.push(v));
    };
    flow.run_tick().await;
    drop(flow);
    output.sort();
    assert_eq!(vec![10, 20, 30, 100, 200, 300], output);
}

/// Test 4: Intertwined diamonds
#[dfir_rs::test]
pub async fn test_inline_intertwined_diamonds() {
    let mut sums = Vec::<i64>::new();
    let mut prods = Vec::<i64>::new();
    let s = &mut sums;
    let p = &mut prods;
    let mut flow = dfir_rs::dfir_syntax! {
        src = source_iter(1..=3_i64) -> tee();
        src -> map(|x: i64| x * 2) -> branch_a;
        src -> map(|x: i64| x * 3) -> branch_b;
        branch_a = tee();
        branch_b = tee();
        branch_a -> union_sum;
        branch_b -> union_sum;
        branch_a -> union_prod;
        branch_b -> union_prod;
        union_sum = union()
            -> fold(|| 0_i64, |a: &mut i64, x: i64| *a += x)
            -> for_each(|v: i64| s.push(v));
        union_prod = union()
            -> fold(|| 1_i64, |a: &mut i64, x: i64| *a *= x)
            -> for_each(|v: i64| p.push(v));
    };
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![30], sums);
    assert_eq!(vec![7776], prods);
}

/// Test 5: Join
#[dfir_rs::test]
pub async fn test_inline_join() {
    let mut output = Vec::<(String, i32, i32)>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(vec![("a", 1_i32), ("b", 2)]) -> [0]my_join;
        source_iter(vec![("b", 10_i32), ("a", 20)]) -> [1]my_join;
        my_join = join::<'tick, 'tick>()
            -> for_each(|(k, (v1, v2)): (&str, (i32, i32))| out.push((k.to_owned(), v1, v2)));
    };
    flow.run_tick().await;
    drop(flow);
    output.sort();
    assert_eq!(
        vec![("a".to_owned(), 1, 20), ("b".to_owned(), 2, 10)],
        output
    );
}

/// Test 6: Multi-stratum cascade
#[dfir_rs::test]
pub async fn test_inline_multi_stratum() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(1..=4_i32)
            -> fold(|| 0_i32, |a: &mut i32, x: i32| *a += x)
            -> map(|sum: i32| sum * 2)
            -> fold(|| 0_i32, |a: &mut i32, x: i32| *a += x)
            -> for_each(|v: i32| out.push(v));
    };
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![20], output);
}

/// Test 7: W-shape mesh
#[dfir_rs::test]
pub async fn test_inline_w_mesh() {
    let mut xs = Vec::<i32>::new();
    let mut ys = Vec::<i32>::new();
    let xr = &mut xs;
    let yr = &mut ys;
    let mut flow = dfir_rs::dfir_syntax! {
        src_a = source_iter(vec![1_i32, 2]) -> tee();
        src_b = source_iter(vec![10_i32, 20]) -> tee();
        src_a -> sink_x;
        src_b -> sink_x;
        src_a -> sink_y;
        src_b -> sink_y;
        sink_x = union() -> for_each(|v: i32| xr.push(v));
        sink_y = union() -> for_each(|v: i32| yr.push(v));
    };
    flow.run_tick().await;
    drop(flow);
    xs.sort();
    ys.sort();
    assert_eq!(vec![1, 2, 10, 20], xs);
    assert_eq!(vec![1, 2, 10, 20], ys);
}

/// Test 8: source_stream
#[dfir_rs::test]
pub async fn test_inline_source_stream() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    send.send(1).unwrap();
    send.send(2).unwrap();
    send.send(3).unwrap();
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_stream(recv) -> for_each(|v: i32| out.push(v));
    };
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![1, 2, 3], output);
}

/// Test 9: resolve_futures — proves real async suspension works.
#[dfir_rs::test]
pub async fn test_inline_resolve_futures() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let (tx, rx) = tokio::sync::oneshot::channel::<i32>();
    tokio::task::spawn_local(async move {
        tokio::task::yield_now().await;
        tx.send(42).unwrap();
    });
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter([rx])
            -> resolve_futures_blocking()
            -> map(|v: Result<i32, _>| v.unwrap())
            -> for_each(|v: i32| out.push(v));
    };
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![42], output);
}

/// Test 10: Multi-tick with source_stream — data arrives across ticks.
#[dfir_rs::test]
pub async fn test_inline_multi_tick_source_stream() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_stream(recv) -> for_each(|v: i32| out_send.send(v).unwrap());
    };

    send.send(1).unwrap();
    send.send(2).unwrap();
    flow.run_tick().await;
    assert_eq!(
        &[1, 2],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(3).unwrap();
    flow.run_tick().await;
    assert_eq!(
        &[3],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );
}

/// Test 11: Multi-tick with fold::<'static> — accumulator persists across ticks.
#[dfir_rs::test]
pub async fn test_inline_multi_tick_fold_static() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_stream(recv)
            -> fold::<'static>(|| 0_i32, |acc: &mut i32, x: i32| *acc += x)
            -> for_each(|v: i32| out_send.send(v).unwrap());
    };

    send.send(1).unwrap();
    flow.run_tick().await;
    assert_eq!(
        &[1],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(2).unwrap();
    flow.run_tick().await;
    assert_eq!(
        &[3],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(10).unwrap();
    flow.run_tick().await;
    assert_eq!(
        &[13],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );
}

/// Test 12: Multi-tick with fold::<'tick> — accumulator resets each tick.
#[dfir_rs::test]
pub async fn test_inline_multi_tick_fold_tick() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_stream(recv)
            -> fold::<'tick>(|| 0_i32, |acc: &mut i32, x: i32| *acc += x)
            -> for_each(|v: i32| out_send.send(v).unwrap());
    };

    send.send(1).unwrap();
    send.send(2).unwrap();
    flow.run_tick().await;
    assert_eq!(
        &[3],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(10).unwrap();
    flow.run_tick().await;
    assert_eq!(
        &[10],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );
}

/// Test 13: defer_tick_lazy — data from tick N appears in tick N+1.
#[dfir_rs::test]
pub async fn test_inline_defer_tick_lazy() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_stream(recv) -> defer_tick_lazy() -> for_each(|v: i32| out_send.send(v).unwrap());
    };

    send.send(1).unwrap();
    send.send(2).unwrap();
    flow.run_tick().await;
    // Tick 0: data is deferred, nothing comes out yet.
    assert_eq!(
        Vec::<i32>::new(),
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(3).unwrap();
    flow.run_tick().await;
    // Tick 1: data from tick 0 appears. Data sent this tick is deferred.
    assert_eq!(
        vec![1, 2],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    flow.run_tick().await;
    // Tick 2: data from tick 1 appears.
    assert_eq!(
        vec![3],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );
}

/// Test 14: defer_tick_lazy flip-flop — a cycle through defer_tick_lazy toggles a boolean.
#[dfir_rs::test]
pub async fn test_inline_defer_tick_flipflop() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<bool>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(vec![true])
                -> state;
        state = union()
                -> inspect(|x: &bool| out_send.send(*x).unwrap())
                -> map(|x: bool| !x)
                -> defer_tick_lazy()
                -> state;
    };

    flow.run_tick().await;
    assert_eq!(
        vec![true],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    flow.run_tick().await;
    assert_eq!(
        vec![false],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    flow.run_tick().await;
    assert_eq!(
        vec![true],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    flow.run_tick().await;
    assert_eq!(
        vec![false],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );
}

/// Test 15: cross_singleton — the pattern Hydro generates for combining streams with singletons.
/// This mimics what Hydro's `stream.cross_singleton(singleton)` compiles to.
#[dfir_rs::test]
pub async fn test_inline_cross_singleton() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(i32, i32)>();
    let mut flow = dfir_rs::dfir_syntax! {
        // A stream of values
        items = source_stream(recv);
        // A singleton: sum of a fixed set
        total = source_iter(1..=3_i32) -> fold::<'tick>(|| 0_i32, |a: &mut i32, x: i32| *a += x);
        // Cross the stream with the singleton
        cross = cross_singleton();
        items -> [input]cross;
        total -> [single]cross;
        cross -> for_each(|(item, total): (i32, i32)| out_send.send((item, total)).unwrap());
    };

    send.send(10).unwrap();
    send.send(20).unwrap();
    flow.run_tick().await;
    // Each stream item is paired with the singleton value (6 = 1+2+3)
    let mut result = dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await;
    result.sort();
    assert_eq!(vec![(10, 6), (20, 6)], result);
}

/// Test 16: Multi-tick Hydro-like pattern — source_stream → fold::<'static> → cross_singleton with
/// another stream, simulating a running total joined with incoming data.
#[dfir_rs::test]
pub async fn test_inline_hydro_pattern_multi_tick() {
    let (data_send, data_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (count_send, count_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(i32, i32)>();
    let mut flow = dfir_rs::dfir_syntax! {
        // Running count across ticks
        running_count = source_stream(count_recv)
            -> fold::<'static>(|| 0_i32, |a: &mut i32, x: i32| *a += x);
        // Data stream
        data = source_stream(data_recv);
        // Cross data with running count
        cross = cross_singleton();
        data -> [input]cross;
        running_count -> [single]cross;
        cross -> for_each(|(d, c): (i32, i32)| out_send.send((d, c)).unwrap());
    };

    // Tick 0: count=5, data=[100]
    count_send.send(5).unwrap();
    data_send.send(100).unwrap();
    flow.run_tick().await;
    assert_eq!(
        vec![(100, 5)],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    // Tick 1: count accumulates to 5+3=8, data=[200,300]
    count_send.send(3).unwrap();
    data_send.send(200).unwrap();
    data_send.send(300).unwrap();
    flow.run_tick().await;
    let mut result = dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await;
    result.sort();
    assert_eq!(vec![(200, 8), (300, 8)], result);
}

/// Regression test for https://github.com/hydro-project/hydro/issues/2747
/// defer_tick_lazy in a cycle must deliver data on the next tick, not two ticks later.
/// This was caused by missing topological sort of subgraphs within a stratum.
#[dfir_rs::test]
pub async fn test_inline_defer_tick_lazy_cycle() {
    let output = std::rc::Rc::new(std::cell::RefCell::new(Vec::<usize>::new()));
    let output_inner = std::rc::Rc::clone(&output);

    let mut flow = dfir_rs::dfir_syntax! {
        a = union() -> tee();
        source_iter([1_usize, 3]) -> [0]a;
        a[0] -> defer_tick_lazy() -> map(|x: usize| 2 * x) -> [1]a;
        a[1] -> for_each(|x: usize| output_inner.borrow_mut().push(x));
    };

    flow.run_tick().await;
    assert_eq!(vec![1, 3], output.take());

    flow.run_tick().await;
    assert_eq!(vec![2, 6], output.take());

    flow.run_tick().await;
    assert_eq!(vec![4, 12], output.take());
}

/// Test: defer_tick (non-lazy) — data from tick N appears in tick N+1.
#[dfir_rs::test]
pub async fn test_inline_defer_tick() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_stream(recv) -> defer_tick() -> for_each(|v: i32| out_send.send(v).unwrap());
    };

    send.send(1).unwrap();
    send.send(2).unwrap();
    flow.run_tick().await;
    // Tick 0: data is deferred, nothing comes out yet.
    assert_eq!(
        Vec::<i32>::new(),
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(3).unwrap();
    flow.run_tick().await;
    // Tick 1: data from tick 0 appears.
    assert_eq!(
        vec![1, 2],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    flow.run_tick().await;
    // Tick 2: data from tick 1 appears.
    assert_eq!(
        vec![3],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );
}

/// Test: defer_tick (non-lazy) flip-flop — a cycle through defer_tick toggles a boolean.
#[dfir_rs::test]
pub async fn test_inline_defer_tick_nonlazy_flipflop() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<bool>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(vec![true])
                -> state;
        state = union()
                -> inspect(|x: &bool| out_send.send(*x).unwrap())
                -> map(|x: bool| !x)
                -> defer_tick()
                -> state;
    };

    flow.run_tick().await;
    assert_eq!(
        vec![true],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    flow.run_tick().await;
    assert_eq!(
        vec![false],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    flow.run_tick().await;
    assert_eq!(
        vec![true],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );
}

/// Test: defer_tick (non-lazy) with run_available — run_available should continue
/// ticking as long as defer_tick buffers have data (the key difference from lazy).
#[dfir_rs::test]
pub async fn test_inline_defer_tick_run_available() {
    let output = std::rc::Rc::new(std::cell::RefCell::new(Vec::<usize>::new()));
    let output_inner = std::rc::Rc::clone(&output);

    let mut flow = dfir_rs::dfir_syntax! {
        a = union() -> tee();
        source_iter([1_usize, 3]) -> [0]a;
        a[0] -> defer_tick() -> map(|x: usize| 2 * x) -> filter(|&x: &usize| x < 20) -> [1]a;
        a[1] -> for_each(|x: usize| output_inner.borrow_mut().push(x));
    };

    // run_available should run multiple ticks until quiescence (defer buffers empty).
    flow.run_available().await;

    let result = output.take();
    // Tick 0: [1, 3]
    // Tick 1: [2, 6] (from defer of 1,3 -> *2)
    // Tick 2: [4, 12] (from defer of 2,6 -> *2)
    // Tick 3: [8] (from defer of 4,12 -> *2, 24 filtered out)
    // Tick 4: [16] (from defer of 8 -> *2)
    // Tick 5: [] (32 filtered out, no more data)
    assert_eq!(vec![1, 3, 2, 6, 4, 12, 8, 16], result);
}

/// Test: mutual defer_tick — two subgraphs each defer_tick to each other.
/// This tests that the topo sort in as_code handles mutual back-edges without
/// creating a cycle constraint.
#[test]
pub fn test_mutual_defer_tick() {
    let (tx, mut rx) = dfir_rs::util::unbounded_channel::<(usize, usize)>();

    let mut df = dfir_rs::dfir_syntax! {
        a = union() -> tee();
        b = union() -> tee();

        source_iter([(1usize, 0usize)]) -> [0]a;
        source_iter([(2usize, 0usize)]) -> [0]b;

        a[0] -> defer_tick() -> map(|(id, g): (usize, usize)| (id, g + 1)) -> [1]b;
        b[0] -> defer_tick() -> map(|(id, g): (usize, usize)| (id, g + 1)) -> [1]a;

        a[1] -> for_each(|x| tx.send(x).unwrap());
        b[1] -> for_each(|x| tx.send(x).unwrap());
    };

    df.run_tick_sync();
    let mut r = dfir_rs::util::collect_ready::<Vec<_>, _>(&mut rx);
    r.sort();
    // Tick 0: a gets (1,0), b gets (2,0)
    assert_eq!(r, vec![(1, 0), (2, 0)]);

    df.run_tick_sync();
    let mut r = dfir_rs::util::collect_ready::<Vec<_>, _>(&mut rx);
    r.sort();
    // Tick 1: a gets (2,1) from b's defer, b gets (1,1) from a's defer
    assert_eq!(r, vec![(1, 1), (2, 1)]);
}
