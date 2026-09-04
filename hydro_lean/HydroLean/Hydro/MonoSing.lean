import HydroLean.Hydro.Growth

/-!
# Monotonic singletons: Rust's `Monotonic` bound, mirrored

(The `_vals`/`_f` equations and the growth/order faces in this module are
the type's documented interface — a face set, kept intentionally whether or
not each is currently consumed by a program.)

hydro_lang's singleton bound hierarchy (singleton.rs:39–99) has, between
`Bounded` (immutable) and `Unbounded` (arbitrarily mutable), the marker
**`Monotonic`** — *"the value will only grow"*. It is produced by
`Stream::fold` when the closure carries a `monotone = <proof>` property
(`AggFuncAlgebra<…, Monotone = Proved>`, properties/mod.rs:194–229): the
output bound is then `B::StreamToMonotone` (`ApplyMonotoneStream`,
properties/mod.rs:532–541), i.e. `Unbounded → Monotonic`. `Monotonic`
singletons unlock `IsMonotonic`-gated APIs (deterministic threshold reads,
snapshot-safe folds) with no `nondet!`.

**Singleton honesty (user ruling)**: an arbitrary singleton over time is NOT
value-monotone — only trajectory-forward. Value-monotonicity exists exactly
when the producing fold's closure is inflationary, and then it is carried
**by the output type**, never re-proven per consumer. This module is that
type:

- `ValueOrder σ`: the growth preorder of a singleton's value (made explicit,
  since Lean types don't fix a canonical lattice per `σ`; in Rust it is the
  order the `monotone` proof is about).
- `MonoSing vo`: a tick singleton whose realized trajectory ascends in `vo`
  — the Lean form of `Singleton<σ, Tick<L>, Monotonic>`. It is a located
  carrier in its own right (`Growth` = trajectory prefix), so **module
  functions return `MonoSing`-typed wires directly** — the guarantee rides
  the signature; consumers project it (`.ascending`, `.tick_lt_of_not_le`)
  instead of rederiving it from the fold.
- `fold_monotonic` / `TickLoop.foldMonotonic`: the `fold` + `monotone =`
  combinators — the closure obligation (`∀ s x, vo.le s (g s x)`,
  inflationary) is discharged **at the definition site**, in exchange for
  the `MonoSing` output type; downstream monotonicity facts are field
  projections, composing through `MonoSing.map` (order-preserving maps,
  mirroring `SingletonMapFuncAlgebra::order_preserving`,
  properties/mod.rs:263).

**Lean-side strengthening (user directive)**: unlike Rust's `Monotonic`
marker, the Lean type can also export *consumer* faces that invert the
guarantee — `tick_lt_of_not_le` reads **tick order off a value gap**
(contrapositive of `ascending`), which is what ballot-ordering arguments
(K1 `promise_order`) actually consume. Stronger value orders (total orders,
strict ascent) slot in as richer `ValueOrder`s without new plumbing.
-/

namespace HydroLean.Hydro

universe u v w

/-- The growth preorder of a monotonic singleton's value. -/
structure ValueOrder (σ : Type v) where
  le : σ → σ → Prop
  le_refl : ∀ a, le a a
  le_trans : ∀ {a b c}, le a b → le b c → le a c

/-- `Nat` under `≤` (ballot numbers, slot counters). -/
def ValueOrder.nat : ValueOrder Nat :=
  ⟨(· ≤ ·), fun _ => Nat.le_refl _, Nat.le_trans⟩

/-! ## Inflationary folds (the run-level faces) -/

section Fold

variable {ι : Type u} {σ : Type v} (vo : ValueOrder σ) {g : σ → ι → σ}

/-- An inflationary step never descends along a batch. -/
theorem ValueOrder.foldl_le (hinfl : ∀ s x, vo.le s (g s x)) :
    ∀ (l : List ι) (s : σ), vo.le s (l.foldl g s)
  | [], s => vo.le_refl s
  | x :: xs, s =>
    vo.le_trans (hinfl s x) (ValueOrder.foldl_le hinfl xs (g s x))

/-- Fold states ascend along stream extension. -/
theorem ValueOrder.foldl_append_le (hinfl : ∀ s x, vo.le s (g s x))
    (l ext : List ι) (s : σ) :
    vo.le (l.foldl g s) ((l ++ ext).foldl g s) := by
  rw [List.foldl_append]
  exact vo.foldl_le hinfl ext _

