import Hydro.Paxos.PaxosCore

/-!
# Executable falsifications: FINDINGS.md B1/B2 (faithful paxos.rs)

The B1/B2 falsification scripts (FINDINGS B1/B2) in the decision
vocabulary (`PaxosCoreDec` at `Values PaxLoc (paxMem 2 3)`).

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

**B2** (paxos.rs:596–672 + 706–734): the recommit/rebase gate of
`sequence_payload`'s accepted-log view — the faithful gate fires on
*every* nonempty view, so a reign re-presenting its view REBASES slot
indexing on every tick: two fresh payloads on two ticks get the same
slot at the same ballot, and one proposer commits two values at one
slot (`SlotFunctional` violated). The guarded gate (`recommittedAt`)
fires once per ballot. Machine-checked below by `#guard` on the
program's output wire; printed by `lake exe falsify`.
-/

namespace Hydro
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
def b1Dec : PaxosCoreDec HB1 2 3 Nat Nat .totalOrder where
  le :=
    { -- neither proposer ever sees the other's ballot
      receivedMax := fun i =>
        if i.val = 0 then [{}, {}] else [{}, {}, {}]
      hb := { sample := fun _ => []
              -- P0 triggers at tick 0; P1 at ticks 0 AND 1 (same
              -- ballot — the faithful re-broadcast_closed)
              timeout := fun i =>
                if i.val = 0 then [true, false] else [true, true, false]
              interval := fun i =>
                if i.val = 0 then [true, false] else [true, true, false] }
      -- acceptor 0 consumes BOTH bB copies in one tick; acceptors 1, 2
      -- promise bA genuinely (acceptor 1 later promises bB)
      p1aBatch := fun j =>
        if j.val = 0 then [{}, {}, {bB, bB}, {}]
        else if j.val = 1 then [{bA}, {}, {bB}, {}]
        else [{bA}, {}]
      p1b := { -- P0 consumes the genuine bA quorum (acceptors 1, 2);
               -- P1 consumes acceptor 0's two duplicate Oks and leaves
               -- acceptor 1's log-carrying reply unconsumed
               cqwr := fun i =>
                 if i.val = 0 then
                   [{}, {(bA, .ok (none, [])), (bA, .ok (none, []))}]
                 else
                   [{}, {}, {(bB, .ok (none, [])), (bB, .ok (none, []))}]
               order := fun i =>
                 if i.val = 0 then [(bA, (none, [])), (bA, (none, []))]
                 else [(bB, (none, [])), (bB, (none, []))]
               snap := fun i =>
                 if i.val = 0 then [{}, {(bA, (none, [])), (bA, (none, []))}]
                 else [{}, {}, {(bB, (none, [])), (bB, (none, []))}] }
      fuelFail := (1 : UnfoldFuel)
      fuelIAL := (1 : UnfoldFuel)
      fuelLead := (3 : UnfoldFuel) }
  sp :=
    { -- P0 sequences y on its leader tick; P1 sequences x on its
      payloadBatch := fun i =>
        if i.val = 0 then [0, 1] else [0, 0, 1]
      -- the recommit fold's order (D71): neither election view carries
      -- an entry (P1's fake quorum is acceptor 0's two empty logs)
      rc := ⟨fun i => if i.val = 0 then [[], []] else [[], [], []]⟩
      ap2 :=
        { p2aBatch := fun j =>
            if j.val = 0 then [{}, {}, {}, {p2aB}]
            else if j.val = 1 then [{}, {p2aA}, {}, {p2aB}]
            else [{}, {p2aA}]
          -- checkpoint snapshot (B2): the acceptors read the (empty)
          -- checkpoint at each of their ticks
          ckSnap := fun _ => [0, 0, 0, 0] }
      -- P0's acks from acceptors {1, 2}; P1's from acceptors {0, 1}
      cqBatch := fun i =>
        if i.val = 0 then
          [{}, {((0, bA), .ok ()), ((0, bA), .ok ())}]
        else
          [{}, {}, {((0, bB), .ok ()), ((0, bB), .ok ())}]
      jrBatch := fun i =>
        if i.val = 0 then [{}, {((0, bA), ())}]
        else [{}, {}, {((0, bB), ())}] }
  fuelSeqMax := (1 : UnfoldFuel)
  fuelALog := (3 : UnfoldFuel)

/-- Clients: proposer 0's client sends `y`, proposer 1's sends `x`. -/
def clients : Fin 2 → List Nat :=
  fun i => if i.val = 0 then [yv] else [xv]

/-! ## B2: every-tick rebase (the program, run)

`sequence_payload` itself (one proposer, two acceptors, `f = 1`), run on
two leader ticks of one reign that re-present the same (frozen)
accepted-log view — one log carrying a slot-0 entry accepted at a prior
ballot by ONE acceptor (count `1 < f + 1`, so it is recommitted) — with
one fresh client payload per tick (`x = 10`, then `y = 20`).

