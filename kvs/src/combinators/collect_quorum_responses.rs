//! Gather one response per participant until a quorum is reached.

use std::collections::HashMap;
use std::hash::Hash;

use hydro_lang::live_collections::keyed_singleton::BoundedValue;
use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::Location;
use hydro_lang::prelude::*;

/// Collects one response per `(request, participant)` and emits each request
/// exactly once when at least `min` distinct participants have responded.
///
/// The input is a `BoundedValue` keyed singleton, which encodes that each
/// participant contributes at most one immutable response to each request. The
/// output retains participant identity in a `HashMap`; callers can therefore
/// combine replica values without relying on arrival order, and duplicate
/// messages cannot inflate the quorum.
///
/// Partial responses are persisted across slice boundaries. Responses arriving
/// after quorum are absorbed until all `max` participants have responded, so a
/// request is never emitted twice.
pub fn collect_quorum_responses<'a, L, K, P, V>(
    responses: KeyedSingleton<(K, P), V, L, BoundedValue>,
    min: usize,
    max: usize,
    nondet_quorum: NonDet,
) -> Stream<(K, HashMap<P, V>), L::DropConsistency, Unbounded, NoOrder>
where
    L: Location<'a>,
    K: Clone + Eq + Hash,
    P: Clone + Eq + Hash,
    V: Clone,
{
    assert!(min > 0, "quorum minimum must be positive");
    assert!(min <= max, "quorum minimum cannot exceed maximum");

    let responses = responses
        .entries()
        .map(q!(|((request, participant), value)| (
            request,
            (participant, value)
        )));

    let quorums = sliced! {
        let new_inputs = use(responses, nondet!(
            /// Partial responses are persisted, but which participants are in
            /// the first quorum is visible in the output map and is captured by
            /// the caller's declared quorum non-determinism.
            nondet_quorum
        ));
        let mut not_complete =
            use::state_null::<Stream<(K, (P, V)), _, Bounded, NoOrder>>();
        let mut already_emitted = use::state_null::<Stream<K, _, Bounded, NoOrder>>();

        let current = not_complete.chain(new_inputs);

        // Inserting disjoint participant keys is commutative. The BoundedValue
        // input guarantees each (request, participant) pair appears only once.
        let aggregated = current.clone().into_keyed().fold(
            q!(|| HashMap::new()),
            q!(
                |responses, (participant, value)| {
                    responses.insert(participant, value);
                },
                commutative = manual_proof!(
                    /** insertions for distinct participant keys commute */
                )
            ),
        );

        let reached_quorum = aggregated
            .clone()
            .entries()
            .filter_map(q!(move |(key, responses)| if responses.len() >= min {
                Some((key, responses))
            } else {
                None
            }));

        let received_from_all = aggregated
            .entries()
            .filter_map(q!(move |(key, responses)| if responses.len() >= max {
                Some(key)
            } else {
                None
            }));

        // Keep persisting responses until every replica has reported.
        not_complete = current.anti_join(received_from_all.clone());

        // Emit keys that just reached quorum (not already emitted before).
        let out = reached_quorum.clone().anti_join(already_emitted);

        // Suppress keys that reached quorum but haven't yet received all
        // replies, so a later batch doesn't re-emit them.
        already_emitted = reached_quorum
            .map(q!(|(key, _)| key))
            .filter_not_in(received_from_all);

        out
    };

    quorums
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use hydro_lang::live_collections::keyed_singleton::BoundedValue;
    use hydro_lang::live_collections::stream::{NoOrder, TotalOrder};
    use hydro_lang::prelude::*;

    use super::collect_quorum_responses;

    fn unique_responses(
        input: Stream<((u32, u32), u32), Process<'_>, Unbounded, NoOrder>,
    ) -> KeyedSingleton<(u32, u32), u32, Process<'_>, BoundedValue> {
        input
            .into_keyed()
            .assume_ordering::<TotalOrder>(nondet!(
                /// The test sends one response per request and participant.
            ))
            .first()
    }

    #[test]
    fn emits_once_at_quorum_with_participant_values() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (send, input) = node.sim_input::<((u32, u32), u32), NoOrder, _>();
        let out = collect_quorum_responses(
            unique_responses(input),
            2,
            3,
            nondet!(
                /// The test accepts any two or more of the three participant responses.
            ),
        )
        .map(q!(|(request, responses)| {
            let mut responses = responses.into_iter().collect::<Vec<_>>();
            responses.sort();
            (request, responses)
        }))
        .sim_output();

        flow.sim().fuzz(async || {
            send.send_many_unordered([
                ((1, 0), 10),
                ((1, 1), 11),
                ((1, 2), 12),
                ((2, 0), 20),
                ((2, 1), 21),
                ((3, 0), 30),
            ]);

            let results = out.collect_sorted::<Vec<_>>().await;
            let keys: Vec<u32> = results.iter().map(|(k, _)| *k).collect();
            assert_eq!(keys, vec![1, 2], "keys 1 and 2 emitted once; key 3 never");

            for (key, responses) in results {
                assert!(
                    responses.len() >= 2,
                    "key {key} emitted with {} responses, expected >= 2",
                    responses.len()
                );
                let allowed: HashMap<u32, u32> = match key {
                    1 => HashMap::from([(0, 10), (1, 11), (2, 12)]),
                    2 => HashMap::from([(0, 20), (1, 21)]),
                    _ => unreachable!(),
                };
                for (participant, value) in responses {
                    assert_eq!(allowed.get(&participant), Some(&value));
                }
            }
        });
    }

    /// A key that never reaches `min` responses is never emitted.
    #[test]
    fn no_emit_below_quorum() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (send, input) = node.sim_input::<((u32, u32), u32), NoOrder, _>();
        let out = collect_quorum_responses(
            unique_responses(input),
            3,
            3,
            nondet!(
                /// No output is produced, so participant arrival timing is unobservable.
            ),
        )
        .sim_output();

        flow.sim().exhaustive(async || {
            send.send_many_unordered([((1, 0), 100), ((1, 1), 101)]);
            out.assert_yields_only_unordered([]).await;
        });
    }
}
