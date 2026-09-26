import HydroV2.Paxos.PaxosCore
import HydroV2.Paxos.SequencePayloadLemmas

/-!
# Executable falsifications: FINDINGS.md B1/B2 (faithful paxos.rs) — V2

The V1 falsification scripts (FINDINGS B1/B2), re-expressed in the V2
decision vocabulary (`PaxosCoreDec` at `Values PaxLoc (paxMem 2 3)`).

**B1** (paxos.rs:311–317): a concrete decision record under which the
**faithful** variant commits two different values for slot 0
(`SlotFunctional` violated), while the **guarded** variant (B1 fix:
send each ballot's P1a once) commits only one — under the same record,
the guarded run's duplicate consumptions are no longer count-legal
(`batchCuts` truncates), so the fake quorum never assembles.

Scenario (nP = 2, nA = 3, f = 1, quorum = 2):
- Proposer 0 (ballot `(0,0)`) is elected by a *genuine* quorum
  {acceptor 1, acceptor 2} and commits `y = 20` at slot 0.
- Proposer 1 (ballot `(0,1)`) triggers twice without a ballot change,
  re-broadcasting the *same* P1a (faithful). Acceptor 0 consumes
  **both copies in one tick** (its max is unchanged by the duplicate)
  and `Ok`s each. Proposer 1's `collect_quorum_with_response` counts
  both: a "quorum" of 2 from **one** distinct acceptor. Elected with
  an empty merged log, it assigns its own payload `x = 10` to slot 0
  and commits it with acks from acceptors {0, 1} (acceptor 1 promised
  `(0,1)` after its `y` vote; that carried log is adversarially left
  unconsumed by proposer 1's quorum batch).

**B2** (paxos.rs:596–672 + 706–734): the recommit gate
(`spGateStep`, the scan state of `sequence_payload`'s accepted-log
view) — the faithful gate fires on *every* nonempty view, so a view
re-presented on two leader ticks recommits twice: duplicate
`(slot, ballot)` P2a keys. The guarded gate (`recommittedAt`) fires
once per ballot. Machine-checked below by `#guard`; printed by
`lake exe falsify`.
-/

namespace HydroV2
namespace Falsification

/-- The `Values` instance at two proposers, three acceptors. -/
abbrev HB1 : HydroSem PaxLoc (paxMem 2 3) := Values PaxLoc (paxMem 2 3)

/-- Proposer 0's ballot. -/
def bA : Ballot 2 := ⟨0, 0⟩
/-- Proposer 1's ballot (higher by owner tiebreak). -/
def bB : Ballot 2 := ⟨0, 1⟩

def yv : Nat := 20
def xv : Nat := 10

/-- Proposer 0's slot-0 `y` proposal. -/
def p2aA : P2a Nat 2 := ⟨0, bA, 0, some yv⟩
/-- Proposer 1's poisoned slot-0 `x` proposal. -/
def p2aB : P2a Nat 2 := ⟨1, bB, 0, some xv⟩

/-- The B1 decision record. Timelines: proposer 0 = 2 ticks (trigger,
elect+commit), proposer 1 = 3 ticks (trigger, re-trigger, fake
elect+commit); acceptor 0 = 4 ticks (…, both P1a copies, P2a x),
acceptor 1 = 4 ticks (P1a bA, P2a y, P1a bB, P2a x), acceptor 2 =
2 ticks (P1a bA, P2a y). Batch legality is checked against completed
pools, so consumption times are free (the adversary's freedom). -/
def b1Dec : PaxosCoreDec HB1 2 3 Nat where
  le :=
    { -- neither proposer ever sees the other's ballot
      receivedMax := fun i =>
        if i.val = 0 then [{}, {}] else [{}, {}, {}]
      hb := { sample := fun _ => []
              -- P0 triggers at tick 0; P1 at ticks 0 AND 1 (same
              -- ballot — the faithful re-broadcast)
              timeout := fun i =>
                if i.val = 0 then [true, false] else [true, true, false]
              interval := fun i =>
                if i.val = 0 then [true, false] else [true, true, false]
              ial := () }
      p1aCh := ()
      -- acceptor 0 consumes BOTH bB copies in one tick; acceptors 1, 2
      -- promise bA genuinely (acceptor 1 later promises bB)
      p1aBatch := fun j =>
        if j.val = 0 then [{}, {}, {bB, bB}, {}]
        else if j.val = 1 then [{bA}, {}, {bB}, {}]
        else [{bA}, {}]
      p1bCh := ()
      p1b := { -- P0 consumes the genuine bA quorum (acceptors 1, 2);
               -- P1 consumes acceptor 0's two duplicate Oks and leaves
               -- acceptor 1's log-carrying reply unconsumed
               cqwr := fun i =>
                 if i.val = 0 then
                   [{}, {(bA, .ok (none, [])), (bA, .ok (none, []))}]
                 else
                   [{}, {}, {(bB, .ok (none, [])), (bB, .ok (none, []))}]
               cqwrEmit := ()
               order := fun i =>
                 if i.val = 0 then [(bA, (none, [])), (bA, (none, []))]
                 else [(bB, (none, [])), (bB, (none, []))]
               snap := fun i =>
                 if i.val = 0 then [0, 2] else [0, 0, 2] }
      fuelFail := (1 : UnfoldFuel)
      fuelIAL := (1 : UnfoldFuel)
      fuelLead := (3 : UnfoldFuel) }
  sp :=
    { -- P0 sequences y on its leader tick; P1 sequences x on its
      payloadBatch := fun i =>
        if i.val = 0 then [0, 1] else [0, 0, 1]
      rcEmit := ()
      p2aCh := ()
      p2aBatch := fun j =>
        if j.val = 0 then [{}, {}, {}, {p2aB}]
        else if j.val = 1 then [{}, {p2aA}, {}, {p2aB}]
        else [{}, {p2aA}]
      p2bCh := ()
      -- P0's acks from acceptors {1, 2}; P1's from acceptors {0, 1}
      cqBatch := fun i =>
        if i.val = 0 then
          [{}, {((0, bA), .ok ()), ((0, bA), .ok ())}]
        else
          [{}, {}, {((0, bB), .ok ()), ((0, bB), .ok ())}]
      cqEmit := ()
      jrBatch := fun i =>
        if i.val = 0 then [{}, {((0, bA), ())}]
        else [{}, {}, {((0, bB), ())}]
      jrEmit := () }
  fuelSeqMax := (1 : UnfoldFuel)
  fuelALog := (3 : UnfoldFuel)

/-- Clients: proposer 0's client sends `y`, proposer 1's sends `x`. -/
def clients : Fin 2 → List Nat :=
  fun i => if i.val = 0 then [yv] else [xv]

/-- No checkpoints. -/
def ckpt : Fin 3 → Trace (Option Nat) :=
  fun _ => [none, none, none, none]

/-! ## B2: every-tick recommit (module-face witness)

The recommit gate + `recommitList`, scanned over two leader ticks that
re-present the same (frozen) accepted-log view. -/

/-- A one-entry recovered log, re-presented on two consecutive leader
ticks. -/
def b2View : Multiset (ALog Nat 2) := {(none, [(0, ⟨⟨0, 0⟩, some 99⟩)])}

/-- Two leader ticks at ballot `(1,1)`: `((ballot, leader),
leader-deferred)`. -/
def b2Ticks : List ((Ballot 2 × Bool) × Bool) :=
  [((⟨1, 1⟩, true), false), ((⟨1, 1⟩, true), true)]

/-- Faithful: the recommit fires on both ticks (duplicate keys). -/
def b2Faithful : List ((Nat × Ballot 2) × Option Nat) :=
  ((scanAcrossTicksTrace
    (fun (s : Option (Ballot 2)) (x : Multiset (ALog Nat 2)
        × ((Ballot 2 × Bool) × Bool)) =>
      spGateStep false s x.1 x.2)
    none (b2Ticks.map (fun x => (b2View, x)))).map
    (fun v => recommitList 1 (⟨1, 1⟩ : Ballot 2) v)).flatten

/-- Guarded: once per ballot. -/
def b2Guarded : List ((Nat × Ballot 2) × Option Nat) :=
  ((scanAcrossTicksTrace
    (fun (s : Option (Ballot 2)) (x : Multiset (ALog Nat 2)
        × ((Ballot 2 × Bool) × Bool)) =>
      spGateStep true s x.1 x.2)
    none (b2Ticks.map (fun x => (b2View, x)))).map
    (fun v => recommitList 1 (⟨1, 1⟩ : Ballot 2) v)).flatten

-- The B2 violation, machine-checked: the faithful gate emits the same
-- `(slot, ballot)` key twice.
#guard (b2Faithful.map (·.1)).Nodup = false
-- The guarded gate's keys are duplicate-free.
#guard (b2Guarded.map (·.1)).Nodup = true

end Falsification
end HydroV2
