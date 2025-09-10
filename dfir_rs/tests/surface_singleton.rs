use dfir_rs::assert_graphvis_snapshots;
use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::util::collect_ready;
use lattices::Max;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_state() {
    let (filter_send, mut filter_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();
    let (max_send, mut max_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();

    let mut df = dfir_rs::dfir_syntax! {
        stream1 = source_iter(1..=10);
        stream2 = source_iter(3..=5) -> map(Max::new);
        max_of_stream2 = stream2 -> state::<'static, Max<_>>();

        filtered_stream1 = stream1
            -> persist::<'static>()
            -> filter(|value| {
                // This is not monotonic.
                value <= #max_of_stream2.as_reveal_ref()
            })
            -> map(|x| (context.current_tick(), x))
            -> for_each(|x| filter_send.send(x).unwrap());

        // Optional:
        max_of_stream2
            -> map(|x| (context.current_tick(), x.into_reveal()))
            -> for_each(|x| max_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);

    df.run_available_sync();

    assert_eq!(
        &[
            (TickInstant::new(0), 1),
            (TickInstant::new(0), 2),
            (TickInstant::new(0), 3),
            (TickInstant::new(0), 4),
            (TickInstant::new(0), 5)
        ],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
    assert_eq!(
        &[
            (TickInstant::new(0), 3),
            (TickInstant::new(0), 4),
            (TickInstant::new(0), 5)
        ],
        &*collect_ready::<Vec<_>, _>(&mut max_recv)
    );

    df.run_available_sync();

    assert_eq!(
        &[
            (TickInstant::new(1), 1),
            (TickInstant::new(1), 2),
            (TickInstant::new(1), 3),
            (TickInstant::new(1), 4),
            (TickInstant::new(1), 5)
        ],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
    assert_eq!(0, collect_ready::<Vec<_>, _>(&mut max_recv).len());
}

/// Just tests that the codegen is valid.
#[multiplatform_test]
pub fn test_state_unused() {
    let mut df = dfir_rs::dfir_syntax! {
        stream2 = source_iter(15..=25) -> map(Max::new);
        max_of_stream2 = stream2 -> state::<'static, Max<_>>();
    };

    assert_graphvis_snapshots!(df);

    df.run_available_sync();
}

/// Just tests that the codegen is valid.
#[multiplatform_test]
pub fn test_state_tick() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (max_send, mut max_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();
    let mut df = dfir_rs::dfir_syntax! {
        stream2 = source_stream(input_recv) -> map(Max::new);
        max_of_stream2 = stream2 -> state::<'tick, Max<_>>();

        max_of_stream2
            -> map(|x| (context.current_tick(), x.into_reveal()))
            -> for_each(|x| max_send.send(x).unwrap());
    };

    input_send.send(3).unwrap();
    input_send.send(4).unwrap();
    input_send.send(5).unwrap();
    df.run_tick_sync();

    input_send.send(2).unwrap();
    df.run_tick_sync();
    df.run_tick_sync();

    assert_eq!(
        &[
            (TickInstant::new(0), 3),
            (TickInstant::new(0), 4),
            (TickInstant::new(0), 5),
            (TickInstant::new(1), 2),
        ],
        &*collect_ready::<Vec<_>, _>(&mut max_recv)
    );
}

#[multiplatform_test]
pub fn test_fold_cross() {
    let (filter_send, mut filter_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();
    let (max_send, mut max_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();

    let mut df = dfir_rs::dfir_syntax! {
        stream1 = source_iter(1..=10);
        stream2 = source_iter(3..=5) -> map(Max::new);
        max_of_stream2 = stream2 -> lattice_reduce() -> tee();

        filtered_stream1 = stream1
            -> [0]filtered_stream2;
        max_of_stream2 -> identity::<Max<_>>() -> [1]filtered_stream2;

        filtered_stream2 = cross_join()
            -> filter(|(value, max_of_stream2)| {
                // This is not monotonic.
                value <= max_of_stream2.as_reveal_ref()
            })
            -> map(|(x, _max)| (context.current_tick(), x))
            -> for_each(|x| filter_send.send(x).unwrap());

        // Optional:
        max_of_stream2
            -> map(|x: Max<_>| (context.current_tick(), x.into_reveal()))
            -> for_each(|x| max_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);

    df.run_available_sync();

    assert_eq!(
        &[
            (TickInstant::new(0), 1),
            (TickInstant::new(0), 2),
            (TickInstant::new(0), 3),
            (TickInstant::new(0), 4),
            (TickInstant::new(0), 5)
        ],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
    assert_eq!(
        &[(TickInstant::new(0), 5)],
        &*collect_ready::<Vec<_>, _>(&mut max_recv)
    );
}

#[multiplatform_test]
pub fn test_fold_singleton() {
    let (filter_send, mut filter_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();
    let (max_send, mut max_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();

    let mut df = dfir_rs::dfir_syntax! {
        stream1 = source_iter(1..=10);
        stream2 = source_iter(3..=5);
        max_of_stream2 = stream2 -> fold(|| 0, |a, b| *a = std::cmp::max(*a, b));

        filtered_stream1 = stream1
            -> filter(|&value| {
                // This is not monotonic.
                value <= #max_of_stream2
            })
            -> map(|x| (context.current_tick(), x))
            -> for_each(|x| filter_send.send(x).unwrap());

        max_of_stream2
            -> map(|x| (context.current_tick(), x))
            -> for_each(|x| max_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);

    df.run_available_sync();

    assert_eq!(
        &[
            (TickInstant::new(0), 1),
            (TickInstant::new(0), 2),
            (TickInstant::new(0), 3),
            (TickInstant::new(0), 4),
            (TickInstant::new(0), 5)
        ],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
    assert_eq!(
        &[(TickInstant::new(0), 5)],
        &*collect_ready::<Vec<_>, _>(&mut max_recv)
    );
}

#[multiplatform_test]
pub fn test_fold_singleton_push() {
    let (filter_send, mut filter_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();

    let mut df = dfir_rs::dfir_syntax! {
        stream1 = source_iter(1..=10);
        stream2 = source_iter(3..=5);
        max_of_stream2 = stream2 -> fold(|| 0, |a, b| *a = std::cmp::max(*a, b));

        filtered_stream1 = stream1
            -> persist::<'static>()
            -> filter(|&value| {
                // This is not monotonic.
                value <= #max_of_stream2
            })
            -> map(|x| (context.current_tick(), x))
            -> for_each(|x| filter_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);

    df.run_available_sync();

    assert_eq!(
        &[
            (TickInstant::new(0), 1),
            (TickInstant::new(0), 2),
            (TickInstant::new(0), 3),
            (TickInstant::new(0), 4),
            (TickInstant::new(0), 5)
        ],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
}

#[multiplatform_test]
pub fn test_reduce_singleton() {
    let (filter_send, mut filter_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();
    let (max_send, mut max_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();

    let mut df = dfir_rs::dfir_syntax! {
        stream1 = source_iter(1..=10);
        stream2 = source_iter(3..=5);
        max_of_stream2 = stream2 -> reduce(|a, b| *a = std::cmp::max(*a, b));

        filtered_stream1 = stream1
            -> persist::<'static>()
            -> filter(|&value| {
                // This is not monotonic.
                value <= #max_of_stream2.unwrap_or(0)
            })
            -> map(|x| (context.current_tick(), x))
            -> for_each(|x| filter_send.send(x).unwrap());

        max_of_stream2
            -> map(|x| (context.current_tick(), x))
            -> for_each(|x| max_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);

    df.run_available_sync();

    assert_eq!(
        &[
            (TickInstant::new(0), 1),
            (TickInstant::new(0), 2),
            (TickInstant::new(0), 3),
            (TickInstant::new(0), 4),
            (TickInstant::new(0), 5)
        ],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
    assert_eq!(
        &[(TickInstant::new(0), 5)],
        &*collect_ready::<Vec<_>, _>(&mut max_recv)
    );
}

#[multiplatform_test]
pub fn test_reduce_singleton_push() {
    let (filter_send, mut filter_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();

    let mut df = dfir_rs::dfir_syntax! {
        stream1 = source_iter(1..=10);
        stream2 = source_iter(3..=5);
        max_of_stream2 = stream2 -> reduce(|a, b| *a = std::cmp::max(*a, b));

        filtered_stream1 = stream1
            -> persist::<'static>()
            -> filter(|&value| {
                // This is not monotonic.
                value <= #max_of_stream2.unwrap_or(0)
            })
            -> map(|x| (context.current_tick(), x))
            -> for_each(|x| filter_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);

    df.run_available_sync();

    assert_eq!(
        &[
            (TickInstant::new(0), 1),
            (TickInstant::new(0), 2),
            (TickInstant::new(0), 3),
            (TickInstant::new(0), 4),
            (TickInstant::new(0), 5)
        ],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
}

#[multiplatform_test]
pub fn test_scheduling() {
    let (inn_send, inn_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();

    let mut df = dfir_rs::dfir_syntax! {
        stream1 = source_iter(1..=10);
        stream2 = source_stream(inn_recv);
        max_of_stream2 = stream2 -> fold(|| 0, |a, b| *a = std::cmp::max(*a, b));

        filtered_stream1 = stream1
            -> persist::<'static>()
            -> filter(|&value| {
                // This is not monotonic.
                value <= #max_of_stream2
            })
            -> map(|x| (context.current_tick(), x))
            -> for_each(|x| out_send.send(x).unwrap());
    };

    for x in [1, 2, 3] {
        inn_send.send(x).unwrap();
    }
    df.run_available_sync();
    for x in [4, 5, 6] {
        inn_send.send(x).unwrap();
    }
    df.run_available_sync();

    assert_eq!(
        &[
            (TickInstant::new(0), 1),
            (TickInstant::new(0), 2),
            (TickInstant::new(0), 3),
            (TickInstant::new(1), 1),
            (TickInstant::new(1), 2),
            (TickInstant::new(1), 3),
            (TickInstant::new(1), 4),
            (TickInstant::new(1), 5),
            (TickInstant::new(1), 6),
        ],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}

#[multiplatform_test]
pub fn test_multi_tick() {
    let (filter_send, mut filter_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();
    let (max_send, mut max_recv) = dfir_rs::util::unbounded_channel::<(TickInstant, usize)>();

    let mut df = dfir_rs::dfir_syntax! {
        stream1 = source_iter(1..=10);
        stream2 = source_iter(3..=5) -> map(Max::new);
        max_of_stream2 = stream2 -> state::<'static, Max<_>>();

        filtered_stream1 = stream1
            -> filter(|value| {
                // This is not monotonic.
                value <= #max_of_stream2.as_reveal_ref()
            })
            -> map(|x| (context.current_tick(), x))
            -> for_each(|x| filter_send.send(x).unwrap());

        // Optional:
        max_of_stream2
            -> map(|x| (context.current_tick(), x.into_reveal()))
            -> for_each(|x| max_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);

    df.run_available_sync();
    assert_eq!(
        &[
            (TickInstant::new(0), 1),
            (TickInstant::new(0), 2),
            (TickInstant::new(0), 3),
            (TickInstant::new(0), 4),
            (TickInstant::new(0), 5)
        ],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
    assert_eq!(
        &[
            (TickInstant::new(0), 3),
            (TickInstant::new(0), 4),
            (TickInstant::new(0), 5)
        ],
        &*collect_ready::<Vec<_>, _>(&mut max_recv)
    );

    df.run_available_sync();
    assert_eq!(0, collect_ready::<Vec<_>, _>(&mut max_recv).len());
}
