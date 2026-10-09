import Hydro.Sem

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

/-- The `Values` ticked carrier (`HydroSem.Ticked`): the realized
per-member tick sequence of a per-tick value. -/
abbrev TickV (n : Nat) (σ : Type) : Type := Fin n → Trace σ

/-! ### Tick shapes at the denotation: slicing a shaped input into
per-tick tuples and un-slicing a per-tick output trace into wires -/

namespace ValuesTick
variable {L : Type} {mem : L → Nat}

/-- The denotation's ticked carriers, as `TickedOf` parameters. -/
abbrev Tk (mem : L → Nat) (ℓ : L) (τ : Type) : Type := Fin (mem ℓ) → Trace τ
abbrev TS (mem : L → Nat) (ℓ : L) (α : Type) [DecidableEq α] (ord : StrOrd)
    (ret : Retries) : Type := Fin (mem ℓ) → Trace (PoolCarrier α ord ret)
/-- The denotation's bounded families, as `BoundedOf` parameters. -/
abbrev BS (σ : Type) : Type := σ

/-- Member `i`'s per-tick input tuples (the zip of the wires' traces). -/
def slice {ℓ : L} : (sh : TickShape) → TickedOf (Tk mem) (TS mem) ℓ sh →
    Fin (mem ℓ) → Trace (BoundedOf BS PoolCarrier sh)
  | .sing _, x, i => x i
  | .stream _ _ _ _, x, i => x i
  | .pair a b, x, i => Trace.zip (slice a x.1 i) (slice b x.2 i)

/-- A per-member trace of output tuples, as the output wires. -/
def unslice {ℓ : L} : (sh : TickShape) →
    (Fin (mem ℓ) → Trace (BoundedOf BS PoolCarrier sh)) →
    TickedOf (Tk mem) (TS mem) ℓ sh
  | .sing _, f => f
  | .stream _ _ _ _, f => f
  | .pair a b, f =>
    (unslice a (fun i => (f i).map Prod.fst),
     unslice b (fun i => (f i).map Prod.snd))

/-- The initial register of a shape from its seed (a stream register
starts empty). -/
def seed : (sh : TickShape) → SeedOf sh → BoundedOf BS PoolCarrier sh
  | .sing _, v => v
  | .stream _ _ ord ret, _ => PoolBot ord ret
  | .pair a b, s => (seed a s.1, seed b s.2)

/-- The tick former at the denotation: the structural fold over each
member's ticks, on the sliced inputs. -/
def scan {ℓ : L} (sts ins outs : TickShape)
    (x : TickedOf (Tk mem) (TS mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier sts →
      BoundedOf BS PoolCarrier ins →
      BoundedOf BS PoolCarrier sts × BoundedOf BS PoolCarrier outs)
    (init : SeedOf sts) : TickedOf (Tk mem) (TS mem) ℓ outs :=
  unslice outs (fun i => scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i))

/-- Leafwise trace prefix between two shaped values. -/
def Le {ℓ : L} : (sh : TickShape) →
    TickedOf (Tk mem) (TS mem) ℓ sh → TickedOf (Tk mem) (TS mem) ℓ sh → Prop
  | .sing _, a, b => ∀ i, a i <+: b i
  | .stream _ _ _ _, a, b => ∀ i, a i <+: b i
  | .pair sa sb, a, b => Le sa a.1 b.1 ∧ Le sb a.2 b.2

theorem slice_prefix {ℓ : L} : (sh : TickShape) →
    (a b : TickedOf (Tk mem) (TS mem) ℓ sh) → Le sh a b →
    ∀ i, slice sh a i <+: slice sh b i
  | .sing _, _, _, h, i => h i
  | .stream _ _ _ _, _, _, h, i => h i
  | .pair sa sb, a, b, h, i =>
    zip_prefix (slice_prefix sa a.1 b.1 h.1 i) (slice_prefix sb a.2 b.2 h.2 i)

theorem unslice_le {ℓ : L} : (sh : TickShape) →
    (F G : Fin (mem ℓ) → Trace (BoundedOf BS PoolCarrier sh)) →
    (∀ i, F i <+: G i) → Le sh (unslice sh F) (unslice sh G)
  | .sing _, _, _, h => h
  | .stream _ _ _ _, _, _, h => h
  | .pair sa sb, F, G, h =>
    ⟨unslice_le sa _ _ (fun i => List.IsPrefix.map _ (h i)),
     unslice_le sb _ _ (fun i => List.IsPrefix.map _ (h i))⟩

