import Examples.Tutorials.QuorumBase
import Hydro.HydroTick
import Hydro.CoupleProj
import Hydro.Project.Model.WiredRun
import Hydro.Project.Model.DistributedModel
import Hydro.Project.Model.WireTrace
import Hydro.Project.Model.EnvOfInc
import Hydro.HydroStage

/-!
# Quickstart: a voting quorum

A coordinator broadcasts requests to a cluster of participants; every participant votes on
every request and sends its vote back; the coordinator collects the votes in a `tick` block and
decides a request once it has `minYes` yes votes. The contract `QuorumEnsures` (proved at
`Values`) says the coordinator decides only proposed requests with at least `minYes` yes
voters, and decides each at most once. `vote_quorum_safe_wired` carries the quorum field to the processes
that run the program and exchange bytes, for any trace at the wire; `lake exe quorum`
(`Examples/Demo/QuorumDemo.lean`) runs those processes over local TCP.

This file holds the finished program. The vocabulary, the records and the contract are in
`Examples/Tutorials/QuorumBase.lean`; the stages the quickstart passes through on the way here
(the program alone, the contract with its proofs still open, …) are under `Examples/TutorialSteps/`.
-/


namespace Hydro

namespace Tutorials

variable {L : Type} {mem : L → Nat}

/-! ## The program -/

