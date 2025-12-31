use std::collections::HashMap;

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::{Atomic, Location, MemberId};
use hydro_lang::prelude::*;
use hydro_std::quorum::collect_quorum;
use hydro_std::request_response::join_responses;
use serde::{Deserialize, Serialize};

use super::paxos::{
    Acceptor, Ballot, LogValue, P2a, PaxosConfig, PaxosPayload, Proposer, acceptor_p2,
    index_payloads, leader_election, recommit_after_leader_election,
};
use super::paxos_with_client::PaxosLike;

#[derive(Serialize, Deserialize, Clone)]
pub struct ProxyLeader {}

#[derive(Clone, Copy)]
pub struct CompartmentalizedPaxosConfig {
    pub paxos_config: PaxosConfig,
    pub num_proxy_leaders: usize,
    /// Number of rows in the acceptor grid. Each row represents a write quorum (for sending p2as).
    pub acceptor_grid_rows: usize,
    /// Number of columns in the acceptor grid. Each column represents a read quorum (for waiting for p1bs).
    pub acceptor_grid_cols: usize,
    pub num_replicas: usize,
    /// How long to wait before resending message to a different write quorum
    pub acceptor_retry_timeout: u64,
}

pub struct CoreCompartmentalizedPaxos<'a> {
    pub proposers: Cluster<'a, Proposer>,
    pub proxy_leaders: Cluster<'a, ProxyLeader>,
    pub acceptors: Cluster<'a, Acceptor>,
    pub config: CompartmentalizedPaxosConfig,
}

impl<'a> PaxosLike<'a> for CoreCompartmentalizedPaxos<'a> {
    type PaxosIn = Proposer;
    type PaxosLog = Acceptor;
    type PaxosOut = ProxyLeader;
    type Ballot = Ballot;

    fn payload_recipients(&self) -> &Cluster<'a, Self::PaxosIn> {
        &self.proposers
    }

    fn log_stores(&self) -> &Cluster<'a, Self::PaxosLog> {
        &self.acceptors
    }

    fn get_recipient_from_ballot<L: Location<'a>>(
        ballot: Optional<Self::Ballot, L, Unbounded>,
    ) -> Optional<MemberId<Self::PaxosIn>, L, Unbounded> {
        ballot.map(q!(|ballot| ballot.proposer_id))
    }

    fn build<P: PaxosPayload>(
        self,
        with_ballot: impl FnOnce(
            Stream<Ballot, Cluster<'a, Self::PaxosIn>, Unbounded>,
        ) -> Stream<P, Cluster<'a, Self::PaxosIn>, Unbounded>,
        a_checkpoint: Optional<usize, Cluster<'a, Acceptor>, Unbounded>,
        nondet_leader: NonDet,
        nondet_commit: NonDet,
    ) -> Stream<(usize, Option<P>), Cluster<'a, Self::PaxosOut>, Unbounded, NoOrder> {
        compartmentalized_paxos_core(
            &self.proposers,
            &self.proxy_leaders,
            &self.acceptors,
            a_checkpoint,
            with_ballot,
            self.config,
            nondet_leader,
            nondet_commit,
        )
        .1
    }
}

