import HydroLean.Programs.Paxos.PaxosCore

/-!
# Executable falsifications: FINDINGS.md B1/B2 (faithful paxos.rs)

A concrete decision script under which the **faithful** variant commits two
different values for slot 0 (`CommitAgreement` violated), while the
**guarded** variant (B1 fix: send P1a once per ballot) commits only one —
the same script blocks at the illegal duplicate consumption.

Scenario (nP = 2, nA = 3, f = 1, quorum = 2):
- Proposer 0 (ballot `(0,0)`) is elected by a *genuine* quorum {acceptor 1,
  acceptor 2} and commits `y = 20` at slot 0.
- Proposer 1 (ballot `(0,1)`) triggers twice without a ballot change and
  re-broadcasts the *same* P1a (faithful paxos.rs:311–317). Acceptor 0
  consumes **both copies in one tick** and `Ok`s each (its max is unchanged
  by the duplicate). Proposer 1's `collect_quorum_with_response` counts both
  (sender identity was dropped by `.values()`): a "quorum" of 2 from **one**
  distinct acceptor. Elected with an empty merged log, it assigns its own
  payload `x = 10` to slot 0 and commits it with votes from acceptors
  {0, 1} (acceptor 1 promised `(0,1)` after its `y` vote; that carried log
  is adversarially dropped by proposer 1's quorum batch).
-/

namespace HydroLean.Programs.Paxos.Falsification

open HydroLean.Hydro
open HydroLean.Programs.Paxos

/-- Proposer 0's ballot. -/
def bA : Ballot 2 := ⟨0, 0⟩
/-- Proposer 1's ballot (higher by owner tiebreak). -/
def bB : Ballot 2 := ⟨0, 1⟩

def yv : Nat := 20
def xv : Nat := 10

def leN : LENondet Nat 2 3 where
  maxSnap := fun _ => [[], [], []]
  heartbeat := fun i =>
    if i.val = 0 then ⟨[], [true, false, false], [true, false, false]⟩
    else ⟨[], [true, true, false], [true, true, false]⟩
  p1b := fun i =>
    if i.val = 0 then
      { quorumBatch := [[(bA, .ok (none, [])), (bA, .ok (none, []))]]
        quorumSnap := [[], [(bA, (none, [])), (bA, (none, []))], []] }
    else
      { quorumBatch := [[(bB, .ok (none, [])), (bB, .ok (none, []))]]
        quorumSnap := [[], [], [(bB, (none, [])), (bB, (none, []))]] }
  p1aBatch := fun j =>
    if j.val = 0 then [[], [], [bB, bB], []]
    else if j.val = 1 then [[bA], [], [bB], []]
    else [[bA], [], [], []]
  fuel := 8

def p2aA : P2a Nat 2 := ⟨0, bA, 0, some yv⟩
def p2aB : P2a Nat 2 := ⟨1, bB, 0, some xv⟩

def spN : SPNondet Nat 2 3 where
  payloadBatch := fun i => if i.val = 0 then [0, 1, 0] else [0, 0, 1]
  p2aBatch := fun j =>
    if j.val = 0 then [[], [], [], [p2aB]]
    else if j.val = 1 then [[], [p2aA], [], [p2aB]]
    else [[], [p2aA], [], []]
  quorumBatch := fun i =>
    if i.val = 0 then [[((0, bA), .ok ()), ((0, bA), .ok ())]]
    else [[((0, bB), .ok ()), ((0, bB), .ok ())]]
  joinBatch := fun i =>
    if i.val = 0 then [[], [], [(0, bA)]] else [[], [], [(0, bB)]]

def nd : PaxosNondet Nat 2 3 := ⟨leN, spN, 8⟩

/-- Clients: proposer 0's client sends `y`, proposer 1's sends `x` (a
constant callback — its staged type carries the trivial mono proof). -/
def clients : (Fin 2 → Stream (Ballot 2)) →ₘ (Fin 2 → Stream Nat) :=
  ⟨fun _ i => if i.val = 0 then [yv] else [xv],
   fun _ _ => List.prefix_refl _⟩

/-- The faithful run. -/
def runFaithful := paxos_core PaxosVariant.faithful 1 clients nd
/-- The guarded run, same script. -/
def runGuarded := paxos_core PaxosVariant.guarded 1 clients nd

/- Evaluation via the compiled executable `falsify` (`lake exe falsify`;
the interpreter takes minutes on the nested fixpoints even with `memoF`).
Verified output (recorded in FINDINGS.md B1):
  runFaithful.2 0 = [(0, some 20)]   -- proposer 0 commits y
  runFaithful.2 1 = [(0, some 10)]   -- proposer 1 commits x — DISAGREEMENT
  runGuarded.2 1  = []               -- B1 fix blocks the fake quorum
-/

/-! ## B2: every-tick recommit (module-face witness)

The faithful `sp_send_step` re-runs `recommit_after_leader_election` on
every tick that re-presents the (frozen) quorum view — duplicate
`(slot, ballot)` P2a keys and duplicate `join_responses` metadata (the
documented one-metadata-per-key contract violation). The guarded step
(`recommittedAt`) emits once. -/

/-- A one-entry recovered log, re-presented on two consecutive leader
ticks. -/
def b2Qlogs : List (P1bPayload Nat 2) := [(none, [(0, ⟨⟨0, 0⟩, some 99⟩)])]

def b2Ins : List (SPTick Nat 2) :=
  [(b2Qlogs, ⟨1, 1⟩, true, []), (b2Qlogs, ⟨1, 1⟩, true, [])]

/-- Faithful: the recommit fires on both ticks (duplicate keys). -/
def b2Faithful : List ((Nat × Ballot 2) × Option Nat) :=
  (HydroLean.Hydro.scan (sp_send_step PaxosVariant.faithful 1)
    ⟨0, none, false⟩ b2Ins).flatten

/-- Guarded: once per ballot. -/
def b2Guarded : List ((Nat × Ballot 2) × Option Nat) :=
  (HydroLean.Hydro.scan (sp_send_step PaxosVariant.guarded 1)
    ⟨0, none, false⟩ b2Ins).flatten

-- The B2 violation, machine-checked: the faithful run emits the same
-- `(slot, ballot)` key twice.
#guard (b2Faithful.map (·.1)).Nodup = false
-- The guarded run's keys are duplicate-free (the `spKeys_inv` face,
-- instantiated).
#guard (b2Guarded.map (·.1)).Nodup = true

end HydroLean.Programs.Paxos.Falsification
