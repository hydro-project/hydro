import Hydro.Values

/-!
# The `Eager` interpretation — the `Values` denotation, materialized

`Values` carriers are member-indexed *functions*, so evaluation is
call-by-name: every wire access re-runs its producing chain, and the
re-evaluation compounds multiplicatively through module composition and
Kleene knots (FINDINGS D14; the naive `paxos_core` execution needs
hours). `Eager` is the **same denotation with a data representation**:
each carrier pairs

- `data` — a `Vector` of the member cells, forced **once** when the op
  is applied (evaluation order = dataflow order, so a compiled program
  at `Eager` executes in one strict pass); with
- `den` — the `Values` carrier it means; and
- `agree` — the proof that the data *is* the denotation, member by
  member.

**No formula is ever re-spelled**: an op's `data` leg is the `Values`
op itself applied to the materialized inputs (`Vector.ofFn ∘ Values-op
∘ EPack.fn`), and its `den` leg is the `Values` op applied to the
inputs' `den` legs — so the per-op projection identities
(`EagerProj.lean`) are `rfl`, and the carrier-level `agree` is one
uniform rewrite (`EPack.fn_eq`). Divergence between execution and
denotation is ill-typed at every op.

The one seam that stays functional (deliberately — user-ratified): a
folded, un-snapshotted `Singleton`'s cell is `CutDec → Trace σ` in both
legs. Its observation is decision-indexed (different snapshot sites may
legally observe different arrival chains), so it cannot be finished
into data before the site's cut decision arrives; the closure is over
the *materialized* pool, applied once per site — site-local, no
compounding.

Nondeterminism vocabulary: identical to `Values` (decisions are
content decisions; the same record drives both instances).
-/

namespace Hydro

/-- `Vector.ofFn` reads back: the materialization is the function. -/
theorem vget_ofFn {n : Nat} {α : Type _} (f : Fin n → α) (i : Fin n) :
    (Vector.ofFn f).get i = f i := by
  rcases i with ⟨iv, hi⟩
  exact Vector.getElem_ofFn hi

/-- A member-indexed wire, **materialized**: the data, the `Values`
denotation it means, and the pointwise agreement. -/
structure EPack (n : Nat) (C : Type _) where
  data : Vector C n
  den : Fin n → C
  agree : ∀ i, data.get i = den i

namespace EPack

variable {n : Nat} {C : Type _}

/-- Read the data as a member-indexed function (the shape `Values` ops
consume). -/
def fn (p : EPack n C) : Fin n → C := fun i => p.data.get i

/-- The data-as-function IS the denotation. -/
theorem fn_eq (p : EPack n C) : p.fn = p.den := funext p.agree

/-- Pack from data (executable inputs): the denotation is the data. -/
def ofData (d : Vector C n) : EPack n C :=
  ⟨d, fun i => d.get i, fun _ => rfl⟩

/-- Pack from a denotation (proof-side embeds; forces the function once
if ever evaluated). -/
def ofDen (v : Fin n → C) : EPack n C :=
  ⟨Vector.ofFn v, v, vget_ofFn v⟩

/-- Packs with equal legs are equal (`agree` is proof-irrelevant). -/
theorem ext' {a b : EPack n C} (h1 : a.data = b.data)
    (h2 : a.den = b.den) : a = b := by
  cases a; cases b
  cases h1; cases h2
  rfl

/-- Pointwise-agreeing data and denotation give the same pack through
the two constructors — the fix-knot induction's step lemma. -/
theorem ofData_eq_ofDen {d : Vector C n} {v : Fin n → C}
    (h : ∀ i, d.get i = v i) : ofData d = ofDen v := by
  refine ext' ?_ (funext h)
  refine Vector.ext (fun i hi => ?_)
  show d[i] = (Vector.ofFn v)[i]
  rw [Vector.getElem_ofFn]
  exact h ⟨i, hi⟩

/-- Uniform agreement for a one-input materialized op. -/
theorem mk_agree {D : Type _} (F : (Fin n → C) → Fin n → D)
    (p : EPack n C) :
    ∀ i, (Vector.ofFn (F p.fn)).get i = F p.den i := by
  intro i
  rw [vget_ofFn, p.fn_eq]

/-- Uniform agreement for a one-input materialized op across locations
(the network edge: `c` senders to `n` receivers). -/
theorem mk_agree_x {m : Nat} {D : Type _} (F : (Fin m → C) → Fin n → D)
    (p : EPack m C) :
    ∀ i, (Vector.ofFn (F p.fn)).get i = F p.den i := by
  intro i
  rw [vget_ofFn, p.fn_eq]

