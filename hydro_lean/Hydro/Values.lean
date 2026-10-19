import Hydro.Sem
import Hydro.MemberMap

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
  tick batches keep their grade (`Trace (Multiset α)` when unordered);
- every cluster-located carrier is a `MemberMap (mem ℓ) _` — one cell
  per member, a function in a structure (`MemberMap.lean`): reads are
  `s i`, definitional; the compiled constructor forces the cells once,
  so a `Values` program *executes* in one dataflow pass (D73 — the
  `Eager` interpretation this retires did the same with a second leg).
-/

namespace Hydro

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
    (ret : Retries) (Pre : α → α → Prop) [DecidableRel Pre] (g : σ → α → σ) (init : σ)
    (ok : FoldOk ord ret Pre g)
    (pool : PoolCarrier α ord ret) (d : CutDec α ord) : Trace σ :=
  (snapViews ord ret pool d).map (PoolFold ord ret Pre g init ok)

/-- Reads of an inflationary fold ascend along ticks, at every grade:
views chain (prefix / sub-multiset / membership-accumulation) and the
graded fold only moves up along the chain. (Unconditional obligation
only — `fold_monotone`'s: a refused-content branch has no ascent.) -/
theorem snapTrace_ascending {α σ : Type _} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret (fun _ _ => True) g)
    (hinfl : ∀ s x, vo.le s (g s x))
    (pool : PoolCarrier α ord ret) (d : CutDec α ord) :
    Ascending vo (snapTrace ord ret (fun _ _ => True) g init ok pool d) := by
  intro t t' h ht'
  have hlen : t' < (snapViews ord ret pool d).length := by
    have := ht'
    unfold snapTrace at this
    rwa [List.length_map] at this
  have hlt : t < (snapViews ord ret pool d).length :=
    Nat.lt_of_le_of_lt h hlen
  have hmt : t < ((snapViews ord ret pool d).map
      (PoolFold ord ret (fun _ _ => True) g init ok)).length := by
    rwa [List.length_map]
  have hmt' : t' < ((snapViews ord ret pool d).map
      (PoolFold ord ret (fun _ _ => True) g init ok)).length := by
    rwa [List.length_map]
  show vo.le
    (((snapViews ord ret pool d).map (PoolFold ord ret (fun _ _ => True) g init ok))[t]'hmt)
    (((snapViews ord ret pool d).map
      (PoolFold ord ret (fun _ _ => True) g init ok))[t']'hmt')
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
    show vo.le (PoolFold .totalOrder .atLeastOnce (fun _ _ => True) g init ok
        (((prefixCuts pool.norm 0 d).map StutterSeq.mk)[t]'(by
          rwa [List.length_map])))
      (PoolFold .totalOrder .atLeastOnce (fun _ _ => True) g init ok
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
        vo.le (PoolFold .noOrder .exactlyOnce (fun _ _ => True) g init ok v)
          (PoolFold .noOrder .exactlyOnce (fun _ _ => True) g init ok v') := by
      intro v v' hv
      show vo.le (Multiset.foldlOn (fun _ _ => True) g init ok v)
        (Multiset.foldlOn (fun _ _ => True) g init ok v')
      rw [Multiset.foldlOn_true, Multiset.foldlOn_true, hv, Multiset.foldl_add]
      exact multiset_le_foldl vo g (fun s x y => ok s x y trivial) hinfl e _
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
    show vo.le (PoolFold .noOrder .atLeastOnce (fun _ _ => True) g init ok
        (((snapshotMemCuts pool.support.val 0 d).map RetryPool.mk)[t]'(by
          rwa [List.length_map])))
      (PoolFold .noOrder .atLeastOnce (fun _ _ => True) g init ok
        (((snapshotMemCuts pool.support.val 0 d).map RetryPool.mk)[t']'(by
          rwa [List.length_map])))
    rw [List.getElem_map, List.getElem_map]
    have hle := snapshotMemCuts_getElem_le
      (pool := pool.support.val) h h2
    obtain ⟨e, he⟩ := Multiset.le_iff_exists_add.mp hle
    have key : ∀ (v v' : Multiset α), v' = v + e →
        vo.le (RetryPool.foldOn (fun _ _ => True) g init ok.1 ok.2 (RetryPool.mk v))
          (RetryPool.foldOn (fun _ _ => True) g init ok.1 ok.2 (RetryPool.mk v')) := by
      intro v v' hv
      rw [RetryPool.foldOn_true_mk, RetryPool.foldOn_true_mk, hv, Multiset.foldl_add]
      exact multiset_le_foldl vo g (fun s x y => ok.1 s x y trivial) hinfl e _
    exact key _ _ he

/-- One member's cell of a `Values` singleton: the graded read function;
`Monotonic` bundles ascent of every read. -/
@[reducible] def SingCell (α σ : Type) [DecidableEq α] (ord : StrOrd) :
    SingBound σ → Type
  | .unbounded => CutDec α ord → Trace σ
  | .monotonic vo => {f : CutDec α ord → Trace σ // ∀ d, Ascending vo (f d)}

/-- The `Values` singleton carrier: one read cell per member (a
`MemberMap` for every bound, so a cell read `s i` is the `MemberMap`
coercion whatever `b` is). -/
abbrev SingletonV (n : Nat) (α σ : Type) [DecidableEq α] (ord : StrOrd)
    (b : SingBound σ) : Type :=
  MemberMap n (SingCell α σ ord b)

/-- Every singleton carrier is inhabited (the empty read; a `ghost pre`
fact parametric in an unused singleton input instantiates at it). -/
instance SingletonV.instInhabitedMonotonicCell (α σ : Type) [DecidableEq α] (ord : StrOrd)
    (vo : ValueOrder σ) : Inhabited {f : CutDec α ord → Trace σ // ∀ d, Ascending vo (f d)} :=
  ⟨⟨fun _ => [], fun _ _ _ _ ht' => absurd ht' (Nat.not_lt_zero _)⟩⟩

instance SingCell.instInhabited (α σ : Type) [DecidableEq α] (ord : StrOrd) :
    (b : SingBound σ) → Inhabited (SingCell α σ ord b)
  | .unbounded => ⟨fun _ => []⟩
  | .monotonic _ => ⟨⟨fun _ => [], fun _ _ _ _ ht' => absurd ht' (Nat.not_lt_zero _)⟩⟩

instance SingletonV.instInhabited (n : Nat) (α σ : Type) [DecidableEq α] (ord : StrOrd)
    (b : SingBound σ) : Inhabited (SingletonV n α σ ord b) :=
  ⟨.mk' fun _ => default⟩

/-- The `Monotonic` singleton cell that never advances (every snapshot
reads `none` at each tick): the scenarios' idle checkpoint. -/
def SingletonV.noneCell (α : Type) [DecidableEq α] (ord : StrOrd) :
    {f : CutDec α ord → Trace (Option Nat) // ∀ d, Ascending ValueOrder.optNat (f d)} :=
  ⟨fun d => List.replicate d.ticks none, fun d _ _ _ ht' => by
    rw [List.getElem_replicate, List.getElem_replicate]
    trivial⟩

/-- The `Monotonic` singleton cell that advances once: `none` at the
first tick, `some c` from the second on (a checkpoint that runs ahead
and collects — the GC scenarios). -/
def SingletonV.stepCell (α : Type) [DecidableEq α] (ord : StrOrd) (c : Nat) :
    {f : CutDec α ord → Trace (Option Nat) // ∀ d, Ascending ValueOrder.optNat (f d)} :=
  ⟨fun d => (List.range d.ticks).map (fun t => if t = 0 then none else some c),
    fun d t t' h ht' => by
      simp only [List.getElem_map, List.getElem_range]
      by_cases h0 : t = 0
      · simp [h0, ValueOrder.optNatLE]
      · have h0' : t' ≠ 0 := by omega
        simp [h0, h0', ValueOrder.optNatLE]⟩

/-- The `Values` ticked carrier (`HydroSem.Atomic`): the realized
per-member tick sequence of a per-tick value. -/
abbrev TickV (n : Nat) (σ : Type) : Type := MemberMap n (Trace σ)

/-! ### Tick shapes at the denotation: slicing a shaped input into
per-tick tuples and un-slicing a per-tick output trace into wires -/

namespace ValuesTick
variable {L : Type} {mem : L → Nat}

/-- The denotation's ticked carriers, as `AtomicOf` parameters. -/
abbrev Tk (mem : L → Nat) (ℓ : L) (τ : Type) : Type := MemberMap (mem ℓ) (Trace τ)
abbrev TS (mem : L → Nat) (ℓ : L) (α : Type) [DecidableEq α] (ord : StrOrd)
    (ret : Retries) : Type := MemberMap (mem ℓ) (Trace (PoolCarrier α ord ret))
abbrev TKS (mem : L → Nat) (ℓ : L) (K V : Type) [DecidableEq K] [DecidableEq V] :
    Type := MemberMap (mem ℓ) (Trace (KeyedMap K V))
/-- The denotation's bounded families, as `BoundedOf` parameters. -/
abbrev BS (σ : Type) : Type := σ
abbrev BKS (K V : Type) [DecidableEq K] [DecidableEq V] : Type := KeyedMap K V

/-- Member `i`'s per-tick input tuples (the zip of the wires' traces). -/
def slice {ℓ : L} : (sh : TickShape) → AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh →
    Fin (mem ℓ) → Trace (BoundedOf BS PoolCarrier BKS sh)
  | .sing _, x, i => x i
  | .stream _ _ _ _, x, i => x i
  | .ksing _ _ _ _, x, i => x i
  | .pair a b, x, i => Trace.zip (slice a x.1 i) (slice b x.2 i)

/-- A per-member trace of output tuples, as the output wires (a leaf is
the trace itself — no copy; a pair projects, each leg materialized
once). -/
def unslice {ℓ : L} : (sh : TickShape) →
    MemberMap (mem ℓ) (Trace (BoundedOf BS PoolCarrier BKS sh)) →
    AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh
  | .sing _, f => f
  | .stream _ _ _ _, f => f
  | .ksing _ _ _ _, f => f
  | .pair a b, f =>
    (unslice a (.mk' fun i => (f i).map Prod.fst),
     unslice b (.mk' fun i => (f i).map Prod.snd))

/-- The initial register of a shape from its seed (a stream register
starts empty). -/
def seed : (sh : TickShape) → SeedOf sh → BoundedOf BS PoolCarrier BKS sh
  | .sing _, v => v
  | .stream _ _ ord ret, _ => PoolBot ord ret
  | .ksing _ _ _ _, _ => KeyedMap.empty
  | .pair a b, s => (seed a s.1, seed b s.2)

/-- The tick former at the denotation: the structural fold over each
member's ticks, on the sliced inputs. -/
def scan {ℓ : L} (sts ins outs : TickShape)
    (x : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier BKS sts →
      BoundedOf BS PoolCarrier BKS ins →
      BoundedOf BS PoolCarrier BKS sts × BoundedOf BS PoolCarrier BKS outs)
    (init : SeedOf sts) : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ outs :=
  unslice outs (.mk' fun i => scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i))

/-- Leafwise trace prefix between two shaped values. -/
def Le {ℓ : L} : (sh : TickShape) →
    AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh → AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh → Prop
  | .sing _, a, b => ∀ i, a i <+: b i
  | .stream _ _ _ _, a, b => ∀ i, a i <+: b i
  | .ksing _ _ _ _, a, b => ∀ i, a i <+: b i
  | .pair sa sb, a, b => Le sa a.1 b.1 ∧ Le sb a.2 b.2

theorem slice_prefix {ℓ : L} : (sh : TickShape) →
    (a b : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh) → Le sh a b →
    ∀ i, slice sh a i <+: slice sh b i
  | .sing _, _, _, h, i => h i
  | .stream _ _ _ _, _, _, h, i => h i
  | .ksing _ _ _ _, _, _, h, i => h i
  | .pair sa sb, a, b, h, i =>
    zip_prefix (slice_prefix sa a.1 b.1 h.1 i) (slice_prefix sb a.2 b.2 h.2 i)

theorem unslice_le {ℓ : L} : (sh : TickShape) →
    (F G : MemberMap (mem ℓ) (Trace (BoundedOf BS PoolCarrier BKS sh))) →
    (∀ i, F i <+: G i) → Le sh (unslice sh F) (unslice sh G)
  | .sing _, _, _, h => h
  | .stream _ _ _ _, _, _, h => h
  | .ksing _ _ _ _, _, _, h => h
  | .pair sa sb, F, G, h =>
    ⟨unslice_le sa _ _ (fun i => List.IsPrefix.map _ (h i)),
     unslice_le sb _ _ (fun i => List.IsPrefix.map _ (h i))⟩

/-- The tick former preserves leafwise prefix (the monotonicity face). -/
theorem scan_le {ℓ : L} (sts ins outs : TickShape)
    (a b : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ ins) (h : Le ins a b)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier BKS sts → BoundedOf BS PoolCarrier BKS ins →
      BoundedOf BS PoolCarrier BKS sts × BoundedOf BS PoolCarrier BKS outs) (init : SeedOf sts) :
    Le outs (scan sts ins outs a g init) (scan sts ins outs b g init) :=
  unslice_le outs _ _ (fun i =>
    scanAcrossTicksTrace_prefix _ (seed sts init) (slice_prefix ins a b h i))

end ValuesTick

/-- The graded denotation. Reducible: the interpretation's type fields
(`BoundedSingleton σ = σ`, …) and operator fields open under `simp`'s
reducible transparency, so a ghost's `simp only [<out>_step, den]` reads
a generated step as the plain term of its Rust lines without the
`show`/bridge-lemma detour (FINDINGS D64, E6). -/
@[reducible] def Values (L : Type) (mem : L → Nat) : HydroSem L mem where
  Stream ℓ α _ ord ret := MemberMap (mem ℓ) (PoolCarrier α ord ret)
  TransportDec _ _ := Unit
  OrderSelDec n α := OrderSelection n α
  TickOrderSelDec n α := TickOrderSelection n α
  SnapDec n α ord := SnapshotCuts n α ord
  BatchDec n α := BatchCuts n α
  OrdBatchDec n := OrderedBatchCuts n
  SampleDec n := SampleTimes n
  TimerDec n := TimerVerdicts n
  PulseDec n := TimingPulses n
  FixDec := UnfoldFuel
  KeyedStream ℓ K V _ _ ord ret := MemberMap (mem ℓ) (KeyedPool K V ord ret)
  Singleton ℓ α σ _ ord _ret b := SingletonV (mem ℓ) α σ ord b
  -- a keyed fold's live map: its read function over increment cuts of
  -- the keyed source's pairs
  KeyedSingleton ℓ α K V _ _ _ _ord _ret _b :=
    MemberMap (mem ℓ) (CutDec α .noOrder → Trace (KeyedMap K V))
  Atomic ℓ σ := TickV (mem ℓ) σ
  -- one tick's bounded stream: the graded content quotient
  BoundedStream α _ ord ret := PoolCarrier α ord ret
  BoundedSingleton σ := σ
  BoundedKeyedStream K V _ _ ord ret := KeyedPool K V ord ret
  BoundedKeyedSingleton K V _ _ := KeyedMap K V
  -- `AtomicStream` is the signature default: `Atomic ℓ (BoundedStream …)`
  -- = `MemberMap (mem ℓ) (Trace (PoolCarrier α ord ret))`
  -- every member-indexed op builds its carrier with `MemberMap.mk'`
  -- (reads are `rfl`; the compiled constructor forces the cells once)
  map {_ℓ _α _β _ _ ord} s f := .mk' fun i => mapPool (ord := ord) (f i) (s i)
  filterMap {_ℓ _α _β _ _ ord} s f := .mk' fun i =>
    filterMapPool (ord := ord) (f i) (s i)
  broadcast_closed _d s := .mk' fun _p => KeyedPool.transport (fun j => s j)
  demux {_c _p _α _ ord} _d s addr := .mk' fun i => KeyedPool.transport (fun j =>
    filterMapPool (ord := ord)
      (fun dx => if dx.1 = addr i then some dx.2 else none) (s j))
  values k := .mk' fun i => (k i).values
  into_keyed {_ℓ _K _V _ _ ord} s := .mk' fun i => KeyedPool.ofPool (ord := ord) (s i)
  kassume_ordering u d := .mk' fun i => KeyedPool.ofList (selectOrder (u i).toMultiset (d i))
  -- (`ks i` was bound once here before D73 — at a knot the keyed source
  -- was a closure re-running the upstream per view, D71; a `MemberMap`
  -- read is a cell lookup, so the `let` is now only a courtesy)
  kfold_early_stop ks f init := .mk' fun i d =>
    let p := ks i
    (p.kviews 0 d).map (fun M => KeyedMap.ofFoldEarlyStop f init (p.readAt M))
  kget_max_key ks := .mk' fun i d => (ks i d).map KeyedMap.maxEntry
  weaken_retries {_ℓ _α _ ord} s :=
    match ord, s with
    | .totalOrder, s => .mk' fun i => StutterSeq.mk (s i)
    | .noOrder, s => .mk' fun i => RetryPool.mk (s i)
  union {_ℓ _α _ ret} a b :=
    match ret, a, b with
    | .exactlyOnce, a, b => .mk' fun i => a i + b i
    | .atLeastOnce, a, b => .mk' fun i => RetryPool.union (a i) (b i)
  assume_ordering u d := .mk' fun i => selectOrder (u i) (d i)
  fold {_ℓ _α _σ _ ord ret} g init Pre _ ok s := .mk' fun i d =>
    snapTrace ord ret Pre g init ok (s i) d
  fold_monotone {_ℓ _α _σ _ ord ret} vo g init ok hinfl s := .mk' fun i =>
    ⟨fun d => snapTrace ord ret (fun _ _ => True) g init ok (s i) d,
     fun d => snapTrace_ascending vo g init ok hinfl (s i) d⟩
  snapshot {_ℓ _α _σ _ _ord _ret b} s d :=
    match b, s with
    | .unbounded, s => .mk' fun i => s i (d i)
    | .monotonic _vo, s => .mk' fun i => (s i).val (d i)
  batch s d := .mk' fun i => batchCuts (s i) 0 (d i)
  batch_ordered s d := .mk' fun i => sliceCuts (s i) 0 (d i)
  sample_every t d := .mk' fun i => StutterSeq.mk (sampleAtOpt (t i) (d i))
  timeout_snapshot _s d := .mk' fun i => d i
  source_interval_batch d := .mk' fun i => d i
  mapTick s f := .mk' fun i => (s i).map (f i)
  zipTick a b := .mk' fun i => Trace.zip (a i) (b i)
  defer_tick init t := .mk' fun i => init :: t i
  allTicks {_ℓ _β _ ord} bs :=
    match ord, bs with
    | .totalOrder, bs => .mk' fun i => (bs i).flatten
    | .noOrder, bs => .mk' fun i => (bs i).sum
  flattenOrdered t := .mk' fun i => t i
  flattenUnordered t := .mk' fun i => (t i).map (fun l => Multiset.ofList l)
  fix_stream {_ℓ _α _ ord ret} fuel body :=
    iterate body (.mk' fun _i => PoolBot ord ret) fuel
  fix_tick fuel body := iterate body (.mk' fun _i => []) fuel
  -- the atomic-trace keyed ops: per-tick keyed content, the whole-map
  -- state scanned across ticks
  tinto_keyed {_ℓ _K _V _ _ ord} x := .mk' fun i => (x i).map (KeyedPool.ofPool (ord := ord))
  -- per tick, the selection realizes that tick's batch (a tick beyond
  -- the decision's horizon is not realized: the trace is cut there)
  tassume_ordering s d := .mk' fun i => realizeTicks (s i) (d i)
  reduce_watermark ks wm f Pre _ ok := .mk' fun i =>
    scanAcrossTicksTrace (reduceWatermarkStep f Pre ok) (KeyedPool.bot, none)
      (Trace.zip (ks i) (wm i))
  -- in-tick operators: the quotient's own operations
  bmap {_α _β _ _ ord} b f := mapPool (ord := ord) f b
  bfilterMap {_α _β _ _ ord} b f := filterMapPool (ord := ord) f b
  bflatMapOrdered l f := l.flatMap f
  bflatMapUnordered {_α _β _ _ ord} b f :=
    poolFlatMapUnordered (ord := ord) b f
  bofList l := (↑l : Multiset _)
  bcount {_α _ ord} b := poolCount (ord := ord) b
  bfold {_α _σ _ ord ret} g init Pre _ ok b := PoolFold ord ret Pre g init ok b
  benumerate l := listEnumerate l
  bfirst l := l.head?
  bcrossSingleton {_α _σ _ _ ord} b s := mapPool (ord := ord) (fun a => (a, s)) b
  bchain {_α _ ord} a b := poolChain (ord := ord) a b
  bweakenOrder {_α _ ord} b := poolWeakenOrder (ord := ord) b
  bfilter {_α _ ord} b p := poolFilter (ord := ord) p b
  binto_keyed {_K _V _ _ ord} b := KeyedPool.ofPool (ord := ord) b
  bkfold g init Pre _ ok ks := KeyedMap.ofFold g init Pre ok ks
  bksFilter m p := m.filter p
  bksMap m f := m.map f
  bksKeys m := m.keysMS
  bksEntries m := m.entries
  bjoin {_K _V _W _ _ _ _ ord ord'} a b := poolJoin (ord := ord) (ord' := ord') a b
  bantiJoin {_K _V _ _ _ ord ord'} a ks := poolAntiJoin (ord := ord) (ord' := ord') a ks
  bfilterNotIn {_α _ _ ord ord'} a o := poolFilterNotIn (ord := ord) (ord' := ord') a o
  bmax {_α _ _ ord} b := poolMax (ord := ord) b
  bfilterIf b flag := poolFilterIf b flag
  bsPure v := v
  bsMap s f := f s
  bsZip a b := (a, b)
  boMap o f := o.map f
  boUnwrapOr o s := o.getD s
  boFilter o p := o.filter p
  boIsSome o := o.isSome
  tick_scan sts ins outs x g init := ValuesTick.scan sts ins outs x g init

/-! ### Member reads through the instance projections

`s i` on a `Values` carrier elaborates through `CoeFun`. Instance lookup
reduces the goal type at *reducible* transparency, where a class
projection of the (reducible) `Values` instance does NOT unfold — class
fields are kept in canonical form (`Lean.Meta.unfoldProjInst?`), so
`(Values L mem).Stream ℓ α ord ret` is keyed as such, not as its
`MemberMap`. One `CoeFun` instance per member-indexed family, keyed on
the projected spelling, closes the gap; the raw `MemberMap` spelling
has its own instance in `MemberMap.lean`. -/
section MemberReads
variable {L : Type} {mem : L → Nat} {ℓ : L}

instance instCoeFunValuesStream {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries} :
    CoeFun ((Values L mem).Stream ℓ α ord ret) (fun _ => Fin (mem ℓ) → PoolCarrier α ord ret) :=
  ⟨MemberMap.get⟩
instance instCoeFunValuesKeyedStream {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} :
    CoeFun ((Values L mem).KeyedStream ℓ K V ord ret) (fun _ => Fin (mem ℓ) → KeyedPool K V ord ret) :=
  ⟨MemberMap.get⟩
instance instCoeFunValuesSingleton {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {b : SingBound σ} :
    CoeFun ((Values L mem).Singleton ℓ α σ ord ret b) (fun _ => Fin (mem ℓ) → SingCell α σ ord b) :=
  ⟨MemberMap.get⟩
instance instCoeFunValuesKeyedSingleton {α K V : Type} [DecidableEq α] [DecidableEq K]
    [DecidableEq V] {ord : StrOrd} {ret : Retries} {kb : KSBound} :
    CoeFun ((Values L mem).KeyedSingleton ℓ α K V ord ret kb)
      (fun _ => Fin (mem ℓ) → CutDec α .noOrder → Trace (KeyedMap K V)) :=
  ⟨MemberMap.get⟩
instance instCoeFunValuesAtomic {σ : Type} :
    CoeFun ((Values L mem).Atomic ℓ σ) (fun _ => Fin (mem ℓ) → Trace σ) :=
  ⟨MemberMap.get⟩
instance instCoeFunValuesAtomicStream {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries} :
    CoeFun ((Values L mem).AtomicStream ℓ α ord ret)
      (fun _ => Fin (mem ℓ) → Trace (PoolCarrier α ord ret)) :=
  ⟨MemberMap.get⟩
instance instCoeFunValuesAtomicKeyedStream {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} :
    CoeFun ((Values L mem).AtomicKeyedStream ℓ K V ord ret)
      (fun _ => Fin (mem ℓ) → Trace (KeyedPool K V ord ret)) :=
  ⟨MemberMap.get⟩
instance instCoeFunValuesAtomicKeyedSingleton {K V : Type} [DecidableEq K] [DecidableEq V] :
    CoeFun ((Values L mem).AtomicKeyedSingleton ℓ K V)
      (fun _ => Fin (mem ℓ) → Trace (KeyedMap K V)) :=
  ⟨MemberMap.get⟩

end MemberReads

/-! ## Reading a `tick` block at the denotation

Rewrite rules for ghost proofs over a `tick` block's output wires: a
leaf-shaped output is the per-member `scanAcrossTicksTrace` of the body
over the sliced (zipped) inputs; slices open structurally. Each is
`rfl` by one constructor step — a ghost proof `rw`s with them instead
of `show`-ing the zipped form, which makes the elaborator reduce the
whole shaped former by definitional unfolding (`TickShape.rec` by the
hundred thousand; FINDINGS 0c-iii). -/
namespace ValuesTick
variable {L : Type} {mem : L → Nat} {ℓ : L}

theorem slice_sing {τ : Type} (x : Tk mem ℓ τ) (i : Fin (mem ℓ)) :
    slice (.sing τ) x i = x i := rfl
theorem slice_stream {α : Type} [inst : DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : TS mem ℓ α ord ret) (i : Fin (mem ℓ)) :
    slice (.stream α inst ord ret) x i = x i := rfl
theorem slice_ksing {K V : Type} [iK : DecidableEq K] [iV : DecidableEq V]
    (x : TKS mem ℓ K V) (i : Fin (mem ℓ)) :
    slice (.ksing K V iK iV) x i = x i := rfl
theorem slice_pair (a b : TickShape)
    (x : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ (.pair a b)) (i : Fin (mem ℓ)) :
    slice (.pair a b) x i = Trace.zip (slice a x.1 i) (slice b x.2 i) := rfl

theorem tick_scan_sing {τ : Type} (sts ins : TickShape)
    (x : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier BKS sts → BoundedOf BS PoolCarrier BKS ins →
      BoundedOf BS PoolCarrier BKS sts × BoundedOf BS PoolCarrier BKS (.sing τ))
    (init : SeedOf sts) (i : Fin (mem ℓ)) :
    (Values L mem).tick_scan sts ins (.sing τ) x g init i
      = scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i) := rfl
theorem tick_scan_stream {α : Type} [inst : DecidableEq α] {ord : StrOrd}
    {ret : Retries} (sts ins : TickShape)
    (x : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier BKS sts → BoundedOf BS PoolCarrier BKS ins →
      BoundedOf BS PoolCarrier BKS sts × BoundedOf BS PoolCarrier BKS (.stream α inst ord ret))
    (init : SeedOf sts) (i : Fin (mem ℓ)) :
    (Values L mem).tick_scan sts ins (.stream α inst ord ret) x g init i
      = scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i) := rfl
theorem tick_scan_ksing {K V : Type} [iK : DecidableEq K] [iV : DecidableEq V]
    (sts ins : TickShape)
    (x : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier BKS sts → BoundedOf BS PoolCarrier BKS ins →
      BoundedOf BS PoolCarrier BKS sts × BoundedOf BS PoolCarrier BKS (.ksing K V iK iV))
    (init : SeedOf sts) (i : Fin (mem ℓ)) :
    (Values L mem).tick_scan sts ins (.ksing K V iK iV) x g init i
      = scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i) := rfl
theorem seed_sing {τ : Type} (v : τ) : seed (.sing τ) v = v := rfl
theorem seed_stream {α : Type} [inst : DecidableEq α] {ord : StrOrd} {ret : Retries}
    (u : Unit) : seed (.stream α inst ord ret) u = PoolBot ord ret := rfl
theorem seed_ksing {K V : Type} [iK : DecidableEq K] [iV : DecidableEq V] (u : Unit) :
    seed (.ksing K V iK iV) u = KeyedMap.empty := rfl
theorem seed_pair (a b : TickShape) (s : SeedOf (.pair a b)) :
    seed (.pair a b) s = (seed a s.1, seed b s.2) := rfl
/-- The output legs of a multi-output block: the tuple trace, projected. -/
theorem tick_scan_pair (sts ins a b : TickShape)
    (x : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier BKS sts → BoundedOf BS PoolCarrier BKS ins →
      BoundedOf BS PoolCarrier BKS sts × BoundedOf BS PoolCarrier BKS (.pair a b))
    (init : SeedOf sts) :
    (Values L mem).tick_scan sts ins (.pair a b) x g init
      = (unslice a (.mk' fun i => (scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i)).map Prod.fst),
         unslice b (.mk' fun i => (scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i)).map Prod.snd)) := rfl
theorem unslice_sing {τ : Type} (f : MemberMap (mem ℓ) (Trace (BoundedOf BS PoolCarrier BKS (.sing τ))))
    (i : Fin (mem ℓ)) : unslice (.sing τ) f i = f i := rfl
theorem unslice_stream {α : Type} [inst : DecidableEq α] {ord : StrOrd} {ret : Retries}
    (f : MemberMap (mem ℓ) (Trace (BoundedOf BS PoolCarrier BKS (.stream α inst ord ret))))
    (i : Fin (mem ℓ)) : unslice (.stream α inst ord ret) f i = f i := rfl
theorem unslice_ksing {K V : Type} [iK : DecidableEq K] [iV : DecidableEq V]
    (f : MemberMap (mem ℓ) (Trace (BoundedOf BS PoolCarrier BKS (.ksing K V iK iV))))
    (i : Fin (mem ℓ)) : unslice (.ksing K V iK iV) f i = f i := rfl
theorem unslice_pair (a b : TickShape)
    (f : MemberMap (mem ℓ) (Trace (BoundedOf BS PoolCarrier BKS (.pair a b)))) :
    unslice (.pair a b) f
      = (unslice a (.mk' fun i => (f i).map Prod.fst), unslice b (.mk' fun i => (f i).map Prod.snd)) := rfl

/-- The wire-level reads a ghost meets at a `tick` block's inputs. -/
@[den] theorem values_zipTick {σ τ : Type} (a : (Values L mem).Atomic ℓ σ)
    (b : (Values L mem).Atomic ℓ τ) (i : Fin (mem ℓ)) :
    (Values L mem).zipTick a b i = Trace.zip (a i) (b i) := rfl
@[den] theorem values_defer_tick {σ : Type} (v : σ) (t : (Values L mem).Atomic ℓ σ)
    (i : Fin (mem ℓ)) :
    (Values L mem).defer_tick v t i = v :: t i := rfl
@[den] theorem values_flattenUnordered {β : Type} [DecidableEq β]
    (t : (Values L mem).Atomic ℓ (List β)) (i : Fin (mem ℓ)) :
    (Values L mem).flattenUnordered t i = (t i).map (fun l => Multiset.ofList l) := rfl
@[den] theorem values_batch_ordered {α : Type} [DecidableEq α]
    (s : (Values L mem).Stream ℓ α .totalOrder .exactlyOnce)
    (d : (Values L mem).OrdBatchDec (mem ℓ)) (i : Fin (mem ℓ)) :
    (Values L mem).batch_ordered s d i = sliceCuts (s i) 0 (d i) := rfl
@[den] theorem values_batch {α : Type} [DecidableEq α]
    (s : (Values L mem).Stream ℓ α .noOrder .exactlyOnce)
    (d : (Values L mem).BatchDec (mem ℓ) α) (i : Fin (mem ℓ)) :
    (Values L mem).batch s d i = batchCuts (s i) 0 (d i) := rfl
@[den] theorem values_mapTick {σ τ : Type} (t : (Values L mem).Atomic ℓ σ)
    (f : Fin (mem ℓ) → σ → τ) (i : Fin (mem ℓ)) :
    (Values L mem).mapTick t f i = (t i).map (f i) := rfl
@[den] theorem values_flattenOrdered {β : Type} [DecidableEq β]
    (t : (Values L mem).Atomic ℓ (List β)) (i : Fin (mem ℓ)) :
    (Values L mem).flattenOrdered t i = t i := rfl
@[den] theorem values_filterMap {α β : Type} [DecidableEq α] [DecidableEq β] {ord : StrOrd}
    (s : (Values L mem).Stream ℓ α ord .exactlyOnce) (f : Fin (mem ℓ) → α → Option β)
    (i : Fin (mem ℓ)) :
    (Values L mem).filterMap s f i = filterMapPool (ord := ord) (f i) (s i) := rfl
@[den] theorem values_union_exactlyOnce {α : Type} [DecidableEq α]
    (a b : (Values L mem).Stream ℓ α .noOrder .exactlyOnce) (i : Fin (mem ℓ)) :
    (Values L mem).union a b i = a i + b i := rfl
@[den] theorem values_demux {c : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c (Nat × α) ord .exactlyOnce)
    (addr : Fin (mem ℓ) → Nat) (i : Fin (mem ℓ)) :
    (Values L mem).demux d s addr i
      = KeyedPool.transport (fun j => filterMapPool (ord := ord)
          (fun dx => if dx.1 = addr i then some dx.2 else none) (s j)) := rfl

/-! The stream-boundary operators a ghost meets at a module's output
(`all_ticks` → `map` → `broadcast` → `values`), read at one member.
Each is `rfl` here, at the semantics; stating them as rewrite rules
spares consumers the `whnf` of the folded `Values` instance's
projections (the lazy-delta unifier otherwise unfolds the *other*
side — `Multiset.map` down to `Quot.lift` — before reducing the
projection). -/
@[den] theorem values_map {α β : Type} [DecidableEq α] [DecidableEq β] {ord : StrOrd}
    (s : (Values L mem).Stream ℓ α ord .exactlyOnce) (f : Fin (mem ℓ) → α → β)
    (i : Fin (mem ℓ)) :
    (Values L mem).map s f i = mapPool (ord := ord) (f i) (s i) := rfl
@[den] theorem values_allTicks_noOrder {β : Type} [DecidableEq β]
    (bs : (Values L mem).AtomicStream ℓ β .noOrder .exactlyOnce) (i : Fin (mem ℓ)) :
    (Values L mem).allTicks bs i = (bs i).sum := rfl
@[den] theorem values_allTicks_totalOrder {β : Type} [DecidableEq β]
    (bs : (Values L mem).AtomicStream ℓ β .totalOrder .exactlyOnce) (i : Fin (mem ℓ)) :
    (Values L mem).allTicks bs i = (bs i).flatten := rfl
@[den] theorem values_broadcast_closed {c : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c α ord ret) (p : Fin (mem ℓ)) :
    (Values L mem).broadcast_closed d s p = KeyedPool.transport (fun j => s j) := rfl
@[den] theorem values_values {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries}
    (k : (Values L mem).KeyedStream ℓ K V ord ret) (i : Fin (mem ℓ)) :
    (Values L mem).values k i = (k i).values := rfl
/-- `.values()` of the network's keyed stream (read under `den`, after
`values_values`/`values_broadcast_closed`): the senders' pools summed,
in member order. -/
@[den] theorem KeyedPool.values_transport {c : Nat} {α : Type} [DecidableEq α]
    (f : Fin c → Multiset α) :
    (KeyedPool.transport (ord := .noOrder) (ret := .exactlyOnce) f).values
      = ((List.finRange c).map f).sum := rfl
@[den] theorem KeyedPool.values_transport_totalOrder {c : Nat} {α : Type} [DecidableEq α]
    (f : Fin c → List α) :
    (KeyedPool.transport (ord := .totalOrder) (ret := .exactlyOnce) f).values
      = ((List.finRange c).map (fun j => (↑(f j) : Multiset α))).sum := rfl
/-- `values ∘ broadcast_closed` at one receiver: the senders' pools,
summed (exactly-once; the network's `NoOrder` merge). -/
@[den] theorem values_broadcast_sum {c : L} {α : Type} [DecidableEq α]
    (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c α .noOrder .exactlyOnce) (p : Fin (mem ℓ)) :
    (Values L mem).values ((Values L mem).broadcast_closed d s) p
      = ((List.finRange (mem c)).map (fun j => s j)).sum := rfl
@[den] theorem values_broadcast_sum_totalOrder {c : L} {α : Type} [DecidableEq α]
    (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c α .totalOrder .exactlyOnce) (p : Fin (mem ℓ)) :
    (Values L mem).values ((Values L mem).broadcast_closed d s) p
      = ((List.finRange (mem c)).map (fun j => (↑(s j) : Multiset α))).sum := rfl
/-- `.broadcast(…).values()` at the retry grade: membership is
membership at some sender (the grouped pool merges every member's
support). -/
@[den] theorem values_broadcast_mem_alo {c : L} {α : Type} [DecidableEq α]
    (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c α .noOrder .atLeastOnce) (p : Fin (mem ℓ)) (b : α) :
    b ∈ (Values L mem).values ((Values L mem).broadcast_closed d s) p
      ↔ ∃ j, b ∈ s j := by
  show b ∈ RetryPool.mk _ ↔ _
  rw [RetryPool.mem_mk]
  show b ∈ ∑ k ∈ (Finset.univ : Finset (Fin (mem c))), KeyedPool.grpMS (s k) ↔ _
  rw [Multiset.mem_sum]
  constructor
  · rintro ⟨j, -, hj⟩
    exact ⟨j, RetryPool.mem_support_val.mp hj⟩
  · rintro ⟨j, hj⟩
    exact ⟨j, Finset.mem_univ j, RetryPool.mem_support_val.mpr hj⟩
/-- The same at the stutter grade (`sample_every` outputs): membership
in some sender's destuttered sequence. -/
@[den] theorem values_broadcast_mem_stutter {c : L} {α : Type} [DecidableEq α]
    (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c α .totalOrder .atLeastOnce) (p : Fin (mem ℓ)) (b : α) :
    b ∈ (Values L mem).values ((Values L mem).broadcast_closed d s) p
      ↔ ∃ j, b ∈ (↑(s j).norm : Multiset α) := by
  show b ∈ RetryPool.mk _ ↔ _
  rw [RetryPool.mem_mk]
  show b ∈ ∑ k ∈ (Finset.univ : Finset (Fin (mem c))), KeyedPool.grpMS (s k) ↔ _
  rw [Multiset.mem_sum]
  constructor
  · rintro ⟨j, -, hj⟩
    exact ⟨j, hj⟩
  · rintro ⟨j, hj⟩
    exact ⟨j, Finset.mem_univ j, hj⟩
@[den] theorem values_demux_sum {c : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c (Nat × α) ord .exactlyOnce)
    (addr : Fin (mem ℓ) → Nat) (i : Fin (mem ℓ)) :
    (Values L mem).values ((Values L mem).demux d s addr) i
      = ((List.finRange (mem c)).map (fun j => poolWeakenOrder (ord := ord)
          (filterMapPool (ord := ord)
            (fun dx => if dx.1 = addr i then some dx.2 else none) (s j)))).sum := by
  cases ord <;> rfl
@[den] theorem values_into_keyed {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} (s : (Values L mem).Stream ℓ (K × V) ord .exactlyOnce) (i : Fin (mem ℓ)) :
    (Values L mem).into_keyed s i = KeyedPool.ofPool (ord := ord) (s i) := rfl
@[den] theorem values_kassume_ordering {K V : Type} [DecidableEq K] [DecidableEq V]
    (u : (Values L mem).KeyedStream ℓ K V .noOrder .exactlyOnce)
    (d : (Values L mem).OrderSelDec (mem ℓ) (K × V)) (i : Fin (mem ℓ)) :
    (Values L mem).kassume_ordering u d i
      = KeyedPool.ofList (selectOrder (u i).toMultiset (d i)) := rfl
@[den] theorem values_kfold_early_stop {K V A : Type} [DecidableEq K] [DecidableEq V]
    [DecidableEq A] (ks : (Values L mem).KeyedStream ℓ K V .totalOrder .exactlyOnce)
    (f : A → V → A × Bool) (init : A) (i : Fin (mem ℓ)) (d : CutDec (K × V) .noOrder) :
    (Values L mem).kfold_early_stop ks f init i d
      = ((ks i).kviews 0 d).map (fun M => KeyedMap.ofFoldEarlyStop f init ((ks i).readAt M)) := rfl
@[den] theorem values_kget_max_key {α K V : Type} [DecidableEq α] [DecidableEq K]
    [DecidableEq V] [LinearOrder K] {ord : StrOrd} {ret : Retries}
    (ks : (Values L mem).KeyedSingleton ℓ α K V ord ret .boundedValue) (i : Fin (mem ℓ))
    (d : CutDec α .noOrder) :
    (Values L mem).kget_max_key ks i d = (ks i d).map KeyedMap.maxEntry := rfl
@[den] theorem values_tassume_ordering {α : Type} [DecidableEq α]
    {ℓ : L} (s : (Values L mem).AtomicStream ℓ α .noOrder .exactlyOnce)
    (d : (Values L mem).TickOrderSelDec (mem ℓ) α) (i : Fin (mem ℓ)) :
    (Values L mem).tassume_ordering s d i = realizeTicks (s i) (d i) := rfl

@[den] theorem values_tinto_keyed {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} (x : (Values L mem).AtomicStream ℓ (K × V) ord .exactlyOnce) (i : Fin (mem ℓ)) :
    (Values L mem).tinto_keyed x i = (x i).map (KeyedPool.ofPool (ord := ord)) := rfl
@[den] theorem values_reduce_watermark {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd}
    (ks : (Values L mem).AtomicKeyedStream ℓ K V ord .exactlyOnce)
    (wm : (Values L mem).Atomic ℓ (Option K))
    (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOk ord .exactlyOnce Pre (reduceStep f)) (i : Fin (mem ℓ)) :
    (Values L mem).reduce_watermark ks wm f Pre ok i
      = scanAcrossTicksTrace (reduceWatermarkStep f Pre ok) (KeyedPool.bot, none)
          (Trace.zip (ks i) (wm i)) := rfl

/-! The stream-level selection/fold/snapshot operators a ghost meets at
an ordered fold's tick reads (`assume_ordering` → `fold` → `snapshot`),
read at one member. -/
@[den] theorem values_assume_ordering {α : Type} [DecidableEq α]
    (u : (Values L mem).Stream ℓ α .noOrder .exactlyOnce)
    (d : (Values L mem).OrderSelDec (mem ℓ) α) (i : Fin (mem ℓ)) :
    (Values L mem).assume_ordering u d i = selectOrder (u i) (d i) := rfl
@[den] theorem values_snapshot_fold_totalOrder {α σ : Type} [DecidableEq α]
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk .totalOrder .exactlyOnce Pre g)
    (s : (Values L mem).Stream ℓ α .totalOrder .exactlyOnce)
    (d : (Values L mem).SnapDec (mem ℓ) α .totalOrder) (i : Fin (mem ℓ)) :
    (Values L mem).snapshot ((Values L mem).fold g init Pre ok s) d i
      = (prefixCuts (s i) 0 (d i)).map (fun v => v.foldl g init) := rfl

/-- The keyed-singleton chain's tick reads (`fold_early_stop` →
`get_max_key` → `snapshot`): `get_max_key` over the buckets of the
realized cumulative views of the keyed source. -/
@[den] theorem values_snapshot_kget_max_key {α K V : Type} [DecidableEq α] [DecidableEq K]
    [DecidableEq V] [LinearOrder K] {ord : StrOrd} {ret : Retries}
    (ks : (Values L mem).KeyedSingleton ℓ α K V ord ret .boundedValue)
    (d : (Values L mem).SnapDec (mem ℓ) α .noOrder) (i : Fin (mem ℓ)) :
    (Values L mem).snapshot ((Values L mem).kget_max_key ks) d i
      = (ks i (d i)).map KeyedMap.maxEntry := rfl

/-! ### The in-tick operators at the denotation — the `den` simp set

A `tick` block's generated step applies the interpretation's in-tick
fields to this tick's values; at `Values` each field IS the pool
operation (`Grades.lean`) and each pool operation at a concrete grade IS
the `List`/`Multiset` method (`den` there). `simp only [<out>_step,
den]` therefore reads one tick of the PROGRAM as the plain term of
its Rust lines — the ghost states facts about it directly (FINDINGS D64,
E6: no pure twin of a body, no body-shape bridge). -/
section TickBody
variable {α β σ : Type} [DecidableEq α] [DecidableEq β] {ord : StrOrd}

@[den] theorem values_bmap (b : (Values L mem).BoundedStream α ord .exactlyOnce)
    (f : α → β) : (Values L mem).bmap b f = mapPool (ord := ord) f b := rfl
@[den] theorem values_bfilterMap (b : (Values L mem).BoundedStream α ord .exactlyOnce)
    (f : α → Option β) : (Values L mem).bfilterMap b f = filterMapPool (ord := ord) f b := rfl
@[den] theorem values_bflatMapOrdered
    (l : (Values L mem).BoundedStream α .totalOrder .exactlyOnce) (f : α → List β) :
    (Values L mem).bflatMapOrdered l f = l.flatMap f := rfl
@[den] theorem values_bflatMapUnordered
    (b : (Values L mem).BoundedStream α ord .exactlyOnce) (f : α → Multiset β) :
    (Values L mem).bflatMapUnordered b f = poolFlatMapUnordered (ord := ord) b f := rfl
@[den] theorem values_bofList (l : List α) :
    (Values L mem).bofList l = (↑l : Multiset α) := rfl
@[den] theorem values_bfold {ret : Retries} (g : σ → α → σ) (init : σ)
    (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk ord ret Pre g) (b : (Values L mem).BoundedStream α ord ret) :
    (Values L mem).bfold g init Pre ok b = PoolFold ord ret Pre g init ok b := rfl
@[den] theorem values_bcount (b : (Values L mem).BoundedStream α ord .exactlyOnce) :
    (Values L mem).bcount b = poolCount (ord := ord) b := rfl
@[den] theorem values_benumerate
    (l : (Values L mem).BoundedStream α .totalOrder .exactlyOnce) :
    (Values L mem).benumerate l = listEnumerate l := rfl
@[den] theorem values_bfirst
    (l : (Values L mem).BoundedStream α .totalOrder .exactlyOnce) :
    (Values L mem).bfirst l = l.head? := rfl
@[den] theorem values_bcrossSingleton [DecidableEq σ]
    (b : (Values L mem).BoundedStream α ord .exactlyOnce)
    (s : (Values L mem).BoundedSingleton σ) :
    (Values L mem).bcrossSingleton b s = mapPool (ord := ord) (fun a => ((a, s) : α × σ)) b := rfl
@[den] theorem values_bchain (a b : (Values L mem).BoundedStream α ord .exactlyOnce) :
    (Values L mem).bchain a b = poolChain (ord := ord) a b := rfl
@[den] theorem values_bweakenOrder (b : (Values L mem).BoundedStream α ord .exactlyOnce) :
    (Values L mem).bweakenOrder b = poolWeakenOrder (ord := ord) b := rfl
@[den] theorem values_bfilter (b : (Values L mem).BoundedStream α ord .exactlyOnce)
    (p : α → Bool) : (Values L mem).bfilter b p = poolFilter (ord := ord) p b := rfl
@[den] theorem values_binto_keyed {K V : Type} [DecidableEq K] [DecidableEq V]
    (b : (Values L mem).BoundedStream (K × V) ord .exactlyOnce) :
    (Values L mem).binto_keyed b = KeyedPool.ofPool (ord := ord) b := rfl
/-- `into_keyed()` by grade (the `ofPool` dispatch, read under `den`). -/
@[den] theorem KeyedPool.ofPool_noOrder {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : Multiset (K × V)) : KeyedPool.ofPool (ord := .noOrder) m = KeyedPool.ofMultiset m := rfl
@[den] theorem KeyedPool.ofPool_totalOrder {K V : Type} [DecidableEq K] [DecidableEq V]
    (l : List (K × V)) : KeyedPool.ofPool (ord := .totalOrder) l = KeyedPool.ofList l := rfl
@[den] theorem values_bkfold {K V A : Type} [DecidableEq K] [DecidableEq V]
    [DecidableEq A] [Hashable K] {ret : Retries} (g : A → V → A) (init : A)
    (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g)
    (ks : (Values L mem).BoundedKeyedStream K V ord ret) :
    (Values L mem).bkfold g init Pre ok ks = KeyedMap.ofFold g init Pre ok ks := rfl
@[den] theorem values_bksFilter {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (Values L mem).BoundedKeyedSingleton K V) (p : V → Bool) :
    (Values L mem).bksFilter m p = m.filter p := rfl
@[den] theorem values_bksMap {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]
    (m : (Values L mem).BoundedKeyedSingleton K V) (f : V → W) :
    (Values L mem).bksMap m f = m.map f := rfl
@[den] theorem values_bksKeys {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (Values L mem).BoundedKeyedSingleton K V) :
    (Values L mem).bksKeys m = m.keysMS := rfl
@[den] theorem values_bksEntries {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (Values L mem).BoundedKeyedSingleton K V) :
    (Values L mem).bksEntries m = m.entries := rfl
@[den] theorem values_bjoin {K V W : Type} [DecidableEq K] [Hashable K] [DecidableEq V]
    [DecidableEq W] {ord' : StrOrd}
    (a : (Values L mem).BoundedStream (K × V) ord .exactlyOnce)
    (b : (Values L mem).BoundedStream (K × W) ord' .exactlyOnce) :
    (Values L mem).bjoin a b = poolJoin (ord := ord) (ord' := ord') a b := rfl
@[den] theorem values_bantiJoin {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V]
    {ord' : StrOrd} (a : (Values L mem).BoundedStream (K × V) ord .exactlyOnce)
    (ks : (Values L mem).BoundedStream K ord' .exactlyOnce) :
    (Values L mem).bantiJoin a ks = poolAntiJoin (ord := ord) (ord' := ord') a ks := rfl
@[den] theorem values_bfilterNotIn [Hashable α] {ord' : StrOrd}
    (a : (Values L mem).BoundedStream α ord .exactlyOnce)
    (o : (Values L mem).BoundedStream α ord' .exactlyOnce) :
    (Values L mem).bfilterNotIn a o = poolFilterNotIn (ord := ord) (ord' := ord') a o := rfl
@[den] theorem values_bmax [LinearOrder α]
    (b : (Values L mem).BoundedStream α ord .exactlyOnce) :
    (Values L mem).bmax b = poolMax (ord := ord) b := rfl
@[den] theorem values_bfilterIf {ret : Retries}
    (b : (Values L mem).BoundedStream α ord ret) (flag : (Values L mem).BoundedSingleton Bool) :
    (Values L mem).bfilterIf b flag = poolFilterIf b flag := rfl
@[den] theorem values_bsPure (v : σ) :
    (Values L mem).bsPure v = v := rfl
@[den] theorem values_bsMap {τ : Type} (s : (Values L mem).BoundedSingleton σ)
    (f : σ → τ) : (Values L mem).bsMap s f = f s := rfl
@[den] theorem values_bsZip {τ : Type} (a : (Values L mem).BoundedSingleton σ)
    (b : (Values L mem).BoundedSingleton τ) :
    (Values L mem).bsZip a b = (a, b) := rfl
@[den] theorem values_boMap {τ : Type} (o : (Values L mem).BoundedOptional σ)
    (f : σ → τ) : (Values L mem).boMap o f = o.map f := rfl
@[den] theorem values_boUnwrapOr (o : (Values L mem).BoundedOptional σ)
    (s : (Values L mem).BoundedSingleton σ) :
    (Values L mem).boUnwrapOr o s = o.getD s := rfl
@[den] theorem values_boFilter (o : (Values L mem).BoundedOptional σ) (p : σ → Bool) :
    (Values L mem).boFilter o p = o.filter p := rfl
@[den] theorem values_boIsSome (o : (Values L mem).BoundedOptional σ) :
    (Values L mem).boIsSome o = o.isSome := rfl
end TickBody

end ValuesTick

end Hydro
