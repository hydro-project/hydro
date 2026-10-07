//! Test fixtures for Verus-checked proofs of algebraic properties
//! (`verus_proof_commutative_*!` / `verus_proof_idempotent_*!`).
//!
//! The flows in this crate are never executed; they exist so that the proof obligations
//! generated at their `q!(...)` call sites are verified (or rejected) by Verus. Run:
//!
//! ```bash
//! cargo verus verify -p hydro_verus_tests --features verus
//! ```
//!
//! The `accepted` module must verify. Each module under `rejected` is gated behind a
//! `reject_*` cargo feature and contains a *bogus* commutativity or idempotence
//! annotation that Verus must refuse to verify; the `verus_rejects` test harness enables
//! them one at a time and asserts that verification fails.

/// Flows whose commutativity and idempotence annotations must be accepted by Verus.
pub mod accepted {
    use hydro_lang::live_collections::stream::{AtLeastOnce, NoOrder};
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

    /// `max` on an unordered stream with retries is both commutative and idempotent;
    /// both proofs annotate the same closure. As a `reduce`, the idempotence proof also
    /// covers the seed obligation (`max(x, x) == x`).
    pub fn max_reduce_commutative_idempotent<'a>(process: &Process<'a, ()>) {
        let _max = process
            .source_iter(q!(vec![1usize, 2, 3]))
            .weaken_ordering::<NoOrder>()
            .weaken_retries::<AtLeastOnce>()
            .reduce(q!(
                |curr, new| {
                    if new > *curr {
                        *curr = new;
                    }
                },
                commutative = verus_proof_commutative_fold!(acc = usize, item = usize),
                idempotent = verus_proof_idempotent_reduce!(item = usize)
            ));
    }

    /// Bitwise `or` is idempotent, which needs solver help (`by (bit_vector)`) through
    /// the `proof = ...` script.
    pub fn bitor_fold_idempotent<'a>(process: &Process<'a, ()>) {
        let _flags = process
            .source_iter(q!(vec![1u32, 2, 4]))
            .weaken_retries::<AtLeastOnce>()
            .fold(
                q!(|| 0u32),
                q!(
                    |flags, x| {
                        *flags |= x;
                    },
                    idempotent = verus_proof_idempotent_fold!(
                        acc = u32,
                        item = u32,
                        proof = |s, x| {
                            assert(((s | x) | x) == (s | x)) by (bit_vector);
                        }
                    )
                ),
            );
    }

    /// A capped `max` that reads a captured (immutable) threshold; the obligation is
    /// universally quantified over the capture value.
    pub fn capped_max_fold_idempotent<'a>(process: &Process<'a, ()>) {
        let cap = 100usize;
        let _max = process
            .source_iter(q!(vec![1usize, 2, 3]))
            .weaken_retries::<AtLeastOnce>()
            .fold(
                q!(|| 0usize),
                q!(
                    move |curr, new| {
                        if new > *curr && new <= cap {
                            *curr = new;
                        }
                    },
                    idempotent = verus_proof_idempotent_fold!(
                        acc = usize,
                        item = usize,
                        captures = |cap: usize|
                    )
                ),
            );
    }

    /// Max-by-key over `(key, value)` pairs, mirroring the `manual_proof!` idempotence
    /// annotation on the `reduce` in `KeyedSingleton::get_max_key`: a repeated element
    /// does not have a strictly greater key, so it is ignored.
    pub fn max_by_key_reduce_idempotent<'a>(process: &Process<'a, ()>) {
        let _max = process
            .source_iter(q!(vec![(1u64, 10u64), (2, 20)]))
            .weaken_retries::<AtLeastOnce>()
            .reduce(q!(
                |curr, new| {
                    if new.0 > curr.0 {
                        *curr = new;
                    }
                },
                idempotent = verus_proof_idempotent_reduce!(item = (u64, u64))
            ));
    }

    /// A map closure that tracks the maximum seen in a mutable singleton reference and
    /// passes the item through: the state update is idempotent, and a retried element
    /// re-emits the same output.
    pub fn max_tracking_map_idempotent<'a>(process: &Process<'a, ()>) {
        let max_seen = process
            .source_iter(q!(0..5u32))
            .fold(q!(|| 0u32), q!(|acc: &mut u32, x| *acc = x));

        let max_mut = max_seen.by_mut();

        let _out = process
            .source_iter(q!(vec![1u32, 2, 3]))
            .weaken_retries::<AtLeastOnce>()
            .map(q!(
                |x| {
                    if x > *max_mut {
                        *max_mut = x;
                    }
                    x
                },
                idempotent = verus_proof_idempotent_map!(
                    item = u32,
                    captures_mut = |max_mut: u32|
                )
            ));
    }

    /// A filter predicate that tracks the maximum seen but decides based only on the
    /// item, so a retried element gets the same decision.
    pub fn threshold_filter_idempotent<'a>(process: &Process<'a, ()>) {
        let max_seen = process
            .source_iter(q!(0..5u32))
            .fold(q!(|| 0u32), q!(|acc: &mut u32, x| *acc = x));

        let max_mut = max_seen.by_mut();

        let _out = process
            .source_iter(q!(vec![1u32, 2, 3]))
            .weaken_retries::<AtLeastOnce>()
            .filter(q!(
                |x| {
                    if *x > *max_mut {
                        *max_mut = *x;
                    }
                    *x > 1
                },
                idempotent = verus_proof_idempotent_filter!(
                    item = &u32,
                    captures_mut = |max_mut: u32|
                )
            ));
    }

    /// The same predicate shape on `partition`, where the decision must match exactly
    /// (a retried element must not land in the other output).
    pub fn threshold_partition_idempotent<'a>(process: &Process<'a, ()>) {
        let max_seen = process
            .source_iter(q!(0..5u32))
            .fold(q!(|| 0u32), q!(|acc: &mut u32, x| *acc = x));

        let max_mut = max_seen.by_mut();

        let (_big, _small) = process
            .source_iter(q!(vec![1u32, 2, 3]))
            .weaken_retries::<AtLeastOnce>()
            .partition(q!(
                |x| {
                    if *x > *max_mut {
                        *max_mut = *x;
                    }
                    *x > 1
                },
                idempotent = verus_proof_idempotent_filter!(
                    item = &u32,
                    captures_mut = |max_mut: u32|
                )
            ));
    }

    /// A unit-returning `for_each` closure that ORs a boolean into a mutable singleton
    /// reference, the example from the `for_each` documentation.
    pub fn bool_or_for_each_idempotent<'a>(process: &Process<'a, ()>) {
        let failed = process
            .source_iter(q!(vec![false]))
            .fold(q!(|| false), q!(|acc: &mut bool, x| *acc = *acc || x));

        let failed_mut = failed.by_mut();

        process
            .source_iter(q!(vec![false, true, false]))
            .weaken_retries::<AtLeastOnce>()
            .for_each(q!(
                |x| {
                    *failed_mut = *failed_mut || x;
                },
                idempotent = verus_proof_idempotent_effect!(
                    item = bool,
                    captures_mut = |failed_mut: bool|
                )
            ));
    }

    /// An `inspect` closure (which borrows its item) that tracks the maximum seen.
    pub fn max_inspect_idempotent<'a>(process: &Process<'a, ()>) {
        let max_seen = process
            .source_iter(q!(0..5u32))
            .fold(q!(|| 0u32), q!(|acc: &mut u32, x| *acc = x));

        let max_mut = max_seen.by_mut();

        let _out = process
            .source_iter(q!(vec![1u32, 2, 3]))
            .weaken_retries::<AtLeastOnce>()
            .inspect(q!(
                |x| {
                    if *x > *max_mut {
                        *max_mut = *x;
                    }
                },
                idempotent = verus_proof_idempotent_effect!(
                    item = &u32,
                    captures_mut = |max_mut: u32|
                )
            ));
    }
}

