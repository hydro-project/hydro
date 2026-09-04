import HydroLean.Flo.Collection
import HydroLean.Collections.Multiset

/-!
# Gyatso-local collection types (dissertation §2.6, §3.5)

Collection languages used by the Gyatso network operators:

- `flagged`: a generic combinator building a `Coll` from any monoid-with-right-
  identity by adjoining a fixedness flag; concatenating a delta whose flag is
  set corresponds to receiving the stream terminator `⊗`. This is the common
  pattern behind the paper's ordered sequences (Fig 2.13), z-sets (Fig 2.19),
  and unordered multisets (Fig 3.10).
- `seqC`: ordered sequences with terminator (Fig 2.13). We append new data on
  the *right* (the paper writes newest-on-the-left; the two presentations are
  mirror images).
- `Mset`/`msetC`: unordered collections `[T]unord` (Fig 3.10) as the quotient of
  lists by permutation — the paper's "collapse variants that differ only in
  order" is literally a quotient type here, which is the type-theoretic heart
  of Gyatso's approach to network nondeterminism: reordering becomes
  *unrepresentable*, so nondeterministic delivery is a deterministic function
  into the quotient.
- `singC`: overwrite singletons (the `Singleton[U]` output collection of
  Fig 3.11): concatenation replaces the current value.

NOTE: namespaced `Gyatso` to avoid racing the concurrent `Collections/` work;
deduplicate in a later wave.
-/

namespace HydroLean.Gyatso

universe u

/-! ## The flagged-carrier combinator -/

/-- Build a collection language from a carrier with a concatenation and right
identity by adjoining a `Bool` fixedness flag. A value `(m, true)` is fixed
(absorbs all deltas); a delta `(δ, t)` extends the payload and, if `t = true`,
marks the result fixed (the terminator `⊗` is the delta `(id, true)`).

`fix` sets the flag, so `fix_fixed` holds; `(id, false)` is the right identity.
This is the common shape of Figs 2.13, 2.16, 2.19, 3.10. -/
def flagged (M : Type u) (op : M → M → M) (id : M) (h : ∀ m, op m id = m) :
    Coll.{u} where
  C := M × Bool
  concat c δ := match c with
    | (m, false) => (op m δ.1, δ.2)
    | (m, true) => (m, true)
  empty := (id, false)
  fix c := (c.1, true)
  concat_empty c := by cases c with
    | mk m f => cases f <;> simp [h]
  fix_fixed c δ := rfl

@[simp] theorem flagged_concat_live {M : Type u} {op : M → M → M} {id : M}
    {h : ∀ m, op m id = m} (m : M) (δ : M × Bool) :
    (flagged M op id h).concat (m, false) δ = (op m δ.1, δ.2) := rfl

@[simp] theorem flagged_concat_fixed {M : Type u} {op : M → M → M} {id : M}
    {h : ∀ m, op m id = m} (m : M) (δ : M × Bool) :
    (flagged M op id h).concat (m, true) δ = (m, true) := rfl

/-- A live (`false`-flagged) value of a `flagged` collection is fixed only if
nothing can extend it; in particular `(m, true)` is always fixed. -/
theorem flagged_fixed_of_flag {M : Type u} {op : M → M → M} {id : M}
    {h : ∀ m, op m id = m} (m : M) : (flagged M op id h).Fixed (m, true) :=
  fun _ => rfl

/-- In a `flagged` collection, fixedness is exactly the flag: the terminator
delta `(id, ⊗)` distinguishes every live value from itself-fixed. -/
theorem flagged_fixed_iff {M : Type u} {op : M → M → M} {id : M}
    {h : ∀ m, op m id = m} (c : M × Bool) :
    (flagged M op id h).Fixed c ↔ c.2 = true := by
  constructor
  · intro hfix
    cases c with
    | mk m fl =>
      cases fl with
      | true => rfl
      | false =>
        have := hfix (id, true)
        simp [flagged] at this
  · intro hfl
    cases c with
    | mk m fl => cases hfl; exact flagged_fixed_of_flag m

