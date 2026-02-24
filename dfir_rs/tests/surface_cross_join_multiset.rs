use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::util::{collect_ready, iter_batches_stream};
use multiplatform_test::multiplatform_test;

macro_rules! assert_contains_each_by_tick {
    ($results:expr, $tick:expr, &[]) => {{
        assert_eq!($results.borrow().get(&$tick), None);
    }};
    ($results:expr, $tick:expr, $input:expr) => {{
        for v in $input {
            assert!(
                $results.borrow()[&$tick].contains(v),
                "did not contain: {:?} in {:?}",
                v,
                $results.borrow()[&$tick]
            );
        }
    }};
}

#[multiplatform_test]
pub fn tick_tick() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([1, 2])
            -> [0]my_cross_join_multiset;

        source_iter([0]) -> unioner;
        source_iter([1]) -> defer_tick() -> unioner;
        source_iter([2]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_cross_join_multiset;

        my_cross_join_multiset = cross_join_multiset::<'tick, 'tick>()
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };

    df.run_available_sync();

    assert_contains_each_by_tick!(results, TickInstant::new(0), &[(1, 0), (2, 0)]);
    assert_contains_each_by_tick!(results, TickInstant::new(1), &[]);
}

#[multiplatform_test]
pub fn tick_static() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([1, 2])
            -> [0]my_cross_join_multiset;

        source_iter([0]) -> unioner;
        source_iter([1]) -> defer_tick() -> unioner;
        source_iter([2]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_cross_join_multiset;

        my_cross_join_multiset = cross_join_multiset::<'tick, 'static>()
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };

    df.run_available_sync();

    assert_contains_each_by_tick!(results, TickInstant::new(0), &[(1, 0), (2, 0)]);
    assert_contains_each_by_tick!(results, TickInstant::new(1), &[]);
}

#[multiplatform_test]
pub fn static_tick() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([1, 2])
            -> [0]my_cross_join_multiset;

        source_iter([0]) -> unioner;
        source_iter([1]) -> defer_tick() -> unioner;
        source_iter([2]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_cross_join_multiset;

        my_cross_join_multiset = cross_join_multiset::<'static, 'tick>()
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };

    df.run_available_sync();

    assert_contains_each_by_tick!(results, TickInstant::new(0), &[(1, 0), (2, 0)]);
    assert_contains_each_by_tick!(results, TickInstant::new(1), &[(1, 1), (2, 1)]);
    assert_contains_each_by_tick!(results, TickInstant::new(2), &[(1, 2), (2, 2)]);
    assert_contains_each_by_tick!(results, TickInstant::new(3), &[]);
}

#[multiplatform_test]
pub fn static_static() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([1, 2])
            -> [0]my_cross_join_multiset;

        source_iter([0]) -> unioner;
        source_iter([1]) -> defer_tick() -> unioner;
        source_iter([2]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_cross_join_multiset;

        my_cross_join_multiset = cross_join_multiset::<'static, 'static>()
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };

    df.run_available_sync();

    #[expect(
        clippy::disallowed_methods,
        reason = "Sorts all vecs regardless of nondeterministic iteration order."
    )]
    for result_vec in results.borrow_mut().values_mut() {
        result_vec.sort_unstable();
    }

    #[rustfmt::skip]
    {
        assert_contains_each_by_tick!(results, TickInstant::new(0), &[(1, 0), (2, 0)]);
        assert_contains_each_by_tick!(results, TickInstant::new(1), &[(1, 0), (2, 0), (1, 1), (2, 1)]);
        assert_contains_each_by_tick!(results, TickInstant::new(2), &[(1, 0), (2, 0), (1, 1), (2, 1), (1, 2), (2, 2)]);
        assert_contains_each_by_tick!(results, TickInstant::new(3), &[]);
    };
}