/-- The tick former preserves leafwise prefix (the monotonicity face). -/
theorem scan_le {ℓ : L} (sts ins outs : TickShape)
    (a b : TickedOf (Tk mem) (TS mem) ℓ ins) (h : Le ins a b)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier sts → BoundedOf BS PoolCarrier ins →
      BoundedOf BS PoolCarrier sts × BoundedOf BS PoolCarrier outs) (init : SeedOf sts) :
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
  Stream ℓ α _ ord ret := Fin (mem ℓ) → PoolCarrier α ord ret
  TransportDec _ _ := Unit
  OrderSelDec n α := OrderSelection n α
  SnapDec n α ord := SnapshotCuts n α ord
  BatchDec n α := BatchCuts n α
  OrdBatchDec n := OrderedBatchCuts n
  SampleDec n := SampleTimes n
  TimerDec n := TimerVerdicts n
  PulseDec n := TimingPulses n
  FixDec := UnfoldFuel
  KeyedStream p c α _ ord ret :=
    Fin (mem p) → Fin (mem c) → PoolCarrier α ord ret
  Singleton ℓ α σ _ ord _ret b := SingletonV (mem ℓ) α σ ord b
  Ticked ℓ σ := TickV (mem ℓ) σ
  -- one tick's bounded stream: the graded content quotient
  BoundedStream α _ ord ret := PoolCarrier α ord ret
  BoundedSingleton σ := σ
  -- `TickStream` is the signature default: `Ticked ℓ (BoundedStream …)`
  -- = `Fin (mem ℓ) → Trace (PoolCarrier α ord ret)`
  map {_ℓ _α _β _ _ ord} s f := fun i => mapPool (ord := ord) (f i) (s i)
  filterMap {_ℓ _α _β _ _ ord} s f := fun i =>
    filterMapPool (ord := ord) (f i) (s i)
  broadcast_closed _d s := fun _p j => s j
  demux {_c _p _α _ ord} _d s addr := fun i j =>
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
  union {_ℓ _α _ ret} a b :=
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
    | .monotonic _vo, s => fun i => (s i).val (d i)
  batch s d := fun i => batchCuts (s i) 0 (d i)
  batch_ordered s d := fun i => sliceCuts (s i) 0 (d i)
  sample_every t d := fun i => StutterSeq.mk (sampleAtOpt (t i) (d i))
  timeout_snapshot _s d := fun i => d i
  source_interval_batch d := fun i => d i
  mapTick s f := fun i => (s i).map (f i)
  zipTick a b := fun i => Trace.zip (a i) (b i)
  defer_tick init t := fun i => init :: t i
  allTicks {_ℓ _β _ ord} bs :=
    match ord, bs with
    | .totalOrder, bs => fun i => (bs i).flatten
    | .noOrder, bs => fun i => (bs i).sum
  flattenOrdered t := fun i => t i
  flattenUnordered t := fun i => (t i).map (fun l => Multiset.ofList l)
  fix_stream {_ℓ _α _ ord ret} fuel body :=
    iterate body (fun _i => PoolBot ord ret) fuel
  fix_tick fuel body := iterate body (fun _i => []) fuel
  -- in-tick operators: the quotient's own operations
  bmap {_α _β _ _ ord} b f := mapPool (ord := ord) f b
  bfilterMap {_α _β _ _ ord} b f := filterMapPool (ord := ord) f b
  bflatMapOrdered l f := l.flatMap f
  bflatMapUnordered {_α _β _ _ ord} b f :=
    poolFlatMapUnordered (ord := ord) b f
  bofList l := (↑l : Multiset _)
  bcount {_α _ ord} b := poolCount (ord := ord) b
  bfold {_α _σ _ ord ret} g init ok b := PoolFold ord ret g init ok b
  benumerate l := listEnumerate l
  bfirst l := l.head?
  bcrossSingleton {_α _σ _ _ ord} b s := mapPool (ord := ord) (fun a => (a, s)) b
  bchain {_α _ ord} a b := poolChain (ord := ord) a b
  bweakenOrder {_α _ ord} b := poolWeakenOrder (ord := ord) b
  bfilter {_α _ ord} b p := poolFilter (ord := ord) p b
  bkeyedFold {_K _V _A _ _ _ ord} g init ok b := poolKeyedFold (ord := ord) g init ok b
  bkeys e := Multiset.map Prod.fst e
  bjoin {_K _V _W _ _ _ ord ord'} a b := poolJoin (ord := ord) (ord' := ord') a b
  bantiJoin {_K _V _ _ ord ord'} a ks := poolAntiJoin (ord := ord) (ord' := ord') a ks
  bfilterNotIn {_α _ ord ord'} a o := poolFilterNotIn (ord := ord) (ord' := ord') a o
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
theorem slice_pair (a b : TickShape)
    (x : TickedOf (Tk mem) (TS mem) ℓ (.pair a b)) (i : Fin (mem ℓ)) :
    slice (.pair a b) x i = Trace.zip (slice a x.1 i) (slice b x.2 i) := rfl

theorem tick_scan_sing {τ : Type} (sts ins : TickShape)
    (x : TickedOf (Tk mem) (TS mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier sts → BoundedOf BS PoolCarrier ins →
      BoundedOf BS PoolCarrier sts × BoundedOf BS PoolCarrier (.sing τ))
    (init : SeedOf sts) (i : Fin (mem ℓ)) :
    (Values L mem).tick_scan sts ins (.sing τ) x g init i
      = scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i) := rfl
theorem tick_scan_stream {α : Type} [inst : DecidableEq α] {ord : StrOrd}
    {ret : Retries} (sts ins : TickShape)
    (x : TickedOf (Tk mem) (TS mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier sts → BoundedOf BS PoolCarrier ins →
      BoundedOf BS PoolCarrier sts × BoundedOf BS PoolCarrier (.stream α inst ord ret))
    (init : SeedOf sts) (i : Fin (mem ℓ)) :
    (Values L mem).tick_scan sts ins (.stream α inst ord ret) x g init i
      = scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i) := rfl
theorem seed_sing {τ : Type} (v : τ) : seed (.sing τ) v = v := rfl
theorem seed_stream {α : Type} [inst : DecidableEq α] {ord : StrOrd} {ret : Retries}
    (u : Unit) : seed (.stream α inst ord ret) u = PoolBot ord ret := rfl
theorem seed_pair (a b : TickShape) (s : SeedOf (.pair a b)) :
    seed (.pair a b) s = (seed a s.1, seed b s.2) := rfl
/-- The output legs of a multi-output block: the tuple trace, projected. -/
theorem tick_scan_pair (sts ins a b : TickShape)
    (x : TickedOf (Tk mem) (TS mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier sts → BoundedOf BS PoolCarrier ins →
      BoundedOf BS PoolCarrier sts × BoundedOf BS PoolCarrier (.pair a b))
    (init : SeedOf sts) :
    (Values L mem).tick_scan sts ins (.pair a b) x g init
      = (unslice a (fun i => (scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i)).map Prod.fst),
         unslice b (fun i => (scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i)).map Prod.snd)) := rfl
theorem unslice_sing {τ : Type} (f : Fin (mem ℓ) → Trace (BoundedOf BS PoolCarrier (.sing τ)))
    (i : Fin (mem ℓ)) : unslice (.sing τ) f i = f i := rfl
theorem unslice_stream {α : Type} [inst : DecidableEq α] {ord : StrOrd} {ret : Retries}
    (f : Fin (mem ℓ) → Trace (BoundedOf BS PoolCarrier (.stream α inst ord ret)))
    (i : Fin (mem ℓ)) : unslice (.stream α inst ord ret) f i = f i := rfl
theorem unslice_pair (a b : TickShape)
    (f : Fin (mem ℓ) → Trace (BoundedOf BS PoolCarrier (.pair a b))) :
    unslice (.pair a b) f
      = (unslice a (fun i => (f i).map Prod.fst), unslice b (fun i => (f i).map Prod.snd)) := rfl

/-- The wire-level reads a ghost meets at a `tick` block's inputs. -/
@[den] theorem values_zipTick {σ τ : Type} (a : (Values L mem).Ticked ℓ σ)
    (b : (Values L mem).Ticked ℓ τ) (i : Fin (mem ℓ)) :
    (Values L mem).zipTick a b i = Trace.zip (a i) (b i) := rfl
@[den] theorem values_defer_tick {σ : Type} (v : σ) (t : (Values L mem).Ticked ℓ σ)
    (i : Fin (mem ℓ)) :
    (Values L mem).defer_tick v t i = v :: t i := rfl
@[den] theorem values_flattenUnordered {β : Type} [DecidableEq β]
    (t : (Values L mem).Ticked ℓ (List β)) (i : Fin (mem ℓ)) :
    (Values L mem).flattenUnordered t i = (t i).map (fun l => Multiset.ofList l) := rfl
@[den] theorem values_batch_ordered {α : Type} [DecidableEq α]
    (s : (Values L mem).Stream ℓ α .totalOrder .exactlyOnce)
    (d : (Values L mem).OrdBatchDec (mem ℓ)) (i : Fin (mem ℓ)) :
    (Values L mem).batch_ordered s d i = sliceCuts (s i) 0 (d i) := rfl
@[den] theorem values_batch {α : Type} [DecidableEq α]
    (s : (Values L mem).Stream ℓ α .noOrder .exactlyOnce)
    (d : (Values L mem).BatchDec (mem ℓ) α) (i : Fin (mem ℓ)) :
    (Values L mem).batch s d i = batchCuts (s i) 0 (d i) := rfl
@[den] theorem values_mapTick {σ τ : Type} (t : (Values L mem).Ticked ℓ σ)
    (f : Fin (mem ℓ) → σ → τ) (i : Fin (mem ℓ)) :
    (Values L mem).mapTick t f i = (t i).map (f i) := rfl
@[den] theorem values_flattenOrdered {β : Type} [DecidableEq β]
    (t : (Values L mem).Ticked ℓ (List β)) (i : Fin (mem ℓ)) :
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
    (addr : Fin (mem ℓ) → Nat) (i : Fin (mem ℓ)) (j : Fin (mem c)) :
    (Values L mem).demux d s addr i j
      = filterMapPool (ord := ord) (fun dx => if dx.1 = addr i then some dx.2 else none) (s j) := rfl

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
    (bs : (Values L mem).TickStream ℓ β .noOrder .exactlyOnce) (i : Fin (mem ℓ)) :
    (Values L mem).allTicks bs i = (bs i).sum := rfl
@[den] theorem values_allTicks_totalOrder {β : Type} [DecidableEq β]
    (bs : (Values L mem).TickStream ℓ β .totalOrder .exactlyOnce) (i : Fin (mem ℓ)) :
    (Values L mem).allTicks bs i = (bs i).flatten := rfl
@[den] theorem values_broadcast_closed {c : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c α ord ret) (p : Fin (mem ℓ)) (j : Fin (mem c)) :
    (Values L mem).broadcast_closed d s p j = s j := rfl
@[den] theorem values_values_noOrder {c : L} {α : Type} [DecidableEq α]
    (k : (Values L mem).KeyedStream ℓ c α .noOrder .exactlyOnce) (i : Fin (mem ℓ)) :
    (Values L mem).values k i = ((List.finRange (mem c)).map (fun j => k i j)).sum := rfl
@[den] theorem values_values_totalOrder {c : L} {α : Type} [DecidableEq α]
    (k : (Values L mem).KeyedStream ℓ c α .totalOrder .exactlyOnce) (i : Fin (mem ℓ)) :
    (Values L mem).values k i
      = ((List.finRange (mem c)).map (fun j => (↑(k i j) : Multiset α))).sum := rfl

/-! The stream-level selection/fold/snapshot operators a ghost meets at
an ordered fold's tick reads (`assume_ordering` → `fold` → `snapshot`),
read at one member. -/
@[den] theorem values_assume_ordering {α : Type} [DecidableEq α]
    (u : (Values L mem).Stream ℓ α .noOrder .exactlyOnce)
    (d : (Values L mem).OrderSelDec (mem ℓ) α) (i : Fin (mem ℓ)) :
    (Values L mem).assume_ordering u d i = selectOrder (u i) (d i) := rfl
@[den] theorem values_snapshot_fold_totalOrder {α σ : Type} [DecidableEq α]
    (g : σ → α → σ) (init : σ) (ok : FoldOk .totalOrder .exactlyOnce g)
    (s : (Values L mem).Stream ℓ α .totalOrder .exactlyOnce)
    (d : (Values L mem).SnapDec (mem ℓ) α .totalOrder) (i : Fin (mem ℓ)) :
    (Values L mem).snapshot ((Values L mem).fold g init ok s) d i
      = (prefixCuts (s i) 0 (d i)).map (fun v => v.foldl g init) := rfl

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
    (ok : FoldOk ord ret g) (b : (Values L mem).BoundedStream α ord ret) :
    (Values L mem).bfold g init ok b = PoolFold ord ret g init ok b := rfl
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
@[den] theorem values_bkeyedFold {K V A : Type} [DecidableEq K] [DecidableEq V]
    [DecidableEq A] (g : A → V → A) (init : A) (ok : FoldOk ord .exactlyOnce g)
    (b : (Values L mem).BoundedStream (K × V) ord .exactlyOnce) :
    (Values L mem).bkeyedFold g init ok b = poolKeyedFold (ord := ord) g init ok b := rfl
@[den] theorem values_bkeys {K V : Type} [DecidableEq K] [DecidableEq V]
    (e : (Values L mem).BoundedStream (K × V) .noOrder .exactlyOnce) :
    (Values L mem).bkeys e = Multiset.map Prod.fst e := rfl
@[den] theorem values_bjoin {K V W : Type} [DecidableEq K] [DecidableEq V]
    [DecidableEq W] {ord' : StrOrd}
    (a : (Values L mem).BoundedStream (K × V) ord .exactlyOnce)
    (b : (Values L mem).BoundedStream (K × W) ord' .exactlyOnce) :
    (Values L mem).bjoin a b = poolJoin (ord := ord) (ord' := ord') a b := rfl
@[den] theorem values_bantiJoin {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord' : StrOrd} (a : (Values L mem).BoundedStream (K × V) ord .exactlyOnce)
    (ks : (Values L mem).BoundedStream K ord' .exactlyOnce) :
    (Values L mem).bantiJoin a ks = poolAntiJoin (ord := ord) (ord' := ord') a ks := rfl
@[den] theorem values_bfilterNotIn {ord' : StrOrd}
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
