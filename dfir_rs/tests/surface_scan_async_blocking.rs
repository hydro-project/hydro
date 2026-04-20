use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_scan_async_blocking_tick() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_rs::dfir_syntax_inline! {
        source_stream(items_recv)
            -> scan_async_blocking::<'tick>(|| 0, |acc: &mut u32, x: u32| {
                *acc += x;
                let val = *acc;
                async move { Some(val) }
            })
            -> for_each(|v| result_send.send(v).unwrap());
    };

    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick_sync();

    assert_eq!(&[1, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    // With 'tick' persistence, accumulator resets each tick
    items_send.send(3).unwrap();
    items_send.send(4).unwrap();
    df.run_tick_sync();

    assert_eq!(&[3, 7], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_scan_async_blocking_static() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_rs::dfir_syntax_inline! {
        source_stream(items_recv)
            -> scan_async_blocking::<'static>(|| 0, |acc: &mut u32, x: u32| {
                *acc += x;
                let val = *acc;
                async move { Some(val) }
            })
            -> for_each(|v| result_send.send(v).unwrap());
    };

    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick_sync();

    assert_eq!(&[1, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    // With 'static' persistence, accumulator persists across ticks
    items_send.send(3).unwrap();
    items_send.send(4).unwrap();
    df.run_tick_sync();

    assert_eq!(&[6, 10], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_scan_async_blocking_filter() {
    // Test that returning None from the future filters the item
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_rs::dfir_syntax_inline! {
        source_stream(items_recv)
            -> scan_async_blocking::<'tick>(|| 0, |acc: &mut u32, x: u32| {
                *acc += x;
                let val = *acc;
                async move {
                    if val.is_multiple_of(2) { None } else { Some(val) }
                }
            })
            -> for_each(|v| result_send.send(v).unwrap());
    };

    items_send.send(1).unwrap(); // acc=1, odd -> Some(1)
    items_send.send(1).unwrap(); // acc=2, even -> None
    items_send.send(1).unwrap(); // acc=3, odd -> Some(3)
    items_send.send(1).unwrap(); // acc=4, even -> None
    df.run_tick_sync();

    assert_eq!(&[1, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    df.run_available_sync();
}
