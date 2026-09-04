import Hydro.MonoRel
import Hydro.Paxos.Types
import Hydro.Std.Timers
import Hydro.HydroDef

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
downstream safety hypothesis. `sample_every` and `timeout` are the
library functions of `Std/Timers.lean` (D89), programs over the two
timing primitives `time` and `source_interval` (D91).

Because `p_ballot` itself depends on heartbeat traffic through the
election cycle, sample reads only make sense under the surrounding
`fix`: reads of unrealized ticks block, so Kleene rounds grow the
sampled stream by stutter-prefix.

The module exports two safety clauses: the **election-trigger gate** (a
trigger only fires at a `p_is_leader = false` tick, the paxos.rs:449
`filter_if` — FINDINGS D21), and **heartbeat traceability** (every
gossiped ballot was some member's realized leader ballot — heartbeats
stay inside the commutative-max argument).
-/

namespace Hydro

variable {L : Type} {mem : L → Nat}

/-- **paxos.rs:414–482 `p_leader_heartbeat`**'s `nondet!` sites as
decision data: the sample interval's pulses, `timeout`'s batching and
clock readings, the delayed interval's pulses (all timing nondets — real
in every interpretation). -/
structure PLHDec (H : HydroSem L mem) (nP : Nat) where
  /-- `.sample_every(…, nondet!(reelection))`: heartbeat sample
  times. -/
  sample : SampleEveryDec H nP
  /-- `.timeout(…, nondet!(…))`: the arrivals' batching and the clock
  readings (pure timing). -/
  timeout : TimeoutDec H nP (Ballot nP)
  /-- `.snapshot(proposer_tick, nondet!(/** absorbed into timeout */))`: the cut at
  which the proposer tick reads the timeout's latest verdict. -/
  timeoutSnap : H.SnapDec nP (Option Unit) .totalOrder
  /-- `source_interval_delayed(…).batch(proposer_tick, nondet!(…))` (paxos.rs:470–477):
  the batches of the election-trigger timer's `()`s. -/
  interval : H.BatchDec nP Unit .totalOrder .atLeastOnce