/// Implements the Compartmentalized Paxos algorithm as described in "Scaling Replicated State Machines with Compartmentalization",
/// which augments regular Paxos with a cluster of Proxy Leaders.
///
/// Proposers that wish to broadcast p2as to acceptors or collect p2bs from acceptors instead
/// go through the Proxy Leaders, which offload networking. The slot is used to determine which Proxy Leader to offload to.
/// Acceptors are arranged into a grid, where each row and column must have at least f+1 members.
/// Rows represent "write quorums"; an entire row of acceptors must confirm a payload before it is committed.
/// Columns represent "read quorums"; an entire column of acceptors must respond to a p1b before a proposer is elected the leader.
/// Read and write quorums were introduced in "Flexible Paxos: Quorum Intersection Revisited".
///
/// Returns a stream of ballots, where new values are emitted when a new leader is elected,
/// and a stream of sequenced payloads with an index and optional payload (in the case of
/// holes in the log).
///
/// # Non-Determinism
/// When the leader is stable, the algorithm will commit incoming payloads to the leader
/// in deterministic order. However, when the leader is changing, payloads may be
/// non-deterministically dropped. The stream of ballots is also non-deterministic because
/// leaders are elected in a non-deterministic process.
#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "internal paxos code // TODO"
)]
pub fn compartmentalized_paxos_core<'a, P: PaxosPayload>(
    proposers: &Cluster<'a, Proposer>,
    proxy_leaders: &Cluster<'a, ProxyLeader>,
    acceptors: &Cluster<'a, Acceptor>,
    a_checkpoint: Optional<usize, Cluster<'a, Acceptor>, Unbounded>,
    c_to_proposers: impl FnOnce(
        Stream<Ballot, Cluster<'a, Proposer>, Unbounded>,
    ) -> Stream<P, Cluster<'a, Proposer>, Unbounded>,
    config: CompartmentalizedPaxosConfig,
    nondet_leader: NonDet,
    nondet_commit_leader_change: NonDet,
) -> (
    Stream<Ballot, Cluster<'a, Proposer>, Unbounded>,
    Stream<(usize, Option<P>), Cluster<'a, ProxyLeader>, Unbounded, NoOrder>,
) {
    proposers
        .source_iter(q!(["Proposers say hello"]))
        .for_each(q!(|s| println!("{}", s)));

    proxy_leaders
        .source_iter(q!(["Proxy leaders say hello"]))
        .for_each(q!(|s| println!("{}", s)));

    acceptors
        .source_iter(q!(["Acceptors say hello"]))
        .for_each(q!(|s| println!("{}", s)));

    let proposer_tick = proposers.tick();
    let proxy_leader_tick = proxy_leaders.tick();
    let acceptor_tick = acceptors.tick();

    let (sequencing_max_ballot_complete_cycle, sequencing_max_ballot_forward_reference) =
        proposers.forward_ref::<Stream<Ballot, _, _, NoOrder>>();
    let (a_log_complete_cycle, a_log_forward_reference) =
        acceptor_tick.forward_ref::<Singleton<_, _, _>>();

    let (p_ballot, p_is_leader, p_relevant_p1bs, a_max_ballot) = leader_election(
        proposers,
        acceptors,
        &proposer_tick,
        &acceptor_tick,
        config.acceptor_grid_rows,
        config.acceptor_grid_rows * config.acceptor_grid_cols,
        config.paxos_config,
        sequencing_max_ballot_forward_reference,
        a_log_forward_reference,
        nondet!(
            /// The primary non-determinism exposed by leader election algorithm lies in which leader
            /// is elected, which affects both the ballot at each proposer and the leader flag. But using a stale ballot
            /// or leader flag will only lead to failure in sequencing rather than commiting the wrong value.
            nondet_leader
        ),
        nondet!(
            /// Because ballots are non-deterministic, the acceptor max ballot is also non-deterministic, although we are
            /// guaranteed that the max ballot will match the current ballot of a proposer who believes they are the leader.
        ),
    );

    let just_became_leader = p_is_leader
        .clone()
        .filter_if_none(p_is_leader.clone().defer_tick());

    let c_to_proposers = c_to_proposers(
        just_became_leader
            .clone()
            .if_some_then(p_ballot.clone())
            .all_ticks(),
    );

    let (p_to_replicas, a_log, sequencing_max_ballots) = sequence_payload(
        proposers,
        proxy_leaders,
        acceptors,
        &proposer_tick,
        &proxy_leader_tick,
        &acceptor_tick,
        c_to_proposers,
        a_checkpoint,
        p_ballot.clone(),
        p_is_leader,
        p_relevant_p1bs,
        config,
        a_max_ballot,
        nondet!(
            /// The relevant p1bs are non-deterministic because they come from a arbitrary quorum, but because
            /// we use a quorum, if we remain the leader there are no missing committed values when we combine the logs.
            /// The remaining non-determinism is in when incoming payloads are batched versus the leader flag and state
            /// of acceptors, which in the worst case will lead to dropped payloads as documented.
            nondet_commit_leader_change
        ),
    );

    a_log_complete_cycle.complete(a_log.snapshot_atomic(nondet!(
        /// We will always write payloads to the log before acknowledging them to the proposers,
        /// which guarantees that if the leader changes the quorum overlap between sequencing and leader
        /// election will include the committed value.
    )));
    sequencing_max_ballot_complete_cycle.complete(sequencing_max_ballots);

    (
        // Only tell the clients once when leader election concludes
        just_became_leader.if_some_then(p_ballot).all_ticks(),
        p_to_replicas,
    )
}

