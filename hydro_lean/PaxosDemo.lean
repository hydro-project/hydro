import Hydro

/-!
# `paxos` — executable non-vacuity (arc M5, at `Values`)

The end-to-end guarded-commit scenario: one proposer, one acceptor,
`f = 0` — leader elected at proposer tick 1, payload `42` sequenced at
slot 0, accepted through the same-tick `a_log` knot (write-before-ack),
committed by the singleton quorum, landing at the replica as
`(0, some 42)`.

The program runs at the **`Values` interpretation itself** (D73):
`paxos_core` applied to the decision record IS the executable — the
denotation's member-indexed carriers are `MemberMap`s whose compiled
constructor forces each member's cell once (`MemberMap.mkImpl_eq`), so
the very term the headline theorems quantify over runs in one strict
dataflow pass. History: the same scenario needed hours at call-by-name
`Values` (D14), ~2 minutes through a hand-built IO harness (M5), and
milliseconds at the `Eager` interpretation D73 retired.
-/

open Hydro

abbrev HVE := Values PaxLoc (paxMem 1 1)

/-- The guarded-commit decision record. -/
def pcDec : PaxosCoreDec (Values PaxLoc (paxMem 1 1)) 1 1 Nat
    Nat .totalOrder :=
  { le := { receivedMax := fun _ => [{}, {}]
            hb := { sample := ⟨fun _ => [[], []]⟩
                    -- nothing ever arrives: the check timeout is expired at both ticks
                    timeout := { arrivals := fun _ => [{}, {}], clock := fun _ => [0, 1] }
                    timeoutSnap := fun _ => [1, 1]
                    interval := fun _ => [[()], [()]] }
            p1aBatch := fun _ => [{Ballot.mk 0 0}, {}]
            p1b := { cqwr := fun _ =>
                       [{(Ballot.mk 0 0, .ok (none, HMap.ofList []))}, {}]
                     order := fun _ => [(Ballot.mk 0 0, (none, HMap.ofList []))]
                     snap := fun _ => [{}, {(Ballot.mk 0 0, (none, HMap.ofList []))}] }
            fuelFail := (1 : UnfoldFuel)
            fuelIAL := (1 : UnfoldFuel)
            fuelLead := (3 : UnfoldFuel) }
    sp := { payloadBatch := fun _ => [0, 1]
            rc := ⟨fun _ => [[], []]⟩
            ap2 := { p2aBatch :=
                       fun _ => [{}, {⟨0, Ballot.mk 0 0, 0, some 42⟩}]
                     ckSnap := fun _ => [0, 0] }
            cqBatch := fun _ => [{}, {((0, Ballot.mk 0 0), .ok ())}]
            jrBatch := fun _ => [{}, {((0, Ballot.mk 0 0), ())}] }
    fuelSeqMax := (1 : UnfoldFuel)
    fuelALog := (3 : UnfoldFuel) }

/-- The election sub-scenario (tick-1 election check). -/
def leRunE :=
  (leader_election HVE .guarded .prop .acc 1 1 ⟨0, 5, 10, 15⟩
    pcDec.le PaxosCoreSched.triv.le
    (.mk' fun _ => (0 : Multiset (Ballot 1)))
    (.mk' fun _ => [(none, HMap.ofList []), (none, HMap.ofList [])]))

/-- The full run — the program applied to the record. -/
def pcRunE :=
  (paxos_core HVE (ckret := .exactlyOnce) .guarded .prop .acc ⟨0, 5, 10, 15⟩
    (.mk' fun _ => [42])
    (.mk' fun _ => SingletonV.noneCell Nat .totalOrder)
    pcDec PaxosCoreSched.triv)

def main : IO UInt32 := do
  let t0 ← IO.monoMsNow
  let le ← IO.lazyPure (fun _ => decide ((leRunE.2.1 0) = [false, true]))
  let t1 ← IO.monoMsNow
  IO.println s!"paxos: leader_election elects at tick 1: {le} ({t1 - t0} ms)"
  let t2 ← IO.monoMsNow
  let replicas ← IO.lazyPure (fun _ => pcRunE.2 0)
  let ballots ← IO.lazyPure (fun _ => pcRunE.1 0)
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
  IO.println s!"paxos: (the same scenario needed hours at call-by-name Values and ~2 min through the retired IO harness — the Values program itself runs it in {t3 - t2} ms)"
  IO.println "paxos: OK"
  return 0
