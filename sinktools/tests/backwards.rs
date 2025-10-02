//! Comprehensive unit tests for all sink adaptors using direct constructors.

use std::cell::RefCell;
use std::rc::Rc;

use sinktools::sink::SinkExt;
use sinktools::*;

/// Helper function to create a collecting sink using Rc<RefCell<Vec<T>>>
fn create_collecting_sink<T: Clone + 'static>() -> (
    impl Sink<T, Error = std::convert::Infallible>,
    Rc<RefCell<Vec<T>>>,
) {
    let collected = Rc::new(RefCell::new(Vec::new()));
    let collected_clone = collected.clone();

    let sink = for_each(move |item: T| {
        collected_clone.borrow_mut().push(item);
    });

    (sink, collected)
}

#[tokio::test]
async fn test_map_adaptor() {
    let (sink, collected) = create_collecting_sink();
    let mut map_sink = map(|x: i32| x * 2, sink);

    // Send some values through the map
    map_sink.send(1).await.unwrap();
    map_sink.send(2).await.unwrap();
    map_sink.send(3).await.unwrap();
    drop(map_sink);

    // Collect results
    assert_eq!(&[2, 4, 6], &**collected.borrow());
}

#[tokio::test]
async fn test_filter_adaptor() {
    let (sink, collected) = create_collecting_sink();
    let mut filter_sink = filter(|x: &i32| *x % 2 == 0, sink);

    // Send values, only evens should pass through
    filter_sink.send(1).await.unwrap();
    filter_sink.send(2).await.unwrap();
    filter_sink.send(3).await.unwrap();
    filter_sink.send(4).await.unwrap();
    filter_sink.send(5).await.unwrap();
    filter_sink.send(6).await.unwrap();
    drop(filter_sink);

    assert_eq!(&[2, 4, 6], &**collected.borrow());
}

#[tokio::test]
async fn test_filter_map_adaptor() {
    let (sink, collected) = create_collecting_sink();
    let mut filter_map_sink = filter_map(
        |x: i32| {
            if x % 2 == 0 { Some(x * 10) } else { None }
        },
        sink,
    );

    // Send values, only evens should pass through and be multiplied by 10
    filter_map_sink.send(1).await.unwrap();
    filter_map_sink.send(2).await.unwrap();
    filter_map_sink.send(3).await.unwrap();
    filter_map_sink.send(4).await.unwrap();
    filter_map_sink.send(5).await.unwrap();
    drop(filter_map_sink);

    assert_eq!(&[20, 40], &**collected.borrow());
}

#[tokio::test]
async fn test_inspect_adaptor() {
    let inspected = Rc::new(RefCell::new(Vec::new()));
    let inspected_clone = inspected.clone();

    let (sink, collected) = create_collecting_sink();
    let mut inspect_sink = inspect(
        move |x: &i32| {
            inspected_clone.borrow_mut().push(*x);
        },
        sink,
    );

    // Send values
    inspect_sink.send(1).await.unwrap();
    inspect_sink.send(2).await.unwrap();
    inspect_sink.send(3).await.unwrap();
    drop(inspect_sink);

    // Both inspected and collected should have the same values
    assert_eq!(&[1, 2, 3], &**inspected.borrow());
    assert_eq!(&[1, 2, 3], &**collected.borrow());
}

#[tokio::test]
async fn test_flat_map_adaptor() {
    let (sink, collected) = create_collecting_sink();
    let mut flat_map_sink = flat_map(|x: i32| vec![x, x + 10, x + 20], sink);

    // Send values, each should expand to 3 values
    flat_map_sink.send(1).await.unwrap();
    flat_map_sink.send(2).await.unwrap();
    drop(flat_map_sink);

    assert_eq!(&[1, 11, 21, 2, 12, 22], &**collected.borrow());
}

#[tokio::test]
async fn test_flatten_adaptor() {
    let (sink, collected) = create_collecting_sink();
    let mut flatten_sink = flatten::<Vec<i32>, _>(sink);

    // Send vectors that should be flattened
    flatten_sink.send(vec![1, 2, 3]).await.unwrap();
    flatten_sink.send(vec![4, 5]).await.unwrap();
    flatten_sink.send(vec![6]).await.unwrap();
    flatten_sink.send(vec![]).await.unwrap(); // Empty vector
    flatten_sink.send(vec![7, 8, 9]).await.unwrap();
    drop(flatten_sink);

    assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8, 9], &**collected.borrow());
}

