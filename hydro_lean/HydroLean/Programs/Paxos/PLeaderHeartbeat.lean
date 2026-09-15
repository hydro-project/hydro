import HydroLean.Programs.Paxos.BallotCalc

/-!
# `p_leader_heartbeat` (paxos.rs:414–482) — module

The heartbeat/election-timer function over the located surface: leaders
gossip their ballot (`p_to_proposers_i_am_leader`, an `AtLeastOnce NoOrder`
broadcast sampled from the leader's latest ballot at `nondet!` times), and
non-leaders whose timeout fired (`nondet!`) with a nonempty delayed interval
batch (`nondet!`) trigger an election.

All three `nondet!` sites here are timing: `sample_every` (paxos.rs:437),
`timeout` (:449) and the delayed-interval batch (:469). Their doc comments
claim leader-election nondeterminism only. The module exports **one safety
clause**: the election-trigger gate (`p_leader_heartbeat_trigger_gate` —
a trigger can only fire at a `p_is_leader = false` tick, the paxos.rs:449
`filter_if`), which stages solicitation across the `forward_ref` cycle and
drives the derivation of `leader_ballot_stable` (FINDINGS D21). The
heartbeats feed only the commutative max; the timing decisions stay
absent from every downstream safety hypothesis.

`p_is_leader` arrives through the tick-level `forward_ref`
(paxos.rs:274–276), i.e. the same-tick flag closed by `p_p1b` — a cycle
input of `leader_election`.
-/

namespace HydroLean.Programs.Paxos

open HydroLean.Hydro

variable {nP : Nat}

/-- The materialized timing `nondet!`s of `p_leader_heartbeat`. -/
structure HeartbeatNondet where
  /-- `sample_every` firing times, as sampled tick indices (paxos.rs:437). -/
  samples : List Nat
  /-- Per-tick `timeout` verdict (paxos.rs:449). -/
  expired : List Bool
  /-- Per-tick delayed-interval batch nonemptiness (paxos.rs:465–478). -/
  delay : List Bool

/-- paxos.rs:414–482 `p_leader_heartbeat`, as the typed dataflow — one `let`
per Rust binding over `(p_is_leader, p_ballot)`; input-growth is carried by
the type. Returns (`p_to_proposers_i_am_leader`, `p_trigger_election`). -/
def p_leader_heartbeatM (nondet : HeartbeatNondet) :
    TSing Bool × TSing (Ballot nP) →ₘ Stream (Ballot nP) × TSing Bool :=
  let p_is_leader := MonoMap.fst
  let p_ballot := MonoMap.snd
  -- p_ballot.filter_if(p_is_leader).latest(), one Option value per tick
  let leader_ballot := (p_ballot.zip p_is_leader).map
    (fun bl => if bl.2 then some bl.1 else none)
  -- .sample_every(…, nondet).broadcast(proposers).values()
  let p_to_proposers_i_am_leader :=
    (leader_ballot.sampleEvery nondet.samples).filterMap _root_.id
  -- p_leader_expired = timeout(…).snapshot(tick).filter_if(!p_is_leader)
  let p_leader_expired :=
    ((MonoMap.const nondet.expired).zip p_is_leader).map
      (fun el => el.1 && !el.2)
  -- p_trigger_election = p_leader_expired.is_some().and(interval batch)
  let p_trigger_election :=
    (p_leader_expired.zip (MonoMap.const nondet.delay)).map
      (fun ed => ed.1 && ed.2)
  MonoMap.pair p_to_proposers_i_am_leader p_trigger_election

/-- The transcription's function face (`.f` of the single-source typed
dataflow `p_leader_heartbeatM`). -/
abbrev p_leader_heartbeat (p_is_leader : TSing Bool)
    (p_ballot : TSing (Ballot nP)) (nondet : HeartbeatNondet) :
    Stream (Ballot nP) × TSing Bool :=
  (p_leader_heartbeatM nondet).f (p_is_leader, p_ballot)

