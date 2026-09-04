import HydroV2.Values

/-!
# `MonoRel` — graded Flo monotonicity as a gluing instance

The ⊑-diagonal relational interpretation: carriers are **pairs of
`Values` carriers related by the type-assigned growth order** —
`PoolLe` on stream content (prefix / stutter-prefix / sub-multiset /
support-inclusion, by grade), per-cursor read-prefix on singletons,
trajectory-prefix on tick carriers — and every op re-proves the relation
for its outputs. A program text instantiated at `MonoRel` *is* its own
monotonicity proof: per-program Flo monotonicity is a projection, with
no induction and no per-program content.

Grading honesty: trajectory-prefix on tick singletons says nothing about
values at fresh ticks (`Unbounded` promises nothing across executions);
`Monotonic` value ascent is carried by the `MonoTrace` **value** (the
type), not this relation.
-/

namespace HydroV2

/-! ## Preservation helpers -/

theorem pool_map_le {α β : Type _} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd} (f : α → β) {a b : PoolCarrier α ord .exactlyOnce}
    (h : PoolLe ord .exactlyOnce a b) :
    PoolLe ord .exactlyOnce (mapPool (ord := ord) f a)
      (mapPool (ord := ord) f b) := by
  cases ord
  · exact h.map f
  · exact Multiset.map_le_map h

theorem pool_filterMap_le {α β : Type _} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd} (f : α → Option β)
    {a b : PoolCarrier α ord .exactlyOnce}
    (h : PoolLe ord .exactlyOnce a b) :
    PoolLe ord .exactlyOnce (filterMapPool (ord := ord) f a)
      (filterMapPool (ord := ord) f b) := by
  cases ord
  · exact prefix_filterMap f h
  · exact Multiset.filterMap_le_filterMap f h

theorem forall₂_map_map {ι α : Type _} {r : α → α → Prop} (xs : List ι)
    (f g : ι → α) (h : ∀ j, r (f j) (g j)) :
    List.Forall₂ r (xs.map f) (xs.map g) := by
  induction xs with
  | nil => exact .nil
  | cons x rest ih => exact .cons (h x) ih

/-- Sums of pointwise-≤ multiset lists are ≤. -/
theorem sum_le_sum {α : Type _} :
    ∀ {xs ys : List (Multiset α)},
      List.Forall₂ (· ≤ ·) xs ys → xs.sum ≤ ys.sum
  | [], [], .nil => le_refl _
  | _ :: _, _ :: _, .cons h hs => by
    rw [List.sum_cons, List.sum_cons]
    exact le_trans (Multiset.add_le_add_right h)
      (Multiset.add_le_add_left (sum_le_sum hs))

/-- Union folds of pointwise-`le` retry pools are `le`. -/
theorem unionFold_le {α : Type _} :
    ∀ {xs ys : List (RetryPool α)},
      List.Forall₂ RetryPool.le xs ys →
      ∀ {a b : RetryPool α}, RetryPool.le a b →
      RetryPool.le (xs.foldl RetryPool.union a)
        (ys.foldl RetryPool.union b)
  | [], [], .nil => fun h => h
  | _ :: _, _ :: _, .cons h hs => fun hab =>
    unionFold_le hs (RetryPool.union_le_union hab h)

