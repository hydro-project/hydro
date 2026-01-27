use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::collect_ready_async;
use multiplatform_test::multiplatform_test;
use web_time::Duration;

/// Tests that everything is initially zero.
#[multiplatform_test(dfir)]
async fn test_initial() {
    let (output_send, _output_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let df: Dfir = dfir_syntax! {
        source_iter(0..5)
            -> map(|x| x * 2)
            -> for_each(|x| output_send.send(x).unwrap());
    };

    // Test that we can access metrics before running
    let metrics = df.metrics();

    println!(
        "Subgraph count: {}, Handoff count: {}",
        metrics.subgraphs.len(),
        metrics.handoffs.len()
    );

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

#[multiplatform_test(dfir)]
async fn test_subgraph_metrics() {
    let mut df: Dfir = dfir_syntax! {
        source_iter(0..3) -> for_each(|x| println!("Processing: {}", x));
    };

    // Run the dataflow
    df.run_available().await;

    let metrics = df.metrics();

    // After running, metrics should be updated
    assert_eq!(1, metrics.subgraphs.len());
    let sg_id = metrics.subgraphs.keys().next().unwrap();

    let sg_metrics = &metrics.subgraphs[sg_id];

    // Should have run once
    assert_eq!(1, sg_metrics.total_run_count());
    assert!(0 < sg_metrics.total_poll_count());

    // Poll duration should be non-zero (though might be very small)
    // We don't assert on exact duration as it depends on system performance

    println!(
        "Subgraph {:?}: runs={}, polls={}, poll_duration={:?}",
        sg_id,
        sg_metrics.total_run_count(),
        sg_metrics.total_poll_count(),
        sg_metrics.total_poll_duration(),
    );
}

#[multiplatform_test(dfir)]
async fn test_handoff_metrics() {
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df: Dfir = dfir_syntax! {
        source_iter(0..5)
            -> map(|x| x * 2)
            -> fold(|| 0, |acc, x| { *acc += x; })
            -> for_each(|x| { output_send.send(x).unwrap(); });
    };

    df.run_available().await;

    let metrics = df.metrics();

    assert_eq!(1, metrics.handoffs.len());
    let handoff_id = metrics.handoffs.keys().next().unwrap();

    let handoff_metrics = &metrics.handoffs[handoff_id];
    assert_eq!(5, handoff_metrics.total_items_count());

    // Verify output
    let output: Vec<_> = collect_ready_async(&mut output_recv).await;
    assert_eq!(output, vec![20]);
}

#[multiplatform_test(dfir)]
async fn test_multiple_ticks() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df: Dfir = dfir_syntax! {
        source_stream(input_recv)
            -> map(|x| x + 1)
            -> for_each(|x| output_send.send(x).unwrap());
    };

    // Send some data and run first tick
    input_send.send(1).unwrap();
    input_send.send(2).unwrap();
    df.run_tick().await;

    let metrics_after_tick1 = df.metrics();
    assert_eq!(1, metrics_after_tick1.subgraphs.len());
    let sg_id = metrics_after_tick1.subgraphs.keys().next().unwrap();

    let sg_metrics = &metrics_after_tick1.subgraphs[sg_id];
    assert_eq!(1, sg_metrics.total_run_count());
    assert_eq!(1, df.current_tick().0);

    // Send more data and run second tick
    input_send.send(3).unwrap();
    input_send.send(4).unwrap();
    df.run_tick().await;

    let metrics_after_tick2 = df.metrics();
    assert_eq!(2, metrics_after_tick2.subgraphs[sg_id].total_run_count());
    assert_eq!(2, df.current_tick().0);

    let output: Vec<_> = collect_ready_async(&mut output_recv).await;
    assert_eq!(output, vec![2, 3, 4, 5]);
}

#[multiplatform_test(dfir)]
async fn test_metrics_intervals() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (output_send, mut output_recv) = dfir_rs::util::unbounded_channel::<i32>();

    let mut df: Dfir = dfir_syntax! {
        source_stream(input_recv)
            -> map(|x| x + 1)
            -> for_each(|x| output_send.send(x).unwrap());
    };
    let mut metrics_intervals = df.metrics_intervals();

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
    df.run_tick().await;

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
    df.run_tick().await;

    // After second tick, metrics updated
    let metrics = metrics_intervals.take_interval();
    let sg_metrics = &metrics.subgraphs[sg_id];
    assert_eq!(1, sg_metrics.total_run_count()); // Still 1 (per tick)
    assert_eq!(1, sg_metrics.total_poll_count());
    let poll_duration_2 = sg_metrics.total_poll_duration();

    // Total duration matches sum of intervals
    assert_eq!(
        poll_duration_1 + poll_duration_2,
        df.metrics().subgraphs[sg_id].total_poll_duration()
    );

    let output: Vec<_> = collect_ready_async(&mut output_recv).await;
    assert_eq!(output[..10], vec![2, 3, 1, 2, 3, 4, 5, 6, 7, 8]);
}
