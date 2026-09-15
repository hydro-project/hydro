import HydroLean.Programs.Paxos.AcausalExploration

/-!
# `lake exe explore` — EXPLORATION E1: the acausal-decision experiment

Runs the acausal fabricated-leadership script (`Acausal.nd`) and its
causal control (`Acausal.ndCausal`) against the **guarded** `paxos_core`
and prints, per proposer: the realized ballot announcements, the realized
commit stream, and per-slot agreement. Also prints diagnostic views (the
p1a streams and leader flags at the final unfolding) so the blocking
behavior of the acausal script is visible.

Same IO-materialization pattern as `Falsify.lean` (FINDINGS D14), but
generalized over the decision bundle and clients.
-/

open HydroLean.Programs.Paxos
open HydroLean.Programs.Paxos.Acausal
open HydroLean.Hydro

def matList {α : Type} (n : Nat) (f : Fin n → α) : IO (Array α) := do
  let mut a : Array α := #[]
  for h : k in [0:n] do
    a := a.push (f ⟨k, h.upper⟩)
  pure a

def matLERef (r : LERef Nat 2 3) : IO (LERef Nat 2 3) := do
  let pf ← matList 2 (fun i => (List.finRange 3).map (fun j => r.p1b_fail i j))
  let ia ← matList 2 (fun i => r.i_am_leader i)
  let pl ← matList 2 (fun i => r.p_is_leader i)
  pure ⟨fun i j => (pf[i.val]!)[j.val]!, fun i => ia[i.val]!,
    fun i => pl[i.val]!⟩

def leaderElectionIO (variant : PaxosVariant)
    (sq : Fin 2 → Fin 3 → Stream (Ballot 2))
    (al : Fin 3 → TSing (Option Nat × LogMap Nat 2))
    (leNd : LENondet Nat 2 3) :
    IO ((Fin 2 → MonoSing (ballotNumVO (nP := 2))) × (Fin 2 → TSing Bool)
      × (Fin 2 → TStream (P1bPayload Nat 2))
      × (Fin 3 → MonoSing (obtVO (nP := 2)))) := do
  let mut r : LERef Nat 2 3 := LERef.init
  for _ in [0:leNd.fuel] do
    r ← matLERef ((leader_election_bodyM variant 2 3 leNd).f (sq, al, r)).1
  let out := ((leader_election_bodyM variant 2 3 leNd).f (sq, al, r)).2
  let pb ← matList 2 (fun i => out.1 i)
  let pl ← matList 2 (fun i => out.2.1 i)
  let pr ← matList 2 (fun i => out.2.2.1 i)
  let am ← matList 3 (fun j => out.2.2.2 j)
  pure (fun i => pb[i.val]!, fun i => pl[i.val]!, fun i => pr[i.val]!,
    fun j => am[j.val]!)

structure StepOut where
  ref : PaxosRef Nat 2 3
  ballots : Fin 2 → Stream (Ballot 2)
  commits : Fin 2 → Stream (Nat × Option Nat)
  /-- Diagnostics: per-proposer (ballot ticks, leader flags, p1a sends). -/
  diagBallots : Fin 2 → List (Ballot 2)
  diagFlags : Fin 2 → List Bool
  diagP1a : Fin 2 → List (Ballot 2)

def paxosStepIO (variant : PaxosVariant) (pn : PaxosNondet Nat 2 3)
    (cl : (Fin 2 → Stream (Ballot 2)) →ₘ (Fin 2 → Stream Nat))
    (r : PaxosRef Nat 2 3) : IO StepOut := do
  let le ← leaderElectionIO variant r.sequencing_max_ballot r.a_log pn.le
  let just := fun i =>
    (TSing.zip (le.2.1 i) (TSing.defer ((le.2.1 i).map (!·)) true)).map
      (fun lw => lw.1 && lw.2)
  let ballots ← matList 2 (fun i =>
    TStream.allTicks (TSing.filterIf (le.1 i).vals (just i)))
  let c := cl.f (fun i => ballots[i.val]!)
  let sp := (sequence_payloadM variant 1 pn.sp).f
    (c, fun i => (le.1 i).vals, le.2.1, le.2.2.1,
     fun j => (le.2.2.2 j).vals)
  let commits ← matList 2 (fun i => sp.1 i)
  let alog ← matList 3 (fun j => (sp.2.1 j).vals)
  let seqm ← matList 2 (fun i => (List.finRange 3).map (fun j => sp.2.2 i j))
  -- diagnostics from the leader-election fixpoint value
  let dgB ← matList 2 (fun i => (le.1 i).vals)
  let dgF ← matList 2 (fun i => le.2.1 i)
  let dgS ← matList 2 (fun i =>
    (le_p_to_acceptors_p1aM (nA := 3) variant pn.le i).f
      (r.sequencing_max_ballot, r.a_log,
       -- re-run the final cycle value for the diagnostic wire
       (LERef.init : LERef Nat 2 3)))
  pure ⟨⟨fun i j => (seqm[i.val]!)[j.val]!, fun j => alog[j.val]!⟩,
    fun i => ballots[i.val]!, fun i => commits[i.val]!,
    fun i => dgB[i.val]!, fun i => dgF[i.val]!, fun i => dgS[i.val]!⟩

def paxosIO (variant : PaxosVariant) (pn : PaxosNondet Nat 2 3)
    (cl : (Fin 2 → Stream (Ballot 2)) →ₘ (Fin 2 → Stream Nat)) :
    IO StepOut := do
  let mut r : PaxosRef Nat 2 3 := PaxosRef.init
  for _ in [0:pn.fuel] do
    r := (← paxosStepIO variant pn cl r).ref
  paxosStepIO variant pn cl r

def agreeReport (name : String) (out : StepOut) : IO Bool := do
  let c0 := out.commits 0
  let c1 := out.commits 1
  IO.println s!"[{name}] P0 ballot ticks: {repr (out.diagBallots 0)}"
  IO.println s!"[{name}] P0 leader flags: {repr (out.diagFlags 0)}"
  IO.println s!"[{name}] P1 ballot ticks: {repr (out.diagBallots 1)}"
  IO.println s!"[{name}] P1 leader flags: {repr (out.diagFlags 1)}"
  IO.println s!"[{name}] P0 commits: {repr c0}"
  IO.println s!"[{name}] P1 commits: {repr c1}"
  let all := c0 ++ c1
  let agree := !(all.any fun (s, v) => all.any fun (s', v') =>
    s = s' && v ≠ v')
  IO.println s!"[{name}] per-slot agreement (within and across): {agree}"
  pure agree

def main : IO Unit := do
  IO.println "== E1: acausal fabricated-leadership attempt (guarded variant) =="
  let acausal ← paxosIO PaxosVariant.guarded Acausal.nd Acausal.clients
  let okA ← agreeReport "acausal" acausal
  IO.println ""
  IO.println "== E1 control: same data, causal delivery (guarded variant) =="
  let causal ← paxosIO PaxosVariant.guarded Acausal.ndCausal Acausal.clients
  let okC ← agreeReport "causal" causal
  IO.println ""
  IO.println s!"acausal script agreement: {okA}"
  IO.println s!"causal control agreement: {okC}"