/-- Rust-visible spec face (intentionally not consumed downstream — the
heartbeats feed only the commutative max): heartbeats quote the sender's
own ballots (ownership survives the
gossip; needed only so heartbeat traffic stays inside the commutative-max
argument). -/
theorem p_leader_heartbeat_own (me : Fin nP)
    (v : TSing (Option (Ballot nP))) (nondet : HeartbeatNondet)
    {g : TSing Bool} :
    ∀ b ∈ (p_leader_heartbeat g (p_ballot_calc me v).1.vals nondet).1,
      (b : Ballot nP).proposerId = me := by
  intro b hb
  obtain ⟨ob, hob, hsome⟩ := List.mem_filterMap.mp hb
  have hmem := mem_sampleEvery hob
  obtain ⟨bl, hmem', hval⟩ := List.mem_map.mp hmem
  obtain ⟨b', l⟩ := bl
  have hb' : b' ∈ (p_ballot_calc me v).1.vals :=
    (List.of_mem_zip hmem').1
  cases l with
  | true =>
    have : ob = some b' := hval.symm
    rw [this] at hsome
    cases hsome
    exact p_ballot_calc_own me v _ hb'
  | false =>
    have : ob = none := hval.symm
    rw [this] at hsome
    cases hsome

/-- **The election-trigger gate (module face)**: a trigger can only fire at
a tick whose `p_is_leader` input is `false` (paxos.rs:449 —
`p_leader_expired` is `filter_if(!p_is_leader)`). Because `p_is_leader`
arrives through the `forward_ref` cycle, this is the fact that stages
solicitation across the fixpoint: a standing leader stops soliciting, so a
usurper ballot must have been solicited from a non-leader tick
(`leader_ballot_stable`'s derivation consumes this gate). -/
theorem p_leader_heartbeat_trigger_gate (flag : HydroLean.Hydro.TSing Bool)
    (pb : HydroLean.Hydro.TSing (Ballot nP)) (nondet : HeartbeatNondet)
    {u : Nat}
    (hu : u < ((p_leader_heartbeat flag pb nondet).2).length)
    (ht : ((p_leader_heartbeat flag pb nondet).2)[u]'hu = true) :
    ∃ hf : u < flag.length, flag[u]'hf = false := by
  have hu' : u < (List.map (fun ed : Bool × Bool => ed.1 && ed.2)
      (List.zipWith Prod.mk
        (List.map (fun el : Bool × Bool => el.1 && !el.2)
          (List.zipWith Prod.mk nondet.expired flag))
        nondet.delay)).length := hu
  have ht' : (List.map (fun ed : Bool × Bool => ed.1 && ed.2)
      (List.zipWith Prod.mk
        (List.map (fun el : Bool × Bool => el.1 && !el.2)
          (List.zipWith Prod.mk nondet.expired flag))
        nondet.delay))[u]'hu' = true := ht
  have hzip : u < (List.zipWith Prod.mk
      (List.map (fun el : Bool × Bool => el.1 && !el.2)
        (List.zipWith Prod.mk nondet.expired flag))
      nondet.delay).length := by
    rwa [List.length_map] at hu'
  rw [List.getElem_map] at ht'
  have hexp : u < (List.map (fun el : Bool × Bool => el.1 && !el.2)
      (List.zipWith Prod.mk nondet.expired flag)).length := by
    rw [List.length_zipWith] at hzip
    omega
  have hinner : u < (List.zipWith Prod.mk nondet.expired flag).length := by
    rwa [List.length_map] at hexp
  have hf : u < flag.length := by
    rw [List.length_zipWith] at hinner
    omega
  refine ⟨hf, ?_⟩
  rw [show (List.zipWith Prod.mk
      (List.map (fun el : Bool × Bool => el.1 && !el.2)
        (List.zipWith Prod.mk nondet.expired flag))
      nondet.delay)[u]'hzip
    = ((List.map (fun el : Bool × Bool => el.1 && !el.2)
        (List.zipWith Prod.mk nondet.expired flag))[u]'hexp,
       nondet.delay[u]'(by rw [List.length_zipWith] at hzip; omega))
    from List.getElem_zipWith ..] at ht'
  rw [List.getElem_map] at ht'
  rw [show (List.zipWith Prod.mk nondet.expired flag)[u]'hinner
    = (nondet.expired[u]'(by rw [List.length_zipWith] at hinner; omega),
       flag[u]'hf) from List.getElem_zipWith ..] at ht'
  have hband := (Bool.and_eq_true _ _).mp ht'
  have hband' := (Bool.and_eq_true _ _).mp hband.1
  cases hflag : flag[u]'hf with
  | false => rfl
  | true =>
    rw [hflag] at hband'
    cases hband'.2

end HydroLean.Programs.Paxos
