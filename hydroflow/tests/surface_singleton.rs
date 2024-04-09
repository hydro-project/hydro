use hydroflow::assert_graphvis_snapshots;
use hydroflow::util::collect_ready;
use lattices::Max;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_state_ref() {
    let (filter_send, mut filter_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (max_send, mut max_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut df = hydroflow::hydroflow_syntax! {
        stream1 = source_iter(1..=10);
        stream2 = source_iter_delta(3..=5) -> map(Max::new);
        max_of_stream2 = stream2 -> state_ref::<Max<_>>();

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

    df.run_available();

    assert_eq!(
        &[(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
    assert_eq!(
        &[(0, 3), (0, 4), (0, 5)],
        &*collect_ready::<Vec<_>, _>(&mut max_recv)
    );
}

/// Just tests that the codegen is valid.
#[multiplatform_test]
pub fn test_state_ref_unused() {
    let mut df = hydroflow::hydroflow_syntax! {
        stream2 = source_iter_delta(15..=25) -> map(Max::new);
        max_of_stream2 = stream2 -> state_ref::<Max<_>>();
    };

    assert_graphvis_snapshots!(df);

    df.run_available();
}

#[multiplatform_test]
pub fn test_fold_cross() {
    let (filter_send, mut filter_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (max_send, mut max_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut df = hydroflow::hydroflow_syntax! {
        stream1 = source_iter(1..=10);
        stream2 = source_iter_delta(3..=5) -> map(Max::new);
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

    df.run_available();

    assert_eq!(
        &[(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
    assert_eq!(&[(0, 5)], &*collect_ready::<Vec<_>, _>(&mut max_recv));
}

#[multiplatform_test]
pub fn test_fold_singleton() {
    let (filter_send, mut filter_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (max_send, mut max_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut df = hydroflow::hydroflow_syntax! {
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

    df.run_available();

    assert_eq!(
        &[(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
    assert_eq!(&[(0, 5)], &*collect_ready::<Vec<_>, _>(&mut max_recv));
}

#[multiplatform_test]
pub fn test_fold_singleton_push() {
    let (filter_send, mut filter_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut df = hydroflow::hydroflow_syntax! {
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
    };

    assert_graphvis_snapshots!(df);

    df.run_available();

    assert_eq!(
        &[(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)],
        &*collect_ready::<Vec<_>, _>(&mut filter_recv)
    );
}
