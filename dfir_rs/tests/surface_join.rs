use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::util::{collect_ready, iter_batches_stream};
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
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
        source_iter([(7, 1), (7, 2)])
            -> [0]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_join;

        my_join = join::<'tick, 'tick>()
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_contains_each_by_tick!(results, TickInstant::new(0), &[(7, (1, 0)), (7, (2, 0))]);
    assert_contains_each_by_tick!(results, TickInstant::new(1), &[]);
}

#[multiplatform_test]
pub fn tick_static() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)])
            -> [0]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_join;

        my_join = join::<'tick, 'static>()
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_contains_each_by_tick!(results, TickInstant::new(0), &[(7, (1, 0)), (7, (2, 0))]);
    assert_contains_each_by_tick!(results, TickInstant::new(1), &[]);
}

#[multiplatform_test]
pub fn static_tick() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)])
            -> [0]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_join;

        my_join = join::<'static, 'tick>()
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_contains_each_by_tick!(results, TickInstant::new(0), &[(7, (1, 0)), (7, (2, 0))]);
    assert_contains_each_by_tick!(results, TickInstant::new(1), &[(7, (1, 1)), (7, (2, 1))]);
    assert_contains_each_by_tick!(results, TickInstant::new(2), &[(7, (1, 2)), (7, (2, 2))]);
    assert_contains_each_by_tick!(results, TickInstant::new(3), &[]);
}

#[multiplatform_test]
pub fn static_static() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)])
            -> [0]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_join;

        my_join = join::<'static, 'static>()
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    #[rustfmt::skip]
    {
        assert_contains_each_by_tick!(results, TickInstant::new(0), &[(7, (1, 0)), (7, (2, 0))]);
        assert_contains_each_by_tick!(results, TickInstant::new(1), &[(7, (1, 0)), (7, (2, 0)), (7, (1, 1)), (7, (2, 1))]);
        assert_contains_each_by_tick!(results, TickInstant::new(2), &[(7, (1, 0)), (7, (2, 0)), (7, (1, 1)), (7, (2, 1)), (7, (1, 2)), (7, (2, 2))]);
        assert_contains_each_by_tick!(results, TickInstant::new(3), &[]);
    };
}

#[multiplatform_test]
pub fn replay_static() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)]) -> [0]my_join;
        source_iter([(7, 3), (7, 4)]) -> [1]my_join;
        my_join = join::<'static, 'static>()
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    df.run_tick();
    df.run_tick();
    df.run_tick();

    #[rustfmt::skip]
    {
        assert_contains_each_by_tick!(results, TickInstant::new(0), &[(7, (1, 3)), (7, (1, 4)), (7, (2, 3)), (7, (2, 4))]);
        assert_contains_each_by_tick!(results, TickInstant::new(1), &[(7, (1, 3)), (7, (1, 4)), (7, (2, 3)), (7, (2, 4))]);
        assert_contains_each_by_tick!(results, TickInstant::new(2), &[(7, (1, 3)), (7, (1, 4)), (7, (2, 3)), (7, (2, 4))]);
    };
}

#[multiplatform_test(test, wasm, env_tracing)]
pub fn loop_lifetimes() {
    let (result1_send, mut result1_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result2_send, mut result2_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result3_send, mut result3_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result4_send, mut result4_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        lb = source_stream(iter_batches_stream([
            (7, 1),
            (7, 2),
            (7, 3),
            (8, 4),
        ], 2)) -> tee();
        rb = source_stream(iter_batches_stream([
            (7, 5),
            (8, 6),
            (7, 7),
            (8, 8),
        ], 2)) -> tee();

        loop {
            lb -> batch() -> [0]join1;
            rb -> batch() -> [1]join1;
            join1 = join::<'loop, 'loop>()
                -> for_each(|x| result1_send.send((context.loop_iter_count(), x)).unwrap());

            lb -> batch() -> [0]join2;
            rb -> batch() -> [1]join2;
            join2 = join::<'loop, 'none>()
                -> for_each(|x| result2_send.send((context.loop_iter_count(), x)).unwrap());

            lb -> batch() -> [0]join3;
            rb -> batch() -> [1]join3;
            join3 = join::<'none, 'loop>()
                -> for_each(|x| result3_send.send((context.loop_iter_count(), x)).unwrap());

            lb -> batch() -> [0]join4;
            rb -> batch() -> [1]join4;
            join4 = join::<'none, 'none>()
                -> for_each(|x| result4_send.send((context.loop_iter_count(), x)).unwrap());
        };
    };
    assert_graphvis_snapshots!(df);

    df.run_available();

    assert_eq!(
        &[
            (0, (7, (1, 5))),
            (0, (7, (2, 5))),
            (1, (8, (4, 6))),
            (1, (8, (4, 8))),
            (1, (7, (1, 5))),
            (1, (7, (2, 5))),
            (1, (7, (3, 5))),
            (1, (7, (1, 7))),
            (1, (7, (2, 7))),
            (1, (7, (3, 7))),
        ],
        &*collect_ready::<Vec<_>, _>(&mut result1_recv)
    );
    assert_eq!(
        &[
            (0, (7, (1, 5))),
            (0, (7, (2, 5))),
            (1, (8, (4, 8))),
            (1, (7, (1, 7))),
            (1, (7, (2, 7))),
            (1, (7, (3, 7))),
        ],
        &*collect_ready::<Vec<_>, _>(&mut result2_recv)
    );
    assert_eq!(
        &[
            (0, (7, (1, 5))),
            (0, (7, (2, 5))),
            (1, (8, (4, 6))),
            (1, (8, (4, 8))),
            (1, (7, (3, 5))),
            (1, (7, (3, 7))),
        ],
        &*collect_ready::<Vec<_>, _>(&mut result3_recv)
    );
    assert_eq!(
        &[
            (0, (7, (1, 5))),
            (0, (7, (2, 5))),
            (1, (8, (4, 8))),
            (1, (7, (3, 7))),
        ],
        &*collect_ready::<Vec<_>, _>(&mut result4_recv)
    );
}
