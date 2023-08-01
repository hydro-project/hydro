use hydroflow::util::collect_ready;
use hydroflow_macro::hydroflow_syntax;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_lattice_join_reducing_behavior() {
    // lattice_fold has the following particular initialization behavior. It uses LatticeFrom to convert the first element into another lattice type to create the accumulator and then it folds the remaining elements into that accumulator.
    // This initialization style supports things like SetUnionSingletonSet -> SetUnionHashSet. It also helps with things like Min, where there is not obvious default value so you want reduce-like behavior.
    let mut df = hydroflow_syntax! {
        use lattices::Min;
        use lattices::Max;

        source_iter([(7, Min::new(5)), (7, Min::new(6))]) -> [0]my_join;
        source_iter([(7, Max::new(5)), (7, Max::new(6))]) -> [1]my_join;

        my_join = lattice_join::<Min<usize>, Max<usize>>()
            -> assert_eq([(7, (Min::new(5), Max::new(6)))]);
    };

    df.run_available();
}

#[multiplatform_test]
pub fn test_lattice_join_set_union() {
    let mut df = hydroflow_syntax! {
        use lattices::set_union::SetUnionSingletonSet;
        use lattices::set_union::SetUnionHashSet;

        source_iter([(7, SetUnionHashSet::new_from([5])), (7, SetUnionHashSet::new_from([6]))]) -> [0]my_join;
        source_iter([(7, SetUnionSingletonSet::new_from(5)), (7, SetUnionSingletonSet::new_from(6))]) -> [1]my_join;

        my_join = lattice_join::<SetUnionHashSet<usize>, SetUnionHashSet<usize>>()
            -> assert_eq([(7, (SetUnionHashSet::new_from([5, 6]), SetUnionHashSet::new_from([5, 6])))]);
    };

    df.run_available();
}

#[multiplatform_test]
pub fn test_lattice_join_map_union() {
    let mut df = hydroflow_syntax! {
        use lattices::map_union::MapUnionSingletonMap;
        use lattices::map_union::MapUnionHashMap;
        use hydroflow::lattices::Min;

        source_iter([(7, MapUnionHashMap::new_from([(3, Min::new(4))])), (7, MapUnionHashMap::new_from([(3, Min::new(5))]))]) -> [0]my_join;
        source_iter([(7, MapUnionSingletonMap::new_from((3, Min::new(5)))), (7, MapUnionSingletonMap::new_from((3, Min::new(4))))]) -> [1]my_join;

        my_join = lattice_join::<MapUnionHashMap<usize, Min<usize>>, MapUnionHashMap<usize, Min<usize>>>()
            -> assert_eq([(7, (MapUnionHashMap::new_from([(3, Min::new(4))]), MapUnionHashMap::new_from([(3, Min::new(4))])))]);
    };

    df.run_available();
}

#[multiplatform_test]
pub fn test_lattice_join() {
    use hydroflow::lattices::Max;

    // 'static, 'tick.
    {
        let (out_tx, mut out_rx) =
            hydroflow::util::unbounded_channel::<(usize, (Max<usize>, Max<usize>))>();
        let (lhs_tx, lhs_rx) = hydroflow::util::unbounded_channel::<(usize, Max<usize>)>();
        let (rhs_tx, rhs_rx) = hydroflow::util::unbounded_channel::<(usize, Max<usize>)>();

        let mut df = hydroflow_syntax! {
            my_join = lattice_join::<'static, 'tick, Max<usize>, Max<usize>>();

            source_stream(lhs_rx) -> [0]my_join;
            source_stream(rhs_rx) -> [1]my_join;

            my_join -> for_each(|v| out_tx.send(v).unwrap());
        };

        // Merges forward correctly, without going backward
        lhs_tx.send((7, Max::new(3))).unwrap();
        lhs_tx.send((7, Max::new(4))).unwrap();
        rhs_tx.send((7, Max::new(6))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, vec![(7, (Max::new(4), Max::new(6)))]);

        // Forgets rhs state
        rhs_tx.send((7, Max::new(6))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, vec![(7, (Max::new(4), Max::new(6)))]);
    }

    // 'static, 'static.
    {
        let (out_tx, mut out_rx) =
            hydroflow::util::unbounded_channel::<(usize, (Max<usize>, Max<usize>))>();
        let (lhs_tx, lhs_rx) = hydroflow::util::unbounded_channel::<(usize, Max<usize>)>();
        let (rhs_tx, rhs_rx) = hydroflow::util::unbounded_channel::<(usize, Max<usize>)>();

        let mut df = hydroflow_syntax! {
            my_join = lattice_join::<'static, 'static, Max<usize>, Max<usize>>();

            source_stream(lhs_rx) -> [0]my_join;
            source_stream(rhs_rx) -> [1]my_join;

            my_join -> for_each(|v| out_tx.send(v).unwrap());
        };

        lhs_tx.send((7, Max::new(3))).unwrap();
        lhs_tx.send((7, Max::new(4))).unwrap();
        rhs_tx.send((7, Max::new(6))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick();
        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, vec![(7, (Max::new(4), Max::new(6)))]);

        // Doesn't forget
        lhs_tx.send((7, Max::new(4))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick();
        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, vec![(7, (Max::new(4), Max::new(6)))]);
    }
}
