use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_flatten_stream_blocking() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        source_iter(vec![
            futures::stream::iter(vec![1, 2]),
            futures::stream::iter(vec![3]),
        ])
            -> flatten_stream_blocking()
            -> for_each(|x| result_send.send(x).unwrap());
    };
    df.run_available_sync();

    assert_eq!(&[1, 2, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[multiplatform_test]
pub fn test_flat_map_stream_blocking() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        source_iter(vec![1, 2, 3])
            -> flat_map_stream_blocking(|x| futures::stream::iter(vec![x, x * 10]))
            -> for_each(|x| result_send.send(x).unwrap());
    };
    df.run_available_sync();

    assert_eq!(
        &[1, 10, 2, 20, 3, 30],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_flat_map_stream_blocking_empty() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        source_iter(vec![1, 2, 3])
            -> flat_map_stream_blocking(|_| futures::stream::empty::<i32>())
            -> for_each(|x| result_send.send(x).unwrap());
    };
    df.run_available_sync();

    assert_eq!(
        Vec::<i32>::new(),
        collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_flatten_stream_blocking_empty() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df = dfir_syntax! {
        source_iter(vec![
            futures::stream::iter(Vec::<i32>::new()),
            futures::stream::iter(vec![1]),
            futures::stream::iter(Vec::<i32>::new()),
        ])
            -> flatten_stream_blocking()
            -> for_each(|x| result_send.send(x).unwrap());
    };
    df.run_available_sync();

    assert_eq!(&[1], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}
