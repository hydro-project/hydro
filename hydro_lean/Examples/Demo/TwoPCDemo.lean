import Examples.TwoPC

/-!
# `lake exe twopc` — executable non-vacuity for the 2PC port

One coordinator, two participants, payloads `{10, 20}`; participant 1
votes no on `20`. Complete decisions (both quorum stages consume their
whole pools). Expected: `10` commits (unanimous yes, the scenario of
`TPCEnsures.commit_unanimous`), `20` does not, and the commit set is
`{10}` (`commit_once`). Exit code 0 when all three hold.

Runs at the `Values` interpretation: the program applied to the decision
record is the executable. The same scenario is a `#guard` in
`Examples/TwoPC.lean`.
-/

open Hydro

inductive TPCLoc | coord | part
deriving DecidableEq, Repr

@[reducible] def tpcMem : TPCLoc → Nat
  | .coord => 1
  | .part => 2

abbrev HT := Values TPCLoc tpcMem

/-- Participant 1 rejects payload 20. -/
def vt : Fin 2 → Nat → Bool := fun j p => !(j.val = 1 && p = 20)

/-- The complete decision record: each stage consumes its whole pool in
one tick. -/
def tdec : TwoPCDec HT 1 2 Nat where
  votes := fun _ => [{(10, .ok ()), (20, .ok ()), (10, .ok ()),
    (20, .error ())}]
  acks := fun _ => [{(10, .ok ()), (10, .ok ())}]

/-- The scheduling record (all `Unit` at `Values`). -/
def tsched : TwoPCSched HT 1 2 Nat :=
  ⟨(), (), (), ()⟩

def runT :=
  two_pc HT .coord .part 2 vt
    (.mk' fun _ => ({10, 20} : Multiset Nat)) tdec tsched

def main : IO UInt32 := do
  let t0 ← IO.monoMsNow
  let commits ← IO.lazyPure (fun _ => runT 0)
  let ok10 := decide ((10 : Nat) ∈ commits)
  let no20 := decide (¬ ((20 : Nat) ∈ commits))
  let once := decide (commits = ({10} : Multiset Nat))
  let t1 ← IO.monoMsNow
  IO.println "== two_pc (hydro_test/src/cluster/two_pc.rs) at Values =="
  IO.println s!"unanimous payload 10 commits: {ok10}"
  IO.println s!"vetoed payload 20 does not commit: {no20}"
  IO.println s!"commits are exactly the singleton 10: {once} ({t1 - t0} ms)"
  unless ok10 && no20 && once do
    IO.println "twopc: FAILED"
    return 1
  IO.println "twopc: OK"
  return 0
