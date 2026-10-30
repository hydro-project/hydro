//! Test fixtures for Verus-checked proofs of algebraic properties
//! (`verus_proof_commutative_*!`, `verus_proof_monotone_fold!`, and
//! `verus_proof_order_preserving_map!`).
//!
//! The flows in this crate are never executed; they exist so that the proof obligations
//! generated at their `q!(...)` call sites are verified (or rejected) by Verus. Run:
//!
//! ```bash
//! cargo verus verify -p hydro_verus_tests --features verus
//! ```
//!
//! The `accepted` module must verify. Each module under `rejected` is gated behind a
//! `reject_*` cargo feature and contains a *bogus* property annotation that Verus must
//! refuse to verify; the `verus_rejects` test harness enables them one at a time and
//! asserts that verification fails.

/// Flows whose property annotations must be accepted by Verus.
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

    /// Plain `+=` guarded by `verus_panic!`: the overflowing path panics, so Verus only
    /// requires commutativity on the paths that continue, where `+=` cannot overflow.
    pub fn guarded_sum_fold<'a>(process: &Process<'a, ()>) {
        let _sum = process
            .source_iter(q!(vec![1u32, 2, 3]))
            .weaken_ordering::<NoOrder>()
            .fold(
                q!(|| 0u32),
                q!(
                    |acc, x| {
                        if *acc > u32::MAX - x {
                            verus_panic!("sum overflowed");
                        }
                        *acc += x;
                    },
                    commutative = verus_proof_commutative_fold!(acc = u32, item = u32)
                ),
            );
    }

    /// A guard stricter than necessary is still sound (it really panics at runtime).
    pub fn strictly_guarded_sum_fold<'a>(process: &Process<'a, ()>) {
        let _sum = process
            .source_iter(q!(vec![1u32, 2, 3]))
            .weaken_ordering::<NoOrder>()
            .fold(
                q!(|| 0u32),
                q!(
                    |acc, x| {
                        if *acc > 1_000_000 || x > 1_000_000 {
                            verus_panic!("sum overflowed");
                        }
                        *acc += x;
                    },
                    commutative = verus_proof_commutative_fold!(acc = u32, item = u32)
                ),
            );
    }

    // === Monotonicity ===

    /// `max` is monotone: the accumulator never decreases. The resulting singleton is
    /// `Monotonic`, so it can be used with `threshold_greater_or_equal`.
    pub fn max_monotone_fold<'a>(process: &Process<'a, ()>) {
        let max = process.source_iter(q!(vec![1u32, 2, 3])).fold(
            q!(|| 0u32),
            q!(
                |curr, new| {
                    if new > *curr {
                        *curr = new;
                    }
                },
                monotone = verus_proof_monotone_fold!(acc = u32, item = u32)
            ),
        );
        let _reached = max.threshold_greater_or_equal(process.singleton(q!(2u32)));
    }

    /// Both commutativity and monotonicity can be proven on the same closure; each
    /// macro generates its own, independent obligation.
    pub fn max_commutative_monotone_fold<'a>(process: &Process<'a, ()>) {
        let max = process
            .source_iter(q!(vec![1u32, 2, 3]))
            .weaken_ordering::<NoOrder>()
            .fold(
                q!(|| 0u32),
                q!(
                    |curr, new| {
                        if new > *curr {
                            *curr = new;
                        }
                    },
                    commutative = verus_proof_commutative_fold!(acc = u32, item = u32),
                    monotone = verus_proof_monotone_fold!(acc = u32, item = u32)
                ),
            );
        let _reached = max.threshold_greater_or_equal(process.singleton(q!(2u32)));
    }

    /// A guarded sum is monotone: on the paths that continue, `acc + x >= acc`.
    pub fn guarded_sum_monotone_fold<'a>(process: &Process<'a, ()>) {
        let _sum = process.source_iter(q!(vec![1u32, 2, 3])).fold(
            q!(|| 0u32),
            q!(
                |acc, x| {
                    if *acc > u32::MAX - x {
                        verus_panic!("sum overflowed");
                    }
                    *acc += x;
                },
                monotone = verus_proof_monotone_fold!(acc = u32, item = u32)
            ),
        );
    }

    /// A counter that stops at a captured cap: monotone for every value of the capture.
    pub fn capped_count_monotone_fold<'a>(process: &Process<'a, ()>) {
        let cap = 100u32;
        let _count = process.source_iter(q!(vec![1u32, 2, 3])).fold(
            q!(|| 0u32),
            q!(
                move |count, _x| {
                    if *count < cap {
                        *count += 1;
                    }
                },
                monotone = verus_proof_monotone_fold!(acc = u32, item = u32, captures = |cap: u32|)
            ),
        );
    }

    /// Bitwise `or` only ever sets bits, so it is monotone, but the solver needs the
    /// `bit_vector` hint. The script receives the initial accumulator and the item.
    pub fn bitor_monotone_fold<'a>(process: &Process<'a, ()>) {
        let _flags = process.source_iter(q!(vec![1u32, 2, 4])).fold(
            q!(|| 0u32),
            q!(
                |flags, x| {
                    *flags |= x;
                },
                monotone = verus_proof_monotone_fold!(
                    acc = u32,
                    item = u32,
                    proof = |s, x| {
                        assert(s <= (s | x)) by (bit_vector);
                    }
                )
            ),
        );
    }

    /// A tuple accumulator `(count, sum)` is ordered lexicographically, so incrementing
    /// the count makes it grow regardless of how the (wrapping) sum changes. The proof
    /// script may also bind the final accumulator.
    pub fn tuple_monotone_fold<'a>(process: &Process<'a, ()>) {
        let _stats = process.source_iter(q!(vec![1u32, 2, 3])).fold(
            q!(|| (0u32, 0u32)),
            q!(
                |stats, x| {
                    if stats.0 == u32::MAX {
                        verus_panic!("count overflowed");
                    }
                    stats.0 += 1;
                    stats.1 = stats.1.wrapping_add(x);
                },
                monotone = verus_proof_monotone_fold!(
                    acc = (u32, u32),
                    item = u32,
                    proof = |before, _x, after| {
                        assert(before.0 < after.0);
                    }
                )
            ),
        );
    }

    // === Order preservation ===

    /// Integer halving preserves order, so a `Monotonic` singleton stays `Monotonic`
    /// through the `map` and can still be thresholded.
    pub fn halving_order_preserving_map<'a>(process: &Process<'a, ()>) {
        let count = process.source_iter(q!(vec![1u32, 2, 3])).count();
        let halved = count.map(q!(
            |v| v / 2,
            order_preserving = verus_proof_order_preserving_map!(input = usize, output = usize)
        ));
        let _reached = halved.threshold_greater_or_equal(process.singleton(q!(1usize)));
    }

    /// A widening cast preserves order across different input and output types.
    pub fn widening_order_preserving_map<'a>(process: &Process<'a, ()>) {
        let count = process.source_iter(q!(vec![1u32, 2, 3])).count();
        let _wide = count.map(q!(
            |v| v as u128,
            order_preserving = verus_proof_order_preserving_map!(input = usize, output = u128)
        ));
    }

    /// Clamping to a captured cap preserves order for every value of the capture.
    pub fn capped_order_preserving_map<'a>(process: &Process<'a, ()>) {
        let cap = 2usize;
        let count = process.source_iter(q!(vec![1u32, 2, 3])).count();
        let _capped = count.map(q!(
            move |v| if v > cap { cap } else { v },
            order_preserving = verus_proof_order_preserving_map!(
                input = usize,
                output = usize,
                captures = |cap: usize|
            )
        ));
    }

    /// A right shift preserves order, but the solver needs the `bit_vector` hint. The
    /// script receives the two inputs (and may also bind the two outputs).
    pub fn shift_order_preserving_map<'a>(process: &Process<'a, ()>) {
        let max = process.source_iter(q!(vec![1u32, 2, 3])).fold(
            q!(|| 0u32),
            q!(
                |curr, new| {
                    if new > *curr {
                        *curr = new;
                    }
                },
                monotone = verus_proof_monotone_fold!(acc = u32, item = u32)
            ),
        );
        let _shifted = max.map(q!(
            |v| v >> 1,
            order_preserving = verus_proof_order_preserving_map!(
                input = u32,
                output = u32,
                proof = |x, y, fx, fy| {
                    assert(x <= y ==> (x >> 1u32) <= (y >> 1u32)) by (bit_vector);
                    assert(fx <= fy);
                }
            )
        ));
    }
}

