import Hydro.Paxos.Falsification
import Hydro.Paxos.EagerCheck

/-!
# `lake exe falsify` — run the B1/B2 falsification scripts

Executes the FINDINGS.md B1 decision record against the faithful and
guarded `paxos_core` models and reports the committed slots, plus the
B2 program witness (the faithful `sequence_payload` commits two values at
one slot) and the same scenario driven through `paxos_core` (D65).

The program runs at the **`Eager` interpretation** (`Eager.lean`): the
`Values` denotation with materialized carriers, so `paxos_core` applied
to the decision record IS the executable — no harness, no restated
wiring; every wire's data carries its agreement with the `Values`
denotation by type.
-/

open Hydro Hydro.Falsification

abbrev HE := Eager PaxLoc (paxMem 2 3)

/-- The clients, materialized: proposer 0 sends `y`, proposer 1 `x`. -/
def cpE : HE.Stream .prop Nat .totalOrder .exactlyOnce :=
  EagStream.input ⟨#[[yv], [xv]], rfl⟩

/-- No checkpoints: the async optional's read is `none` at every cut
(B2 — the snapshot happens inside `acceptor_p2`). -/
def ckE : HE.Singleton .acc Nat (Option Nat) .totalOrder .exactlyOnce
    .unbounded :=
  EagSing.input (α := Nat) (ord := .totalOrder)
      (ret := .exactlyOnce) ⟨#[fun (d : List Nat) => d.map (fun _ => none),
    fun (d : List Nat) => d.map (fun _ => none),
    fun (d : List Nat) => d.map (fun _ => none)], rfl⟩

/-- `paxos_core` at the B1 record — the executable is the program, and
what it prints is **provably** the `Values` denotation at the same
decisions (`paxos_eager_commits`, `EagerCheck.lean`). -/
def paxosE (variant : PaxosVariant) :
    Vector (Multiset (Nat × Option Nat)) 2 :=
  (paxos_core HE variant .prop .acc 1 cpE ckE (pcdecE b1Dec) pcschedE).2.data

/-- Slot-disagreement between two commit pools. -/
def disagrees (a b : Multiset (Nat × Option Nat)) : Bool :=
  !(decide (∀ v ∈ a, ∀ w ∈ b, v.1 = w.1 → v.2 = w.2))

/-! ## B2 through `paxos_core` (D65 rider)

The module-level B2 witness (`Falsification.lean`, `b2Run`) driven
through the whole program: two proposers, three acceptors, `f = 1`.
Proposer 0 (ballot `(0,0)`) is elected by acceptors {0, 1} and sends
`99` at slot 0; only acceptor 0 accepts it (never acked by a quorum).
Proposer 1 (ballot `(0,1)`) is then elected by acceptors {0, 2} — its
P1b view carries acceptor 0's slot-0 entry (count `1 < f + 1`, so the
reign recommits it and REBASES to `max_slot + 1 = 1`) — and leads for
two ticks, sequencing one fresh payload each (`x = 10`, then `y = 20`),
with the view re-read on both leader ticks. Faithful: the gate fires on
both ticks, the base is re-pinned, `y` lands at `x`'s slot 1 and
proposer 1 commits `(1, x)` AND `(1, y)`. Guarded: the gate fires once,
`y` takes slot 2. (`ySlot` is the only variant-dependent datum of the
record: the acceptors' P2a batches and the ack/join batches name `y`'s
placement, and a cut naming a P2a the program never sent would be
illegal — `batchCuts` truncates the chain there.) -/

/-- Proposer 0's client sends `99`; proposer 1's sends `x` then `y`. -/
def cpE3 : HE.Stream .prop Nat .totalOrder .exactlyOnce :=
  EagStream.input ⟨#[[99], [xv, yv]], rfl⟩

