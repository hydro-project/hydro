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
