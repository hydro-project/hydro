//! Rendezvous-hash scatter — a reusable, protocol-agnostic routing combinator.

use std::hash::{DefaultHasher, Hash, Hasher};

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::MemberId;
use hydro_lang::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Rendezvous-hash scatter.
///
/// Given a stream of `(routing_key, payload)` pairs at `Src` and a target
/// `Dest` cluster, routes each payload to the top-`replication_factor` members
/// of `Dest` chosen by a highest-random-weight (rendezvous) hash of
/// `(routing_key, member)`. The same routing key always yields the same ranking,
/// so repeated requests for a key target the same members.
///
/// This is deliberately generic — it knows nothing about keys, values, or
/// operations. The caller projects its requests down to just the routing key
/// plus whatever payload it wants delivered, so the same combinator serves any
/// sharded/replicated protocol. Hydro lets us factor out this *pattern* on its
/// own, independent of where the code runs, rather than splitting along a
/// network boundary.
///
/// Targeting is driven by the cluster's *live* membership (rather than a fixed
/// `0..N` range of synthetic ids) so that the resulting `MemberId`s address real
/// cluster members — required for real (e.g. containerized) deployments.
///
/// Requests are buffered until at least `expected_members` destination members
/// are visible. Also returns, per source member, the membership count derived
/// from the same snapshot used for routing.
pub fn hrw_scatter<'a, Src, Dest, K, T>(
    targets: &Cluster<'a, Dest>,
    requests: Stream<(K, T), Cluster<'a, Src>, Unbounded, NoOrder>,
    replication_factor: usize,
    expected_members: usize,
) -> (
    Stream<(MemberId<Dest>, T), Cluster<'a, Src>, Unbounded, NoOrder>,
    Stream<usize, Cluster<'a, Src>, Unbounded, NoOrder>,
)
where
    Src: 'a,
    Dest: 'a,
    K: Clone + Hash + Serialize + DeserializeOwned + 'a,
    T: Clone + Serialize + DeserializeOwned + 'a,
{
    assert!(
        replication_factor <= expected_members,
        "replication factor cannot exceed expected membership"
    );

    let member_presence = hydro_std::membership::track_membership(
        requests.location().source_cluster_membership_stream(
            targets,
            nondet!(/** late joiners may miss events */),
        ),
    );

    sliced! {
        let member_snapshot = use(member_presence, nondet!(
            /// Routing uses whichever members are currently live; membership is
            /// assumed stable for the duration of a request.
        ));
        let request_batch = use(requests, nondet!(
            /// Batch boundaries are not observable: each request is routed
            /// independently against the same membership snapshot.
        ));
        let mut pending_requests =
            use::state_null::<Stream<(K, T), _, Bounded, NoOrder>>();

        // The set of target members currently present, as a single value we can
        // pair with each request. Both the routing below and the membership
        // count reported out are derived from this one snapshot, so they cannot
        // disagree.
        let live_members = member_snapshot
            .filter(q!(|present| *present))
            .keys()
            .assume_ordering::<hydro_lang::live_collections::stream::TotalOrder>(nondet!(
                /// Member order only affects HRW tie-breaking, which we make
                /// deterministic by sorting on the member id below.
            ))
            .collect_vec();

        let ready = live_members
            .clone()
            .map(q!(move |members| members.len() >= expected_members));
        let current_requests = pending_requests.chain(request_batch);

        // Retain requests while membership is incomplete. A membership update
        // re-runs the slice and releases the buffered requests once ready.
        pending_requests = current_requests
            .clone()
            .filter_if(ready.clone().map(q!(|ready| !ready)));

        let routed = current_requests
            .filter_if(ready)
            .cross_singleton(live_members.clone())
            .flat_map_unordered(q!(move |((key, payload), members)| {
                // Rank the live members by HRW weight for this key. (The member
                // type is left inferred so the quoted closure doesn't name the
                // caller's generic parameter, which stageleft codegen can't see.)
                let mut weighted: Vec<(u64, _)> = members
                    .into_iter()
                    .map(|member| {
                        let mut hasher = DefaultHasher::new();
                        key.hash(&mut hasher);
                        member.hash(&mut hasher);
                        (hasher.finish(), member)
                    })
                    .collect();
                // Highest weight first; break ties by member id for determinism.
                weighted.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                weighted
                    .into_iter()
                    .take(replication_factor)
                    .map(move |(_, member)| (member, payload.clone()))
                    .collect::<Vec<_>>()
            }));

        // Report the number of target members each source member currently sees.
        // The slice re-runs whenever membership changes (not only on requests),
        // so this count is emitted as the cluster is discovered.
        let member_count = live_members
            .map(q!(|members| members.len()))
            .into_stream()
            .weaken_ordering::<NoOrder>();

        (routed, member_count)
    }
}