/-- Acceptor 0's log after `99`: the one-entry view proposer 1 recovers. -/
def b2cL0 : ALog Nat 2 := (none, [(0, ⟨bA, some 99⟩)])
/-- Proposer 0's `99` at slot 0. -/
def b2cP99 : P2a Nat 2 := ⟨0, bA, 0, some 99⟩
/-- Proposer 1's recommit of the recovered entry. -/
def b2cR : P2a Nat 2 := ⟨1, bB, 0, some 99⟩
/-- Proposer 1's `x` at slot 1 (both variants). -/
def b2cX : P2a Nat 2 := ⟨1, bB, 1, some xv⟩
/-- Proposer 1's `y` at `ySlot` (1 faithful, 2 guarded). -/
def b2cY (ySlot : Nat) : P2a Nat 2 := ⟨1, bB, ySlot, some yv⟩

/-- The B2 record at `paxos_core`. Timelines: proposer 0 = 2 ticks
(trigger; elect + sequence `99`); proposer 1 = 3 ticks (trigger; elect +
recommit + sequence `x`; [re-recommit +] sequence `y` + acks + commits);
acceptor 0 = 5 ticks (P1a `(0,0)`; P2a `99`; P1a `(0,1)`; P2as
recommit + `x`; P2a `y`), acceptor 1 = 4 (P1a `(0,0)`; P1a `(0,1)`;
recommit + `x`; `y`), acceptor 2 = 3 (P1a `(0,1)`; recommit + `x`;
`y`). Proposer 1 consumes two acks at each committed key. -/
def b2cDec (ySlot : Nat) :
    PaxosCoreDec (Values PaxLoc (paxMem 2 3)) 2 3 Nat Nat .totalOrder where
  le :=
    { receivedMax := fun i => if i.val = 0 then [{}, {}] else [{}, {}, {}]
      hb := { sample := fun _ => []
              timeout := fun i =>
                if i.val = 0 then [true, false] else [true, false, false]
              interval := fun i =>
                if i.val = 0 then [true, false] else [true, false, false] }
      p1aBatch := fun j =>
        if j.val = 0 then [{bA}, {}, {bB}, {}, {}]
        else if j.val = 1 then [{bA}, {bB}, {}, {}]
        else [{bB}, {}, {}]
      p1b := { cqwr := fun i =>
                 if i.val = 0 then
                   [{}, {(bA, .ok (none, [])), (bA, .ok (none, []))}]
                 else [{}, {(bB, .ok b2cL0), (bB, .ok (none, []))}, {}]
               order := fun i =>
                 if i.val = 0 then [(bA, (none, [])), (bA, (none, []))]
                 else [(bB, b2cL0), (bB, (none, []))]
               -- proposer 1's two P1bs are read on BOTH leader ticks
               snap := fun i => if i.val = 0 then [0, 2] else [0, 2, 0] }
      fuelFail := (1 : UnfoldFuel)
      fuelIAL := (1 : UnfoldFuel)
      fuelLead := (3 : UnfoldFuel) }
  sp :=
    { payloadBatch := fun i => if i.val = 0 then [0, 1] else [0, 1, 1]
      ap2 :=
        { p2aBatch := fun j =>
            if j.val = 0 then [{}, {b2cP99}, {}, {b2cR, b2cX}, {b2cY ySlot}]
            else if j.val = 1 then [{}, {}, {b2cR, b2cX}, {b2cY ySlot}]
            else [{}, {b2cR, b2cX}, {b2cY ySlot}]
          ckSnap := fun j =>
            if j.val = 0 then [0, 0, 0, 0, 0]
            else if j.val = 1 then [0, 0, 0, 0] else [0, 0, 0] }
      -- two acks at each key `(0, bB)`, …, `(ySlot, bB)`; join all keys
      cqBatch := fun i =>
        if i.val = 0 then [{}, {}]
        else [{}, {}, Multiset.ofList ((List.range (ySlot + 1)).flatMap
          fun s => [((s, bB), .ok ()), ((s, bB), .ok ())])]
      jrBatch := fun i =>
        if i.val = 0 then [{}, {}]
        else [{}, {}, Multiset.ofList ((List.range (ySlot + 1)).map
          fun s => ((s, bB), ()))] }
  fuelSeqMax := (1 : UnfoldFuel)
  fuelALog := (3 : UnfoldFuel)

