//! Test fixtures for Verus-checked proofs of algebraic properties
//! (`verus_proof_commutative_fold!` / `verus_proof_commutative_map!`).
//!
//! The flows in this crate are never executed; they exist so that the proof obligations
//! generated at their `q!(...)` call sites are verified (or rejected) by Verus. Run:
//!
//! ```bash
//! cargo verus verify -p hydro_verus_tests --features verus
//! ```
//!
//! The `accepted` module must verify. Each module under `rejected` is gated behind a
//! `reject_*` cargo feature and contains a *bogus* commutativity annotation that Verus
//! must refuse to verify; the `verus_rejects` test harness enables them one at a time
//! and asserts that verification fails.

/// Flows whose commutativity annotations must be accepted by Verus.
pub mod accepted {
    use hydro_lang::live_collections::stream::NoOrder;
    use hydro_lang::prelude::*;

    // Non-`Copy` types used by `noncopy_sum_fold`. Types that appear in proof
    // obligations must be known to Verus as datatypes, so they are declared inside
    // `verus!` (whose output compiles as plain Rust under normal cargo builds).
    ::vstd::prelude::verus! {
        pub struct NonCopyAcc {
            pub total: u64,
        }

        pub struct NonCopyItem {
            pub v: u64,
        }
    }

    /// Non-`Copy` accumulator and item types: the generated obligation never reuses
    /// exec values (the items are duplicated parameters constrained equal in the
    /// `requires` clause, each moved into exactly one run of the closure body), ghost
    /// snapshots are spec-level copies, and spec `==` is mathematical equality — so no
    /// `Copy` or `Clone` bounds are needed.
    pub fn noncopy_sum_fold<'a>(process: &Process<'a, ()>) {
        let _sum = process
            .source_iter(q!(vec![NonCopyItem { v: 1 }, NonCopyItem { v: 2 }]))
            .weaken_ordering::<NoOrder>()
            .fold(
                q!(|| NonCopyAcc { total: 0 }),
                q!(
                    |acc, item| {
                        acc.total = acc.total.wrapping_add(item.v);
                    },
                    commutative =
                        verus_proof_commutative_fold!(acc = NonCopyAcc, item = NonCopyItem)
                ),
            );
    }

    /// `max` is commutative: proven automatically by the SMT solver.
    pub fn max_fold<'a>(process: &Process<'a, ()>) {
        let _max = process
            .source_iter(q!(vec![1usize, 2, 3]))
            .weaken_ordering::<NoOrder>()
            .reduce(q!(
                |curr, new| {
                    if new > *curr {
                        *curr = new;
                    }
                },
                commutative = verus_proof_commutative_fold!(acc = usize, item = usize)
            ));
    }

    /// A capped `max` that reads a captured (immutable) threshold; the obligation is
    /// universally quantified over the capture value.
    pub fn capped_max_fold<'a>(process: &Process<'a, ()>) {
        let cap = 100usize;
        let _max = process
            .source_iter(q!(vec![1usize, 2, 3]))
            .weaken_ordering::<NoOrder>()
            .reduce(q!(
                move |curr, new| {
                    if new > *curr && new <= cap {
                        *curr = new;
                    }
                },
                commutative = verus_proof_commutative_fold!(
                    acc = usize,
                    item = usize,
                    captures = |cap: usize|
                )
            ));
    }

    /// Bitwise `or` needs solver help (`by (bit_vector)`), provided through the
    /// `proof = ...` script. The script cannot weaken the obligation; it is itself
    /// checked by Verus.
    pub fn bitor_fold<'a>(process: &Process<'a, ()>) {
        let _flags = process
            .source_iter(q!(vec![1u32, 2, 4]))
            .weaken_ordering::<NoOrder>()
            .fold(
                q!(|| 0u32),
                q!(
                    |flags, x| {
                        *flags |= x;
                    },
                    commutative = verus_proof_commutative_fold!(
                        acc = u32,
                        item = u32,
                        proof = |s, x, y| {
                            assert(((s | x) | y) == ((s | y) | x)) by (bit_vector);
                        }
                    )
                ),
            );
    }

    /// A map closure that updates a mutable singleton reference and returns the running
    /// *count* of processed items (ignoring the item): the multiset of outputs
    /// {s+1, s+2} is order-independent (the proof's crossed-equality branch), and the
    /// captured state commutes, universally quantified over its initial value.
    pub fn counting_map_capture<'a>(process: &Process<'a, ()>) {
        let count = process
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));

        let count_mut = count.by_mut();

        let _out = process
            .source_iter(q!(1..=3i32))
            .weaken_ordering::<NoOrder>()
            .map(q!(
                |_x| {
                    *count_mut = count_mut.wrapping_add(1);
                    *count_mut
                },
                commutative = verus_proof_commutative_map!(
                    item = i32,
                    captures_mut = |count_mut: i32|
                )
            ));
    }

    /// A filter predicate that counts how many elements it has seen in a mutable
    /// singleton reference: the retained elements depend only on the item (so the
    /// retained multiset is order-independent), and the counter update commutes.
    pub fn counting_filter_capture<'a>(process: &Process<'a, ()>) {
        let seen = process.source_iter(q!(0..5u32)).fold(
            q!(|| 0u32),
            q!(|acc: &mut u32, _x| *acc = acc.wrapping_add(1)),
        );

        let seen_mut = seen.by_mut();

        let _out = process
            .source_iter(q!(vec![1u32, 2, 3]))
            .weaken_ordering::<NoOrder>()
            .filter(q!(
                |x| {
                    *seen_mut = seen_mut.wrapping_add(1);
                    *x > 1
                },
                commutative = verus_proof_commutative_filter!(
                    item = &u32,
                    captures_mut = |seen_mut: u32|
                )
            ));
    }

    /// A unit-returning `for_each` closure that ORs items into a mutable singleton
    /// reference: the captured state is the only observable effect, and bitwise `or`
    /// commutes (shown with a `by (bit_vector)` proof script).
    pub fn bitor_for_each_effect<'a>(process: &Process<'a, ()>) {
        let flags = process
            .source_iter(q!(0..1u32))
            .fold(q!(|| 0u32), q!(|acc: &mut u32, x| *acc |= x));

        let flags_mut = flags.by_mut();

        process
            .source_iter(q!(vec![1u32, 2, 4]))
            .weaken_ordering::<NoOrder>()
            .for_each(q!(
                |x| {
                    *flags_mut |= x;
                },
                commutative = verus_proof_commutative_effect!(
                    item = u32,
                    captures_mut = |flags_mut: u32|,
                    proof = |s, x, y| {
                        assert(((s | x) | y) == ((s | y) | x)) by (bit_vector);
                    }
                )
            ));
    }
}