#[tokio::test]
async fn test_for_each_adaptor() {
    let processed = Rc::new(RefCell::new(Vec::new()));
    let processed_clone = processed.clone();

    let mut for_each_sink = for_each(move |x: i32| {
        processed_clone.borrow_mut().push(x * 2);
    });

    // Send values
    for_each_sink.send(1).await.unwrap();
    for_each_sink.send(2).await.unwrap();
    for_each_sink.send(3).await.unwrap();
    drop(for_each_sink);

    assert_eq!(&[2, 4, 6], &**processed.borrow());
}

#[tokio::test]
async fn test_unzip_adaptor() {
    let (sink1, collected1) = create_collecting_sink();
    let (sink2, collected2) = create_collecting_sink();
    let mut unzip_sink = unzip(sink1, sink2);

    // Send tuples that should be unzipped
    unzip_sink.send((1, "a")).await.unwrap();
    unzip_sink.send((2, "b")).await.unwrap();
    unzip_sink.send((3, "c")).await.unwrap();
    drop(unzip_sink);

    assert_eq!(&[1, 2, 3], &**collected1.borrow());
    assert_eq!(&["a", "b", "c"], &**collected2.borrow());
}

#[tokio::test]
async fn test_send_iter() {
    let (sink, collected) = create_collecting_sink();

    let data = vec![1, 2, 3, 4, 5];
    let send_iter_future = send_iter(data, sink);

    send_iter_future.await.unwrap();

    assert_eq!(&[1, 2, 3, 4, 5], &**collected.borrow());
}

#[tokio::test]
async fn test_chained_adaptors() {
    let (sink, collected) = create_collecting_sink();

    // Chain multiple adaptors: filter -> inspect -> map -> sink
    // Building from inside out: sink <- map <- inspect <- filter
    let inspected = Rc::new(RefCell::new(Vec::new()));
    let inspected_clone = inspected.clone();

    let map_sink = map(|x: i32| x * 2, sink); // Last: Double the values that pass through
    let inspect_sink = inspect(
        move |x: &i32| {
            inspected_clone.borrow_mut().push(*x);
        },
        map_sink,
    ); // Second: Inspect values that pass the filter
    let mut chained_sink = filter(|x: &i32| *x > 2, inspect_sink); // First: Only values > 2

    // Send values
    chained_sink.send(1).await.unwrap(); // Filtered out (1 <= 2)
    chained_sink.send(2).await.unwrap(); // Filtered out (2 <= 2)
    chained_sink.send(3).await.unwrap(); // 3 > 2, inspected as 3, then doubled to 6
    chained_sink.send(4).await.unwrap(); // 4 > 2, inspected as 4, then doubled to 8
    drop(chained_sink);

    // Only values > 2 should be inspected (before doubling)
    assert_eq!(&[3, 4], &**inspected.borrow());
    // Those values should then be doubled and collected
    assert_eq!(&[6, 8], &**collected.borrow());
}

#[tokio::test]
async fn test_fanout() {
    let (sink1, collected1) = create_collecting_sink();
    let (sink2, collected2) = create_collecting_sink();

    let mut fanout_sink = SinkExt::fanout(sink1, sink2);

    // Send values that should go to both sinks
    fanout_sink.send(1).await.unwrap();
    fanout_sink.send(2).await.unwrap();
    fanout_sink.send(3).await.unwrap();
    drop(fanout_sink);

    // Both sinks should receive the same values
    assert_eq!(&[1, 2, 3], &**collected1.borrow());
    assert_eq!(&[1, 2, 3], &**collected2.borrow());
}

#[tokio::test]
async fn test_empty_inputs() {
    // Test adaptors with empty inputs
    let (sink, collected) = create_collecting_sink();

    // Test flatten with empty vector
    let mut flatten_sink = flatten::<Vec<i32>, _>(sink);
    flatten_sink.send(Vec::<i32>::new()).await.unwrap();
    drop(flatten_sink);

    assert_eq!(&[] as &[i32], &**collected.borrow());
}