#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "internal paxos code // TODO"
)]
fn sequence_payload<'a, P: PaxosPayload>(
    proposers: &Cluster<'a, Proposer>,
    proxy_leaders: &Cluster<'a, ProxyLeader>,
    acceptors: &Cluster<'a, Acceptor>,
    proposer_tick: &Tick<Cluster<'a, Proposer>>,
    proxy_leader_tick: &Tick<Cluster<'a, ProxyLeader>>,
    acceptor_tick: &Tick<Cluster<'a, Acceptor>>,
    c_to_proposers: Stream<P, Cluster<'a, Proposer>, Unbounded>,
    a_checkpoint: Optional<usize, Cluster<'a, Acceptor>, Unbounded>,

    p_ballot: Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
    p_is_leader: Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,

    p_relevant_p1bs: Stream<
        (Option<usize>, HashMap<usize, LogValue<P>>),
        Tick<Cluster<'a, Proposer>>,
        Bounded,
        NoOrder,
    >,
    config: CompartmentalizedPaxosConfig,
    a_max_ballot: Singleton<Ballot, Tick<Cluster<'a, Acceptor>>, Bounded>,
    nondet_commit_leader_change: NonDet,
) -> (
    Stream<(usize, Option<P>), Cluster<'a, ProxyLeader>, Unbounded, NoOrder>,
    Singleton<
        (Option<usize>, HashMap<usize, LogValue<P>>),
        Atomic<Cluster<'a, Acceptor>>,
        Unbounded,
    >,
    Stream<Ballot, Cluster<'a, Proposer>, Unbounded, NoOrder>,
) {
    let (p_log_to_recommit, p_max_slot) =
        recommit_after_leader_election(p_relevant_p1bs, p_ballot.clone(), config.paxos_config.f);

    let p_indexed_payloads = index_payloads(
        p_max_slot,
        c_to_proposers
            .batch(
                proposer_tick,
                nondet!(
                    /// We batch payloads so that we can compute the correct slot based on
                    /// base slot. In the case of a leader re-election, the base slot is updated which
                    /// affects the computed payload slots. This non-determinism can lead to non-determinism
                    /// in which payloads are committed when the leader is changing, which is documented at
                    /// the function level.
                    nondet_commit_leader_change
                ),
            )
            .filter_if_some(p_is_leader.clone()),
    );

    let num_proxy_leaders = config.num_proxy_leaders;
    let p_to_proxy_leaders_p2a = p_indexed_payloads
        .cross_singleton(p_ballot.clone())
        .map(q!(move |((slot, payload), ballot)| (
            MemberId::<ProxyLeader>::from_raw_id((slot % num_proxy_leaders) as u32),
            ((slot, ballot), Some(payload))
        )))
        .chain(p_log_to_recommit.map(q!(move |((slot, ballot), payload)| (
            MemberId::<ProxyLeader>::from_raw_id((slot % num_proxy_leaders) as u32),
            ((slot, ballot), payload)
        ))))
        .all_ticks()
        .demux_bincode(proxy_leaders)
        .values()
        .atomic(proxy_leader_tick);

    // Send to a specific acceptor row
    let num_acceptor_rows = config.acceptor_grid_rows;
    let num_acceptor_cols = config.acceptor_grid_cols;
    let pl_to_acceptors_p2a_thrifty = p_to_proxy_leaders_p2a
        .clone()
        .flat_map_unordered(q!(move |((slot, ballot), payload)| {
            let row = slot % num_acceptor_rows;
            let mut p2as = Vec::new();
            for i in 0..num_acceptor_cols {
                p2as.push((
                    MemberId::<Acceptor>::from_raw_id((row * num_acceptor_cols + i) as u32),
                    P2a {
                        sender: MemberId::<ProxyLeader>::from_raw_id(
                            (slot % num_proxy_leaders) as u32,
                        ),
                        slot,
                        ballot: ballot.clone(),
                        value: payload.clone(),
                    },
                ));
            }
            p2as
        }))
        .end_atomic()
        .demux_bincode(acceptors)
        .values();

    let (a_log, a_to_proxy_leaders_p2b) = acceptor_p2(
        acceptor_tick,
        a_max_ballot.clone(),
        pl_to_acceptors_p2a_thrifty,
        a_checkpoint,
        proxy_leaders,
    );

    // TODO: This is a liveness problem if any node in the thrifty quorum fails
    // Need special operator for per-value timeout detection
    let (quorums, fails) = collect_quorum(
        a_to_proxy_leaders_p2b,
        config.acceptor_grid_cols,
        config.acceptor_grid_cols,
    );

    let pl_to_replicas = join_responses(
        quorums.map(q!(|k| (k, ()))),
        p_to_proxy_leaders_p2a.batch_atomic(nondet!(
            /// The metadata will always be generated before we get a quorum
            /// because our batch of `p_to_proxy_leaders_p2a` is at least after
            /// what we sent to the acceptors.
        )),
    );

    let pl_failed_p2b_to_proposer = fails
        .map(q!(|(_, ballot)| (ballot.proposer_id.clone(), ballot)))
        .inspect(q!(|(_, ballot)| println!("Failed P2b: {:?}", ballot)))
        .demux_bincode(proposers)
        .values();

    (
        pl_to_replicas.map(q!(|((slot, _ballot), (value, _))| (slot, value))),
        a_log,
        pl_failed_p2b_to_proposer,
    )
}
