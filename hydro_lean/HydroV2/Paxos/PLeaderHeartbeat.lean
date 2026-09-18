import HydroV2.MonoRel
import HydroV2.Paxos.Types

/-!
# `p_leader_heartbeat` (paxos.rs:414–482)

Leaders gossip their latest ballot: `.filter_if(p_is_leader).latest()
.sample_every(…, nondet!(reelection))` — **sampling introduces
`AtLeastOnce`** (an unchanged latest sampled twice is a consecutive
stutter), and `.broadcast(…).values()` adds `NoOrder`, giving Rust's
`Stream<Ballot, _, Unbounded, NoOrder, AtLeastOnce>` exactly. The
election timer (`timeout(…).snapshot(…)`) and the delayed interval batch
are **pure timing decisions**: their `nondet!` doc comments claim
leader-election nondeterminism only, and they stay absent from every
downstream safety hypothesis.

Because `p_ballot` itself depends on heartbeat traffic through the
election cycle, sample reads only make sense under the surrounding
`fix`: reads of unrealized ticks block, so Kleene rounds grow the
sampled stream by stutter-prefix (the `MonoRel` field of
`sample_every`).

The module exports two safety clauses: the **election-trigger gate** (a
trigger only fires at a `p_is_leader = false` tick, the paxos.rs:449
`filter_if` — v1 FINDINGS D21), and **heartbeat traceability** (every
gossiped ballot was some member's realized leader ballot — heartbeats
stay inside the commutative-max argument).
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat}

/-- **paxos.rs:414–482 `p_leader_heartbeat`** over the proposer cluster:
returns (`p_to_proposers_i_am_leader`, `p_trigger_election`). The three
`nondet!` sites enter as decision data: sampled tick times, per-tick
timeout verdicts, per-tick interval-batch nonemptiness. -/
structure PLHDec (nP : Nat) where
  /-- `.sample_every(…, nondet!(reelection))`: heartbeat sample
  times. -/
  sample : SampleTimes nP
  /-- `.timeout(…, nondet!(…))`: expiry verdicts (pure timing). -/
  timeout : TimerVerdicts nP
  /-- `source_interval_delayed(…).batch(…)`: election-trigger
  pulses (pure timing). -/
  interval : TimingPulses nP