/// Flows whose commutativity annotations must be **rejected** by Verus. Each is behind a
/// cargo feature so the `verus_rejects` harness can check them one at a time.
pub mod rejected {
    /// Last-writer-wins overwrite is not commutative.
    #[cfg(feature = "reject_overwrite_fold")]
    pub mod overwrite_fold {
        use hydro_lang::live_collections::stream::NoOrder;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let _last = process
                .source_iter(q!(vec![1usize, 2, 3]))
                .weaken_ordering::<NoOrder>()
                .reduce(q!(
                    |curr, new| {
                        *curr = new;
                    },
                    commutative = verus_proof_commutative_fold!(acc = usize, item = usize)
                ));
        }
    }

    /// `acc = acc / 2 + x` (wrapping) is panic-free but order-dependent.
    #[cfg(feature = "reject_halving_fold")]
    pub mod halving_fold {
        use hydro_lang::live_collections::stream::NoOrder;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let _acc = process
                .source_iter(q!(vec![1u32, 2, 3]))
                .weaken_ordering::<NoOrder>()
                .fold(
                    q!(|| 0u32),
                    q!(
                        |acc, x| {
                            *acc = (*acc / 2).wrapping_add(x);
                        },
                        commutative = verus_proof_commutative_fold!(acc = u32, item = u32)
                    ),
                );
        }
    }

    /// Plain `+=` is mathematically commutative, but the obligation currently also
    /// requires panic-freedom, so the potential overflow is rejected. (Making the
    /// obligation conditional on both orders completing without panics is future work.)
    #[cfg(feature = "reject_overflowing_fold")]
    pub mod overflowing_fold {
        use hydro_lang::live_collections::stream::NoOrder;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let _sum = process
                .source_iter(q!(vec![1u32, 2, 3]))
                .weaken_ordering::<NoOrder>()
                .fold(
                    q!(|| 0u32),
                    q!(
                        |acc, x| {
                            *acc += x;
                        },
                        commutative = verus_proof_commutative_fold!(acc = u32, item = u32)
                    ),
                );
        }
    }

    /// Overwriting a captured singleton reference is not a commutative state update.
    #[cfg(feature = "reject_overwrite_map_capture")]
    pub mod overwrite_map_capture {
        use hydro_lang::live_collections::stream::NoOrder;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let last = process
                .source_iter(q!(0..5i32))
                .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));

            let last_mut = last.by_mut();

            let _out = process
                .source_iter(q!(1..=3i32))
                .weaken_ordering::<NoOrder>()
                .map(q!(
                    |x| {
                        *last_mut = x;
                        *last_mut
                    },
                    commutative = verus_proof_commutative_map!(
                        item = i32,
                        captures_mut = |last_mut: i32|
                    )
                ));
        }
    }

    /// A map that outputs a running total: the *state update* (wrapping addition)
    /// commutes, but the outputs {s+x, s+x+y} vs {s+y, s+y+x} expose the processing
    /// order, so the output-multiset half of the obligation must reject it.
    #[cfg(feature = "reject_running_total_map")]
    pub mod running_total_map {
        use hydro_lang::live_collections::stream::NoOrder;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let count = process
                .source_iter(q!(0..5i32))
                .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));

            let count_mut = count.by_mut();

            let _out = process
                .source_iter(q!(1..=3i32))
                .weaken_ordering::<NoOrder>()
                .map(q!(
                    |x| {
                        *count_mut = count_mut.wrapping_add(x);
                        *count_mut
                    },
                    commutative = verus_proof_commutative_map!(
                        item = i32,
                        captures_mut = |count_mut: i32|
                    )
                ));
        }
    }

    /// A rate-limiting filter: the budget converges to the same value in either order,
    /// but *which* element passes depends on the order, so the retained-multiset half
    /// of the obligation must reject it.
    #[cfg(feature = "reject_rate_limit_filter")]
    pub mod rate_limit_filter {
        use hydro_lang::live_collections::stream::NoOrder;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let budget = process
                .source_iter(q!(0..1u32))
                .fold(q!(|| 1u32), q!(|acc: &mut u32, x| *acc |= x));

            let budget_mut = budget.by_mut();

            let _out = process
                .source_iter(q!(vec![1u32, 2, 3]))
                .weaken_ordering::<NoOrder>()
                .filter(q!(
                    |_x| {
                        if *budget_mut > 0 {
                            *budget_mut -= 1;
                            true
                        } else {
                            false
                        }
                    },
                    commutative = verus_proof_commutative_filter!(
                        item = &u32,
                        captures_mut = |budget_mut: u32|
                    )
                ));
        }
    }
}
