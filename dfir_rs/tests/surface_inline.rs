//! Tests for the experimental `dfir_syntax_inline!` macro.
//! This runs the dataflow inline using local Vec buffers instead of the Dfir scheduler.

/// Test 1: Simple linear pipeline
#[dfir_rs::test]
pub async fn test_inline_linear() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_iter(0..5_i32) -> map(|x: i32| x * 10) -> for_each(|v: i32| out.push(v));
    };
    tick().await;
    drop(tick);
    assert_eq!(vec![0, 10, 20, 30, 40], output);
}

/// Test 2: Fold (crosses stratum boundary, uses state API)
#[dfir_rs::test]
pub async fn test_inline_fold() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_iter(0..5_i32)
            -> fold(|| 0_i32, |acc: &mut i32, x: i32| *acc += x)
            -> for_each(|v: i32| out.push(v));
    };
    tick().await;
    drop(tick);
    assert_eq!(vec![10], output);
}

/// Test 3: Diamond DAG
#[dfir_rs::test]
pub async fn test_inline_diamond() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut tick = dfir_rs::dfir_syntax_inline! {
        my_tee = source_iter(1..=3_i32) -> tee();
        my_tee -> map(|x: i32| x * 10) -> my_union;
        my_tee -> map(|x: i32| x * 100) -> my_union;
        my_union = union() -> for_each(|v: i32| out.push(v));
    };
    tick().await;
    drop(tick);
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
    let mut tick = dfir_rs::dfir_syntax_inline! {
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
    tick().await;
    drop(tick);
    assert_eq!(vec![30], sums);
    assert_eq!(vec![7776], prods);
}

/// Test 5: Join
#[dfir_rs::test]
pub async fn test_inline_join() {
    let mut output = Vec::<(String, i32, i32)>::new();
    let out = &mut output;
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_iter(vec![("a", 1_i32), ("b", 2)]) -> [0]my_join;
        source_iter(vec![("b", 10_i32), ("a", 20)]) -> [1]my_join;
        my_join = join::<'tick, 'tick>()
            -> for_each(|(k, (v1, v2)): (&str, (i32, i32))| out.push((k.to_owned(), v1, v2)));
    };
    tick().await;
    drop(tick);
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
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_iter(1..=4_i32)
            -> fold(|| 0_i32, |a: &mut i32, x: i32| *a += x)
            -> map(|sum: i32| sum * 2)
            -> fold(|| 0_i32, |a: &mut i32, x: i32| *a += x)
            -> for_each(|v: i32| out.push(v));
    };
    tick().await;
    drop(tick);
    assert_eq!(vec![20], output);
}

/// Test 7: W-shape mesh
#[dfir_rs::test]
pub async fn test_inline_w_mesh() {
    let mut xs = Vec::<i32>::new();
    let mut ys = Vec::<i32>::new();
    let xr = &mut xs;
    let yr = &mut ys;
    let mut tick = dfir_rs::dfir_syntax_inline! {
        src_a = source_iter(vec![1_i32, 2]) -> tee();
        src_b = source_iter(vec![10_i32, 20]) -> tee();
        src_a -> sink_x;
        src_b -> sink_x;
        src_a -> sink_y;
        src_b -> sink_y;
        sink_x = union() -> for_each(|v: i32| xr.push(v));
        sink_y = union() -> for_each(|v: i32| yr.push(v));
    };
    tick().await;
    drop(tick);
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
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_stream(recv) -> for_each(|v: i32| out.push(v));
    };
    tick().await;
    drop(tick);
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
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_iter([rx])
            -> resolve_futures_blocking()
            -> map(|v: Result<i32, _>| v.unwrap())
            -> for_each(|v: i32| out.push(v));
    };
    tick().await;
    drop(tick);
    assert_eq!(vec![42], output);
}

/// Test 10: Multi-tick with source_stream — data arrives across ticks.
#[dfir_rs::test]
pub async fn test_inline_multi_tick_source_stream() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_stream(recv) -> for_each(|v: i32| out_send.send(v).unwrap());
    };

    send.send(1).unwrap();
    send.send(2).unwrap();
    tick().await;
    assert_eq!(
        &[1, 2],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(3).unwrap();
    tick().await;
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
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_stream(recv)
            -> fold::<'static>(|| 0_i32, |acc: &mut i32, x: i32| *acc += x)
            -> for_each(|v: i32| out_send.send(v).unwrap());
    };

    send.send(1).unwrap();
    tick().await;
    assert_eq!(
        &[1],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(2).unwrap();
    tick().await;
    assert_eq!(
        &[3],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(10).unwrap();
    tick().await;
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
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_stream(recv)
            -> fold::<'tick>(|| 0_i32, |acc: &mut i32, x: i32| *acc += x)
            -> for_each(|v: i32| out_send.send(v).unwrap());
    };

    send.send(1).unwrap();
    send.send(2).unwrap();
    tick().await;
    assert_eq!(
        &[3],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(10).unwrap();
    tick().await;
    assert_eq!(
        &[10],
        &*dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );
}

/// Test 13: defer_tick — data from tick N appears in tick N+1.
#[dfir_rs::test]
pub async fn test_inline_defer_tick() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_stream(recv) -> defer_tick() -> for_each(|v: i32| out_send.send(v).unwrap());
    };

    send.send(1).unwrap();
    send.send(2).unwrap();
    tick().await;
    // Tick 0: data is deferred, nothing comes out yet.
    assert_eq!(
        Vec::<i32>::new(),
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    send.send(3).unwrap();
    tick().await;
    // Tick 1: data from tick 0 appears. Data sent this tick is deferred.
    assert_eq!(
        vec![1, 2],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    tick().await;
    // Tick 2: data from tick 1 appears.
    assert_eq!(
        vec![3],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );
}

/// Test 14: defer_tick flip-flop — a cycle through defer_tick toggles a boolean.
#[dfir_rs::test]
pub async fn test_inline_defer_tick_flipflop() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<bool>();
    let mut tick = dfir_rs::dfir_syntax_inline! {
        source_iter(vec![true])
                -> state;
        state = union()
                -> inspect(|x: &bool| out_send.send(*x).unwrap())
                -> map(|x: bool| !x)
                -> defer_tick()
                -> state;
    };

    tick().await;
    assert_eq!(
        vec![true],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    tick().await;
    assert_eq!(
        vec![false],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    tick().await;
    assert_eq!(
        vec![true],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );

    tick().await;
    assert_eq!(
        vec![false],
        dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut out_recv).await
    );
}
