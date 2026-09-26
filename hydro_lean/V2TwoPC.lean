import HydroV2.TwoPC
import HydroV2.Paxos.EagerCheck

/-!
# `v2twopc` — executable non-vacuity for the 2PC port

One coordinator, two participants, payloads `{10, 20}`; participant 1
votes **no** on `20`. Complete decisions (both quorum stages consume
their whole pools). Expected (the `two_pc_commit_iff` face,
executably): `10` commits (unanimous yes), `20` does not.

Runs at the `Eager` interpretation — the program applied to the
decision record is the executable.
-/

open HydroV2

inductive TPCLoc | coord | part
deriving DecidableEq, Repr

@[reducible] def tpcMem : TPCLoc → Nat
  | .coord => 1
  | .part => 2

abbrev HT := Eager TPCLoc tpcMem

/-- Participant 1 rejects payload 20. -/
def vt : Fin 2 → Nat → Bool := fun j p => !(j.val = 1 && p = 20)

/-- The complete decision record: each stage consumes its whole pool in
one tick. -/
def tdec : TwoPCDec HT 1 2 Nat where
  prepCh := ()
  voteCh := ()
  votes := fun _ => [{(10, .ok ()), (20, .ok ()), (10, .ok ()),
    (20, .error ())}]
  votesEmit := ()
  commitCh := ()
  ackCh := ()
  acks := fun _ => [{(10, .ok ()), (10, .ok ())}]
  acksEmit := ()

def runT :=
  (two_pc HT .coord .part 2 vt
    (EagStream.input ⟨#[[10, 20]], rfl⟩) tdec).val

def main : IO UInt32 := do
  let t0 ← IO.monoMsNow
  let commits ← IO.lazyPure (fun _ => runT.data.get 0)
  let ok10 := decide ((10 : Nat) ∈ commits)
  let no20 := decide (¬ ((20 : Nat) ∈ commits))
  let once := decide (commits = ({10} : Multiset Nat))
  let t1 ← IO.monoMsNow
  IO.println "== two_pc (hydro_test/src/cluster/two_pc.rs) at Eager =="
  IO.println s!"unanimous payload 10 commits: {ok10}"
  IO.println s!"vetoed payload 20 does not commit: {no20}"
  IO.println s!"commits are exactly the singleton 10: {once} ({t1 - t0} ms)"
  unless ok10 && no20 && once do
    IO.println "v2twopc: FAILED"
    return 1
  IO.println "v2twopc: OK"
  return 0
