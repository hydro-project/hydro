//! Surface codegen must be clean under `forbid(unused_results)` (issue #2705).
//!
//! The attribute is on the function so it covers macro-expanded statements. User
//! closures here return `()`.

#[test]
#[forbid(unused_results)]
fn dfir_codegen_forbids_unused_results() {
    // Linear pipeline: metrics init/update and operator statements.
    let mut linear = dfir_rs::dfir_syntax! {
        source_iter([1, 1, 2]) -> unique() -> null();
    };
    linear.run_available_sync();

    // Handoff across a tick boundary (recv-side metrics).
    let mut deferred = dfir_rs::dfir_syntax! {
        source_iter([1]) -> defer_tick() -> null();
    };
    deferred.run_available_sync();

    // `VacantEntry::insert` returns a reference.
    let mut reduced = dfir_rs::dfir_syntax! {
        source_iter([("a", 1), ("a", 2)])
            -> reduce_keyed(|acc: &mut i32, v| *acc += v)
            -> null();
    };
    reduced.run_available_sync();

    // `HashSet::insert` on the negative side.
    let mut diff = dfir_rs::dfir_syntax! {
        source_iter([1, 2]) -> [pos]diff;
        source_iter([1]) -> [neg]diff;
        diff = difference() -> null();
    };
    diff.run_available_sync();

    // `Merge::merge` returns whether the lattice changed.
    let mut lattices = dfir_rs::dfir_syntax! {
        source_iter([1, 2, 3])
            -> map(dfir_rs::lattices::Max::new)
            -> lattice_reduce()
            -> null();
    };
    lattices.run_available_sync();
}
