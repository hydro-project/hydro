//! Tests for the experimental `dfir_syntax_inline!` macro.
//! This runs the dataflow inline using local Vec buffers instead of the Dfir scheduler.

/// Test 1: Simple linear pipeline: source_iter -> map -> for_each
#[test]
pub fn test_inline_linear() {
    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        source_iter(0..5_i32) -> map(|x: i32| x * 10) -> for_each(|v: i32| output.borrow_mut().push(v));
    };

    assert_eq!(&[0, 10, 20, 30, 40][..], &*output.borrow());
}

/// Test 2: Pipeline with fold (uses state API, crosses stratum boundary):
/// source_iter -> fold -> for_each
#[test]
pub fn test_inline_fold() {
    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        source_iter(0..5_i32)
            -> fold(|| 0_i32, |acc: &mut i32, x: i32| *acc += x)
            -> for_each(|v: i32| output.borrow_mut().push(v));
    };

    assert_eq!(&[10][..], &*output.borrow());
}

/// Test 3: Diamond DAG: source -> tee -> (branch_a, branch_b) -> union -> for_each
#[test]
pub fn test_inline_diamond() {
    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        my_tee = source_iter(1..=3_i32) -> tee();
        my_tee -> map(|x: i32| x * 10) -> my_union;
        my_tee -> map(|x: i32| x * 100) -> my_union;
        my_union = union() -> for_each(|v: i32| output.borrow_mut().push(v));
    };

    let mut result = output.borrow().clone();
    result.sort();
    assert_eq!(vec![10, 20, 30, 100, 200, 300], result);
}

/// Test 4: Intertwined diamonds — two diamonds sharing internal branches.
///   src → tee → (*2 → tee_a, *3 → tee_b)
///   tee_a → union_sum, tee_b → union_sum   (diamond 1)
///   tee_a → union_prod, tee_b → union_prod  (diamond 2)
#[test]
pub fn test_inline_intertwined_diamonds() {
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
    };

    // *2 produces [2,4,6], *3 produces [3,6,9]
    // sum = 2+4+6+3+6+9 = 30
    assert_eq!(&[30][..], &*sums.borrow());
    // prod = 2*4*6*3*6*9 = 7776
    assert_eq!(&[7776][..], &*prods.borrow());
}

/// Test 5: Join — two independent sources joined by key.
#[test]
pub fn test_inline_join() {
    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        source_iter(vec![("a", 1_i32), ("b", 2)]) -> [0]my_join;
        source_iter(vec![("b", 10_i32), ("a", 20)]) -> [1]my_join;
        my_join = join::<'tick, 'tick>()
            -> for_each(|(k, (v1, v2)): (&str, (i32, i32))| output.borrow_mut().push((k.to_string(), v1, v2)));
    };

    let mut result = output.borrow().clone();
    result.sort();
    assert_eq!(vec![("a".to_string(), 1, 20), ("b".to_string(), 2, 10)], result);
}

/// Test 6: Multi-stratum cascade — source → fold → map → fold → for_each
/// Each fold creates a stratum boundary, so data flows through 3 strata.
#[test]
pub fn test_inline_multi_stratum() {
    let output = std::cell::RefCell::new(Vec::new());

    dfir_rs::dfir_syntax_inline_noemit! {
        source_iter(1..=4_i32)
            -> fold(|| 0_i32, |a: &mut i32, x: i32| *a += x)
            -> map(|sum: i32| sum * 2)
            -> fold(|| 0_i32, |a: &mut i32, x: i32| *a += x)
            -> for_each(|v: i32| output.borrow_mut().push(v));
    };

    // 1+2+3+4 = 10, *2 = 20, second fold sees just [20], so result = 20
    assert_eq!(&[20][..], &*output.borrow());
}

/// Test 7: W-shape mesh — two sources each tee into two shared sinks.
///   src_a → tee → (sink_x, sink_y)
///   src_b → tee → (sink_x, sink_y)
#[test]
pub fn test_inline_w_mesh() {
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
    };

    let mut rx = xs.borrow().clone();
    rx.sort();
    let mut ry = ys.borrow().clone();
    ry.sort();
    // Both sinks see all 4 items
    assert_eq!(vec![1, 2, 10, 20], rx);
    assert_eq!(vec![1, 2, 10, 20], ry);
}
