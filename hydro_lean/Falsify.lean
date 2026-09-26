import HydroV2.Paxos.Falsification
import HydroV2.Paxos.EagerCheck

/-!
# `lake exe falsify` — run the B1/B2 falsification scripts (V2)

Executes the FINDINGS.md B1 decision record against the faithful and
guarded `paxos_core` models and reports the committed slots, plus the
B2 module-face witness.

The program runs at the **`Eager` interpretation** (`Eager.lean`): the
`Values` denotation with materialized carriers, so `paxos_core` applied
to the decision record IS the executable — no harness, no restated
wiring; every wire's data carries its agreement with the `Values`
denotation by type.
-/

open HydroV2 HydroV2.Falsification

abbrev HE := Eager PaxLoc (paxMem 2 3)

/-- The clients, materialized: proposer 0 sends `y`, proposer 1 `x`. -/
def cpE : HE.Stream .prop Nat .totalOrder .exactlyOnce :=
  EagStream.input ⟨#[[yv], [xv]], rfl⟩

/-- No checkpoints (four acceptor ticks). -/
def ckE : HE.TickSingleton .acc (Option Nat) .unbounded :=
  EagTickSing.input ⟨#[[none, none, none, none], [none, none, none, none],
    [none, none, none, none]], rfl⟩

/-- `paxos_core` at the B1 record — the executable is the program, and
what it prints is **provably** the `Values` denotation at the same
decisions (`paxos_eager_commits`, `EagerCheck.lean`). -/
def paxosE (variant : PaxosVariant) :
    Vector (Multiset (Nat × Option Nat)) 2 :=
  (paxos_core HE variant .prop .acc 1 cpE ckE (pcdecE b1Dec)).val.2.data

/-- Slot-disagreement between two commit pools. -/
def disagrees (a b : Multiset (Nat × Option Nat)) : Bool :=
  !(decide (∀ v ∈ a, ∀ w ∈ b, v.1 = w.1 → v.2 = w.2))

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
  IO.println "== B2: every-tick recommit (paxos.rs:596–672 + 706–734) =="
  IO.println s!"faithful send-side emissions: {repr b2Faithful}"
  IO.println s!"guarded  send-side emissions: {repr b2Guarded}"
  let b2fNodup := decide ((b2Faithful.map (·.1)).Nodup)
  let b2gNodup := decide ((b2Guarded.map (·.1)).Nodup)
  IO.println s!"faithful keys duplicate-free: {b2fNodup} (expected: false — the B2 bug)"
  IO.println s!"guarded  keys duplicate-free: {b2gNodup} (expected: true)"
  if disagree && gAgree && fy && fx && gy && g1e && !b2fNodup && b2gNodup then
    IO.println "\nAll falsification checks PASSED."
    return 0
  else
    IO.println "\nFALSIFICATION CHECKS FAILED."
    return 1
