use dfir_rs::util::collect_ready;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_forwardref_basic_forward() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_iter(0..10) -> forward_ref;
        forward_ref = for_each(|v| out_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}

#[multiplatform_test]
pub fn test_forwardref_basic_backward() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        forward_ref -> for_each(|v| out_send.send(v).unwrap());
        forward_ref = source_iter(0..10);
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}

#[multiplatform_test]
pub fn test_forwardref_basic_middle() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_iter(0..10) -> forward_ref;
        forward_ref -> for_each(|v| out_send.send(v).unwrap());
        forward_ref = identity();
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}