/// Flows whose commutativity or idempotence annotations must be **rejected** by Verus.
/// Each is behind a cargo feature so the `verus_rejects` harness can check them one at a
/// time.
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

    /// Wrapping addition is commutative but not idempotent: adding a retried element
    /// twice double-counts it.
    #[cfg(feature = "reject_sum_fold_idempotent")]
    pub mod sum_fold_idempotent {
        use hydro_lang::live_collections::stream::AtLeastOnce;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let _sum = process
                .source_iter(q!(vec![1u32, 2, 3]))
                .weaken_retries::<AtLeastOnce>()
                .fold(
                    q!(|| 0u32),
                    q!(
                        |acc, x| {
                            *acc = acc.wrapping_add(x);
                        },
                        idempotent = verus_proof_idempotent_fold!(acc = u32, item = u32)
                    ),
                );
        }
    }

    /// `acc = max(acc, x + 1)` passes the fold obligation (applying `x` twice equals
    /// applying it once), but not the `reduce` seed obligation: reducing `[a, a]` yields
    /// `a + 1` instead of `a`.
    #[cfg(feature = "reject_reduce_seed")]
    pub mod reduce_seed {
        use hydro_lang::live_collections::stream::AtLeastOnce;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let _max = process
                .source_iter(q!(vec![1u32, 2, 3]))
                .weaken_retries::<AtLeastOnce>()
                .reduce(q!(
                    |acc, x| {
                        let bumped = if x < u32::MAX { x + 1 } else { x };
                        if bumped > *acc {
                            *acc = bumped;
                        }
                    },
                    idempotent = verus_proof_idempotent_reduce!(item = u32)
                ));
        }
    }

    /// Counting processed elements in a captured singleton reference is not an
    /// idempotent state update: a retried element is counted twice.
    #[cfg(feature = "reject_counter_map_idempotent")]
    pub mod counter_map_idempotent {
        use hydro_lang::live_collections::stream::AtLeastOnce;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let count = process.source_iter(q!(0..5u32)).fold(
                q!(|| 0u32),
                q!(|acc: &mut u32, _x| *acc = acc.wrapping_add(1)),
            );

            let count_mut = count.by_mut();

            let _out = process
                .source_iter(q!(vec![1u32, 2, 3]))
                .weaken_retries::<AtLeastOnce>()
                .map(q!(
                    |x| {
                        *count_mut = count_mut.wrapping_add(1);
                        x
                    },
                    idempotent = verus_proof_idempotent_map!(
                        item = u32,
                        captures_mut = |count_mut: u32|
                    )
                ));
        }
    }

    /// A map that reports whether the element is a new maximum: the *state update*
    /// (`max`) is idempotent, but the retried element emits `false` where the original
    /// emitted `true`, so the output half of the obligation must reject it.
    #[cfg(feature = "reject_is_new_map")]
    pub mod is_new_map {
        use hydro_lang::live_collections::stream::AtLeastOnce;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let max_seen = process
                .source_iter(q!(0..5u32))
                .fold(q!(|| 0u32), q!(|acc: &mut u32, x| *acc = x));

            let max_mut = max_seen.by_mut();

            let _out = process
                .source_iter(q!(vec![1u32, 2, 3]))
                .weaken_retries::<AtLeastOnce>()
                .map(q!(
                    |x| {
                        let is_new = x > *max_mut;
                        if is_new {
                            *max_mut = x;
                        }
                        is_new
                    },
                    idempotent = verus_proof_idempotent_map!(
                        item = u32,
                        captures_mut = |max_mut: u32|
                    )
                ));
        }
    }

    /// A filter that retains only elements it has seen before: the state update is
    /// idempotent, but a retried element is retained where the original was not, so
    /// the decision half of the obligation must reject it.
    #[cfg(feature = "reject_keep_retries_filter")]
    pub mod keep_retries_filter {
        use hydro_lang::live_collections::stream::AtLeastOnce;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let max_seen = process
                .source_iter(q!(0..5u32))
                .fold(q!(|| 0u32), q!(|acc: &mut u32, x| *acc = x));

            let max_mut = max_seen.by_mut();

            let _out = process
                .source_iter(q!(vec![1u32, 2, 3]))
                .weaken_retries::<AtLeastOnce>()
                .filter(q!(
                    |x| {
                        let seen = *x <= *max_mut;
                        if *x > *max_mut {
                            *max_mut = *x;
                        }
                        seen
                    },
                    idempotent = verus_proof_idempotent_filter!(
                        item = &u32,
                        captures_mut = |max_mut: u32|
                    )
                ));
        }
    }

    /// Toggling a captured boolean is not idempotent: applying it twice undoes it.
    #[cfg(feature = "reject_toggle_effect")]
    pub mod toggle_effect {
        use hydro_lang::live_collections::stream::AtLeastOnce;
        use hydro_lang::prelude::*;

        pub fn flow<'a>(process: &Process<'a, ()>) {
            let parity = process
                .source_iter(q!(vec![false]))
                .fold(q!(|| false), q!(|acc: &mut bool, x| *acc = *acc != x));

            let parity_mut = parity.by_mut();

            process
                .source_iter(q!(vec![1u32, 2, 3]))
                .weaken_retries::<AtLeastOnce>()
                .for_each(q!(
                    |_x| {
                        *parity_mut = !*parity_mut;
                    },
                    idempotent = verus_proof_idempotent_effect!(
                        item = u32,
                        captures_mut = |parity_mut: bool|
                    )
                ));
        }
    }
}
