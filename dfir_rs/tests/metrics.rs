use dfir_rs::util::collect_ready_async;
use web_time::Duration;

/// Tests that everything is initially zero.
#[dfir_rs::test]
async fn test_initial() {
    let (output_send, _output_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let flow = dfir_rs::dfir_syntax_inline! {
        source_iter(0..5)
            -> map(|x| x * 2)
            -> for_each(|x| output_send.send(x).unwrap());
    };

    let metrics = flow.metrics();

    // Should have one subgraph
    assert_eq!(1, metrics.subgraphs.len());

    // Initial metrics should be zero
    for sg_id in metrics.subgraphs.keys() {
        let sg_metrics = &metrics.subgraphs[sg_id];
        assert_eq!(0, sg_metrics.total_run_count());
        assert_eq!(0, sg_metrics.total_poll_count());
        assert_eq!(0, sg_metrics.total_idle_count());
        assert_eq!(Duration::ZERO, sg_metrics.total_poll_duration());
        assert_eq!(Duration::ZERO, sg_metrics.total_idle_duration());
    }

    for handoff_id in metrics.handoffs.keys() {
        let handoff_metrics = &metrics.handoffs[handoff_id];
        assert_eq!(0, handoff_metrics.total_items_count());
    }
}

#[dfir_rs::test]
async fn test_subgraph_metrics() {
    let (output_send, _output_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut flow = dfir_rs::dfir_syntax_inline! {
        source_iter(0..3) -> for_each(|x| output_send.send(x).unwrap());
    };

    flow.run_tick().await;

    let metrics = flow.metrics();

    assert_eq!(1, metrics.subgraphs.len());
    let sg_id = metrics.subgraphs.keys().next().unwrap();

    let sg_metrics = &metrics.subgraphs[sg_id];
    assert_eq!(1, sg_metrics.total_run_count());
    assert!(0 < sg_metrics.total_poll_count());
}

#[dfir_rs::test]
async fn test_handoff_metrics() {
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut flow = dfir_rs::dfir_syntax_inline! {
        source_iter(0..5)
            -> map(|x| x * 2)
            -> fold(|| 0, |acc: &mut _, x| { *acc += x; })
            -> for_each(|x| { output_send.send(x).unwrap(); });
    };

    flow.run_available().await;

    let metrics = flow.metrics();

    assert_eq!(1, metrics.handoffs.len());
    let handoff_id = metrics.handoffs.keys().next().unwrap();

    let handoff_metrics = &metrics.handoffs[handoff_id];
    assert_eq!(5, handoff_metrics.total_items_count());

    let output: Vec<_> = collect_ready_async(&mut output_recv).await;
    assert_eq!(output, vec![20]);
}

#[dfir_rs::test]
async fn test_multiple_ticks() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut flow = dfir_rs::dfir_syntax_inline! {
        source_stream(input_recv)
            -> map(|x| x + 1)
            -> for_each(|x| output_send.send(x).unwrap());
    };

    input_send.send(1).unwrap();
    input_send.send(2).unwrap();
    flow.run_tick().await;

    let metrics = flow.metrics();
    assert_eq!(1, metrics.subgraphs.len());
    let sg_id = metrics.subgraphs.keys().next().unwrap();
    assert_eq!(1, metrics.subgraphs[sg_id].total_run_count());
    assert_eq!(1, flow.current_tick().0);

    input_send.send(3).unwrap();
    input_send.send(4).unwrap();
    flow.run_tick().await;

    let metrics = flow.metrics();
    assert_eq!(2, metrics.subgraphs[sg_id].total_run_count());
    assert_eq!(2, flow.current_tick().0);

    let output: Vec<_> = collect_ready_async(&mut output_recv).await;
    assert_eq!(output, vec![2, 3, 4, 5]);
}

#[dfir_rs::test]
async fn test_metrics_intervals() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut flow = dfir_rs::dfir_syntax_inline! {
        source_stream(input_recv)
            -> map(|x| x + 1)
            -> for_each(|x| output_send.send(x).unwrap());
    };
    let mut metrics_intervals = flow.metrics_intervals();

    // Zero at start
    let metrics = metrics_intervals.take_interval();
    assert_eq!(1, metrics.subgraphs.len());
    let sg_id = metrics.subgraphs.keys().next().unwrap();
    let sg_metrics = &metrics.subgraphs[sg_id];
    assert_eq!(0, sg_metrics.total_run_count());
    assert_eq!(0, sg_metrics.total_poll_count());
    assert_eq!(Duration::ZERO, sg_metrics.total_poll_duration());

    // Send some data and run first tick
    input_send.send(1).unwrap();
    input_send.send(2).unwrap();
    flow.run_tick().await;

    // After first tick, metrics should be updated
    let metrics = metrics_intervals.take_interval();
    let sg_metrics = &metrics.subgraphs[sg_id];
    assert_eq!(1, sg_metrics.total_run_count());
    assert_eq!(1, sg_metrics.total_poll_count());
    let poll_duration_1 = sg_metrics.total_poll_duration();

    // Send some more data
    for x in 0..10_000 {
        input_send.send(x).unwrap();
    }
    flow.run_tick().await;

    // After second tick, metrics updated
    let metrics = metrics_intervals.take_interval();
    let sg_metrics = &metrics.subgraphs[sg_id];
    assert_eq!(1, sg_metrics.total_run_count());
    assert_eq!(1, sg_metrics.total_poll_count());
    let poll_duration_2 = sg_metrics.total_poll_duration();

    // Total duration matches sum of intervals
    assert_eq!(
        poll_duration_1 + poll_duration_2,
        flow.metrics().subgraphs[sg_id].total_poll_duration()
    );

    let output: Vec<_> = collect_ready_async(&mut output_recv).await;
    assert_eq!(output[..10], vec![2, 3, 1, 2, 3, 4, 5, 6, 7, 8]);
}