#[cfg(test)]
mod tests {
    use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};
    use hydro_lang::prelude::*;

    use super::hrw_scatter;

    struct Src;
    struct Dest;

    /// With 5 target members and a replication factor of 3, each routed key
    /// should land on exactly 3 distinct members, and routing the same key
    /// twice should pick the same 3 members (rendezvous stability).
    #[test]
    fn routes_to_replication_factor_distinct_members() {
        let mut flow = FlowBuilder::new();
        let src = flow.cluster::<Src>();
        let dest = flow.cluster::<Dest>();
        let collector = flow.process::<()>();

        let (send, requests) = src.sim_input::<(String, u32), TotalOrder, ExactlyOnce>();
        let (routed, membership) = hrw_scatter(&dest, requests.weaken_ordering(), 3, 5);

        // `routed` holds the routing *decisions* — `(target_member, payload)`
        // pairs, still located at the source (we don't demux to `dest` here).
        // Project to `(payload, target_member_raw_id)` and gather everything at a
        // single collector process so we can read the full picture with one
        // `collect`.
        let received = routed
            .map(q!(|(member, payload)| (payload, member.get_raw_id())))
            .send(&collector, TCP.fail_stop().bincode().name("to_collector"))
            .values()
            .sim_output();
        let membership = membership
            .assume_ordering::<TotalOrder>(nondet!(/** counts are monotone per member */))
            .sim_cluster_output();

        flow.sim()
            .with_cluster_size(&src, 1)
            .with_cluster_size(&dest, 5)
            .fuzz(async || {
                // Wait until the source has discovered all 5 destination members,
                // so routing ranks over the full set (otherwise an early request
                // could be routed to a partial view).
                loop {
                    match membership.next(0).await {
                        Some(5) => break,
                        Some(_) => continue,
                        None => panic!("source never discovered all 5 target members"),
                    }
                }

                // Send the same key twice (payloads 1 and 2) and a different
                // key, all from source member 0.
                send.send(0, ("apple".to_owned(), 1));
                send.send(0, ("apple".to_owned(), 2));
                send.send(0, ("banana".to_owned(), 9));

                // Gather (payload, dest_member) pairs at the collector.
                let mut recipients_for = std::collections::HashMap::<u32, Vec<u32>>::new();
                for (payload, member) in received.collect_sorted::<Vec<_>>().await {
                    recipients_for.entry(payload).or_default().push(member);
                }

                // Each payload reached exactly 3 distinct members.
                for payload in [1u32, 2, 9] {
                    let mut members = recipients_for.get(&payload).cloned().unwrap_or_default();
                    members.sort();
                    members.dedup();
                    assert_eq!(
                        members.len(),
                        3,
                        "payload {payload} reached members {members:?}, expected 3"
                    );
                }

                // The two requests for "apple" (payloads 1 and 2) picked the same
                // set of members — rendezvous hashing is stable per key.
                let mut apple_1 = recipients_for.get(&1).cloned().unwrap();
                let mut apple_2 = recipients_for.get(&2).cloned().unwrap();
                apple_1.sort();
                apple_2.sort();
                assert_eq!(apple_1, apple_2, "same key routed to different members");
            });
    }

    /// The membership stream reports the number of live target members each
    /// source member has discovered.
    #[test]
    fn reports_target_membership_count() {
        let mut flow = FlowBuilder::new();
        let src = flow.cluster::<Src>();
        let dest = flow.cluster::<Dest>();

        let (_send, requests) = src.sim_input::<(String, u32), TotalOrder, ExactlyOnce>();
        let (_routed, membership) = hrw_scatter(&dest, requests.weaken_ordering(), 3, 4);
        let membership = membership
            .assume_ordering::<TotalOrder>(nondet!(/** counts are monotone per member */))
            .sim_cluster_output();

        flow.sim()
            .with_cluster_size(&src, 1)
            .with_cluster_size(&dest, 4)
            .fuzz(async || {
                // Source member 0 must eventually discover all 4 target members.
                loop {
                    match membership.next(0).await {
                        Some(4) => break,
                        Some(_) => continue,
                        None => panic!("source never discovered all 4 target members"),
                    }
                }
            });
    }
}