/-- `paxos_core` at the B2 record (`y` placed where `variant` puts it):
proposer 1's commit pool. -/
def paxosB2 (variant : PaxosVariant) : Multiset (Nat × Option Nat) :=
  let ySlot := match variant with | .faithful => 1 | .guarded => 2
  ((paxos_core HE variant .prop .acc 1 cpE3 ckE (pcdecE (b2cDec ySlot))
    pcschedE).2.data).get 1

def main : IO UInt32 := do
  IO.println "== B1: duplicate P1a broadcasts (paxos.rs:311–317) =="
  let t0 ← IO.monoMsNow
  let f ← IO.lazyPure (fun _ => paxosE .faithful)
  let g ← IO.lazyPure (fun _ => paxosE .guarded)
  let fy := decide (f.get 0 = ({(0, some yv)} : Multiset (Nat × Option Nat)))
  let fx := decide (f.get 1 = ({(0, some xv)} : Multiset (Nat × Option Nat)))
  let gy := decide (g.get 0 = ({(0, some yv)} : Multiset (Nat × Option Nat)))
  let g1e := decide (g.get 1 = (0 : Multiset (Nat × Option Nat)))
  let t1 ← IO.monoMsNow
  IO.println s!"faithful: proposer 0 commits (0, some {yv}): {fy}"
  IO.println s!"faithful: proposer 1 commits (0, some {xv}): {fx}"
  IO.println s!"guarded:  proposer 0 commits (0, some {yv}): {gy}"
  IO.println s!"guarded:  proposer 1 commits nothing: {g1e}"
  let disagree := disagrees (f.get 0) (f.get 1)
  let gAgree := !(disagrees (g.get 0) (g.get 1))
  IO.println s!"faithful DISAGREES at a slot: {disagree} (expected: true — the B1 bug)"
  IO.println s!"guarded agrees: {gAgree} (expected: true — the proven theorem) ({t1 - t0} ms)"
  IO.println ""
  IO.println "== B2: every-tick rebase (paxos.rs:596–672 + 706–734) =="
  let b2f := b2Run .faithful
  let b2g := b2Run .guarded
  IO.println s!"faithful commits (1, x={xv}): {decide ((1, some 10) ∈ b2f)}, (1, y=20): {decide ((1, some 20) ∈ b2f)}"
  IO.println s!"guarded  commits (1, x={xv}): {decide ((1, some 10) ∈ b2g)}, (1, y=20): {decide ((1, some 20) ∈ b2g)}"
  let b2fDis := disagreesWithin b2f
  let b2gOk := !(disagreesWithin b2g)
  IO.println s!"faithful commits two values at one slot: {b2fDis} (expected: true — the B2 bug)"
  IO.println s!"guarded  commits are slot-functional: {b2gOk} (expected: true)"
  IO.println ""
  IO.println "== B2 through paxos_core (two proposers, three acceptors, f = 1) =="
  let t2 ← IO.monoMsNow
  let c2f ← IO.lazyPure (fun _ => paxosB2 .faithful)
  let c2g ← IO.lazyPure (fun _ => paxosB2 .guarded)
  let c2fx := decide ((1, some xv) ∈ c2f)
  let c2fy := decide ((1, some yv) ∈ c2f)
  let c2gx := decide ((1, some xv) ∈ c2g)
  let c2gy := decide ((2, some yv) ∈ c2g)
  let t3 ← IO.monoMsNow
  IO.println s!"faithful: proposer 1 commits (1, x={xv}): {c2fx}, (1, y={yv}): {c2fy}"
  IO.println s!"guarded:  proposer 1 commits (1, x={xv}): {c2gx}, (2, y={yv}): {c2gy}"
  let c2fDis := disagreesWithin c2f
  let c2gOk := !(disagreesWithin c2g)
  IO.println s!"faithful commits two values at one slot: {c2fDis} (expected: true — the B2 bug, whole program)"
  IO.println s!"guarded  commits are slot-functional: {c2gOk} (expected: true — the proven theorem) ({t3 - t2} ms)"
  if disagree && gAgree && fy && fx && gy && g1e && b2fDis && b2gOk
      && c2fx && c2fy && c2gx && c2gy && c2fDis && c2gOk then
    IO.println "\nAll falsification checks PASSED."
    return 0
  else
    IO.println "\nFALSIFICATION CHECKS FAILED."
    return 1
