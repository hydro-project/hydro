import HydroV2

/-!
# `v2paxos` — executable non-vacuity + micro-benchmark (arc M5)

**The perf finding, executably**: `Values` carriers are member-indexed
*functions*, so evaluation is call-by-name — every wire access re-runs
its producing chain, and sharing compounds multiplicatively through
module composition and Kleene knots: one `leader_election` (three
nested knots) evaluates in ~1 ms, while the naive `paxos_core`
composition of the *same* two-tick scenario was aborted after 15+ minutes. Pure
`let`-forcing does not help (the compiler sinks pure lets into
closures); the fix requires evaluating wires **as data, in evaluation
order** — the eager `Machine` interpretation (staged). This demo
realizes that move in `IO` at the module/knot boundaries: each wire is
materialized once (`IO.lazyPure`), and the same scenario drops back to
milliseconds.

The scenario: one proposer, one acceptor, `f = 0` — leader elected at
proposer tick 1, payload `42` sequenced at slot 0, accepted through the
same-tick `a_log` knot (write-before-ack), committed by the singleton
quorum, landing at the replica as `(0, some 42)`.
-/

open HydroV2

abbrev HV := Values PaxLoc (paxMem 1 1)

def pcDec : PaxosCoreDec 1 1 Nat :=
  { le := { receivedMax := fun _ => [{}, {}]
            hb := { sample := fun _ => []
                    timeout := fun _ => [true, true]
                    interval := fun _ => [true, true] }
            p1aBatch := fun _ => [{Ballot.mk 0 0}, {}]
            p1b := { order := fun _ => [(Ballot.mk 0 0, (none, []))]
                     snap := fun _ => [0, 1] }
            fuelFail := 1
            fuelIAL := 1
            fuelLead := 3 }
    sp := { payloadBatch := fun _ => [0, 1]
            p2aBatch := fun _ => [{}, {⟨0, Ballot.mk 0 0, 0, some 42⟩}]
            p2bBatch := fun _ => [{}, {⟨0, Ballot.mk 0 0, .ok ()⟩}] }
    fuelSeqMax := 1
    fuelALog := 3 }

/-- One `paxos_core_body` pass with every module-boundary wire
materialized in `IO` (the eager-`Machine` move; extensionally the
`Values` run). Returns concrete data: (new-leader ballots, replica
pool, a_log trace, fail pool). -/
def bodyIO (payload : Nat) (sm : Multiset (Ballot 1))
    (al : Trace (ALog Nat 1)) :
    IO (List (Ballot 1) × Multiset (Nat × Option Nat)
      × Trace (ALog Nat 1) × Multiset (Ballot 1)) := do
  let le := (leader_election HV .guarded .prop .acc 1
    pcDec.le 0 1 2 3 10 11 12).val (fun _ => sm) (fun _ => al)
  let pBallotV ← IO.lazyPure (fun _ => HV.forgetBound le.1 0)
  let pIsLeadV ← IO.lazyPure (fun _ => le.2.1 0)
  let acceptedV ← IO.lazyPure (fun _ => le.2.2.1 0)
  let aMaxV ← IO.lazyPure (fun _ => HV.forgetBound le.2.2.2 0)
  let pBallot : TickV 1 (Ballot 1) .unbounded := fun _ => pBallotV
  let pIsLead : TickV 1 Bool .unbounded := fun _ => pIsLeadV
  let accepted : Fin 1 → Trace (Multiset (ALog Nat 1)) :=
    fun _ => acceptedV
  let aMax : TickV 1 (Option (Ballot 1)) .unbounded := fun _ => aMaxV
  let just_became_leader := HV.mapTick (ℓ := .prop)
    (HV.zipTick (ℓ := .prop) pIsLead (HV.defer (ℓ := .prop) false pIsLead))
    (fun _me x => x.1 && !x.2)
  let sp := (sequence_payload HV .guarded .prop .acc (fun _ => [payload])
    (fun _ => [none, none]) pBallot pIsLead accepted 0 aMax
    pcDec.sp 4 5).val
  let ballots ← IO.lazyPure (fun _ =>
    HV.allTicks (HV.emitBatches (HV.mapTick (ℓ := .prop)
      (HV.zipTick (ℓ := .prop) pBallot just_became_leader)
      (fun _me x => if x.2 then [x.1] else []))) 0)
  let repl ← IO.lazyPure (fun _ => sp.1 0)
  let alog ← IO.lazyPure (fun _ => sp.2.1 0)
  let fails ← IO.lazyPure (fun _ => sp.2.2 0)
  return (ballots, repl, alog, fails)

/-- The two outer Kleene chains, materialized stepwise. -/
def pcRunIO (payload : Nat) :
    IO (List (Ballot 1) × Multiset (Nat × Option Nat)) := do
  let alogChain := fun (sm : Multiset (Ballot 1)) => do
    let mut al : Trace (ALog Nat 1) := []
    for _ in [0:pcDec.fuelALog] do
      let w ← bodyIO payload sm al
      al := w.2.2.1
    pure al
  let mut sm : Multiset (Ballot 1) := 0
  for _ in [0:pcDec.fuelSeqMax] do
    let al ← alogChain sm
    let w ← bodyIO payload sm al
    sm := w.2.2.2
  let al ← alogChain sm
  let w ← bodyIO payload sm al
  return (w.1, w.2.1)

def leRun :=
  (leader_election HV .guarded .prop .acc 1
    { receivedMax := fun _ => [{}, {}]
      hb := { sample := fun _ => []
              timeout := fun _ => [true, true]
              interval := fun _ => [true, true] }
      p1aBatch := fun _ => [{Ballot.mk 0 0}, {}]
      p1b := { order := fun _ => [(Ballot.mk 0 0, (none, []))]
               snap := fun _ => [0, 1] }
      fuelFail := 1, fuelIAL := 1, fuelLead := 3 }
    0 1 2 3 0 1 2).val
    (fun _ => (0 : Multiset (Ballot 1)))
    ((fun _ => [(none, []), (none, [])]) : TickV 1 (ALog Nat 1) .unbounded)

def main : IO UInt32 := do
  let t0 ← IO.monoMsNow
  let le ← IO.lazyPure (fun _ => decide (leRun.2.1 0 = [false, true]))
  let t1 ← IO.monoMsNow
  IO.println s!"v2paxos: leader_election elects at tick 1: {le} ({t1 - t0} ms)"
  let t2 ← IO.monoMsNow
  let run ← pcRunIO 42
  let replicas := run.2
  let ballots := run.1
  let okReplicas := decide
    (replicas = ({(0, some 42)} : Multiset (Nat × Option Nat)))
  let okBallots := decide (ballots = [Ballot.mk 0 0])
  let okSlotFn := decide (∀ v ∈ replicas, ∀ w ∈ replicas,
    v.1 = w.1 → v.2 = w.2)
  let t3 ← IO.monoMsNow
  IO.println s!"v2paxos: commit (0, some 42) lands: {okReplicas}; new-leader announcement once: {okBallots}; slot-functional: {okSlotFn} ({t3 - t2} ms)"
  unless le && okReplicas && okBallots && okSlotFn do
    IO.println "v2paxos: FAILED"
    return 1
  IO.println s!"v2paxos: (naive paxos_core on the same scenario needs hours — the eager-Machine staging above brings one run to {t3 - t2} ms)"
  IO.println "v2paxos: OK"
  return 0
