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


/-- What `p_leader_heartbeat` **ensures**: the output pair (gossip
stream, election trigger) against the inputs `(p_is_leader, p_ballot)`. -/
structure PLHEnsures (p_is_leader : TSing Bool)
    (p_ballot : TSing (Ballot nP))
    (out : Stream (Ballot nP) × TSing Bool) : Prop where
  /-- **The election-trigger gate**: a trigger can only fire at a tick
  whose `p_is_leader` input is `false` (paxos.rs:449 — `p_leader_expired`
  is `filter_if(!p_is_leader)`). Because `p_is_leader` arrives through the
  `forward_ref` cycle, this stages solicitation across the fixpoint: a
  standing leader stops soliciting (FINDINGS D21). -/
  trigger_gate : ∀ {u : Nat} (hu : u < out.2.length),
    out.2[u]'hu = true →
    ∃ hf : u < p_is_leader.length, p_is_leader[u]'hf = false
  /-- Heartbeats quote the sender's own ballots (given the own-ballot
  input requirement) — ownership survives the gossip, so heartbeat
  traffic stays inside the commutative-max argument (Rust-visible face,
  paxos.rs:436–441). -/
  i_am_leader_own : ∀ {me : Fin nP},
    (∀ b ∈ p_ballot, (b : Ballot nP).proposerId = me) →
    ∀ b ∈ out.1, (b : Ballot nP).proposerId = me

/-- **paxos.rs:414–482 `p_leader_heartbeat`** — the single verified
artifact: the body (one `let` per Rust binding over
`(p_is_leader, p_ballot)`), prefix-monotone by type, guarantees on the
signature. Returns (`p_to_proposers_i_am_leader`, `p_trigger_election`). -/
def p_leader_heartbeat (nondet : HeartbeatNondet) :
    Verified (TSing Bool × TSing (Ballot nP))
      (Stream (Ballot nP) × TSing Bool)
      (fun x out => PLHEnsures x.1 x.2 out) :=
  Verified.ofMono
    (let p_is_leader := MonoMap.fst
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
     MonoMap.pair p_to_proposers_i_am_leader p_trigger_election)
    (fun x =>
      { trigger_gate := fun {u} hu ht => by
            obtain ⟨flag, pb⟩ := x
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
        i_am_leader_own := fun {me} hown b hb => by
          obtain ⟨ob, hob, hsome⟩ := List.mem_filterMap.mp hb
          have hmem := mem_sampleEvery hob
          obtain ⟨bl, hmem', hval⟩ := List.mem_map.mp hmem
          obtain ⟨b', l⟩ := bl
          have hb' : b' ∈ x.2 := (List.of_mem_zip hmem').1
          cases l with
          | true =>
            have : ob = some b' := hval.symm
            rw [this] at hsome
            cases hsome
            exact hown _ hb'
          | false =>
            have : ob = none := hval.symm
            rw [this] at hsome
            cases hsome })

end HydroLean.Programs.Paxos
