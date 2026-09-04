import HydroV2.Paxos.PaxosCoreLemmas
import HydroV2.HydroDef

/-!
# `paxos_core` (paxos.rs:136–246)

The algorithm: `leader_election` and `sequence_payload`, tied by the
two remaining `forward_ref` cycles —

- **`sequencing_max_ballots`** (stream at proposers, `NoOrder
  ExactlyOnce`): P2b rejection ballots feed leader election's
  received-max merge;
- **`a_log`** (tick singleton at acceptors): `acceptor_p2`'s
  `(checkpoint, log)` wire closes into `acceptor_p1`'s same-tick reply
  read — Rust's `snapshot_atomic` *write-before-ack* (a tick's P1b
  replies exist only once that tick's log value is realized).

Returns (the new-leader ballot stream, `p_to_replicas`).

The safety face is `SlotFunctional` (agreement per slot across
replicas): `paxos_core` returns it **colocated** (`PCEnsures`), derived
end to end from the module contracts by K4, the knot induction
(`PaxosCoreLemmas.lean`). The end-to-end commit run is exercised
executably by `lake exe v2paxos`.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- All decision data `paxos_core` threads to its callees — nested by
owning module (`LEDec` and `SPDec` carry their own `nondet!` sites),
plus the two outer knot fuels. -/
structure PaxosCoreDec (H : HydroSem L mem) (nP nA : Nat) (P : Type)
    [DecidableEq P] where
  /-- `leader_election`'s decisions (received-max snapshot, heartbeat
  timing, P1a batching, quorum collection, its three cycle fuels). -/
  le : LEDec H nP nA P
  /-- `sequence_payload`'s decisions (payload/P2a/P2b batching). -/
  sp : SPDec H nP nA P
  /-- `sequencing_max_ballot` knot depth (`forward_ref`). -/
  fuelSeqMax : H.FixDec
  /-- `a_log` knot depth (`forward_ref`, `snapshot_atomic`'s
  write-before-ack staging). -/
  fuelALog : H.FixDec

/-- What `paxos_core` **ensures**, over the `Values` denotation — THE
HEADLINE: with the bugfix flags (`variant = .guarded`) and intersecting
`f + 1` quorums (`mem acc ≤ 2f + 1`), commits are slot-functional, over
the full decision space. -/
structure PCEnsures (variant : PaxosVariant) (prop acc : L) (f : Nat)
    (out : (Fin (mem prop) → Trace (Ballot (mem prop)))
      × (Fin (mem prop) → Multiset (Nat × Option P))) : Prop where
  /-- Per-slot agreement of the replica stream. -/
  slot_functional : variant = .guarded → mem acc ≤ 2 * f + 1 →
    SlotFunctional out.2

set_option maxHeartbeats 3200000 in
set_option maxRecDepth 65536 in
/-- **paxos.rs:136–246 `paxos_core`**: `leader_election` +
`sequence_payload`, tied by the `sequencing_max_ballot` and `a_log`
`forward_ref` knots. Returns (new-leader ballots, `p_to_replicas`),
with the safety headline colocated. -/
hydro [p1bPairDecEq] def paxos_core (H : HydroSem L mem) (variant : PaxosVariant)
    (prop acc : L) (f : Nat)
    (c_to_proposers : H.Stream prop P .totalOrder .exactlyOnce)
    (a_checkpoint : H.TickSingleton acc (Option Nat) .unbounded)
    (dec : PaxosCoreDec H (mem prop) (mem acc) P) :
    (H.Stream prop (Ballot (mem prop)) .totalOrder .exactlyOnce
      × H.Stream prop (Nat × Option P) .noOrder .exactlyOnce)
  ensures out => PCEnsures variant prop acc f out :=
  -- paxos.rs:141–148: the `a_log` and `sequencing_max_ballot`
  -- forward_ref cycles, closed mutually (`a_log` re-closed per
  -- sequencing iterate — Rust's `snapshot_atomic` write-before-ack)
  fix (a_log : H.TickSingleton acc (ALog P (mem prop)) .unbounded)
      (sequencing_max_ballots : H.Stream prop (Ballot (mem prop))
        .noOrder .exactlyOnce)
      via (dec.fuelALog, dec.fuelSeqMax) :=
    -- (p_ballot, p_is_leader, p_relevant_p1bs, a_max_ballot) =
    --   leader_election(proposers, acceptors, …, seq_max→, a_log→)
    let le := (leader_election H variant prop acc (f + 1) (2 * f + 1)
      dec.le sequencing_max_ballots a_log).val
    -- just_became_leader = p_is_leader.and(¬ p_is_leader.defer_tick())
    let just_became_leader := H.mapTick
      (H.zipTick le.2.1 (H.defer false le.2.1))
      (fun _me x => x.1 && !x.2)
    -- (p_to_replicas, a_log, sequencing_max_ballots) =
    --   sequence_payload(…, c_to_proposers, a_checkpoint, p_ballot, …)
    let sp := (sequence_payload H variant prop acc c_to_proposers
      a_checkpoint
      (H.forgetBound le.1) le.2.1 le.2.2.1 f
      (H.forgetBound le.2.2.2) dec.sp).val
    -- a_log_complete_cycle.complete(a_log);
    -- sequencing_max_ballot_complete_cycle.complete(seq_max_ballots)
    complete (sp.2.1, sp.2.2)
    (-- p_ballot.filter_if(just_became_leader).all_ticks()
     H.allTicks (H.emitBatches (H.mapTick
       (H.zipTick (H.forgetBound le.1) just_became_leader)
       (fun _me x => if x.2 then [x.1] else []))),
     sp.1)
    prove
      slot_functional := fun hvar hnA => by
        subst hvar
        exact paxos_core_agree prop acc f c_to_proposers a_checkpoint
          dec.le dec.sp dec.fuelALog sequencing_max_ballots hnA

/-! ## Executable non-vacuity

The end-to-end guarded-commit scenario (one proposer, one acceptor,
`f = 0`: election at tick 1, payload `42` sequenced at slot 0, accepted
through the same-tick `a_log` knot, committed by the singleton quorum)
runs in the compiled demo executable `v2paxos` — kernel-level `#guard`
evaluation of the nested Kleene closures is prohibitively slow, so the
non-vacuity check is a build artifact instead (`lake exe v2paxos`). -/


end HydroV2

