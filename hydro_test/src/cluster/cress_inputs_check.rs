//! The attribute's handling of the two input shapes the first checks outside the corpus ran into:
//! a `KeyedStream` parameter, which the harness feeds as `(K, V)` pairs and converts with
//! `.into_keyed()`, and a `NoOrder` stream parameter, whose simulator sender has only
//! `send_many_unordered`. The program is a pure request/response counter, so the check is a
//! benign control; what is tested is that the generated harness compiles and runs.
//!
//! Runs with `cargo test -p hydro_test --features cress --lib cress_inputs_check`; the module is
//! compiled only with the `cress` feature, which links the checker.

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;

pub struct Counter;

/// Counts increments and answers unordered get requests with the count seen so far.
#[cress::amplification_check(name = keyed_and_unordered)]
pub fn keyed_counter_with_unordered_gets<'a>(
    increments: KeyedStream<u32, (), Process<'a, Counter>>,
    gets: Stream<u32, Process<'a, Counter>, Unbounded, NoOrder>,
) -> (
    KeyedStream<u32, (), Process<'a, Counter>>,
    Stream<(u32, usize), Process<'a, Counter>, Unbounded, NoOrder>,
) {
    let processing = increments.atomic();
    let count = processing.clone().values().count();
    let acks = processing.end_atomic();
    let responses = sliced! {
        let gets = use::batch(gets, nondet!(/** the checker drives this */));
        let count = use::atomic(count, nondet!(/** atomic with the increments */));
        gets.cross_singleton(count)
    };
    (acks, responses)
}

#[cfg(test)]
mod tests {
    use cress::{Verdict, harness};

    #[test]
    fn keyed_and_unordered_inputs_are_fed_by_the_generated_harness() {
        let results = harness::run_registered_checks(
            Some("keyed_counter_with_unordered_gets"),
            None,
            false,
        )
        .expect("the attribute registered a check");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].configuration, "keyed_and_unordered");
        assert!(
            matches!(results[0].verdict, Verdict::Benign),
            "{:?}",
            results[0]
        );
    }
}
