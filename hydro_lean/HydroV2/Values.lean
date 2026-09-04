import HydroV2.Sem

/-!
# The `Values` interpretation — the graded denotation

Decisions-as-inputs over the graded content carriers:

- a stream denotes, per member, its `PoolCarrier` — the marker-graded
  quotient of its content (`Grades.lean`): order and retry multiplicity
  are unobservable exactly where unpromised;
- a folded `Singleton` denotes its **read function**: from per-tick cut
  decisions (`CutDec` — prefix counts for ordered sources, increment
  multisets for unordered ones) to the trace of fold intermediates;
  illegal cuts block (count-legal at `ExactlyOnce`, membership-legal at
  `AtLeastOnce`) — legality is realizability;
- tick carriers denote realized values **across all ticks** (`Trace σ`);
  tick batches keep their grade (`Trace (Multiset α)` when unordered).
-/

namespace HydroV2

/-- The graded snapshot views of a fold's source. -/
def snapViews {α : Type _} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → PoolCarrier α ord ret →
      CutDec α ord → Trace (PoolCarrier α ord ret)
  | .totalOrder, .exactlyOnce => fun pool d => prefixCuts pool 0 d
  | .totalOrder, .atLeastOnce => fun pool d =>
      (prefixCuts pool.norm 0 d).map StutterSeq.mk
  | .noOrder, .exactlyOnce => fun pool d => snapshotCuts pool 0 d
  | .noOrder, .atLeastOnce => fun pool d =>
      (snapshotMemCuts pool.support.val 0 d).map RetryPool.mk

/-- The trace of fold intermediates a read decision exposes: the graded
fold of each realized view. -/
def snapTrace {α σ : Type _} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (g : σ → α → σ) (init : σ) (ok : FoldOk ord ret g)
    (pool : PoolCarrier α ord ret) (d : CutDec α ord) : Trace σ :=
  (snapViews ord ret pool d).map (PoolFold ord ret g init ok)