/-- What `p_leader_heartbeat` **ensures**, over the `Values`
denotation. -/
structure PLHEnsures (ℓ : L)
    (p_is_leader : TickV (mem ℓ) Bool .unbounded)
    (p_ballot : TickV (mem ℓ) (Ballot (mem ℓ)) .unbounded)
    (out : (Fin (mem ℓ) → RetryPool (Ballot (mem ℓ)))
      × TickV (mem ℓ) Bool .unbounded) : Prop where
  /-- **The election-trigger gate**: a trigger only fires at a tick whose
  `p_is_leader` input is `false` (paxos.rs:449). Because `p_is_leader`
  arrives through the `forward_ref` cycle, this stages solicitation
  across the fixpoint: a standing leader stops soliciting. -/
  trigger_gate : ∀ (i : Fin (mem ℓ)) {u : Nat} (hu : u < (out.2 i).length),
    (out.2 i)[u]'hu = true →
    ∃ hf : u < (p_is_leader i).length, (p_is_leader i)[u]'hf = false
  /-- **Heartbeat traceability**: every ballot in the received pool was
  some member's realized ballot at a leading tick — gossip quotes real
  ballots, so heartbeat traffic stays inside the commutative-max
  argument (paxos.rs:436–441; ownership then comes per sender from
  `p_ballot_calc`'s `own`). -/
  heartbeat_src : ∀ (i : Fin (mem ℓ)), ∀ b ∈ out.1 i,
    ∃ j : Fin (mem ℓ), b ∈ p_ballot j

def p_leader_heartbeat (H : HydroSem L mem) (ℓ : L)
    (p_is_leader : H.TickSingleton ℓ Bool .unbounded)
    (p_ballot : H.TickSingleton ℓ (Ballot (mem ℓ)) .unbounded)
    (dec : PLHDec (mem ℓ))
    (chIAL : ChannelId) :
    {out : H.Stream ℓ (Ballot (mem ℓ)) .noOrder .atLeastOnce
        × H.TickSingleton ℓ Bool .unbounded //
      ∀ hv : H = Values L mem,
        match H, hv, p_is_leader, p_ballot, out with
        | _, rfl, fl, pb, o => PLHEnsures ℓ fl pb o} :=
  -- p_ballot.filter_if(p_is_leader): the leader's latest, when leading
  let leader_ballot := H.mapTick (H.zipTick p_ballot p_is_leader)
    (fun _me bl => if bl.2 then some bl.1 else none)
  -- .latest().sample_every(…, nondet!(reelection)): AtLeastOnce born here
  let sampled := H.sample_every leader_ballot dec.sample
  -- .broadcast(proposers, …).values(): NoOrder on top
  let p_to_proposers_i_am_leader := H.values (H.broadcast chIAL sampled)
  -- p_leader_expired = heartbeats.timeout(…).snapshot(tick)
  --   .filter_if(!p_is_leader)
  let p_leader_expired := H.mapTick
    (H.zipTick (H.timeout_snapshot p_to_proposers_i_am_leader dec.timeout)
      p_is_leader)
    (fun _me el => el.1 && !el.2)
  -- p_trigger_election = p_leader_expired.is_some()
  --   .and(source_interval_delayed(…).batch(…).first().is_some())
  let p_trigger_election := H.mapTick
    (H.zipTick p_leader_expired (H.source_interval_batch dec.interval))
    (fun _me ed => ed.1 && ed.2)
  ⟨(p_to_proposers_i_am_leader, p_trigger_election), by
    intro hv
    subst hv
    constructor
    · -- the trigger gate
      intro i u hu ht
      have ht' : ((Trace.zip
          ((Trace.zip (dec.timeout i) (p_is_leader i)).map
            (fun el => el.1 && !el.2)) (dec.interval i)).map
          (fun ed => ed.1 && ed.2))[u]'hu = true := ht
      have hu' : u < ((Trace.zip
          ((Trace.zip (dec.timeout i) (p_is_leader i)).map
            (fun el => el.1 && !el.2)) (dec.interval i)).map
          (fun ed => ed.1 && ed.2)).length := hu
      have hzip : u < (Trace.zip
          ((Trace.zip (dec.timeout i) (p_is_leader i)).map
            (fun el => el.1 && !el.2)) (dec.interval i)).length := by
        rwa [List.length_map] at hu'
      have hf : u < (p_is_leader i).length := by
        simp only [Trace.zip, List.length_zip, List.length_map] at hzip
        omega
      refine ⟨hf, ?_⟩
      simp only [Trace.zip, List.getElem_map, List.getElem_zip,
        Bool.and_eq_true, Bool.not_eq_true'] at ht'
      exact ht'.1.2
    · -- heartbeat traceability
      intro i b hb
      have hb' : b ∈ (((List.finRange (mem ℓ)).map
          (fun j => RetryPool.mk
            (↑(StutterSeq.mk (sampleAtOpt
              ((Trace.zip (p_ballot j) (p_is_leader j)).map
                (fun bl => if bl.2 then some bl.1 else none))
              (dec.sample j))).norm : Multiset _))).foldl
          RetryPool.union (RetryPool.mk 0)) := hb
      rcases RetryPool.mem_foldl_union hb' with h0 | ⟨p, hp, hbp⟩
      · exact absurd ((RetryPool.mem_mk _ _).mp h0) (by simp)
      · obtain ⟨j, -, rfl⟩ := List.mem_map.mp hp
        refine ⟨j, ?_⟩
        have h1 : b ∈ (↑(destutter (sampleAtOpt
            ((Trace.zip (p_ballot j) (p_is_leader j)).map
              (fun bl => if bl.2 then some bl.1 else none))
            (dec.sample j))) : Multiset _) := hbp
        have h2 := Multiset.mem_coe.mp h1
        have h3 := (destutter_sublist _).subset h2
        have h4 := sampleAtOpt_mem h3
        obtain ⟨bl, hbl, hval⟩ := List.mem_map.mp h4
        cases hl2 : bl.2 with
        | false =>
          rw [hl2] at hval
          simp at hval
        | true =>
          rw [hl2] at hval
          rw [if_pos rfl] at hval
          injection hval with hval'
          rw [← hval']
          exact (List.of_mem_zip hbl).1⟩

/-! ## Executable smoke tests (`@Values` evaluates; quotient content is
observed through its normalized views) -/

-- Member 0 leads at tick 1 with ballot `(1,0)` and samples tick 1;
-- member 1 never leads (its sample reads skip empty latests). Every
-- member's received heartbeat support is exactly `{(1,0)}`.
#guard ((p_leader_heartbeat (Values Unit (fun _ => 2)) ()
    (fun i : Fin 2 => if i = 0 then [false, true] else [false, false])
    (fun _ => [Ballot.mk 0 0, Ballot.mk 1 0])
    ⟨fun i : Fin 2 => if i = 0 then [1] else [0, 1],
     fun _ => [true, true], fun _ => [true, true]⟩ 0).val.1 0).support
  = {Ballot.mk 1 0}

-- The trigger fires only at the non-leader tick 0 (gate observed
-- executably): expired ∧ interval ∧ ¬leader.
#guard (p_leader_heartbeat (Values Unit (fun _ => 2)) ()
    (fun i : Fin 2 => if i = 0 then [false, true] else [false, false])
    (fun _ => [Ballot.mk 0 0, Ballot.mk 1 0])
    ⟨fun i : Fin 2 => if i = 0 then [1] else [0, 1],
     fun _ => [true, true], fun _ => [true, true]⟩ 0).val.2 0
  = [true, false]

-- A stuttering sample (same latest read twice) is *the same* entitlement
-- class as sampling once: the `AtLeastOnce` quotient at work.
#guard (StutterSeq.mk [(Ballot.mk 1 0 : Ballot 2), Ballot.mk 1 0]).norm
  = (StutterSeq.mk [(Ballot.mk 1 0 : Ballot 2)]).norm

end HydroV2
