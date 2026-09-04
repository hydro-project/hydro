import Hydro.Sched
import Hydro.Values
import Mathlib.Data.List.Chain

/-!
# Hydro · the transfer coupling (concrete runs ⊑ graded quotients)

The lemma layer relating the step machine's plain-list runs
(`Sched.lean`) to the graded denotational carriers (`Values.lean`):

- `ListLe ord ret` — **the coupling**: a concrete buffer is a
  representative-below of a graded pool, in exactly the grade's own
  relaxation (prefix / destuttered prefix / sub-multiset / support
  inclusion). One relation, graded; the four faces mirror `PoolLe`.
- preservation lemmas: every machine construction (map, filterMap,
  delivery `take`, fan-in merges, grade weakening) stays coupled.
- **read agreement** (`sched_snap_eq`): the machine's fold-accumulator
  reads at its tick steps *equal* the denotational `snapTrace` at
  **derived** cut decisions, constructed from the run — the per-op
  content of the `∀ machine run, ∃ denotational run` transfer. The
  grade obligations (`FoldOkP`) are consumed exactly here: the machine
  folds raw arrival order; commutativity/idempotence pay the difference
  to the quotient reads.
-/

namespace Hydro

private theorem ms_shuffle2 {α : Type} (a b c d : Multiset α) :
    a + b + (c + d) = a + c + (b + d) := by
  rw [Multiset.add_assoc,
    show b + (c + d) = c + (b + d) from by
      rw [← Multiset.add_assoc, Multiset.add_comm b c,
        Multiset.add_assoc],
    ← Multiset.add_assoc]

private theorem ms_shuffle {α : Type} (a b c d : Multiset α) :
    a + b + c + d = a + c + (b + d) := by
  rw [Multiset.add_assoc (a + b) c d]
  exact ms_shuffle2 a b c d

/-! ## The coupling -/

/-- Concrete-buffer-below-graded-pool, in the grade's own relaxation. -/
@[reducible] def ListLe {α : Type} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → List α →
      PoolCarrier α ord ret → Prop
  | .totalOrder, .exactlyOnce => fun s v => s <+: v
  | .totalOrder, .atLeastOnce => fun s v => StutterSeq.le (.mk s) v
  | .noOrder, .exactlyOnce => fun s v => (↑s : Multiset α) ≤ v
  | .noOrder, .atLeastOnce => fun s v => RetryPool.le (.mk ↑s) v