--[signature]--
hydro def vote_quorum (H : HydroSem L mem) (coord part : L) (minYes : Nat)
    (vote : Fin (mem part) → Nat → Bool)
    (requests : H.Stream coord Nat .noOrder .exactlyOnce)
    (dec : QuorumDec H (mem coord)) (sched : QuorumSched H (mem coord) (mem part)) :
    H.Stream coord Nat .noOrder .exactlyOnce
  ensures out => QuorumEnsures minYes vote requests out :=
  --[/signature]--
  --[network]--
  -- the coordinator broadcasts every request to every participant
  let p_requests := H.values (H.broadcast_closed sched.toParticipants requests)
  -- each participant votes on each request it receives
  let p_votes := H.map p_requests (fun j p => (p, vote j p))
  -- the votes travel back to the coordinator
  let c_votes := H.values (H.broadcast_closed sched.toCoordinator p_votes)
  --[/network]--
  --[tick]--
  --[tick-header]--
  tick (state seen : H.BoundedStream (Nat × Bool) .noOrder .exactlyOnce)
      (state decided : H.BoundedStream Nat .noOrder .exactlyOnce)
      (input incoming := H.batch c_votes dec.votes)
      (invariant ((decisions : List (Multiset Nat)) (seen : Multiset (Nat × Bool))
          (decided : Multiset Nat) (incoming : Trace (Multiset (Nat × Bool)))) =>
        seen = (incoming.take decisions.length).sum ∧ decided = decisions.sum) :=
    --[/tick-header]--
    --[tick-body]--
    let all := H.bchain seen incoming
    let yes := H.bkfold yesStep 0 (fun _ _ => True) (fun s x y _ => yesStep_comm s x y)
      (H.binto_keyed all)
    let reached := H.bksKeys (H.bksFilter yes (fun n => decide (minYes ≤ n)))
    let fresh := H.bfilterNotIn reached decided
    rebind (seen := all, decided := H.bchain decided fresh)
    yield (decisions := fresh)
    --[/tick-body]--
    --[tick-prove]--
    -- before the first tick: both states are empty, and nothing was consumed or emitted
    prove init := fun _i => by
        simp [ValuesTick.seed_pair, ValuesTick.seed_stream, den],
      -- one tick: from the invariant before it (`ih`) to the invariant after it
      tick := fun i n out st b_t hb hlen ih => by
        -- "the first `n + 1` batches" is "the first `n` batches, then `b_t`"
        simp only [List.append_eq, List.length_append, List.length_singleton, hlen,
          List.take_add_one, hb, Option.toList_some, List.sum_append, List.sum_singleton] at ih ⊢
        -- evaluate the tick at `Values`: `bchain` is `+`
        simp only [decisions_step, den]
        obtain ⟨hseen, hdecided⟩ := ih
        exact ⟨by rw [hseen], by rw [hdecided]⟩;
    --[/tick-prove]--
  --[/tick]--
  --[ghost-tick]--
  -- the register before every tick, opaque, with the invariant at each tick
  ghost obtain ⟨reg, -, hreg_at, -, -, hreg_inv⟩ := hdecisions_reg
  -- one tick, decoded: tick `n`'s decisions are distinct, each has `minYes` yes votes among
  -- the votes consumed through tick `n`, and none was decided before
  ghost have htick : ∀ (c : Fin (mem coord)) (n : Nat) (e : Multiset Nat),
      (decisions c)[n]? = some e →
      ∃ b_t, (incoming c)[n]? = some b_t ∧ e.Nodup ∧
        ∀ p ∈ e, minYes ≤ yesVotes (((incoming c).take n).sum + b_t) p
          ∧ p ∉ ((decisions c).take n).sum := fun c n e he => by
    have hn := Trace.read_lt he
    -- tick `n`'s decisions are the step on `reg c n` and this tick's batch `b_t`
    obtain ⟨b_t, hb, rfl⟩ := (hreg_at c n e).mp he
    -- the invariant before tick `n`: `reg c n` is the votes consumed and the requests decided
    obtain ⟨hseen, hdecided⟩ := hreg_inv c n
    rw [List.length_take_of_le (Nat.le_of_lt hn)] at hseen
    refine ⟨b_t, hb, ?_⟩
    -- evaluate the step at `Values`, then rewrite the state with what the invariant says it holds
    simp only [decisions_step, den]
    rw [← hseen, ← hdecided]
    -- the reached requests are a counter's keys, so distinct; the filter keeps the undecided
    refine ⟨(reached_nodup minYes _).filter _, fun p hp => ?_⟩
    obtain ⟨hr, hnot⟩ := Multiset.mem_filter.mp hp
    exact ⟨le_yesVotes_of_mem_reached hr, fun hm => by
      revert hnot
      simp only [Bool.not_eq_true']
      exact fun h => (decide_eq_false_iff_not.mp h) hm⟩
  --[/ghost-tick]--
  --[ghost-votes]--
  -- the votes at the coordinator, read at `Values`: every participant's vote on every
  -- proposed request
  ghost have hvotes : ∀ c, c_votes c = ((List.finRange (mem part)).map (fun j =>
      (proposed requests).map (fun q => (q, vote j q)))).sum := fun _ => rfl
  -- the yes votes for `p`: one per yes voter per copy of `p` proposed
  ghost have hyes : ∀ c p, yesVotes (c_votes c) p = yesVoters vote p * (proposed requests).count p :=
    fun c p => by
    rw [hvotes, yesVotes_listSum, List.map_map]
    have : (List.finRange (mem part)).map
        ((fun m => yesVotes m p) ∘ fun j => (proposed requests).map (fun q => (q, vote j q)))
        = (List.finRange (mem part)).map
            (fun j => if vote j p then (proposed requests).count p else 0) :=
      List.map_congr_left (fun j _ => yesVotes_map_tag _ _ p)
    rw [this, sum_map_ite_const]
    rfl
  -- a decision has `minYes` yes votes among all the votes the coordinator receives
  ghost have hdecided : ∀ c, ∀ p ∈ (decisions c).sum, minYes ≤ yesVotes (c_votes c) p :=
    fun c p hp => by
    obtain ⟨e, he, hpe⟩ := mem_list_sum.mp hp
    obtain ⟨n, hn⟩ := List.mem_iff_getElem?.mp he
    obtain ⟨b_t, hb, -, hall⟩ := htick c n e hn
    have hle : ((incoming c).take n).sum + b_t ≤ c_votes c := by
      have : ((incoming c).take n).sum + b_t = ((incoming c).take (n + 1)).sum := by
        rw [List.take_add_one, hb, List.sum_append]
        simp only [Option.toList_some, List.sum_singleton]
      rw [this]
      exact le_trans (sublist_sum_le (List.take_sublist _ _)) (cqConsumed_le _ _)
    exact le_trans (hall p hpe).1 (yesVotes_mono hle p)
  --[/ghost-votes]--
  --[result]--
  H.allTicks decisions
  --[/result]--
  --[prove]--
  prove
    decided_proposed := fun c p hp h1 => by
      -- `minYes ≤ yesVoters * count`, with `1 ≤ minYes`, forces `count ≥ 1`
      have h := le_trans h1 (hdecided c p hp)
      rw [hyes] at h
      rcases Nat.eq_zero_or_pos ((proposed requests).count p) with h0 | hpos
      · rw [h0, Nat.mul_zero] at h; omega
      · exact Multiset.count_pos.mp hpos,
    quorum := fun c p hp hone => by
      -- `minYes ≤ yesVoters * count ≤ yesVoters` when `count ≤ 1`
      have h := hdecided c p hp
      rw [hyes] at h
      exact le_trans h (le_trans (Nat.mul_le_mul_left _ hone) (Nat.mul_one _).le),
    --[once]--
    once := fun c p => by
      show ((decisions c).sum).count p ≤ 1
      -- the decisions of the first `n` ticks hold `p` at most once
      have key : ∀ n, (((decisions c).take n).sum).count p ≤ 1 := by
        intro n
        induction n with
        | zero => simp
        | succ n ih =>
          rw [List.take_add_one]
          cases he : (decisions c)[n]? with
          -- no tick `n`: nothing is added
          | none => simpa using ih
          | some e =>
            -- tick `n` decides `e`: distinct (`hnd`), none decided before (`(hall p _).2`)
            obtain ⟨_b_t, _hb, hnd, hall⟩ := htick c n e he
            rw [Option.toList_some, List.sum_append, List.sum_singleton, Multiset.count_add]
            by_cases hpe : p ∈ e
            -- `p` decided now: 0 times before, once in `e`
            · rw [Multiset.count_eq_zero.mpr (hall p hpe).2, Multiset.count_eq_one_of_mem hnd hpe]
            -- `p` not decided now: the count before is at most 1
            · rw [Multiset.count_eq_zero.mpr hpe]
              exact ih
      simpa using key (decisions c).length
    --[/once]--
  --[/prove]--

--[census]--
#nondet_census vote_quorum (nondets := 1) (scheds := 2) (fuels := 0)
--[/census]--

/-! ## Running it at `Values` -/

--[locs]--
/-- One coordinator, three participants. -/
inductive QLoc | coord | part
  deriving DecidableEq

@[reducible] def qMem : QLoc → Nat
  | .coord => 1
  | .part => 3

/-- Participant 0 votes no on `20`; participant 1 votes no on `20` and `30`. -/
def qVote : Fin 3 → Nat → Bool := fun j p =>
  !((j.val = 0 && p = 20) || (j.val = 1 && (p = 20 || p = 30)))
--[/locs]--

--[run-values]--
-- three ticks of votes: `10` reaches two yes votes in the first, `30` in the third
#guard vote_quorum (Values QLoc qMem) .coord .part 2 qVote
    (.mk' fun _ => ({10, 20, 30} : Multiset Nat))
    ⟨fun _ => [{(10, true), (10, true)}, {(10, true), (30, true)},
               {(30, false), (30, true), (20, false), (20, false), (20, true)}]⟩
    ⟨(), ()⟩ 0
  = ({10, 30} : Multiset Nat)
-- all the votes in one tick: the same decisions
#guard vote_quorum (Values QLoc qMem) .coord .part 2 qVote
    (.mk' fun _ => ({10, 20, 30} : Multiset Nat))
    ⟨fun _ => [{(10, true), (10, true), (10, true), (20, false), (20, false), (20, true),
                (30, true), (30, false), (30, true)}]⟩
    ⟨(), ()⟩ 0
  = ({10, 30} : Multiset Nat)
-- a batching that stops early: only what was consumed is decided
#guard vote_quorum (Values QLoc qMem) .coord .part 2 qVote
    (.mk' fun _ => ({10, 20, 30} : Multiset Nat))
    ⟨fun _ => [{(10, true), (10, true)}]⟩
    ⟨(), ()⟩ 0
  = ({10} : Multiset Nat)
--[/run-values]--

/-! ## The step-machine lemma the wired theorem rests on -/

section Machine

variable {pacing : Pacing L mem} (coord part : L) (minYes : Nat) (vote : Fin (mem part) → Nat → Bool)

--[safe-pool]--
/-- On the step machine, under a `Values` pool `v` above every view of the request history `h`:
a request the coordinator has decided by step `T`, proposed at most once in `v`, was voted yes
by `minYes` participants. Any pacing, any delivery cursors, any horizon. -/
theorem vote_quorum_safe_sched'
    (h : (SchedSem L mem pacing).Stream coord Nat .noOrder .exactlyOnce)
    (v : (Values L mem).Stream coord Nat .noOrder .exactlyOnce)
    (hc : ∀ t i, ListLe .noOrder .exactlyOnce ((h i).view t) (v i))
    (sched : QuorumSched (SchedSem L mem pacing) (mem coord) (mem part))
    (T : Nat) (c : Fin (mem coord)) (p : Nat)
    (hp : p ∈ ((vote_quorum (SchedSem L mem pacing) coord part minYes vote h ⟨()⟩ sched) c).view T)
    (hone : (proposed v).count p ≤ 1) :
    minYes ≤ yesVoters vote p := by
  -- the machine's schedule, repackaged for the coupling; the batch decision is derived there
  let cdec : QuorumDec (CoupleSem L mem pacing T T (Nat.le_refl T)) (mem coord) :=
    ⟨CoDec.batch _⟩
  let csched : QuorumSched (CoupleSem L mem pacing T T (Nat.le_refl T)) (mem coord) (mem part) :=
    ⟨CoDec.cursors sched.toParticipants, CoDec.cursors sched.toCoordinator⟩
  -- 1. run the program at the coupling; open the output corner's `cpl`
  have hcpl := (vote_quorum (CoupleSem L mem pacing T T (Nat.le_refl T)) coord part minYes vote
      (CoStream.inputC h v hc) cdec csched).cpl
    (vote_quorum_co_wf₁ coord part minYes vote (CoStream.inputC h v hc) cdec csched trivial)
    T (by co_lag [vote_quorum_co_lag₁]) c
  -- 2. and 3. the machine run `sr` equals the `SchedSem` run; the denotational run `rr` equals the `Values` run at
  -- the derived decision `vote_quorum_vdec`
  rw [vote_quorum_co_sr₁, vote_quorum_co_rr₁] at hcpl
  -- 4. the contract at the derived decision
  have hens := vote_quorum.ensures coord part minYes vote v
    (vote_quorum_vdec (pacing := pacing) (Td := T) coord part minYes vote h
      sched.toParticipants sched.toCoordinator) ⟨(), ()⟩
  exact hens.quorum c p (Multiset.mem_of_le hcpl (Multiset.mem_coe.mpr hp)) hone
--[/safe-pool]--

open Project in
--[safe]--
/-- On the step machine, for any request history: a request the coordinator has decided by
step `T`, proposed at most once among the requests received by `T`, was voted yes by `minYes`
participants. -/
theorem vote_quorum_safe_sched
    (h : (SchedSem L mem pacing).Stream coord Nat .noOrder .exactlyOnce)
    (sched : QuorumSched (SchedSem L mem pacing) (mem coord) (mem part))
    (T : Nat) (c : Fin (mem coord)) (p : Nat)
    (hp : p ∈ ((vote_quorum (SchedSem L mem pacing) coord part minYes vote h ⟨()⟩ sched) c).view T)
    (hone : (proposed fun i => (↑((h i).view T) : Multiset Nat)).count p ≤ 1) :
    minYes ≤ yesVoters vote p := by
  -- the input frozen at the horizon
  let h' : (SchedSem L mem pacing).Stream coord Nat .noOrder .exactlyOnce :=
    fun i => (h i).truncate T
  let env : HList (Kind.sched (mem := mem)) [Kind.stream coord Nat inferInstance .noOrder .exactlyOnce] :=
    HList.cons (f := Kind.sched (mem := mem)) h HList.nil
  let env' : HList (Kind.sched (mem := mem)) [Kind.stream coord Nat inferInstance .noOrder .exactlyOnce] :=
    HList.cons (f := Kind.sched (mem := mem)) h' HList.nil
  have hagree : HAgree T env env' := by
    refine ⟨fun i t ht => ?_, trivial⟩
    show (h i).view t = (h i).view (min t T)
    rw [Nat.min_eq_left ht]
  -- causality through the graph: the run and the truncated run agree up to `T`
  have hcaus := Dag.denote_causal (pacing := pacing) (vote_quorum.dag coord part minYes vote)
    (vote_quorum.schedOf pacing coord part minYes vote ⟨()⟩ sched) T env env' hagree
  rw [vote_quorum.dag_denote, vote_quorum.dag_denote] at hcaus
  have hout : ((vote_quorum (SchedSem L mem pacing) coord part minYes vote h ⟨()⟩ sched) c).view T
      = ((vote_quorum (SchedSem L mem pacing) coord part minYes vote h' ⟨()⟩ sched) c).view T :=
    hcaus.1 c T (Nat.le_refl T)
  rw [hout] at hp
  -- the truncated input's view at `T` is a pool above all its views
  exact vote_quorum_safe_sched' coord part minYes vote h' (.mk' fun i => ↑((h i).view T))
    (fun t i => Multiset.coe_le.mpr ((h i).mono_le (Nat.min_le_right t T)).sublist.subperm)
    sched T c p hp hone
--[/safe]--

end Machine

/-! ## The processes -/

section Wired

open Project

variable [DecidableEq L]

--[run]--
/-- The activation each process runs: `vote_quorum.dag` staged into straight-line code for
member `m`, with the codecs the elaborator built from the message types' `Wire` instances. -/
hydro_run quorumRun {L : Type} {mem : L → Nat} [DecidableEq L] (coord part : L) (minYes : Nat)
    (vote : Fin (mem part) → Nat → Bool) (m : Member L mem) where
  member := m,
  graph := vote_quorum.dag (mem := mem) coord part minYes vote,
  codecs := vote_quorum.wireCodecs (mem := mem) coord part minYes vote

/-- The body a process runs at an activation: the staged `quorumRun`, ticking. -/
def quorumBody (coord part : L) (minYes : Nat) (vote : Fin (mem part) → Nat → Bool) :
    WireTrace.Body (vote_quorum.dag (mem := mem) coord part minYes vote) :=
  fun m r chans Δ s => quorumRun coord part minYes vote m true r chans Δ s

/-- The staged body equals the interpreter's body `WireTrace.Body.ofCodecs`. -/
theorem quorumBody_eq (coord part : L) (minYes : Nat) (vote : Fin (mem part) → Nat → Bool) :
    quorumBody (mem := mem) coord part minYes vote
      = WireTrace.Body.ofCodecs (vote_quorum.wireCodecs (mem := mem) coord part minYes vote) := by
  funext m r chans Δ s
  exact quorumRun.eq coord part minYes vote m true r chans Δ s
--[/run]--

/-- An increment of a step history is part of its view at that step. -/
theorem mem_view_of_mem_incView' {α : Type} (h : StepHist α) (k : Nat) {x : α}
    (hx : x ∈ incView h k) : x ∈ h.view k := by
  cases k with
  | zero => exact hx
  | succ k =>
    have hd : incView h (k + 1) = (h.view (k + 1)).drop (h.view k).length := by
      simp only [incView, StepHist.inc]
    rw [hd] at hx
    exact List.mem_of_mem_drop hx

--[injected]--
/-- The requests a trace has handed coordinator member `c` at its own activations among the
first `T + 1` steps (an increment given to a process that is not running is not consumed). -/
def injected {coord part : L} {minYes : Nat} {vote : Fin (mem part) → Nat → Bool}
    (τ : WireTrace (vote_quorum.dag (mem := mem) coord part minYes vote))
    (c : Fin (mem coord)) (T : Nat) : Multiset Nat :=
  ↑(accum (fun t => ((envMask (τ.who t) (τ.inp t)).get Var.zero c).toList) T)
--[/injected]--

--[safe-wired]--
--[safe-wired-stmt]--
/-- **The quorum, as processes exchanging bytes.** For any trace at the wire (which process
runs at each step, what it is handed, how its sockets cut the bytes): a request that coordinator
member `c` has output as decided by step `T`, and that the trace injected at most once, was
voted yes by `minYes` participants. -/
theorem vote_quorum_safe_wired (coord part : L) (minYes : Nat)
    (vote : Fin (mem part) → Nat → Bool)
    (τ : WireTrace (vote_quorum.dag (mem := mem) coord part minYes vote))
    (T : Nat) (c : Fin (mem coord)) (p : Nat)
    (hp : p ∈ τ.streamOutputsAt (quorumBody coord part minYes vote) T ⟨coord, c⟩ Var.zero ⟨c, rfl⟩)
    (hone : (proposed fun c => injected τ c T).count p ≤ 1) :
    minYes ≤ yesVoters vote p := by
  --[/safe-wired-stmt]--
  -- the staged body equals the interpreter's; the trace's run equals its induced interleaving's
  rw [quorumBody_eq, τ.streamOutputsAt_eq (vote_quorum.dag_valid coord part minYes vote)] at hp
  set I := τ.interleaving (vote_quorum.wireCodecs (mem := mem) coord part minYes vote) with hI
  -- the activation that output it, and its outputs as the lock-step step's
  obtain ⟨k, hkT, hk, hp⟩ := I.mem_streamOutputsAt hp
  rw [I.loopPass_outputs] at hp
  -- the schedule, as the program's decision and schedule records
  let sdec := (vote_quorum.recordOf I.pacing coord part minYes vote I.sched).1
  let ssched := (vote_quorum.recordOf I.pacing coord part minYes vote I.sched).2
  have hS : vote_quorum.schedOf I.pacing coord part minYes vote sdec ssched = I.sched :=
    vote_quorum.schedOf_recordOf I.pacing coord part minYes vote I.sched
  let reqS := I.wires.get Var.zero
  have hout : ∀ k (i : Hosted (I.who k) coord),
      (I.active k).2.1.get Var.zero i
        = (incView ((vote_quorum (SchedSem L mem I.pacing) coord part minYes vote reqS sdec
            ssched) i.1) k).toArray := by
    intro k i
    rw [I.outputs (vote_quorum.dag_valid coord part minYes vote) k]
    show (envRestrict (I.who k) (envIncAt (Dag.denote I.pacing
      (vote_quorum.dag coord part minYes vote) I.sched I.wires) k)).get Var.zero i = _
    rw [← hS, vote_quorum.dag_denote I.pacing coord part minYes vote sdec ssched I.wires,
      envRestrict_get]
    rfl
  rw [hout k _] at hp
  have hpk := mem_view_of_mem_incView' _ _ (List.mem_toArray.mp hp)
  -- the step-machine theorem at step `k`
  refine vote_quorum_safe_sched coord part minYes vote reqS ssched k _ p hpk ?_
  · refine le_trans (Multiset.count_le_of_le p ?_) hone
    unfold proposed
    refine List.sum_le_sum (fun i _ => ?_)
    have hv : (reqS i).view k
        = accum (fun t => ((envMask (τ.who t) (τ.inp t)).get Var.zero i).toList) k := rfl
    unfold injected
    rw [hv]
    exact Multiset.coe_le.mpr (accum_prefix_le _ (Nat.le_of_lt hkT)).sublist.subperm
--[/safe-wired]--

end Wired

end Tutorials

end Hydro
