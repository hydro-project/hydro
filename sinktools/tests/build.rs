//! Comprehensive unit tests for all sink adaptors using forward [`SinkBuild`] pattern.

use std::cell::RefCell;
use std::rc::Rc;

use futures_util::sink::SinkExt;
use sinktools::*;

/// Helper function to create a collecting sink using Vec
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
async fn test_forward_map() {
    let (_sink, collected) = create_collecting_sink();
    let collected_clone = collected.clone();

    let mut final_sink = SinkBuilder::<i32>::new()
        .map(|x| x * 2)
        .for_each(move |item| {
            collected_clone.borrow_mut().push(item);
        });

    final_sink.send(1).await.unwrap();
    final_sink.send(2).await.unwrap();
    final_sink.send(3).await.unwrap();
    drop(final_sink);

    assert_eq!(&[2, 4, 6], &**collected.borrow());
}

#[tokio::test]
async fn test_forward_filter() {
    let (_sink, collected) = create_collecting_sink();
    let collected_clone = collected.clone();

    let mut final_sink = SinkBuilder::<i32>::new()
        .filter(|x| *x % 2 == 0)
        .for_each(move |item| {
            collected_clone.borrow_mut().push(item);
        });

    final_sink.send(1).await.unwrap();
    final_sink.send(2).await.unwrap();
    final_sink.send(3).await.unwrap();
    final_sink.send(4).await.unwrap();
    drop(final_sink);

    assert_eq!(&[2, 4], &**collected.borrow());
}

#[tokio::test]
async fn test_forward_filter_map() {
    let (_sink, collected) = create_collecting_sink();
    let collected_clone = collected.clone();

    let mut final_sink = SinkBuilder::<i32>::new()
        .filter_map(|x| if x % 2 == 0 { Some(x * 10) } else { None })
        .for_each(move |item| {
            collected_clone.borrow_mut().push(item);
        });

    final_sink.send(1).await.unwrap();
    final_sink.send(2).await.unwrap();
    final_sink.send(3).await.unwrap();
    final_sink.send(4).await.unwrap();
    drop(final_sink);

    assert_eq!(&[20, 40], &**collected.borrow());
}

#[tokio::test]
async fn test_forward_inspect() {
    let inspected = Rc::new(RefCell::new(Vec::new()));
    let inspected_clone = inspected.clone();
    let (_sink, collected) = create_collecting_sink();
    let collected_clone = collected.clone();

    let mut final_sink = SinkBuilder::<i32>::new()
        .inspect(move |x| {
            inspected_clone.borrow_mut().push(*x);
        })
        .for_each(move |item| {
            collected_clone.borrow_mut().push(item);
        });

    final_sink.send(1).await.unwrap();
    final_sink.send(2).await.unwrap();
    final_sink.send(3).await.unwrap();
    drop(final_sink);

    assert_eq!(&[1, 2, 3], &**inspected.borrow());
    assert_eq!(&[1, 2, 3], &**collected.borrow());
}

#[tokio::test]
async fn test_forward_flat_map() {
    let (_sink, collected) = create_collecting_sink();
    let collected_clone = collected.clone();

    let mut final_sink = SinkBuilder::<i32>::new()
        .flat_map(|x| vec![x, x + 10, x + 20])
        .for_each(move |item| {
            collected_clone.borrow_mut().push(item);
        });

    final_sink.send(1).await.unwrap();
    final_sink.send(2).await.unwrap();
    drop(final_sink);

    assert_eq!(&[1, 11, 21, 2, 12, 22], &**collected.borrow());
}

#[tokio::test]
async fn test_forward_flatten() {
    let (_sink, collected) = create_collecting_sink();
    let collected_clone = collected.clone();

    let mut final_sink = SinkBuilder::<Vec<i32>>::new()
        .flatten::<Vec<i32>>()
        .for_each(move |item| {
            collected_clone.borrow_mut().push(item);
        });

    final_sink.send(vec![1, 2, 3]).await.unwrap();
    final_sink.send(vec![4, 5]).await.unwrap();
    final_sink.send(vec![]).await.unwrap();
    final_sink.send(vec![6]).await.unwrap();
    drop(final_sink);

    assert_eq!(&[1, 2, 3, 4, 5, 6], &**collected.borrow());
}

