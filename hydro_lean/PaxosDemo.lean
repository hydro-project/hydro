import Hydro

/-!
# `paxos` — executable non-vacuity (arc M5, at `Eager`)

The end-to-end guarded-commit scenario: one proposer, one acceptor,
`f = 0` — leader elected at proposer tick 1, payload `42` sequenced at
slot 0, accepted through the same-tick `a_log` knot (write-before-ack),
committed by the singleton quorum, landing at the replica as
`(0, some 42)`.

The program runs at the **`Eager` interpretation**: `paxos_core`
applied to the decision record IS the executable (one strict dataflow
pass over materialized carriers), and what it computes is provably the
`Values` denotation at the same decisions (`paxos_eager_commits`,
`EagerCheck.lean`) — the run below is the D15 non-vacuity witness for
the ∀-decisions headline. History: the same scenario needed hours at
naive call-by-name `Values` and ~2 minutes through the hand-built IO
materialization harness this file used to carry (FINDINGS D14/M5);
`Eager` retires the harness.
-/

open Hydro

abbrev HVE := Eager PaxLoc (paxMem 1 1)

/-- The guarded-commit decision record (at `Values`; `Eager` repacks). -/
def pcDec : PaxosCoreDec (Values PaxLoc (paxMem 1 1)) 1 1 Nat
    Nat .totalOrder :=
  { le := { receivedMax := fun _ => [{}, {}]
            hb := { sample := fun _ => []
                    timeout := fun _ => [true, true]
                    interval := fun _ => [true, true] }
            p1aBatch := fun _ => [{Ballot.mk 0 0}, {}]
            p1b := { cqwr := fun _ =>
                       [{(Ballot.mk 0 0, .ok (none, []))}, {}]
                     order := fun _ => [(Ballot.mk 0 0, (none, []))]
                     snap := fun _ => [0, 1] }
            fuelFail := (1 : UnfoldFuel)
            fuelIAL := (1 : UnfoldFuel)
            fuelLead := (3 : UnfoldFuel) }
    sp := { payloadBatch := fun _ => [0, 1]
            ap2 := { p2aBatch :=
                       fun _ => [{}, {⟨0, Ballot.mk 0 0, 0, some 42⟩}]
                     ckSnap := fun _ => [0, 0] }
            cqBatch := fun _ => [{}, {((0, Ballot.mk 0 0), .ok ())}]
            jrBatch := fun _ => [{}, {((0, Ballot.mk 0 0), ())}] }
    fuelSeqMax := (1 : UnfoldFuel)
    fuelALog := (3 : UnfoldFuel) }

/-- The election sub-scenario (tick-1 election check). -/
def leRunE :=
  (leader_election HVE .guarded .prop .acc 1 1
    (ledecE pcDec.le) pcschedE.le
    (EagStream.input ⟨#[(0 : Multiset (Ballot 1))], rfl⟩)
    (EagTickSing.input ⟨#[[(none, []), (none, [])]], rfl⟩))

/-- The full run — the program applied to the record. -/
def pcRunE :=
  (paxos_core HVE .guarded .prop .acc 0
    (EagStream.input ⟨#[[42]], rfl⟩)
    (EagSing.input (α := Nat) (ord := .totalOrder)
      (ret := .exactlyOnce) ⟨#[fun (d : List Nat) => d.map (fun _ => none)], rfl⟩)
    (pcdecE pcDec) pcschedE)

def main : IO UInt32 := do
  let t0 ← IO.monoMsNow
  let le ← IO.lazyPure (fun _ => decide ((leRunE.2.1.data.get 0) = [false, true]))
  let t1 ← IO.monoMsNow
  IO.println s!"paxos: leader_election elects at tick 1: {le} ({t1 - t0} ms)"
  let t2 ← IO.monoMsNow
  let replicas ← IO.lazyPure (fun _ => pcRunE.2.data.get 0)
  let ballots ← IO.lazyPure (fun _ => pcRunE.1.data.get 0)
  let okReplicas := decide
    (replicas = ({(0, some 42)} : Multiset (Nat × Option Nat)))
  let okBallots := decide (ballots = [Ballot.mk 0 0])
  let okSlotFn := decide (∀ v ∈ replicas, ∀ w ∈ replicas,
    v.1 = w.1 → v.2 = w.2)
  let t3 ← IO.monoMsNow
  IO.println s!"paxos: commit (0, some 42) lands: {okReplicas}; new-leader announcement once: {okBallots}; slot-functional: {okSlotFn} ({t3 - t2} ms)"
  unless le && okReplicas && okBallots && okSlotFn do
    IO.println "paxos: FAILED"
    return 1
  IO.println s!"paxos: (the same scenario needed hours at call-by-name Values and ~2 min through the retired IO harness — Eager runs it in {t3 - t2} ms, provably the same denotation)"
  IO.println "paxos: OK"
  return 0
