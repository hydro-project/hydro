use std::collections::HashSet;

use dfir_macro::dfir_syntax;
use dfir_rs::util::collect_ready;
use lattices::DeepReveal;
use lattices::collections::SingletonMap;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_lattice_join_fused_join_reducing_behavior() {
    // lattice_fold has the following particular initialization behavior. It uses LatticeFrom to convert the first element into another lattice type to create the accumulator and then it folds the remaining elements into that accumulator.
    // This initialization style supports things like SetUnionSingletonSet -> SetUnionHashSet. It also helps with things like Min, where there is not obvious default value so you want reduce-like behavior.
    let mut df = dfir_syntax! {
        use lattices::Min;
        use lattices::Max;

        source_iter([(7, Min::new(5)), (7, Min::new(6))]) -> [0]my_join;
        source_iter([(7, Max::new(5)), (7, Max::new(6))]) -> [1]my_join;

        my_join = _lattice_join_fused_join::<Min<usize>, Max<usize>>()
            -> map(DeepReveal::deep_reveal)
            -> assert_eq([
                SingletonMap(7, (5, 6))
            ]);
    };

    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_lattice_join_fused_join_set_union() {
    let mut df = dfir_syntax! {
        use lattices::set_union::SetUnionSingletonSet;
        use lattices::set_union::SetUnionHashSet;

        source_iter([(7, SetUnionHashSet::new_from([5])), (7, SetUnionHashSet::new_from([6]))]) -> [0]my_join;
        source_iter([(7, SetUnionSingletonSet::new_from(5)), (7, SetUnionSingletonSet::new_from(6))]) -> [1]my_join;

        my_join = _lattice_join_fused_join::<SetUnionHashSet<usize>, SetUnionHashSet<usize>>()
            -> map(DeepReveal::deep_reveal)
            -> assert_eq([SingletonMap(7, (HashSet::from_iter([5, 6]), HashSet::from_iter([5, 6])))]);
    };

    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_lattice_join_fused_join_map_union() {
    let mut df = dfir_syntax! {
        use lattices::map_union::MapUnionSingletonMap;
        use lattices::map_union::MapUnionHashMap;
        use dfir_rs::lattices::Min;

        source_iter([(7, MapUnionHashMap::new_from([(3, Min::new(4))])), (7, MapUnionHashMap::new_from([(3, Min::new(5))]))]) -> [0]my_join;
        source_iter([(7, MapUnionSingletonMap::new_from((3, Min::new(5)))), (7, MapUnionSingletonMap::new_from((3, Min::new(4))))]) -> [1]my_join;

        my_join = _lattice_join_fused_join::<MapUnionHashMap<usize, Min<usize>>, MapUnionHashMap<usize, Min<usize>>>()
            -> map(DeepReveal::deep_reveal)
            -> assert(|SingletonMap(_k, (mu1, mu2))| mu1.len() == 1 && mu2.len() == 1)
            -> map(|SingletonMap(k, (mu1, mu2))| {
                let v1 = mu1.into_iter().next().unwrap();
                let v2 = mu2.into_iter().next().unwrap();
                (k, (v1, v2))
            })
            -> assert_eq([
                (7, ((3, 4), (3, 4)))
            ]);
    };

    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_lattice_join_fused_join() {
    use dfir_rs::lattices::Max;

    // 'static, 'tick.
    {
        let (out_tx, mut out_rx) =
            dfir_rs::util::unbounded_channel::<SingletonMap<usize, (usize, usize)>>();
        let (lhs_tx, lhs_rx) = dfir_rs::util::unbounded_channel::<(usize, Max<usize>)>();
        let (rhs_tx, rhs_rx) = dfir_rs::util::unbounded_channel::<(usize, Max<usize>)>();

        let mut df = dfir_syntax! {
            my_join = _lattice_join_fused_join::<'static, 'tick, Max<usize>, Max<usize>>();

            source_stream(lhs_rx) -> [0]my_join;
            source_stream(rhs_rx) -> [1]my_join;

            my_join
                -> map(DeepReveal::deep_reveal)
                -> for_each(|v| out_tx.send(v).unwrap());
        };

        // Merges forward correctly, without going backward
        lhs_tx.send((7, Max::new(3))).unwrap();
        lhs_tx.send((7, Max::new(4))).unwrap();
        rhs_tx.send((7, Max::new(6))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick_sync();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, [SingletonMap(7, (4, 6))]);

        // Forgets rhs state
        rhs_tx.send((7, Max::new(6))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick_sync();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, [SingletonMap(7, (4, 6))]);
    }

    // 'static, 'static.
    {
        let (out_tx, mut out_rx) =
            dfir_rs::util::unbounded_channel::<SingletonMap<usize, (usize, usize)>>();
        let (lhs_tx, lhs_rx) = dfir_rs::util::unbounded_channel::<(usize, Max<usize>)>();
        let (rhs_tx, rhs_rx) = dfir_rs::util::unbounded_channel::<(usize, Max<usize>)>();

        let mut df = dfir_syntax! {
            my_join = _lattice_join_fused_join::<'static, 'static, Max<usize>, Max<usize>>();

            source_stream(lhs_rx) -> [0]my_join;
            source_stream(rhs_rx) -> [1]my_join;

            my_join
                -> map(DeepReveal::deep_reveal)
                -> for_each(|v| out_tx.send(v).unwrap());
        };

        lhs_tx.send((7, Max::new(3))).unwrap();
        lhs_tx.send((7, Max::new(4))).unwrap();
        rhs_tx.send((7, Max::new(6))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick_sync();
        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, [SingletonMap(7, (4, 6))]);

        // Doesn't forget
        lhs_tx.send((7, Max::new(4))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick_sync();
        let out: Vec<_> = collect_ready(&mut out_rx);
        // TODO(mingwei): Should only be one, but bug: https://github.com/hydro-project/hydro/issues/1050#issuecomment-1924338317
        assert_eq!(out, [SingletonMap(7, (4, 6)), SingletonMap(7, (4, 6))]);
    }
}
