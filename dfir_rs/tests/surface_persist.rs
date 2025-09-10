use std::collections::HashSet;

use dfir_rs::compiled::pull::HalfMultisetJoinState;
use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::util::collect_ready;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_persist_basic() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut hf = dfir_syntax! {
        source_iter([1])
            -> persist::<'static>()
            -> persist::<'static>()
            -> fold(|| 0, |a: &mut _, b| *a += b)
            -> for_each(|x| result_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(hf);

    for tick in 0..10 {
        assert_eq!(TickInstant::new(tick), hf.current_tick());
        hf.run_tick_sync();
    }
    assert_eq!(
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_persist_pull() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut hf = dfir_syntax! {
        // Structured to ensure `persist::<'static>()` is pull-based.
        source_iter([1]) -> persist::<'static>() -> m0;
        null() -> m0;
        m0 = union() -> persist::<'static>() -> m1;
        null() -> m1;
        m1 = union()
            -> fold(|| 0, |a: &mut _, b| *a += b)
            -> for_each(|x| result_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(hf);

    for tick in 0..10 {
        assert_eq!(TickInstant::new(tick), hf.current_tick());
        hf.run_tick_sync();
    }
    assert_eq!(
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_persist_push() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut hf = dfir_syntax! {
        t0 = source_iter([1]) -> persist::<'static>() -> tee();
        t0 -> null();
        t1 = t0 -> persist::<'static>() -> tee();
        t1 -> null();
        t1 -> fold(|| 0, |a: &mut _, b| *a += b) -> for_each(|x| result_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(hf);

    for tick in 0..10 {
        assert_eq!(TickInstant::new(tick), hf.current_tick());
        hf.run_tick_sync();
    }
    assert_eq!(
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_persist_join() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<(&str, &str)>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter([("hello", "world")]) -> persist::<'static>() -> [0]my_join;
        source_stream(input_recv) -> persist::<'static>() -> [1]my_join;
        my_join = join::<'tick>() -> for_each(|(k, (v1, v2))| println!("({}, ({}, {}))", k, v1, v2));
    };
    input_send.send(("hello", "oakland")).unwrap();
    flow.run_tick_sync();
    input_send.send(("hello", "san francisco")).unwrap();
    flow.run_tick_sync();
}

#[multiplatform_test]
pub fn test_persist_replay_join() {
    let (persist_input_send, persist_input) = dfir_rs::util::unbounded_channel::<u32>();
    let (other_input_send, other_input) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<(u32, u32)>();

    let mut hf = dfir_syntax! {
        source_stream(persist_input)
            -> persist::<'static>()
            -> fold::<'tick>(|| 0, |a: &mut _, b| *a += b)
            -> next_stratum()
            -> [0]product_node;

        source_stream(other_input) -> [1] product_node;

        product_node = cross_join::<'tick, 'tick>() -> for_each(|x| result_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(hf);

    persist_input_send.send(1).unwrap();
    other_input_send.send(2).unwrap();
    hf.run_tick_sync();
    assert_eq!(&[(1, 2)], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    persist_input_send.send(2).unwrap();
    other_input_send.send(2).unwrap();
    hf.run_tick_sync();
    assert_eq!(&[(3, 2)], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    other_input_send.send(3).unwrap();
    hf.run_tick_sync();
    assert_eq!(&[(3, 3)], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[multiplatform_test]
pub fn test_persist_double_handoff() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (input_2_send, input_2_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();
    let mut flow = dfir_rs::dfir_syntax! {
        teed_first_sg = source_stream(input_2_recv) -> tee();
        teed_first_sg -> [0] joined_second_sg;
        teed_first_sg -> [1] joined_second_sg;

        source_stream(input_recv) -> persist::<'static>()
            -> inspect(|x| println!("LHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [0] cross;
        joined_second_sg = cross_join::<'tick, 'tick>() -> map(|t| t.0)
            -> inspect(|x| println!("RHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [1] cross;
        cross = cross_join::<'tick, 'tick, HalfMultisetJoinState>() -> for_each(|x| output_send.send(x).unwrap());
    };
    println!("A {}:{}", flow.current_tick(), flow.current_stratum());

    input_send.send(0).unwrap();
    flow.run_tick_sync();
    println!("B {}:{}", flow.current_tick(), flow.current_stratum());
    assert!(collect_ready::<Vec<_>, _>(&mut output_recv).is_empty());

    input_2_send.send(1).unwrap();
    flow.run_tick_sync();
    println!("C {}:{}", flow.current_tick(), flow.current_stratum());
    assert_eq!(&[(0, 1)], &*collect_ready::<Vec<_>, _>(&mut output_recv));
}

#[multiplatform_test]
pub fn test_persist_single_handoff() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (input_2_send, input_2_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();
    let mut flow = dfir_rs::dfir_syntax! {
        teed_first_sg = source_stream(input_2_recv) -> tee();
        teed_first_sg [0] -> null();
        teed_first_sg [1] -> joined_second_sg;
        null() -> joined_second_sg;

        source_stream(input_recv) -> persist::<'static>()
            -> inspect(|x| println!("LHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [0] cross;
        joined_second_sg = union()
            -> inspect(|x| println!("RHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [1] cross;
        cross = cross_join::<'tick, 'tick, HalfMultisetJoinState>() -> for_each(|x| output_send.send(x).unwrap());
    };
    println!("A {}:{}", flow.current_tick(), flow.current_stratum());

    input_send.send(0).unwrap();
    flow.run_tick_sync();
    println!("B {}:{}", flow.current_tick(), flow.current_stratum());
    assert!(collect_ready::<Vec<_>, _>(&mut output_recv).is_empty());

    input_2_send.send(1).unwrap();
    flow.run_tick_sync();
    println!("C {}:{}", flow.current_tick(), flow.current_stratum());
    assert_eq!(&[(0, 1)], &*collect_ready::<Vec<_>, _>(&mut output_recv));
}

#[multiplatform_test]
pub fn test_persist_single_subgraph() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (input_2_send, input_2_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();
    let mut flow = dfir_rs::dfir_syntax! {
        source_stream(input_2_recv) -> joined_second_sg;

        source_stream(input_recv) -> persist::<'static>()
            -> inspect(|x| println!("LHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [0] cross;
        joined_second_sg = inspect(|x| println!("RHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [1] cross;
        cross = cross_join::<'tick, 'tick, HalfMultisetJoinState>() -> for_each(|x| output_send.send(x).unwrap());
    };
    println!("A {}:{}", flow.current_tick(), flow.current_stratum());

    input_send.send(0).unwrap();
    flow.run_tick_sync();
    println!("B {}:{}", flow.current_tick(), flow.current_stratum());
    assert!(collect_ready::<Vec<_>, _>(&mut output_recv).is_empty());

    input_2_send.send(1).unwrap();
    flow.run_tick_sync();
    println!("C {}:{}", flow.current_tick(), flow.current_stratum());
    assert_eq!(&[(0, 1)], &*collect_ready::<Vec<_>, _>(&mut output_recv));
}

#[multiplatform_test]
pub fn test_persist() {
    let (pull_tx, mut pull_rx) = dfir_rs::util::unbounded_channel::<usize>();
    let (push_tx, mut push_rx) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {

        my_tee = source_iter([1, 2, 3])
            -> persist::<'static>() // pull
            -> tee();

        my_tee
            -> for_each(|v| pull_tx.send(v).unwrap());

        my_tee
            -> persist::<'static>() // push
            -> for_each(|v| push_tx.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(&[1, 2, 3], &*collect_ready::<Vec<_>, _>(&mut pull_rx));
    assert_eq!(&[1, 2, 3], &*collect_ready::<Vec<_>, _>(&mut push_rx));
}

#[multiplatform_test]
pub fn test_persist_mut() {
    use dfir_rs::util::Persistence::*;

    let (pull_tx, mut pull_rx) = dfir_rs::util::unbounded_channel::<usize>();
    let (push_tx, mut push_rx) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {

        my_tee = source_iter([Persist(1), Persist(2), Persist(3), Persist(4), Delete(2)])
            -> persist_mut::<'mutable>() // pull
            -> tee();

        my_tee
            -> for_each(|v| pull_tx.send(v).unwrap());

        my_tee
            -> flat_map(|x| if x == 3 {vec![Persist(x), Delete(x)]} else {vec![Persist(x)]})
            -> persist_mut::<'mutable>() // push
            -> for_each(|v| push_tx.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(&[1, 3, 4], &*collect_ready::<Vec<_>, _>(&mut pull_rx));
    assert_eq!(&[1, 4], &*collect_ready::<Vec<_>, _>(&mut push_rx));
}

#[multiplatform_test]
pub fn test_persist_mut_keyed() {
    use dfir_rs::util::PersistenceKeyed::*;

    let (pull_tx, mut pull_rx) = dfir_rs::util::unbounded_channel::<usize>();
    let (push_tx, mut push_rx) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {

        my_tee = source_iter([Persist(1, 1), Persist(2, 2), Persist(3, 3), Persist(4, 4), Delete(2)])
            -> persist_mut_keyed::<'mutable>() // pull
            -> tee();

        my_tee
            -> for_each(|(_k, v)| pull_tx.send(v).unwrap());

        my_tee
            -> flat_map(|(k, v)| if v == 3 {vec![Persist(k, v), Delete(k)]} else {vec![Persist(k, v)]})
            -> persist_mut_keyed::<'mutable>() // push
            -> for_each(|(_k, v)| push_tx.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(
        HashSet::from_iter([1, 3, 4]),
        collect_ready::<HashSet<_>, _>(&mut pull_rx)
    );
    assert_eq!(
        HashSet::from_iter([1, 4]),
        collect_ready::<HashSet<_>, _>(&mut push_rx)
    );
}
