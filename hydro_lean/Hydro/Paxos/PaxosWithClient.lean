import Hydro.Paxos.PaxosCore

/-!
# `hydro_test/src/cluster/paxos_with_client.rs` — `PaxosLike::with_client`

One Rust function = one Lean definition: `with_client`
(paxos_with_client.rs:47–110) for `CorePaxos` (paxos.rs:78–118:
`get_recipient_from_ballot` is `Optional::map(|ballot|
ballot.proposer_id)`, `build` is `paxos_core(…).1`). The `PaxosLike`
trait's other instances (compartmentalized Paxos) are not mirrored.

**Closures, flattened (ruling F).** Rust's `with_client` hands `build` a
closure from the new-leader ballot stream to the payload stream at the
proposers; `paxos_core` closes that cycle through its `forward_ref`s.
Here the closure's body IS the `fix` body over the payload wire
`c_to_proposers` (what the closure returns feeds `paxos_core`, whose
new-leader output feeds the closure): one knot, one fuel more than
`paxos_core`'s. The let-chain is otherwise the Rust's; `.inspect(println!)`
has no denotation.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- The closure's decisions: its three `nondet!` sites. -/
structure PTLDec (H : HydroSem L mem) (nP nC : Nat) (P : Type) [DecidableEq P] where
  /-- `use::batch(payloads, nondet!(/** see below */))` (:70). -/
  payloads : H.BatchDec nC P .totalOrder .exactlyOnce
  /-- `use::snapshot(cur_leader_id, nondet!(nondet_commit))` (:71–78). -/
  leader : H.SnapDec nC (Ballot nP) .noOrder
  /-- `payloads_at_proposer.assume_ordering(nondet!(nondet_order))` (:94–99). -/
  order : H.OrderSelDec nP P

/-- The closure's adversary-side (sched-det) bundle: its two transports. -/
structure PTLSched (H : HydroSem L mem) (nP nC : Nat) where
  /-- `new_leader_elected.broadcast(clients, TCP.fail_stop().bincode(), nondet!(/** TODO */))` (:57–60). -/
  leaderCh : H.TransportDec nC nP
  /-- `.demux(&leaders, TCP.fail_stop().bincode())` (:90). -/
  payloadCh : H.TransportDec nP nC

/-- `with_client`'s decisions: `paxos_core`'s, the closure's, and the
closure's cycle through `paxos_core` (F). -/
structure PWCDec (H : HydroSem L mem) (nP nA nC : Nat) (P : Type) [DecidableEq P]
    (ckα : Type) [DecidableEq ckα] (ckord : StrOrd) where
  /-- `paxos_core`'s. -/
  core : PaxosCoreDec H nP nA P ckα ckord
  /-- The closure's. -/
  client : PTLDec H nP nC P
  /-- The payload-generator closure's cycle through `paxos_core` (F). -/
  fuelPayloads : H.FixDec

/-- `with_client`'s adversary-side (sched-det) bundle. -/
structure PWCSched (H : HydroSem L mem) (nP nA nC : Nat) (P : Type) [DecidableEq P] where
  core : PaxosCoreSched H nP nA P
  client : PTLSched H nP nC

/-- The trivial bundle at the denotation. -/
def PWCSched.triv {nP nA nC : Nat} {P : Type} [DecidableEq P] :
    PWCSched (Values L mem) nP nA nC P := ⟨.triv, (), ()⟩

/-- What `with_client` **ensures**: `paxos_core`'s headline, on its one
output — with the bugfix flags and intersecting `f + 1` quorums, the
commits are slot-functional, whatever the clients send. -/
structure PWCEnsures (variant : PaxosVariant) (prop acc : L) (f : Nat)
    (out : MemberMap (mem prop) (Multiset (Nat × Option P))) : Prop where
  slot_functional : variant = .guarded → mem acc ≤ 2 * f + 1 → SlotFunctional out

/-- `Stream::max()`'s reduce commutes. -/
theorem ballotMax_reduceOk {nP : Nat} :
    FoldOkP .noOrder .exactlyOnce (fun (_ _ : Ballot nP) => True)
      (reduceStep (fun a b : Ballot nP => max a b)) :=
  fun s x y _ => by cases s <;> simp [reduceStep, max_comm, max_left_comm]

