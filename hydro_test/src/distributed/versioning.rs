use hydro_lang::live_collections::keyed_stream::KeyedStream;
use hydro_lang::live_collections::sliced::sliced;
use hydro_lang::live_collections::stream::{ExactlyOnce, NoOrder, TotalOrder};
use hydro_lang::location::MemberId;
use hydro_lang::location::tick::Tick;
use hydro_lang::networking::NetworkFor;
use hydro_lang::nondet::NonDet;
use hydro_lang::prelude::*;
use hydro_lang::properties::StreamMapFuncAlgebra;
use hydro_std::membership::track_membership;
use serde::{Deserialize, Serialize};
use stageleft::IntoQuotedMut;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Request {
    Write { value: usize },
    Read,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Response {
    pub value: usize,
}

pub struct GossipServer {}

/// A second cluster introduced only by version 3, to exercise differing topologies
/// across versions (v1/v2 do not have a `Logger`).
pub struct Logger {}

/// Routes each request to a single `GossipServer` member chosen by `route`, based on a
/// nondeterministic snapshot of the cluster's current membership. Returns the demuxed
/// [`KeyedStream`] received at the target (keyed by the sending member) plus the membership
/// snapshots used for routing.
#[expect(
    clippy::type_complexity,
    reason = "returns tuple with membership stream"
)]
fn hash_demux<'a, F, N: NetworkFor<Request>>(
    requests: Stream<Request, Cluster<'a, GossipServer>, Unbounded, TotalOrder>,
    to: &Cluster<'a, GossipServer>,
    route: impl IntoQuotedMut<'a, F, Tick<Cluster<'a, GossipServer>>, StreamMapFuncAlgebra>,
    via: N,
    nondet_membership: NonDet,
) -> (
    KeyedStream<
        MemberId<GossipServer>,
        Request,
        Cluster<'a, GossipServer>,
        Unbounded,
        N::OrderingGuarantee,
        ExactlyOnce,
    >,
    Stream<Vec<MemberId<GossipServer>>, Cluster<'a, GossipServer>, Unbounded>,
)
where
    F: Fn((Request, Vec<MemberId<GossipServer>>)) -> Option<(MemberId<GossipServer>, Request)> + 'a,
{
    let ids = track_membership(
        requests
            .location()
            .source_cluster_membership_stream(to, nondet_membership),
    );
    let (filtered, members_out) = sliced! {
        let members_snapshot = use(ids, nondet_membership);
        let elements = use(requests, nondet_membership);

        let current_members = members_snapshot
            .filter(q!(|b| *b))
            .keys()
            .assume_ordering::<TotalOrder>(nondet_membership)
            .collect_vec();

        let filtered = elements
            .cross_singleton(current_members.clone())
            .filter_map::<_, _, _, _, false>(route);

        (filtered, current_members.into_stream())
    };

    (filtered.demux(to, via), members_out)
}

