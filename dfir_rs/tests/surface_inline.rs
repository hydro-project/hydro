//! Tests for the experimental `dfir_syntax_inline!` macro.
//! This runs the dataflow inline using local Vec buffers instead of the Dfir scheduler.

/// Test 1: Simple linear pipeline: source_iter -> map -> for_each
#[dfir_rs::test]
pub async fn test_inline_linear() {
    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        source_iter(0..5_i32) -> map(|x: i32| x * 10) -> for_each(|v: i32| output.borrow_mut().push(v));
    }.await;

    assert_eq!(&[0, 10, 20, 30, 40][..], &*output.borrow());
}

/// Test 2: Pipeline with fold (uses state API, crosses stratum boundary)
#[dfir_rs::test]
pub async fn test_inline_fold() {
    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        source_iter(0..5_i32)
            -> fold(|| 0_i32, |acc: &mut i32, x: i32| *acc += x)
            -> for_each(|v: i32| output.borrow_mut().push(v));
    }.await;

    assert_eq!(&[10][..], &*output.borrow());
}

/// Test 3: Diamond DAG: source -> tee -> (branch_a, branch_b) -> union -> for_each
#[dfir_rs::test]
pub async fn test_inline_diamond() {
    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        my_tee = source_iter(1..=3_i32) -> tee();
        my_tee -> map(|x: i32| x * 10) -> my_union;
        my_tee -> map(|x: i32| x * 100) -> my_union;
        my_union = union() -> for_each(|v: i32| output.borrow_mut().push(v));
    }.await;

    let mut result = output.borrow().clone();
    result.sort();
    assert_eq!(vec![10, 20, 30, 100, 200, 300], result);
}

/// Test 4: Intertwined diamonds — two diamonds sharing internal branches.
#[dfir_rs::test]
pub async fn test_inline_intertwined_diamonds() {
    let sums = std::cell::RefCell::new(Vec::new());
    let prods = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
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
            -> for_each(|v: i64| sums.borrow_mut().push(v));
        union_prod = union()
            -> fold(|| 1_i64, |a: &mut i64, x: i64| *a *= x)
            -> for_each(|v: i64| prods.borrow_mut().push(v));
    }.await;

    assert_eq!(&[30][..], &*sums.borrow());
    assert_eq!(&[7776][..], &*prods.borrow());
}

/// Test 5: Join — two independent sources joined by key.
#[dfir_rs::test]
pub async fn test_inline_join() {
    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        source_iter(vec![("a", 1_i32), ("b", 2)]) -> [0]my_join;
        source_iter(vec![("b", 10_i32), ("a", 20)]) -> [1]my_join;
        my_join = join::<'tick, 'tick>()
            -> for_each(|(k, (v1, v2)): (&str, (i32, i32))| output.borrow_mut().push((k.to_string(), v1, v2)));
    }.await;

    let mut result = output.borrow().clone();
    result.sort();
    assert_eq!(vec![("a".to_string(), 1, 20), ("b".to_string(), 2, 10)], result);
}

/// Test 6: Multi-stratum cascade — source → fold → map → fold → for_each
#[dfir_rs::test]
pub async fn test_inline_multi_stratum() {
    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        source_iter(1..=4_i32)
            -> fold(|| 0_i32, |a: &mut i32, x: i32| *a += x)
            -> map(|sum: i32| sum * 2)
            -> fold(|| 0_i32, |a: &mut i32, x: i32| *a += x)
            -> for_each(|v: i32| output.borrow_mut().push(v));
    }.await;

    assert_eq!(&[20][..], &*output.borrow());
}

/// Test 7: W-shape mesh — two sources each tee into two shared sinks.
#[dfir_rs::test]
pub async fn test_inline_w_mesh() {
    let xs = std::cell::RefCell::new(Vec::new());
    let ys = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        src_a = source_iter(vec![1_i32, 2]) -> tee();
        src_b = source_iter(vec![10_i32, 20]) -> tee();
        src_a -> sink_x;
        src_b -> sink_x;
        src_a -> sink_y;
        src_b -> sink_y;
        sink_x = union() -> for_each(|v: i32| xs.borrow_mut().push(v));
        sink_y = union() -> for_each(|v: i32| ys.borrow_mut().push(v));
    }.await;

    let mut rx = xs.borrow().clone();
    rx.sort();
    let mut ry = ys.borrow().clone();
    ry.sort();
    assert_eq!(vec![1, 2, 10, 20], rx);
    assert_eq!(vec![1, 2, 10, 20], ry);
}

/// Test 8: source_stream — async channel input.
#[dfir_rs::test]
pub async fn test_inline_source_stream() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    send.send(1).unwrap();
    send.send(2).unwrap();
    send.send(3).unwrap();

    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        source_stream(recv) -> for_each(|v: i32| output.borrow_mut().push(v));
    }.await;

    assert_eq!(&[1, 2, 3][..], &*output.borrow());
}

/// Test 9: resolve_futures — async futures resolved concurrently via tokio.
/// A spawned task sends a value through a oneshot after yielding, proving
/// the inline future actually suspends and resumes via the async runtime.
#[dfir_rs::test]
pub async fn test_inline_resolve_futures() {
    let output = std::cell::RefCell::new(Vec::new());

    let (tx, rx) = dfir_rs::tokio::sync::oneshot::channel::<i32>();

    // Spawn a task that sends after yielding — forces the inline future to suspend.
    dfir_rs::tokio::task::spawn_local(async move {
        dfir_rs::tokio::task::yield_now().await;
        tx.send(42).unwrap();
    });

    dfir_rs::dfir_syntax_inline_noemit! {
        source_iter([rx])
            -> resolve_futures_blocking()
            -> map(|v: Result<i32, _>| v.unwrap())
            -> for_each(|v: i32| output.borrow_mut().push(v));
    }.await;

    assert_eq!(&[42][..], &*output.borrow());
}
