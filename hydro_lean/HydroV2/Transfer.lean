import HydroV2.Sched
import HydroV2.Values
import Mathlib.Data.List.Chain

/-!
# HydroV2 · the transfer coupling (concrete runs ⊑ graded quotients)

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

namespace HydroV2

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

/-- Weakening exactly-once content into the retry quotient stays
coupled (the machine's wire is unchanged; the pool coarsens). -/
theorem ListLe.weaken {α : Type} [DecidableEq α] :
    {ord : StrOrd} → {s : List α} →
    {v : PoolCarrier α ord .exactlyOnce} →
    ListLe ord .exactlyOnce s v →
    ListLe ord .atLeastOnce s
      (match ord, v with
       | .totalOrder, v => StutterSeq.mk v
       | .noOrder, v => RetryPool.mk v)
  | .totalOrder, _, _, h => destutter_prefix h
  | .noOrder, _, _, h => fun x hx => by
      rw [RetryPool.mem_mk] at hx ⊢
      exact Multiset.mem_of_le h hx

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

private theorem map_msfold_eq {α σ : Type} [DecidableEq α]
    (g : σ → α → σ) (init : σ)
    (ok : FoldOkP .noOrder .exactlyOnce g) :
    ∀ ws : List (List α),
      (ws.map (fun w => Multiset.ofList w)).map
        (PoolFold .noOrder .exactlyOnce g init ok)
      = ws.map (fun w => w.foldl g init)
  | [] => rfl
  | w :: ws => by
    simp only [List.map_cons]
    rw [map_msfold_eq g init ok ws]
    rfl

private theorem map_rpfold_eq {α σ : Type} [DecidableEq α]
    (g : σ → α → σ) (init : σ)
    (ok : FoldOkP .noOrder .atLeastOnce g) :
    ∀ ws : List (List α),
      ((ws.map (fun w => Multiset.ofList w)).map RetryPool.mk).map
        (PoolFold .noOrder .atLeastOnce g init ok)
      = ws.map (fun w => w.foldl g init)
  | [] => rfl
  | w :: ws => by
    simp only [List.map_cons]
    rw [map_rpfold_eq g init ok ws]
    rfl

/-- **Read agreement**: the machine's raw-arrival-order fold reads of an
ascending view chain *equal* the denotational `snapTrace` at derived
cuts, at every grade — the quotients' obligations pay the difference
between raw and quotient folds. -/
theorem sched_snap_eq {α σ : Type} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (g : σ → α → σ) (init : σ) (ok : FoldOkP ord ret g)
    (pool : PoolCarrier α ord ret) (ws : List (List α))
    (hchain : List.IsChain (· <+: ·) (([] : List α) :: ws))
    (hcpl : ∀ w ∈ ws, ListLe ord ret w pool) :
    ∃ d, ws.map (fun w => w.foldl g init)
      = snapTrace ord ret g init ok pool d := by
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
      (PoolFold .totalOrder .atLeastOnce g init ok)
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
      (PoolFold .noOrder .exactlyOnce g init ok)
    rw [show (0 : Multiset α) = (↑([] : List α) : Multiset α) from rfl,
      snapshotCuts_views ws [] hchain hcpl]
    exact (map_msfold_eq g init ok ws).symm
  case noOrder.atLeastOnce =>
    refine ⟨cutsMS 0 ws, ?_⟩
    show _ = ((snapshotMemCuts (RetryPool.support pool).val 0
        (cutsMS 0 ws)).map RetryPool.mk).map
      (PoolFold .noOrder .atLeastOnce g init ok)
    rw [show (0 : Multiset α) = (↑([] : List α) : Multiset α) from rfl,
      snapshotMemCuts_views ws []
        hchain
        (by
          intro w hw x hx
          exact RetryPool.mem_support_val.mpr
            (hcpl w hw x (by
              rw [RetryPool.mem_mk, Multiset.mem_coe]; exact hx))),
    ]
    exact (map_rpfold_eq g init ok ws).symm



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
    (ret : Retries) (g : σ → α → σ) (init : σ) (ok : FoldOkP ord ret g)
    (pool : PoolCarrier α ord ret) (ws : List (List α))
    (hchain : List.IsChain (· <+: ·) (([] : List α) :: ws))
    (hcpl : ∀ w ∈ ws, ListLe ord ret w pool) :
    ws.map (fun w => w.foldl g init)
      = snapTrace ord ret g init ok pool (snapCut ord ret ws) := by
  cases ord <;> cases ret
  case totalOrder.exactlyOnce =>
    show _ = (prefixCuts pool 0 (cutsLen 0 ws)).map
      (fun l => l.foldl g init)
    rw [show (0 : Nat) = ([] : List α).length from rfl,
      prefixCuts_views ws [] (List.nil_prefix) hchain hcpl]
  case totalOrder.atLeastOnce =>
    show _ = ((prefixCuts pool.norm 0
        (cutsLen 0 (ws.map destutter))).map StutterSeq.mk).map
      (PoolFold .totalOrder .atLeastOnce g init ok)
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
      (PoolFold .noOrder .exactlyOnce g init ok)
    rw [show (0 : Multiset α) = (↑([] : List α) : Multiset α) from rfl,
      snapshotCuts_views ws [] hchain hcpl]
    exact (map_msfold_eq g init ok ws).symm
  case noOrder.atLeastOnce =>
    show _ = ((snapshotMemCuts (RetryPool.support pool).val 0
        (cutsMS 0 ws)).map RetryPool.mk).map
      (PoolFold .noOrder .atLeastOnce g init ok)
    rw [show (0 : Multiset α) = (↑([] : List α) : Multiset α) from rfl,
      snapshotMemCuts_views ws []
        hchain
        (by
          intro w hw x hx
          exact RetryPool.mem_support_val.mpr
            (hcpl w hw x (by
              rw [RetryPool.mem_mk, Multiset.mem_coe]; exact hx))),
    ]
    exact (map_rpfold_eq g init ok ws).symm

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

/-- A state scan absorbs an entry translation. -/
theorem scanAcrossTicksTrace_map {ι κ σ β : Type _} (f : ι → κ)
    (g : σ → κ → σ × β) :
    ∀ (l : List ι) (init : σ),
      scanAcrossTicksTrace g init (l.map f)
        = scanAcrossTicksTrace (fun s x => g s (f x)) init l
  | [], _ => rfl
  | x :: xs, init => by
    show (g init (f x)).2 :: _ = (g init (f x)).2 :: _
    rw [scanAcrossTicksTrace_map f g xs (g init (f x)).1]

/-- A state fold absorbs an entry translation. -/
theorem foldAcrossTicksTrace_map {ι κ σ : Type _} (f : ι → κ)
    (g : σ → κ → σ) :
    ∀ (l : List ι) (init : σ),
      foldAcrossTicksTrace g init (l.map f)
        = foldAcrossTicksTrace (fun s x => g s (f x)) init l
  | [], _ => rfl
  | x :: xs, init => by
    show g init (f x) :: _ = g init (f x) :: _
    rw [foldAcrossTicksTrace_map f g xs (g init (f x))]

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

/-- Validated emission is a prefix of the emitted values, entrywise. -/
theorem emitLin_map_ofList_prefix {β : Type} [DecidableEq β] :
    ∀ (vs : Trace (Multiset β)) (ls : List (List β)),
      (emitLin vs ls).map (fun l => Multiset.ofList l) <+: vs
  | [], _ => List.nil_prefix
  | _ :: _, [] => List.nil_prefix
  | m :: vs, l :: ls => by
    show (if (↑l : Multiset β) = m then l :: emitLin vs ls
      else []).map (fun l => Multiset.ofList l) <+: m :: vs
    split
    · next hml =>
      rw [List.map_cons]
      exact List.cons_prefix_cons.mpr
        ⟨hml, emitLin_map_ofList_prefix vs ls⟩
    · exact List.nil_prefix

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

/-! ## The coupled carriers (per-step ∃-witness shapes)

Every relation is `∀ step, ∃ v ∈ V, …`: at each step the machine's
concrete state is coupled to *some* denotational run in the relational
carrier. The witness may vary with the step — that is what lets cycles
couple (step `t` couples to the depth-`t+1` Kleene iterate); reads that
assemble machine state across steps (snapshots of folds) recover a
single witness through the stream carriers' *bundled monotonicity*
(views at earlier steps are prefixes of the current view). -/

def StreamRel (n : Nat) (α : Type) [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (h : Fin n → StepHist α)
    (V : Set (Fin n → PoolCarrier α ord ret)) : Prop :=
  ∀ t, ∃ v ∈ V, ∀ i, ListLe ord ret ((h i).view t) (v i)

def KeyedRel (p c : Nat) (α : Type) [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (h : Fin p → Fin c → StepHist α)
    (V : Set (Fin p → Fin c → PoolCarrier α ord ret)) : Prop :=
  ∀ t, ∃ v ∈ V, ∀ i j, ListLe ord ret ((h i j).view t) (v i j)

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
  | .totalOrder, .atLeastOnce => fun _ _ => True
  | .noOrder, .atLeastOnce => fun _ _ => True

def TickStreamRel (n : Nat) (α : Type) [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (s : Fin n → Nat → Trace (List α))
    (V : Set (Fin n → Trace (PoolCarrier α ord ret))) : Prop :=
  ∀ step, ∃ v ∈ V, ∀ i, BatchTraceLe ord ret (s i step) (v i)

/-- Read a `TickV` uniformly across the bound. -/
@[reducible] def tickVals {n : Nat} {σ : Type} :
    (b : SingBound σ) → TickV n σ b → Fin n → Trace σ
  | .unbounded => fun v => v
  | .monotonic _ => fun v i => (v i).vals

def TickSingRel (n : Nat) (σ : Type) (b : SingBound σ)
    (s : Fin n → Nat → Trace σ) (V : Set (TickV n σ b)) : Prop :=
  ∀ step, ∃ v ∈ V, ∀ i, s i step <+: tickVals b v i

/-- Read a `SingletonV` uniformly across the bound. -/
@[reducible] def singReads {n : Nat} {α σ : Type} [DecidableEq α]
    {ord : StrOrd} :
    (b : SingBound σ) → SingletonV n α σ ord b →
      Fin n → CutDec α ord → Trace σ
  | .unbounded => fun v => v
  | .monotonic _ => fun v i => (v i).val

/-- Fold-singleton coupling: at every step the machine's fold is the
raw-order list fold of a source dominated by *some* denotational pool
whose `snapTrace` is the witness's read function. -/
def FoldSingRel (n : Nat) (α σ : Type) [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (b : SingBound σ) (sc : Fin n → SchedFold α σ)
    (V : Set (SingletonV n α σ ord b)) : Prop :=
  ∀ T, ∃ vr ∈ V, ∃ (g : σ → α → σ) (init : σ)
    (ok : FoldOkP ord ret g) (pool : Fin n → PoolCarrier α ord ret),
    (∀ i, singReads b vr i = snapTrace ord ret g init ok (pool i)) ∧
    (∀ i, (sc i).read = fun l => l.foldl g init) ∧
    (∀ i, ListLe ord ret ((sc i).src.view T) (pool i))

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

/-! ## Per-op coupling lemmas (the `CorrSem` fields' content) -/

section CorrOps

variable {L : Type} {mem : L → Nat}

/-- `values` fan-in stays coupled, at every grade: emergent interleaving
is below the denotational merge. -/
theorem corr_values {p c : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    {h : Fin (mem p) → Fin (mem c) → StepHist α}
    {v : Fin (mem p) → Fin (mem c) → PoolCarrier α ord ret} {t : Nat}
    (hc : ∀ i j, ListLe ord ret ((h i j).view t) (v i j)) :
    ∀ i, ListLe .noOrder ret (mergeNView (h i) t)
      ((Values L mem).values (p := p) (c := c) v i) := by
  intro i
  cases ord <;> cases ret
  case totalOrder.exactlyOnce =>
    show Multiset.ofList (mergeNView (h i) t) ≤ _
    rw [coe_mergeNView]
    exact sum_le_sum_pointwise (fun j _ =>
      Multiset.coe_le.mpr (hc i j).sublist.subperm)
  case noOrder.exactlyOnce =>
    show Multiset.ofList (mergeNView (h i) t) ≤ _
    rw [coe_mergeNView]
    exact sum_le_sum_pointwise (fun j _ => hc i j)
  case totalOrder.atLeastOnce =>
    intro x hx
    rw [RetryPool.mem_mk, Multiset.mem_coe] at hx
    obtain ⟨j, hj⟩ := mem_mergeNView hx
    refine mem_foldl_union _ _ (Or.inl ?_)
    refine ⟨RetryPool.mk (↑(v i j).norm : Multiset α),
      List.mem_map.mpr ⟨j, List.mem_finRange j, rfl⟩, ?_⟩
    rw [RetryPool.mem_mk, Multiset.mem_coe]
    exact (hc i j).sublist.mem (mem_destutter hj)
  case noOrder.atLeastOnce =>
    intro x hx
    rw [RetryPool.mem_mk, Multiset.mem_coe] at hx
    obtain ⟨j, hj⟩ := mem_mergeNView hx
    refine mem_foldl_union _ _ (Or.inl ?_)
    refine ⟨v i j, List.mem_map.mpr ⟨j, List.mem_finRange j, rfl⟩, ?_⟩
    exact hc i j x (by rw [RetryPool.mem_mk, Multiset.mem_coe]; exact hj)

/-- `union` fan-in stays coupled at both retry grades. -/
theorem corr_union {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries}
    {a b : Fin (mem ℓ) → StepHist α}
    {va vb : Fin (mem ℓ) → PoolCarrier α .noOrder ret} {t : Nat}
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
    {v : Fin (mem ℓ) → PoolCarrier α ord .exactlyOnce} {t : Nat}
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

/-- The machine's snapshot trace at its tick steps *equals* a
denotational snapshot at derived cuts (one per member, at its own
skeleton). -/
theorem corr_snapshot {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {n : Nat} (p : Fin n → Nat → Bool) (T : Nat)
    (sc : Fin n → SchedFold α σ) (g : σ → α → σ) (init : σ)
    (ok : FoldOkP ord ret g) (pool : Fin n → PoolCarrier α ord ret)
    (hfold : ∀ i, (sc i).read = fun l => l.foldl g init)
    (hcpl : ∀ i, ListLe ord ret ((sc i).src.view T) (pool i)) :
    ∀ i, ∃ d, (tickSteps (p i) T).map
        (fun st => (sc i).read ((sc i).src.view st))
      = snapTrace ord ret g init ok (pool i) d := by
  intro i
  have h1 : (tickSteps (p i) T).map
      (fun st => (sc i).read ((sc i).src.view st))
      = ((tickSteps (p i) T).map (fun st => (sc i).src.view st)).map
        (fun w => w.foldl g init) := by
    rw [List.map_map]
    exact List.map_congr_left (fun st _ => by rw [hfold i]; rfl)
  obtain ⟨d, hd⟩ := sched_snap_eq ord ret g init ok (pool i)
    ((tickSteps (p i) T).map (fun st => (sc i).src.view st))
    (viewsChain _ (tickSteps_pairwise (p i) T))
    (fun w hw => by
      obtain ⟨st, hst, rfl⟩ := List.mem_map.mp hw
      exact ListLe.of_prefix ((sc i).src.mono_le (tickSteps_le hst))
        (hcpl i))
  exact ⟨d, h1.trans hd⟩

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

end CorrOps2

/-- Batch-entry translation under a per-entry quotient agreement:
mapping machine batches then quotienting equals quotienting then
mapping denotationally. -/
theorem map_ofList_batches {α γ β : Type} [DecidableEq α]
    [DecidableEq β] (F : List α × γ → List β)
    (G : Multiset α × γ → Multiset β)
    (hFG : ∀ bx : List α × γ,
      Multiset.ofList (F bx) = G (Multiset.ofList bx.1, bx.2)) :
    ∀ zs : List (List α × γ),
      (zs.map F).map (fun b => Multiset.ofList b)
        = (zs.map (fun p => (Multiset.ofList p.1, p.2))).map G
  | [] => rfl
  | z :: zs => by
    simp only [List.map_cons, hFG z,
      map_ofList_batches F G hFG zs]

/-! ## The coupled interpretation

Each carrier pairs a step-machine run with a set of denotational runs
it is coupled to; each op runs the machine on the first component and
images the `Values` op over the second, proving the coupling — the
per-op content of "every machine run is below some denotational run at
derived decisions". Totality of this instance is the cheat detector:
an unfillable coupling field would mean the denotational decision
space cannot express a machine behavior. -/

set_option warn.classDefReducibility false in
/-- The transfer coupling instance, over a per-member tick skeleton
`pacing`. -/
def CorrSem (L : Type) (mem : L → Nat)
    (pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) :
    HydroSem L mem where
  Stream ℓ α _ ord ret :=
    {p : (Fin (mem ℓ) → StepHist α)
        × Set (Fin (mem ℓ) → PoolCarrier α ord ret) //
      StreamRel (mem ℓ) α ord ret p.1 p.2}
  KeyedStream p c α _ ord ret :=
    {q : (Fin (mem p) → Fin (mem c) → StepHist α)
        × Set (Fin (mem p) → Fin (mem c) → PoolCarrier α ord ret) //
      KeyedRel (mem p) (mem c) α ord ret q.1 q.2}
  Singleton ℓ α σ _ ord ret b :=
    {q : (Fin (mem ℓ) → SchedFold α σ)
        × Set (SingletonV (mem ℓ) α σ ord b) //
      FoldSingRel (mem ℓ) α σ ord ret b q.1 q.2}
  TickSingleton ℓ σ b :=
    {q : (Fin (mem ℓ) → Nat → Trace σ) × Set (TickV (mem ℓ) σ b) //
      TickSingRel (mem ℓ) σ b q.1 q.2}
  TickStream ℓ α _ ord ret :=
    {q : (Fin (mem ℓ) → Nat → Trace (List α))
        × Set (Fin (mem ℓ) → Trace (PoolCarrier α ord ret)) //
      TickStreamRel (mem ℓ) α ord ret q.1 q.2}
  TransportDec p c := Fin p → Fin c → Nat → Nat
  OrderSelDec _ _ := Unit
  SnapDec _ _ _ := Unit
  BatchDec _ _ := Unit
  OrdBatchDec _ := Unit
  BatchOrdSelDec _ _ := Unit
  SampleDec n := SampleTimes n
  TimerDec n := TimerVerdicts n
  PulseDec n := TimingPulses n
  EmitDec n β := Fin n → List (List β)
  FixDec := Unit
  map s f :=
    ⟨(fun i => (s.val.1 i).map (f i),
      {y | ∃ x ∈ s.val.2, y = (Values L mem).map x f}),
     fun t => by
       obtain ⟨v, hv, hc⟩ := s.property t
       exact ⟨(Values L mem).map v f, ⟨v, hv, rfl⟩,
         fun i => ListLe.map_eo (f i) (hc i)⟩⟩
  filterMap s f :=
    ⟨(fun i => (s.val.1 i).filterMap (f i),
      {y | ∃ x ∈ s.val.2, y = (Values L mem).filterMap x f}),
     fun t => by
       obtain ⟨v, hv, hc⟩ := s.property t
       exact ⟨(Values L mem).filterMap v f, ⟨v, hv, rfl⟩,
         fun i => ListLe.filterMap_eo (f i) (hc i)⟩⟩
  broadcast d s :=
    ⟨(fun i j => (s.val.1 j).deliver (d i j),
      {y | ∃ x ∈ s.val.2, y = (Values L mem).broadcast () x}),
     fun t => by
       obtain ⟨v, hv, hc⟩ := s.property t
       refine ⟨(Values L mem).broadcast () v, ⟨v, hv, rfl⟩,
         fun i j => ?_⟩
       cases t with
       | zero => exact ListLe.nil _ _ _
       | succ u =>
         exact ListLe.take _
           (ListLe.of_prefix ((s.val.1 j).mono_le (Nat.le_succ u))
             (hc j))⟩
  demux d s addr :=
    ⟨(fun i j => ((s.val.1 j).filterMap
        (fun dx => if dx.1 = addr i then some dx.2 else none)).deliver
        (d i j),
      {y | ∃ x ∈ s.val.2, y = (Values L mem).demux () x addr}),
     fun t => by
       obtain ⟨v, hv, hc⟩ := s.property t
       refine ⟨(Values L mem).demux () v addr, ⟨v, hv, rfl⟩,
         fun i j => ?_⟩
       cases t with
       | zero => exact ListLe.nil _ _ _
       | succ u =>
         exact ListLe.take _
           (ListLe.of_prefix
             (prefix_filterMap _ ((s.val.1 j).mono_le (Nat.le_succ u)))
             (ListLe.filterMap_eo _ (hc j)))⟩
  values k :=
    ⟨(fun i => mergeN (k.val.1 i),
      {y | ∃ x ∈ k.val.2, y = (Values L mem).values x}),
     fun t => by
       obtain ⟨v, hv, hc⟩ := k.property t
       exact ⟨(Values L mem).values v, ⟨v, hv, rfl⟩, corr_values hc⟩⟩
  weaken_retries s :=
    ⟨(s.val.1,
      {y | ∃ x ∈ s.val.2, y = (Values L mem).weaken_retries x}),
     fun t => by
       obtain ⟨v, hv, hc⟩ := s.property t
       exact ⟨(Values L mem).weaken_retries v, ⟨v, hv, rfl⟩,
         corr_weaken hc⟩⟩
  union a b :=
    ⟨(fun i => merge2 (a.val.1 i) (b.val.1 i),
      {y | ∃ xa ∈ a.val.2, ∃ xb ∈ b.val.2,
        y = (Values L mem).union xa xb}),
     fun t => by
       obtain ⟨va, hva, hca⟩ := a.property t
       obtain ⟨vb, hvb, hcb⟩ := b.property t
       exact ⟨(Values L mem).union va vb, ⟨va, hva, vb, hvb, rfl⟩,
         corr_union hca hcb⟩⟩
  assume_ordering u _d :=
    ⟨(u.val.1,
      {y | ∃ x ∈ u.val.2, ∃ d, y = (Values L mem).assume_ordering x d}),
     fun t => by
       obtain ⟨v, hv, hc⟩ := u.property t
       refine ⟨(Values L mem).assume_ordering v
           (fun i => (u.val.1 i).view t),
         ⟨v, hv, _, rfl⟩, fun i => ?_⟩
       show (u.val.1 i).view t
         <+: selectOrder (v i) ((u.val.1 i).view t)
       rw [selectOrder_of_le _ _ (hc i)]⟩
  fold g init ok s :=
    ⟨(fun i => ⟨s.val.1 i, fun l => l.foldl g init⟩,
      {y | ∃ x ∈ s.val.2, y = (Values L mem).fold g init ok x}),
     fun T => by
       obtain ⟨v, hv, hc⟩ := s.property T
       exact ⟨(Values L mem).fold g init ok v, ⟨v, hv, rfl⟩,
         g, init, ok, v, fun i => rfl, fun i => rfl, hc⟩⟩
  fold_monotone vo g init ok hinfl s :=
    ⟨(fun i => ⟨s.val.1 i, fun l => l.foldl g init⟩,
      {y | ∃ x ∈ s.val.2,
        y = (Values L mem).fold_monotone vo g init ok hinfl x}),
     fun T => by
       obtain ⟨v, hv, hc⟩ := s.property T
       exact ⟨(Values L mem).fold_monotone vo g init ok hinfl v,
         ⟨v, hv, rfl⟩, g, init, ok, v, fun i => rfl, fun i => rfl, hc⟩⟩
  snapshot {ℓ _α _σ _ ord ret b} s _d :=
    ⟨(fun i step => (tickSteps (pacing ℓ i) step).map
        (fun st => (s.val.1 i).read ((s.val.1 i).src.view st)),
      {y | ∃ x ∈ s.val.2, ∃ d,
        y = (Values L mem).snapshot (ord := ord) (ret := ret) x d}),
     fun T => by
       obtain ⟨vr, hvr, g, init, ok, pool, hread, hfold, hcpl⟩ :=
         s.property T
       choose d hd using corr_snapshot (pacing ℓ) T s.val.1 g init ok
         pool hfold hcpl
       refine ⟨(Values L mem).snapshot (ord := ord) (ret := ret) vr d,
         ?_, ?_⟩
       · exact ⟨vr, hvr, d, rfl⟩
       · intro i
         have hv : (tickSteps (pacing ℓ i) T).map
             (fun st => (s.val.1 i).read ((s.val.1 i).src.view st))
             = singReads b vr i (d i) := by
           rw [hd i, ← hread i]
         cases b with
         | unbounded =>
           show (tickSteps (pacing ℓ i) T).map
               (fun st => (s.val.1 i).read ((s.val.1 i).src.view st))
             <+: vr i (d i)
           rw [hv]
         | monotonic vo =>
           show (tickSteps (pacing ℓ i) T).map
               (fun st => (s.val.1 i).read ((s.val.1 i).src.view st))
             <+: (vr i).val (d i)
           rw [hv]⟩
  batch {ℓ _α _} s _d :=
    ⟨(fun i step => batchesFrom ((s.val.1 i).view)
        (tickSteps (pacing ℓ i) step) 0,
      {y | ∃ x ∈ s.val.2, ∃ d, y = (Values L mem).batch x d}),
     fun T => by
       obtain ⟨v, hv, hc⟩ := s.property T
       refine ⟨(Values L mem).batch v (fun i =>
           (batchesFrom ((s.val.1 i).view)
             (tickSteps (pacing ℓ i) T) 0).map
             (fun b => Multiset.ofList b)),
         ⟨v, hv, _, rfl⟩, fun i => ?_⟩
       show (batchesFrom ((s.val.1 i).view)
           (tickSteps (pacing ℓ i) T) 0).map (fun b => Multiset.ofList b)
         <+: batchCuts (v i) 0
           ((batchesFrom ((s.val.1 i).view)
             (tickSteps (pacing ℓ i) T) 0).map (fun b => Multiset.ofList b))
       rw [corr_batch (pacing ℓ) T s.val.1 v hc i]⟩
  batch_ordered {ℓ _α _} s _d :=
    ⟨(fun i step => batchesFrom ((s.val.1 i).view)
        (tickSteps (pacing ℓ i) step) 0,
      {y | ∃ x ∈ s.val.2, ∃ d, y = (Values L mem).batch_ordered x d}),
     fun T => by
       obtain ⟨v, hv, hc⟩ := s.property T
       refine ⟨(Values L mem).batch_ordered v (fun i =>
           (batchesFrom ((s.val.1 i).view)
             (tickSteps (pacing ℓ i) T) 0).map List.length),
         ⟨v, hv, _, rfl⟩, fun i => ?_⟩
       show batchesFrom ((s.val.1 i).view) (tickSteps (pacing ℓ i) T) 0
         <+: sliceCuts (v i) 0
           ((batchesFrom ((s.val.1 i).view)
             (tickSteps (pacing ℓ i) T) 0).map List.length)
       rw [corr_batch_ordered (pacing ℓ) T s.val.1 v hc i]⟩
  assume_ordering_batch bs _d :=
    ⟨(bs.val.1,
      {y | ∃ x ∈ bs.val.2, ∃ d,
        y = (Values L mem).assume_ordering_batch x d}),
     fun T => by
       obtain ⟨v, hv, hc⟩ := bs.property T
       refine ⟨(Values L mem).assume_ordering_batch v
           (fun i => bs.val.1 i T), ⟨v, hv, _, rfl⟩, fun i => ?_⟩
       show bs.val.1 i T <+: (Trace.zip (v i) (bs.val.1 i T)).map
         (fun bd => selectOrder bd.1 bd.2)
       rw [zip_of_map_prefix _ _ _ (hc i), List.map_map]
       have hmap : (bs.val.1 i T).map
           ((fun bd => selectOrder bd.1 bd.2)
             ∘ (fun b => (Multiset.ofList b, b)))
           = bs.val.1 i T := by
         calc (bs.val.1 i T).map _
             = (bs.val.1 i T).map id :=
               List.map_congr_left (fun b _ =>
                 selectOrder_of_le b _ (le_refl _))
           _ = bs.val.1 i T := List.map_id _
       rw [hmap]⟩
  mapBatchWith bs t f :=
    ⟨(fun i step => (Trace.zip (bs.val.1 i step) (t.val.1 i step)).map
        (fun bx => f i bx.1 bx.2),
      {y | ∃ xb ∈ bs.val.2, ∃ xt ∈ t.val.2,
        y = (Values L mem).mapBatchWith xb xt f}),
     fun T => by
       obtain ⟨vb, hvb, hb⟩ := bs.property T
       obtain ⟨vt, hvt, ht⟩ := t.property T
       exact ⟨(Values L mem).mapBatchWith vb vt f,
         ⟨vb, hvb, vt, hvt, rfl⟩,
         fun i => (zip_prefix (hb i) (ht i)).map _⟩⟩
  mapBatch bs f :=
    ⟨(fun i step => (bs.val.1 i step).map (f i),
      {y | ∃ x ∈ bs.val.2, y = (Values L mem).mapBatch x f}),
     fun T => by
       obtain ⟨v, hv, hb⟩ := bs.property T
       exact ⟨(Values L mem).mapBatch v f, ⟨v, hv, rfl⟩,
         fun i => List.IsPrefix.map (f i) (hb i)⟩⟩
  mapBatchesWith bs t f :=
    ⟨(fun i step => (Trace.zip (bs.val.1 i step) (t.val.1 i step)).map
        (fun bx => bx.1.map (fun a => f i a bx.2)),
      {y | ∃ xb ∈ bs.val.2, ∃ xt ∈ t.val.2,
        y = (Values L mem).mapBatchesWith xb xt f}),
     fun T => by
       obtain ⟨vb, hvb, hb⟩ := bs.property T
       obtain ⟨vt, hvt, ht⟩ := t.property T
       refine ⟨(Values L mem).mapBatchesWith vb vt f,
         ⟨vb, hvb, vt, hvt, rfl⟩, fun i => ?_⟩
       show ((Trace.zip (bs.val.1 i T) (t.val.1 i T)).map
           (fun bx => bx.1.map (fun a => f i a bx.2))).map
           (fun b => Multiset.ofList b) <+: _
       refine List.IsPrefix.trans ?_ ((zip_prefix (hb i) (ht i)).map _)
       rw [map_ofList_batches
           (fun bx => bx.1.map (fun a => f i a bx.2))
           (fun bx => bx.1.map (fun a => f i a bx.2))
           (fun _ => rfl), ← zip_map_left']⟩
  scan_batches_across_ticks bs t g init :=
    ⟨(fun i step => scanAcrossTicksTrace
        (fun s bt => g i s bt.1 bt.2) init
        (Trace.zip (bs.val.1 i step) (t.val.1 i step)),
      {y | ∃ xb ∈ bs.val.2, ∃ xt ∈ t.val.2,
        y = (Values L mem).scan_batches_across_ticks xb xt g init}),
     fun T => by
       obtain ⟨vb, hvb, hb⟩ := bs.property T
       obtain ⟨vt, hvt, ht⟩ := t.property T
       exact ⟨(Values L mem).scan_batches_across_ticks vb vt g init,
         ⟨vb, hvb, vt, hvt, rfl⟩,
         fun i => scanAcrossTicksTrace_prefix _ init
           (zip_prefix (hb i) (ht i))⟩⟩
  fold_batches_across_ticks_monotone vo g init comm hinfl bs :=
    ⟨(fun i step => foldAcrossTicksTrace
        (fun s b => b.foldl (g i) s) init (bs.val.1 i step),
      {y | ∃ x ∈ bs.val.2, y =
        (Values L mem).fold_batches_across_ticks_monotone
          vo g init comm hinfl x}),
     fun T => by
       obtain ⟨v, hv, hb⟩ := bs.property T
       refine ⟨(Values L mem).fold_batches_across_ticks_monotone
           vo g init comm hinfl v, ⟨v, hv, rfl⟩, fun i => ?_⟩
       show foldAcrossTicksTrace (fun s b => b.foldl (g i) s) init
           (bs.val.1 i T)
         <+: foldAcrossTicksTrace (fun s b => @Multiset.foldl _ _ (g i)
           ⟨fun s x y => comm i s x y⟩ s b) init (v i)
       refine List.IsPrefix.trans ?_
         (foldAcrossTicksTrace_prefix _ init (hb i))
       rw [foldAcrossTicksTrace_map]
       exact List.prefix_refl _⟩
  sample_every t d :=
    ⟨(famHist (fun step i => sampleAtOpt (t.val.1 i step) (d i)),
      {y | ∃ x ∈ t.val.2, ∃ d',
        y = (Values L mem).sample_every x d'}),
     fun T => by
       obtain ⟨k, _hk, he⟩ := famFreeze_eq_raw
         (fun step i => sampleAtOpt (t.val.1 i step) (d i)) T
       obtain ⟨v, hv, hc⟩ := t.property k
       refine ⟨(Values L mem).sample_every v d, ⟨v, hv, d, rfl⟩,
         fun i => ?_⟩
       show StutterSeq.le (StutterSeq.mk (famFreeze
           (fun step i => sampleAtOpt (t.val.1 i step) (d i)) T i))
         (StutterSeq.mk (sampleAtOpt (v i) (d i)))
       rw [congrFun he i]
       exact destutter_prefix (sampleAtOpt_prefix (hc i) (d i))⟩
  timeout_snapshot {ℓ _α _ _ord _ret} s d :=
    ⟨(fun i step => (d i).take (tickSteps (pacing ℓ i) step).length,
      {y | ∃ x ∈ s.val.2, ∃ d',
        y = (Values L mem).timeout_snapshot x d'}),
     fun _T => by
       obtain ⟨v, hv, _⟩ := s.property 0
       exact ⟨(Values L mem).timeout_snapshot v d, ⟨v, hv, d, rfl⟩,
         fun i => List.take_prefix _ _⟩⟩
  source_interval_batch {ℓ} d :=
    ⟨(fun i step => (d i).take (tickSteps (pacing ℓ i) step).length,
      {y | ∃ d', y = (Values L mem).source_interval_batch d'}),
     fun _T =>
       ⟨(Values L mem).source_interval_batch d, ⟨d, rfl⟩,
        fun i => List.take_prefix _ _⟩⟩
  scan_batches_unordered_across_ticks bs t g init :=
    ⟨(fun i step => scanAcrossTicksTrace
        (fun s bt => g i s (Multiset.ofList bt.1) bt.2) init
        (Trace.zip (bs.val.1 i step) (t.val.1 i step)),
      {y | ∃ xb ∈ bs.val.2, ∃ xt ∈ t.val.2, y =
        (Values L mem).scan_batches_unordered_across_ticks xb xt g init}),
     fun T => by
       obtain ⟨vb, hvb, hb⟩ := bs.property T
       obtain ⟨vt, hvt, ht⟩ := t.property T
       refine ⟨(Values L mem).scan_batches_unordered_across_ticks
           vb vt g init, ⟨vb, hvb, vt, hvt, rfl⟩, fun i => ?_⟩
       refine List.IsPrefix.trans ?_
         (scanAcrossTicksTrace_prefix _ init (zip_prefix (hb i) (ht i)))
       rw [zip_map_left', scanAcrossTicksTrace_map]⟩
  scan_batches_unordered bs g init :=
    ⟨(fun i step => scanAcrossTicksTrace
        (fun s b => g i s (Multiset.ofList b)) init (bs.val.1 i step),
      {y | ∃ x ∈ bs.val.2,
        y = (Values L mem).scan_batches_unordered x g init}),
     fun T => by
       obtain ⟨v, hv, hb⟩ := bs.property T
       refine ⟨(Values L mem).scan_batches_unordered v g init,
         ⟨v, hv, rfl⟩, fun i => ?_⟩
       refine List.IsPrefix.trans ?_
         (scanAcrossTicksTrace_prefix _ init (hb i))
       rw [scanAcrossTicksTrace_map]⟩
  scan_batches_unordered₂ bs cs g init :=
    ⟨(fun i step => scanAcrossTicksTrace
        (fun s bt => g i s (Multiset.ofList bt.1) (Multiset.ofList bt.2))
        init (Trace.zip (bs.val.1 i step) (cs.val.1 i step)),
      {y | ∃ xb ∈ bs.val.2, ∃ xc ∈ cs.val.2,
        y = (Values L mem).scan_batches_unordered₂ xb xc g init}),
     fun T => by
       obtain ⟨vb, hvb, hb⟩ := bs.property T
       obtain ⟨vc, hvc, hcb⟩ := cs.property T
       refine ⟨(Values L mem).scan_batches_unordered₂ vb vc g init,
         ⟨vb, hvb, vc, hvc, rfl⟩, fun i => ?_⟩
       refine List.IsPrefix.trans ?_
         (scanAcrossTicksTrace_prefix _ init (zip_prefix (hb i) (hcb i)))
       rw [zip_map_right', zip_map_left', List.map_map,
         scanAcrossTicksTrace_map]
       exact List.prefix_refl _⟩
  scan_across_ticks t g init :=
    ⟨(fun i step => scanAcrossTicksTrace (g i) init (t.val.1 i step),
      {y | ∃ x ∈ t.val.2,
        y = (Values L mem).scan_across_ticks x g init}),
     fun T => by
       obtain ⟨v, hv, ht⟩ := t.property T
       exact ⟨(Values L mem).scan_across_ticks v g init, ⟨v, hv, rfl⟩,
         fun i => scanAcrossTicksTrace_prefix _ init (ht i)⟩⟩
  mapTick s f :=
    ⟨(fun i step => (s.val.1 i step).map (f i),
      {y | ∃ x ∈ s.val.2, y = (Values L mem).mapTick x f}),
     fun T => by
       obtain ⟨v, hv, ht⟩ := s.property T
       exact ⟨(Values L mem).mapTick v f, ⟨v, hv, rfl⟩,
         fun i => List.IsPrefix.map (f i) (ht i)⟩⟩
  zipTick a b :=
    ⟨(fun i step => Trace.zip (a.val.1 i step) (b.val.1 i step),
      {y | ∃ xa ∈ a.val.2, ∃ xb ∈ b.val.2,
        y = (Values L mem).zipTick xa xb}),
     fun T => by
       obtain ⟨va, hva, ha⟩ := a.property T
       obtain ⟨vb, hvb, hb⟩ := b.property T
       exact ⟨(Values L mem).zipTick va vb, ⟨va, hva, vb, hvb, rfl⟩,
         fun i => zip_prefix (ha i) (hb i)⟩⟩
  fold_across_ticks_monotone vo g init hinfl s :=
    ⟨(fun i step => foldAcrossTicksTrace (g i) init (s.val.1 i step),
      {y | ∃ x ∈ s.val.2, y =
        (Values L mem).fold_across_ticks_monotone vo g init hinfl x}),
     fun T => by
       obtain ⟨v, hv, ht⟩ := s.property T
       exact ⟨(Values L mem).fold_across_ticks_monotone vo g init hinfl v,
         ⟨v, hv, rfl⟩,
         fun i => foldAcrossTicksTrace_prefix _ init (ht i)⟩⟩
  mapMonotone vo' m h hpres :=
    ⟨(fun i step => (m.val.1 i step).map (h i),
      {y | ∃ x ∈ m.val.2,
        y = (Values L mem).mapMonotone vo' x h hpres}),
     fun T => by
       obtain ⟨v, hv, hm⟩ := m.property T
       exact ⟨(Values L mem).mapMonotone vo' v h hpres, ⟨v, hv, rfl⟩,
         fun i => List.IsPrefix.map (h i) (hm i)⟩⟩
  forgetBound m :=
    ⟨(m.val.1, {y | ∃ x ∈ m.val.2, y = (Values L mem).forgetBound x}),
     fun T => by
       obtain ⟨v, hv, hm⟩ := m.property T
       exact ⟨(Values L mem).forgetBound v, ⟨v, hv, rfl⟩, hm⟩⟩
  defer init t :=
    ⟨(fun i step => init :: t.val.1 i step,
      {y | ∃ x ∈ t.val.2, y = (Values L mem).defer init x}),
     fun T => by
       obtain ⟨v, hv, ht⟩ := t.property T
       exact ⟨(Values L mem).defer init v, ⟨v, hv, rfl⟩,
         fun i => List.cons_prefix_cons.mpr ⟨rfl, ht i⟩⟩⟩
  filterMapBatchesWith bs t f :=
    ⟨(fun i step => (Trace.zip (bs.val.1 i step) (t.val.1 i step)).map
        (fun bx => bx.1.filterMap (fun a => f i a bx.2)),
      {y | ∃ xb ∈ bs.val.2, ∃ xt ∈ t.val.2,
        y = (Values L mem).filterMapBatchesWith xb xt f}),
     fun T => by
       obtain ⟨vb, hvb, hb⟩ := bs.property T
       obtain ⟨vt, hvt, ht⟩ := t.property T
       refine ⟨(Values L mem).filterMapBatchesWith vb vt f,
         ⟨vb, hvb, vt, hvt, rfl⟩, fun i => ?_⟩
       show ((Trace.zip (bs.val.1 i T) (t.val.1 i T)).map
           (fun bx => bx.1.filterMap (fun a => f i a bx.2))).map
           (fun b => Multiset.ofList b) <+: _
       refine List.IsPrefix.trans ?_ ((zip_prefix (hb i) (ht i)).map _)
       rw [map_ofList_batches
           (fun bx => bx.1.filterMap (fun a => f i a bx.2))
           (fun bx => bx.1.filterMap (fun a => f i a bx.2))
           (fun bx => (Multiset.filterMap_coe _ _).symm),
         ← zip_map_left']⟩
  mapBatchesUnordered bs t f :=
    ⟨(fun i step => (Trace.zip (bs.val.1 i step) (t.val.1 i step)).map
        (fun bx => f i (Multiset.ofList bx.1) bx.2),
      {y | ∃ xb ∈ bs.val.2, ∃ xt ∈ t.val.2,
        y = (Values L mem).mapBatchesUnordered xb xt f}),
     fun T => by
       obtain ⟨vb, hvb, hb⟩ := bs.property T
       obtain ⟨vt, hvt, ht⟩ := t.property T
       refine ⟨(Values L mem).mapBatchesUnordered vb vt f,
         ⟨vb, hvb, vt, hvt, rfl⟩, fun i => ?_⟩
       refine List.IsPrefix.trans ?_ ((zip_prefix (hb i) (ht i)).map _)
       rw [zip_map_left', List.map_map]
       exact List.prefix_refl _⟩
  emitMultisetBatches t d :=
    ⟨(fun i step => emitLin (t.val.1 i step) (d i),
      {y | ∃ x ∈ t.val.2,
        y = (Values L mem).emitMultisetBatches x ()}),
     fun T => by
       obtain ⟨v, hv, hc⟩ := t.property T
       exact ⟨(Values L mem).emitMultisetBatches v (), ⟨v, hv, rfl⟩,
         fun i => (emitLin_map_ofList_prefix _ _).trans (hc i)⟩⟩
  allTicks {ℓ _β _ ord} bs :=
    ⟨(famHist (fun step i => (bs.val.1 i step).flatten),
      {y | ∃ x ∈ bs.val.2, y = (Values L mem).allTicks x}),
     fun T => by
       obtain ⟨k, _hk, he⟩ := famFreeze_eq_raw
         (fun step i => (bs.val.1 i step).flatten) T
       obtain ⟨v, hv, hc⟩ := bs.property k
       refine ⟨(Values L mem).allTicks v, ⟨v, hv, rfl⟩, fun i => ?_⟩
       have hview : famFreeze
           (fun step i => (bs.val.1 i step).flatten) T i
           = (bs.val.1 i k).flatten := congrFun he i
       cases ord with
       | totalOrder =>
         show famFreeze (fun step i => (bs.val.1 i step).flatten) T i
           <+: (v i).flatten
         rw [hview]
         exact prefix_flatten (hc i)
       | noOrder =>
         show Multiset.ofList (famFreeze
             (fun step i => (bs.val.1 i step).flatten) T i)
           ≤ (v i).sum
         rw [hview]
         exact flatten_le_sum (hc i)⟩
  emitBatches t :=
    ⟨(t.val.1, {y | ∃ x ∈ t.val.2, y = (Values L mem).emitBatches x}),
     fun T => by
       obtain ⟨v, hv, hc⟩ := t.property T
       exact ⟨(Values L mem).emitBatches v, ⟨v, hv, rfl⟩, hc⟩⟩
  emitBatchesUnordered t :=
    ⟨(t.val.1,
      {y | ∃ x ∈ t.val.2, y = (Values L mem).emitBatchesUnordered x}),
     fun T => by
       obtain ⟨v, hv, hc⟩ := t.property T
       exact ⟨(Values L mem).emitBatchesUnordered v, ⟨v, hv, rfl⟩,
         fun i => List.IsPrefix.map _ (hc i)⟩⟩
  fix_stream {ℓ α _ ord ret} _d body :=
    let cbot : {p : (Fin (mem ℓ) → StepHist α)
        × Set (Fin (mem ℓ) → PoolCarrier α ord ret) //
      StreamRel (mem ℓ) α ord ret p.1 p.2} :=
      ⟨(fun _i => StepHist.bot, {v | v = fun _i => PoolBot ord ret}),
       fun _t => ⟨fun _i => PoolBot ord ret, rfl,
         fun _i => ListLe.bot ord ret⟩⟩
    let sbody := fun (C : {p : (Fin (mem ℓ) → StepHist α)
        × Set (Fin (mem ℓ) → PoolCarrier α ord ret) //
        StreamRel (mem ℓ) α ord ret p.1 p.2}) =>
      (⟨(fun j => ((body C).val.1 j).shift, (body C).val.2),
        fun t => by
          cases t with
          | zero =>
            obtain ⟨v, hv, _⟩ := (body C).property 0
            exact ⟨v, hv, fun _i => ListLe.nil _ _ _⟩
          | succ u =>
            obtain ⟨v, hv, hc⟩ := (body C).property u
            exact ⟨v, hv, fun i => hc i⟩⟩ :
        {p : (Fin (mem ℓ) → StepHist α)
          × Set (Fin (mem ℓ) → PoolCarrier α ord ret) //
          StreamRel (mem ℓ) α ord ret p.1 p.2})
    ⟨(famHist (fun t i =>
        ((iterate sbody cbot (t + 1)).val.1 i).view t),
      {y | ∃ k, y ∈ (iterate sbody cbot k).val.2}),
     fun T => by
       obtain ⟨k, _hk, he⟩ := famFreeze_eq_raw
         (fun t i => ((iterate sbody cbot (t + 1)).val.1 i).view t) T
       obtain ⟨v, hv, hc⟩ := (iterate sbody cbot (k + 1)).property k
       refine ⟨v, ⟨k + 1, hv⟩, fun i => ?_⟩
       have hview : famFreeze (fun t i =>
           ((iterate sbody cbot (t + 1)).val.1 i).view t) T i
           = ((iterate sbody cbot (k + 1)).val.1 i).view k :=
         congrFun he i
       show ListLe ord ret (famFreeze (fun t i =>
         ((iterate sbody cbot (t + 1)).val.1 i).view t) T i) (v i)
       rw [hview]
       exact hc i⟩
  fix_tick {ℓ σ} _d body :=
    let cbot : {q : (Fin (mem ℓ) → Nat → Trace σ)
        × Set (TickV (mem ℓ) σ .unbounded) //
      TickSingRel (mem ℓ) σ .unbounded q.1 q.2} :=
      ⟨(fun _i _t => ([] : Trace σ),
        {v | v = fun _i => ([] : Trace σ)}),
       fun _T => ⟨fun _i => ([] : Trace σ), rfl,
         fun _i => List.nil_prefix⟩⟩
    let sbody := fun (C : {q : (Fin (mem ℓ) → Nat → Trace σ)
        × Set (TickV (mem ℓ) σ .unbounded) //
        TickSingRel (mem ℓ) σ .unbounded q.1 q.2}) =>
      (⟨(fun j u => match u with
          | 0 => ([] : Trace σ)
          | u + 1 => (body C).val.1 j u, (body C).val.2),
        fun t => by
          cases t with
          | zero =>
            obtain ⟨v, hv, _⟩ := (body C).property 0
            exact ⟨v, hv, fun _i => List.nil_prefix⟩
          | succ u =>
            obtain ⟨v, hv, hc⟩ := (body C).property u
            exact ⟨v, hv, fun i => hc i⟩⟩ :
        {q : (Fin (mem ℓ) → Nat → Trace σ)
          × Set (TickV (mem ℓ) σ .unbounded) //
          TickSingRel (mem ℓ) σ .unbounded q.1 q.2})
    ⟨(fun i t => (iterate sbody cbot (t + 1)).val.1 i t,
      {y | ∃ k, y ∈ (iterate sbody cbot k).val.2}),
     fun T => by
       obtain ⟨v, hv, hc⟩ := (iterate sbody cbot (T + 1)).property T
       exact ⟨v, ⟨T + 1, hv⟩, hc⟩⟩

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
    {ret : Retries} (g : σ → α → σ) (init : σ) (ok : FoldOkP ord ret g)
    (pool : PoolCarrier α ord ret) {d d' : CutDec α ord}
    (h : CutLe ord d d') :
    snapTrace ord ret g init ok pool d
      <+: snapTrace ord ret g init ok pool d' := by
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

/-- Derived batch cuts (ordered): the machine's tick segment sizes. -/
def batchOrdDerive {n : Nat} {α : Type} (p : Fin n → Nat → Bool) (T : Nat)
    (s : Fin n → StepHist α) : OrderedBatchCuts n :=
  fun i => (batchesFrom ((s i).view) (tickSteps (p i) T) 0).map List.length

/-- Derived snapshot cuts: the machine's tick-step reads. -/
def snapDerive {n : Nat} {α σ : Type} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (p : Fin n → Nat → Bool) (T : Nat)
    (sc : Fin n → SchedFold α σ) : SnapshotCuts n α ord :=
  fun i => snapCut ord ret
    ((tickSteps (p i) T).map (fun st => (sc i).src.view st))

/-- Derived order selection: the machine's arrival order. -/
def ordSelDerive {n : Nat} {α : Type} (T : Nat)
    (u : Fin n → StepHist α) : OrderSelection n α :=
  fun i => (u i).view T

theorem tickSteps_le_steps {p : Nat → Bool} {T T' : Nat} (h : T ≤ T') :
    tickSteps p T <+: tickSteps p T' := by
  induction T' with
  | zero => cases Nat.le_zero.mp h; exact List.prefix_refl _
  | succ T' ih =>
    rcases Nat.lt_or_ge T (T' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (tickSteps_prefix p T')
    · cases Nat.le_antisymm h hge; exact List.prefix_refl _

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

theorem snapDerive_mono {n : Nat} {α σ : Type} [DecidableEq α]
    (ord : StrOrd) (ret : Retries) (p : Fin n → Nat → Bool) {T T' : Nat}
    (h : T ≤ T') (sc : Fin n → SchedFold α σ) (i : Fin n) :
    CutLe ord (snapDerive ord ret p T sc i)
      (snapDerive ord ret p T' sc i) := by
  obtain ⟨e, he⟩ := tickSteps_le_steps (p := p i) h
  show CutLe ord
    (snapCut ord ret
      ((tickSteps (p i) T).map (fun st => (sc i).src.view st)))
    (snapCut ord ret
      ((tickSteps (p i) T').map (fun st => (sc i).src.view st)))
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

/-! ## Shared machine/values utility lemmas (moved from the retired
`Square.lean`; consumed by the coupling corner) -/

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

/-- Reading a snapshot output uniformly across the bound. -/
theorem tickVals_snapshot {L : Type} {mem : L → Nat} {ℓ : L}
    {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (b : SingBound σ) (vr : SingletonV (mem ℓ) α σ ord b)
    (cut : SnapshotCuts (mem ℓ) α ord) (i : Fin (mem ℓ)) :
    tickVals b ((Values L mem).snapshot (ℓ := ℓ) (ord := ord)
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

end HydroV2
