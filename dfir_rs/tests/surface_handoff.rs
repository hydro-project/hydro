//! Tests for the `handoff()` and `singleton()` pseudo-operators.

use dfir_rs::assert_graphvis_snapshots;

/// Test: `handoff()` pseudo-operator forces a subgraph boundary.
#[dfir_rs::test]
pub async fn test_handoff_basic() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(0..5_i32) -> handoff() -> for_each(|v: i32| out.push(v));
    };
    assert_graphvis_snapshots!(flow);
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![0, 1, 2, 3, 4], output);
}

/// Test: `handoff()` in the middle of a pipeline with transforms on both sides.
#[dfir_rs::test]
pub async fn test_handoff_mid_pipeline() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(0..5_i32)
            -> map(|x| x * 2)
            -> handoff()
            -> filter(|x: &i32| *x > 4)
            -> for_each(|v: i32| out.push(v));
    };
    assert_graphvis_snapshots!(flow);
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![6, 8], output);
}

/// Test: `singleton()` stores exactly one item and passes it through.
#[dfir_rs::test]
pub async fn test_singleton_basic() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter([42_i32]) -> singleton() -> for_each(|v: i32| out.push(v));
    };
    assert_graphvis_snapshots!(flow);
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![42], output);
}

/// Test: `singleton()` in a pipeline with transforms.
#[dfir_rs::test]
pub async fn test_singleton_with_fold() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(1..=5_i32)
            -> fold(|| 0_i32, |acc: &mut i32, x| *acc += x)
            -> singleton()
            -> map(|x: i32| x * 10)
            -> for_each(|v: i32| out.push(v));
    };
    assert_graphvis_snapshots!(flow);
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![150], output);
}

/// Test: `singleton()` panics if it receives more than one item.
#[dfir_rs::test]
#[should_panic(expected = "singleton() received more than one item")]
pub async fn test_singleton_panics_on_multiple_items() {
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter([1_i32, 2, 3]) -> singleton() -> for_each(|_| {});
    };
    flow.run_tick().await;
}

/// Test: `singleton()` w/ consumer across multiple ticks verifies the slot is drained each tick.
#[dfir_rs::test]
pub async fn test_singleton_multi_tick_consumed() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let output = std::rc::Rc::new(std::cell::RefCell::new(Vec::<i32>::new()));
    let out = output.clone();
    let mut flow = dfir_rs::dfir_syntax! {
        source_stream(recv)
            -> fold::<'static>(|| 0_i32, |acc: &mut i32, x| *acc += x)
            -> singleton()
            -> for_each(|v: i32| out.borrow_mut().push(v));
    };
    send.send(10).unwrap();
    flow.run_tick().await;
    assert_eq!(vec![10], *output.borrow());

    send.send(5).unwrap();
    flow.run_tick().await;
    assert_eq!(vec![10, 15], *output.borrow());

    // No new input: fold still emits its accumulated value.
    flow.run_tick().await;
    assert_eq!(vec![10, 15, 15], *output.borrow());
}

/// Test: `singleton()` w/o consumer across multiple ticks verifies the slot is drained each tick.
#[dfir_rs::test]
pub async fn test_singleton_multi_tick() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_stream(recv)
            -> fold::<'static>(|| 0_i32, |acc: &mut i32, x| *acc += x)
            -> singleton();
    };
    send.send(10).unwrap();
    flow.run_tick().await;
    send.send(5).unwrap();
    flow.run_tick().await;
    flow.run_tick().await;
}

/// Test: singleton() can be referenced via #var.
#[dfir_rs::test]
pub async fn test_singleton_reference() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        my_val = source_iter([42_i32]) -> singleton();
        my_val -> for_each(|_| {});
        source_iter(1..=3_i32) -> map(|x| x + #my_val) -> for_each(|v: i32| out.push(v));
    };
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![43, 44, 45], output);
}

/// Test: singleton() referenced via #var with no direct consumer (0 successors).
#[dfir_rs::test]
pub async fn test_singleton_reference_only() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        my_val = source_iter([42_i32]) -> singleton();
        source_iter(1..=3_i32) -> map(|x| x + #my_val) -> for_each(|v: i32| out.push(v));
    };
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![43, 44, 45], output);
}

/// Test: singleton() referenced via #var with no direct consumer (0 successors), multiple ticks.
#[dfir_rs::test]
pub async fn test_singleton_reference_only_multi_tick() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<i32>();
    let output = std::rc::Rc::new(std::cell::RefCell::new(Vec::<i32>::new()));
    let out = output.clone();
    let mut flow = dfir_rs::dfir_syntax! {
        my_val = source_iter([42_i32])
            -> persist::<'static>()
            -> singleton();
        source_stream(recv)
            -> map(|x| x + #my_val)
            -> for_each(|v: i32| out.borrow_mut().push(v));
    };
    send.send(1).unwrap();
    send.send(3).unwrap();
    flow.run_tick().await;
    assert_eq!(vec![43, 45], *output.borrow());

    send.send(100).unwrap();
    flow.run_tick().await;
    assert_eq!(vec![142], *output.borrow());
}