/// Flows whose property annotations must be **rejected** by Verus. Each is behind a
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

    /// Plain `+=` is mathematically commutative, but Verus requires the closure to be
    /// panic-free, so the potential overflow is rejected (see `guarded_sum_fold` for the
    /// accepted version using `verus_panic!`).
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

    /// A `verus_panic!` guard that is too weak (it misses the case `x == 1`) must not
    /// make the overflowing `+=` verify.
    #[cfg(feature = "reject_weak_panic_guard")]
    pub mod weak_panic_guard {
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
                            if *acc > u32::MAX - x && x > 1 {
                                verus_panic!("sum overflowed");
                            }
                            *acc += x;
                        },
                        commutative = verus_proof_commutative_fold!(acc = u32, item = u32)
                    ),
                );
        }
    }

    /// `verus_panic!` only removes the paths that reach it; a non-commutative update on
    /// the paths that continue (`acc / 2 + x`) must still be rejected.
    #[cfg(feature = "reject_guarded_halving_fold")]
    pub mod guarded_halving_fold {
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
                            if *acc / 2 > u32::MAX - x {
                                verus_panic!("sum overflowed");
                            }
                            *acc = *acc / 2 + x;
                        },
                        commutative = verus_proof_commutative_fold!(acc = u32, item = u32)
                    ),
                );
        }
    }

    /// Last-writer-wins overwrite is not monotone: the new value may be smaller.
    #[cfg(feature = "reject_overwrite_monotone_fold")]
    pub mod overwrite_monotone_fold {
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let _last = process.source_iter(q!(vec![1u32, 2, 3])).fold(
                q!(|| 0u32),
                q!(
                    |curr, new| {
                        *curr = new;
                    },
                    monotone = verus_proof_monotone_fold!(acc = u32, item = u32)
                ),
            );
        }
    }

    /// A wrapping sum is panic-free but not monotone: it decreases when it wraps around.
    #[cfg(feature = "reject_wrapping_sum_monotone_fold")]
    pub mod wrapping_sum_monotone_fold {
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let _sum = process.source_iter(q!(vec![1u32, 2, 3])).fold(
                q!(|| 0u32),
                q!(
                    |acc, x| {
                        *acc = acc.wrapping_add(x);
                    },
                    monotone = verus_proof_monotone_fold!(acc = u32, item = u32)
                ),
            );
        }
    }

    /// Plain `+=` is monotone whenever it completes, but Verus requires the closure to be
    /// panic-free, so the potential overflow is rejected (see `guarded_sum_monotone_fold`
    /// for the accepted version using `verus_panic!`).
    #[cfg(feature = "reject_overflowing_monotone_fold")]
    pub mod overflowing_monotone_fold {
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let _sum = process.source_iter(q!(vec![1u32, 2, 3])).fold(
                q!(|| 0u32),
                q!(
                    |acc, x| {
                        *acc += x;
                    },
                    monotone = verus_proof_monotone_fold!(acc = u32, item = u32)
                ),
            );
        }
    }

    /// `MAX - v` reverses the order, so it is not order-preserving.
    #[cfg(feature = "reject_reversing_map")]
    pub mod reversing_map {
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let count = process.source_iter(q!(vec![1u32, 2, 3])).count();
            let _reversed = count.map(q!(
                |v| usize::MAX - v,
                order_preserving = verus_proof_order_preserving_map!(input = usize, output = usize)
            ));
        }
    }

    /// Wrapping doubling is panic-free but not order-preserving: it wraps around for
    /// large inputs, so `x <= y` does not imply `2x <= 2y`.
    #[cfg(feature = "reject_wrapping_double_map")]
    pub mod wrapping_double_map {
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let count = process.source_iter(q!(vec![1u32, 2, 3])).count();
            let _doubled = count.map(q!(
                |v| v.wrapping_mul(2),
                order_preserving = verus_proof_order_preserving_map!(input = usize, output = usize)
            ));
        }
    }
}