/-- Fold states ascend along prefix cuts. -/
theorem ValueOrder.foldl_take_le (hinfl : ∀ s x, vo.le s (g s x))
    {l : List ι} {t t' : Nat} (h : t ≤ t') (s : σ) :
    vo.le ((l.take t).foldl g s) ((l.take t').foldl g s) := by
  have hsplit : l.take t ++ (l.take t').drop t = l.take t' := by
    have h1 : (l.take t').take t = l.take t := by
      rw [List.take_take, Nat.min_eq_left h]
    rw [← h1, List.take_append_drop]
  rw [← hsplit]
  exact vo.foldl_append_le hinfl _ _ s

/-- **Cut order from a value gap** (contrapositive of `foldl_take_le`,
total — `take` clamps, so no realization bounds): if the cut value at `t'`
is *not* above the cut value at `t`, the `t'`-cut is strictly shorter. The
cut-form sibling of `MonoSing.tick_lt_of_not_le`, for consumers whose
ambient facts are take-folds (loop eliminators). -/
theorem ValueOrder.cut_lt_of_not_le (hinfl : ∀ s x, vo.le s (g s x))
    {l : List ι} {t t' : Nat} (s : σ)
    (hgap : ¬ vo.le ((l.take t).foldl g s) ((l.take t').foldl g s)) :
    t' < t := by
  rcases Nat.lt_or_ge t' t with h | h
  · exact h
  · exact absurd (vo.foldl_take_le hinfl h s) hgap

end Fold

/-! ## The monotonic singleton type -/

/-- A tick singleton whose realized trajectory ascends in the value order —
`Singleton<σ, Tick<L>, Monotonic>` (singleton.rs:72). -/
structure MonoSing {σ : Type v} (vo : ValueOrder σ) : Type v where
  vals : TSing σ
  ascending : ∀ {t t' : Nat} (h : t ≤ t') (ht' : t' < vals.length),
    vo.le (vals[t]'(Nat.lt_of_le_of_lt h ht')) (vals[t']'ht')

namespace MonoSing

variable {σ : Type v} {σ' : Type w} {vo : ValueOrder σ} {vo' : ValueOrder σ'}

/-- The empty wire (no realized ticks — cycle bottoms). -/
def empty : MonoSing vo where
  vals := []
  ascending := by intro _ _ _ ht'; cases ht'

instance : Inhabited (MonoSing vo) := ⟨empty⟩

/-- Order-preserving maps compose type-derived monotonicity downstream
(`SingletonMapFuncAlgebra::order_preserving`). -/
def map (m : MonoSing vo) (f : σ → σ')
    (hf : ∀ {a b}, vo.le a b → vo'.le (f a) (f b)) : MonoSing vo' where
  vals := m.vals.map f
  ascending := by
    intro t t' h ht'
    have hlen : t' < m.vals.length := by
      simpa using ht'
    have ht : t < m.vals.length := Nat.lt_of_le_of_lt h hlen
    simpa [List.getElem_map] using hf (m.ascending h hlen)

@[simp] theorem map_vals (m : MonoSing vo) (f : σ → σ')
    (hf : ∀ {a b}, vo.le a b → vo'.le (f a) (f b)) :
    (m.map f hf).vals = m.vals.map f := rfl

/-- **Tick order from a value gap** (contrapositive of `ascending`): if the
wire's value at `t'` is *not* above its value at `t`, then `t'` was realized
strictly before `t`. This is the consumer face ballot-ordering arguments
(K1 `promise_order`) project from the type — a Lean-side strengthening with
no Rust `Monotonic` analogue. -/
theorem tick_lt_of_not_le (m : MonoSing vo) {t t' : Nat}
    (ht : t < m.vals.length) (ht' : t' < m.vals.length)
    (h : ¬ vo.le (m.vals[t]'ht) (m.vals[t']'ht')) : t' < t := by
  rcases Nat.lt_or_ge t' t with h' | h'
  · exact h'
  · exact absurd (m.ascending h' ht') h

end MonoSing

/-! ## `MonoSing` as a located carrier -/

/-- A `MonoSing` wire grows by trajectory prefix — realized ticks are final;
the value order is already carried by the type. This makes `MonoSing` a
first-class output carrier of `→ₘ` stages. -/
instance instGrowthMonoSing {σ : Type v} {vo : ValueOrder σ} :
    Growth (MonoSing vo) where
  le a b := a.vals <+: b.vals
  le_refl _ := List.prefix_refl _
  le_trans h₁ h₂ := h₁.trans h₂

/-- The value view of a wire, as a typed stage (forgetting the value order,
keeping trajectory growth). -/
def MonoSing.valsM {σ : Type v} {vo : ValueOrder σ} :
    MonoSing vo →ₘ TSing σ :=
  ⟨MonoSing.vals, fun h => h⟩

/-- Order-preserving wire map, as a typed stage. -/
def MonoSing.mapM {σ : Type v} {σ' : Type w} {vo : ValueOrder σ}
    {vo' : ValueOrder σ'} (f : σ → σ')
    (hf : ∀ {a b}, vo.le a b → vo'.le (f a) (f b)) :
    MonoSing vo →ₘ MonoSing vo' :=
  ⟨fun m => m.map f hf, fun h => h.map f⟩

@[simp] theorem MonoSing.valsM_f {σ : Type v} {vo : ValueOrder σ}
    (m : MonoSing vo) : (MonoSing.valsM (vo := vo)).f m = m.vals := rfl

@[simp] theorem MonoSing.mapM_f {σ : Type v} {σ' : Type w}
    {vo : ValueOrder σ} {vo' : ValueOrder σ'} (f : σ → σ')
    (hf : ∀ {a b}, vo.le a b → vo'.le (f a) (f b)) (m : MonoSing vo) :
    (MonoSing.mapM f hf).f m = m.map f hf := rfl

/-! ### Wire lifts (reader-style; see `Growth.lean` Wire combinators) -/

section Wires

variable {Γ : Type u} [Growth Γ]

/-- The value view of a `Monotonic` wire. -/
def MonoMap.vals {σ : Type v} {vo : ValueOrder σ} (w : Γ →ₘ MonoSing vo) :
    Γ →ₘ TSing σ := MonoSing.valsM ∘ₘ w

/-- Order-preserving map on a `Monotonic` wire. -/
def MonoMap.mapWire {σ : Type v} {σ' : Type w} {vo : ValueOrder σ}
    {vo' : ValueOrder σ'} (w : Γ →ₘ MonoSing vo) (f : σ → σ')
    (hf : ∀ {a b}, vo.le a b → vo'.le (f a) (f b)) : Γ →ₘ MonoSing vo' :=
  MonoSing.mapM f hf ∘ₘ w

end Wires

/-! ## `fold_monotonic`: the `fold` + `monotone =` combinator -/

/-- Rust `stream.fold(init, comb)` with `monotone = <proof>` on the closure
(`AggFuncAlgebra::monotone` ⇒ output bound `Monotonic` via
`ApplyMonotoneStream`): the inflationary obligation is paid here, once, and
the output **type** carries the trajectory guarantee. -/
def fold_monotonic {ι : Type u} {σ : Type v} (vo : ValueOrder σ)
    (g : σ → ι → σ) (init : σ) (hinfl : ∀ s x, vo.le s (g s x))
    (ins : List ι) : MonoSing vo where
  vals := scanSt g init ins
  ascending := by
    intro t t' h ht'
    have ht : t < (scanSt g init ins).length := Nat.lt_of_le_of_lt h ht'
    rw [scanSt_getElem g init ins t ht, scanSt_getElem g init ins t' ht']
    exact vo.foldl_take_le hinfl (Nat.succ_le_succ h) init

@[simp] theorem fold_monotonic_vals {ι : Type u} {σ : Type v}
    (vo : ValueOrder σ) (g : σ → ι → σ) (init : σ)
    (hinfl : ∀ s x, vo.le s (g s x)) (ins : List ι) :
    (fold_monotonic vo g init hinfl ins).vals = scanSt g init ins := rfl

/-- `fold_monotonic` as a typed stage: the wire grows (by trajectory prefix)
with the consumed stream. -/
def fold_monotonicM {ι : Type u} {σ : Type v} (vo : ValueOrder σ)
    (g : σ → ι → σ) (init : σ) (hinfl : ∀ s x, vo.le s (g s x)) :
    List ι →ₘ MonoSing vo :=
  ⟨fold_monotonic vo g init hinfl, fun h => scanSt_prefix g init h⟩

@[simp] theorem fold_monotonicM_f {ι : Type u} {σ : Type v}
    (vo : ValueOrder σ) (g : σ → ι → σ) (init : σ)
    (hinfl : ∀ s x, vo.le s (g s x)) (ins : List ι) :
    (fold_monotonicM vo g init hinfl).f ins
      = fold_monotonic vo g init hinfl ins := rfl

/-- A `TickLoop` whose state update is inflationary publishes its state wire
as a monotonic singleton (`across_ticks` folds; the `use::state` face). -/
def TickLoop.foldMonotonic {In : Type u} {St : Type v} {Out : Type w}
    (t : TickLoop In St Out) (vo : ValueOrder St)
    (hinfl : ∀ s b, vo.le s (t.step s b).1) (ins : List In) : MonoSing vo :=
  fold_monotonic vo (fun s b => (t.step s b).1) t.init hinfl ins

@[simp] theorem TickLoop.foldMonotonic_vals {In : Type u} {St : Type v}
    {Out : Type w} (t : TickLoop In St Out) (vo : ValueOrder St)
    (hinfl : ∀ s b, vo.le s (t.step s b).1) (ins : List In) :
    (t.foldMonotonic vo hinfl ins).vals = t.states ins := rfl

/-- `TickLoop.foldMonotonic` as a typed stage. -/
def TickLoop.foldMonotonicM {In : Type u} {St : Type v} {Out : Type w}
    (t : TickLoop In St Out) (vo : ValueOrder St)
    (hinfl : ∀ s b, vo.le s (t.step s b).1) :
    List In →ₘ MonoSing vo :=
  ⟨t.foldMonotonic vo hinfl, fun h => scanSt_prefix _ _ h⟩

@[simp] theorem TickLoop.foldMonotonicM_f {In : Type u} {St : Type v}
    {Out : Type w} (t : TickLoop In St Out) (vo : ValueOrder St)
    (hinfl : ∀ s b, vo.le s (t.step s b).1) (ins : List In) :
    (t.foldMonotonicM vo hinfl).f ins = t.foldMonotonic vo hinfl ins := rfl

/-- `fold` + `monotone =` on a wire: the closure obligation is paid here,
the output wire carries the `Monotonic` type. -/
def MonoMap.foldMonotonic {Γ : Type u} [Growth Γ] {ι : Type u'} {σ : Type v}
    (w : Γ →ₘ List ι) (vo : ValueOrder σ) (g : σ → ι → σ) (init : σ)
    (hinfl : ∀ s x, vo.le s (g s x)) : Γ →ₘ MonoSing vo :=
  fold_monotonicM vo g init hinfl ∘ₘ w

/-- A `TickLoop` with inflationary state update, consuming a wire: the
state wire at its `Monotonic` type. -/
def MonoMap.loopFoldMonotonic {Γ : Type u} [Growth Γ] {In : Type u'}
    {St : Type v} {Out : Type w} (w : Γ →ₘ List In) (t : TickLoop In St Out)
    (vo : ValueOrder St) (hinfl : ∀ s b, vo.le s (t.step s b).1) :
    Γ →ₘ MonoSing vo :=
  t.foldMonotonicM vo hinfl ∘ₘ w

/-- The final-state form: `finalState` ascends along stream extension —
the run-level reading of the `Monotonic` bound. -/
theorem TickLoop.finalState_le {In : Type u} {St : Type v} {Out : Type w}
    (t : TickLoop In St Out) (vo : ValueOrder St)
    (hinfl : ∀ s b, vo.le s (t.step s b).1) (ins ext : List In) :
    vo.le (t.finalState ins) (t.finalState (ins ++ ext)) := by
  rw [← foldl_eq_finalState, ← foldl_eq_finalState]
  exact vo.foldl_append_le hinfl ins ext t.init

/-- The prefix-cut form: `finalState` over growing takes ascends. -/
theorem TickLoop.finalState_take_le {In : Type u} {St : Type v} {Out : Type w}
    (t : TickLoop In St Out) (vo : ValueOrder St)
    (hinfl : ∀ s b, vo.le s (t.step s b).1) {ins : List In} {n n' : Nat}
    (h : n ≤ n') :
    vo.le (t.finalState (ins.take n)) (t.finalState (ins.take n')) := by
  rw [← foldl_eq_finalState, ← foldl_eq_finalState]
  exact vo.foldl_take_le hinfl h t.init

end HydroLean.Hydro
