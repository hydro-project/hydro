import Hydro.MonoRel

/-!
# `Values` monotonicity, per operation (`vmono_*`)

Each `vmono_*` theorem states that one `Values` op preserves the growth order on its carrier:
pointwise `PoolLe` on streams, `KeyedPool.Le` on keyed streams, trajectory prefix on tick
carriers, read prefix on fold singletons — the relations of `Hydro/MonoRel.lean`, stated at
the instance spelling with two independent inputs. The knot generator's monotonicity
resolver (`K_mono₁` in `Hydro/HydroGenKnot.lean`) applies these as leaves when it threads a
knot body's monotonicity through its operators. Module-level monotonicity follows from
instantiating the program at `MonoRel`, not from these lemmas.
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
    ∀ i, KeyedPool.Le
      ((Values L mem).broadcast_closed (c := c) (p := p) dt a i)
      ((Values L mem).broadcast_closed (c := c) (p := p) dt b i) :=
  fun _i => KeyedPool.transport_le h

theorem vmono_demux {α : Type} [DecidableEq α] {ord : StrOrd}
    (dt : (Values L mem).TransportDec (mem p) (mem c))
    {a b : (Values L mem).Stream c (Nat × α) ord .exactlyOnce}
    (addr : Fin (mem p) → Nat)
    (h : ∀ j, PoolLe ord .exactlyOnce (a j) (b j)) :
    ∀ i, KeyedPool.Le
      ((Values L mem).demux (c := c) (p := p) dt a addr i)
      ((Values L mem).demux (c := c) (p := p) dt b addr i) :=
  fun _i => KeyedPool.transport_le (fun j => pool_filterMap_le _ (h j))

