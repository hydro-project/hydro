/-!
# Hydro · value orders

The partial order a `Monotonic` bound is about: in Rust, `Singleton<_, _, Monotonic>` is
produced by a fold whose `monotone = <proof>` obligation specifies an order on the
accumulator and promises inflation in it. `ValueOrder` is that order, with the two
instances the programs use and the fold lemmas a monotonicity proof needs.
-/

namespace Hydro

/-- Decidable equality on pairs, core's `instDecidableEqProd` term for term, marked for
inlining. Core's instance is a closed function that takes the components' instances as
dictionaries, so at a hash map keyed by a pair every bucket probe is a `lean_apply` of each
component's instance with boxed arguments; inlined into the specialized hash-map code the
component instances are known and the comparison is a direct call. The `match` is core's, so
the two instances are definitionally equal: the elaborated graphs' `Kind`s contain whichever
instance synthesis chose, proofs are insensitive because `DecidableEq α` is a subsingleton,
and the explicit `instDecidableEqProd` annotations in `Std/Quorum.lean` typecheck by
unfolding. -/
@[inline] instance (priority := high) instDecidableEqProdInline {α : Type u} {β : Type v}
    [DecidableEq α] [DecidableEq β] : DecidableEq (α × β) := fun (a, b) (a', b') =>
  match decEq a a' with
  | isTrue e₁ =>
    match decEq b b' with
    | isTrue e₂ => isTrue (e₁ ▸ e₂ ▸ rfl)
    | isFalse n₂ => isFalse fun h => n₂ (congrArg Prod.snd h)
  | isFalse n₁ => isFalse fun h => n₁ (congrArg Prod.fst h)

/-- A preorder on values: the vocabulary of the `Monotonic` bound. -/
structure ValueOrder (σ : Type _) where
  le : σ → σ → Prop
  le_refl : ∀ a, le a a
  le_trans : ∀ {a b c}, le a b → le b c → le a c

/-- `Nat` under `≤` (ballot numbers, slot counters). -/
@[reducible] def ValueOrder.nat : ValueOrder Nat :=
  ⟨(· ≤ ·), fun _ => Nat.le_refl _, Nat.le_trans⟩

/-- `Option Nat` with `none` as bottom: `Option<usize>` registers that only grow, such as a
checkpoint sequence number. -/
def ValueOrder.optNatLE : Option Nat → Option Nat → Prop
  | none, _ => True
  | some _, none => False
  | some a, some b => a ≤ b

instance : DecidableRel ValueOrder.optNatLE := fun a b => by
  cases a <;> cases b <;> simp only [ValueOrder.optNatLE] <;> infer_instance

@[reducible] def ValueOrder.optNat : ValueOrder (Option Nat) where
  le := ValueOrder.optNatLE
  le_refl a := by cases a <;> simp [ValueOrder.optNatLE]
  le_trans {a b c} h₁ h₂ := by
    cases a <;> cases b <;> cases c <;> simp_all [ValueOrder.optNatLE] <;> omega

/-- An inflationary step (`hinfl`) folds upward: the seed is below any fold from it. -/
theorem ValueOrder.le_foldl {σ ι : Type _} (vo : ValueOrder σ)
    {g : σ → ι → σ} (hinfl : ∀ s x, vo.le s (g s x)) :
    ∀ (l : List ι) (s : σ), vo.le s (l.foldl g s)
  | [], s => vo.le_refl s
  | x :: xs, s =>
    vo.le_trans (hinfl s x) (ValueOrder.le_foldl vo hinfl xs (g s x))

/-- Appending input only moves an inflationary fold up. -/
theorem ValueOrder.foldl_append_le {σ ι : Type _} (vo : ValueOrder σ)
    {g : σ → ι → σ} (hinfl : ∀ s x, vo.le s (g s x))
    (l e : List ι) (s : σ) :
    vo.le (l.foldl g s) ((l ++ e).foldl g s) := by
  rw [List.foldl_append]
  exact vo.le_foldl hinfl e _

/-- A longer prefix of the input (`h : t ≤ t'`) gives a larger inflationary fold. -/
theorem ValueOrder.foldl_take_le {σ ι : Type _} (vo : ValueOrder σ)
    {g : σ → ι → σ} (hinfl : ∀ s x, vo.le s (g s x))
    {l : List ι} {t t' : Nat} (h : t ≤ t') (s : σ) :
    vo.le ((l.take t).foldl g s) ((l.take t').foldl g s) := by
  have hsplit : l.take t ++ (l.take t').drop t = l.take t' := by
    have h1 : (l.take t').take t = l.take t := by
      rw [List.take_take, Nat.min_eq_left h]
    rw [← h1, List.take_append_drop]
  rw [← hsplit]
  exact vo.foldl_append_le hinfl _ _ s

end Hydro