/-- Reads of an inflationary fold ascend along ticks, at every grade:
views chain (prefix / sub-multiset / membership-accumulation) and the
graded fold only moves up along the chain. -/
theorem snapTrace_ascending {α σ : Type _} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret g)
    (hinfl : ∀ s x, vo.le s (g s x))
    (pool : PoolCarrier α ord ret) (d : CutDec α ord) :
    Ascending vo (snapTrace ord ret g init ok pool d) := by
  intro t t' h ht'
  have hlen : t' < (snapViews ord ret pool d).length := by
    have := ht'
    unfold snapTrace at this
    rwa [List.length_map] at this
  have hlt : t < (snapViews ord ret pool d).length :=
    Nat.lt_of_le_of_lt h hlen
  have hmt : t < ((snapViews ord ret pool d).map
      (PoolFold ord ret g init ok)).length := by
    rwa [List.length_map]
  have hmt' : t' < ((snapViews ord ret pool d).map
      (PoolFold ord ret g init ok)).length := by
    rwa [List.length_map]
  show vo.le
    (((snapViews ord ret pool d).map (PoolFold ord ret g init ok))[t]'hmt)
    (((snapViews ord ret pool d).map
      (PoolFold ord ret g init ok))[t']'hmt')
  rw [List.getElem_map, List.getElem_map]
  -- per grade: the view chain + fold inflation along the chain
  cases ord <;> cases ret
  case totalOrder.exactlyOnce =>
    obtain ⟨e, he⟩ := prefixCuts_getElem_prefix (pool := pool) h hlen
    have key : ∀ (v v' : List α), v ++ e = v' →
        vo.le (v.foldl g init) (v'.foldl g init) := by
      intro v v' hv
      rw [← hv, List.foldl_append]
      exact vo.le_foldl hinfl e _
    exact key _ _ he
  case totalOrder.atLeastOnce =>
    have h1 : t < (prefixCuts pool.norm 0 d).length := by
      have := hlt
      unfold snapViews at this
      rwa [List.length_map] at this
    have h2 : t' < (prefixCuts pool.norm 0 d).length := by
      have := hlen
      unfold snapViews at this
      rwa [List.length_map] at this
    show vo.le (PoolFold .totalOrder .atLeastOnce g init ok
        (((prefixCuts pool.norm 0 d).map StutterSeq.mk)[t]'(by
          rwa [List.length_map])))
      (PoolFold .totalOrder .atLeastOnce g init ok
        (((prefixCuts pool.norm 0 d).map StutterSeq.mk)[t']'(by
          rwa [List.length_map])))
    rw [List.getElem_map, List.getElem_map]
    obtain ⟨e, he⟩ := prefixCuts_getElem_prefix (pool := pool.norm) h h2
    have key : ∀ (v v' : List α), v ++ e = v' →
        vo.le (v.foldl g init) (v'.foldl g init) := by
      intro v v' hv
      rw [← hv, List.foldl_append]
      exact vo.le_foldl hinfl e _
    exact key _ _ he
  case noOrder.exactlyOnce =>
    have hle := snapshotCuts_getElem_le (pool := pool) h hlen
    obtain ⟨e, he⟩ := Multiset.le_iff_exists_add.mp hle
    have key : ∀ (v v' : Multiset α), v' = v + e →
        vo.le (@Multiset.foldl α σ g ⟨fun s x y => ok s x y⟩ init v)
          (@Multiset.foldl α σ g ⟨fun s x y => ok s x y⟩ init v') := by
      intro v v' hv
      rw [hv, Multiset.foldl_add]
      exact multiset_le_foldl vo g (fun s x y => ok s x y) hinfl e _
    exact key _ _ he
  case noOrder.atLeastOnce =>
    have h1 : t < (snapshotMemCuts pool.support.val 0 d).length := by
      have := hlt
      unfold snapViews at this
      rwa [List.length_map] at this
    have h2 : t' < (snapshotMemCuts pool.support.val 0 d).length := by
      have := hlen
      unfold snapViews at this
      rwa [List.length_map] at this
    show vo.le (PoolFold .noOrder .atLeastOnce g init ok
        (((snapshotMemCuts pool.support.val 0 d).map RetryPool.mk)[t]'(by
          rwa [List.length_map])))
      (PoolFold .noOrder .atLeastOnce g init ok
        (((snapshotMemCuts pool.support.val 0 d).map RetryPool.mk)[t']'(by
          rwa [List.length_map])))
    rw [List.getElem_map, List.getElem_map]
    have hle := snapshotMemCuts_getElem_le
      (pool := pool.support.val) h h2
    obtain ⟨e, he⟩ := Multiset.le_iff_exists_add.mp hle
    have key : ∀ (v v' : Multiset α), v' = v + e →
        vo.le (RetryPool.fold g init ok.1 ok.2 (RetryPool.mk v))
          (RetryPool.fold g init ok.1 ok.2 (RetryPool.mk v')) := by
      intro v v' hv
      rw [RetryPool.fold_mk, RetryPool.fold_mk, hv, Multiset.foldl_add]
      exact multiset_le_foldl vo g (fun s x y => ok.1 s x y) hinfl e _
    exact key _ _ he

/-- The `Values` singleton carrier: the graded read function;
`Monotonic` bundles ascent of every read. -/
def SingletonV (n : Nat) (α σ : Type) [DecidableEq α] (ord : StrOrd) :
    SingBound σ → Type
  | .unbounded => Fin n → (CutDec α ord → Trace σ)
  | .monotonic vo =>
    Fin n → {f : CutDec α ord → Trace σ // ∀ d, Ascending vo (f d)}

/-- The `Values` tick carrier: realized values across all ticks. -/
def TickV (n : Nat) (σ : Type) : SingBound σ → Type
  | .unbounded => Fin n → Trace σ
  | .monotonic vo => Fin n → MonoTrace vo

set_option warn.classDefReducibility false in
/-- The graded denotation. -/
def Values (L : Type) (mem : L → Nat) : HydroSem L mem where
  Stream ℓ α _ ord ret := Fin (mem ℓ) → PoolCarrier α ord ret
  KeyedStream p c α _ ord ret :=
    Fin (mem p) → Fin (mem c) → PoolCarrier α ord ret
  Singleton ℓ α σ _ ord _ret b := SingletonV (mem ℓ) α σ ord b
  TickSingleton ℓ σ b := TickV (mem ℓ) σ b
  TickStream ℓ α _ ord ret :=
    Fin (mem ℓ) → Trace (PoolCarrier α ord ret)
  map {_ℓ _α _β _ _ ord} s f := fun i => mapPool (ord := ord) (f i) (s i)
  filterMap {_ℓ _α _β _ _ ord} s f := fun i =>
    filterMapPool (ord := ord) (f i) (s i)
  broadcast _ch s := fun _p j => s j
  demux {_c _p _α _ ord} _ch s addr := fun i j =>
    filterMapPool (ord := ord)
      (fun dx => if dx.1 = addr i then some dx.2 else none) (s j)
  values {_p _c _α _ ord ret} k :=
    match ord, ret, k with
    | .totalOrder, .exactlyOnce, k => fun i =>
        ((List.finRange _).map (fun j => (↑(k i j) : Multiset _))).sum
    | .noOrder, .exactlyOnce, k => fun i =>
        ((List.finRange _).map (fun j => k i j)).sum
    | .totalOrder, .atLeastOnce, k => fun i =>
        ((List.finRange _).map
          (fun j => RetryPool.mk (↑(k i j).norm : Multiset _))).foldl
          RetryPool.union (RetryPool.mk 0)
    | .noOrder, .atLeastOnce, k => fun i =>
        ((List.finRange _).map (fun j => k i j)).foldl
          RetryPool.union (RetryPool.mk 0)
  weaken_retries {_ℓ _α _ ord} s :=
    match ord, s with
    | .totalOrder, s => fun i => StutterSeq.mk (s i)
    | .noOrder, s => fun i => RetryPool.mk (s i)
  union {_ℓ _α _ ret} _ch a b :=
    match ret, a, b with
    | .exactlyOnce, a, b => fun i => a i + b i
    | .atLeastOnce, a, b => fun i => RetryPool.union (a i) (b i)
  assume_ordering u d := fun i => selectOrder (u i) (d i)
  fold {_ℓ _α _σ _ ord ret} g init ok s := fun i d =>
    snapTrace ord ret g init ok (s i) d
  fold_monotone {_ℓ _α _σ _ ord ret} vo g init ok hinfl s := fun i =>
    ⟨fun d => snapTrace ord ret g init ok (s i) d,
     fun d => snapTrace_ascending vo g init ok hinfl (s i) d⟩
  snapshot {_ℓ _α _σ _ _ord _ret b} s d :=
    match b, s with
    | .unbounded, s => fun i => s i (d i)
    | .monotonic _vo, s => fun i =>
      ⟨(s i).val (d i), (s i).property (d i)⟩
  batch s d := fun i => batchCuts (s i) 0 (d i)
  batch_ordered s d := fun i => sliceCuts (s i) 0 (d i)
  assume_ordering_batch bs d := fun i =>
    (Trace.zip (bs i) (d i)).map fun bd => selectOrder bd.1 bd.2
  mapBatchWith bs t f := fun i =>
    (Trace.zip (bs i) (t i)).map (fun bx => f i bx.1 bx.2)
  mapBatch bs f := fun i => (bs i).map (f i)
  mapBatchesWith bs t f := fun i =>
    (Trace.zip (bs i) (t i)).map
      (fun bx => bx.1.map (fun a => f i a bx.2))
  filterMapBatchesWith bs t f := fun i =>
    (Trace.zip (bs i) (t i)).map
      (fun bx => bx.1.filterMap (fun a => f i a bx.2))
  scan_batches_across_ticks bs t g init := fun i =>
    scanAcrossTicksTrace (fun s bt => g i s bt.1 bt.2) init (Trace.zip (bs i) (t i))
  fold_batches_across_ticks_monotone vo g init comm hinfl bs := fun i =>
    foldAcrossTicksMonotoneTrace vo
      (fun s b => @Multiset.foldl _ _ (g i)
        ⟨fun s x y => comm i s x y⟩ s b)
      init
      (fun s b => multiset_le_foldl vo (g i)
        (fun s x y => comm i s x y) (hinfl i) b s)
      (bs i)
  scan_batches_unordered_across_ticks bs t g init := fun i =>
    scanAcrossTicksTrace (fun s bt => g i s bt.1 bt.2) init (Trace.zip (bs i) (t i))
  scan_across_ticks t g init := fun i => scanAcrossTicksTrace (g i) init (t i)
  sample_every t d := fun i => StutterSeq.mk (sampleAtOpt (t i) (d i))
  timeout_snapshot _s d := fun i => d i
  source_interval_batch d := fun i => d i
  mapTick s f := fun i => (s i).map (f i)
  zipTick a b := fun i => Trace.zip (a i) (b i)
  fold_across_ticks_monotone vo g init hinfl s := fun i =>
    foldAcrossTicksMonotoneTrace vo (g i) init (hinfl i) (s i)
  mapMonotone _vo' m h hpres := fun i => (m i).map (h i) (hpres i)
  forgetBound m := fun i => (m i).vals
  defer init t := fun i => init :: t i
  allTicks {_ℓ _β _ ord} bs :=
    match ord, bs with
    | .totalOrder, bs => fun i => (bs i).flatten
    | .noOrder, bs => fun i => (bs i).sum
  mapBatchesUnordered bs t f := fun i =>
    (Trace.zip (bs i) (t i)).map (fun bx => f i bx.1 bx.2)
  emitBatches t := fun i => t i
  emitMultisetBatches t := fun i => t i
  emitBatchesUnordered t := fun i => (t i).map (fun l => Multiset.ofList l)
  fix_stream {_ℓ _α _ ord ret} _cyc fuel body :=
    iterate body (fun _i => PoolBot ord ret) fuel
  fix_tick _cyc fuel body := iterate body (fun _i => []) fuel


end HydroV2