/-- **`p_leader_heartbeat`'s adversary-side (sched-det) bundle**
(`Unit` at `Values`; see `Sem.lean`'s classification table). -/
structure PLHSched (H : HydroSem L mem) (nP : Nat) where
  /-- `i_am_leader` broadcast_closed transport (proposers → proposers):
  delivery cursors. (The Rust site's `nondet!` is `nondet_membership`
  — unmodeled under closed membership; delivery itself is unmarked.) -/
  ial : H.TransportDec nP nP
  /-- `sample_every`'s interval timer (`i_am_leader_send_timeout`). -/
  sample : SampleEverySched H nP
  /-- `source_interval_delayed(…)` (paxos.rs:462–469): the election-trigger timer's
  fire timing. -/
  interval : H.IntervalDec nP

/-- The trivial bundle at the denotation. -/
def PLHSched.triv {nP : Nat} : PLHSched (Values L mem) nP := ⟨(), ⟨()⟩, ()⟩

/-- What `p_leader_heartbeat` **ensures**, over the `Values`
denotation. -/
structure PLHEnsures (ℓ : L)
    (p_is_leader : TickV (mem ℓ) Bool)
    (p_ballot : TickV (mem ℓ) (Ballot (mem ℓ)))
    (out : (MemberMap (mem ℓ) (RetryPool (Ballot (mem ℓ))))
      × TickV (mem ℓ) Bool) : Prop where
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

hydro def p_leader_heartbeat (H : HydroSem L mem) (ℓ : L)
    (p_is_leader : H.Atomic ℓ Bool)
    (p_ballot : H.Atomic ℓ (Ballot (mem ℓ)))
    (paxos_config : PaxosConfig)
    (dec : PLHDec H (mem ℓ)) (sched : PLHSched H (mem ℓ)) :
    (H.Stream ℓ (Ballot (mem ℓ)) .noOrder .atLeastOnce
      × H.Atomic ℓ Bool)
  ensures out => PLHEnsures ℓ p_is_leader p_ballot out :=
  -- let i_am_leader_send_timeout = paxos_config.i_am_leader_send_timeout;
  -- let i_am_leader_check_timeout = paxos_config.i_am_leader_check_timeout;
  -- let i_am_leader_check_timeout_delay_multiplier = paxos_config.i_am_leader_check_timeout_delay_multiplier;
  -- p_ballot.filter_if(p_is_leader): the leader's latest, when leading
  let leader_ballot := H.mapTick (H.zipTick p_ballot p_is_leader)
    (fun _me bl => if bl.2 then some bl.1 else none)
  -- .latest().sample_every(Duration::from_secs(i_am_leader_send_timeout), nondet!(reelection)):
  -- AtLeastOnce born here
  let sampled := sample_every H ℓ leader_ballot (Duration.from_secs paxos_config.i_am_leader_send_timeout) dec.sample sched.sample
  ghost have hsampled := sample_every.ensures ℓ leader_ballot (Duration.from_secs paxos_config.i_am_leader_send_timeout) dec.sample sched.sample
  -- .broadcast(proposers, …).values(): NoOrder on top
  let p_to_proposers_i_am_leader :=
    H.values (H.broadcast_closed sched.ial sampled)
  -- p_leader_expired = p_to_proposers_i_am_leader.clone()
  --   .timeout(Duration::from_secs(i_am_leader_check_timeout), nondet!(…))
  let expired_latest := timeout H ℓ p_to_proposers_i_am_leader
    (Duration.from_secs paxos_config.i_am_leader_check_timeout) dec.timeout
  --   .snapshot(proposer_tick, nondet!(/** absorbed into timeout */))
  let expired := H.snapshot expired_latest dec.timeoutSnap
  --   .filter_if(!p_is_leader)
  let p_leader_expired := H.mapTick (H.zipTick expired p_is_leader)
    (fun _me el => if el.2 then none else el.1)
  -- // Add random delay depending on node ID so not everyone sends p1a at the same time
  -- p_trigger_election = p_leader_expired.is_some().and(proposers
  --   .source_interval_delayed(q!(Duration::from_secs((CLUSTER_SELF_ID.get_raw_id()
  --     * i_am_leader_check_timeout_delay_multiplier as u32).into())),
  --     q!(Duration::from_secs(i_am_leader_check_timeout)))   (paxos.rs:462–469)
  let election_interval := H.source_interval
    (fun me => Duration.from_secs (me.val * paxos_config.i_am_leader_check_timeout_delay_multiplier))
    (Duration.from_secs paxos_config.i_am_leader_check_timeout) sched.interval
  --   .batch(proposer_tick, nondet!(/** If the leader 'un-expires' … */))   (:470–477)
  --   .first()   (:478)
  let election_fire := H.firstTick (H.batch election_interval dec.interval)
  --   .is_some())  — and `Optional::is_some().and(…)` (:460, :479)
  let p_trigger_election := H.mapTick
    (H.zipTick p_leader_expired election_fire)
    (fun _me ed => ed.1.isSome && ed.2.isSome)
  (p_to_proposers_i_am_leader, p_trigger_election)
  prove
    trigger_gate := fun i u hu ht => by
      -- re-anchor the read at the denotational spelling, once
      have hu' : u < ((Trace.zip
          ((Trace.zip (expired i) (p_is_leader i)).map
            (fun el : Option Unit × Bool => if el.2 then none else el.1)) (election_fire i)).map
          (fun ed : Option Unit × Option Unit => ed.1.isSome && ed.2.isSome)).length := hu
      have ht' : ((Trace.zip
          ((Trace.zip (expired i) (p_is_leader i)).map
            (fun el : Option Unit × Bool => if el.2 then none else el.1)) (election_fire i)).map
          (fun ed : Option Unit × Option Unit => ed.1.isSome && ed.2.isSome))[u]'hu' = true := ht
      -- two nested tick reads: (expired ∧ interval) over (timeout ∧ ¬flag)
      obtain ⟨he, hd, hread⟩ := Trace.zip_map_getElem hu'
      rw [hread] at ht'
      obtain ⟨hto, hf, hread'⟩ := Trace.zip_map_getElem he
      rw [hread'] at ht'
      refine ⟨hf, ?_⟩
      by_contra hl
      rw [Bool.not_eq_false] at hl
      simp [hl] at ht',
    heartbeat_src := fun i b hb => by
      obtain ⟨j, hbp⟩ := (ValuesTick.values_broadcast_mem_stutter _ _ _ _).mp hb
      obtain ⟨u, hu⟩ := hsampled.sampled_src j b hbp
      -- the filter_if shape pins bl.2 = true and bl.1 = b
      have hu' : (((Trace.zip (p_ballot j) (p_is_leader j)).map
          (fun bl : Ballot (mem ℓ) × Bool => if bl.2 then some bl.1 else none)))[u]? = some (some b) := hu
      have hmem := List.mem_of_getElem? hu'
      obtain ⟨u', hpb, hfl, hval⟩ := Trace.mem_zip_map hmem
      refine ⟨j, ?_⟩
      by_cases hl2 : (p_is_leader j)[u']'hfl = true
      · rw [hl2, if_pos rfl] at hval
        injection hval with hval'
        rw [hval']
        exact List.getElem_mem _
      · rw [Bool.not_eq_true] at hl2
        rw [hl2] at hval
        simp at hval

/-! ## Executable smoke tests (`@Values` evaluates; quotient content is
observed through its normalized views) -/

-- Member 0 leads at tick 1 with ballot `(1,0)` and samples tick 1;
-- member 1 never leads. Every member's received heartbeat support is
-- exactly `{(1,0)}`.
#guard_eval ((p_leader_heartbeat (Values Unit (fun _ => 2)) ()
    (.mk' fun i : Fin 2 => if i = 0 then [false, true] else [false, false])
    (.mk' fun _ => [Ballot.mk 0 0, Ballot.mk 1 0])
    ⟨1, 5, 10, 15⟩
    ⟨⟨fun i : Fin 2 => if i = 0 then [[], [()]] else [[()], [()]]⟩,
     ⟨fun _ => [{}, {Ballot.mk 1 0}], fun _ => [0, 1]⟩,
     fun _ => [1, 1], fun _ => [[()], [()]]⟩ .triv).1 0).support
  = {Ballot.mk 1 0}

-- The trigger fires only at the non-leader tick 0 (gate observed
-- executably): expired ∧ interval ∧ ¬leader — at tick 0 nothing has
-- arrived (expired), at tick 1 the heartbeat arrives (fresh); the proposer
-- tick reads the timeout's latest verdict synchronously (one entry per tick).
#guard_eval (p_leader_heartbeat (Values Unit (fun _ => 2)) ()
    (.mk' fun i : Fin 2 => if i = 0 then [false, true] else [false, false])
    (.mk' fun _ => [Ballot.mk 0 0, Ballot.mk 1 0])
    ⟨1, 5, 10, 15⟩
    ⟨⟨fun i : Fin 2 => if i = 0 then [[], [()]] else [[()], [()]]⟩,
     ⟨fun _ => [{}, {Ballot.mk 1 0}], fun _ => [0, 1]⟩,
     fun _ => [1, 1], fun _ => [[()], [()]]⟩ .triv).2 0
  = [true, false]

-- A stuttering sample (same latest read twice) is *the same* entitlement
-- class as sampling once: the `AtLeastOnce` quotient at work.
#guard_eval (StutterSeq.mk [(Ballot.mk 1 0 : Ballot 2), Ballot.mk 1 0]).norm
  = (StutterSeq.mk [(Ballot.mk 1 0 : Ballot 2)]).norm

#nondet_census p_leader_heartbeat (nondets := 5) (scheds := 3) (fuels := 0)

end Hydro
