//! Tests that blocking operators work on the push side of a subgraph.
//! These operators end up push-side when a tee (multi-output) is upstream in the same subgraph.

use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

/// sort on push side: source -> tee -> sort -> for_each
#[multiplatform_test]
pub fn test_sort_push() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut df = dfir_syntax! {
        my_tee = source_iter([3, 1, 2]) -> tee();
        my_tee -> sort() -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };
    df.run_available_sync();
    assert_eq!(&[1, 2, 3], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

/// reduce on push side: source -> tee -> reduce -> for_each
#[multiplatform_test]
pub fn test_reduce_push() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut df = dfir_syntax! {
        my_tee = source_iter([1, 2, 3]) -> tee();
        my_tee -> reduce(|a: &mut _, b| *a += b) -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };
    df.run_available_sync();
    assert_eq!(&[6], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}