theorem vmono_values {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {ret : Retries}
    {a b : (Values L mem).KeyedStream ℓ K V ord ret}
    (h : ∀ i, KeyedPool.Le (a i) (b i)) :
    ∀ i, PoolLe .noOrder ret
      ((Values L mem).values (ℓ := ℓ) a i)
      ((Values L mem).values (ℓ := ℓ) b i) :=
  fun i => KeyedPool.values_le (h i)

theorem vmono_into_keyed {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {a b : (Values L mem).Stream ℓ (K × V) ord .exactlyOnce}
    (h : ∀ i, PoolLe ord .exactlyOnce (a i) (b i)) :
    ∀ i, KeyedPool.Le
      ((Values L mem).into_keyed (ℓ := ℓ) a i)
      ((Values L mem).into_keyed (ℓ := ℓ) b i) :=
  fun i => KeyedPool.ofPool_le (h i)

theorem vmono_kassume_ordering {K V : Type} [DecidableEq K] [DecidableEq V]
    {a b : (Values L mem).KeyedStream ℓ K V .noOrder .exactlyOnce}
    (d : (Values L mem).OrderSelDec (mem ℓ) (K × V))
    (h : ∀ i, KeyedPool.Le (a i) (b i)) :
    ∀ i, KeyedPool.Le
      ((Values L mem).kassume_ordering (ℓ := ℓ) a d i)
      ((Values L mem).kassume_ordering (ℓ := ℓ) b d i) :=
  fun i => KeyedPool.ofList_prefix (selectOrder_le (KeyedPool.toMultiset_le (h i)) (d i))

theorem vmono_kfold_early_stop {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    {a b : (Values L mem).KeyedStream ℓ K V .totalOrder .exactlyOnce}
    (f : A → V → A × Bool) (init : A)
    (h : ∀ i, KeyedPool.Le (a i) (b i)) :
    ∀ i d, ((Values L mem).kfold_early_stop (ℓ := ℓ) a f init i) d
      <+: ((Values L mem).kfold_early_stop (ℓ := ℓ) b f init i) d :=
  fun i d => KeyedPool.kviews_map_readAt_le (h i) (KeyedMap.ofFoldEarlyStop f init) 0 d

theorem vmono_kget_max_key {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd} {ret : Retries}
    {a b : (Values L mem).KeyedSingleton ℓ α K V ord ret .boundedValue}
    (h : ∀ i d, a i d <+: b i d) :
    ∀ i d, ((Values L mem).kget_max_key (ℓ := ℓ) (ord := ord) (ret := ret) a i) d
      <+: ((Values L mem).kget_max_key (ℓ := ℓ) (ord := ord) (ret := ret) b i) d :=
  fun i d => (h i d).map _

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
    {ret : Retries} (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk ord ret Pre g)
    {a b : (Values L mem).Stream ℓ α ord ret}
    (h : ∀ i, PoolLe ord ret (a i) (b i)) :
    ∀ i d, ((Values L mem).fold (ℓ := ℓ) g init Pre ok a i) d
      <+: ((Values L mem).fold (ℓ := ℓ) g init Pre ok b i) d :=
  fun i d => snapTrace_le Pre g init ok (h i) d

theorem vmono_fold_monotone {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret (fun _ _ => True) g) (hinfl : ∀ s x, vo.le s (g s x))
    {a b : (Values L mem).Stream ℓ α ord ret}
    (h : ∀ i, PoolLe ord ret (a i) (b i)) :
    ∀ i d, ((Values L mem).fold_monotone (ℓ := ℓ) vo g init ok hinfl
        a i).val d
      <+: ((Values L mem).fold_monotone (ℓ := ℓ) vo g init ok hinfl
        b i).val d :=
  fun i d => snapTrace_le _ g init ok (h i) d

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

theorem vmono_batch {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {a b : (Values L mem).Stream ℓ α ord ret}
    (d : (Values L mem).BatchDec (mem ℓ) α ord ret)
    (h : ∀ i, PoolLe ord ret (a i) (b i)) :
    ∀ i, ((Values L mem).batch (ℓ := ℓ) a d i)
      <+: ((Values L mem).batch (ℓ := ℓ) b d i) := by
  cases ord <;> cases ret
  · exact fun i => sliceCuts_le (h i) 0 (d i)
  · exact fun i => stutterBatchCuts_le (h i) [] (d i)
  · exact fun i => batchCuts_le (h i) 0 (d i)
  · exact fun i => retryBatchCuts_le (h i) (d i)

end StreamOps

section TickOps

variable {ℓ : L}

theorem vmono_source_interval (delay : Fin (mem ℓ) → Nat) (period : Nat)
    (d : (Values L mem).IntervalDec (mem ℓ)) :
    ∀ i, PoolLe .totalOrder .atLeastOnce
      ((Values L mem).source_interval (ℓ := ℓ) delay period d i)
      ((Values L mem).source_interval (ℓ := ℓ) delay period d i) :=
  fun _i => StutterSeq.le_refl _

theorem vmono_firstTick {α : Type} [DecidableEq α] {ret : Retries}
    {a b : (Values L mem).AtomicStream ℓ α .totalOrder ret}
    (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).firstTick (ℓ := ℓ) a i)
      <+: ((Values L mem).firstTick (ℓ := ℓ) b i) :=
  fun i => (h i).map (poolHead? ret)

theorem vmono_mapTick {α β : Type}
    {a b : (Values L mem).Atomic ℓ α}
    (f : Fin (mem ℓ) → α → β) (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).mapTick (ℓ := ℓ) a f i)
      <+: ((Values L mem).mapTick (ℓ := ℓ) b f i) :=
  fun i => (h i).map (f i)

theorem vmono_zipTick {α β : Type}
    {a b : (Values L mem).Atomic ℓ α}
    {c c' : (Values L mem).Atomic ℓ β}
    (h : ∀ i, a i <+: b i) (hc : ∀ i, c i <+: c' i) :
    ∀ i, ((Values L mem).zipTick (ℓ := ℓ) a c i)
      <+: ((Values L mem).zipTick (ℓ := ℓ) b c' i) :=
  fun i => zip_prefix (h i) (hc i)

theorem vmono_defer_tick {σ : Type} (init : σ)
    {a b : (Values L mem).Atomic ℓ σ}
    (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).defer_tick (ℓ := ℓ) init a i)
      <+: ((Values L mem).defer_tick (ℓ := ℓ) init b i) :=
  fun i => List.cons_prefix_cons.mpr ⟨rfl, h i⟩

theorem vmono_allTicks {β : Type} [DecidableEq β] {ord : StrOrd}
    {a b : (Values L mem).AtomicStream ℓ β ord .exactlyOnce}
    (h : ∀ i, a i <+: b i) :
    ∀ i, PoolLe ord .exactlyOnce
      ((Values L mem).allTicks (ℓ := ℓ) a i)
      ((Values L mem).allTicks (ℓ := ℓ) b i) := by
  cases ord
  case totalOrder => exact fun i => prefix_flatten (h i)
  case noOrder => exact fun i => sum_le_sum_of_prefix (h i)

theorem vmono_flattenOrdered {β : Type} [DecidableEq β]
    {a b : (Values L mem).Atomic ℓ (List β)}
    (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).flattenOrdered (ℓ := ℓ) a i)
      <+: ((Values L mem).flattenOrdered (ℓ := ℓ) b i) :=
  fun i => h i

theorem vmono_flattenUnordered {β : Type} [DecidableEq β]
    {a b : (Values L mem).Atomic ℓ (List β)}
    (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).flattenUnordered (ℓ := ℓ) a i)
      <+: ((Values L mem).flattenUnordered (ℓ := ℓ) b i) :=
  fun i => (h i).map _

theorem vmono_tinto_keyed {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {a b : (Values L mem).AtomicStream ℓ (K × V) ord .exactlyOnce}
    (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).tinto_keyed (ℓ := ℓ) a i)
      <+: ((Values L mem).tinto_keyed (ℓ := ℓ) b i) :=
  fun i => (h i).map _

theorem vmono_tassume_ordering {α : Type} [DecidableEq α]
    {a b : (Values L mem).AtomicStream ℓ α .noOrder .exactlyOnce}
    (d : (Values L mem).TickOrderSelDec (mem ℓ) α)
    (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).tassume_ordering (ℓ := ℓ) a d i)
      <+: ((Values L mem).tassume_ordering (ℓ := ℓ) b d i) :=
  fun i => realizeTicks_prefix_left (h i) (d i)

theorem vmono_reduce_watermark {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V] [LinearOrder K]
    {ord : StrOrd}
    {a b : (Values L mem).AtomicKeyedStream ℓ K V ord .exactlyOnce}
    {w w' : (Values L mem).Atomic ℓ (Option K)}
    (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOk ord .exactlyOnce Pre (reduceStep f))
    (h : ∀ i, a i <+: b i) (hw : ∀ i, w i <+: w' i) :
    ∀ i, ((Values L mem).reduce_watermark (ℓ := ℓ) a w f Pre ok i)
      <+: ((Values L mem).reduce_watermark (ℓ := ℓ) b w' f Pre ok i) :=
  fun i => scanAcrossTicksTrace_prefix _ _ (zip_prefix (h i) (hw i))

/-! ## Keyed-stream and keyed-singleton ops -/

theorem vmono_kentries {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {a b : (Values L mem).KeyedStream ℓ K V ord .exactlyOnce}
    (h : ∀ i, KeyedPool.Le (a i) (b i)) :
    ∀ i, PoolLe .noOrder .exactlyOnce ((Values L mem).kentries (ℓ := ℓ) a i)
      ((Values L mem).kentries (ℓ := ℓ) b i) :=
  fun i => KeyedPool.toMultiset_le (h i)

theorem vmono_kmap {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W] {ord : StrOrd}
    {a b : (Values L mem).KeyedStream ℓ K V ord .exactlyOnce} (f : Fin (mem ℓ) → V → W)
    (h : ∀ i, KeyedPool.Le (a i) (b i)) :
    ∀ i, KeyedPool.Le ((Values L mem).kmap (ℓ := ℓ) a f i) ((Values L mem).kmap (ℓ := ℓ) b f i) :=
  fun i => KeyedPool.map_Le (f i) (h i)

theorem vmono_kallTicks {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {a b : (Values L mem).AtomicKeyedStream ℓ K V ord .exactlyOnce}
    (h : ∀ i, a i <+: b i) :
    ∀ i, KeyedPool.Le ((Values L mem).kallTicks (ℓ := ℓ) a i) ((Values L mem).kallTicks (ℓ := ℓ) b i) :=
  fun i => KeyedPool.accum_prefix_le (h i)

theorem vmono_kfold {K V A : Type} [DecidableEq K] [Hashable K] [DecidableEq V] [DecidableEq A]
    {ord : StrOrd} (g : A → V → A) (init : A) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOk ord .exactlyOnce Pre g)
    {a b : (Values L mem).KeyedStream ℓ K V ord .exactlyOnce}
    (h : ∀ i, KeyedPool.Le (a i) (b i)) :
    ∀ i d, ((Values L mem).kfold (ℓ := ℓ) g init Pre ok a i) d
      <+: ((Values L mem).kfold (ℓ := ℓ) g init Pre ok b i) d :=
  fun i d => KeyedPool.kviews_map_readAt_le (h i) (KeyedMap.ofFold g init Pre ok) 0 d

theorem vmono_ksnapshot {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} {b : KSBound}
    {x y : (Values L mem).KeyedSingleton ℓ α K V ord ret b}
    (d : (Values L mem).SnapDec (mem ℓ) α .noOrder)
    (h : ∀ i c, x i c <+: y i c) :
    ∀ i, ((Values L mem).ksnapshot (ℓ := ℓ) x d i) <+: ((Values L mem).ksnapshot (ℓ := ℓ) y d i) :=
  fun i => h i (d i)

theorem vmono_tfold {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk ord ret Pre g)
    {a b : (Values L mem).AtomicStream ℓ α ord ret} (h : ∀ i, a i <+: b i) :
    ∀ i, ((Values L mem).tfold (ℓ := ℓ) g init Pre ok a i)
      <+: ((Values L mem).tfold (ℓ := ℓ) g init Pre ok b i) :=
  fun i => foldAcrossTicksTrace_prefix _ _ (h i)

theorem vmono_latest {σ : Type} [DecidableEq σ] {a b : (Values L mem).Atomic ℓ (Option σ)}
    (h : ∀ i, a i <+: b i) :
    ∀ i d, ((Values L mem).latest (ℓ := ℓ) a i) d
      <+: ((Values L mem).latest (ℓ := ℓ) b i) d :=
  fun i d => List.IsPrefix.map _ (prefixCuts_le (h i) 0 d)

theorem vmono_latest_monotone {a b : (Values L mem).Atomic ℓ (Option Nat)}
    (h : ∀ i, a i <+: b i) :
    ∀ i d, ((Values L mem).latest_monotone (ℓ := ℓ) a i).val d
      <+: ((Values L mem).latest_monotone (ℓ := ℓ) b i).val d :=
  fun i d => List.IsPrefix.map _ (takeWhile_prefix_mono _ (prefixCuts_le (h i) 0 d))

theorem vmono_time (d : (Values L mem).ClockDec (mem ℓ)) :
    ∀ i, ((Values L mem).time (ℓ := ℓ) d i) <+: ((Values L mem).time (ℓ := ℓ) d i) :=
  fun _i => List.prefix_refl _

theorem vmono_snapshot_bounded {σ : Type} (x : σ) (d : (Values L mem).TicksDec (mem ℓ)) :
    ∀ i, ((Values L mem).snapshot_bounded (ℓ := ℓ) x d i)
      <+: ((Values L mem).snapshot_bounded (ℓ := ℓ) x d i) :=
  fun _i => List.prefix_refl _

end TickOps

/-! ## Knot ops

The two bodies may differ; the hypothesis `hb` relates them on related inputs, and the
conclusion compares the two Kleene iterates at the same fuel. -/

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
    (R := fun (x y : MemberMap (mem ℓ) (PoolCarrier α ord ret)) =>
      ∀ i, PoolLe ord ret (x i) (y i))
    (fun {x y} hxy => hb x y hxy)
    (fun _i => PoolLe.refl _ _ _) df

theorem vmono_fix_tick {σ : Type} (df : (Values L mem).FixDec)
    {bodyA bodyB : (Values L mem).Atomic ℓ σ →
      (Values L mem).Atomic ℓ σ}
    (hb : ∀ x y, (∀ i, x i <+: y i) →
      ∀ i, bodyA x i <+: bodyB y i) :
    ∀ i, ((Values L mem).fix_tick (ℓ := ℓ) df bodyA i)
      <+: ((Values L mem).fix_tick (ℓ := ℓ) df bodyB i) :=
  iterate_mono_param
    (R := fun (x y : TickV (mem ℓ) σ) => ∀ i, x i <+: y i)
    (fun {x y} hxy => hb x y hxy)
    (fun _i => List.nil_prefix) df

end KnotOps
end Hydro