#[tokio::test]
async fn test_forward_unzip() {
    let (sink1, collected1) = create_collecting_sink();
    let (sink2, collected2) = create_collecting_sink();

    let mut final_sink = SinkBuilder::<(i32, String)>::new()
        .map(|x: (i32, String)| (x.0 * 2, x.1.to_uppercase()))
        .unzip(sink1, sink2);

    final_sink.send((1, "a".to_string())).await.unwrap();
    final_sink.send((2, "b".to_string())).await.unwrap();
    final_sink.send((3, "c".to_string())).await.unwrap();
    drop(final_sink);

    assert_eq!(&[2, 4, 6], &**collected1.borrow());
    assert_eq!(&["A", "B", "C"], &**collected2.borrow());
}

#[tokio::test]
async fn test_forward_chaining() {
    let (_sink, collected) = create_collecting_sink();
    let collected_clone = collected.clone();

    let mut final_sink = SinkBuilder::<i32>::new()
        .filter(|x| *x > 0)
        .map(|x| x * 2)
        .filter_map(|x| if x < 10 { Some(x + 100) } else { None })
        .inspect(|x| println!("Processing: {}", x))
        .for_each(move |item| {
            collected_clone.borrow_mut().push(item);
        });

    final_sink.send(-1).await.unwrap(); // Filtered out by first filter
    final_sink.send(1).await.unwrap(); // 1 -> 2 -> 102
    final_sink.send(2).await.unwrap(); // 2 -> 4 -> 104
    final_sink.send(3).await.unwrap(); // 3 -> 6 -> 106
    final_sink.send(5).await.unwrap(); // 5 -> 10 -> filtered out by filter_map
    drop(final_sink);

    assert_eq!(&[102, 104, 106], &**collected.borrow());
}

#[tokio::test]
async fn test_forward_fanout() {
    let (sink1, collected1) = create_collecting_sink();
    let (sink2, collected2) = create_collecting_sink();

    let mut final_sink = SinkBuilder::<i32>::new()
        .map(|x| x * 2)
        .fanout(sink1, sink2);

    final_sink.send(1).await.unwrap();
    final_sink.send(2).await.unwrap();
    final_sink.send(3).await.unwrap();
    drop(final_sink);

    // Both sinks should receive the same mapped values
    assert_eq!(&[2, 4, 6], &**collected1.borrow());
    assert_eq!(&[2, 4, 6], &**collected2.borrow());
}

#[tokio::test]
async fn test_forward_complex_pipeline() {
    let (_sink, collected) = create_collecting_sink();
    let collected_clone = collected.clone();

    // Complex pipeline: flatten -> filter -> map -> filter_map -> inspect
    let mut final_sink = SinkBuilder::<Vec<i32>>::new()
        .flatten::<Vec<i32>>()                    // Flatten input vectors
        .filter(|x| *x > 0)                      // Keep positive numbers
        .map(|x| x * 2)                          // Double them
        .filter_map(|x| {                        // Keep only numbers < 20, add 100
            if x < 20 { Some(x + 100) } else { None }
        })
        .inspect(|x| println!("Final value: {}", x))
        .for_each(move |item| {
            collected_clone.borrow_mut().push(item);
        });

    final_sink.send(vec![-1, 1, 2]).await.unwrap(); // -1 filtered, 1->2->102, 2->4->104
    final_sink.send(vec![3, 15]).await.unwrap(); // 3->6->106, 15->30->filtered
    final_sink.send(vec![]).await.unwrap(); // Empty vector
    final_sink.send(vec![0, 5]).await.unwrap(); // 0 filtered, 5->10->110
    drop(final_sink);

    assert_eq!(&[102, 104, 106, 110], &**collected.borrow());
}