/-- A stutter-prefix's entitlement is contained in the longer one's. -/
theorem norm_mem_le {α : Type _} [DecidableEq α] {a b : StutterSeq α}
    (h : StutterSeq.le a b) :
    RetryPool.le (RetryPool.mk (↑a.norm : Multiset α))
      (RetryPool.mk (↑b.norm : Multiset α)) :=
  fun x hx => by
    have hx' : x ∈ a.norm := by
      have : x ∈ (↑a.norm : Multiset _) := hx
      exact Multiset.mem_coe.mp this
    show x ∈ (↑b.norm : Multiset _)
    exact Multiset.mem_coe.mpr (h.sublist.subset hx')

/-- Membership cuts extend under support growth. -/
theorem snapshotMemCuts_le {α : Type _} [DecidableEq α]
    {pool pool' : Multiset α} (h : ∀ x ∈ pool, x ∈ pool')
    (acc : Multiset α) (d : List (Multiset α)) :
    snapshotMemCuts pool acc d <+: snapshotMemCuts pool' acc d := by
  induction d generalizing acc with
  | nil => exact List.prefix_refl _
  | cons b ds ih =>
    unfold snapshotMemCuts
    by_cases hb : ∀ x ∈ b, x ∈ pool
    · rw [if_pos hb, if_pos (fun x hx => h x (hb x hx))]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih _⟩
    · rw [if_neg hb]
      exact List.nil_prefix

/-- The graded snapshot views extend under content growth. -/
theorem snapViews_le {α : Type _} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {a b : PoolCarrier α ord ret}
    (h : PoolLe ord ret a b) (d : CutDec α ord) :
    snapViews ord ret a d <+: snapViews ord ret b d := by
  cases ord <;> cases ret
  case totalOrder.exactlyOnce => exact prefixCuts_le h 0 d
  case totalOrder.atLeastOnce =>
    exact (prefixCuts_le h 0 d).map StutterSeq.mk
  case noOrder.exactlyOnce => exact snapshotCuts_le h 0 d
  case noOrder.atLeastOnce =>
    refine (snapshotMemCuts_le ?_ 0 d).map RetryPool.mk
    intro x hx
    exact RetryPool.mem_support_val.mpr
      (h x (RetryPool.mem_support_val.mp hx))

/-- Reads of a fold extend under content growth (decision fixed). -/
theorem snapTrace_le {α σ : Type _} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (ok : FoldOk ord ret g)
    {a b : PoolCarrier α ord ret} (h : PoolLe ord ret a b)
    (d : CutDec α ord) :
    snapTrace ord ret g init ok a d <+: snapTrace ord ret g init ok b d :=
  (snapViews_le h d).map _

/-! ## The relations at singleton and tick carriers -/

/-- Read-prefix at every cursor decision. -/
def SingRel (n : Nat) (α σ : Type) [DecidableEq α] (ord : StrOrd) :
    (b : SingBound σ) → SingletonV n α σ ord b →
      SingletonV n α σ ord b → Prop
  | .unbounded => fun f f' => ∀ (i : Fin n) (d : CutDec α ord),
      f i d <+: f' i d
  | .monotonic _vo => fun f f' => ∀ (i : Fin n) (d : CutDec α ord),
      (f i).val d <+: (f' i).val d

/-- Trajectory prefix. -/
def TickRel (n : Nat) (σ : Type) :
    (b : SingBound σ) → TickV n σ b → TickV n σ b → Prop
  | .unbounded => fun t t' => ∀ i : Fin n, t i <+: t' i
  | .monotonic _vo => fun m m' => ∀ i : Fin n, (m i).vals <+: (m' i).vals

/-! ## The gluing instance -/

set_option warn.classDefReducibility false in
/-- The diagonal relational interpretation: each carrier is a related
pair of `Values` carriers; each op is the `Values` op on both components
plus the preservation proof — projections are definitionally the
`Values` runs. -/
def MonoRel (L : Type) (mem : L → Nat) : HydroSem L mem where
  Stream ℓ α _ ord ret :=
    {p : (Fin (mem ℓ) → PoolCarrier α ord ret)
        × (Fin (mem ℓ) → PoolCarrier α ord ret) //
      ∀ i, PoolLe ord ret (p.1 i) (p.2 i)}
  KeyedStream p c α _ ord ret :=
    {q : (Fin (mem p) → Fin (mem c) → PoolCarrier α ord ret)
        × (Fin (mem p) → Fin (mem c) → PoolCarrier α ord ret) //
      ∀ i j, PoolLe ord ret (q.1 i j) (q.2 i j)}
  Singleton ℓ α σ _ ord _ret b :=
    {p : SingletonV (mem ℓ) α σ ord b × SingletonV (mem ℓ) α σ ord b //
      SingRel (mem ℓ) α σ ord b p.1 p.2}
  TickSingleton ℓ σ b :=
    {p : TickV (mem ℓ) σ b × TickV (mem ℓ) σ b //
      TickRel (mem ℓ) σ b p.1 p.2}
  TickStream ℓ α _ ord ret :=
    {p : (Fin (mem ℓ) → Trace (PoolCarrier α ord ret))
        × (Fin (mem ℓ) → Trace (PoolCarrier α ord ret)) //
      ∀ i, p.1 i <+: p.2 i}
  map {_ℓ _α _β _ _ ord} s f :=
    ⟨(fun i => mapPool (ord := ord) (f i) (s.val.1 i),
      fun i => mapPool (ord := ord) (f i) (s.val.2 i)),
     fun i => pool_map_le (f i) (s.property i)⟩
  filterMap {_ℓ _α _β _ _ ord} s f :=
    ⟨(fun i => filterMapPool (ord := ord) (f i) (s.val.1 i),
      fun i => filterMapPool (ord := ord) (f i) (s.val.2 i)),
     fun i => pool_filterMap_le (f i) (s.property i)⟩
  broadcast _ch s :=
    ⟨(fun _p j => s.val.1 j, fun _p j => s.val.2 j),
     fun _p j => s.property j⟩
  demux {_c _p _α _ ord} _ch s addr :=
    ⟨(fun i j => filterMapPool (ord := ord)
        (fun dx => if dx.1 = addr i then some dx.2 else none) (s.val.1 j),
      fun i j => filterMapPool (ord := ord)
        (fun dx => if dx.1 = addr i then some dx.2 else none) (s.val.2 j)),
     fun i j => pool_filterMap_le _ (s.property j)⟩
  values {_p _c _α _ ord ret} k :=
    match ord, ret, k with
    | .totalOrder, .exactlyOnce, k =>
      ⟨(fun i => ((List.finRange _).map
          (fun j => (↑(k.val.1 i j) : Multiset _))).sum,
        fun i => ((List.finRange _).map
          (fun j => (↑(k.val.2 i j) : Multiset _))).sum),
       fun i => sum_le_sum (forall₂_map_map _ _ _
         (fun j => ((k.property i j).sublist).subperm))⟩
    | .noOrder, .exactlyOnce, k =>
      ⟨(fun i => ((List.finRange _).map (fun j => k.val.1 i j)).sum,
        fun i => ((List.finRange _).map (fun j => k.val.2 i j)).sum),
       fun i => sum_le_sum (forall₂_map_map _ _ _
         (fun j => k.property i j))⟩
    | .totalOrder, .atLeastOnce, k =>
      ⟨(fun i => ((List.finRange _).map
          (fun j => RetryPool.mk (↑(k.val.1 i j).norm : Multiset _))).foldl
          RetryPool.union (RetryPool.mk 0),
        fun i => ((List.finRange _).map
          (fun j => RetryPool.mk (↑(k.val.2 i j).norm : Multiset _))).foldl
          RetryPool.union (RetryPool.mk 0)),
       fun i => by
         exact unionFold_le (forall₂_map_map _ _ _
           (fun j => norm_mem_le (k.property i j)))
           (RetryPool.le_refl _)⟩
    | .noOrder, .atLeastOnce, k =>
      ⟨(fun i => ((List.finRange _).map (fun j => k.val.1 i j)).foldl
          RetryPool.union (RetryPool.mk 0),
        fun i => ((List.finRange _).map (fun j => k.val.2 i j)).foldl
          RetryPool.union (RetryPool.mk 0)),
       fun i => by
         exact unionFold_le (forall₂_map_map _ _ _
           (fun j => k.property i j)) (RetryPool.le_refl _)⟩
  weaken_retries {_ℓ _α _ ord} s :=
    match ord, s with
    | .totalOrder, s =>
      ⟨(fun i => StutterSeq.mk (s.val.1 i),
        fun i => StutterSeq.mk (s.val.2 i)),
       fun i => by
         show StutterSeq.le (StutterSeq.mk (s.val.1 i))
           (StutterSeq.mk (s.val.2 i))
         obtain ⟨e, he⟩ := s.property i
         rw [← he]
         exact StutterSeq.le_mk_append _ _⟩
    | .noOrder, s =>
      ⟨(fun i => RetryPool.mk (s.val.1 i),
        fun i => RetryPool.mk (s.val.2 i)),
       fun i => fun x hx => Multiset.mem_of_le (s.property i) hx⟩
  union {_ℓ _α _ ret} _ch a b :=
    match ret, a, b with
    | .exactlyOnce, a, b =>
      ⟨(fun i => a.val.1 i + b.val.1 i, fun i => a.val.2 i + b.val.2 i),
       fun i => le_trans (Multiset.add_le_add_right (a.property i))
         (Multiset.add_le_add_left (b.property i))⟩
    | .atLeastOnce, a, b =>
      ⟨(fun i => RetryPool.union (a.val.1 i) (b.val.1 i),
        fun i => RetryPool.union (a.val.2 i) (b.val.2 i)),
       fun i => RetryPool.union_le_union (a.property i) (b.property i)⟩
  assume_ordering u d :=
    ⟨(fun i => selectOrder (u.val.1 i) (d i),
      fun i => selectOrder (u.val.2 i) (d i)),
     fun i => selectOrder_le (u.property i) (d i)⟩
  fold {_ℓ _α _σ _ ord ret} g init ok s :=
    ⟨(fun i d => snapTrace ord ret g init ok (s.val.1 i) d,
      fun i d => snapTrace ord ret g init ok (s.val.2 i) d),
     fun i d => snapTrace_le g init ok (s.property i) d⟩
  fold_monotone {_ℓ _α _σ _ ord ret} vo g init ok hinfl s :=
    ⟨(fun i => ⟨fun d => snapTrace ord ret g init ok (s.val.1 i) d,
        fun d => snapTrace_ascending vo g init ok hinfl (s.val.1 i) d⟩,
      fun i => ⟨fun d => snapTrace ord ret g init ok (s.val.2 i) d,
        fun d => snapTrace_ascending vo g init ok hinfl (s.val.2 i) d⟩),
     fun i d => snapTrace_le g init ok (s.property i) d⟩
  snapshot {_ℓ _α _σ _ _ord _ret b} s d :=
    match b, s with
    | .unbounded, s =>
      ⟨(fun i => s.val.1 i (d i), fun i => s.val.2 i (d i)),
       fun i => s.property i (d i)⟩
    | .monotonic _vo, s =>
      ⟨(fun i => ⟨(s.val.1 i).val (d i), (s.val.1 i).property (d i)⟩,
        fun i => ⟨(s.val.2 i).val (d i), (s.val.2 i).property (d i)⟩),
       fun i => s.property i (d i)⟩
  batch s d :=
    ⟨(fun i => batchCuts (s.val.1 i) 0 (d i),
      fun i => batchCuts (s.val.2 i) 0 (d i)),
     fun i => batchCuts_le (s.property i) 0 (d i)⟩
  batch_ordered s d :=
    ⟨(fun i => sliceCuts (s.val.1 i) 0 (d i),
      fun i => sliceCuts (s.val.2 i) 0 (d i)),
     fun i => sliceCuts_le (s.property i) 0 (d i)⟩
  assume_ordering_batch bs d :=
    ⟨(fun i => (Trace.zip (bs.val.1 i) (d i)).map
        (fun bd => selectOrder bd.1 bd.2),
      fun i => (Trace.zip (bs.val.2 i) (d i)).map
        (fun bd => selectOrder bd.1 bd.2)),
     fun i => (zip_prefix (bs.property i) (List.prefix_refl _)).map _⟩
  mapBatchWith bs t f :=
    ⟨(fun i => (Trace.zip (bs.val.1 i) (t.val.1 i)).map
        (fun bx => f i bx.1 bx.2),
      fun i => (Trace.zip (bs.val.2 i) (t.val.2 i)).map
        (fun bx => f i bx.1 bx.2)),
     fun i => (zip_prefix (bs.property i) (t.property i)).map _⟩
  mapBatch bs f :=
    ⟨(fun i => (bs.val.1 i).map (f i), fun i => (bs.val.2 i).map (f i)),
     fun i => (bs.property i).map (f i)⟩
  mapBatchesWith bs t f :=
    ⟨(fun i => (Trace.zip (bs.val.1 i) (t.val.1 i)).map
        (fun bx => bx.1.map (fun a => f i a bx.2)),
      fun i => (Trace.zip (bs.val.2 i) (t.val.2 i)).map
        (fun bx => bx.1.map (fun a => f i a bx.2))),
     fun i => (zip_prefix (bs.property i) (t.property i)).map _⟩
  scan_batches_across_ticks bs t g init :=
    ⟨(fun i => scanAcrossTicksTrace (fun s bt => g i s bt.1 bt.2) init
        (Trace.zip (bs.val.1 i) (t.val.1 i)),
      fun i => scanAcrossTicksTrace (fun s bt => g i s bt.1 bt.2) init
        (Trace.zip (bs.val.2 i) (t.val.2 i))),
     fun i => scanAcrossTicksTrace_prefix _ init
       (zip_prefix (bs.property i) (t.property i))⟩
  fold_batches_across_ticks_monotone vo g init comm hinfl bs :=
    ⟨(fun i => foldAcrossTicksMonotoneTrace vo
        (fun s b => @Multiset.foldl _ _ (g i)
          ⟨fun s x y => comm i s x y⟩ s b) init
        (fun s b => multiset_le_foldl vo (g i)
          (fun s x y => comm i s x y) (hinfl i) b s)
        (bs.val.1 i),
      fun i => foldAcrossTicksMonotoneTrace vo
        (fun s b => @Multiset.foldl _ _ (g i)
          ⟨fun s x y => comm i s x y⟩ s b) init
        (fun s b => multiset_le_foldl vo (g i)
          (fun s x y => comm i s x y) (hinfl i) b s)
        (bs.val.2 i)),
     fun i => foldAcrossTicksTrace_prefix _ init (bs.property i)⟩
  scan_batches_unordered_across_ticks bs t g init :=
    ⟨(fun i => scanAcrossTicksTrace (fun s bt => g i s bt.1 bt.2) init
        (Trace.zip (bs.val.1 i) (t.val.1 i)),
      fun i => scanAcrossTicksTrace (fun s bt => g i s bt.1 bt.2) init
        (Trace.zip (bs.val.2 i) (t.val.2 i))),
     fun i => scanAcrossTicksTrace_prefix _ init
       (zip_prefix (bs.property i) (t.property i))⟩
  scan_across_ticks t g init :=
    ⟨(fun i => scanAcrossTicksTrace (g i) init (t.val.1 i),
      fun i => scanAcrossTicksTrace (g i) init (t.val.2 i)),
     fun i => scanAcrossTicksTrace_prefix _ init (t.property i)⟩
  sample_every t d :=
    ⟨(fun i => StutterSeq.mk (sampleAtOpt (t.val.1 i) (d i)),
      fun i => StutterSeq.mk (sampleAtOpt (t.val.2 i) (d i))),
     fun i => by
       show StutterSeq.le (StutterSeq.mk (sampleAtOpt (t.val.1 i) (d i)))
         (StutterSeq.mk (sampleAtOpt (t.val.2 i) (d i)))
       obtain ⟨e, he⟩ := sampleAtOpt_prefix (t.property i) (d i)
       rw [← he]
       exact StutterSeq.le_mk_append _ _⟩
  timeout_snapshot _s d :=
    ⟨(fun i => d i, fun i => d i), fun i => List.prefix_refl _⟩
  source_interval_batch d :=
    ⟨(fun i => d i, fun i => d i), fun i => List.prefix_refl _⟩
  mapTick s f :=
    ⟨(fun i => (s.val.1 i).map (f i), fun i => (s.val.2 i).map (f i)),
     fun i => (s.property i).map (f i)⟩
  zipTick a b :=
    ⟨(fun i => Trace.zip (a.val.1 i) (b.val.1 i),
      fun i => Trace.zip (a.val.2 i) (b.val.2 i)),
     fun i => zip_prefix (a.property i) (b.property i)⟩
  fold_across_ticks_monotone vo g init hinfl s :=
    ⟨(fun i => foldAcrossTicksMonotoneTrace vo (g i) init (hinfl i) (s.val.1 i),
      fun i => foldAcrossTicksMonotoneTrace vo (g i) init (hinfl i) (s.val.2 i)),
     fun i => foldAcrossTicksTrace_prefix (g i) init (s.property i)⟩
  mapMonotone _vo' m h hpres :=
    ⟨(fun i => (m.val.1 i).map (h i) (hpres i),
      fun i => (m.val.2 i).map (h i) (hpres i)),
     fun i => (m.property i).map (h i)⟩
  forgetBound m :=
    ⟨(fun i => (m.val.1 i).vals, fun i => (m.val.2 i).vals),
     fun i => m.property i⟩
  defer init t :=
    ⟨(fun i => init :: t.val.1 i, fun i => init :: t.val.2 i),
     fun i => List.cons_prefix_cons.mpr ⟨rfl, t.property i⟩⟩
  filterMapBatchesWith bs t f :=
    ⟨(fun i => (Trace.zip (bs.val.1 i) (t.val.1 i)).map
        (fun bx => bx.1.filterMap (fun a => f i a bx.2)),
      fun i => (Trace.zip (bs.val.2 i) (t.val.2 i)).map
        (fun bx => bx.1.filterMap (fun a => f i a bx.2))),
     fun i => (zip_prefix (bs.property i) (t.property i)).map _⟩
  allTicks {_ℓ _β _ ord} bs :=
    match ord, bs with
    | .totalOrder, bs =>
      ⟨(fun i => (bs.val.1 i).flatten, fun i => (bs.val.2 i).flatten),
       fun i => prefix_flatten (bs.property i)⟩
    | .noOrder, bs =>
      ⟨(fun i => (bs.val.1 i).sum, fun i => (bs.val.2 i).sum),
       fun i => sum_le_sum_of_prefix (bs.property i)⟩
  mapBatchesUnordered bs t f :=
    ⟨(fun i => (Trace.zip (bs.val.1 i) (t.val.1 i)).map
        (fun bx => f i bx.1 bx.2),
      fun i => (Trace.zip (bs.val.2 i) (t.val.2 i)).map
        (fun bx => f i bx.1 bx.2)),
     fun i => (zip_prefix (bs.property i) (t.property i)).map _⟩
  emitBatches t :=
    ⟨(fun i => t.val.1 i, fun i => t.val.2 i), fun i => t.property i⟩
  emitMultisetBatches t :=
    ⟨(fun i => t.val.1 i, fun i => t.val.2 i), fun i => t.property i⟩
  emitBatchesUnordered {_ℓ β _} t :=
    ⟨(fun i => List.map (fun l => (Multiset.ofList l : Multiset β))
        (t.val.1 i),
      fun i => List.map (fun l => (Multiset.ofList l : Multiset β))
        (t.val.2 i)),
     fun i => List.IsPrefix.map
       (fun l => (Multiset.ofList l : Multiset β)) (t.property i)⟩
  fix_stream {_ℓ _α _ ord ret} _cyc fuel body :=
    iterate body
      ⟨(fun _i => PoolBot ord ret, fun _i => PoolBot ord ret),
       fun _i => PoolLe.refl ord ret _⟩ fuel
  fix_tick _cyc fuel body :=
    iterate body
      ⟨((fun _i => []), (fun _i => [])), fun _i => List.nil_prefix⟩ fuel

end HydroV2