#[tokio::test]
async fn test_error_handling() {
    // Test ForEach with error
    let mut error_sink = try_for_each(|x: i32| if x == 3 { Err("error on 3") } else { Ok(()) });

    // These should succeed
    error_sink.send(1).await.unwrap();
    error_sink.send(2).await.unwrap();

    // This should fail
    let result = error_sink.send(3).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_complex_chain() {
    let (sink, collected) = create_collecting_sink();

    // Complex chain: flatten -> filter_map -> map -> filter
    // Building from inside out: sink <- filter <- map <- filter_map <- flatten
    let filter_sink = filter(|x: &i32| *x < 100, sink); // Only values < 100
    let map_sink = map(|x: i32| x + 1, filter_sink); // Add 1
    let filter_map_sink = filter_map(
        |x: i32| {
            // Double evens, filter odds
            if x % 2 == 0 { Some(x * 2) } else { None }
        },
        map_sink,
    );
    let mut complex_sink = flatten::<Vec<i32>, _>(filter_map_sink); // Flatten input vectors

    // Send nested data
    complex_sink.send(vec![1, 2, 3, 4]).await.unwrap();
    complex_sink.send(vec![5, 6]).await.unwrap();
    complex_sink.send(vec![]).await.unwrap();
    complex_sink.send(vec![7, 8, 9]).await.unwrap();
    drop(complex_sink);

    // Processing: [1,2,3,4,5,6,7,8,9] -> filter_map(even after +1) -> [4,8,12,16] -> +1 -> [5,9,13,17] -> filter(<100) -> [5,9,13,17]
    // Wait, let me trace this more carefully:
    // Input: [1,2,3,4,5,6,7,8,9] (flattened)
    // filter_map: keep evens and double -> [4, 8, 12, 16] (2*2, 4*2, 6*2, 8*2)
    // map +1: [5, 9, 13, 17]
    // filter <100: [5, 9, 13, 17] (all pass)
    assert_eq!(&[5, 9, 13, 17], &**collected.borrow());
}

#[cfg(feature = "variadics")]
#[tokio::test]
async fn test_demux_var_adaptor() {
    let (sink1, collected1) = create_collecting_sink();
    let (sink2, collected2) = create_collecting_sink();
    let (sink3, collected3) = create_collecting_sink();

    let sinks = (sink1, (sink2, (sink3, ())));
    let mut demux_sink = demux_var(sinks);

    // Send indexed items to different sinks
    demux_sink.send((0, 10)).await.unwrap(); // Goes to sink1
    demux_sink.send((1, 20)).await.unwrap(); // Goes to sink2
    demux_sink.send((2, 30)).await.unwrap(); // Goes to sink3
    demux_sink.send((0, 11)).await.unwrap(); // Goes to sink1
    demux_sink.send((1, 21)).await.unwrap(); // Goes to sink2
    drop(demux_sink);

    assert_eq!(&[10, 11], &**collected1.borrow());
    assert_eq!(&[20, 21], &**collected2.borrow());
    assert_eq!(&[30], &**collected3.borrow());
}

#[cfg(feature = "variadics")]
#[tokio::test]
#[should_panic(expected = "index out of bounds")]
async fn test_demux_var_out_of_bounds() {
    let (sink1, _) = create_collecting_sink();
    let (sink2, _) = create_collecting_sink();

    let sinks = (sink1, (sink2, ()));
    let mut demux_sink = demux_var(sinks);

    // This should panic - index 2 is out of bounds for 2 sinks
    demux_sink.send((2, 10)).await.unwrap();
}

#[tokio::test]
async fn test_filter_all_filtered_out() {
    let (sink, collected) = create_collecting_sink();
    let mut filter_sink = filter(|_: &i32| false, sink); // Filter out everything

    filter_sink.send(1).await.unwrap();
    filter_sink.send(2).await.unwrap();
    filter_sink.send(3).await.unwrap();
    drop(filter_sink);

    assert_eq!(&[] as &[i32], &**collected.borrow());
}

#[tokio::test]
async fn test_filter_map_all_none() {
    let (sink, collected) = create_collecting_sink();
    let mut filter_map_sink = filter_map(|_: i32| None::<i32>, sink);

    filter_map_sink.send(1).await.unwrap();
    filter_map_sink.send(2).await.unwrap();
    filter_map_sink.send(3).await.unwrap();
    drop(filter_map_sink);

    assert_eq!(&[] as &[i32], &**collected.borrow());
}

#[tokio::test]
async fn test_flat_map_empty_iterators() {
    let (sink, collected) = create_collecting_sink();
    let mut flat_map_sink = flat_map(|_: i32| Vec::<i32>::new(), sink);

    flat_map_sink.send(1).await.unwrap();
    flat_map_sink.send(2).await.unwrap();
    flat_map_sink.send(3).await.unwrap();
    drop(flat_map_sink);

    assert_eq!(&[] as &[i32], &**collected.borrow());
}

#[tokio::test]
async fn test_send_iter_empty() {
    let (sink, collected) = create_collecting_sink();

    let data: Vec<i32> = vec![];
    let send_iter_future = send_iter(data, sink);

    send_iter_future.await.unwrap();

    assert_eq!(&[] as &[i32], &**collected.borrow());
}