The hazard is the REBASE, not the duplicate recommit: the faithful gate
lets the view through on every tick, so `index_payloads` re-pins its
base to `max_slot + 1 = 1` on BOTH ticks — `x` and `y` are both indexed
at slot 1, at the same ballot. Both P2as pass the acceptors' ballot
check and are acked at the one key `(1, b)`; the key is quorum'd and
joined with both sent values, so ONE proposer commits `(1, x)` and
`(1, y)`: two values at one slot (`SlotFunctional` violated — the
headline). The guarded gate fires once, `y` is indexed at slot 2, and
the commits are slot-functional. Observed on the module's OUTPUT wire
(`p_to_replicas`), never on an internal one. The same scenario driven
through `paxos_core` (two proposers, three acceptors: proposer 1
recovers proposer 0's lone slot-0 entry) runs in `lake exe falsify`
(`Falsify.lean`, `paxosB2`; exe-only — kernel `#guard` of the nested
knots is prohibitive). -/

/-- A one-entry recovered log (accepted at ballot `(0,0)` by one
acceptor), re-presented on two consecutive leader ticks. -/
def b2View : Multiset (ALog Nat 1) := {(none, [(0, ⟨⟨0, 0⟩, some 99⟩)])}

/-- The reign's ballot. -/
def b2B : Ballot 1 := ⟨1, 0⟩

/-- The recommit P2a (slot 0, the recovered entry). -/
def b2P2aR : P2a Nat 1 := ⟨0, b2B, 0, some 99⟩
/-- Tick 0's fresh payload `x`, indexed at slot 1 (both variants). -/
def b2P2aX : P2a Nat 1 := ⟨0, b2B, 1, some 10⟩
/-- Tick 1's fresh payload `y`, indexed at slot 1 again under the
faithful re-rebase (under guarded it sits at slot 2 and this P2a does
not exist — the acceptor batch naming it is illegal and dropped). -/
def b2P2aY : P2a Nat 1 := ⟨0, b2B, 1, some 20⟩

/-- `sequence_payload`'s replica stream at proposer 0 under `variant`.
Each acceptor consumes tick 0's two P2as, then `y`'s; acks each; the
quorum batch consumes the slot-0 and slot-1 acks; the join consumes the
quorum'd keys on the tick that sees both sends (`join_responses`'
metadata side is `atomic`). -/
def b2Run (variant : PaxosVariant) : Multiset (Nat × Option Nat) :=
  (sequence_payload (Values PaxLoc (paxMem 1 2)) variant .prop .acc
    (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce)
    (.mk' fun _ => [10, 20])                          -- client payloads x, y
    (ckNone (mem := paxMem 1 2) .acc)                 -- a_checkpoint
    (.mk' fun _ => [b2B, b2B])                        -- p_ballot: one reign
    (.mk' fun _ => [true, true])                      -- two leader ticks
    (.mk' fun _ => [b2View, b2View])                  -- the view, re-presented
    1
    (.mk' fun _ => [some b2B, some b2B])              -- a_max
    ⟨fun _ => [1, 1],                                 -- one payload per tick
     ⟨fun _ => if variant.recommitOnce               -- recommit orders: the view's one entry
        then [[(0, ⟨⟨0, 0⟩, some 99⟩)], []]           --   (guarded: tick 1's gate is closed;
        else [[(0, ⟨⟨0, 0⟩, some 99⟩)],              --    faithful: re-presented)
              [(0, ⟨⟨0, 0⟩, some 99⟩)]]⟩,
     ⟨fun _ => [{b2P2aR, b2P2aX}, {b2P2aY}],          -- p2a batches
      fun _ => [0, 0]⟩,                               -- checkpoint snap
     fun _ => [{((0, b2B), .ok ()), ((0, b2B), .ok ()),
                ((1, b2B), .ok ()), ((1, b2B), .ok ())}, {}],  -- cq acks
     fun _ => [{}, {((0, b2B), ()), ((1, b2B), ())}]⟩ -- jr key batches
    .triv).1 0

/-- Two commits at one slot with different values. -/
def disagreesWithin (a : Multiset (Nat × Option Nat)) : Bool :=
  !(decide (∀ v ∈ a, ∀ w ∈ a, v.1 = w.1 → v.2 = w.2))

-- The B2 violation, machine-checked: the faithful program commits
-- `(1, x)` AND `(1, y)` — two values at slot 1.
#guard_eval disagreesWithin (b2Run .faithful) = true
#guard_eval (1, some 10) ∈ b2Run .faithful ∧ (1, some 20) ∈ b2Run .faithful
-- The guarded program's commits are slot-functional.
#guard_eval disagreesWithin (b2Run .guarded) = false
#guard_eval (1, some 10) ∈ b2Run .guarded

/-! ## B4: a fresh payload at a collected slot (D71, GC)

`sequence_payload` (two proposers, one acceptor, `f = 0`) over two reigns.
Proposer 0 leads reign `b₀` and commits `x` at slot 0; the acceptor's
checkpoint then advances to 1 and `reduce_watermark` collects slot 0, so
the acceptor's published view is `(some 1, [])`. Proposer 1 — a leader
that never led before (its `index_payloads` register still reads 0) —
takes reign `b₁` over that view: nothing to recommit,
`p_p1b_max_checkpoint = some 1`, and — faithful — `p_max_slot =
keys().max() = None`, so `index_payloads` falls back to its register
(paxos.rs:788 `unwrap_or(next_slot)`) and indexes the fresh payload `y`
at slot 0. The acceptor accepts `(0, b₁, y)` (its max is `b₁`), acks
`Ok`, the singleton quorum commits `(0, y)` at proposer 1: two values at
slot 0 across the replicas' feed (`SlotFunctional` violated — the
headline, now that GC is modeled). Guarded (B4 fix): `p_max_slot` is
lifted to the view's checkpoint, `y` is indexed at slot 2. The
acceptor's `requires` holds throughout (the two slot-0 P2as carry
different ballots): the merge is never refused — the hazard is the
proposer's rebase. Upstream: paxos.rs:655 should take
`p_max_slot.max(p_p1b_max_checkpoint)` (SCHED_AUDIT). -/

/-- Reign `b₀` (proposer 0) and reign `b₁` (proposer 1). -/
def b4B0 : Ballot 2 := ⟨0, 0⟩
def b4B1 : Ballot 2 := ⟨1, 1⟩
/-- Proposer 0's payload `x`, at slot 0. -/
def b4P2aX : P2a Nat 2 := ⟨0, b4B0, 0, some 10⟩
/-- Proposer 1's payload `y`: at slot 0 under the faithful rebase (the
collected slot), at slot 2 under guarded. -/
def b4YSlot (variant : PaxosVariant) : Nat := if variant = .faithful then 0 else 2
def b4P2aY (variant : PaxosVariant) : P2a Nat 2 := ⟨1, b4B1, b4YSlot variant, some 20⟩

/-- Per-proposer timelines: proposer 0 leads at tick 0, proposer 1 at
tick 1 (over the collected view). -/
def b4At {α : Type} (a b : α) : Fin 2 → α := fun i => if i.val = 0 then a else b

/-- `sequence_payload`'s replica feeds under `variant`, both proposers'
commits summed. (The decisions name the P2a/ack/key the variant actually
produces: a batch increment naming an absent element is illegal and
dropped.) -/
def b4Run (variant : PaxosVariant) : Multiset (Nat × Option Nat) :=
  let r := sequence_payload (Values PaxLoc (paxMem 2 1)) variant .prop .acc
    (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce)
    (.mk' (b4At [10] [20]))                           -- clients: x at 0, y at 1
    (ckStep (mem := paxMem 2 1) .acc 1)               -- a_checkpoint: none, then 1
    (.mk' (b4At [b4B0, b4B0] [b4B1, b4B1]))           -- p_ballot
    (.mk' (b4At [true, false] [false, true]))         -- p_is_leader
    (.mk' (b4At [{((none : Option Nat), [])}, {}]
          [{}, {((some 1 : Option Nat), [])}]))       -- the views
    0
    (.mk' fun _ => [some b4B0, some b4B1])            -- a_max
    ⟨b4At [1, 0] [0, 1],                              -- payload batches
     ⟨b4At [[], []] [[], []]⟩,                         -- recommit orders (entry-less views)
     ⟨fun _ => [{b4P2aX}, {b4P2aY variant}],          -- p2a batches
      fun _ => [0, 0]⟩,                               -- checkpoint snap
     b4At [{((0, b4B0), .ok ())}, {}] [{}, {((b4YSlot variant, b4B1), .ok ())}],  -- cq acks
     b4At [{((0, b4B0), ())}, {}] [{}, {((b4YSlot variant, b4B1), ())}]⟩  -- jr key batches
    .triv
  r.1 0 + r.1 1

-- The B4 violation, machine-checked: the faithful program commits
-- `(0, x)` AND `(0, y)` — two values at the collected slot 0.
#guard_eval disagreesWithin (b4Run .faithful) = true
#guard_eval (0, some 10) ∈ b4Run .faithful ∧ (0, some 20) ∈ b4Run .faithful
-- The guarded program rebases above the checkpoint: `y` lands at slot 2.
#guard_eval disagreesWithin (b4Run .guarded) = false
#guard_eval (0, some 10) ∈ b4Run .guarded ∧ (2, some 20) ∈ b4Run .guarded

end Falsification
end Hydro
