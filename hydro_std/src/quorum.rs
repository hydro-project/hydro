use std::hash::Hash;

use hydro_lang::live_collections::stream::{NoOrder, Ordering};
use hydro_lang::location::{Location, NoTick};
use hydro_lang::prelude::*;

#[expect(clippy::type_complexity, reason = "stream types with ordering")]
pub fn collect_quorum_with_response<
    'a,
    L: Location<'a> + NoTick,
    Order: Ordering,
    K: Clone + Eq + Hash,
    V: Clone,
    E: Clone,
>(
    responses: Stream<(K, Result<V, E>), L, Unbounded, Order>,
    min: usize,
    max: usize,
) -> (
    Stream<(K, V), L, Unbounded, Order>,
    Stream<(K, E), L, Unbounded, Order>,
) {
    let tick = responses.location().tick();
    let (not_all_complete_cycle, not_all) = tick.cycle::<Stream<_, _, _, Order>>();

    let current_responses = not_all.chain(responses.clone().batch(
        &tick,
        nondet!(
            /// We always persist values that have not reached quorum, so even
            /// with arbitrary batching we always produce deterministic quorum results.
        ),
    ));

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
        let (min_but_not_max_complete_cycle, min_but_not_max) =
            tick.cycle::<Stream<K, _, Bounded, NoOrder>>();

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
            .all_ticks(),
        responses.filter_map(q!(move |(key, res)| match res {
            Ok(_) => None,
            Err(e) => Some((key, e)),
        })),
    )
}

#[expect(clippy::type_complexity, reason = "stream types with ordering")]
pub fn collect_quorum<
    'a,
    L: Location<'a> + NoTick,
    Order: Ordering,
    K: Clone + Eq + Hash,
    E: Clone,
>(
    responses: Stream<(K, Result<(), E>), L, Unbounded, Order>,
    min: usize,
    max: usize,
) -> (
    Stream<K, L, Unbounded, NoOrder>,
    Stream<(K, E), L, Unbounded, Order>,
) {
    let tick = responses.location().tick();
    let (not_all_complete_cycle, not_all) = tick.cycle::<Stream<_, _, _, Order>>();

    let current_responses = not_all.chain(responses.clone().batch(
        &tick,
        nondet!(
            /// We always persist values that have not reached quorum, so even
            /// with arbitrary batching we always produce deterministic quorum results.
        ),
    ));

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
        just_reached_quorum.all_ticks(),
        responses.filter_map(q!(move |(key, res)| match res {
            Ok(_) => None,
            Err(e) => Some((key, e)),
        })),
    )
}

#[cfg(test)]
mod tests {
    use hydro_lang::live_collections::stream::{NoOrder, TotalOrder};
    use hydro_lang::prelude::*;

    use super::{collect_quorum, collect_quorum_with_response};

    #[test]
    fn collect_quorum_with_response_preserves_order() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode(&external);
        let out_port = collect_quorum_with_response(input, 3, 3)
            .0
            .send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let out_recv = compiled.connect(&out_port);
            compiled.launch();

            in_send.send((1, Ok::<(), ()>(())));
            in_send.send((1, Ok(())));
            in_send.send((1, Ok(())));
            in_send.send((2, Ok(())));
            in_send.send((2, Ok(())));
            in_send.send((3, Ok(())));
            in_send.send((3, Ok(())));
            in_send.send((3, Ok(())));

