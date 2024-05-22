use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use hydroflow::scheduled::ticks::TickInstant;
use hydroflow::{assert_graphvis_snapshots, hydroflow_syntax};
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

    let mut df = hydroflow_syntax! {
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

    let mut df = hydroflow_syntax! {
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

    let mut df = hydroflow_syntax! {
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

    let mut df = hydroflow_syntax! {
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

    let mut df = hydroflow_syntax! {
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
