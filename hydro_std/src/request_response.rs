use std::hash::Hash;

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::Location;
use hydro_lang::prelude::*;

type JoinResponses<K, M, V, L> = Stream<(K, (M, V)), L, Unbounded, NoOrder>;

/// Given an incoming stream of request-response responses, joins with metadata generated
/// at request time that is stored in-memory.
///
/// The metadata must be generated in the same or a previous tick than the response,
/// typically at request time. Only one response element should be produced with a given
/// key, same for the metadata stream.
pub fn join_responses<'a, K: Clone + Eq + Hash, M: Clone, V: Clone, L: Location<'a>>(
    responses: Stream<(K, V), L, Unbounded, NoOrder>,
    metadata: Stream<(K, M), Tick<L>, Bounded, NoOrder>,
) -> JoinResponses<K, M, V, L::DropConsistency> {
    sliced! {
        let mut remaining_to_join = use::state_null::<Stream<(K, M), _, _, NoOrder>>();

        let response_batch = use(responses, nondet!(
            /// Because we persist the metadata, delays resulting from
            /// batching boundaries do not affect the output contents.
        ));
        let metadata_batch = use::atomic(metadata.all_ticks_atomic(), nondet!(
            /// Metadata is synchronized with the tick input.
        ));

        let remaining_and_new = remaining_to_join.chain(metadata_batch);

        // TODO(shadaj): we should have a "split-join" operator
        // that returns both join and anti-join without cloning
        let joined_this_tick = remaining_and_new
            .clone()
            .join(response_batch.clone())
            .map(q!(|(key, (meta, resp))| (key, (meta, resp))));

        remaining_to_join = remaining_and_new.anti_join(response_batch.map(q!(|(key, _)| key)));

        joined_this_tick
    }
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    use super::*;

    /// Test that join_responses correctly joins metadata with responses.
    #[test]
    fn test_join_responses_basic() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        // Set up inputs (TotalOrder for deterministic simulation)
        let (response_send, responses) = process.sim_input::<(u32, String), _, _>();
        let (metadata_send, metadata_processing) = process.sim_atomic_input::<(u32, i32), _, _>();

        let metadata = metadata_processing
            .batch_atomic(&process.tick(), nondet!(/** test */))
            .weaken_ordering();

        // Join responses with metadata (weaken ordering for join_responses)
        let joined = join_responses(responses.weaken_ordering(), metadata);

        // Set up outputs
        let joined_recv = joined.sim_output();

        flow.sim().exhaustive(async || {
            // Send metadata first
            metadata_send.send_atomic((1, 42));

            // Now send response
            response_send.send((1, "hello".to_owned()));
            // Should get joined result
            joined_recv
                .assert_yields_unordered([(1, (42, "hello".to_owned()))])
                .await;
        });
    }

    /// Test that metadata persists across ticks until matched with a response.
    #[test]
    fn test_join_responses_metadata_persists() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (response_send, responses) = process.sim_input::<(u32, String), _, _>();
        let (metadata_send, metadata_processing) = process.sim_atomic_input::<(u32, i32), _, _>();

        let metadata = metadata_processing
            .batch_atomic(&process.tick(), nondet!(/** test */))
            .weaken_ordering();

        let joined = join_responses(responses.weaken_ordering(), metadata);

        let joined_recv = joined.sim_output();

        flow.sim().exhaustive(async || {
            // Send multiple metadata entries
            metadata_send.send_many_atomic([(1, 10), (2, 20)]);

            // Send responses for both keys
            response_send.send_many([(2, "two".to_owned()), (1, "one".to_owned())]);
            joined_recv
                .assert_yields_only_unordered([
                    (1, (10, "one".to_owned())),
                    (2, (20, "two".to_owned())),
                ])
                .await;
        });
    }

    /// Test that responses without metadata are not emitted.
    #[test]
    fn test_join_responses_no_metadata() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (response_send, responses) = process.sim_input::<(u32, String), _, _>();
        let (metadata_send, metadata_processing) = process.sim_atomic_input::<(u32, i32), _, _>();

        let metadata = metadata_processing
            .batch_atomic(&process.tick(), nondet!(/** test */))
            .weaken_ordering();

        let joined = join_responses(responses.weaken_ordering(), metadata);

        let joined_recv = joined.sim_output();

        flow.sim().exhaustive(async || {
            // Send metadata for key 1
            metadata_send.send_atomic((1, 42));

            // Send responses for key 1 (has metadata) and key 2 (no metadata)
            response_send.send_many([(1, "matched".to_owned()), (2, "unmatched".to_owned())]);

            // Only key 1 should produce output
            joined_recv
                .assert_yields_only_unordered([(1, (42, "matched".to_owned()))])
                .await;
        });
    }

    /// Test that metadata is removed after being matched with a response.
    #[test]
    fn test_join_responses_metadata_removed_after_match() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (response_send, responses) = process.sim_input::<(u32, String), _, _>();
        let (metadata_send, metadata_processing) = process.sim_atomic_input::<(u32, i32), _, _>();

        let metadata = metadata_processing
            .batch_atomic(&process.tick(), nondet!(/** test */))
            .weaken_ordering();

        let joined = join_responses(responses.weaken_ordering(), metadata);

        let joined_recv = joined.sim_output();

        flow.sim().exhaustive(async || {
            // Send metadata for key 1
            metadata_send.send_atomic((1, 42));

            // First response for key 1 should match
            response_send.send((1, "first".to_owned()));
            joined_recv
                .assert_yields_unordered([(1, (42, "first".to_owned()))])
                .await;

            // Second response for key 1 should be dropped (metadata already consumed)
            response_send.send((1, "second".to_owned()));
            joined_recv.assert_no_more().await;
        });
    }
}
