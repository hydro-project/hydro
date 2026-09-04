import Hydro.Values

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

Grading honesty: trajectory-prefix on ticked wires says nothing about
values at fresh ticks (a tick collection promises nothing across
executions); cross-tick value ascent is a contract fact of the producing
module (`ensures`), not this relation.
-/

namespace Hydro

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
def TickRel (n : Nat) (σ : Type) (t t' : TickV n σ) : Prop :=
  ∀ i : Fin n, t i <+: t' i

/-! ## Tick shapes at the diagonal: two denotation legs in leafwise prefix -/
namespace MonoTick
variable {L : Type} {mem : L → Nat}
abbrev Tk (mem : L → Nat) (ℓ : L) (τ : Type) : Type :=
  {p : TickV (mem ℓ) τ × TickV (mem ℓ) τ // TickRel (mem ℓ) τ p.1 p.2}
abbrev TS (mem : L → Nat) (ℓ : L) (α : Type) [DecidableEq α] (ord : StrOrd)
    (ret : Retries) : Type := Tk mem ℓ (PoolCarrier α ord ret)
def fst {ℓ : L} : (sh : TickShape) → TickedOf (Tk mem) (TS mem) ℓ sh →
    TickedOf (ValuesTick.Tk mem) (ValuesTick.TS mem) ℓ sh
  | .sing _, x => x.val.1
  | .stream _ _ _ _, x => x.val.1
  | .pair a b, x => (fst a x.1, fst b x.2)
def snd {ℓ : L} : (sh : TickShape) → TickedOf (Tk mem) (TS mem) ℓ sh →
    TickedOf (ValuesTick.Tk mem) (ValuesTick.TS mem) ℓ sh
  | .sing _, x => x.val.2
  | .stream _ _ _ _, x => x.val.2
  | .pair a b, x => (snd a x.1, snd b x.2)
theorem le {ℓ : L} : (sh : TickShape) → (x : TickedOf (Tk mem) (TS mem) ℓ sh) →
    ValuesTick.Le sh (fst sh x) (snd sh x)
  | .sing _, x => x.property
  | .stream _ _ _ _, x => x.property
  | .pair a b, x => ⟨le a x.1, le b x.2⟩
def mk {ℓ : L} : (sh : TickShape) →
    (a b : TickedOf (ValuesTick.Tk mem) (ValuesTick.TS mem) ℓ sh) →
    ValuesTick.Le sh a b → TickedOf (Tk mem) (TS mem) ℓ sh
  | .sing _, a, b, h => ⟨(a, b), h⟩
  | .stream _ _ _ _, a, b, h => ⟨(a, b), h⟩
  | .pair sa sb, a, b, h => (mk sa a.1 b.1 h.1, mk sb a.2 b.2 h.2)
end MonoTick

/-! ## The gluing instance -/

set_option warn.classDefReducibility false in
/-- The diagonal relational interpretation: each carrier is a related
pair of `Values` carriers; each op is the `Values` op on both components
plus the preservation proof — projections are definitionally the
`Values` runs. -/
def MonoRel (L : Type) (mem : L → Nat) : HydroSem L mem where
  TransportDec _ _ := Unit
  OrderSelDec n α := OrderSelection n α
  SnapDec n α ord := SnapshotCuts n α ord
  BatchDec n α := BatchCuts n α
  OrdBatchDec n := OrderedBatchCuts n
  SampleDec n := SampleTimes n
  TimerDec n := TimerVerdicts n
  PulseDec n := TimingPulses n
  FixDec := UnfoldFuel
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
  Ticked ℓ σ :=
    {p : TickV (mem ℓ) σ × TickV (mem ℓ) σ // TickRel (mem ℓ) σ p.1 p.2}
  BoundedStream α _ ord ret := PoolCarrier α ord ret
  BoundedSingleton σ := (Values L mem).BoundedSingleton σ
  -- `TickStream` is the signature default: pairs of tick-stream wires
  -- in trace prefix, `Ticked ℓ (PoolCarrier …)`
  map {_ℓ _α _β _ _ ord} s f :=
    ⟨(fun i => mapPool (ord := ord) (f i) (s.val.1 i),
      fun i => mapPool (ord := ord) (f i) (s.val.2 i)),
     fun i => pool_map_le (f i) (s.property i)⟩
  filterMap {_ℓ _α _β _ _ ord} s f :=
    ⟨(fun i => filterMapPool (ord := ord) (f i) (s.val.1 i),
      fun i => filterMapPool (ord := ord) (f i) (s.val.2 i)),
     fun i => pool_filterMap_le (f i) (s.property i)⟩
  broadcast_closed _d s :=
    ⟨(fun _p j => s.val.1 j, fun _p j => s.val.2 j),
     fun _p j => s.property j⟩
  demux {_c _p _α _ ord} _d s addr :=
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
  union {_ℓ _α _ ret} a b :=
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
      ⟨(fun i => (s.val.1 i).val (d i), fun i => (s.val.2 i).val (d i)),
       fun i => s.property i (d i)⟩
  batch s d :=
    ⟨(fun i => batchCuts (s.val.1 i) 0 (d i),
      fun i => batchCuts (s.val.2 i) 0 (d i)),
     fun i => batchCuts_le (s.property i) 0 (d i)⟩
  batch_ordered s d :=
    ⟨(fun i => sliceCuts (s.val.1 i) 0 (d i),
      fun i => sliceCuts (s.val.2 i) 0 (d i)),
     fun i => sliceCuts_le (s.property i) 0 (d i)⟩
  tick_scan sts ins outs t g init :=
    MonoTick.mk outs (ValuesTick.scan sts ins outs (MonoTick.fst ins t) g init)
      (ValuesTick.scan sts ins outs (MonoTick.snd ins t) g init)
      (ValuesTick.scan_le sts ins outs _ _ (MonoTick.le ins t) g init)
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
  defer_tick init t :=
    ⟨(fun i => init :: t.val.1 i, fun i => init :: t.val.2 i),
     fun i => List.cons_prefix_cons.mpr ⟨rfl, t.property i⟩⟩
  allTicks {_ℓ _β _ ord} bs :=
    match ord, bs with
    | .totalOrder, bs =>
      ⟨(fun i => (bs.val.1 i).flatten, fun i => (bs.val.2 i).flatten),
       fun i => prefix_flatten (bs.property i)⟩
    | .noOrder, bs =>
      ⟨(fun i => (bs.val.1 i).sum, fun i => (bs.val.2 i).sum),
       fun i => sum_le_sum_of_prefix (bs.property i)⟩
  flattenOrdered t :=
    ⟨(fun i => t.val.1 i, fun i => t.val.2 i), fun i => t.property i⟩
  flattenUnordered {_ℓ β _} t :=
    ⟨(fun i => List.map (fun l => (Multiset.ofList l : Multiset β))
        (t.val.1 i),
      fun i => List.map (fun l => (Multiset.ofList l : Multiset β))
        (t.val.2 i)),
     fun i => List.IsPrefix.map
       (fun l => (Multiset.ofList l : Multiset β)) (t.property i)⟩
  fix_stream {_ℓ _α _ ord ret} fuel body :=
    iterate body
      ⟨(fun _i => PoolBot ord ret, fun _i => PoolBot ord ret),
       fun _i => PoolLe.refl ord ret _⟩ fuel
  fix_tick fuel body :=
    iterate body
      ⟨((fun _i => []), (fun _i => [])), fun _i => List.nil_prefix⟩ fuel
  -- in-tick operators: one tick's content is a single value (the two
  -- legs of a wire are related tick-by-tick; within a tick the body
  -- runs on the denotation's quotient)
  bmap b f := (Values L mem).bmap b f
  bfilterMap b f := (Values L mem).bfilterMap b f
  bflatMapOrdered l f := (Values L mem).bflatMapOrdered l f
  bflatMapUnordered b f := (Values L mem).bflatMapUnordered b f
  bofList l := (Values L mem).bofList l
  bcount b := (Values L mem).bcount b
  bfold g init ok b := (Values L mem).bfold g init ok b
  benumerate l := (Values L mem).benumerate l
  bfirst l := (Values L mem).bfirst l
  bcrossSingleton b s := (Values L mem).bcrossSingleton b s
  bchain a b := (Values L mem).bchain a b
  bweakenOrder b := (Values L mem).bweakenOrder b
  bfilter b p := (Values L mem).bfilter b p
  bkeyedFold g init ok b := (Values L mem).bkeyedFold g init ok b
  bkeys e := (Values L mem).bkeys e
  bjoin a b := (Values L mem).bjoin a b
  bantiJoin a ks := (Values L mem).bantiJoin a ks
  bfilterNotIn a o := (Values L mem).bfilterNotIn a o
  bmax b := (Values L mem).bmax b
  bfilterIf b flag := (Values L mem).bfilterIf b flag
  bsPure v := (Values L mem).bsPure v
  bsMap s f := (Values L mem).bsMap s f
  bsZip a b := (Values L mem).bsZip a b
  boMap o f := (Values L mem).boMap o f
  boUnwrapOr o s := (Values L mem).boUnwrapOr o s
  boFilter o p := (Values L mem).boFilter o p
  boIsSome o := (Values L mem).boIsSome o

end Hydro
