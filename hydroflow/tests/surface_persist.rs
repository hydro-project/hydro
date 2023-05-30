use hydroflow::compiled::pull::HalfMultisetJoinState;
use hydroflow::util::collect_ready;
use hydroflow::{assert_graphvis_snapshots, hydroflow_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_persist_basic() {
    let (result_send, mut result_recv) = hydroflow::util::unbounded_channel::<u32>();

    let mut hf = hydroflow_syntax! {
        repeat_iter([1])
            -> persist()
            -> fold(0, |a, b| (a + b))
            -> for_each(|x| result_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(hf);

    for tick in 0..10 {
        assert_eq!(tick, hf.current_tick());
        hf.run_tick();
    }
    assert_eq!(
        &[1, 3, 6, 10, 15, 21, 28, 36, 45, 55],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_persist_pull() {
    let (result_send, mut result_recv) = hydroflow::util::unbounded_channel::<u32>();

    let mut hf = hydroflow_syntax! {
        // Structured to ensure `persist()` is pull-based.
        repeat_iter([1]) -> m0;
        null() -> m0;
        m0 = union() -> persist() -> m1;
        null() -> m1;
        m1 = union()
            -> fold(0, |a, b| (a + b))
            -> for_each(|x| result_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(hf);

    for tick in 0..10 {
        assert_eq!(tick, hf.current_tick());
        hf.run_tick();
    }
    assert_eq!(
        &[1, 3, 6, 10, 15, 21, 28, 36, 45, 55],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_persist_push() {
    let (result_send, mut result_recv) = hydroflow::util::unbounded_channel::<u32>();

    let mut hf = hydroflow_syntax! {
        t0 = repeat_iter([1]) -> tee();
        t0 -> null();
        t1 = t0 -> persist() -> tee();
        t1 -> null();
        t1 -> fold(0, |a, b| (a + b)) -> for_each(|x| result_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(hf);

    for tick in 0..10 {
        assert_eq!(tick, hf.current_tick());
        hf.run_tick();
    }
    assert_eq!(
        &[1, 3, 6, 10, 15, 21, 28, 36, 45, 55],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_persist_join() {
    let (input_send, input_recv) = hydroflow::util::unbounded_channel::<(&str, &str)>();
    let mut flow = hydroflow::hydroflow_syntax! {
        repeat_iter([("hello", "world")]) -> [0]my_join;
        source_stream(input_recv) -> persist() -> [1]my_join;
        my_join = join::<'tick>() -> for_each(|(k, (v1, v2))| println!("({}, ({}, {}))", k, v1, v2));
    };
    input_send.send(("hello", "oakland")).unwrap();
    flow.run_tick();
    input_send.send(("hello", "san francisco")).unwrap();
    flow.run_tick();
}

#[multiplatform_test]
pub fn test_persist_replay_join() {
    let (persist_input_send, persist_input) = hydroflow::util::unbounded_channel::<u32>();
    let (other_input_send, other_input) = hydroflow::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = hydroflow::util::unbounded_channel::<(u32, u32)>();

    let mut hf = hydroflow_syntax! {
        source_stream(persist_input)
            -> persist()
            -> fold::<'tick>(0, |a, b| (a + b))
            -> next_stratum()
            -> [0]product_node;

        source_stream(other_input) -> [1] product_node;

        product_node = cross_join::<'tick, 'tick>() -> for_each(|x| result_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(hf);

    persist_input_send.send(1).unwrap();
    other_input_send.send(2).unwrap();
    hf.run_tick();
    assert_eq!(&[(1, 2)], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    persist_input_send.send(2).unwrap();
    other_input_send.send(2).unwrap();
    hf.run_tick();
    assert_eq!(&[(3, 2)], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    other_input_send.send(3).unwrap();
    hf.run_tick();
    assert_eq!(&[(3, 3)], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[multiplatform_test]
pub fn test_persist_double_handoff() {
    let (input_send, input_recv) = hydroflow::util::unbounded_channel::<usize>();
    let (input_2_send, input_2_recv) = hydroflow::util::unbounded_channel::<usize>();
    let (output_send, mut output_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let mut flow = hydroflow::hydroflow_syntax! {
        teed_first_sg = source_stream(input_2_recv) -> tee();
        teed_first_sg -> [0] joined_second_sg;
        teed_first_sg -> [1] joined_second_sg;

        source_stream(input_recv) -> persist()
            -> inspect(|x| println!("LHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [0] cross;
        joined_second_sg = cross_join::<'tick, 'tick>() -> map(|t| t.0)
            -> inspect(|x| println!("RHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [1] cross;
        cross = cross_join::<'tick, 'tick, HalfMultisetJoinState>() -> for_each(|x| output_send.send(x).unwrap());
    };
    println!("A {}:{}", flow.current_tick(), flow.current_stratum());

    input_send.send(0).unwrap();
    flow.run_tick();
    println!("B {}:{}", flow.current_tick(), flow.current_stratum());
    assert!(collect_ready::<Vec<_>, _>(&mut output_recv).is_empty());

    input_2_send.send(1).unwrap();
    flow.run_tick();
    println!("C {}:{}", flow.current_tick(), flow.current_stratum());
    assert_eq!(&[(0, 1)], &*collect_ready::<Vec<_>, _>(&mut output_recv));
}

#[multiplatform_test]
pub fn test_persist_single_handoff() {
    let (input_send, input_recv) = hydroflow::util::unbounded_channel::<usize>();
    let (input_2_send, input_2_recv) = hydroflow::util::unbounded_channel::<usize>();
    let (output_send, mut output_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let mut flow = hydroflow::hydroflow_syntax! {
        teed_first_sg = source_stream(input_2_recv) -> tee();
        teed_first_sg [0] -> null();
        teed_first_sg [1] -> joined_second_sg;
        null() -> joined_second_sg;

        source_stream(input_recv) -> persist()
            -> inspect(|x| println!("LHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [0] cross;
        joined_second_sg = union()
            -> inspect(|x| println!("RHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [1] cross;
        cross = cross_join::<'tick, 'tick, HalfMultisetJoinState>() -> for_each(|x| output_send.send(x).unwrap());
    };
    println!("A {}:{}", flow.current_tick(), flow.current_stratum());

    input_send.send(0).unwrap();
    flow.run_tick();
    println!("B {}:{}", flow.current_tick(), flow.current_stratum());
    assert!(collect_ready::<Vec<_>, _>(&mut output_recv).is_empty());

    input_2_send.send(1).unwrap();
    flow.run_tick();
    println!("C {}:{}", flow.current_tick(), flow.current_stratum());
    assert_eq!(&[(0, 1)], &*collect_ready::<Vec<_>, _>(&mut output_recv));
}

#[multiplatform_test]
pub fn test_persist_single_subgraph() {
    let (input_send, input_recv) = hydroflow::util::unbounded_channel::<usize>();
    let (input_2_send, input_2_recv) = hydroflow::util::unbounded_channel::<usize>();
    let (output_send, mut output_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let mut flow = hydroflow::hydroflow_syntax! {
        source_stream(input_2_recv) -> joined_second_sg;

        source_stream(input_recv) -> persist()
            -> inspect(|x| println!("LHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [0] cross;
        joined_second_sg = inspect(|x| println!("RHS {} {}:{}", x, context.current_tick(), context.current_stratum())) -> [1] cross;
        cross = cross_join::<'tick, 'tick, HalfMultisetJoinState>() -> for_each(|x| output_send.send(x).unwrap());
    };
    println!("A {}:{}", flow.current_tick(), flow.current_stratum());

    input_send.send(0).unwrap();
    flow.run_tick();
    println!("B {}:{}", flow.current_tick(), flow.current_stratum());
    assert!(collect_ready::<Vec<_>, _>(&mut output_recv).is_empty());

    input_2_send.send(1).unwrap();
    flow.run_tick();
    println!("C {}:{}", flow.current_tick(), flow.current_stratum());
    assert_eq!(&[(0, 1)], &*collect_ready::<Vec<_>, _>(&mut output_recv));
}