/-- **paxos_with_client.rs:52–100**, the closure `move |new_leader_elected|
{ … }` that `with_client` hands to `build`, as the function Rust would
write (`fn payloads_to_leader(new_leader_elected, payloads) -> Stream<P,
Cluster<Proposer>, Unbounded, TotalOrder>`): clients learn the leader from
the new-leader ballots, batch their payloads to it (holding them while no
leader is known). -/
hydro def payloads_to_leader (H : HydroSem L mem) (prop clients : L)
    (new_leader_elected : H.Stream prop (Ballot (mem prop)) .totalOrder .exactlyOnce)
    (payloads : H.Stream clients P .totalOrder .exactlyOnce)
    (dec : PTLDec H (mem prop) (mem clients) P)
    (sched : PTLSched H (mem prop) (mem clients)) :
    H.Stream prop P .totalOrder .exactlyOnce :=
  -- let cur_leader_id = Self::get_recipient_from_ballot(
  --   new_leader_elected.broadcast(clients, TCP.fail_stop().bincode(), nondet!(/** TODO */)).values()
  --     .inspect(q!(|ballot| println!("Client notified that leader was elected: {:?}", ballot)))
  --     .max().into());
  --   — `get_recipient_from_ballot` (`Optional::map(|ballot| ballot.proposer_id)`) is applied at the
  --   snapshot below (map and snapshot commute); `inspect` has no denotation
  let cur_leader : H.Singleton clients (Ballot (mem prop)) (Option (Ballot (mem prop))) .noOrder .exactlyOnce .unbounded :=
    H.fold (reduceStep (fun a b : Ballot (mem prop) => max a b)) none (fun _ _ => True) ballotMax_reduceOk
      (H.values (H.broadcast_closed sched.leaderCh new_leader_elected))
  -- let payloads_at_proposer = sliced! {
  --   let mut unsent_payloads = use::state_null::<Stream<_, _, _, TotalOrder>>();
  --   let payload_batch = use::batch(payloads, nondet!(/** see below */));
  --   let latest_leader = use::snapshot(cur_leader_id, nondet!(nondet_commit));
  tick (state unsent_payloads : H.BoundedStream P .totalOrder .exactlyOnce)
      (input payload_batch := H.batch payloads dec.payloads)
      (input latest_leader := H.snapshot cur_leader dec.leader) :=
    let latest_leader := H.boMap latest_leader (fun ballot : Ballot (mem prop) => ballot.proposerId)
    -- let all_payloads = unsent_payloads.chain(payload_batch);
    let all_payloads := H.bchain unsent_payloads payload_batch
    -- unsent_payloads = all_payloads.clone().filter_if(latest_leader.clone().is_none());
    rebind (unsent_payloads := H.bfilterIf all_payloads (H.boIsNone latest_leader))
    -- all_payloads.cross_singleton(latest_leader)
    yield (payloads_at_proposer := H.bcrossOptional all_payloads latest_leader);
  -- }
  -- .map(q!(move |(payload, leader_id)| (leader_id, payload)))
  -- .demux(&leaders, TCP.fail_stop().bincode())
  -- .values();
  let payloads_at_proposer := H.values (H.demux sched.payloadCh
    (H.map (H.allTicks payloads_at_proposer) (fun _me (payload, leader_id) => (leader_id.val, payload)))
    (fun i => i.val))
  -- let payloads_at_proposer = { payloads_at_proposer.assume_ordering(nondet!(nondet_order)) };
  -- payloads_at_proposer
  H.assume_ordering payloads_at_proposer dec.order

#nondet_census payloads_to_leader (nondets := 3) (scheds := 2) (fuels := 0)

/-- **paxos_with_client.rs:47–110 `with_client`** (for `CorePaxos`): the
closure's payloads feed `paxos_core`, whose new-leader ballots feed the
closure. Returns `paxos_core`'s `p_to_replicas`. -/
hydro def paxos_with_client {ckα : Type} [DecidableEq ckα]
    {ckord : StrOrd} {ckret : Retries}
    (H : HydroSem L mem) (variant : PaxosVariant)
    (prop acc clients : L) (paxos_config : PaxosConfig)
    (payloads : H.Stream clients P .totalOrder .exactlyOnce)
    (checkpoints : H.Singleton acc ckα (Option Nat) ckord ckret (.monotonic ValueOrder.optNat))
    (dec : PWCDec H (mem prop) (mem acc) (mem clients) P ckα ckord)
    (sched : PWCSched H (mem prop) (mem acc) (mem clients) P) :
    H.Stream prop (Nat × Option P) .noOrder .exactlyOnce
  requires variant = .guarded
  ensures out => PWCEnsures variant prop acc paxos_config.f out :=
  -- let leaders = self.payload_recipients().clone();
  -- self.build(move |new_leader_elected| { … }, checkpoints, nondet!(nondet_commit), nondet_commit)
  --   — `build` is `paxos_core(…)`; the closure's cycle through it is this knot (F)
  fix (c_to_proposers : H.Stream prop P .totalOrder .exactlyOnce) via dec.fuelPayloads :=
    let core := paxos_core H variant prop acc paxos_config c_to_proposers checkpoints dec.core sched.core
    -- the core's contract, at these wires (replayed for the prove leg at the closed knot)
    ghost have hcore := paxos_core.ensures variant prop acc paxos_config c_to_proposers checkpoints dec.core sched.core
    -- the closure, at `paxos_core`'s new-leader ballots
    let payloads_at_proposer := payloads_to_leader H prop clients core.1 payloads dec.client sched.client
  -- the closure's result closes the cycle
  complete (payloads_at_proposer)
  core.2
  prove slot_functional := fun hv hn => hcore.slot_functional hv hn

#nondet_census paxos_with_client (nondets := 19) (scheds := 9) (fuels := 6)

end Hydro
