import HydroLean.Prelude

/-!
# `forward_ref`: the cycle combinator (any located carrier)

Rust `forward_ref` (`Location::forward_ref` for unbounded streams,
`Tick::forward_ref` for tick singletons) declares a dataflow cycle: a stream
usable before the code that completes it. Denotationally a cycle is a
**fixpoint**: the body is a function of the cycle stream that returns what
it feeds back into the cycle (plus its other outputs), and the semantics
Kleene-iterates it from the empty history.

This is well-defined *without domain theory* because Hydro bodies are
prefix-monotone with decisions fixed (`TStream.lean`): re-running the whole
body on a grown cycle history only appends — realized output is final —
so the iterates form a chain and every within-tick knot is exact at every
unfolding depth. The combinator is **polymorphic in the cycle carrier**:
unbounded streams (`List α`), tick singletons (`TSing β`), member-indexed
families (`Fin n → …`), and products of these (a body with several
`forward_ref`s uses one combined cycle state, or nests combinators).

`fuel` is the unfolding depth — an adversarial decision like any other
(safety statements quantify over it; more fuel only extends realized ticks).

Proof principles:
- `iterate_ind` — fixpoint induction over reachable iterates: the ONE
  generic induction of the methodology. Cross-tick feedback ("unbounded
  loop") is absorbed here ("bounded reasoning"): per-program proofs supply
  invariant clauses; no event/schedule induction exists anywhere.
- `iterate_chain` — iterates chain under any transitive relation the body
  preserves (instantiated at pointwise stream-prefix).
-/

namespace HydroLean.Hydro

universe u v

/-- Iterate a function (the Kleene chain of a `forward_ref` body). -/
def iterate {τ : Type u} (F : τ → τ) (x : τ) : Nat → τ
  | 0 => x
  | k + 1 => F (iterate F x k)

@[simp] theorem iterate_zero {τ : Type u} (F : τ → τ) (x : τ) :
    iterate F x 0 = x := rfl

@[simp] theorem iterate_succ {τ : Type u} (F : τ → τ) (x : τ) (k : Nat) :
    iterate F x (k + 1) = F (iterate F x k) := rfl

/-- **Fixpoint induction over reachable iterates**: prove an invariant at
the bottom and through one body application; it holds at every unfolding
depth. -/
theorem iterate_ind {τ : Type u} {F : τ → τ} {x : τ} (P : τ → Prop)
    (h0 : P x) (hF : ∀ y, P y → P (F y)) : ∀ k, P (iterate F x k)
  | 0 => h0
  | k + 1 => hF _ (iterate_ind P h0 hF k)

/-- Iterates form a chain under any transitive relation the body preserves
(instantiated with pointwise stream-prefix: realized ticks are final across
unfolding depths — the within-tick-knot exactness lemma). -/
theorem iterate_chain {τ : Type u} {F : τ → τ} {x : τ} {R : τ → τ → Prop}
    (htrans : ∀ {a b c}, R a b → R b c → R a c)
    (hrefl : ∀ a, R a a)
    (hmono : ∀ a b, R a b → R (F a) (F b)) (h0 : R x (F x)) :
    ∀ {k k'}, k ≤ k' → R (iterate F x k) (iterate F x k') := by
  have hstep : ∀ k, R (iterate F x k) (iterate F x (k + 1)) := by
    intro k
    induction k with
    | zero => exact h0
    | succ k ih => exact hmono _ _ ih
  intro k k' hle
  induction hle with
  | refl => exact hrefl _
  | step _ ih => exact htrans ih (hstep _)

/-- Relate the iterates of two bodies pointwise (used to transport growth of
a `forward_ref`'s *inputs* through its unfolding: if one body dominates the
other step-wise, the whole chains relate). -/
theorem iterate_rel {τ : Type u} {F G : τ → τ} {x y : τ} {R : τ → τ → Prop}
    (hFG : ∀ a b, R a b → R (F a) (G b)) (h0 : R x y) :
    ∀ k, R (iterate F x k) (iterate G y k)
  | 0 => h0
  | k + 1 => hFG _ _ (iterate_rel hFG h0 k)

/-- **Rust `forward_ref` as a higher-order function**: the body receives the
cycle value and returns (what it feeds back into the cycle, its other
outputs). The combinator Kleene-iterates the feedback `fuel` times from
`init` (the empty history) and runs the body once more on the result. Users
never write recursion. -/
def forward_ref {τ : Type u} {Out : Type v} (fuel : Nat) (init : τ)
    (body : τ → τ × Out) : Out :=
  (body (iterate (fun h => (body h).1) init fuel)).2

/-- The cycle history a `forward_ref` closes over (for proofs). -/
def forward_ref_hist {τ : Type u} {Out : Type v} (fuel : Nat) (init : τ)
    (body : τ → τ × Out) : τ :=
  iterate (fun h => (body h).1) init fuel

/-- The unfolding face of `forward_ref` (kept as the definitional interface
even where proofs use `rfl` directly). -/
theorem forward_ref_eq {τ : Type u} {Out : Type v} (fuel : Nat) (init : τ)
    (body : τ → τ × Out) :
    forward_ref fuel init body = (body (forward_ref_hist fuel init body)).2 :=
  rfl

/-- The memo table backing `memoF`. The extra `table` field is a deliberate
second reference: without it the compiler float-sinks the table into the
returned closure, silently destroying the sharing — every read of a cycle
value then re-runs every previous unfolding, making nested `forward_ref`s
exponentially slow (FINDINGS D14). -/
structure MemoTable (n : Nat) (α : Type u) : Type u where
  fn : Fin n → α
  table : List α

/-- Materialize a family (`@[noinline]` so the table is built exactly once,
when the value is created). -/
@[noinline] def MemoTable.of {n : Nat} {α : Type u} (f : Fin n → α) :
    MemoTable n α :=
  let l := (List.finRange n).map f
  ⟨fun i => l[i.val]'(by
    rw [List.length_map, List.length_finRange]
    exact i.isLt), l⟩

/-- Memoize a member-indexed family (eager: forces every member when the
value is created). Semantically the identity (`memoF_eq`); operationally it
keeps `forward_ref` iteration linear — without it, cycle values are closure
chains re-running every previous unfolding on each read. `@[inline]` so the
call site (inside a pair-returning body, safe from eta-expansion) evaluates
the `MemoTable` once. -/
@[inline] def memoF {n : Nat} {α : Type u} (f : Fin n → α) : Fin n → α :=
  (MemoTable.of f).fn

@[simp] theorem memoF_eq {n : Nat} {α : Type u} (f : Fin n → α) (i : Fin n) :
    memoF f i = f i := by
  show ((List.finRange n).map f)[i.val]'_ = f i
  rw [List.getElem_map]
  congr 1
  exact List.getElem_finRange _

@[simp] theorem memoF_def {n : Nat} {α : Type u} (f : Fin n → α) :
    memoF f = f := funext (memoF_eq f)

/-! ## Gas-less `forward_ref`: termination from the decision budget

With a monotone body, every realized tick of the cycle is paid for by a
finite decision list, so the Kleene chain stabilizes: supply a measure `μ`
(e.g. total cycle-stream length) that (a) strictly grows on every non-fixed
body application and (b) is bounded by the decision budget `B`. Then `B`
unfoldings reach a **genuine fixpoint** (`iterate_fixed_of_bounded`), the
fuel parameter disappears from statements (`forward_ref_of_bounded` +
`forward_ref_of_bounded_spec`), and `∀ fuel` safety collapses to facts about
*the* run. Unbounded retries collapse too: `AtLeastOnce` re-delivery
legality spends the same finite budget — the quotiented representation
bounds the measure. -/

/-- A fixed point stays fixed. -/
theorem iterate_eq_of_fixed {τ : Type u} {F : τ → τ} {x : τ} (h : F x = x) :
    ∀ k, iterate F x k = x
  | 0 => rfl
  | k + 1 => by rw [iterate_succ, iterate_eq_of_fixed h k, h]

/-- **Termination from a bounded, strictly-growing measure**: `B` iterations
reach a fixed point of the body. -/
theorem iterate_fixed_of_bounded {τ : Type u} {F : τ → τ} {x : τ}
    {μ : τ → Nat} {B : Nat}
    (hgrow : ∀ h, F h = h ∨ μ h < μ (F h))
    (hbound : ∀ h, μ h ≤ B) :
    F (iterate F x B) = iterate F x B := by
  -- either some iterate ≤ B is fixed (then all later ones equal it), or μ
  -- strictly increases along the chain and exceeds the bound
  by_cases hfix : ∃ k, k ≤ B ∧ F (iterate F x k) = iterate F x k
  · obtain ⟨k, hk, hfk⟩ := hfix
    have hstay : iterate F x B = iterate F x k := by
      have : ∀ d, iterate F x (k + d) = iterate F x k := by
        intro d
        induction d with
        | zero => rfl
        | succ d ih =>
          rw [← Nat.add_assoc, iterate_succ, ih, hfk]
      have := this (B - k)
      rwa [Nat.add_sub_cancel' hk] at this
    rw [hstay, hfk]
  · exfalso
    have hfix' : ∀ k, k ≤ B → F (iterate F x k) ≠ iterate F x k :=
      fun k hk hfx => hfix ⟨k, hk, hfx⟩
    -- μ grows by ≥ 1 per step up to B, so μ (iterate B) ≥ B, and one more
    -- strict step exceeds the bound
    have hstep : ∀ k, k ≤ B → k ≤ μ (iterate F x k) := by
      intro k
      induction k with
      | zero => intro _; exact Nat.zero_le _
      | succ k ih =>
        intro hk
        have hk' : k ≤ B := Nat.le_of_succ_le hk
        rcases hgrow (iterate F x k) with hfx | hlt
        · exact absurd hfx (hfix' k hk')
        · have := ih hk'
          rw [iterate_succ]
          omega
    have := hstep B (Nat.le_refl B)
    rcases hgrow (iterate F x B) with hfx | hlt
    · exact hfix' B (Nat.le_refl B) hfx
    · have hb' := hbound (F (iterate F x B))
      omega

/-- Gas-less `forward_ref`: the unfolding depth is the decision budget `B`;
by `iterate_fixed_of_bounded` the reached history is a true fixpoint, so
the result does not depend on any arbitrary depth. -/
def forward_ref_of_bounded {τ : Type u} {Out : Type v} (B : Nat) (init : τ)
    (body : τ → τ × Out) : Out :=
  forward_ref B init body

/-- The gas-less form runs the body on a **fixed** cycle history. -/
theorem forward_ref_of_bounded_spec {τ : Type u} {Out : Type v} {B : Nat}
    {init : τ} {body : τ → τ × Out} {μ : τ → Nat}
    (hgrow : ∀ h, (body h).1 = h ∨ μ h < μ ((body h).1))
    (hbound : ∀ h, μ h ≤ B) :
    (body (forward_ref_hist B init body)).1 = forward_ref_hist B init body :=
  iterate_fixed_of_bounded (F := fun h => (body h).1) hgrow hbound

/-! ### Bridging gas and gas-less

Safety proven once at the fixpoint holds for **every** fuel: at or beyond
the budget the run *is* the fixpoint run (`forward_ref_ge_budget`); below it
the cycle history is a ⊑-prefix of the fixpoint's (`forward_ref_le_budget`),
so any prefix-closed property transfers. -/

/-- At or beyond the budget, every fuel gives the fixpoint run. -/
theorem forward_ref_ge_budget {τ : Type u} {Out : Type v} {B : Nat}
    {init : τ} {body : τ → τ × Out} {μ : τ → Nat}
    (hgrow : ∀ h, (body h).1 = h ∨ μ h < μ ((body h).1))
    (hbound : ∀ h, μ h ≤ B)
    {fuel : Nat} (hf : B ≤ fuel) :
    forward_ref fuel init body = forward_ref_of_bounded B init body := by
  have hfix := iterate_fixed_of_bounded (F := fun h => (body h).1)
    (x := init) hgrow hbound
  have hall : ∀ d, iterate (fun h => (body h).1) init (B + d)
      = iterate (fun h => (body h).1) init B := by
    intro d
    induction d with
    | zero => rfl
    | succ d ih => rw [← Nat.add_assoc, iterate_succ, ih, hfix]
  have h := hall (fuel - B)
  rw [Nat.add_sub_cancel' hf] at h
  show (body (iterate (fun h => (body h).1) init fuel)).2
    = (body (iterate (fun h => (body h).1) init B)).2
  rw [h]

/-- Below the budget, the cycle history is a prefix of the fixpoint's
(under any transitive-reflexive relation the body preserves). -/
theorem forward_ref_le_budget {τ : Type u} {Out : Type v} {B : Nat}
    {init : τ} {body : τ → τ × Out} {R : τ → τ → Prop}
    (htrans : ∀ {a b c}, R a b → R b c → R a c)
    (hrefl : ∀ a, R a a)
    (hmono : ∀ a b, R a b → R ((body a).1) ((body b).1))
    (h0 : R init ((body init).1))
    {fuel : Nat} (hf : fuel ≤ B) :
    R (forward_ref_hist fuel init body) (forward_ref_hist B init body) := by
  show R (iterate (fun h => (body h).1) init fuel)
    (iterate (fun h => (body h).1) init B)
  exact iterate_chain (F := fun h => (body h).1) (x := init) (R := R)
    htrans hrefl hmono h0 hf

end HydroLean.Hydro
