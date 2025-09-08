use std::hash::Hash;

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::{Atomic, Location, NoTick};
use hydro_lang::prelude::*;

#[expect(clippy::type_complexity, reason = "stream types with ordering")]
pub fn collect_quorum_with_response<
    'a,
    L: Location<'a> + NoTick,
    Order,
    K: Clone + Eq + Hash,
    V: Clone,
    E: Clone,
>(
    responses: Stream<(K, Result<V, E>), Atomic<L>, Unbounded, Order>,
    min: usize,
    max: usize,
) -> (
    Stream<(K, V), Atomic<L>, Unbounded, Order>,
    Stream<(K, E), Atomic<L>, Unbounded, Order>,
) {
    let tick = responses.atomic_source();
    let (not_all_complete_cycle, not_all) = tick.cycle::<Stream<_, _, _, Order>>();

    let current_responses = not_all.chain(responses.clone().batch(nondet!(
        /// We always persist values that have not reached quorum, so even
        /// with arbitrary batching we always produce deterministic quorum results.
    )));

    let count_per_key = current_responses.clone().into_keyed().fold_commutative(
        q!(move || (0, 0)),
        q!(move |accum, value| {
            if value.is_ok() {
                accum.0 += 1;
            } else {
                accum.1 += 1;
            }
        }),
    );

    let not_reached_min_count = count_per_key
        .clone()
        .filter(q!(move |(success, _error)| success < &min))
        .keys();

    let reached_min_count = count_per_key
        .clone()
        .filter(q!(move |(success, _error)| success >= &min))
        .keys();

    let just_reached_quorum = if max == min {
        not_all_complete_cycle
            .complete_next_tick(current_responses.clone().anti_join(reached_min_count));

        current_responses.anti_join(not_reached_min_count)
    } else {
        let (min_but_not_max_complete_cycle, min_but_not_max) = tick.cycle();

        let received_from_all = count_per_key
            .filter(q!(move |(success, error)| (success + error) >= max))
            .keys();

        min_but_not_max_complete_cycle
            .complete_next_tick(reached_min_count.filter_not_in(received_from_all.clone()));

        not_all_complete_cycle
            .complete_next_tick(current_responses.clone().anti_join(received_from_all));

        current_responses
            .anti_join(not_reached_min_count)
            .anti_join(min_but_not_max)
    };

    (
        just_reached_quorum
            .filter_map(q!(move |(key, res)| match res {
                Ok(v) => Some((key, v)),
                Err(_) => None,
            }))
            .all_ticks_atomic(),
        responses.filter_map(q!(move |(key, res)| match res {
            Ok(_) => None,
            Err(e) => Some((key, e)),
        })),
    )
}

#[expect(clippy::type_complexity, reason = "stream types with ordering")]
pub fn collect_quorum<'a, L: Location<'a> + NoTick, Order, K: Clone + Eq + Hash, E: Clone>(
    responses: Stream<(K, Result<(), E>), Atomic<L>, Unbounded, Order>,
    min: usize,
    max: usize,
) -> (
    Stream<K, Atomic<L>, Unbounded, NoOrder>,
    Stream<(K, E), Atomic<L>, Unbounded, Order>,
) {
    let tick = responses.atomic_source();
    let (not_all_complete_cycle, not_all) = tick.cycle::<Stream<_, _, _, Order>>();

    let current_responses = not_all.chain(responses.clone().batch(nondet!(
        /// We always persist values that have not reached quorum, so even
        /// with arbitrary batching we always produce deterministic quorum results.
    )));

    let count_per_key = current_responses.clone().into_keyed().fold_commutative(
        q!(move || (0, 0)),
        q!(move |accum, value| {
            if value.is_ok() {
                accum.0 += 1;
            } else {
                accum.1 += 1;
            }
        }),
    );

    let reached_min_count =
        count_per_key
            .clone()
            .entries()
            .filter_map(q!(move |(key, (success, _error))| if success >= min {
                Some(key)
            } else {
                None
            }));

    let just_reached_quorum = if max == min {
        not_all_complete_cycle.complete_next_tick(
            current_responses
                .clone()
                .anti_join(reached_min_count.clone()),
        );

        reached_min_count
    } else {
        let (min_but_not_max_complete_cycle, min_but_not_max) = tick.cycle();

        let received_from_all = count_per_key
            .filter(q!(move |(success, error)| (success + error) >= max))
            .keys();

        min_but_not_max_complete_cycle.complete_next_tick(
            reached_min_count
                .clone()
                .filter_not_in(received_from_all.clone()),
        );

        not_all_complete_cycle.complete_next_tick(current_responses.anti_join(received_from_all));

        reached_min_count.filter_not_in(min_but_not_max)
    };

    (
        just_reached_quorum.all_ticks_atomic(),
        responses.filter_map(q!(move |(key, res)| match res {
            Ok(_) => None,
            Err(e) => Some((key, e)),
        })),
    )
}
