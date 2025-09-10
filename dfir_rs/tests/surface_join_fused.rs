use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dfir_rs::lattices::set_union::SetUnionSingletonSet;
use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use lattices::Merge;
use lattices::set_union::SetUnionHashSet;
use multiplatform_test::multiplatform_test;

macro_rules! assert_contains_each_by_tick {
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
pub fn tick_tick_lhs_blocking_rhs_streaming() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)])
            -> map(|(k, v)| (k, SetUnionSingletonSet::new_from(v)))
            -> [0]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_join;

        my_join = join_fused_lhs(Fold(SetUnionHashSet::default, Merge::merge))
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_contains_each_by_tick!(
        results,
        TickInstant::new(0),
        &[(7, (SetUnionHashSet::new_from([1, 2]), 0))]
    );
}

#[multiplatform_test]
pub fn static_tick_lhs_blocking_rhs_streaming() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)])
            -> map(|(k, v)| (k, SetUnionSingletonSet::new_from(v)))
            -> [0]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_join;

        my_join = join_fused_lhs::<'static, 'tick>(Fold(SetUnionHashSet::default, Merge::merge))
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_contains_each_by_tick!(
        results,
        TickInstant::new(0),
        &[(7, (SetUnionHashSet::new_from([1, 2]), 0))]
    );
    assert_contains_each_by_tick!(
        results,
        TickInstant::new(1),
        &[(7, (SetUnionHashSet::new_from([1, 2]), 1))]
    );
    assert_contains_each_by_tick!(
        results,
        TickInstant::new(2),
        &[(7, (SetUnionHashSet::new_from([1, 2]), 2))]
    );
}

#[multiplatform_test]
pub fn static_static_lhs_blocking_rhs_streaming() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)])
            -> map(|(k, v)| (k, SetUnionSingletonSet::new_from(v)))
            -> [0]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_join;

        my_join = join_fused_lhs::<'static, 'static>(Fold(SetUnionHashSet::default, Merge::merge))
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    #[rustfmt::skip]
    {
        assert_contains_each_by_tick!(results, TickInstant::new(0), &[(7, (SetUnionHashSet::new_from([1, 2]), 0))]);
        assert_contains_each_by_tick!(results, TickInstant::new(1), &[(7, (SetUnionHashSet::new_from([1, 2]), 0)), (7, (SetUnionHashSet::new_from([1, 2]), 1))]);
        assert_contains_each_by_tick!(results, TickInstant::new(2), &[(7, (SetUnionHashSet::new_from([1, 2]), 0)), (7, (SetUnionHashSet::new_from([1, 2]), 1)), (7, (SetUnionHashSet::new_from([1, 2]), 2))]);
    };
}

#[multiplatform_test]
pub fn tick_tick_lhs_streaming_rhs_blocking() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)])
            -> map(|(k, v)| (k, SetUnionSingletonSet::new_from(v)))
            -> [1]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [0]my_join;

        my_join = join_fused_rhs(Fold(SetUnionHashSet::default, Merge::merge))
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_contains_each_by_tick!(
        results,
        TickInstant::new(0),
        &[(7, (0, (SetUnionHashSet::new_from([1, 2]))))]
    );
}

#[multiplatform_test]
pub fn static_tick_lhs_streaming_rhs_blocking() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)])
            -> map(|(k, v)| (k, SetUnionSingletonSet::new_from(v)))
            -> [1]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [0]my_join;

        my_join = join_fused_rhs::<'static, 'tick>(Fold(SetUnionHashSet::default, Merge::merge))
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_contains_each_by_tick!(
        results,
        TickInstant::new(0),
        &[(7, (0, SetUnionHashSet::new_from([1, 2])))]
    );
    assert_contains_each_by_tick!(
        results,
        TickInstant::new(1),
        &[(7, (1, SetUnionHashSet::new_from([1, 2])))]
    );
    assert_contains_each_by_tick!(
        results,
        TickInstant::new(2),
        &[(7, (2, SetUnionHashSet::new_from([1, 2])))]
    );
}

#[multiplatform_test]
pub fn static_static_lhs_streaming_rhs_blocking() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)])
            -> map(|(k, v)| (k, SetUnionSingletonSet::new_from(v)))
            -> [1]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [0]my_join;

        my_join = join_fused_rhs::<'static, 'static>(Fold(SetUnionHashSet::default, Merge::merge))
            -> inspect(|x| println!("{}, {x:?}", context.current_tick()))
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    #[rustfmt::skip]
    {
        assert_contains_each_by_tick!(results, TickInstant::new(0), &[(7, (0, SetUnionHashSet::new_from([1, 2])))]);
        assert_contains_each_by_tick!(results, TickInstant::new(1), &[(7, (0, SetUnionHashSet::new_from([1, 2]))), (7, (1, SetUnionHashSet::new_from([1, 2])))]);
        assert_contains_each_by_tick!(results, TickInstant::new(2), &[(7, (0, SetUnionHashSet::new_from([1, 2]))), (7, (1, SetUnionHashSet::new_from([1, 2]))), (7, (2, SetUnionHashSet::new_from([1, 2])))]);
    };
}

#[multiplatform_test]
pub fn tick_tick_lhs_fold_rhs_reduce() {
    let results = Rc::new(RefCell::new(HashMap::<TickInstant, Vec<_>>::new()));
    let results_inner = Rc::clone(&results);

    let mut df = dfir_syntax! {
        source_iter([(7, 1), (7, 2)])
            -> map(|(k, v)| (k, SetUnionSingletonSet::new_from(v)))
            -> [0]my_join;

        source_iter([(7, 0)]) -> unioner;
        source_iter([(7, 1)]) -> defer_tick() -> unioner;
        source_iter([(7, 2)]) -> defer_tick() -> defer_tick() -> unioner;
        unioner = union()
            -> [1]my_join;

        my_join = join_fused(Fold(SetUnionHashSet::default, Merge::merge), Reduce(std::ops::AddAssign::add_assign))
            -> for_each(|x| results_inner.borrow_mut().entry(context.current_tick()).or_default().push(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_contains_each_by_tick!(
        results,
        TickInstant::new(0),
        &[(7, (SetUnionHashSet::new_from([1, 2]), 0))]
    );
}
