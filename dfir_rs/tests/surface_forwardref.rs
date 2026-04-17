use dfir_rs::dfir_syntax_inline;
use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_forwardref_basic_forward() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax_inline! {
        source_iter(0..10) -> forward_ref;
        forward_ref = for_each(|v| out_send.send(v).unwrap());
    };
    df.run_available_sync();

    assert_eq!(
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}

#[multiplatform_test]
pub fn test_forwardref_basic_backward() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax_inline! {
        forward_ref -> for_each(|v| out_send.send(v).unwrap());
        forward_ref = source_iter(0..10);
    };
    df.run_available_sync();

    assert_eq!(
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}

#[multiplatform_test]
pub fn test_forwardref_basic_middle() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax_inline! {
        source_iter(0..10) -> forward_ref;
        forward_ref -> for_each(|v| out_send.send(v).unwrap());
        forward_ref = identity();
    };
    df.run_available_sync();

    assert_eq!(
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}
