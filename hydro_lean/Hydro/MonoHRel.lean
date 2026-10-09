import Hydro.HydroRelLaws
import Hydro.MonoRel

/-!
# Hydro · the Flo-monotonicity relation (`HRel` instance)

⊑-preservation for every `Values` op (`vmono_*`, stated at the
instance spelling with heterogeneous inputs), packaged as an `HRel`
instance: the relation is the ⊑-diagonal at `Values` (pointwise
`PoolLe` on streams, trajectory prefix on tick carriers, read prefix
on fold singletons — the `MonoRel.lean` gluing relations), and the op
laws are the `vmono_*` lemmas. `HydroParam.lean`'s per-module free
theorems instantiated here ARE per-module Flo monotonicity — no
per-module generation, no walker.

Unindexed guarantee: `I := Unit`.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat}

section StreamOps

variable {ℓ p c : L}

theorem vmono_map {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd}
    {a b : (Values L mem).Stream ℓ α ord .exactlyOnce}
    (f : Fin (mem ℓ) → α → β)
    (h : ∀ i, PoolLe ord .exactlyOnce (a i) (b i)) :
    ∀ i, PoolLe ord .exactlyOnce
      ((Values L mem).map (ℓ := ℓ) a f i)
      ((Values L mem).map (ℓ := ℓ) b f i) :=
  fun i => pool_map_le (f i) (h i)

theorem vmono_filterMap {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd}
    {a b : (Values L mem).Stream ℓ α ord .exactlyOnce}
    (f : Fin (mem ℓ) → α → Option β)
    (h : ∀ i, PoolLe ord .exactlyOnce (a i) (b i)) :
    ∀ i, PoolLe ord .exactlyOnce
      ((Values L mem).filterMap (ℓ := ℓ) a f i)
      ((Values L mem).filterMap (ℓ := ℓ) b f i) :=
  fun i => pool_filterMap_le (f i) (h i)

theorem vmono_broadcast_closed {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}
    (dt : (Values L mem).TransportDec (mem p) (mem c))
    {a b : (Values L mem).Stream c α ord ret}
    (h : ∀ j, PoolLe ord ret (a j) (b j)) :
    ∀ i j, PoolLe ord ret
      ((Values L mem).broadcast_closed (c := c) (p := p) dt a i j)
      ((Values L mem).broadcast_closed (c := c) (p := p) dt b i j) :=
  fun _i j => h j

theorem vmono_demux {α : Type} [DecidableEq α] {ord : StrOrd}
    (dt : (Values L mem).TransportDec (mem p) (mem c))
    {a b : (Values L mem).Stream c (Nat × α) ord .exactlyOnce}
    (addr : Fin (mem p) → Nat)
    (h : ∀ j, PoolLe ord .exactlyOnce (a j) (b j)) :
    ∀ i j, PoolLe ord .exactlyOnce
      ((Values L mem).demux (c := c) (p := p) dt a addr i j)
      ((Values L mem).demux (c := c) (p := p) dt b addr i j) :=
  fun _i j => pool_filterMap_le _ (h j)

theorem vmono_values {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}
    {a b : (Values L mem).KeyedStream p c α ord ret}
    (h : ∀ i j, PoolLe ord ret (a i j) (b i j)) :
    ∀ i, PoolLe .noOrder ret
      ((Values L mem).values (p := p) (c := c) a i)
      ((Values L mem).values (p := p) (c := c) b i) := by
  cases ord <;> cases ret
  case totalOrder.exactlyOnce =>
    exact fun i => sum_le_sum (forall₂_map_map _ _ _
      (fun j => ((h i j).sublist).subperm))
  case totalOrder.atLeastOnce =>
    exact fun i => unionFold_le (forall₂_map_map _ _ _
      (fun j => norm_mem_le (h i j))) (RetryPool.le_refl _)
  case noOrder.exactlyOnce =>
    exact fun i => sum_le_sum (forall₂_map_map _ _ _ (fun j => h i j))
  case noOrder.atLeastOnce =>
    exact fun i => unionFold_le (forall₂_map_map _ _ _
      (fun j => h i j)) (RetryPool.le_refl _)

