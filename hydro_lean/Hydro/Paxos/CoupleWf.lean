import Hydro.Paxos.CoupleSafety

/-!
# Machine-run Paxos safety: whole-program wf + the premise-free
headline (D41)

`paxos_co_wf` (the corner run of `paxos_core` carries no residual
obligation), its corollary `paxos_co_cpl` (the machine/denotational
coupling with **no** satisfiability premise), and the headline
`paxos_safe_sched'` — all assembled from the artifacts generated at
the tail of `Paxos/PaxosCore.lean` — and the machine-only headline
`paxos_safe_sched`, which discharges `paxos_safe_sched'`'s Values-side
inputs.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

section PaxosWf

variable (pacing : Pacing L mem) (prop acc : L)
  (T : Nat) (config : PaxosConfig)
  (cpS : (SchedSem L mem pacing).Stream prop P .totalOrder .exactlyOnce)
  (cpV : (Values L mem).Stream prop P .totalOrder .exactlyOnce)
  (hcp : ∀ t i, ((cpS i).view t) <+: cpV i)
  {ckα : Type} [inst_ck : DecidableEq ckα] {ckord : StrOrd}
  {ckret : Retries}
  (ckS : (SchedSem L mem pacing).Singleton acc ckα (Option Nat) ckord
    ckret (.monotonic ValueOrder.optNat))
  (ckSrc : (SchedSem L mem pacing).Stream acc ckα ckord ckret)
  (ckV : (Values L mem).Singleton acc ckα (Option Nat) ckord ckret
    (.monotonic ValueOrder.optNat))
  (hck : ∃ (g : Option Nat → ckα → Option Nat) (init : Option Nat)
    (ok : FoldOkP ckord ckret (fun _ _ => True) g)
    (pool : Fin (mem acc) → PoolCarrier ckα ckord ckret),
    (∀ j, (ckV j).val = snapTrace ckord ckret (fun _ _ => True) g init ok (pool j)) ∧
    (∀ j t, ckS j t = ((ckSrc j).view t).foldl g init) ∧
    (∀ t j, ListLe ckord ckret ((ckSrc j).view t) (pool j)))
  (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc)
    P ckα ckord)
  (ssched : PaxosCoreSched (SchedSem L mem pacing) (mem prop)
    (mem acc) P)

/-- **Whole-program well-formedness**: the corner run of `paxos_core`
carries no residual obligation — all five knots' triples are
discharged by construction (D41; the generated
`paxos_core_co_wf₂`). -/
theorem paxos_co_wf :
    (paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
        .guarded prop acc config (CoStream.inputC cpS cpV hcp)
        (CoSing.inputC ckS ckSrc ckV hck)
        (paxosCoDec pacing prop acc T sdec)
        (paxosCoSched pacing prop acc T ssched)).2.wf :=
  paxos_core_co_wf₂ .guarded prop acc config (CoStream.inputC cpS cpV hcp)
    (CoSing.inputC ckS ckSrc ckV hck) (paxosCoDec pacing prop acc T sdec)
    (paxosCoSched pacing prop acc T ssched) trivial trivial rfl

/-- **The coupling, premise-free**: the machine run's replica views
below any horizon sit inside the denotational pools. -/
theorem paxos_co_cpl (i : Fin (mem prop)) :
    ListLe .noOrder .exactlyOnce
      (((paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
          .guarded prop acc config (CoStream.inputC cpS cpV hcp)
          (CoSing.inputC ckS ckSrc ckV hck)
          (paxosCoDec pacing prop acc T sdec)
          (paxosCoSched pacing prop acc T ssched)).2.sr i).view T)
      ((paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
          .guarded prop acc config (CoStream.inputC cpS cpV hcp)
          (CoSing.inputC ckS ckSrc ckV hck)
          (paxosCoDec pacing prop acc T sdec)
          (paxosCoSched pacing prop acc T ssched)).2.rr i) :=
  (paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
      .guarded prop acc config (CoStream.inputC cpS cpV hcp)
      (CoSing.inputC ckS ckSrc ckV hck)
      (paxosCoDec pacing prop acc T sdec)
      (paxosCoSched pacing prop acc T ssched)).2.cpl
    (paxos_co_wf pacing prop acc T config cpS cpV hcp ckS ckSrc ckV hck sdec
      ssched) i

