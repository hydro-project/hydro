use dfir_rs::dfir_syntax;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_lattice_batch() {
    type SetUnionHashSet = lattices::set_union::SetUnionHashSet<usize>;
    type SetUnionSingletonSet = lattices::set_union::SetUnionSingletonSet<usize>;

    let mut df = dfir_syntax! {
        // Can release in the same tick
        source_iter([SetUnionSingletonSet::new_from(0), SetUnionSingletonSet::new_from(1)]) -> [input]b1;
        source_iter([()]) -> defer_tick() -> [signal]b1;
        b1 = _lattice_fold_batch::<SetUnionHashSet>() -> assert_eq([SetUnionHashSet::new_from([1, 0])]);

        // Can hold the data across a tick
        source_iter([SetUnionSingletonSet::new_from(0), SetUnionSingletonSet::new_from(1)]) -> [input]b2;
        source_iter([()]) -> defer_tick() -> [signal]b2;
        b2 = _lattice_fold_batch::<SetUnionHashSet>() -> assert_eq([SetUnionHashSet::new_from([1, 0])]);

        // Doesn't release without a signal
        source_iter([SetUnionSingletonSet::new_from(0), SetUnionSingletonSet::new_from(1)]) -> [input]b3;
        source_iter([(); 0]) -> [signal]b3;
        b3 = _lattice_fold_batch::<SetUnionHashSet>() -> assert(|_| false);
    };

    df.run_available_sync();
}