#[multiplatform_test]
pub fn replay_static() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([1, 2]) -> [0]my_cross_join_multiset;
        source_iter([3, 4]) -> [1]my_cross_join_multiset;
        my_cross_join_multiset = cross_join_multiset::<'static, 'static>()
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    df.run_tick_sync();
    df.run_tick_sync();
    df.run_tick_sync();

    #[expect(
        clippy::disallowed_methods,
        reason = "Sorts all vecs regardless of nondeterministic iteration order."
    )]
    for result_vec in results.borrow_mut().values_mut() {
        result_vec.sort_unstable();
    }

    #[rustfmt::skip]
    {
        assert_contains_each_by_tick!(results, TickInstant::new(0), &[(1, 3), (1, 4), (2, 3), (2, 4)]);
        assert_contains_each_by_tick!(results, TickInstant::new(1), &[(1, 3), (1, 4), (2, 3), (2, 4)]);
        assert_contains_each_by_tick!(results, TickInstant::new(2), &[(1, 3), (1, 4), (2, 3), (2, 4)]);
    };
}

#[multiplatform_test(test, wasm, env_tracing)]
pub fn loop_lifetimes() {
    let (result1_send, mut result1_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result2_send, mut result2_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result3_send, mut result3_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result4_send, mut result4_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        lb = source_stream(iter_batches_stream([1, 2, 3, 4], 2)) -> tee();
        rb = source_stream(iter_batches_stream([5, 6, 7, 8], 2)) -> tee();

        loop {
            lb -> batch() -> [0]cross_join_multiset1;
            rb -> batch() -> [1]cross_join_multiset1;
            cross_join_multiset1 = cross_join_multiset::<'loop, 'loop>()
                -> for_each(|x| result1_send.send((context.loop_iter_count(), x)).unwrap());

            lb -> batch() -> [0]cross_join_multiset2;
            rb -> batch() -> [1]cross_join_multiset2;
            cross_join_multiset2 = cross_join_multiset::<'loop, 'none>()
                -> for_each(|x| result2_send.send((context.loop_iter_count(), x)).unwrap());

            lb -> batch() -> [0]cross_join_multiset3;
            rb -> batch() -> [1]cross_join_multiset3;
            cross_join_multiset3 = cross_join_multiset::<'none, 'loop>()
                -> for_each(|x| result3_send.send((context.loop_iter_count(), x)).unwrap());

            lb -> batch() -> [0]cross_join_multiset4;
            rb -> batch() -> [1]cross_join_multiset4;
            cross_join_multiset4 = cross_join_multiset::<'none, 'none>()
                -> for_each(|x| result4_send.send((context.loop_iter_count(), x)).unwrap());
        };
    };

    df.run_available_sync();

    assert_eq!(
        BTreeSet::from_iter([
            (0, (1, 5)),
            (0, (1, 6)),
            (0, (2, 5)),
            (0, (2, 6)),
            (1, (1, 5)),
            (1, (1, 6)),
            (1, (1, 7)),
            (1, (1, 8)),
            (1, (2, 5)),
            (1, (2, 6)),
            (1, (2, 7)),
            (1, (2, 8)),
            (1, (3, 5)),
            (1, (3, 6)),
            (1, (3, 7)),
            (1, (3, 8)),
            (1, (4, 5)),
            (1, (4, 6)),
            (1, (4, 7)),
            (1, (4, 8)),
        ]),
        collect_ready(&mut result1_recv)
    );
    assert_eq!(
        BTreeSet::from_iter([
            (0, (1, 5)),
            (0, (1, 6)),
            (0, (2, 5)),
            (0, (2, 6)),
            (1, (1, 7)),
            (1, (1, 8)),
            (1, (2, 7)),
            (1, (2, 8)),
            (1, (3, 7)),
            (1, (3, 8)),
            (1, (4, 7)),
            (1, (4, 8)),
        ]),
        collect_ready(&mut result2_recv)
    );
    assert_eq!(
        BTreeSet::from_iter([
            (0, (1, 5)),
            (0, (1, 6)),
            (0, (2, 5)),
            (0, (2, 6)),
            (1, (3, 5)),
            (1, (3, 6)),
            (1, (3, 7)),
            (1, (3, 8)),
            (1, (4, 5)),
            (1, (4, 6)),
            (1, (4, 7)),
            (1, (4, 8)),
        ]),
        collect_ready(&mut result3_recv)
    );
    assert_eq!(
        BTreeSet::from_iter([
            (0, (1, 5)),
            (0, (1, 6)),
            (0, (2, 5)),
            (0, (2, 6)),
            (1, (3, 7)),
            (1, (3, 8)),
            (1, (4, 7)),
            (1, (4, 8)),
        ]),
        collect_ready(&mut result4_recv)
    );
}
