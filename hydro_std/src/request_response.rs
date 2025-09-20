use std::hash::Hash;

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::{Atomic, Location, NoTick};
use hydro_lang::prelude::*;

type JoinResponses<K, M, V, L> = Stream<(K, (M, V)), Atomic<L>, Unbounded, NoOrder>;

/// Given an incoming stream of request-response responses, joins with metadata generated
/// at request time that is stored in-memory.
///
/// The metadata must be generated in the same or a previous tick than the response,
/// typically at request time. Only one response element should be produced with a given
/// key, same for the metadata stream.
pub fn join_responses<'a, K: Clone + Eq + Hash, M: Clone, V: Clone, L: Location<'a> + NoTick>(
    tick: &Tick<L>,
    responses: Stream<(K, V), Atomic<L>, Unbounded, NoOrder>,
    metadata: Stream<(K, M), Tick<L>, Bounded, NoOrder>,
) -> JoinResponses<K, M, V, L> {
    let (remaining_to_join_complete_cycle, remaining_to_join) =
        tick.cycle::<Stream<_, _, _, NoOrder>>();

    let remaining_and_new: Stream<(K, M), Tick<L>, Bounded, _> = remaining_to_join.chain(metadata);

    let responses = responses.batch_atomic(nondet!(
        /// Because we persist the metadata, delays resulting from
        /// batching boundaries do not affect the output contents.
    ));

    // TODO(shadaj): we should have a "split-join" operator
    // that returns both join and anti-join without cloning
    let joined_this_tick =
        remaining_and_new
            .clone()
            .join(responses.clone())
            .map(q!(|(key, (meta, resp))| (key, (meta, resp))));

    remaining_to_join_complete_cycle
        .complete_next_tick(remaining_and_new.anti_join(responses.map(q!(|(key, _)| key))));

    joined_this_tick.all_ticks_atomic()
}