@[simp] theorem flagged_fix {M : Type u} {op : M → M → M} {id : M}
    {h : ∀ m, op m id = m} (c : M × Bool) :
    (flagged M op id h).fix c = (c.1, true) := rfl

/-- Fixing a flagged value is the same as concatenating the terminator delta
`(id, true)` — the algebraic fact that lets output maximality (Def 2.3.2) be
derived generically from eager execution (see `Op1.outputsMaximal_of_fixDelta`). -/
theorem flagged_fix_eq_concat_term {M : Type u} {op : M → M → M} {id : M}
    {h : ∀ m, op m id = m} (c : M × Bool) :
    (flagged M op id h).fix c = (flagged M op id h).concat c (id, true) := by
  cases c with
  | mk m fl => cases fl <;> simp [flagged, h]

/-! ## Ordered sequences with terminator (Fig 2.13) -/

/-- Ordered sequences with terminator (Fig 2.13): payload `List α`, new data
appended on the right, terminator sets the flag. -/
def seqC (α : Type u) : Coll.{u} :=
  flagged (List α) (· ++ ·) [] (by simp)

/-! ## Multisets: lists modulo permutation (Fig 3.10)

`Mset` is an alias for the shared `HydroLean.Multiset` (`Collections/Multiset.lean`,
the quotient of `List` by `List.Perm` over core Lean's `List.isSetoid`); the
operations below delegate to the shared library. Only the lemmas with no
counterpart there (`eq_nil_or_cons`, `mk_cons_ne_nil`, and the removal diamond
`cons_diamond` used for §3.5.2 confluence peaks) keep local proofs. -/

/-- Multisets as the quotient of lists by permutation: the paper's `[T]unord`
payload (Fig 3.10). Alias of the shared `HydroLean.Multiset`. -/
abbrev Mset (α : Type u) : Type u := HydroLean.Multiset α

namespace Mset

variable {α : Type u}

/-- The multiset represented by a list. Alias of `Multiset.ofList`. -/
abbrev mk (l : List α) : Mset α := Multiset.ofList l

theorem sound {l₁ l₂ : List α} (h : l₁.Perm l₂) : mk l₁ = mk l₂ :=
  Quotient.sound h

theorem exact {l₁ l₂ : List α} (h : mk l₁ = mk l₂) : l₁.Perm l₂ :=
  Quotient.exact h

/-- Every multiset is represented by some list. -/
theorem ind {motive : Mset α → Prop} (h : ∀ l, motive (mk l)) : ∀ m, motive m :=
  Quotient.ind h

/-- Multiset union (the concatenation of Fig 3.10: pointwise cardinality sum).
Alias of `Multiset.add`. -/
abbrev union (m₁ m₂ : Mset α) : Mset α := m₁ + m₂

@[simp] theorem union_mk (l₁ l₂ : List α) : union (mk l₁) (mk l₂) = mk (l₁ ++ l₂) := rfl

@[simp] theorem union_nil (m : Mset α) : union m (mk []) = m :=
  Multiset.add_nil m

theorem union_comm (m₁ m₂ : Mset α) : union m₁ m₂ = union m₂ m₁ :=
  Multiset.add_comm m₁ m₂

theorem union_assoc (m₁ m₂ m₃ : Mset α) :
    union (union m₁ m₂) m₃ = union m₁ (union m₂ m₃) :=
  Multiset.add_assoc m₁ m₂ m₃

theorem union_right_comm (m : Mset α) (x y : Mset α) :
    union (union m x) y = union (union m y) x := by
  rw [union_assoc, union_assoc, union_comm x y]

/-- Number of elements (with multiplicity). Alias of `Multiset.card`. -/
abbrev card (m : Mset α) : Nat := Multiset.card m

@[simp] theorem card_mk (l : List α) : card (mk l) = l.length := rfl

/-- Every multiset is empty or exposes some head element. -/
theorem eq_nil_or_cons (m : Mset α) :
    m = mk [] ∨ ∃ (a : α) (l : List α), m = mk (a :: l) := by
  induction m using ind with
  | h l =>
    cases l with
    | nil => exact Or.inl rfl
    | cons a t => exact Or.inr ⟨a, t, rfl⟩

theorem mk_cons_ne_nil {a : α} {l : List α} : mk (a :: l) ≠ mk [] := by
  intro h
  have := (exact h).length_eq
  simp at this

/-- **The removal diamond**: if the same multiset is exposed as `a :: l₁` and as
`b :: l₂`, then either the heads agree and the tails are equal multisets, or
each tail contains the other's head over a common remainder `l''`. This is the
peak analysis used for confluence of every operator that nondeterministically
consumes one element of an unordered input (§3.5.2). Requires decidable
equality (the Rust side requires `Eq + Hash` for the same reason). -/
theorem cons_diamond [DecidableEq α] {a b : α} {l₁ l₂ : List α}
    (h : mk (a :: l₁) = mk (b :: l₂)) :
    (a = b ∧ mk l₁ = mk l₂) ∨
    (∃ l'' : List α, mk l₁ = mk (b :: l'') ∧ mk l₂ = mk (a :: l'')) := by
  have hperm := exact h
  by_cases hab : a = b
  · subst hab
    exact Or.inl ⟨rfl, sound hperm.cons_inv⟩
  · right
    -- b ∈ l₁ since b ∈ b :: l₂ ~ a :: l₁ and b ≠ a
    have hbmem : b ∈ l₁ := by
      have : b ∈ a :: l₁ := hperm.symm.mem_iff.mp (List.mem_cons_self ..)
      cases List.mem_cons.mp this with
      | inl hba => exact absurd hba.symm hab
      | inr h => exact h
    refine ⟨l₁.erase b, sound (List.perm_cons_erase hbmem), ?_⟩
    -- erase b from both sides of `a :: l₁ ~ b :: l₂`
    have herase := hperm.erase b
    rw [List.erase_cons_head] at herase
    have hcons : (a :: l₁).erase b = a :: l₁.erase b := by
      rw [List.erase_cons_tail]
      simp only [beq_iff_eq]
      exact hab
    rw [hcons] at herase
    exact (sound herase).symm

/-- Folding a commutative function over a list is invariant under permutation.
This is the algebraic heart of `fold_commutative` (Fig 3.11): commutativity of
the closure makes the fold well-defined on the multiset quotient, so
nondeterministic arrival order cannot leak into the accumulator. Alias of
`Multiset.foldl_perm` (the hypothesis is `Multiset.AccComm f`, stated
pointfully). -/
theorem foldl_perm {β : Type v} {f : β → α → β}
    (hcomm : ∀ s a b, f (f s a) b = f (f s b) a)
    {l₁ l₂ : List α} (h : l₁.Perm l₂) : ∀ s, l₁.foldl f s = l₂.foldl f s :=
  fun s => Multiset.foldl_perm hcomm h s

end Mset

/-- The unordered collection `[T]unord` (Fig 3.10): multiset payload with
terminator flag. -/
def msetC (α : Type u) : Coll.{u} :=
  flagged (Mset α) Mset.union (Mset.mk []) Mset.union_nil

/-! ## Overwrite singletons (Fig 3.11) -/

/-- Overwrite: `s ++ x = x` when `x` is present, identity on `none`. -/
def overwrite (v w : Option α) : Option α :=
  match w with
  | none => v
  | some x => some x

@[simp] theorem overwrite_none (v : Option α) : overwrite v none = v := rfl
@[simp] theorem overwrite_some (v : Option α) (x : α) :
    overwrite v (some x) = some x := rfl

/-- The `Singleton[U]` output collection of Fig 3.11: an asynchronously updated
single value whose concatenation *replaces* the current value; only the settled
value is observable, which is why eager execution for `fold_*` operators only
constrains the final delta. -/
def singC (α : Type u) : Coll.{u} :=
  flagged (Option α) overwrite none (fun _ => rfl)

end HydroLean.Gyatso
