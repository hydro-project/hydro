import HydroLean.Programs.Paxos.Falsification

/-!
# `lake exe falsify` — run the B1/B2 falsification scripts

Executes the FINDINGS.md B1 decision script against the faithful and
guarded `paxos_core` models and prints the committed slots, plus the B2
module-face witness.

**Why an IO harness**: the pure `paxos_core` interpreter is exponentially
slow to *execute* (not to reason about): the Lean compiler does not
reliably share `let`-bound closure-carrying structs, so every read of a
cycle value re-runs prior unfoldings (FINDINGS D14). The harness below
runs the *same* typed bodies (`leader_election_bodyM`, the `paxos_core`
wiring, `sequence_payloadM` — applied at the boundary via `.f`) and forces
each cycle family into
arrays through `IO` between unfoldings — the operational realization of
`memoF` (semantically the identity, `memoF_def`). The wiring below mirrors
`paxos_core_bodyM`/`leader_electionM` line by line.
-/

open HydroLean.Programs.Paxos
open HydroLean.Programs.Paxos.Falsification
open HydroLean.Hydro

/-- Force a member-indexed family into an array (the `IO`-realized
`memoF`). -/
def matList {α : Type} (n : Nat) (f : Fin n → α) : IO (Array α) := do
  let mut a : Array α := #[]
  for h : k in [0:n] do
    a := a.push (f ⟨k, h.upper⟩)
  pure a

/-- Materialize an `LERef` (cycle carrier of `leader_election`). -/
def matLERef (r : LERef Nat 2 3) : IO (LERef Nat 2 3) := do
  let pf ← matList 2 (fun i => (List.finRange 3).map (fun j => r.p1b_fail i j))
  let ia ← matList 2 (fun i => r.i_am_leader i)
  let pl ← matList 2 (fun i => r.p_is_leader i)
  pure ⟨fun i j => (pf[i.val]!)[j.val]!, fun i => ia[i.val]!,
    fun i => pl[i.val]!⟩

/-- `leader_election` (paxos.rs:253–345) with `IO`-materialized cycle
unfoldings; the body is the model's `leader_election_body`. -/
def leaderElectionIO (variant : PaxosVariant)
    (sq : Fin 2 → Fin 3 → Stream (Ballot 2))
    (al : Fin 3 → TSing (Option Nat × LogMap Nat 2))
    (nd : LENondet Nat 2 3) :
    IO ((Fin 2 → MonoSing (ballotNumVO (nP := 2))) × (Fin 2 → TSing Bool)
      × (Fin 2 → TStream (P1bPayload Nat 2))
      × (Fin 3 → MonoSing (obtVO (nP := 2)))) := do
  let mut r : LERef Nat 2 3 := LERef.init
  for _ in [0:nd.fuel] do
    r ← matLERef ((leader_election_bodyM variant 2 3 nd).f (sq, al, r)).1
  let out := ((leader_election_bodyM variant 2 3 nd).f (sq, al, r)).2
  let pb ← matList 2 (fun i => out.1 i)
  let pl ← matList 2 (fun i => out.2.1 i)
  let pr ← matList 2 (fun i => out.2.2.1 i)
  let am ← matList 3 (fun j => out.2.2.2 j)
  pure (fun i => pb[i.val]!, fun i => pl[i.val]!, fun i => pr[i.val]!,
    fun j => am[j.val]!)

/-- One `paxos_core_body` unfolding (paxos.rs:136–246 let-chain, mirrored)
with materialized handoffs. -/
def paxosStepIO (variant : PaxosVariant) (r : PaxosRef Nat 2 3) :
    IO (PaxosRef Nat 2 3
      × (Fin 2 → Stream (Ballot 2)) × (Fin 2 → Stream (Nat × Option Nat))) := do
  -- leader_election(…) (paxos.rs:169–189)
  let le ← leaderElectionIO variant r.sequencing_max_ballot r.a_log leN
  -- just_became_leader (paxos.rs:191–197)
  let just := fun i =>
    (TSing.zip (le.2.1 i) (TSing.defer ((le.2.1 i).map (!·)) true)).map
      (fun lw => lw.1 && lw.2)
  -- p_ballot.filter_if(just_became_leader).all_ticks() (:201–203)
  let ballots ← matList 2 (fun i =>
    TStream.allTicks (TSing.filterIf (le.1 i).vals (just i)))
  let c := clients.f (fun i => ballots[i.val]!)
  -- sequence_payload(…) (paxos.rs:206–226)
  let sp := (sequence_payloadM variant 1 spN).f
    (c, fun i => (le.1 i).vals, le.2.1, le.2.2.1,
     fun j => (le.2.2.2 j).vals)
  let commits ← matList 2 (fun i => sp.1 i)
  let alog ← matList 3 (fun j => (sp.2.1 j).vals)
  let seqm ← matList 2 (fun i => (List.finRange 3).map (fun j => sp.2.2 i j))
  pure (⟨fun i j => (seqm[i.val]!)[j.val]!, fun j => alog[j.val]!⟩,
    fun i => ballots[i.val]!, fun i => commits[i.val]!)

/-- `paxos_core` at the B1 script's decisions, `IO`-materialized. -/
def paxosIO (variant : PaxosVariant) :
    IO ((Fin 2 → Stream (Ballot 2)) × (Fin 2 → Stream (Nat × Option Nat))) := do
  let mut r : PaxosRef Nat 2 3 := PaxosRef.init
  for _ in [0:nd.fuel] do
    r := (← paxosStepIO variant r).1
  let (_, ballots, commits) ← paxosStepIO variant r
  pure (ballots, commits)

def main : IO Unit := do
  IO.println "== B1: duplicate P1a broadcasts (paxos.rs:311–317) =="
  let (_, fCommits) ← paxosIO PaxosVariant.faithful
  let (_, gCommits) ← paxosIO PaxosVariant.guarded
  let f0 := fCommits 0
  let f1 := fCommits 1
  let g0 := gCommits 0
  let g1 := gCommits 1
  IO.println s!"faithful: proposer 0 commits {repr f0}"
  IO.println s!"faithful: proposer 1 commits {repr f1}"
  IO.println s!"guarded:  proposer 0 commits {repr g0}"
  IO.println s!"guarded:  proposer 1 commits {repr g1}"
  let disagree := f0.any fun (s, v) => f1.any fun (s', v') => s = s' && v ≠ v'
  let gAgree := !(g0.any fun (s, v) => g1.any fun (s', v') => s = s' && v ≠ v')
  IO.println s!"faithful DISAGREES at a slot: {disagree} (expected: true — the B1 bug)"
  IO.println s!"guarded agrees: {gAgree} (expected: true — the proven theorem)"
  IO.println ""
  IO.println "== B2: every-tick recommit (paxos.rs:596–672 + 706–734) =="
  IO.println s!"faithful send-side emissions: {repr b2Faithful}"
  IO.println s!"guarded  send-side emissions: {repr b2Guarded}"
  let b2fNodup := decide ((b2Faithful.map (·.1)).Nodup)
  let b2gNodup := decide ((b2Guarded.map (·.1)).Nodup)
  IO.println s!"faithful keys duplicate-free: {b2fNodup} (expected: false — the B2 bug)"
  IO.println s!"guarded  keys duplicate-free: {b2gNodup} (expected: true)"
  if disagree ∧ gAgree ∧ !b2fNodup ∧ b2gNodup then
    IO.println "\nAll falsification checks PASSED."
  else
    IO.println "\nFALSIFICATION CHECKS FAILED."
    (throw (IO.userError "unexpected outputs") : IO Unit)