theorem vmono_weaken_retries {α : Type} [DecidableEq α] {ord : StrOrd}
    {a b : (Values L mem).Stream ℓ α ord .exactlyOnce}
    (h : ∀ i, PoolLe ord .exactlyOnce (a i) (b i)) :
    ∀ i, PoolLe ord .atLeastOnce
      ((Values L mem).weaken_retries (ℓ := ℓ) a i)
      ((Values L mem).weaken_retries (ℓ := ℓ) b i) := by
  cases ord
  case totalOrder =>
    intro i
    show StutterSeq.le (StutterSeq.mk (a i)) (StutterSeq.mk (b i))
    obtain ⟨e, he⟩ := h i
    rw [← he]
    exact StutterSeq.le_mk_append _ _
  case noOrder =>
    exact fun i x hx => Multiset.mem_of_le (h i) hx

theorem vmono_union {α : Type} [DecidableEq α] {ret : Retries}
    {a b c' d' : (Values L mem).Stream ℓ α .noOrder ret}
    (h1 : ∀ i, PoolLe .noOrder ret (a i) (b i))
    (h2 : ∀ i, PoolLe .noOrder ret (c' i) (d' i)) :
    ∀ i, PoolLe .noOrder ret
      ((Values L mem).union (ℓ := ℓ) a c' i)
      ((Values L mem).union (ℓ := ℓ) b d' i) := by
  cases ret
  case exactlyOnce =>
    exact fun i => le_trans (Multiset.add_le_add_right (h1 i))
      (Multiset.add_le_add_left (h2 i))
  case atLeastOnce =>
    exact fun i => RetryPool.union_le_union (h1 i) (h2 i)

theorem vmono_assume_ordering {α : Type} [DecidableEq α]
    {a b : (Values L mem).Stream ℓ α .noOrder .exactlyOnce}
    (d : (Values L mem).OrderSelDec (mem ℓ) α)
    (h : ∀ i, PoolLe .noOrder .exactlyOnce (a i) (b i)) :
    ∀ i, PoolLe .totalOrder .exactlyOnce
      ((Values L mem).assume_ordering (ℓ := ℓ) a d i)
      ((Values L mem).assume_ordering (ℓ := ℓ) b d i) :=
  fun i => selectOrder_le (h i) (d i)

theorem vmono_fold {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (ok : FoldOk ord ret g)
    {a b : (Values L mem).Stream ℓ α ord ret}
    (h : ∀ i, PoolLe ord ret (a i) (b i)) :
    ∀ i d, ((Values L mem).fold (ℓ := ℓ) g init ok a i) d
      <+: ((Values L mem).fold (ℓ := ℓ) g init ok b i) d :=
  fun i d => snapTrace_le g init ok (h i) d

theorem vmono_fold_monotone {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret g) (hinfl : ∀ s x, vo.le s (g s x))
    {a b : (Values L mem).Stream ℓ α ord ret}
    (h : ∀ i, PoolLe ord ret (a i) (b i)) :
    ∀ i d, ((Values L mem).fold_monotone (ℓ := ℓ) vo g init ok hinfl
        a i).val d
      <+: ((Values L mem).fold_monotone (ℓ := ℓ) vo g init ok hinfl
        b i).val d :=
  fun i d => snapTrace_le g init ok (h i) d

theorem vmono_snapshot_unbounded {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    {a b : (Values L mem).Singleton ℓ α σ ord ret .unbounded}
    (d : (Values L mem).SnapDec (mem ℓ) α ord)
    (h : ∀ i c, a i c <+: b i c) :
    ∀ i, ((Values L mem).snapshot (ℓ := ℓ) (ret := ret) a d i)
      <+: ((Values L mem).snapshot (ℓ := ℓ) (ret := ret) b d i) :=
  fun i => h i (d i)

theorem vmono_snapshot_monotonic {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {vo : ValueOrder σ}
    {a b : (Values L mem).Singleton ℓ α σ ord ret (.monotonic vo)}
    (d : (Values L mem).SnapDec (mem ℓ) α ord)
    (h : ∀ i c, (a i).val c <+: (b i).val c) :
    ∀ i, ((Values L mem).snapshot (ℓ := ℓ) (ret := ret) a d i)
      <+: ((Values L mem).snapshot (ℓ := ℓ) (ret := ret) b d i) :=
  fun i => h i (d i)

theorem vmono_batch {α : Type} [DecidableEq α]
    {a b : (Values L mem).Stream ℓ α .noOrder .exactlyOnce}
    (d : (Values L mem).BatchDec (mem ℓ) α)
    (h : ∀ i, PoolLe .noOrder .exactlyOnce (a i) (b i)) :
    ∀ i, ((Values L mem).batch (ℓ := ℓ) a d i)
      <+: ((Values L mem).batch (ℓ := ℓ) b d i) :=
  fun i => batchCuts_le (h i) 0 (d i)

theorem vmono_batch_ordered {α : Type} [DecidableEq α]
    {a b : (Values L mem).Stream ℓ α .totalOrder .exactlyOnce}
    (d : (Values L mem).OrdBatchDec (mem ℓ))
    (h : ∀ i, PoolLe .totalOrder .exactlyOnce (a i) (b i)) :
    ∀ i, ((Values L mem).batch_ordered (ℓ := ℓ) a d i)
      <+: ((Values L mem).batch_ordered (ℓ := ℓ) b d i) :=
  fun i => sliceCuts_le (h i) 0 (d i)

end StreamOps

section TickOps

variable {ℓ : L}

theorem vmono_sample_every {α : Type} [DecidableEq α]
    {a b : (Values L mem).Ticked ℓ (Option α)}
    (d : (Values L mem).SampleDec (mem ℓ))
    (h : ∀ i, a i <+: b i) :
    ∀ i, PoolLe .totalOrder .atLeastOnce
      ((Values L mem).sample_every (ℓ := ℓ) a d i)
      ((Values L mem).sample_every (ℓ := ℓ) b d i) := by
  intro i
  show StutterSeq.le (StutterSeq.mk (sampleAtOpt (a i) (d i)))
    (StutterSeq.mk (sampleAtOpt (b i) (d i)))
  obtain ⟨e, he⟩ := sampleAtOpt_prefix (h i) (d i)
  rw [← he]
  exact StutterSeq.le_mk_append _ _

theorem vmono_timeout_snapshot {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    {a b : (Values L mem).Stream ℓ α ord ret}
    (d : (Values L mem).TimerDec (mem ℓ)) :
    ∀ i, ((Values L mem).timeout_snapshot (ℓ := ℓ) (ord := ord)
        (ret := ret) a d i)
      <+: ((Values L mem).timeout_snapshot (ℓ := ℓ) (ord := ord)
        (ret := ret) b d i) :=
  fun _i => List.prefix_refl _

theorem vmono_source_interval_batch
    (d : (Values L mem).PulseDec (mem ℓ)) :
    ∀ i, ((Values L mem).source_interval_batch (ℓ := ℓ) d i)
      <+: ((Values L mem).source_interval_batch (ℓ := ℓ) d i) :=
  fun _i => List.prefix_refl _

theorem vmono_mapTick {α β : Type}
    {a b : (Values L mem).Ticked ℓ α}
    (f : Fin (mem ℓ) → α → β) (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).mapTick (ℓ := ℓ) a f i)
      <+: ((Values L mem).mapTick (ℓ := ℓ) b f i) :=
  fun i => (h i).map (f i)

theorem vmono_zipTick {α β : Type}
    {a b : (Values L mem).Ticked ℓ α}
    {c c' : (Values L mem).Ticked ℓ β}
    (h : ∀ i, a i <+: b i) (hc : ∀ i, c i <+: c' i) :
    ∀ i, ((Values L mem).zipTick (ℓ := ℓ) a c i)
      <+: ((Values L mem).zipTick (ℓ := ℓ) b c' i) :=
  fun i => zip_prefix (h i) (hc i)

theorem vmono_defer_tick {σ : Type} (init : σ)
    {a b : (Values L mem).Ticked ℓ σ}
    (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).defer_tick (ℓ := ℓ) init a i)
      <+: ((Values L mem).defer_tick (ℓ := ℓ) init b i) :=
  fun i => List.cons_prefix_cons.mpr ⟨rfl, h i⟩

theorem vmono_allTicks {β : Type} [DecidableEq β] {ord : StrOrd}
    {a b : (Values L mem).TickStream ℓ β ord .exactlyOnce}
    (h : ∀ i, a i <+: b i) :
    ∀ i, PoolLe ord .exactlyOnce
      ((Values L mem).allTicks (ℓ := ℓ) a i)
      ((Values L mem).allTicks (ℓ := ℓ) b i) := by
  cases ord
  case totalOrder => exact fun i => prefix_flatten (h i)
  case noOrder => exact fun i => sum_le_sum_of_prefix (h i)

theorem vmono_flattenOrdered {β : Type} [DecidableEq β]
    {a b : (Values L mem).Ticked ℓ (List β)}
    (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).flattenOrdered (ℓ := ℓ) a i)
      <+: ((Values L mem).flattenOrdered (ℓ := ℓ) b i) :=
  fun i => h i

theorem vmono_flattenUnordered {β : Type} [DecidableEq β]
    {a b : (Values L mem).Ticked ℓ (List β)}
    (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).flattenUnordered (ℓ := ℓ) a i)
      <+: ((Values L mem).flattenUnordered (ℓ := ℓ) b i) :=
  fun i => (h i).map _

end TickOps

/-! ## Knot ops (hetero bodies, same fuel) -/

section KnotOps

variable {ℓ : L}

theorem vmono_fix_stream {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (df : (Values L mem).FixDec)
    {bodyA bodyB : (Values L mem).Stream ℓ α ord ret →
      (Values L mem).Stream ℓ α ord ret}
    (hb : ∀ x y, (∀ i, PoolLe ord ret (x i) (y i)) →
      ∀ i, PoolLe ord ret (bodyA x i) (bodyB y i)) :
    ∀ i, PoolLe ord ret
      ((Values L mem).fix_stream (ℓ := ℓ) df bodyA i)
      ((Values L mem).fix_stream (ℓ := ℓ) df bodyB i) :=
  iterate_mono_param
    (R := fun (x y : Fin (mem ℓ) → PoolCarrier α ord ret) =>
      ∀ i, PoolLe ord ret (x i) (y i))
    (fun {x y} hxy => hb x y hxy)
    (fun _i => PoolLe.refl _ _ _) df

theorem vmono_fix_tick {σ : Type} (df : (Values L mem).FixDec)
    {bodyA bodyB : (Values L mem).Ticked ℓ σ →
      (Values L mem).Ticked ℓ σ}
    (hb : ∀ x y, (∀ i, x i <+: y i) →
      ∀ i, bodyA x i <+: bodyB y i) :
    ∀ i, ((Values L mem).fix_tick (ℓ := ℓ) df bodyA i)
      <+: ((Values L mem).fix_tick (ℓ := ℓ) df bodyB i) :=
  iterate_mono_param
    (R := fun (x y : TickV (mem ℓ) σ) => ∀ i, x i <+: y i)
    (fun {x y} hxy => hb x y hxy)
    (fun _i => List.nil_prefix) df

end KnotOps

/-! ## The `HRel` instance -/

/-- The Flo-monotonicity relation families: the ⊑-diagonal at
`Values` (decisions equal). -/
def monoC (L : Type) (mem : L → Nat) :
    HRelC Unit (Values L mem) (Values L mem) where
  streamRel _ a b := ∀ i, PoolLe _ _ (a i) (b i)
  keyedRel _ a b := ∀ i j, PoolLe _ _ (a i j) (b i j)
  singRel {_ _ _ _ _ _ b} _ x y := SingRel _ _ _ _ b x y
  tickedRel _ x y := TickRel _ _ x y
  -- one tick's content is a fixed value (ticked traces grow by whole
  -- ticks; within a tick there is nothing to ascend): equality
  boundedRel _ x y := x = y
  bsingRel _ x y := x = y
  tickedBoundedRel _ x y := TickRel _ _ x y
  tickStreamRel _ a b := ∀ i, a i <+: b i
  transportRel _ d d' := d = d'
  orderSelRel _ d d' := d = d'
  snapRel _ d d' := d = d'
  batchRel _ d d' := d = d'
  ordBatchRel _ d d' := d = d'
  sampleRel _ d d' := d = d'
  timerRel _ d d' := d = d'
  pulseRel _ d d' := d = d'
  fixRel _ d d' := d = d'

variable {L : Type} {mem : L → Nat}

/-- The uniform op-law discharge: expose the relations, substitute the
decision equalities, close by the matching `vmono_*` lemma
(grade-matched ops case on their bound first). -/
macro "mono_law_tac" : tactic =>
  `(tactic| (
    intro i
    intros
    try casesm* SingBound _
    all_goals simp only [monoC, SingRel, TickRel] at *
    all_goals subst_eqs
    all_goals solve
      | (apply vmono_map <;> assumption)
      | (apply vmono_filterMap <;> assumption)
      | (apply vmono_broadcast_closed <;> assumption)
      | (apply vmono_demux <;> assumption)
      | (apply vmono_values <;> assumption)
      | (apply vmono_weaken_retries <;> assumption)
      | (apply vmono_union <;> assumption)
      | (apply vmono_assume_ordering <;> assumption)
      | (apply vmono_fold <;> assumption)
      | (apply vmono_fold_monotone <;> assumption)
      | (apply vmono_snapshot_unbounded <;> assumption)
      | (apply vmono_snapshot_monotonic <;> assumption)
      | (apply vmono_batch <;> assumption)
      | (apply vmono_batch_ordered <;> assumption)
      | (apply vmono_sample_every <;> assumption)
      | (apply vmono_timeout_snapshot <;> assumption)
      | (apply vmono_source_interval_batch <;> assumption)
      | (apply vmono_mapTick <;> assumption)
      | (apply vmono_zipTick <;> assumption)
      | (apply vmono_defer_tick <;> assumption)
      | (apply vmono_allTicks <;> assumption)
      | (apply vmono_flattenOrdered <;> assumption)
      | (apply vmono_flattenUnordered <;> assumption)
      -- in-tick operators: one tick's content is related by equality
      -- (closures pointwise), wires by the ticked prefix
      | rfl
      | assumption
      | (rename_i hf; have hfe := funext hf; subst hfe; rfl)))

/-- The knot former laws: spend the strong-form body premise at the
(unit) index and feed `vmono_fix_*`. -/
theorem mono_law_fix_stream : HRel.law_fix_stream (monoC L mem) := by
  intro i ℓ α _ ord ret d d' hd b₁ b₂ hb
  have hd' : d = d' := hd
  subst hd'
  exact vmono_fix_stream d
    (fun x y hxy => hb () le_rfl x y (fun _ _ => hxy))

theorem mono_law_fix_tick : HRel.law_fix_tick (monoC L mem) := by
  intro i ℓ σ d d' hd b₁ b₂ hb
  have hd' : d = d' := hd
  subst hd'
  exact vmono_fix_tick d
    (fun x y hxy => hb () le_rfl x y (fun _ _ => hxy))

/-- At the diagonal, shaped in-tick tuples are related by equality. -/
theorem monoC_boundedOfRel_iff : (sh : TickShape) →
    (x y : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream sh) →
    ((monoC L mem).boundedOfRel () sh x y ↔ x = y)
  | .sing _, _, _ => Iff.rfl
  | .stream _ _ _ _, _, _ => Iff.rfl
  | .pair a b, x, y => by
    show ((monoC L mem).boundedOfRel () a x.1 y.1 ∧ (monoC L mem).boundedOfRel () b x.2 y.2)
      ↔ x = y
    rw [monoC_boundedOfRel_iff a, monoC_boundedOfRel_iff b]
    exact ⟨fun h => Prod.ext h.1 h.2, fun h => ⟨congrArg Prod.fst h, congrArg Prod.snd h⟩⟩

/-- At the diagonal, shaped wire tuples are related by leafwise prefix. -/
theorem monoC_tickedOfRel_iff {ℓ : L} : (sh : TickShape) →
    (x y : TickedOf (Values L mem).Ticked (Values L mem).TickStream ℓ sh) →
    ((monoC L mem).tickedOfRel () sh x y ↔ ValuesTick.Le sh x y)
  | .sing _, _, _ => Iff.rfl
  | .stream _ _ _ _, _, _ => Iff.rfl
  | .pair a b, x, y => by
    show ((monoC L mem).tickedOfRel () a x.1 y.1 ∧ (monoC L mem).tickedOfRel () b x.2 y.2)
      ↔ (ValuesTick.Le a x.1 y.1 ∧ ValuesTick.Le b x.2 y.2)
    rw [monoC_tickedOfRel_iff a, monoC_tickedOfRel_iff b]

theorem mono_law_tick_scan : HRel.law_tick_scan (monoC L mem) := by
  intro i ℓ sts ins outs x x' hx g g' hg init
  have hgg : g = g' := funext fun j => funext fun s => funext fun inp => by
    have := hg j s s ((monoC_boundedOfRel_iff sts s s).2 rfl) inp inp
      ((monoC_boundedOfRel_iff ins inp inp).2 rfl)
    exact Prod.ext ((monoC_boundedOfRel_iff sts _ _).1 this.1)
      ((monoC_boundedOfRel_iff outs _ _).1 this.2)
  subst hgg
  exact (monoC_tickedOfRel_iff outs _ _).2
    (ValuesTick.scan_le sts ins outs x x' ((monoC_tickedOfRel_iff ins x x').1 hx) g init)

/-- The Flo-monotonicity laws — one obligation per signature op, in
declaration order. -/
theorem monoLaws : HRel.Laws (monoC L mem) :=
  ⟨by mono_law_tac, -- map
   by mono_law_tac, -- filterMap
   by mono_law_tac, -- broadcast_closed
   by mono_law_tac, -- demux
   by mono_law_tac, -- values
   by mono_law_tac, -- weaken_retries
   by mono_law_tac, -- union
   by mono_law_tac, -- assume_ordering
   by mono_law_tac, -- fold
   by mono_law_tac, -- fold_monotone
   by mono_law_tac, -- snapshot
   by mono_law_tac, -- batch
   by mono_law_tac, -- batch_ordered
   by mono_law_tac, -- sample_every
   by mono_law_tac, -- timeout_snapshot
   by mono_law_tac, -- source_interval_batch
   by mono_law_tac, -- mapTick
   by mono_law_tac, -- zipTick
   by mono_law_tac, -- defer_tick
   by mono_law_tac, -- allTicks
   by mono_law_tac, -- flattenOrdered
   by mono_law_tac, -- flattenUnordered
   mono_law_fix_stream,
   mono_law_fix_tick,
   by mono_law_tac, -- bmap
   by mono_law_tac, -- bfilterMap
   by mono_law_tac, -- bflatMapOrdered
   by mono_law_tac, -- bflatMapUnordered
   by mono_law_tac, -- bofList
   by mono_law_tac, -- bcount
   by mono_law_tac, -- bfold
   by mono_law_tac, -- benumerate
   by mono_law_tac, -- bfirst
   by mono_law_tac, -- bcrossSingleton
   by mono_law_tac, -- bchain
   by mono_law_tac, -- bweakenOrder
   by mono_law_tac, -- bfilter
   by mono_law_tac, -- bkeyedFold
   by mono_law_tac, -- bkeys
   by mono_law_tac, -- bjoin
   by mono_law_tac, -- bantiJoin
   by mono_law_tac, -- bfilterNotIn
   by mono_law_tac, -- bmax
   by mono_law_tac, -- bfilterIf
   by mono_law_tac, -- bsPure
   by mono_law_tac, -- bsMap
   by mono_law_tac, -- bsZip
   by mono_law_tac, -- boMap
   by mono_law_tac, -- boUnwrapOr
   by mono_law_tac, -- boFilter
   by mono_law_tac, -- boIsSome
   mono_law_tick_scan⟩

end Hydro