include cpV ckV hcp hck in
/-- **The transfer lemma** (the D41 headline before its scaffolding was discharged):
any two commits the step machine has emitted at one slot, by any horizon, under any
pacing and any schedule, agree — with **no** satisfiability premise and **no** decision
argument — *given* Values-side pools above the inputs (`cpV`/`hcp`, `ckV`/`hck`). Those
are built from the inputs themselves in `paxos_safe_sched` below, which is the
statement to read. (Stated here rather than in `CoupleSafety.lean` because it consumes
`paxos_co_cpl`, which lives above `CoupleSafety` in the import order.) -/
theorem paxos_safe_sched' (hnA : mem acc ≤ 2 * config.f + 1)
    (i j : Fin (mem prop)) (s : Nat) (w w' : Option P)
    (hw : (s, w) ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop
      acc config cpS ckS sdec ssched).2 i).view T)
    (hw' : (s, w') ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop
      acc config cpS ckS sdec ssched).2 j).view T) :
    w = w' := by
  have hens := (paxos_core.ensures .guarded prop acc config cpV ckV
    (paxosVDecG pacing prop acc T config cpS ckS ckSrc sdec ssched)
    PaxosCoreSched.triv).slot_functional rfl hnA
  have hcpl_i := paxos_co_cpl pacing prop acc T config cpS cpV hcp ckS ckSrc ckV
    hck sdec ssched i
  have hcpl_j := paxos_co_cpl pacing prop acc T config cpS cpV hcp ckS ckSrc ckV
    hck sdec ssched j
  rw [paxos_co_sr pacing prop acc T config cpS cpV hcp ckS ckSrc ckV hck sdec
      ssched,
    paxos_co_rr pacing prop acc T config cpS cpV hcp ckS ckSrc ckV hck sdec
      ssched]
    at hcpl_i hcpl_j
  exact hens i j s w w'
    (Multiset.mem_of_le hcpl_i (Multiset.mem_coe.mpr hw))
    (Multiset.mem_of_le hcpl_j (Multiset.mem_coe.mpr hw'))

end PaxosWf

/-! ## The machine-only headline, inputs unconstrained

`paxos_safe_sched'` is what the coupling transfer produces; its Values-side inputs
(`cpV`, `hcp`, `ckV`, `hck`) are the shape `CoStream.inputC`/`CoSing.inputC` need, not
facts about a run. `paxos_safe_sched` discharges them: the pools are **built in the
proof** from the inputs truncated at the horizon (every finite prefix is its own pool;
the checkpoint register is the `optMax`-fold of its own value history), and the
commits are carried back to the untruncated run by causality (`paxos_core.dag_denote`
+ `Dag.denote_causal`). This closes `SCHED_AUDIT.md` F2: unbounded inputs are inside
the quantifiers. The one input-side premise that remains is the program's own typing
of the checkpoint wire: it only grows (`CkMono`). -/

section Machine

variable (pacing : Pacing L mem) (prop acc : L) (config : PaxosConfig)

open Project

/-- The machine's checkpoint input (the acceptor's `Option<usize>` register): one
value per member per step. -/
abbrev CkInput := (SchedSem L mem pacing).Singleton acc (Option Nat) (Option Nat) .totalOrder .exactlyOnce
  (.monotonic ValueOrder.optNat)

/-- Checkpoints only grow — the program's `Monotonic` typing of the wire, as a fact
about the machine input. -/
def CkMono (ck : CkInput pacing acc) : Prop :=
  ∀ j t, ValueOrder.optNat.le (ck j t) (ck j (t + 1))

-- (`optMax`, `optMax_le_left`, `optMax_of_le` — the inflationary `max` on
-- `none`-bottomed naturals — live in `Trace.lean` since D81.)

/-- The source list a checkpoint register is the running fold of: its values up to `m`. -/
def ckList (ck : Fin (mem acc) → Nat → Option Nat) (j : Fin (mem acc)) (m : Nat) : List (Option Nat) :=
  (List.range (m + 1)).map (ck j)

theorem range_prefix {a b : Nat} (h : a ≤ b) : List.range a <+: List.range b := by
  induction b with
  | zero => cases h; exact List.prefix_rfl
  | succ b ih =>
    rcases Nat.lt_or_eq_of_le h with h | h
    · exact (ih (Nat.le_of_lt_succ h)).trans ⟨[b], (List.range_succ).symm⟩
    · subst h; exact List.prefix_rfl

theorem ckList_prefix (ck : Fin (mem acc) → Nat → Option Nat) (j : Fin (mem acc)) {m m' : Nat} (h : m ≤ m') :
    ckList acc ck j m <+: ckList acc ck j m' :=
  (range_prefix (Nat.succ_le_succ h)).map _

/-- On a monotone register the inflationary fold reads the register. -/
theorem foldl_optMax_ckList (ck : Fin (mem acc) → Nat → Option Nat)
    (hmono : ∀ j t, ValueOrder.optNat.le (ck j t) (ck j (t + 1))) (j : Fin (mem acc)) :
    ∀ m, (ckList acc ck j m).foldl optMax none = ck j m
  | 0 => by simp [ckList, optMax]
  | m + 1 => by
    unfold ckList
    rw [List.range_succ, List.map_append, List.map_singleton, List.foldl_append, List.foldl_cons, List.foldl_nil]
    show optMax ((ckList acc ck j m).foldl optMax none) (ck j (m + 1)) = _
    rw [foldl_optMax_ckList ck hmono j m]
    exact optMax_of_le (hmono j m)

/-- **Machine-run Paxos safety, inputs unconstrained.** Any client stream, any
checkpoint register that only grows, any decisions, any horizon: two commits at one slot
agree. -/
theorem paxos_safe_sched (T : Nat) (hnA : mem acc ≤ 2 * config.f + 1)
    (cpS : (SchedSem L mem pacing).Stream prop P .totalOrder .exactlyOnce)
    (ckS : CkInput pacing acc) (hck : CkMono pacing acc ckS)
    (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc) P (Option Nat) .totalOrder)
    (ssched : PaxosCoreSched (SchedSem L mem pacing) (mem prop) (mem acc) P)
    (i j : Fin (mem prop)) (s : Nat) (w w' : Option P)
    (hw : (s, w) ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop acc config cpS ckS sdec ssched).2 i).view T)
    (hw' : (s, w') ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop acc config cpS ckS sdec ssched).2 j).view T) :
    w = w' := by
  -- the inputs cut at the horizon
  let cpS' : (SchedSem L mem pacing).Stream prop P .totalOrder .exactlyOnce := fun i => (cpS i).truncate T
  let ckS' : CkInput pacing acc := fun j t => ckS j (min t T)
  -- the run agrees with the truncated run up to `T` (causality, through the graph)
  let env : Env [Kind.stream prop P inferInstance .totalOrder .exactlyOnce,
      Kind.sing acc (Option Nat) (Option Nat) inferInstance .totalOrder .exactlyOnce (.monotonic ValueOrder.optNat)] :=
    HList.cons (f := Kind.sched (mem := mem)) cpS (HList.cons (f := Kind.sched (mem := mem)) ckS HList.nil)
  let env' : Env [Kind.stream prop P inferInstance .totalOrder .exactlyOnce,
      Kind.sing acc (Option Nat) (Option Nat) inferInstance .totalOrder .exactlyOnce (.monotonic ValueOrder.optNat)] :=
    HList.cons (f := Kind.sched (mem := mem)) cpS' (HList.cons (f := Kind.sched (mem := mem)) ckS' HList.nil)
  have hagree : HAgree T env env' := by
    refine ⟨fun i t ht => ?_, fun j t ht => ?_, trivial⟩
    · show (cpS i).view t = (cpS i).view (min t T)
      rw [Nat.min_eq_left ht]
    · show ckS j t = ckS j (min t T)
      rw [Nat.min_eq_left ht]
  have hcaus := Dag.denote_causal (pacing := pacing) (paxos_core.dag (ckret := .exactlyOnce) .guarded prop acc config)
    (paxos_core.schedOf pacing .guarded prop acc config sdec ssched) T env env' hagree
  rw [paxos_core.dag_denote, paxos_core.dag_denote] at hcaus
  have hcommits : ∀ i, ((paxos_core (SchedSem L mem pacing) .guarded prop acc config cpS ckS sdec ssched).2 i).view T
      = ((paxos_core (SchedSem L mem pacing) .guarded prop acc config cpS' ckS' sdec ssched).2 i).view T :=
    fun i => hcaus.2.1 i T (Nat.le_refl T)
  rw [hcommits] at hw hw'
  -- the pools above the truncated inputs: the views at `T`
  let cpV : (Values L mem).Stream prop P .totalOrder .exactlyOnce := .mk' fun i => (cpS i).view T
  have hcp : ∀ t i, ((cpS' i).view t) <+: cpV i := fun t i => (cpS i).mono_le (Nat.min_le_right t T)
  let ckSrc : (SchedSem L mem pacing).Stream acc (Option Nat) .totalOrder .exactlyOnce :=
    fun j => ⟨fun t => ckList acc ckS j (min t T), fun t => ckList_prefix acc ckS j (min_le_min_right T (Nat.le_succ t))⟩
  let pool : Fin (mem acc) → PoolCarrier (Option Nat) .totalOrder .exactlyOnce := fun j => ckList acc ckS j T
  have hinfl : ∀ s x, ValueOrder.optNat.le s (optMax s x) := optMax_le_left
  let ckV : (Values L mem).Singleton acc (Option Nat) (Option Nat) .totalOrder .exactlyOnce (.monotonic ValueOrder.optNat) :=
    .mk' fun j => ⟨snapTrace .totalOrder .exactlyOnce (fun _ _ => True) optMax none trivial (pool j),
      fun d => snapTrace_ascending ValueOrder.optNat optMax none trivial hinfl (pool j) d⟩
  have hckc : ∃ (g : Option Nat → Option Nat → Option Nat) (init : Option Nat)
      (ok : FoldOkP .totalOrder .exactlyOnce (fun _ _ => True) g)
      (pool : Fin (mem acc) → PoolCarrier (Option Nat) .totalOrder .exactlyOnce),
      (∀ j, (ckV j).val = snapTrace .totalOrder .exactlyOnce (fun _ _ => True) g init ok (pool j)) ∧
      (∀ j t, ckS' j t = ((ckSrc j).view t).foldl g init) ∧
      (∀ t j, ListLe .totalOrder .exactlyOnce ((ckSrc j).view t) (pool j)) :=
    ⟨optMax, none, trivial, pool, fun _ => rfl,
      fun j t => (foldl_optMax_ckList acc ckS hck j (min t T)).symm,
      fun t j => ckList_prefix acc ckS j (Nat.min_le_right t T)⟩
  exact paxos_safe_sched' pacing prop acc T config cpS' cpV hcp ckS' ckSrc ckV hckc sdec ssched hnA i j s w w' hw hw'

end Machine


end Hydro