/-- Uniform agreement for a two-input materialized op. -/
theorem mk_agree₂ {C' D : Type _}
    (F : (Fin n → C) → (Fin n → C') → Fin n → D)
    (p : EPack n C) (q : EPack n C') :
    ∀ i, (Vector.ofFn (F p.fn q.fn)).get i = F p.den q.den i := by
  intro i
  rw [vget_ofFn, p.fn_eq, q.fn_eq]

/-- Uniform agreement for a decision-only materialized op. -/
theorem mk_agree₀ {D : Type _} (g : Fin n → D) :
    ∀ i, (Vector.ofFn g).get i = g i := vget_ofFn g

end EPack

/-- The `Eager` singleton carrier: the graded read cell (`Values`'s
`SingletonV` cells), materialized per member. -/
def ESing (n : Nat) (α σ : Type) [DecidableEq α] (ord : StrOrd) :
    SingBound σ → Type
  | .unbounded => EPack n (CutDec α ord → Trace σ)
  | .monotonic vo =>
    EPack n {f : CutDec α ord → Trace σ // ∀ d, Ascending vo (f d)}

/-- The `Eager` ticked carrier: the per-member tick sequences, as data
and as denotation, agreeing pointwise. -/
abbrev ETick (n : Nat) (σ : Type) : Type := EPack n (Trace σ)


/-! ### Tick shapes at `Eager`: the data leg and the denotation leg of a
shaped input, and their agreement -/
namespace EagerTick
variable {L : Type} {mem : L → Nat}
abbrev Tk (mem : L → Nat) (ℓ : L) (τ : Type) : Type := EPack (mem ℓ) (Trace τ)
abbrev TS (mem : L → Nat) (ℓ : L) (α : Type) [DecidableEq α] (ord : StrOrd)
    (ret : Retries) : Type := EPack (mem ℓ) (Trace (PoolCarrier α ord ret))
abbrev TKS (mem : L → Nat) (ℓ : L) (K V : Type) [DecidableEq K] [DecidableEq V] :
    Type := EPack (mem ℓ) (Trace (KeyedMap K V))
def fn {ℓ : L} : (sh : TickShape) → AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh →
    AtomicOf (Values L mem).Atomic (Values L mem).AtomicStream (Values L mem).AtomicKeyedSingleton ℓ sh
  | .sing _, x => x.fn
  | .stream _ _ _ _, x => x.fn
  | .ksing _ _ _ _, x => x.fn
  | .pair a b, x => (fn a x.1, fn b x.2)
def den {ℓ : L} : (sh : TickShape) → AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh →
    AtomicOf (Values L mem).Atomic (Values L mem).AtomicStream (Values L mem).AtomicKeyedSingleton ℓ sh
  | .sing _, x => x.den
  | .stream _ _ _ _, x => x.den
  | .ksing _ _ _ _, x => x.den
  | .pair a b, x => (den a x.1, den b x.2)
theorem fn_eq {ℓ : L} : (sh : TickShape) → (x : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh) →
    fn sh x = den sh x
  | .sing _, x => x.fn_eq
  | .stream _ _ _ _, x => x.fn_eq
  | .ksing _ _ _ _, x => x.fn_eq
  | .pair a b, x => by
    show (fn a x.1, fn b x.2) = (den a x.1, den b x.2)
    rw [fn_eq a x.1, fn_eq b x.2]
/-- Pack a data leg and a denotation leg that are provably the same
function, leaf by leaf. -/
def pack {ℓ : L} : (sh : TickShape) →
    (f d : AtomicOf (Values L mem).Atomic (Values L mem).AtomicStream (Values L mem).AtomicKeyedSingleton ℓ sh) →
    f = d → AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh
  | .sing τ, f, d, h =>
    let f' : (Values L mem).Atomic ℓ τ := f
    ⟨Vector.ofFn f', d, fun i => (vget_ofFn f' i).trans (congrFun h i)⟩
  | .stream α _ ord ret, f, d, h =>
    let f' : (Values L mem).AtomicStream ℓ α ord ret := f
    ⟨Vector.ofFn f', d, fun i => (vget_ofFn f' i).trans (congrFun h i)⟩
  | .ksing K V _ _, f, d, h =>
    let f' : (Values L mem).AtomicKeyedSingleton ℓ K V := f
    ⟨Vector.ofFn f', d, fun i => (vget_ofFn f' i).trans (congrFun h i)⟩
  | .pair a b, f, d, h =>
    (pack a f.1 d.1 (by rw [h]), pack b f.2 d.2 (by rw [h]))

theorem den_pack {ℓ : L} : (sh : TickShape) →
    (f d : AtomicOf (Values L mem).Atomic (Values L mem).AtomicStream (Values L mem).AtomicKeyedSingleton ℓ sh) →
    (h : f = d) → den sh (pack sh f d h) = d
  | .sing _, _, _, _ => rfl
  | .stream _ _ _ _, _, _, _ => rfl
  | .ksing _ _ _ _, _, _, _ => rfl
  | .pair a b, f, d, h => by
    show (den a (pack a f.1 d.1 _), den b (pack b f.2 d.2 _)) = d
    rw [den_pack a, den_pack b]
end EagerTick

set_option warn.classDefReducibility false in
/-- The materialized denotation: every op is the `Values` op — once on
data, once on denotations — with the agreement carried in the carrier.
Decision vocabulary identical to `Values`. -/
def Eager (L : Type) (mem : L → Nat) : HydroSem L mem where
  Stream ℓ α _ ord ret := EPack (mem ℓ) (PoolCarrier α ord ret)
  KeyedStream ℓ K V _ _ ord ret := EPack (mem ℓ) (KeyedPool K V ord ret)
  Singleton ℓ α σ _ ord _ret b := ESing (mem ℓ) α σ ord b
  KeyedSingleton ℓ α K V _ _ _ _ord _ret _b :=
    EPack (mem ℓ) (CutDec α .noOrder → Trace (KeyedMap K V))
  Atomic ℓ σ := ETick (mem ℓ) σ
  -- the data leg still runs on the denotation's quotients (a `List`
  -- data leg with `ofList data = den` agreement is the executable
  -- analogue of the Couple coupling — a later stage)
  BoundedStream α _ ord ret := PoolCarrier α ord ret
  BoundedSingleton σ := (Values L mem).BoundedSingleton σ
  BoundedKeyedStream K V _ _ ord ret := (Values L mem).BoundedKeyedStream K V ord ret
  BoundedKeyedSingleton K V _ _ := (Values L mem).BoundedKeyedSingleton K V
  -- `AtomicStream` is the signature default: `EPack n (Trace (PoolCarrier …))`
  TransportDec := (Values L mem).TransportDec
  OrderSelDec := (Values L mem).OrderSelDec
  SnapDec := (Values L mem).SnapDec
  BatchDec := (Values L mem).BatchDec
  OrdBatchDec := (Values L mem).OrdBatchDec
  SampleDec := (Values L mem).SampleDec
  TimerDec := (Values L mem).TimerDec
  PulseDec := (Values L mem).PulseDec
  FixDec := (Values L mem).FixDec
  map s f :=
    ⟨Vector.ofFn ((Values L mem).map s.fn f), (Values L mem).map s.den f,
     EPack.mk_agree (fun x => (Values L mem).map x f) s⟩
  filterMap s f :=
    ⟨Vector.ofFn ((Values L mem).filterMap s.fn f),
     (Values L mem).filterMap s.den f,
     EPack.mk_agree (fun x => (Values L mem).filterMap x f) s⟩
  broadcast_closed d s :=
    ⟨Vector.ofFn ((Values L mem).broadcast_closed d s.fn),
     (Values L mem).broadcast_closed d s.den,
     EPack.mk_agree_x (fun x => (Values L mem).broadcast_closed d x) s⟩
  demux d s addr :=
    ⟨Vector.ofFn ((Values L mem).demux d s.fn addr),
     (Values L mem).demux d s.den addr,
     EPack.mk_agree_x (fun x => (Values L mem).demux d x addr) s⟩
  values k :=
    ⟨Vector.ofFn ((Values L mem).values k.fn), (Values L mem).values k.den,
     EPack.mk_agree (fun x => (Values L mem).values x) k⟩
  into_keyed s :=
    ⟨Vector.ofFn ((Values L mem).into_keyed s.fn), (Values L mem).into_keyed s.den,
     EPack.mk_agree (fun x => (Values L mem).into_keyed x) s⟩
  kassume_ordering u d :=
    ⟨Vector.ofFn ((Values L mem).kassume_ordering u.fn d),
     (Values L mem).kassume_ordering u.den d,
     EPack.mk_agree (fun x => (Values L mem).kassume_ordering x d) u⟩
  kfold_early_stop ks f init :=
    ⟨Vector.ofFn ((Values L mem).kfold_early_stop ks.fn f init),
     (Values L mem).kfold_early_stop ks.den f init,
     EPack.mk_agree (fun x => (Values L mem).kfold_early_stop x f init) ks⟩
  kget_max_key {_ℓ _α _K _V _ _ _ _ ord ret} ks :=
    ⟨Vector.ofFn ((Values L mem).kget_max_key (ord := ord) (ret := ret) ks.fn),
     (Values L mem).kget_max_key (ord := ord) (ret := ret) ks.den,
     EPack.mk_agree (fun x => (Values L mem).kget_max_key (ord := ord) (ret := ret) x) ks⟩
  weaken_retries s :=
    ⟨Vector.ofFn ((Values L mem).weaken_retries s.fn),
     (Values L mem).weaken_retries s.den,
     EPack.mk_agree (fun x => (Values L mem).weaken_retries x) s⟩
  union a b :=
    ⟨Vector.ofFn ((Values L mem).union a.fn b.fn),
     (Values L mem).union a.den b.den,
     EPack.mk_agree₂ (fun x y => (Values L mem).union x y) a b⟩
  assume_ordering u d :=
    ⟨Vector.ofFn ((Values L mem).assume_ordering u.fn d),
     (Values L mem).assume_ordering u.den d,
     EPack.mk_agree (fun x => (Values L mem).assume_ordering x d) u⟩
  fold g init ok s :=
    ⟨Vector.ofFn ((Values L mem).fold g init ok s.fn),
     (Values L mem).fold g init ok s.den,
     EPack.mk_agree (fun x => (Values L mem).fold g init ok x) s⟩
  fold_monotone vo g init ok hinfl s :=
    ⟨Vector.ofFn ((Values L mem).fold_monotone vo g init ok hinfl s.fn),
     (Values L mem).fold_monotone vo g init ok hinfl s.den,
     EPack.mk_agree
       (fun x => (Values L mem).fold_monotone vo g init ok hinfl x) s⟩
  snapshot {_ℓ _α _σ _ _ord _ret b} s d :=
    match b, s with
    | .unbounded, s =>
      ⟨Vector.ofFn ((Values L mem).snapshot (ret := _ret) (b := .unbounded) s.fn d),
       (Values L mem).snapshot (ret := _ret) (b := .unbounded) s.den d,
       EPack.mk_agree
         (fun x => (Values L mem).snapshot (ret := _ret) (b := .unbounded) x d) s⟩
    | .monotonic vo, s =>
      ⟨Vector.ofFn ((Values L mem).snapshot (ret := _ret) (b := .monotonic vo) s.fn d),
       (Values L mem).snapshot (ret := _ret) (b := .monotonic vo) s.den d,
       EPack.mk_agree
         (fun x => (Values L mem).snapshot (ret := _ret) (b := .monotonic vo) x d) s⟩
  batch s d :=
    ⟨Vector.ofFn ((Values L mem).batch s.fn d),
     (Values L mem).batch s.den d,
     EPack.mk_agree (fun x => (Values L mem).batch x d) s⟩
  batch_ordered s d :=
    ⟨Vector.ofFn ((Values L mem).batch_ordered s.fn d),
     (Values L mem).batch_ordered s.den d,
     EPack.mk_agree (fun x => (Values L mem).batch_ordered x d) s⟩
  sample_every t d :=
    ⟨Vector.ofFn ((Values L mem).sample_every t.fn d),
     (Values L mem).sample_every t.den d,
     EPack.mk_agree (fun x => (Values L mem).sample_every x d) t⟩
  timeout_snapshot s d :=
    ⟨Vector.ofFn ((Values L mem).timeout_snapshot s.fn d),
     (Values L mem).timeout_snapshot s.den d,
     EPack.mk_agree (fun x => (Values L mem).timeout_snapshot x d) s⟩
  source_interval_batch d :=
    ⟨Vector.ofFn ((Values L mem).source_interval_batch d),
     (Values L mem).source_interval_batch d,
     EPack.mk_agree₀ _⟩
  mapTick s f :=
    ⟨Vector.ofFn ((Values L mem).mapTick s.fn f),
     (Values L mem).mapTick s.den f,
     EPack.mk_agree (fun x => (Values L mem).mapTick x f) s⟩
  zipTick a b :=
    ⟨Vector.ofFn ((Values L mem).zipTick a.fn b.fn),
     (Values L mem).zipTick a.den b.den,
     EPack.mk_agree₂ (fun x y => (Values L mem).zipTick x y) a b⟩
  defer_tick init t :=
    ⟨Vector.ofFn ((Values L mem).defer_tick init t.fn),
     (Values L mem).defer_tick init t.den,
     EPack.mk_agree (fun x => (Values L mem).defer_tick init x) t⟩
  allTicks bs :=
    ⟨Vector.ofFn ((Values L mem).allTicks bs.fn),
     (Values L mem).allTicks bs.den,
     EPack.mk_agree (fun x => (Values L mem).allTicks x) bs⟩
  flattenOrdered t :=
    ⟨Vector.ofFn ((Values L mem).flattenOrdered t.fn),
     (Values L mem).flattenOrdered t.den,
     EPack.mk_agree (fun x => (Values L mem).flattenOrdered x) t⟩
  flattenUnordered t :=
    ⟨Vector.ofFn ((Values L mem).flattenUnordered t.fn),
     (Values L mem).flattenUnordered t.den,
     EPack.mk_agree (fun x => (Values L mem).flattenUnordered x) t⟩
  fix_stream {_ℓ _α _ ord ret} fuel body :=
    { data := iterate (fun x => (body (EPack.ofData x)).data)
        (Vector.replicate _ (PoolBot ord ret)) fuel
      den := (Values L mem).fix_stream fuel
        (fun v => (body (EPack.ofDen v)).den)
      agree := by
        suffices h : ∀ k,
            iterate (fun x => (body (EPack.ofData x)).data)
              (Vector.replicate _ (PoolBot ord ret)) k
              = Vector.ofFn (iterate
                  (fun v => (body (EPack.ofDen v)).den)
                  (fun _i => PoolBot ord ret) k) by
          intro i
          rw [h fuel]
          exact vget_ofFn _ i
        intro k
        induction k with
        | zero =>
          refine Vector.ext (fun i hi => ?_)
          simp
        | succ k ih =>
          show (body (EPack.ofData _)).data = Vector.ofFn (body _).den
          rw [ih, EPack.ofData_eq_ofDen (vget_ofFn _)]
          refine Vector.ext (fun i hi => ?_)
          rw [Vector.getElem_ofFn]
          exact (body _).agree ⟨i, hi⟩ }
  fix_tick {_ℓ _σ} fuel body :=
    { data := iterate (fun x => (body (EPack.ofData x)).data)
        (Vector.replicate _ []) fuel
      den := (Values L mem).fix_tick fuel
        (fun v => (body (EPack.ofDen v)).den)
      agree := by
        suffices h : ∀ k,
            iterate (fun x => (body (EPack.ofData x)).data)
              (Vector.replicate _ []) k
              = Vector.ofFn (iterate
                  (fun v => (body (EPack.ofDen v)).den)
                  (fun _i => []) k) by
          intro i
          rw [h fuel]
          exact vget_ofFn _ i
        intro k
        induction k with
        | zero =>
          refine Vector.ext (fun i hi => ?_)
          simp
        | succ k ih =>
          show (body (EPack.ofData _)).data = Vector.ofFn (body _).den
          rw [ih, EPack.ofData_eq_ofDen (vget_ofFn _)]
          refine Vector.ext (fun i hi => ?_)
          rw [Vector.getElem_ofFn]
          exact (body _).agree ⟨i, hi⟩ }
  tinto_keyed x :=
    ⟨Vector.ofFn ((Values L mem).tinto_keyed x.fn),
     (Values L mem).tinto_keyed x.den,
     EPack.mk_agree (fun y => (Values L mem).tinto_keyed y) x⟩
  reduce_watermark ks wm f ok :=
    ⟨Vector.ofFn ((Values L mem).reduce_watermark ks.fn wm.fn f ok),
     (Values L mem).reduce_watermark ks.den wm.den f ok,
     EPack.mk_agree₂ (fun x y => (Values L mem).reduce_watermark x y f ok) ks wm⟩
  -- in-tick operators: the data leg runs the denotation's quotient ops
  -- (a `List` data leg is a later stage)
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
  binto_keyed b := (Values L mem).binto_keyed b
  bkfold g init ok ks := (Values L mem).bkfold g init ok ks
  bksFilter m p := (Values L mem).bksFilter m p
  bksMap m f := (Values L mem).bksMap m f
  bksKeys m := (Values L mem).bksKeys m
  bksEntries m := (Values L mem).bksEntries m
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
  -- the data leg and the denotation leg run the same fold (they are
  -- the same function: `EagerTick.fn_eq`), packed back by shape
  tick_scan sts ins outs x g init :=
    EagerTick.pack outs ((Values L mem).tick_scan sts ins outs (EagerTick.fn ins x) g init)
      ((Values L mem).tick_scan sts ins outs (EagerTick.den ins x) g init)
      (by rw [EagerTick.fn_eq])

end Hydro