#[tokio::test]
async fn test_forward_vs_direct_equivalence() {
    // Test that forward and direct construction produce the same results
    let (_forward_sink, forward_collected) = create_collecting_sink();
    let (backward_sink, backward_collected) = create_collecting_sink();

    // Forward chaining
    let mut forward_pipeline = SinkBuilder::<i32>::new()
        .filter(|x| *x % 2 == 0)
        .map(|x| x * 3)
        .filter_map(|x| if x < 20 { Some(x + 1) } else { None })
        .for_each({
            let forward_collected = forward_collected.clone();
            move |item| {
                forward_collected.borrow_mut().push(item);
            }
        });

    // Direct construction (equivalent)
    let filter_map_sink = filter_map(
        |x: i32| if x < 20 { Some(x + 1) } else { None },
        backward_sink,
    );
    let map_sink = map(|x| x * 3, filter_map_sink);
    let mut backward_pipeline = filter(|x: &i32| *x % 2 == 0, map_sink);

    // Send the same data to both
    let test_data = vec![1, 2, 3, 4, 5, 6, 7, 8];

    for &item in &test_data {
        forward_pipeline.send(item).await.unwrap();
        backward_pipeline.send(item).await.unwrap();
    }

    drop(forward_pipeline);
    drop(backward_pipeline);

    // Both should produce the same results
    assert_eq!(
        &**forward_collected.borrow(),
        &**backward_collected.borrow()
    );
    assert_eq!(&[7, 13, 19], &**forward_collected.borrow()); // 2->6->7, 4->12->13, 6->18->19
}

#[cfg(feature = "variadics")]
#[tokio::test]
async fn test_forward_demux_var() {
    let (sink1, collected1) = create_collecting_sink();
    let (sink2, collected2) = create_collecting_sink();
    let (sink3, collected3) = create_collecting_sink();

    let sinks = (sink1, (sink2, (sink3, ())));
    let mut final_sink = SinkBuilder::<(usize, i32)>::new().demux_var(sinks);

    // Send indexed items to different sinks
    final_sink.send((0, 10)).await.unwrap(); // Goes to sink1
    final_sink.send((1, 20)).await.unwrap(); // Goes to sink2
    final_sink.send((2, 30)).await.unwrap(); // Goes to sink3
    final_sink.send((0, 11)).await.unwrap(); // Goes to sink1
    final_sink.send((1, 21)).await.unwrap(); // Goes to sink2
    drop(final_sink);

    assert_eq!(&[10, 11], &**collected1.borrow());
    assert_eq!(&[20, 21], &**collected2.borrow());
    assert_eq!(&[30], &**collected3.borrow());
}

#[cfg(feature = "variadics")]
#[tokio::test]
#[should_panic(expected = "index out of bounds")]
async fn test_forward_demux_var_out_of_bounds() {
    let (sink1, _) = create_collecting_sink();
    let (sink2, _) = create_collecting_sink();

    let sinks = (sink1, (sink2, ()));
    let mut final_sink = SinkBuilder::<(usize, i32)>::new().demux_var(sinks);

    // This should panic - index 2 is out of bounds for 2 sinks
    final_sink.send((2, 10)).await.unwrap();
}

#[tokio::test]
async fn test_forward_demux_map_basic() {
    let (sink1, collected1) = create_collecting_sink();
    let (sink2, collected2) = create_collecting_sink();
    let (sink3, collected3) = create_collecting_sink();

    let sinks = [("a", sink1), ("b", sink2), ("c", sink3)];
    let mut final_sink = SinkBuilder::<(&str, i32)>::new().demux_map(sinks);

    // Send items to different sinks based on keys
    final_sink.send(("a", 10)).await.unwrap();
    final_sink.send(("b", 20)).await.unwrap();
    final_sink.send(("c", 30)).await.unwrap();
    final_sink.send(("a", 11)).await.unwrap();
    final_sink.send(("b", 21)).await.unwrap();
    drop(final_sink);

    assert_eq!(&[10, 11], &**collected1.borrow());
    assert_eq!(&[20, 21], &**collected2.borrow());
    assert_eq!(&[30], &**collected3.borrow());
}

#[tokio::test]
#[should_panic(expected = "`DemuxMap` missing key")]
async fn test_forward_demux_map_missing_key() {
    let (sink, _) = create_collecting_sink();

    let sinks = [("existing", sink)];
    let mut final_sink = SinkBuilder::<(&str, i32)>::new().demux_map(sinks);

    // This should panic because "missing" key doesn't exist
    final_sink.send(("missing", 42)).await.unwrap();
}