theorem ListLe.trans_pool {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {s : List α} {v v' : PoolCarrier α ord ret}
    (h : ListLe ord ret s v) (hvv : PoolLe ord ret v v') :
    ListLe ord ret s v' := by
  cases ord <;> cases ret
  · exact h.trans hvv
  · exact StutterSeq.le_trans h hvv
  · exact le_trans h hvv
  · exact RetryPool.le_trans h hvv

/-! ## Preservation: pure ops -/

theorem ListLe.map_eo {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd} (f : α → β) {s : List α}
    {v : PoolCarrier α ord .exactlyOnce}
    (h : ListLe ord .exactlyOnce s v) :
    ListLe ord .exactlyOnce (s.map f) (mapPool (ord := ord) f v) := by
  cases ord
  · exact List.IsPrefix.map f h
  · show (↑(s.map f) : Multiset β) ≤ Multiset.map f v
    rw [← Multiset.map_coe]
    exact Multiset.map_le_map h

theorem ListLe.filterMap_eo {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd} (f : α → Option β) {s : List α}
    {v : PoolCarrier α ord .exactlyOnce}
    (h : ListLe ord .exactlyOnce s v) :
    ListLe ord .exactlyOnce (s.filterMap f)
      (filterMapPool (ord := ord) f v) := by
  cases ord
  · exact prefix_filterMap f h
  · show (↑(s.filterMap f) : Multiset β) ≤ Multiset.filterMap f v
    rw [← Multiset.filterMap_coe]
    exact Multiset.filterMap_le_filterMap f h

/-! ## Preservation: delivery (`take`) -/

/-- Destuttering a take yields a prefix of the destuttering. -/
theorem destutter_take_prefix {α : Type _} [DecidableEq α]
    (l : List α) (k : Nat) :
    destutter (l.take k) <+: destutter l := by
  conv => rhs; rw [← List.take_append_drop k l]
  exact destutter_append_prefix (l.take k) (l.drop k)

/-- A prefix's destuttering is a prefix of the destuttering. -/
theorem destutter_prefix {α : Type _} [DecidableEq α]
    {l l' : List α} (h : l <+: l') : destutter l <+: destutter l' := by
  obtain ⟨e, rfl⟩ := h
  exact destutter_append_prefix l e

theorem ListLe.take {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (k : Nat) {s : List α}
    {v : PoolCarrier α ord ret} (h : ListLe ord ret s v) :
    ListLe ord ret (s.take k) v := by
  cases ord <;> cases ret
  · exact (List.take_prefix _ _).trans h
  · exact StutterSeq.le_trans
      (show StutterSeq.le (.mk (s.take k)) (.mk s) from
        destutter_take_prefix s k) h
  · exact le_trans
      (Multiset.coe_le.mpr (List.take_sublist _ _).subperm) h
  · exact RetryPool.le_trans
      (fun x hx => by
        have : x ∈ s.take k := by
          rwa [RetryPool.mem_mk, Multiset.mem_coe] at hx
        rw [RetryPool.mem_mk, Multiset.mem_coe]
        exact List.mem_of_mem_take this) h

/-! ## Preservation: grade weakening -/

/-! ## Preservation: fan-in merges -/

theorem coe_merge2View {α : Type} [DecidableEq α] (a b : StepHist α) :
    ∀ t, (↑(merge2View a b t) : Multiset α)
      = (↑(a.view t) : Multiset α) + ↑(b.view t)
  | 0 => by simp [merge2View]
  | t + 1 => by
    show (↑(merge2View a b t ++ a.inc t ++ b.inc t) : Multiset α) = _
    rw [← Multiset.coe_add, ← Multiset.coe_add, coe_merge2View a b t,
      a.view_succ t, b.view_succ t, ← Multiset.coe_add,
      ← Multiset.coe_add]
    exact ms_shuffle _ _ _ _

theorem coe_mergeNView {m : Nat} {α : Type} [DecidableEq α]
    (k : Fin m → StepHist α) :
    ∀ t, (↑(mergeNView k t) : Multiset α)
      = ((List.finRange m).map (fun j => (↑((k j).view t) : Multiset α))).sum
  | 0 => by
    show (↑((List.finRange m).flatMap (fun j => (k j).view 0)) : Multiset α) = _
    induction (List.finRange m) with
    | nil => simp
    | cons j js ih => simp_all [List.flatMap_cons, ← Multiset.coe_add]
  | t + 1 => by
    show (↑(mergeNView k t
        ++ (List.finRange m).flatMap (fun j => (k j).inc t)) : Multiset α) = _
    rw [← Multiset.coe_add, coe_mergeNView k t]
    have hflat : ∀ (js : List (Fin m)),
        (↑(js.flatMap (fun j => (k j).inc t)) : Multiset α)
          = (js.map (fun j => (↑((k j).inc t) : Multiset α))).sum := by
      intro js
      induction js with
      | nil => simp
      | cons j js ih => simp_all [List.flatMap_cons, ← Multiset.coe_add]
    have hsum : ∀ (js : List (Fin m)),
        (js.map (fun j => (↑((k j).view t) : Multiset α))).sum
          + (js.map (fun j => (↑((k j).inc t) : Multiset α))).sum
        = (js.map (fun j =>
            (↑((k j).view t) : Multiset α) + ↑((k j).inc t))).sum := by
      intro js
      induction js with
      | nil => simp
      | cons j js ih =>
        simp only [List.map_cons, List.sum_cons, ← ih]
        exact ms_shuffle2 _ _ _ _
    have hview : (List.finRange m).map
        (fun j => (↑((k j).view (t + 1)) : Multiset α))
        = (List.finRange m).map (fun j =>
            (↑((k j).view t) : Multiset α) + ↑((k j).inc t)) :=
      List.map_congr_left (fun j _ => by
        rw [(k j).view_succ t]; exact (Multiset.coe_add _ _).symm)
    rw [hflat, hview, ← hsum]

/-! ## Read agreement (the derived cuts) -/

/-- Derived prefix cuts: length increments of an ascending view chain. -/
def cutsLen {α : Type} (acc : Nat) : List (List α) → List Nat
  | [] => []
  | w :: ws => (w.length - acc) :: cutsLen w.length ws

/-- Derived multiset cuts: content increments of a view chain. -/
def cutsMS {α : Type} [DecidableEq α] (acc : Multiset α) :
    List (List α) → List (Multiset α)
  | [] => []
  | w :: ws => (↑w - acc) :: cutsMS (↑w : Multiset α) ws

/-- Ordered reads realize the views exactly at the derived cuts. -/
theorem prefixCuts_views {α : Type} {pool : List α} :
    ∀ (ws : List (List α)) (prev : List α), prev <+: pool →
      List.IsChain (· <+: ·) (prev :: ws) →
      (∀ w ∈ ws, w <+: pool) →
      prefixCuts pool prev.length (cutsLen prev.length ws) = ws
  | [], _, _, _, _ => rfl
  | w :: ws, prev, hprev, hchain, hpool => by
    have hpw : prev <+: w := (List.isChain_cons.mp hchain).1 w rfl
    have hwp : w <+: pool := hpool w (List.mem_cons_self ..)
    have hlen : prev.length + (w.length - prev.length) = w.length :=
      Nat.add_sub_cancel' hpw.length_le
    show prefixCuts pool prev.length
      ((w.length - prev.length) :: cutsLen w.length ws) = w :: ws
    unfold prefixCuts
    rw [hlen, if_pos hwp.length_le,
      (List.prefix_iff_eq_take.mp hwp).symm]
    exact congrArg (w :: ·)
      (prefixCuts_views ws w hwp (List.isChain_cons.mp hchain).2
        (fun u hu => hpool u (List.mem_cons_of_mem _ hu)))

/-- Count-legal unordered reads realize the views' contents exactly at
the derived increments. -/
theorem snapshotCuts_views {α : Type} [DecidableEq α]
    {pool : Multiset α} :
    ∀ (ws : List (List α)) (prev : List α),
      List.IsChain (· <+: ·) (prev :: ws) →
      (∀ w ∈ ws, (↑w : Multiset α) ≤ pool) →
      snapshotCuts pool ↑prev (cutsMS (↑prev) ws)
        = ws.map (fun w => Multiset.ofList w)
  | [], _, _, _ => rfl
  | w :: ws, prev, hchain, hpool => by
    have hpw : prev <+: w := (List.isChain_cons.mp hchain).1 w rfl
    have hle : (↑prev : Multiset α) ≤ ↑w :=
      Multiset.coe_le.mpr hpw.sublist.subperm
    have hacc : (↑prev : Multiset α) + (↑w - ↑prev) = ↑w := by
      rw [Multiset.add_comm]; exact Multiset.sub_add_cancel hle
    show snapshotCuts pool ↑prev
      ((↑w - ↑prev) :: cutsMS (↑w : Multiset α) ws) = _
    unfold snapshotCuts
    rw [hacc, if_pos (hpool w (List.mem_cons_self ..))]
    exact congrArg ((↑w : Multiset α) :: ·)
      (snapshotCuts_views ws w (List.isChain_cons.mp hchain).2
        (fun u hu => hpool u (List.mem_cons_of_mem _ hu)))

/-- Membership-legal unordered reads realize the views' contents at the
derived increments. -/
theorem snapshotMemCuts_views {α : Type} [DecidableEq α]
    {spool : Multiset α} :
    ∀ (ws : List (List α)) (prev : List α),
      List.IsChain (· <+: ·) (prev :: ws) →
      (∀ w ∈ ws, ∀ x ∈ w, x ∈ spool) →
      snapshotMemCuts spool ↑prev (cutsMS (↑prev) ws)
        = ws.map (fun w => Multiset.ofList w)
  | [], _, _, _ => rfl
  | w :: ws, prev, hchain, hpool => by
    have hpw : prev <+: w := (List.isChain_cons.mp hchain).1 w rfl
    have hle : (↑prev : Multiset α) ≤ ↑w :=
      Multiset.coe_le.mpr hpw.sublist.subperm
    have hacc : (↑prev : Multiset α) + (↑w - ↑prev) = ↑w := by
      rw [Multiset.add_comm]; exact Multiset.sub_add_cancel hle
    have hmem : ∀ x ∈ (↑w - ↑prev : Multiset α), x ∈ spool := by
      intro x hx
      exact hpool w (List.mem_cons_self ..) x
        (by
          have := Multiset.mem_of_le (Multiset.sub_le_self _ _) hx
          rwa [Multiset.mem_coe] at this)
    show snapshotMemCuts spool ↑prev
      ((↑w - ↑prev) :: cutsMS (↑w : Multiset α) ws) = _
    unfold snapshotMemCuts
    rw [if_pos hmem, hacc]
    exact congrArg ((↑w : Multiset α) :: ·)
      (snapshotMemCuts_views ws w (List.isChain_cons.mp hchain).2
        (fun u hu => hpool u (List.mem_cons_of_mem _ hu)))

private theorem map_msfold_eq {α σ : Type} [DecidableEq α] (Pre : α → α → Prop)
    [DecidableRel Pre] (g : σ → α → σ) (init : σ)
    (ok : FoldOkP .noOrder .exactlyOnce Pre g) :
    ∀ ws : List (List α), (∀ w ∈ ws, PairwiseL Pre w) →
      (ws.map (fun w => Multiset.ofList w)).map
        (PoolFold .noOrder .exactlyOnce Pre g init ok)
      = ws.map (fun w => w.foldl g init)
  | [], _ => rfl
  | w :: ws, hp => by
    simp only [List.map_cons]
    rw [map_msfold_eq Pre g init ok ws (fun w hw => hp w (List.mem_cons_of_mem _ hw))]
    show Multiset.foldlOn Pre g init ok (↑w) :: _ = _
    rw [Multiset.foldlOn_coe Pre g init ok (hp w (List.mem_cons_self ..))]

private theorem map_rpfold_eq {α σ : Type} [DecidableEq α] (Pre : α → α → Prop)
    [DecidableRel Pre] (g : σ → α → σ) (init : σ)
    (ok : FoldOkP .noOrder .atLeastOnce Pre g) :
    ∀ ws : List (List α), (∀ w ∈ ws, PairwiseL Pre w) →
      ((ws.map (fun w => Multiset.ofList w)).map RetryPool.mk).map
        (PoolFold .noOrder .atLeastOnce Pre g init ok)
      = ws.map (fun w => w.foldl g init)
  | [], _ => rfl
  | w :: ws, hp => by
    simp only [List.map_cons]
    rw [map_rpfold_eq Pre g init ok ws (fun w hw => hp w (List.mem_cons_of_mem _ hw))]
    show RetryPool.foldOn Pre g init ok.1 ok.2 (RetryPool.mk ↑w) :: _ = _
    rw [RetryPool.foldOn_mk_coe Pre g init ok.1 ok.2 (hp w (List.mem_cons_self ..))]

/-- **Read agreement**: the machine's raw-arrival-order fold reads of an
ascending view chain *equal* the denotational `snapTrace` at derived
cuts, at every grade — the quotients' obligations pay the difference
between raw and quotient folds. -/
theorem sched_snap_eq {α σ : Type} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (Pre : α → α → Prop) [DecidableRel Pre] (g : σ → α → σ) (init : σ)
    (ok : FoldOkP ord ret Pre g)
    (pool : PoolCarrier α ord ret) (ws : List (List α))
    (hchain : List.IsChain (· <+: ·) (([] : List α) :: ws))
    (hcpl : ∀ w ∈ ws, ListLe ord ret w pool)
    (hpre : PoolPairwise ord ret Pre pool) :
    ∃ d, ws.map (fun w => w.foldl g init)
      = snapTrace ord ret Pre g init ok pool d := by
  cases ord <;> cases ret
  case totalOrder.exactlyOnce =>
    refine ⟨cutsLen 0 ws, ?_⟩
    show _ = (prefixCuts pool 0 (cutsLen 0 ws)).map
      (fun l => l.foldl g init)
    rw [show (0 : Nat) = ([] : List α).length from rfl,
      prefixCuts_views ws [] (List.nil_prefix) hchain hcpl]
  case totalOrder.atLeastOnce =>
    refine ⟨cutsLen 0 (ws.map destutter), ?_⟩
    show _ = ((prefixCuts pool.norm 0 _).map StutterSeq.mk).map
      (PoolFold .totalOrder .atLeastOnce Pre g init ok)
    have hch : List.IsChain (· <+: ·)
        (([] : List α) :: ws.map destutter) := by
      rcases ws with _ | ⟨w, ws⟩
      · exact hchain
      · rw [List.map_cons]
        refine List.isChain_cons.mpr ⟨fun y hy => ?_, ?_⟩
        · cases hy; exact List.nil_prefix
        · rw [← List.map_cons]
          exact List.isChain_map_of_isChain destutter
            (fun _ _ h => destutter_prefix h)
            (List.isChain_cons.mp hchain).2
    rw [show (0 : Nat) = ([] : List α).length from rfl,
      prefixCuts_views (ws.map destutter) [] List.nil_prefix hch
        (by
          intro u hu
          obtain ⟨w, hw, rfl⟩ := List.mem_map.mp hu
          exact hcpl w hw)]
    rw [List.map_map, List.map_map]
    refine List.map_congr_left (fun w _hw => ?_)
    exact StutterSeq.foldl_destutter g ok w init
  case noOrder.exactlyOnce =>
    refine ⟨cutsMS 0 ws, ?_⟩
    show _ = (snapshotCuts pool 0 (cutsMS 0 ws)).map
      (PoolFold .noOrder .exactlyOnce Pre g init ok)
    rw [show (0 : Multiset α) = (↑([] : List α) : Multiset α) from rfl,
      snapshotCuts_views ws [] hchain hcpl]
    exact (map_msfold_eq Pre g init ok ws
      (fun w hw => PoolPairwise.of_coe_le (hcpl w hw) hpre)).symm
  case noOrder.atLeastOnce =>
    refine ⟨cutsMS 0 ws, ?_⟩
    show _ = ((snapshotMemCuts (RetryPool.support pool).val 0
        (cutsMS 0 ws)).map RetryPool.mk).map
      (PoolFold .noOrder .atLeastOnce Pre g init ok)
    rw [show (0 : Multiset α) = (↑([] : List α) : Multiset α) from rfl,
      snapshotMemCuts_views ws []
        hchain
        (by
          intro w hw x hx
          exact RetryPool.mem_support_val.mpr
            (hcpl w hw x (by
              rw [RetryPool.mem_mk, Multiset.mem_coe]; exact hx))),
    ]
    exact (map_rpfold_eq Pre g init ok ws
      (fun w hw => PoolPairwise.of_mk_le (hcpl w hw) hpre)).symm



/-- The named snapshot-cut derivation: the `sched_snap_eq` witness in
closed form — *the* function from a machine run to the denotational
decision its reads realize. -/
def snapCut {α : Type} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → List (List α) → CutDec α ord
  | .totalOrder, .exactlyOnce => fun ws => cutsLen 0 ws
  | .totalOrder, .atLeastOnce => fun ws => cutsLen 0 (ws.map destutter)
  | .noOrder, .exactlyOnce => fun ws => cutsMS 0 ws
  | .noOrder, .atLeastOnce => fun ws => cutsMS 0 ws

/-- `sched_snap_eq`, with its witness named (`snapCut`). -/
theorem sched_snap_eq' {α σ : Type} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (Pre : α → α → Prop) [DecidableRel Pre] (g : σ → α → σ) (init : σ)
    (ok : FoldOkP ord ret Pre g)
    (pool : PoolCarrier α ord ret) (ws : List (List α))
    (hchain : List.IsChain (· <+: ·) (([] : List α) :: ws))
    (hcpl : ∀ w ∈ ws, ListLe ord ret w pool)
    (hpre : PoolPairwise ord ret Pre pool) :
    ws.map (fun w => w.foldl g init)
      = snapTrace ord ret Pre g init ok pool (snapCut ord ret ws) := by
  cases ord <;> cases ret
  case totalOrder.exactlyOnce =>
    show _ = (prefixCuts pool 0 (cutsLen 0 ws)).map
      (fun l => l.foldl g init)
    rw [show (0 : Nat) = ([] : List α).length from rfl,
      prefixCuts_views ws [] (List.nil_prefix) hchain hcpl]
  case totalOrder.atLeastOnce =>
    show _ = ((prefixCuts pool.norm 0
        (cutsLen 0 (ws.map destutter))).map StutterSeq.mk).map
      (PoolFold .totalOrder .atLeastOnce Pre g init ok)
    have hch : List.IsChain (· <+: ·)
        (([] : List α) :: ws.map destutter) := by
      rcases ws with _ | ⟨w, ws⟩
      · exact hchain
      · rw [List.map_cons]
        refine List.isChain_cons.mpr ⟨fun y hy => ?_, ?_⟩
        · cases hy; exact List.nil_prefix
        · rw [← List.map_cons]
          exact List.isChain_map_of_isChain destutter
            (fun _ _ h => destutter_prefix h)
            (List.isChain_cons.mp hchain).2
    rw [show (0 : Nat) = ([] : List α).length from rfl,
      prefixCuts_views (ws.map destutter) [] List.nil_prefix hch
        (by
          intro u hu
          obtain ⟨w, hw, rfl⟩ := List.mem_map.mp hu
          exact hcpl w hw)]
    rw [List.map_map, List.map_map]
    refine List.map_congr_left (fun w _hw => ?_)
    exact StutterSeq.foldl_destutter g ok w init
  case noOrder.exactlyOnce =>
    show _ = (snapshotCuts pool 0 (cutsMS 0 ws)).map
      (PoolFold .noOrder .exactlyOnce Pre g init ok)
    rw [show (0 : Multiset α) = (↑([] : List α) : Multiset α) from rfl,
      snapshotCuts_views ws [] hchain hcpl]
    exact (map_msfold_eq Pre g init ok ws
      (fun w hw => PoolPairwise.of_coe_le (hcpl w hw) hpre)).symm
  case noOrder.atLeastOnce =>
    show _ = ((snapshotMemCuts (RetryPool.support pool).val 0
        (cutsMS 0 ws)).map RetryPool.mk).map
      (PoolFold .noOrder .atLeastOnce Pre g init ok)
    rw [show (0 : Multiset α) = (↑([] : List α) : Multiset α) from rfl,
      snapshotMemCuts_views ws []
        hchain
        (by
          intro w hw x hx
          exact RetryPool.mem_support_val.mpr
            (hcpl w hw x (by
              rw [RetryPool.mem_mk, Multiset.mem_coe]; exact hx))),
    ]
    exact (map_rpfold_eq Pre g init ok ws
      (fun w hw => PoolPairwise.of_mk_le (hcpl w hw) hpre)).symm

/-! ## Small kit for the coupling instance -/

/-- The empty buffer is below every pool. -/
theorem ListLe.nil {α : Type} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (v : PoolCarrier α ord ret) :
    ListLe ord ret [] v := by
  cases ord <;> cases ret
  · exact List.nil_prefix
  · exact List.nil_prefix
  · exact Multiset.zero_le v
  · exact fun x hx => absurd (RetryPool.mem_mk x 0 |>.mp hx)
      (by simp)

theorem ListLe.of_prefix {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {s' s : List α} {v : PoolCarrier α ord ret}
    (hp : s' <+: s) (h : ListLe ord ret s v) : ListLe ord ret s' v := by
  cases ord <;> cases ret
  · exact hp.trans h
  · exact StutterSeq.le_trans
      (show StutterSeq.le (.mk s') (.mk s) from destutter_prefix hp) h
  · exact le_trans (Multiset.coe_le.mpr hp.sublist.subperm) h
  · exact RetryPool.le_trans
      (fun x hx => by
        rw [RetryPool.mem_mk, Multiset.mem_coe] at hx ⊢
        exact hp.sublist.mem hx) h

theorem ListLe.bot {α : Type} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) :
    ListLe ord ret ([] : List α) (PoolBot ord ret) := by
  cases ord <;> cases ret
  · exact List.nil_prefix
  · exact List.nil_prefix
  · exact le_refl _
  · exact fun x hx => by
      rw [RetryPool.mem_mk] at hx
      cases hx

/-- Membership survives destuttering. -/
theorem mem_destutter {α : Type _} [DecidableEq α] {x : α} :
    ∀ {l : List α}, x ∈ l → x ∈ destutter l
  | [], h => h
  | [_], h => h
  | y :: z :: rest, h => by
    by_cases hyz : y = z
    · subst hyz
      rw [destutter_cons_cons, if_pos rfl]
      rcases List.mem_cons.mp h with rfl | h
      · exact mem_destutter (List.mem_cons_self ..)
      · exact mem_destutter h
    · rw [destutter_cons_cons, if_neg hyz]
      rcases List.mem_cons.mp h with rfl | h
      · exact List.mem_cons_self ..
      · exact List.mem_cons_of_mem _ (mem_destutter h)

/-- A realizable selection realizes itself (drawing without
replacement from a dominating pool changes nothing). -/
theorem selectOrder_of_le {α : Type} [DecidableEq α] :
    ∀ (l : List α) (pool : Multiset α),
      (↑l : Multiset α) ≤ pool → selectOrder pool l = l
  | [], _, _ => rfl
  | x :: xs, pool, h => by
    have hx : x ∈ pool :=
      Multiset.mem_of_le h (by rw [Multiset.mem_coe]; exact List.mem_cons_self ..)
    have hxs : (↑xs : Multiset α) ≤ pool.erase x := by
      have h' : (x ::ₘ (↑xs : Multiset α)) ≤ pool := by
        rw [Multiset.cons_coe]; exact h
      have := Multiset.erase_le_erase x h'
      rwa [Multiset.erase_cons_head] at this
    show (if x ∈ pool then x :: selectOrder (pool.erase x) xs else []) = _
    rw [if_pos hx, selectOrder_of_le xs (pool.erase x) hxs]

/-- A prefix reconstitutes its extension by dropping its own length. -/
theorem prefix_append_drop {α : Type _} {l w : List α} (h : l <+: w) :
    l ++ w.drop l.length = w := by
  obtain ⟨e, rfl⟩ := h
  rw [List.drop_left]

theorem pairwise_isChain {α : Type _} {R : α → α → Prop} :
    ∀ {l : List α}, List.Pairwise R l → List.IsChain R l
  | [], _ => List.isChain_nil
  | [_], _ => by simp
  | a :: b :: rest, h => by
    refine List.isChain_cons.mpr ⟨fun y hy => ?_, ?_⟩
    · cases hy
      exact (List.pairwise_cons.mp h).1 b (List.mem_cons_self ..)
    · exact pairwise_isChain (List.pairwise_cons.mp h).2

theorem tickSteps_le {p : Nat → Bool} {t s : Nat}
    (h : s ∈ tickSteps p t) : s ≤ t := by
  have hm : s ∈ List.range (t + 1) := (List.mem_filter.mp h).1
  exact Nat.lt_succ_iff.mp (List.mem_range.mp hm)

theorem tickSteps_pairwise (p : Nat → Bool) (t : Nat) :
    List.Pairwise (· ≤ ·) (tickSteps p t) := by
  have hr : List.Pairwise (· ≤ ·) (List.range (t + 1)) :=
    (List.pairwise_lt_range).imp Nat.le_of_lt
  exact hr.sublist List.filter_sublist

/-- The tick-step reads of a live wire chain by prefix. -/
theorem viewsChain {α : Type} (h : StepHist α) {ss : List Nat}
    (hss : List.Pairwise (· ≤ ·) ss) :
    List.IsChain (· <+: ·) (([] : List α) :: ss.map h.view) := by
  refine List.isChain_cons.mpr ⟨fun y _hy => List.nil_prefix, ?_⟩
  refine pairwise_isChain (List.pairwise_map.mpr ?_)
  exact hss.imp (fun hab => h.mono_le hab)

/-- Pushing accumulated `RetryPool` unions: membership from any
summand or the accumulator. -/
theorem mem_foldl_union {α : Type _} {x : α} :
    ∀ (l : List (RetryPool α)) (acc : RetryPool α),
      ((∃ p ∈ l, x ∈ p) ∨ x ∈ acc) →
      x ∈ l.foldl RetryPool.union acc
  | [], acc, h => by
    rcases h with ⟨p, hp, _⟩ | h
    · cases hp
    · exact h
  | q :: rest, acc, h => by
    show x ∈ rest.foldl RetryPool.union (RetryPool.union acc q)
    refine mem_foldl_union rest _ ?_
    rcases h with ⟨p, hp, hx⟩ | h
    · rcases List.mem_cons.mp hp with rfl | hp
      · exact Or.inr (RetryPool.mem_union.mpr (Or.inr hx))
      · exact Or.inl ⟨p, hp, hx⟩
    · exact Or.inr (RetryPool.mem_union.mpr (Or.inl h))

/-- Pointwise-dominated multiset sums are dominated. -/
theorem sum_le_sum_pointwise {ι : Type _} {α : Type _}
    {f g : ι → Multiset α} :
    ∀ {js : List ι}, (∀ j ∈ js, f j ≤ g j) →
      (js.map f).sum ≤ (js.map g).sum
  | [], _ => le_refl _
  | j :: js, h => by
    rw [List.map_cons, List.map_cons, List.sum_cons, List.sum_cons]
    exact le_trans
      (Multiset.add_le_add_right (h j (List.mem_cons_self ..)))
      (Multiset.add_le_add_left
        (sum_le_sum_pointwise (fun u hu => h u (List.mem_cons_of_mem _ hu))))

/-! ## Realization: batches, slices, zips, emissions -/

/-- Count-legal unordered batching realizes the machine's tick segments
verbatim at the derived increments. -/
theorem batchCuts_real {α : Type} [DecidableEq α] {pool : Multiset α}
    (src : Nat → List α) :
    ∀ (ss : List Nat) (wprev : List α),
      List.IsChain (· <+: ·) (wprev :: ss.map src) →
      (∀ s ∈ ss, Multiset.ofList (src s) ≤ pool) →
      batchCuts pool (Multiset.ofList wprev)
        ((batchesFrom src ss wprev.length).map
          (fun b => Multiset.ofList b))
        = (batchesFrom src ss wprev.length).map
          (fun b => Multiset.ofList b)
  | [], _, _, _ => rfl
  | s :: ss, wprev, hchain, hpool => by
    have hpw : wprev <+: src s :=
      (List.isChain_cons.mp hchain).1 (src s) rfl
    have hofl : Multiset.ofList wprev
        + Multiset.ofList ((src s).drop wprev.length)
        = Multiset.ofList (src s) := by
      rw [Multiset.coe_add, prefix_append_drop hpw]
    have hle : Multiset.ofList wprev
        + Multiset.ofList ((src s).drop wprev.length) ≤ pool := by
      rw [hofl]; exact hpool s (List.mem_cons_self ..)
    show batchCuts pool (Multiset.ofList wprev)
      (Multiset.ofList ((src s).drop wprev.length)
        :: (batchesFrom src ss (src s).length).map
          (fun b => Multiset.ofList b)) = _
    unfold batchCuts
    rw [if_pos hle, hofl,
      batchCuts_real src ss (src s) (List.isChain_cons.mp hchain).2
        (fun u hu => hpool u (List.mem_cons_of_mem _ hu))]
    rfl

/-- Ordered batching realizes the machine's tick segments verbatim at
the derived slice sizes. -/
theorem sliceCuts_real {α : Type} {pool : List α} (src : Nat → List α) :
    ∀ (ss : List Nat) (wprev : List α),
      List.IsChain (· <+: ·) (wprev :: ss.map src) →
      (∀ s ∈ ss, src s <+: pool) →
      sliceCuts pool wprev.length
        ((batchesFrom src ss wprev.length).map List.length)
        = batchesFrom src ss wprev.length
  | [], _, _, _ => rfl
  | s :: ss, wprev, hchain, hpool => by
    have hpw : wprev <+: src s :=
      (List.isChain_cons.mp hchain).1 (src s) rfl
    have hsp : src s <+: pool := hpool s (List.mem_cons_self ..)
    have hlen : wprev.length + ((src s).drop wprev.length).length
        = (src s).length := by
      rw [List.length_drop]
      exact Nat.add_sub_cancel' hpw.length_le
    have hleg : wprev.length + ((src s).drop wprev.length).length
        ≤ pool.length := by
      rw [hlen]; exact hsp.length_le
    have hseg : (pool.drop wprev.length).take
        ((src s).drop wprev.length).length
        = (src s).drop wprev.length := by
      conv_rhs => rw [List.prefix_iff_eq_take.mp hsp]
      rw [List.length_drop, List.drop_take]
    show sliceCuts pool wprev.length
      (((src s).drop wprev.length).length
        :: (batchesFrom src ss (src s).length).map List.length) = _
    unfold sliceCuts
    rw [if_pos hleg, hseg, hlen,
      sliceCuts_real src ss (src s) (List.isChain_cons.mp hchain).2
        (fun u hu => hpool u (List.mem_cons_of_mem _ hu))]
    rfl

/-- Ordered at-least-once batching (D91) realizes the machine's tick segments, each as its
class: the concatenation of the segments so far is the view at the tick, whose
destutter is a prefix of the pool's normal form. -/
theorem stutterBatchCuts_real {α : Type} [DecidableEq α] {pool : StutterSeq α}
    (src : Nat → List α) :
    ∀ (ss : List Nat) (wprev : List α),
      List.IsChain (· <+: ·) (wprev :: ss.map src) →
      (∀ s ∈ ss, destutter (src s) <+: pool.norm) →
      stutterBatchCuts pool wprev (batchesFrom src ss wprev.length)
        = (batchesFrom src ss wprev.length).map StutterSeq.mk
  | [], _, _, _ => rfl
  | s :: ss, wprev, hchain, hpool => by
    have hpw : wprev <+: src s :=
      (List.isChain_cons.mp hchain).1 (src s) rfl
    have hcat : wprev ++ (src s).drop wprev.length = src s := prefix_append_drop hpw
    show stutterBatchCuts pool wprev
      ((src s).drop wprev.length :: batchesFrom src ss (src s).length) = _
    unfold stutterBatchCuts
    rw [hcat, if_pos (hpool s (List.mem_cons_self ..)),
      stutterBatchCuts_real src ss (src s) (List.isChain_cons.mp hchain).2
        (fun u hu => hpool u (List.mem_cons_of_mem _ hu))]
    rfl

/-- Zipping against a left entry translation. -/
theorem zip_map_left' {α β γ : Type _} (f : α → γ) :
    ∀ (l : List α) (r : List β),
      Trace.zip (l.map f) r = (Trace.zip l r).map (fun p => (f p.1, p.2))
  | [], _ => rfl
  | _ :: _, [] => rfl
  | x :: xs, y :: ys => by
    exact congrArg (List.cons (f x, y)) (zip_map_left' f xs ys)

/-- Zipping against a right entry translation. -/
theorem zip_map_right' {α β γ : Type _} (f : β → γ) :
    ∀ (l : List α) (r : List β),
      Trace.zip l (r.map f) = (Trace.zip l r).map (fun p => (p.1, f p.2))
  | [], _ => rfl
  | _ :: _, [] => rfl
  | x :: xs, y :: ys => by
    exact congrArg (List.cons (x, f y)) (zip_map_right' f xs ys)

/-- Zipping a dominating trace against the machine trace it dominates
pairs each machine entry with its own image. -/
theorem zip_of_map_prefix {α β : Type _} (f : α → β) :
    ∀ (raw : List α) (v : List β), raw.map f <+: v →
      Trace.zip v raw = raw.map (fun b => (f b, b))
  | [], v, _ => by
    show List.zip v [] = []
    exact List.zip_nil_right
  | b :: raw, v, h => by
    obtain ⟨e, he⟩ := h
    rw [← he, List.map_cons, List.cons_append, List.map_cons]
    exact congrArg (List.cons (f b, b))
      (zip_of_map_prefix f raw _ ⟨e, rfl⟩)

private theorem ms_sum_append {α : Type} :
    ∀ (l e : List (Multiset α)), (l ++ e).sum = l.sum + e.sum
  | [], e => by rw [List.nil_append, List.sum_nil, Multiset.zero_add]
  | m :: l, e => by
    rw [List.cons_append, List.sum_cons, List.sum_cons,
      ms_sum_append l e, Multiset.add_assoc]

theorem coe_flatten {α : Type} :
    ∀ (bs : List (List α)),
      (Multiset.ofList bs.flatten)
        = (bs.map (fun b => Multiset.ofList b)).sum
  | [] => rfl
  | b :: bs => by
    rw [List.flatten_cons, List.map_cons, List.sum_cons,
      ← Multiset.coe_add, coe_flatten bs]

/-- A machine's concatenated ticks are dominated by the denotational
tick sum. -/
theorem flatten_le_sum {α : Type} {bs : List (List α)}
    {v : List (Multiset α)}
    (h : bs.map (fun b => Multiset.ofList b) <+: v) :
    Multiset.ofList bs.flatten ≤ v.sum := by
  obtain ⟨e, rfl⟩ := h
  rw [coe_flatten, ms_sum_append]
  exact Multiset.le_iff_exists_add.mpr ⟨e.sum, rfl⟩

/-! ## The per-leg coupling relations

The leaf relations the coupling corner (`Couple.lean`) states between a
machine leg (as of a step) and a denotation leg: batch traces are
prefixes with equal entries, singleton reads are uniform across the
bound. -/

/-- Tick-batch coupling: realized batch traces are prefixes with equal
entries (`ExactlyOnce`; ordered entries verbatim, unordered entries up
to the list→multiset quotient). No operator constructs an `AtLeastOnce`
tick stream, so that coupling is trivially `True`. -/
@[reducible] def BatchTraceLe {α : Type} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) →
      Trace (List α) → Trace (PoolCarrier α ord ret) → Prop
  | .totalOrder, .exactlyOnce => fun s v => s <+: v
  | .noOrder, .exactlyOnce => fun s v =>
      s.map (fun b => Multiset.ofList b) <+: v
  -- the retry grades (D89): the machine's batches, each as its entitlement class, prefix the
  -- denotation's — a batch that re-delivers (a trailing stutter at `TotalOrder`, a duplicate
  -- out of order at `NoOrder`) is the same class; a batch outside the class is not
  | .totalOrder, .atLeastOnce => fun s v => s.map (fun b => StutterSeq.mk b) <+: v
  | .noOrder, .atLeastOnce => fun s v => s.map (fun b : List α => RetryPool.mk (Multiset.ofList b)) <+: v

/-- Read a `SingletonV` uniformly across the bound. -/
@[reducible] def singReads {n : Nat} {α σ : Type} [DecidableEq α]
    {ord : StrOrd} :
    (b : SingBound σ) → SingletonV n α σ ord b →
      Fin n → CutDec α ord → Trace σ
  | .unbounded => fun v => v
  | .monotonic _ => fun v i => (v i).val

theorem mem_merge2View {α : Type} [DecidableEq α] {a b : StepHist α}
    {t : Nat} {x : α} (hx : x ∈ merge2View a b t) :
    x ∈ a.view t ∨ x ∈ b.view t := by
  have hm : x ∈ (Multiset.ofList (merge2View a b t)) :=
    Multiset.mem_coe.mpr hx
  rw [coe_merge2View] at hm
  rcases Multiset.mem_add.mp hm with h | h
  · exact Or.inl (Multiset.mem_coe.mp h)
  · exact Or.inr (Multiset.mem_coe.mp h)

theorem mem_mergeNView {m : Nat} {α : Type} [DecidableEq α]
    {k : Fin m → StepHist α} {t : Nat} {x : α}
    (hx : x ∈ mergeNView k t) : ∃ j, x ∈ (k j).view t := by
  have hm : x ∈ (Multiset.ofList (mergeNView k t)) :=
    Multiset.mem_coe.mpr hx
  rw [coe_mergeNView] at hm
  obtain ⟨mm, hmm, hxm⟩ := mem_list_sum.mp hm
  obtain ⟨j, _, rfl⟩ := List.mem_map.mp hmm
  exact ⟨j, Multiset.mem_coe.mp hxm⟩

/-! ## Per-op coupling lemmas (the corner's `cpl` content per operator) -/

section CorrOps

variable {L : Type} {mem : L → Nat}

/-! ### Keyed wires: the machine's flat pairs against a keyed pool -/

/-- A machine wire of pairs against a keyed pool: per key, the arrived
values sit below the key's content in the grade's relaxation. -/
@[reducible] def KeyedListLe {K V : Type} [DecidableEq K] [DecidableEq V]
    (ord : StrOrd) (ret : Retries) (l : List (K × V))
    (p : KeyedPool K V ord ret) : Prop :=
  ∀ k, ListLe ord ret (keyVals k l) (p.grp k)

theorem KeyedListLe.of_prefix {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} {l l' : List (K × V)} {p : KeyedPool K V ord ret}
    (hp : l <+: l') (h : KeyedListLe ord ret l' p) : KeyedListLe ord ret l p :=
  fun k => ListLe.of_prefix (keyVals_prefix hp) (h k)

theorem KeyedListLe.trans_pool {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} {l : List (K × V)} {p p' : KeyedPool K V ord ret}
    (h : KeyedListLe ord ret l p) (hpp : KeyedPool.Le p p') : KeyedListLe ord ret l p' :=
  fun k => ListLe.trans_pool (h k) (hpp k)

/-- A key with arrivals is a key of the pool (its content is not the
bottom). -/
theorem KeyedListLe.mem_keys {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} {l : List (K × V)} {p : KeyedPool K V ord ret}
    (h : KeyedListLe ord ret l p) {k : K} (hk : k ∈ l.map Prod.fst) : k ∈ p.keys := by
  by_contra hkp
  have hbot := p.dom k hkp
  obtain ⟨e, he, hek⟩ := List.mem_map.mp hk
  have hx : e.2 ∈ keyVals k l := by
    unfold keyVals
    exact List.mem_map.mpr ⟨e, List.mem_filter.mpr ⟨he, by simp [hek]⟩, rfl⟩
  have hl := h k
  rw [hbot] at hl
  cases ord <;> cases ret
  · rw [List.prefix_nil.mp hl] at hx
    exact List.not_mem_nil hx
  · have hsub : destutter (keyVals k l) <+: destutter [] := hl
    rw [destutter_nil] at hsub
    have := mem_destutter hx
    rw [List.prefix_nil.mp hsub] at this
    exact List.not_mem_nil this
  · have : (↑(keyVals k l) : Multiset V) = 0 := Multiset.le_zero.mp hl
    have hx' : e.2 ∈ (↑(keyVals k l) : Multiset V) := Multiset.mem_coe.mpr hx
    rw [this] at hx'
    exact Multiset.notMem_zero _ hx'
  · have h' := hl e.2 (by rw [RetryPool.mem_mk, Multiset.mem_coe]; exact hx)
    have h'' : e.2 ∈ RetryPool.mk (0 : Multiset V) := h'
    rw [RetryPool.mem_mk] at h''
    exact Multiset.notMem_zero _ h''

/-- A list of pairs, imaged, is the sum over any key set covering its
keys of its per-key value lists, imaged. -/
theorem coe_map_eq_sum_keyVals {K V β : Type} [DecidableEq K]
    (F : K → V → β) :
    ∀ (S : Finset K) (l : List (K × V)), (∀ e ∈ l, e.1 ∈ S) →
      (↑(l.map (fun e => F e.1 e.2)) : Multiset β)
        = (S.val.map (fun k => (↑(keyVals k l) : Multiset V).map (F k))).sum := by
  intro S
  induction S using Finset.induction_on with
  | empty =>
    intro l hl
    cases l with
    | nil => rfl
    | cons e _ => exact absurd (hl e (List.mem_cons_self ..)) (Finset.notMem_empty _)
  | insert k S hk ih =>
    intro l hl
    rw [Finset.insert_val_of_notMem hk, Multiset.map_cons, Multiset.sum_cons]
    have hpart : (↑(l.map (fun e => F e.1 e.2)) : Multiset β)
        = ↑((l.filter (fun e => decide (e.1 = k))).map (fun e => F e.1 e.2))
          + ↑((l.filter (fun e => !decide (e.1 = k))).map (fun e => F e.1 e.2)) := by
      rw [Multiset.coe_add, ← List.map_append, Multiset.coe_eq_coe]
      exact ((List.filter_append_perm _ l).symm.map _)
    rw [hpart]
    congr 1
    · unfold keyVals
      rw [Multiset.map_coe, List.map_map, Multiset.coe_eq_coe]
      refine List.Perm.of_eq (List.map_congr_left fun e he => ?_)
      have := of_decide_eq_true (List.mem_filter.mp he).2
      show F e.1 e.2 = F k e.2
      rw [this]
    · rw [ih (l.filter (fun e => !decide (e.1 = k))) (fun e he => by
        have hm := List.mem_filter.mp he
        have := hl e hm.1
        rw [Finset.mem_insert] at this
        rcases this with h | h
        · exact absurd h (by simpa using hm.2)
        · exact h)]
      refine congrArg Multiset.sum (Multiset.map_congr rfl fun k' hk' => ?_)
      have hne : k' ≠ k := fun h => hk (h ▸ hk')
      congr 2
      unfold keyVals
      rw [List.filter_filter]
      refine congrArg _ (List.filter_congr fun e _ => ?_)
      by_cases h : e.1 = k'
      · simp [h, hne]
      · simp [h]

/-- Grouping unordered pairs and flattening again is the identity (the
keyed `assume_ordering` selects from exactly the pool's pairs). -/
theorem KeyedPool.toMultiset_ofMultiset {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : Multiset (K × V)) : (KeyedPool.ofMultiset m).toMultiset = m := by
  induction m using Quotient.inductionOn with
  | h l =>
    change (KeyedPool.ofMultiset (Multiset.ofList l)).toMultiset = Multiset.ofList l
    have h := coe_map_eq_sum_keyVals (fun k v => ((k, v) : K × V))
      ((l.map Prod.fst).toFinset) l (fun e he => List.mem_toFinset.mpr (List.mem_map.mpr ⟨e, he, rfl⟩))
    simp only [Prod.mk.eta, List.map_id'] at h
    unfold KeyedPool.toMultiset KeyedPool.ofMultiset
    dsimp only
    rw [Multiset.map_coe]
    conv_rhs => rw [h]
    refine congrArg Multiset.sum (Multiset.map_congr rfl fun k _ => ?_)
    rw [Multiset.filter_coe, Multiset.map_coe]
    rfl

/-- The arrived pairs, as a multiset, sit inside the pool's flat content. -/
theorem KeyedListLe.coe_le_toMultiset {K V : Type} [DecidableEq K] [DecidableEq V]
    {l : List (K × V)} {p : KeyedPool K V .noOrder .exactlyOnce}
    (h : KeyedListLe .noOrder .exactlyOnce l p) : (↑l : Multiset (K × V)) ≤ p.toMultiset := by
  have hl : l = l.map (fun e => ((e.1, e.2) : K × V)) := by simp
  rw [hl, coe_map_eq_sum_keyVals (fun k v => (k, v)) p.keys l
    (fun e he => h.mem_keys (List.mem_map.mpr ⟨e, he, rfl⟩))]
  unfold KeyedPool.toMultiset
  exact Multiset.sum_map_le_sum_map _ _ (fun k _ => Multiset.map_le_map (h k))

/-- `.values()` stays coupled at every grade: the flat machine wire's
values are below the merged denotational content. -/
theorem corr_values {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} {l : List (K × V)} {p : KeyedPool K V ord ret}
    (h : KeyedListLe ord ret l p) :
    ListLe .noOrder ret (l.map Prod.snd) p.values := by
  have hkeys : ∀ e ∈ l, e.1 ∈ p.keys := fun e he => h.mem_keys (List.mem_map.mpr ⟨e, he, rfl⟩)
  cases ret
  · show (↑(l.map Prod.snd) : Multiset V) ≤ p.sumMS
    rw [show l.map Prod.snd = l.map (fun e => (fun _ v => v) e.1 e.2) from rfl,
      coe_map_eq_sum_keyVals (fun _ v => v) p.keys l hkeys]
    unfold KeyedPool.sumMS
    refine Multiset.sum_map_le_sum_map _ _ (fun k _ => ?_)
    rw [Multiset.map_id']
    cases ord
    · exact Multiset.coe_le.mpr (h k).sublist.subperm
    · exact h k
  · intro x hx
    rw [RetryPool.mem_mk, Multiset.mem_coe] at hx
    obtain ⟨e, he, rfl⟩ := List.mem_map.mp hx
    show e.2 ∈ RetryPool.mk p.sumMS
    rw [RetryPool.mem_mk]
    unfold KeyedPool.sumMS
    refine Multiset.mem_sum.mpr ⟨e.1, Finset.mem_val.mpr (hkeys e he), ?_⟩
    have hv : e.2 ∈ keyVals e.1 l := by
      unfold keyVals
      exact List.mem_map.mpr ⟨e, List.mem_filter.mpr ⟨he, by simp⟩, rfl⟩
    have hl := h e.1
    cases ord
    · show e.2 ∈ (↑(p.grp e.1).norm : Multiset V)
      rw [Multiset.mem_coe]
      exact (show StutterSeq.le (.mk (keyVals e.1 l)) (p.grp e.1) from hl).subset
        (mem_destutter hv)
    · show e.2 ∈ (RetryPool.support (p.grp e.1)).val
      exact RetryPool.mem_support_val.mpr
        (hl e.2 (by rw [RetryPool.mem_mk, Multiset.mem_coe]; exact hv))

/-- `into_keyed` stays coupled: grouping a coupled wire gives per-key
coupling. -/
theorem corr_into_keyed {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {l : List (K × V)} {v : PoolCarrier (K × V) ord .exactlyOnce}
    (h : ListLe ord .exactlyOnce l v) :
    KeyedListLe ord .exactlyOnce l (KeyedPool.ofPool v) := by
  intro k
  cases ord
  · exact keyVals_prefix h
  · show (↑(keyVals k l) : Multiset V) ≤ (v.filter (fun e => e.1 = k)).map Prod.snd
    unfold keyVals
    rw [← Multiset.map_coe, ← Multiset.filter_coe]
    exact Multiset.map_le_map (Multiset.filter_le_filter _ h)

/-- The key-`j` values of sender-tagged lists concatenated over a
duplicate-free sender list: sender `j`'s list (if present). -/
theorem keyVals_flatMap_tag {J V : Type} [DecidableEq J] (g : J → List V) (j : J) :
    ∀ (js : List J), js.Nodup →
      keyVals j (js.flatMap (fun j' => (g j').map (fun v => (j', v))))
        = if j ∈ js then g j else []
  | [], _ => by simp [keyVals]
  | j' :: js, hnd => by
    rw [List.flatMap_cons, keyVals_append, keyVals_flatMap_tag g j js (List.nodup_cons.mp hnd).2]
    have hhead : keyVals j ((g j').map (fun v => (j', v))) = if j' = j then g j else [] := by
      unfold keyVals
      by_cases h : j' = j
      · subst h
        simp [List.filter_map, Function.comp_def]
      · simp [h]
    rw [hhead]
    by_cases h : j' = j
    · subst h
      rw [if_pos rfl, if_pos (List.mem_cons_self ..), if_neg (List.nodup_cons.mp hnd).1,
        List.append_nil]
    · rw [if_neg h, List.nil_append]
      by_cases hm : j ∈ js
      · rw [if_pos hm, if_pos (List.mem_cons_of_mem _ hm)]
      · rw [if_neg hm, if_neg (fun h' => by
          rcases List.mem_cons.mp h' with h' | h'
          · exact h h'.symm
          · exact hm h')]

theorem StepHist.inc_map {α β : Type} (f : α → β) (h : StepHist α) (t : Nat) :
    (h.map f).inc t = (h.inc t).map f := by
  show ((h.view (t + 1)).map f).drop ((h.view t).map f).length = _
  rw [List.length_map, ← List.map_drop]
  rfl

/-- The tagged sender wires merged: key `j`'s values are sender `j`'s
wire. -/
theorem keyVals_mergeN_tag {m : Nat} {V : Type} (h : Fin m → StepHist V) (j : Fin m) :
    ∀ t, keyVals j (mergeNView (fun j => (h j).map (fun v => (j, v))) t) = (h j).view t
  | 0 => by
    show keyVals j ((List.finRange m).flatMap (fun j' => ((h j').view 0).map (fun v => (j', v))))
      = (h j).view 0
    rw [keyVals_flatMap_tag (fun j' => (h j').view 0) j _ (List.nodup_finRange m),
      if_pos (List.mem_finRange j)]
  | t + 1 => by
    show keyVals j (mergeNView (fun j => (h j).map (fun v => (j, v))) t
        ++ (List.finRange m).flatMap (fun j' => ((h j').map (fun v => (j', v))).inc t))
      = (h j).view (t + 1)
    rw [keyVals_append, keyVals_mergeN_tag h j t, (h j).view_succ]
    congr 1
    have : (fun j' : Fin m => ((h j').map (fun v => (j', v))).inc t)
        = fun j' => ((h j').inc t).map (fun v => (j', v)) :=
      funext fun j' => StepHist.inc_map _ _ _
    rw [this, keyVals_flatMap_tag (fun j' => (h j').inc t) j _ (List.nodup_finRange m),
      if_pos (List.mem_finRange j)]

/-- The network edge stays coupled, per sender key. -/
theorem corr_transport {m : Nat} {V : Type} [DecidableEq V] {ord : StrOrd} {ret : Retries}
    {h : Fin m → StepHist V} {v : Fin m → PoolCarrier V ord ret} {t : Nat}
    (hc : ∀ j, ListLe ord ret ((h j).view t) (v j)) :
    KeyedListLe ord ret (mergeNView (fun j => (h j).map (fun v => (j, v))) t)
      (KeyedPool.transport v) := by
  intro j
  rw [keyVals_mergeN_tag h j t]
  exact hc j

/-- Legal views along an ascending view chain are realized exactly at
the derived increment cuts (the keyed `snapshotCuts_views`). -/
theorem kviews_cutsMS {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (p : KeyedPool K V ord .exactlyOnce) :
    ∀ (ws : List (List (K × V))) (prev : List (K × V)),
      List.IsChain (· <+: ·) (prev :: ws) → (∀ w ∈ ws, p.legalView (↑w) = true) →
      p.kviews (Multiset.ofList prev) (cutsMS (Multiset.ofList prev) ws)
        = ws.map (fun w => Multiset.ofList w)
  | [], _, _, _ => rfl
  | w :: ws, prev, hchain, hlegal => by
    have hpw : prev <+: w := (List.isChain_cons.mp hchain).1 w rfl
    have hadd : Multiset.ofList prev + (Multiset.ofList w - Multiset.ofList prev) = Multiset.ofList w :=
      add_tsub_cancel_of_le (Multiset.coe_le.mpr hpw.sublist.subperm)
    show p.kviews (Multiset.ofList prev)
        ((Multiset.ofList w - Multiset.ofList prev) :: cutsMS (Multiset.ofList w) ws)
      = Multiset.ofList w :: ws.map (fun w => Multiset.ofList w)
    unfold KeyedPool.kviews
    rw [hadd, if_pos (hlegal w (List.mem_cons_self ..))]
    exact congrArg _ (kviews_cutsMS p ws w (List.isChain_cons.mp hchain).2
      (fun w' hw' => hlegal w' (List.mem_cons_of_mem _ hw')))

/-- `union` fan-in stays coupled at both retry grades. -/
theorem corr_union {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries}
    {a b : Fin (mem ℓ) → StepHist α}
    {va vb : MemberMap (mem ℓ) (PoolCarrier α .noOrder ret)} {t : Nat}
    (ha : ∀ i, ListLe .noOrder ret ((a i).view t) (va i))
    (hb : ∀ i, ListLe .noOrder ret ((b i).view t) (vb i)) :
    ∀ i, ListLe .noOrder ret (merge2View (a i) (b i) t)
      ((Values L mem).union (ℓ := ℓ) va vb i) := by
  intro i
  cases ret
  case exactlyOnce =>
    show Multiset.ofList (merge2View (a i) (b i) t) ≤ va i + vb i
    rw [coe_merge2View]
    exact le_trans (Multiset.add_le_add_right (ha i))
      (Multiset.add_le_add_left (hb i))
  case atLeastOnce =>
    intro x hx
    rw [RetryPool.mem_mk, Multiset.mem_coe] at hx
    rcases mem_merge2View hx with h | h
    · exact RetryPool.mem_union.mpr (Or.inl (ha i x
        (by rw [RetryPool.mem_mk, Multiset.mem_coe]; exact h)))
    · exact RetryPool.mem_union.mpr (Or.inr (hb i x
        (by rw [RetryPool.mem_mk, Multiset.mem_coe]; exact h)))

/-- `weaken_retries` stays coupled: the machine's wire is unchanged, the
pool coarsens into the retry quotient. -/
theorem corr_weaken {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {h : Fin (mem ℓ) → StepHist α}
    {v : MemberMap (mem ℓ) (PoolCarrier α ord .exactlyOnce)} {t : Nat}
    (hc : ∀ i, ListLe ord .exactlyOnce ((h i).view t) (v i)) :
    ∀ i, ListLe ord .atLeastOnce ((h i).view t)
      ((Values L mem).weaken_retries (ℓ := ℓ) v i) := by
  intro i
  cases ord
  · exact destutter_prefix (hc i)
  · intro x hx
    have hx' : x ∈ (Multiset.ofList ((h i).view t)) := by
      rwa [RetryPool.mem_mk] at hx
    have hgoal : x ∈ RetryPool.mk (v i) := by
      rw [RetryPool.mem_mk]
      exact Multiset.mem_of_le (hc i) hx'
    exact hgoal

end CorrOps

section CorrOps2

variable {L : Type} {mem : L → Nat}

/-- Unordered batching: the machine's tick segments equal a denotational
batch run at the derived increments. -/
theorem corr_batch {α : Type} [DecidableEq α] {n : Nat}
    (p : Fin n → Nat → Bool) (T : Nat) (h : Fin n → StepHist α)
    (v : Fin n → Multiset α)
    (hc : ∀ i, Multiset.ofList ((h i).view T) ≤ v i) :
    ∀ i, batchCuts (v i) 0
        ((batchesFrom ((h i).view) (tickSteps (p i) T) 0).map
          (fun b => Multiset.ofList b))
      = (batchesFrom ((h i).view) (tickSteps (p i) T) 0).map
          (fun b => Multiset.ofList b) := by
  intro i
  have := batchCuts_real (pool := v i) ((h i).view) (tickSteps (p i) T)
    ([] : List α)
    (viewsChain (h i) (tickSteps_pairwise (p i) T))
    (fun s hs => le_trans
      (Multiset.coe_le.mpr ((h i).mono_le (tickSteps_le hs)).sublist.subperm)
      (hc i))
  exact this

/-- Ordered batching: the machine's tick segments equal a denotational
slice run at the derived sizes. -/
theorem corr_batch_ordered {α : Type} [DecidableEq α] {n : Nat}
    (p : Fin n → Nat → Bool) (T : Nat) (h : Fin n → StepHist α)
    (v : Fin n → List α)
    (hc : ∀ i, (h i).view T <+: v i) :
    ∀ i, sliceCuts (v i) 0
        ((batchesFrom ((h i).view) (tickSteps (p i) T) 0).map List.length)
      = batchesFrom ((h i).view) (tickSteps (p i) T) 0 := by
  intro i
  exact sliceCuts_real (pool := v i) ((h i).view) (tickSteps (p i) T)
    ([] : List α)
    (viewsChain (h i) (tickSteps_pairwise (p i) T))
    (fun s hs => ((h i).mono_le (tickSteps_le hs)).trans (hc i))

/-- Ordered at-least-once batching (D91): the machine's tick segments are admitted, each
as its class, when every tick's view destutters to a prefix of the pool's normal form
(`StutterSeq.le` of the view's class below the pool). -/
theorem corr_batch_stutter {α : Type} [DecidableEq α] {n : Nat}
    (p : Fin n → Nat → Bool) (T : Nat) (h : Fin n → StepHist α)
    (v : Fin n → StutterSeq α)
    (hc : ∀ i, StutterSeq.le (StutterSeq.mk ((h i).view T)) (v i)) :
    ∀ i, stutterBatchCuts (v i) []
        (batchesFrom ((h i).view) (tickSteps (p i) T) 0)
      = (batchesFrom ((h i).view) (tickSteps (p i) T) 0).map StutterSeq.mk := by
  intro i
  exact stutterBatchCuts_real (pool := v i) ((h i).view) (tickSteps (p i) T)
    ([] : List α)
    (viewsChain (h i) (tickSteps_pairwise (p i) T))
    (fun s hs => (destutter_prefix ((h i).mono_le (tickSteps_le hs))).trans (hc i))

end CorrOps2

/-! ## List→pool embeddings and the shaped leaf order -/

/-- `poolOfList` at every retry grade (the retry pools' canonical
embeddings of a list). -/
@[reducible] def poolOfListR {β : Type} [DecidableEq β] :
    (ord : StrOrd) → (ret : Retries) → List β → PoolCarrier β ord ret
  | .totalOrder, .exactlyOnce => fun l => l
  | .totalOrder, .atLeastOnce => fun l => StutterSeq.mk l
  | .noOrder, .exactlyOnce => fun l => (↑l : Multiset β)
  | .noOrder, .atLeastOnce => fun l => RetryPool.mk (↑l : Multiset β)

/-! ### The leafwise machine-vs-denotation order over a tick shape (what
the corner's `cpl` states for every shaped `tick_scan` leg) -/
namespace CorrTick
variable {L : Type} {mem : L → Nat}

/-- Leafwise `BatchTraceLe`/prefix between a machine-shaped value as of
step `k` and a denotation-shaped value. -/
def Le {ℓ : L} : (sh : TickShape) →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh → Nat →
    AtomicOf (ValuesTick.Tk mem) (ValuesTick.TS mem) (ValuesTick.TKS mem) (ValuesTick.TKStr mem) ℓ sh → Prop
  | .sing _, m, k, v => ∀ i, m i k <+: v i
  | .stream _ _ ord ret, m, k, v => ∀ i, BatchTraceLe ord ret (m i k) (v i)
  | .ksing _ _ _ _, m, k, v => ∀ i, (m i k).map KeyedMap.ofAssoc <+: v i
  | .kstream _ _ _ _ ord ret, m, k, v => ∀ i, KBatchTraceLe ord ret (m i k) (v i)
  | .pair a b, m, k, v => Le a m.1 k v.1 ∧ Le b m.2 k v.2

end CorrTick

/-! ## Decision-extension orders and read-extension lemmas -/

/-- Extension order on cut decisions (more ticks realized). -/
@[reducible] def CutLe {α : Type} : (ord : StrOrd) →
    CutDec α ord → CutDec α ord → Prop
  | .totalOrder => fun d d' => d <+: d'
  | .noOrder => fun d d' => d <+: d'

/-- `prefixCuts` extends under decision extension. -/
theorem prefixCuts_mono_dec {α : Type} {pool : List α} {acc : Nat}
    {d d' : List Nat} (h : d <+: d') :
    prefixCuts pool acc d <+: prefixCuts pool acc d' := by
  obtain ⟨e, rfl⟩ := h
  induction d generalizing acc with
  | nil => exact List.nil_prefix
  | cons n ds ih =>
    rw [List.cons_append]
    unfold prefixCuts
    by_cases hc : acc + n ≤ pool.length
    · rw [if_pos hc, if_pos hc]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih⟩
    · rw [if_neg hc, if_neg hc]

/-- `snapshotCuts` extends under decision extension. -/
theorem snapshotCuts_mono_dec {α : Type} [DecidableEq α]
    {pool : Multiset α} {acc : Multiset α}
    {d d' : List (Multiset α)} (h : d <+: d') :
    snapshotCuts pool acc d <+: snapshotCuts pool acc d' := by
  obtain ⟨e, rfl⟩ := h
  induction d generalizing acc with
  | nil => exact List.nil_prefix
  | cons b ds ih =>
    rw [List.cons_append]
    unfold snapshotCuts
    by_cases hc : acc + b ≤ pool
    · rw [if_pos hc, if_pos hc]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih⟩
    · rw [if_neg hc, if_neg hc]

/-- `snapshotMemCuts` extends under decision extension. -/
theorem snapshotMemCuts_mono_dec {α : Type} [DecidableEq α]
    {pool : Multiset α} {acc : Multiset α}
    {d d' : List (Multiset α)} (h : d <+: d') :
    snapshotMemCuts pool acc d <+: snapshotMemCuts pool acc d' := by
  obtain ⟨e, rfl⟩ := h
  induction d generalizing acc with
  | nil => exact List.nil_prefix
  | cons b ds ih =>
    rw [List.cons_append]
    unfold snapshotMemCuts
    by_cases hc : ∀ x ∈ b, x ∈ pool
    · rw [if_pos hc, if_pos hc]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih⟩
    · rw [if_neg hc, if_neg hc]

/-- `batchCuts` extends under decision extension. -/
theorem batchCuts_mono_dec {α : Type} [DecidableEq α]
    {pool : Multiset α} {c : Multiset α}
    {d d' : List (Multiset α)} (h : d <+: d') :
    batchCuts pool c d <+: batchCuts pool c d' := by
  obtain ⟨e, rfl⟩ := h
  induction d generalizing c with
  | nil => exact List.nil_prefix
  | cons b ds ih =>
    rw [List.cons_append]
    unfold batchCuts
    by_cases hc : c + b ≤ pool
    · rw [if_pos hc, if_pos hc]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih⟩
    · rw [if_neg hc, if_neg hc]

/-- `sliceCuts` extends under decision extension. -/
theorem sliceCuts_mono_dec {α : Type} {pool : List α} {c : Nat}
    {d d' : List Nat} (h : d <+: d') :
    sliceCuts pool c d <+: sliceCuts pool c d' := by
  obtain ⟨e, rfl⟩ := h
  induction d generalizing c with
  | nil => exact List.nil_prefix
  | cons n ds ih =>
    rw [List.cons_append]
    unfold sliceCuts
    by_cases hc : c + n ≤ pool.length
    · rw [if_pos hc, if_pos hc]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih⟩
    · rw [if_neg hc, if_neg hc]

/-- `snapTrace` extends under decision extension. -/
theorem snapTrace_mono_dec {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (Pre : α → α → Prop) [DecidableRel Pre] (g : σ → α → σ) (init : σ)
    (ok : FoldOkP ord ret Pre g)
    (pool : PoolCarrier α ord ret) {d d' : CutDec α ord}
    (h : CutLe ord d d') :
    snapTrace ord ret Pre g init ok pool d
      <+: snapTrace ord ret Pre g init ok pool d' := by
  refine List.IsPrefix.map _ ?_
  cases ord <;> cases ret
  · exact prefixCuts_mono_dec h
  · exact (prefixCuts_mono_dec h).map _
  · exact snapshotCuts_mono_dec h
  · exact (snapshotMemCuts_mono_dec h).map _

/-! ## Named derived decisions and their horizon monotonicity -/

/-- The end length of a batch walk. -/
def batchesEnd {α : Type} (src : Nat → List α) :
    List Nat → Nat → Nat
  | [], c => c
  | s :: rest, _ => batchesEnd src rest (src s).length

theorem sqBatchesFrom_append {α : Type} (src : Nat → List α) :
    ∀ (ss ss' : List Nat) (c : Nat),
      batchesFrom src (ss ++ ss') c
        = batchesFrom src ss c ++ batchesFrom src ss' (batchesEnd src ss c)
  | [], _, _ => rfl
  | s :: rest, ss', c => by
    show (src s).drop c :: batchesFrom src (rest ++ ss') (src s).length = _
    rw [sqBatchesFrom_append src rest ss' (src s).length]
    rfl

/-- Derived batch cuts (unordered): the machine's tick segments, each
member at its own skeleton. -/
def batchDerive {n : Nat} {α : Type} [DecidableEq α]
    (p : Fin n → Nat → Bool)
    (T : Nat) (s : Fin n → StepHist α) : BatchCuts n α :=
  fun i => (batchesFrom ((s i).view) (tickSteps (p i) T) 0).map
    (fun b => Multiset.ofList b)

/-- Derived batch cuts (unordered at-least-once, D91): the machine's tick segments as
sets. -/
def batchRetryDerive {n : Nat} {α : Type} [DecidableEq α]
    (p : Fin n → Nat → Bool)
    (T : Nat) (s : Fin n → StepHist α) : RetryBatchCuts n α :=
  fun i => (batchesFrom ((s i).view) (tickSteps (p i) T) 0).map
    (fun b => (Multiset.ofList b).toFinset)

/-- Derived batch cuts (ordered): the machine's tick segment sizes. -/
def batchOrdDerive {n : Nat} {α : Type} (p : Fin n → Nat → Bool) (T : Nat)
    (s : Fin n → StepHist α) : OrderedBatchCuts n :=
  fun i => (batchesFrom ((s i).view) (tickSteps (p i) T) 0).map List.length

/-- Derived batch cuts (ordered at-least-once, D91): the machine's tick segments
themselves — each tick's realized batch, duplicates and all. -/
def batchStutterDerive {n : Nat} {α : Type} (p : Fin n → Nat → Bool) (T : Nat)
    (s : Fin n → StepHist α) : StutterBatchCuts n α :=
  fun i => batchesFrom ((s i).view) (tickSteps (p i) T) 0

/-- Derived snapshot cuts: the machine's tick-step reads. -/
def snapDerive {n : Nat} {α : Type} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (p : Fin n → Nat → Bool) (T : Nat)
    (src : Fin n → StepHist α) : SnapshotCuts n α ord :=
  fun i => snapCut ord ret
    ((tickSteps (p i) T).map (fun st => (src i).view st))

/-- Derived order selection: the machine's arrival order. -/
def ordSelDerive {n : Nat} {α : Type} (T : Nat)
    (u : Fin n → StepHist α) : OrderSelection n α :=
  fun i => (u i).view T

/-- Derived per-tick order selection (D71): the machine's arrival order
within each tick, as realized at step `T` — a tick's list never changes
once realized (`TMono`), so this is the order the machine held at every
coupled step. -/
def tkOrdSelDerive {n : Nat} {α : Type} (T : Nat)
    (u : Fin n → Nat → Trace (List α)) : TickOrderSelection n α :=
  fun i => u i T

/-- The per-tick order coupling: a machine tick trace below an
unordered denotation trace, with the selection being the machine's own
later-realized trace, is below the realized denotation trace — each
realized tick is its own permutation (`prefix_realizeTicks`). -/
theorem tassume_cpl {α : Type} [DecidableEq α]
    {l s : Trace (List α)} {v : Trace (Multiset α)}
    (hv : l.map (fun b => Multiset.ofList b) <+: v) (hs : l <+: s) :
    l <+: realizeTicks v s :=
  prefix_realizeTicks hv hs

theorem tickSteps_le_steps {p : Nat → Bool} {T T' : Nat} (h : T ≤ T') :
    tickSteps p T <+: tickSteps p T' := by
  induction T' with
  | zero => cases Nat.le_zero.mp h; exact List.prefix_refl _
  | succ T' ih =>
    rcases Nat.lt_or_ge T (T' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (tickSteps_prefix p T')
    · cases Nat.le_antisymm h hge; exact List.prefix_refl _

/-! ### Derived timing (D84)

The machine's timing decisions are total per-tick scripts
(`TickScript`); the denotation's are per-tick lists (the trace itself).
The corner derives the lists from the script and the pacing at the
horizon — the number of ticks the member has run by `T` — exactly as it
derives `snapshot_bounded`'s tick count. -/

/-- A script's first `ticks-by-T` entries, per member. -/
def scriptDerive {n : Nat} {β : Type} (p : Fin n → Nat → Bool) (T : Nat)
    (sc : TickScript n β) : Fin n → List β :=
  fun i => scriptTrace (sc i) (tickSteps (p i) T).length

theorem scriptDerive_mono {n : Nat} {β : Type} (p : Fin n → Nat → Bool) {T T' : Nat}
    (h : T ≤ T') (sc : TickScript n β) (i : Fin n) :
    scriptDerive p T sc i <+: scriptDerive p T' sc i :=
  scriptTrace_prefix (sc i) (tickSteps_le_steps h).length_le

theorem batchDerive_mono {n : Nat} {α : Type} [DecidableEq α]
    (p : Fin n → Nat → Bool) {T T' : Nat} (h : T ≤ T')
    (s : Fin n → StepHist α)
    (i : Fin n) : batchDerive p T s i <+: batchDerive p T' s i := by
  obtain ⟨e, he⟩ := tickSteps_le_steps (p := p i) h
  show (batchesFrom ((s i).view) (tickSteps (p i) T) 0).map
      (fun b => Multiset.ofList b)
    <+: (batchesFrom ((s i).view) (tickSteps (p i) T') 0).map
      (fun b => Multiset.ofList b)
  rw [← he, sqBatchesFrom_append]
  exact (List.prefix_append _ _).map _

theorem batchOrdDerive_mono {n : Nat} {α : Type} (p : Fin n → Nat → Bool)
    {T T' : Nat} (h : T ≤ T') (s : Fin n → StepHist α) (i : Fin n) :
    batchOrdDerive p T s i <+: batchOrdDerive p T' s i := by
  obtain ⟨e, he⟩ := tickSteps_le_steps (p := p i) h
  show (batchesFrom ((s i).view) (tickSteps (p i) T) 0).map List.length
    <+: (batchesFrom ((s i).view) (tickSteps (p i) T') 0).map List.length
  rw [← he, sqBatchesFrom_append]
  exact (List.prefix_append _ _).map _

theorem batchStutterDerive_mono {n : Nat} {α : Type} (p : Fin n → Nat → Bool)
    {T T' : Nat} (h : T ≤ T') (s : Fin n → StepHist α) (i : Fin n) :
    batchStutterDerive p T s i <+: batchStutterDerive p T' s i := by
  obtain ⟨e, he⟩ := tickSteps_le_steps (p := p i) h
  show batchesFrom ((s i).view) (tickSteps (p i) T) 0
    <+: batchesFrom ((s i).view) (tickSteps (p i) T') 0
  rw [← he, sqBatchesFrom_append]
  exact List.prefix_append _ _

theorem batchRetryDerive_mono {n : Nat} {α : Type} [DecidableEq α] (p : Fin n → Nat → Bool)
    {T T' : Nat} (h : T ≤ T') (s : Fin n → StepHist α) (i : Fin n) :
    batchRetryDerive p T s i <+: batchRetryDerive p T' s i := by
  obtain ⟨e, he⟩ := tickSteps_le_steps (p := p i) h
  show (batchesFrom ((s i).view) (tickSteps (p i) T) 0).map _
    <+: (batchesFrom ((s i).view) (tickSteps (p i) T') 0).map _
  rw [← he, sqBatchesFrom_append]
  exact (List.prefix_append _ _).map _

theorem cutsLen_append {α : Type} :
    ∀ (ws ws' : List (List α)) (a : Nat),
      cutsLen a (ws ++ ws')
        = cutsLen a ws ++ cutsLen (((a :: ws.map List.length).getLast
            (by simp)) ) ws' := by
  intro ws
  induction ws with
  | nil => intro ws' a; rfl
  | cons w ws ih =>
    intro ws' a
    show (w.length - a) :: cutsLen w.length (ws ++ ws') = _
    rw [ih ws' w.length]
    rfl

theorem cutsMS_append {α : Type} [DecidableEq α] :
    ∀ (ws ws' : List (List α)) (a : Multiset α),
      cutsMS a (ws ++ ws')
        = cutsMS a ws ++ cutsMS (((a :: ws.map
            (fun w => Multiset.ofList w)).getLast (by simp))) ws' := by
  intro ws
  induction ws with
  | nil => intro ws' a; rfl
  | cons w ws ih =>
    intro ws' a
    show (Multiset.ofList w - a) :: cutsMS (Multiset.ofList w) (ws ++ ws') = _
    rw [ih ws' (Multiset.ofList w)]
    rfl

theorem snapDerive_mono {n : Nat} {α : Type} [DecidableEq α]
    (ord : StrOrd) (ret : Retries) (p : Fin n → Nat → Bool) {T T' : Nat}
    (h : T ≤ T') (src : Fin n → StepHist α) (i : Fin n) :
    CutLe ord (snapDerive ord ret p T src i)
      (snapDerive ord ret p T' src i) := by
  obtain ⟨e, he⟩ := tickSteps_le_steps (p := p i) h
  show CutLe ord
    (snapCut ord ret
      ((tickSteps (p i) T).map (fun st => (src i).view st)))
    (snapCut ord ret
      ((tickSteps (p i) T').map (fun st => (src i).view st)))
  rw [← he, List.map_append]
  cases ord <;> cases ret
  · rw [show snapCut .totalOrder .exactlyOnce = fun ws => cutsLen 0 ws
      from rfl]
    simp only [cutsLen_append]
    exact ⟨_, rfl⟩
  · rw [show snapCut .totalOrder .atLeastOnce
      = fun ws => cutsLen 0 (ws.map destutter) from rfl]
    simp only [List.map_append, cutsLen_append]
    exact ⟨_, rfl⟩
  · rw [show snapCut .noOrder .exactlyOnce = fun ws => cutsMS 0 ws
      from rfl]
    simp only [cutsMS_append]
    exact ⟨_, rfl⟩
  · rw [show snapCut .noOrder .atLeastOnce = fun ws => cutsMS 0 ws
      from rfl]
    simp only [cutsMS_append]
    exact ⟨_, rfl⟩

theorem ordSelDerive_mono {n : Nat} {α : Type} {T T' : Nat} (h : T ≤ T')
    (u : Fin n → StepHist α) (i : Fin n) :
    ordSelDerive T u i <+: ordSelDerive T' u i :=
  (u i).mono_le h

/-! ## Shared machine/values utility lemmas (consumed by the coupling
corner) -/

/-- Bottom pools are least. -/
theorem poolBot_le {α : Type} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (v : PoolCarrier α ord ret) :
    PoolLe ord ret (PoolBot ord ret) v := by
  cases ord <;> cases ret
  · exact List.nil_prefix
  · show StutterSeq.le (StutterSeq.mk []) v
    show destutter [] <+: v.norm
    rw [destutter_nil]
    exact List.nil_prefix
  · exact Multiset.zero_le v
  · exact fun x hx => absurd (RetryPool.mem_mk x 0 |>.mp hx) (by simp)

/-- Reading a snapshot output uniformly across the source's bound. -/
theorem tickVals_snapshot {L : Type} {mem : L → Nat} {ℓ : L}
    {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (b : SingBound σ) (vr : SingletonV (mem ℓ) α σ ord b)
    (cut : SnapshotCuts (mem ℓ) α ord) (i : Fin (mem ℓ)) :
    ((Values L mem).snapshot (ℓ := ℓ) (ord := ord)
      (ret := ret) vr cut) i = singReads b vr i (cut i) := by
  cases b <;> rfl

/-- Drawing an already-realizable prefix first: `selectOrder` on an
extension of a legal draw list realizes the draw list as a prefix. -/
theorem selectOrder_prefix_ext {α : Type} [DecidableEq α] :
    ∀ (l e : List α) (pool : Multiset α),
      (↑l : Multiset α) ≤ pool →
      l <+: selectOrder pool (l ++ e)
  | [], _, _, _ => List.nil_prefix
  | x :: xs, e, pool, h => by
    have hx : x ∈ pool := Multiset.mem_of_le h
      (by rw [Multiset.mem_coe]; exact List.mem_cons_self ..)
    show x :: xs <+: selectOrder pool (x :: (xs ++ e))
    unfold selectOrder
    rw [if_pos hx]
    refine List.cons_prefix_cons.mpr ⟨rfl, ?_⟩
    refine selectOrder_prefix_ext xs e (pool.erase x) ?_
    have h' : (x ::ₘ (↑xs : Multiset α)) ≤ pool := by
      rw [Multiset.cons_coe]; exact h
    have := Multiset.erase_le_erase x h'
    rwa [Multiset.erase_cons_head] at this

/-- Glue a pointwise Kleene chain into an inequality between stages. -/
theorem pool_chain_glue {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {f : Nat → PoolCarrier α ord ret} {a : Nat} :
    ∀ {b : Nat}, a ≤ b →
      (∀ m, a ≤ m → m < b → PoolLe ord ret (f m) (f (m + 1))) →
      PoolLe ord ret (f a) (f b) := by
  intro b
  induction b with
  | zero => intro hab _; cases Nat.le_zero.mp hab; exact PoolLe.refl _ _ _
  | succ b ih =>
    intro hab hstep
    rcases Nat.lt_or_ge a (b + 1) with hlt | hge
    · exact PoolLe.trans
        (ih (Nat.lt_succ_iff.mp hlt)
          (fun m hm hmb => hstep m hm (Nat.lt_succ_of_lt hmb)))
        (hstep b (Nat.lt_succ_iff.mp hlt) (Nat.lt_succ_self b))
    · cases Nat.le_antisymm hab hge
      exact PoolLe.refl _ _ _

/-- Glue a pointwise trace-prefix chain. -/
theorem trace_chain_glue {σ : Type} {f : Nat → Trace σ} {a : Nat} :
    ∀ {b : Nat}, a ≤ b →
      (∀ m, a ≤ m → m < b → f m <+: f (m + 1)) →
      f a <+: f b := by
  intro b
  induction b with
  | zero => intro hab _; cases Nat.le_zero.mp hab; exact List.prefix_refl _
  | succ b ih =>
    intro hab hstep
    rcases Nat.lt_or_ge a (b + 1) with hlt | hge
    · exact List.IsPrefix.trans
        (ih (Nat.lt_succ_iff.mp hlt)
          (fun m hm hmb => hstep m hm (Nat.lt_succ_of_lt hmb)))
        (hstep b (Nat.lt_succ_iff.mp hlt) (Nat.lt_succ_self b))
    · cases Nat.le_antisymm hab hge
      exact List.prefix_refl _

end Hydro