            assert_eq!(
                out_recv.collect::<Vec<_>>().await,
                vec![(1, ()), (1, ()), (1, ()), (3, ()), (3, ()), (3, ())]
            )
        });
    }

    #[test]
    fn collect_quorum_with_response_no_order() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode::<_, _, NoOrder, _>(&external);
        let out_port = collect_quorum_with_response(input, 2, 2)
            .0
            .send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let out_recv = compiled.connect(&out_port);
            compiled.launch();

            in_send
                .send_many_unordered([
                    (1, Ok::<(), ()>(())),
                    (1, Ok(())),
                    (2, Ok(())),
                    (3, Ok(())),
                    (3, Ok(())),
                ])
                .unwrap();

            out_recv
                .assert_yields_only_unordered([(1, ()), (1, ()), (3, ()), (3, ())])
                .await;
        });
    }

    #[test]
    fn collect_quorum_functionality() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode(&external);
        let (success_port, error_port) = {
            let (success, error) = collect_quorum(input, 2, 3);
            (
                success.send_bincode_external(&external),
                error.send_bincode_external(&external),
            )
        };

        let compiled_sim = flow.sim().compiled();

        // Test case 1: Key reaches exact minimum quorum (2/3)
        compiled_sim.exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let success_recv = compiled.connect(&success_port);
            let error_recv = compiled.connect(&error_port);
            compiled.launch();

            in_send.send((1, Ok::<(), ()>(())));
            in_send.send((1, Ok(())));

            success_recv.assert_yields_only_unordered([1]).await;
            error_recv.assert_no_more().await;
        });

        // Test case 2: Key reaches maximum responses with mixed results (2 success, 1 error)
        compiled_sim.exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let success_recv = compiled.connect(&success_port);
            let error_recv = compiled.connect(&error_port);
            compiled.launch();

            in_send.send((2, Ok::<(), ()>(())));
            in_send.send((2, Ok(())));
            in_send.send((2, Err(())));

            success_recv.assert_yields_only_unordered([2]).await;
            error_recv.assert_yields_only([(2, ())]).await;
        });

        // Test case 3: Key doesn't reach quorum (1 success, 2 errors)
        compiled_sim.exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let success_recv = compiled.connect(&success_port);
            let error_recv = compiled.connect(&error_port);
            compiled.launch();

            in_send.send((3, Ok::<(), ()>(())));
            in_send.send((3, Err(())));
            in_send.send((3, Err(())));

            success_recv.assert_no_more().await;
            error_recv.assert_yields_only([(3, ()), (3, ())]).await;
        });

        // Test case 4: Key reaches quorum with extra responses
        compiled_sim.exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let success_recv = compiled.connect(&success_port);
            let error_recv = compiled.connect(&error_port);
            compiled.launch();

            in_send.send((4, Ok::<(), ()>(())));
            in_send.send((4, Ok(())));
            in_send.send((4, Ok(()))); // This should be ignored after quorum

            success_recv.assert_yields_only_unordered([4]).await;
            error_recv.assert_no_more().await;
        });

        // Test case 5: Key with only errors (no quorum)
        compiled_sim.exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let success_recv = compiled.connect(&success_port);
            let error_recv = compiled.connect(&error_port);
            compiled.launch();

            in_send.send((5, Err::<(), ()>(())));
            in_send.send((5, Err(())));
            in_send.send((5, Err(())));

            success_recv.assert_no_more().await;
            error_recv
                .assert_yields_only([(5, ()), (5, ()), (5, ())])
                .await;
        });

        // Test case 6: Key that reaches quorum exactly at max (2 success, 1 error)
        compiled_sim.exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let success_recv = compiled.connect(&success_port);
            let error_recv = compiled.connect(&error_port);
            compiled.launch();

            in_send.send((6, Err::<(), ()>(())));
            in_send.send((6, Ok(())));
            in_send.send((6, Ok(())));

            success_recv.assert_yields_only_unordered([6]).await;
            error_recv.assert_yields_only([(6, ())]).await;
        });
    }

    #[test]
    fn collect_quorum_min_equals_max() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode::<_, _, TotalOrder, _>(&external);
        let success_port = collect_quorum(input, 2, 2)
            .0
            .send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let success_recv = compiled.connect(&success_port);
            compiled.launch();

            // When min == max, we need exactly that many responses
            in_send.send((1, Ok::<(), ()>(())));
            in_send.send((1, Ok(())));

            // This key gets exactly 2 responses (1 success, 1 error) - should not reach quorum
            in_send.send((2, Ok(())));
            in_send.send((2, Err(())));

            // This key gets 2 successes - should reach quorum
            in_send.send((3, Ok(())));
            in_send.send((3, Ok(())));

            // Only keys 1 and 3 should reach quorum (both have 2 successes)
            success_recv.assert_yields_only_unordered([1, 3]).await;
        });
    }

    #[test]
    fn collect_quorum_single_response() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode::<_, _, TotalOrder, _>(&external);
        let success_port = collect_quorum(input, 1, 1)
            .0
            .send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let success_recv = compiled.connect(&success_port);
            compiled.launch();

            // With min=max=1, any single success should immediately reach quorum
            in_send.send((1, Ok::<(), ()>(())));
            in_send.send((2, Err(())));
            in_send.send((3, Ok(())));

            // Keys 1 and 3 should reach quorum immediately
            success_recv.assert_yields_only_unordered([1, 3]).await;
        });
    }

    #[test]
    fn collect_quorum_no_responses() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode::<_, _, TotalOrder, _>(&external);
        let success_port = {
            let (success, _error) = collect_quorum::<_, _, i32, ()>(input, 2, 3);
            success.send_bincode_external(&external)
        };

        flow.sim().exhaustive(async |mut compiled| {
            let _in_send = compiled.connect(&port);
            let success_recv = compiled.connect(&success_port);
            compiled.launch();

            // No responses sent - should get empty results
            success_recv.assert_no_more().await;
        });
    }

    #[test]
    fn collect_quorum_no_double_quorum_before_max() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode::<_, _, TotalOrder, _>(&external);
        let success_port = collect_quorum(input, 2, 4)
            .0
            .send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let success_recv = compiled.connect(&success_port);
            compiled.launch();

            // Key 1: First reaches quorum with 2 successes
            in_send.send((1, Ok::<(), ()>(())));
            in_send.send((1, Ok(())));

            // Key 1: Additional responses after quorum - should not trigger quorum again
            in_send.send((1, Ok(())));
            in_send.send((1, Ok(())));

            // Key 2: Reaches quorum later with mixed responses
            in_send.send((2, Err(())));
            in_send.send((2, Ok(())));
            in_send.send((2, Ok(())));
            in_send.send((2, Err(()))); // Additional error after quorum

            // Each key should appear exactly once, even though they received
            // additional responses after reaching quorum
            success_recv.assert_yields_only_unordered([1, 2]).await;
        });
    }
}