/// A server that forwards all requests to one other member (chosen by `route`) via gossip; the
/// target member folds writes into its state and replies.
fn gossip_server<'a, F>(
    requests: Stream<Request, Cluster<'a, GossipServer>, Unbounded, TotalOrder>,
    servers: &Cluster<'a, GossipServer>,
    route: impl IntoQuotedMut<'a, F, Tick<Cluster<'a, GossipServer>>, StreamMapFuncAlgebra>,
) -> (
    Stream<Response, Cluster<'a, GossipServer>, Unbounded, NoOrder>,
    Stream<Vec<MemberId<GossipServer>>, Cluster<'a, GossipServer>, Unbounded>,
)
where
    F: Fn((Request, Vec<MemberId<GossipServer>>)) -> Option<(MemberId<GossipServer>, Request)> + 'a,
{
    let (on_other, membership) = hash_demux(
        requests,
        servers,
        route,
        TCP.fail_stop().bincode().name("gossip"),
        nondet!(/** gossip membership */),
    );

    // The target member processes requests and responds.
    let responses = on_other
        .entries()
        .assume_ordering::<TotalOrder>(nondet!(/** ordering on target */))
        .scan(
            q!(|| 0usize),
            q!(|acc, (sender, req)| {
                match req {
                    Request::Write { value } => {
                        *acc = usize::max(*acc, value);
                    }
                    Request::Read => {}
                }
                Some((sender, Response { value: *acc }))
            }),
        )
        .map(q!(|(sender, response)| (sender, response)))
        .into_keyed()
        .demux(servers, TCP.fail_stop().bincode().name("gossip_response"))
        .values()
        .weaken_ordering();

    (responses, membership)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GossipVersion {
    /// Routes each request to the **last** member in sorted order.
    V1,
    /// Routes each request to the **first** member in sorted order — the incompatible-routing
    /// change from v1 that the multi-version sim is designed to catch.
    V2,
    /// Same gossip routing as v1, but additionally forwards a copy of each request to a brand-new
    /// `Logger` cluster that did not exist in v1 or v2 (a different topology). Requires `loggers`.
    V3,
}

pub fn gossip_server_versioned<'a>(
    version: GossipVersion,
    requests: Stream<Request, Cluster<'a, GossipServer>, Unbounded, TotalOrder>,
    servers: &Cluster<'a, GossipServer>,
    loggers: Option<&Cluster<'a, Logger>>,
) -> (
    Stream<Response, Cluster<'a, GossipServer>, Unbounded, NoOrder>,
    Stream<Vec<MemberId<GossipServer>>, Cluster<'a, GossipServer>, Unbounded>,
) {
    // v3 additionally logs a copy of every request to a brand-new `Logger` cluster.
    if version == GossipVersion::V3 {
        let loggers = loggers.expect("GossipVersion::V3 requires a `Logger` cluster");
        requests
            .clone()
            .broadcast(
                loggers,
                TCP.fail_stop().bincode().name("log"),
                nondet!(/** logging */),
            )
            .values()
            .assume_ordering::<TotalOrder>(nondet!(/** logging order irrelevant */))
            .for_each(q!(|req| println!("[logger] {:?}", req)));
    }

    // v2 routes to the first member in sorted order; v1 and v3 route to the last.
    match version {
        GossipVersion::V2 => gossip_server(
            requests,
            servers,
            q!(
                |(request, members): (Request, Vec<MemberId<GossipServer>>)| {
                    let mut sorted: Vec<_> = members.iter().collect();
                    sorted.sort();
                    sorted.first().map(|&m| (m.clone(), request.clone()))
                }
            ),
        ),
        GossipVersion::V1 | GossipVersion::V3 => gossip_server(
            requests,
            servers,
            q!(
                |(request, members): (Request, Vec<MemberId<GossipServer>>)| {
                    let mut sorted: Vec<_> = members.iter().collect();
                    sorted.sort();
                    sorted.pop().map(|m| (m.clone(), request.clone()))
                }
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    use super::*;

    /// Write to member 0, read from member 1; both should see the value because gossip replicates
    /// it.
    #[test]
    fn sim_gossip_server_write_then_read_v1() {
        let mut flow = FlowBuilder::new();
        let servers = flow.cluster::<GossipServer>();
        let (inputter, requests) = servers.sim_input();

        let (responses, membership_snapshots) =
            gossip_server_versioned(GossipVersion::V1, requests, &servers, None);
        let output = responses.sim_cluster_output();
        let membership_output = membership_snapshots
            .map(q!(|members| members.len()))
            .sim_cluster_output();

        let mut write_confirmed = false;

        flow.sim()
            .with_cluster_size(&servers, 2)
            .exhaustive(async || {
                // Wait until both members have discovered full membership.
                for member in [0, 1] {
                    loop {
                        match membership_output.next(member).await {
                            Some(count) if count >= 2 => break,
                            Some(_) => continue,
                            None => return,
                        }
                    }
                }

                inputter.send(0, Request::Write { value: 42 });

                let r1 = output.collect_sorted::<Vec<_>>(0).await;
                match r1.as_slice() {
                    [Response { value: 42 }] => {
                        write_confirmed = true;
                    }
                    other => panic!("unexpected write response: {other:?}"),
                }

                inputter.send(1, Request::Read);
                let r2 = output.collect_sorted::<Vec<_>>(1).await;
                match r2.as_slice() {
                    [Response { value: 42 }] => {}
                    other => panic!("read from member 1 got unexpected response: {other:?}"),
                }
            });

        assert!(
            write_confirmed,
            "expected at least one execution where write was confirmed"
        );
    }

    /// Two versions of the gossip server with incompatible routing (v1 last-member, v2
    /// first-member): the multi-version sim catches the resulting linearizability violation.
    #[test]
    #[should_panic(
        expected = r#"read from member 1 got unexpected response: [Response { value: 0 }]"#
    )]
    fn sim_multi_version_gossip() {
        let mut flow = FlowBuilder::new();

        let servers_v1 = flow.cluster::<GossipServer>();
        let (inputter, requests_v1) = servers_v1.sim_input();
        let (responses_v1, membership_v1) =
            gossip_server_versioned(GossipVersion::V1, requests_v1, &servers_v1, None);
        let output = responses_v1.sim_cluster_output();
        let membership_output = membership_v1
            .map(q!(|members| members.len()))
            .sim_cluster_output();

        let servers_v2 = flow.next_version(&servers_v1);
        let (inputter_v2, requests_v2) = servers_v2.sim_input();
        let (responses_v2, membership_v2) =
            gossip_server_versioned(GossipVersion::V2, requests_v2, &servers_v2, None);
        let output_v2 = responses_v2.sim_cluster_output();
        let membership_output_v2 = membership_v2
            .map(q!(|members| members.len()))
            .sim_cluster_output();

        let mut write_confirmed = false;

        flow.sim()
            .with_cluster_size(&servers_v1, 1)
            .with_cluster_size(&servers_v2, 1)
            .exhaustive(async || {
                loop {
                    match membership_output.next(0).await {
                        Some(count) if count >= 2 => break,
                        Some(_) => continue,
                        None => return,
                    }
                }

                loop {
                    match membership_output_v2.next(1).await {
                        Some(count) if count >= 2 => break,
                        Some(_) => continue,
                        None => return,
                    }
                }

                inputter.send(0, Request::Write { value: 42 });

                let r1 = output.collect_sorted::<Vec<_>>(0).await;
                match r1.as_slice() {
                    [] => return,
                    [Response { value: 42 }] => {
                        write_confirmed = true;
                    }
                    other => panic!("unexpected write response: {other:?}"),
                }

                // v2 routes differently than v1, exposing the bug: the read should see 42 but won't.
                inputter_v2.send(1, Request::Read);
                let r2 = output_v2.collect_sorted::<Vec<_>>(1).await;
                match r2.as_slice() {
                    [] => {}
                    [Response { value: 42 }] => {}
                    other => panic!("read from member 1 got unexpected response: {other:?}"),
                }
            });

        assert!(
            write_confirmed,
            "expected at least one execution where write was confirmed"
        );
    }

    pub struct Coordinator {}

    /// Corresponding clusters are linked explicitly via [`Cluster::next_version`], independent of
    /// any other locations a version declares. Version 2 additionally declares an unrelated
    /// `Coordinator` process; both versions run identical (v1) routing, so a write to member 0 must
    /// be observed by a read from member 1.
    #[test]
    fn sim_multi_version_extra_unrelated_location() {
        let mut flow = FlowBuilder::new();

        let servers_v1 = flow.cluster::<GossipServer>();
        let (inputter, requests_v1) = servers_v1.sim_input();
        let (responses_v1, membership_v1) =
            gossip_server_versioned(GossipVersion::V1, requests_v1, &servers_v1, None);
        let output = responses_v1.sim_cluster_output();
        let membership_output = membership_v1
            .map(q!(|members| members.len()))
            .sim_cluster_output();

        // V2 additionally declares an unrelated process that does not correspond to anything.
        let servers_v2 = flow.next_version(&servers_v1);
        let _coordinator = flow.process::<Coordinator>();
        let (inputter_v2, requests_v2) = servers_v2.sim_input();
        let (responses_v2, membership_v2) =
            gossip_server_versioned(GossipVersion::V1, requests_v2, &servers_v2, None);
        let output_v2 = responses_v2.sim_cluster_output();
        let membership_output_v2 = membership_v2
            .map(q!(|members| members.len()))
            .sim_cluster_output();

        let mut write_confirmed = false;

        flow.sim()
            .with_cluster_size(&servers_v1, 1)
            .with_cluster_size(&servers_v2, 1)
            .exhaustive(async || {
                for (handle, member) in [(&membership_output, 0u32), (&membership_output_v2, 1)] {
                    loop {
                        match handle.next(member).await {
                            Some(count) if count >= 2 => break,
                            Some(_) => continue,
                            None => return,
                        }
                    }
                }

                inputter.send(0, Request::Write { value: 42 });

                let r1 = output.collect_sorted::<Vec<_>>(0).await;
                match r1.as_slice() {
                    [Response { value: 42 }] => {
                        write_confirmed = true;
                    }
                    other => panic!("unexpected write response: {other:?}"),
                }

                // Both versions route identically, so the read must observe the write.
                inputter_v2.send(1, Request::Read);
                let r2 = output_v2.collect_sorted::<Vec<_>>(1).await;
                match r2.as_slice() {
                    [Response { value: 42 }] => {}
                    other => panic!("read from member 1 got unexpected response: {other:?}"),
                }
            });

        assert!(
            write_confirmed,
            "expected at least one execution where write was confirmed"
        );
    }

    /// Differing-topology, three-version test. v1 and v2 have only `GossipServer`; v3 adds a
    /// `Logger` cluster. This exercises both N>2 versions and versions with different
    /// topologies in a single multi-version simulation.
    #[test]
    fn sim_multi_version_differing_topology() {
        let mut flow = FlowBuilder::new();

        let servers_v1 = flow.cluster::<GossipServer>();
        let (inputter, requests_v1) = servers_v1.sim_input();
        let (responses_v1, membership_v1) =
            gossip_server_versioned(GossipVersion::V1, requests_v1, &servers_v1, None);
        let output = responses_v1.sim_cluster_output();
        let membership_output = membership_v1
            .map(q!(|members| members.len()))
            .sim_cluster_output();

        let servers_v2 = flow.next_version(&servers_v1);
        let (_inputter_v2, requests_v2) = servers_v2.sim_input();
        let (responses_v2, membership_v2) =
            gossip_server_versioned(GossipVersion::V2, requests_v2, &servers_v2, None);
        let _output_v2 = responses_v2.sim_cluster_output();
        let _membership_output_v2 = membership_v2
            .map(q!(|members| members.len()))
            .sim_cluster_output();

        // V3 adds a brand-new Logger cluster (different topology).
        let servers_v3 = flow.next_version(&servers_v2);
        let loggers_v3 = flow.cluster::<Logger>();
        let (inputter_v3, requests_v3) = servers_v3.sim_input();
        let (responses_v3, membership_v3) = gossip_server_versioned(
            GossipVersion::V3,
            requests_v3,
            &servers_v3,
            Some(&loggers_v3),
        );
        let output_v3 = responses_v3.sim_cluster_output();
        let membership_output_v3 = membership_v3
            .map(q!(|members| members.len()))
            .sim_cluster_output();

        let mut write_confirmed = false;
        let mut read_observed = false;

        // v2 contributes no GossipServer member (size 0) but still participates in the merge; the
        // two GossipServer members are global 0 (v1) and global 1 (v3). Two members keeps the
        // gossip membership-ordering search tractable.
        flow.sim()
            .with_cluster_size(&servers_v1, 1)
            .with_cluster_size(&servers_v2, 0)
            .with_cluster_size(&servers_v3, 1)
            .with_cluster_size(&loggers_v3, 1)
            .exhaustive(async || {
                for (handle, member) in [(&membership_output, 0u32), (&membership_output_v3, 1)] {
                    loop {
                        match handle.next(member).await {
                            Some(count) if count >= 2 => break,
                            Some(_) => continue,
                            None => return,
                        }
                    }
                }

                inputter.send(0, Request::Write { value: 42 });

                let r1 = output.collect_sorted::<Vec<_>>(0).await;
                match r1.as_slice() {
                    [Response { value: 42 }] => {
                        write_confirmed = true;
                    }
                    other => panic!("unexpected write response: {other:?}"),
                }

                // v1 and v3 route identically, so a read through v3's member must observe the
                // value written through v1 — exercising v3's external input and output.
                inputter_v3.send(1, Request::Read);
                let r2 = output_v3.collect_sorted::<Vec<_>>(1).await;
                match r2.as_slice() {
                    [] => {}
                    [Response { value: 42 }] => read_observed = true,
                    other => panic!("read from v3 member 1 got unexpected response: {other:?}"),
                }
            });

        assert!(
            write_confirmed,
            "expected at least one execution where write was confirmed"
        );
        assert!(
            read_observed,
            "expected at least one execution where v3's read observed the gossip-replicated value"
        );
    }

    /// A trivial per-member pipeline with no networking: each member maps its own input.
    fn echo<'a>(
        input: Stream<usize, Cluster<'a, GossipServer>, Unbounded, TotalOrder>,
    ) -> Stream<usize, Cluster<'a, GossipServer>, Unbounded, TotalOrder> {
        input.map(q!(|x| x + 1))
    }

    /// Wires an `echo` pipeline (no networking) onto `servers`, returning its input/output handles.
    fn echo_version<'a>(
        servers: &Cluster<'a, GossipServer>,
    ) -> (
        hydro_lang::sim::SimClusterSender<usize, TotalOrder, ExactlyOnce>,
        hydro_lang::sim::SimClusterReceiver<usize, TotalOrder, ExactlyOnce>,
    ) {
        let (inputter, requests) = servers.sim_input::<usize>();
        let output = echo(requests).sim_cluster_output();
        (inputter, output)
    }

    /// Two identical no-networking versions: global member 0 runs v1, member 1 runs v2, each echoes
    /// its own input. Exercises version-scoped member construction and external I/O with no
    /// cross-version channel.
    #[test]
    fn sim_multi_version_basic_echo_no_networking() {
        let mut flow = FlowBuilder::new();

        let servers_v1 = flow.cluster::<GossipServer>();
        let (inputter, output) = echo_version(&servers_v1);

        let servers_v2 = flow.next_version(&servers_v1);
        let (inputter_v2, output_v2) = echo_version(&servers_v2);

        flow.sim()
            .with_cluster_size(&servers_v1, 1)
            .with_cluster_size(&servers_v2, 1)
            .exhaustive(async || {
                inputter.send(0, 10);
                inputter_v2.send(1, 20);

                let r0 = output.collect::<Vec<_>>(0).await;
                assert!(
                    matches!(r0.as_slice(), [11]),
                    "member 0 echo mismatch: {r0:?}"
                );

                let r1 = output_v2.collect::<Vec<_>>(1).await;
                assert!(
                    matches!(r1.as_slice(), [21]),
                    "member 1 echo mismatch: {r1:?}"
                );
            });
    }

    /// A second cluster type used to give a different version chain its own member space.
    pub struct Worker {}

    /// Wires a no-networking `echo` pipeline onto any cluster, returning its input/output handles.
    fn echo_version_for<'a, C>(
        cluster: &Cluster<'a, C>,
    ) -> (
        hydro_lang::sim::SimClusterSender<usize, TotalOrder, ExactlyOnce>,
        hydro_lang::sim::SimClusterReceiver<usize, TotalOrder, ExactlyOnce>,
    ) {
        let (inputter, requests) = cluster.sim_input::<usize>();
        let output = requests.map(q!(|x| x + 1)).sim_cluster_output();
        (inputter, output)
    }

    /// Two independent clusters with **different numbers of versions**: `GossipServer` has two
    /// versions and `Worker` has three. Because version numbers are assigned per correspondence
    /// group, the overlapping version counts across the two clusters must not collide; each
    /// version's member echoes its own input over a no-networking pipeline.
    #[test]
    fn sim_multi_version_two_clusters_differing_version_counts() {
        let mut flow = FlowBuilder::new();

        // Cluster A: `GossipServer` with two versions. Members are globals 0 (v1) and 1 (v2).
        let a_v1 = flow.cluster::<GossipServer>();
        let (a_in_v1, a_out_v1) = echo_version_for(&a_v1);
        let a_v2 = flow.next_version(&a_v1);
        let (a_in_v2, a_out_v2) = echo_version_for(&a_v2);

        // Cluster B: `Worker` with three versions. Members are globals 0 (v1), 1 (v2), 2 (v3).
        let b_v1 = flow.cluster::<Worker>();
        let (b_in_v1, b_out_v1) = echo_version_for(&b_v1);
        let b_v2 = flow.next_version(&b_v1);
        let (b_in_v2, b_out_v2) = echo_version_for(&b_v2);
        let b_v3 = flow.next_version(&b_v2);
        let (b_in_v3, b_out_v3) = echo_version_for(&b_v3);

        flow.sim()
            .with_cluster_size(&a_v1, 1)
            .with_cluster_size(&a_v2, 1)
            .with_cluster_size(&b_v1, 1)
            .with_cluster_size(&b_v2, 1)
            .with_cluster_size(&b_v3, 1)
            .exhaustive(async || {
                a_in_v1.send(0, 10);
                a_in_v2.send(1, 20);
                b_in_v1.send(0, 100);
                b_in_v2.send(1, 200);
                b_in_v3.send(2, 300);

                let a0 = a_out_v1.collect::<Vec<_>>(0).await;
                assert!(matches!(a0.as_slice(), [11]), "A v1 member 0: {a0:?}");
                let a1 = a_out_v2.collect::<Vec<_>>(1).await;
                assert!(matches!(a1.as_slice(), [21]), "A v2 member 1: {a1:?}");

                let b0 = b_out_v1.collect::<Vec<_>>(0).await;
                assert!(matches!(b0.as_slice(), [101]), "B v1 member 0: {b0:?}");
                let b1 = b_out_v2.collect::<Vec<_>>(1).await;
                assert!(matches!(b1.as_slice(), [201]), "B v2 member 1: {b1:?}");
                let b2 = b_out_v3.collect::<Vec<_>>(2).await;
                assert!(matches!(b2.as_slice(), [301]), "B v3 member 2: {b2:?}");
            });
    }

    /// Exercises a **process → cluster** named channel in a single-version sim. This is the
    /// mixed-direction case (sender is a process, receiver is a cluster), distinct from the
    /// intra-cluster `gossip` channel the other tests use, so it covers a different branch of the
    /// simulator's network lowering.
    fn broadcast_echo<'a>(
        input: Stream<usize, Process<'a, ()>, Unbounded, TotalOrder>,
        servers: &Cluster<'a, GossipServer>,
    ) -> Stream<usize, Cluster<'a, GossipServer>, Unbounded, TotalOrder> {
        input
            .broadcast(
                servers,
                TCP.fail_stop().bincode().name("p2c_broadcast"),
                nondet!(/** test: stable membership */),
            )
            .map(q!(|v| v + 1))
    }

    #[test]
    fn sim_process_to_cluster_named_channel() {
        let mut flow = FlowBuilder::new();
        let source = flow.process::<()>();
        let servers = flow.cluster::<GossipServer>();
        let (inputter, input) = source.sim_input::<usize, TotalOrder, _>();
        let output = broadcast_echo(input, &servers).sim_cluster_output();

        let mut broadcast_observed = false;

        flow.sim()
            .with_cluster_size(&servers, 2)
            .exhaustive(async || {
                inputter.send(5);

                for member in 0..2 {
                    let r = output.collect::<Vec<_>>(member).await;
                    match r.as_slice() {
                        [] => {}
                        [6] => broadcast_observed = true,
                        other => panic!("member {member} broadcast echo mismatch: {other:?}"),
                    }
                }
            });

        assert!(
            broadcast_observed,
            "expected at least one execution where a member received the broadcast"
        );
    }

    /// Two separately-declared `cluster::<GossipServer>()` are independent locations, not one
    /// merged cluster — cross-version correspondence is established only via
    /// [`Cluster::next_version`], not by sharing a type tag. Sizing them differently (2 and 3) and
    /// driving each through its own handles confirms each has its own `0..size` member space.
    #[test]
    fn sim_same_tag_clusters_single_version_are_independent() {
        let mut flow = FlowBuilder::new();

        let servers_a = flow.cluster::<GossipServer>();
        let (inputter_a, requests_a) = servers_a.sim_input::<usize>();
        let output_a = echo(requests_a).sim_cluster_output();

        let servers_b = flow.cluster::<GossipServer>();
        let (inputter_b, requests_b) = servers_b.sim_input::<usize>();
        let output_b = echo(requests_b).sim_cluster_output();

        flow.sim()
            .with_cluster_size(&servers_a, 2)
            .with_cluster_size(&servers_b, 3)
            .exhaustive(async || {
                inputter_a.send(0, 10);
                inputter_a.send(1, 11);
                inputter_b.send(0, 20);
                inputter_b.send(1, 21);
                inputter_b.send(2, 22);

                for (member, expected) in [(0u32, 11usize), (1, 12)] {
                    let r = output_a.collect::<Vec<_>>(member).await;
                    assert!(
                        matches!(r.as_slice(), [v] if *v == expected),
                        "cluster A member {member} echo mismatch: {r:?}"
                    );
                }

                for (member, expected) in [(0u32, 21usize), (1, 22), (2, 23)] {
                    let r = output_b.collect::<Vec<_>>(member).await;
                    assert!(
                        matches!(r.as_slice(), [v] if *v == expected),
                        "cluster B member {member} echo mismatch: {r:?}"
                    );
                }
            });
    }
}
