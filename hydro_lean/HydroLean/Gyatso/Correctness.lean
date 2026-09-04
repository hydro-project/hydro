import HydroLean.Flo.Theorems

/-!
# Gyatso distributed correctness (dissertation §3.4)

Gyatso's two distributed guarantees:

- **Eventual determinism** (Thm 3.4.1): when all machines are live and all
  messages are delivered, the outputs settle to a unique value. In our
  semantics, machine liveness and message delivery are *fairness of the
  scheduler*: a live execution is one that keeps taking available steps, so the
  theorem is exactly Flo's unique-stuck-state property (`Graph.unique_stuck`),
  inherited because Gyatso reuses Flo's small-step semantics unchanged
  (§3.4.1, §3.4.3).

- **Monotone outputs** (Thm 3.4.2): even under crash-stop failures, already
  emitted outputs are always a *prefix/subset* (in the collection's natural
  order, Def 3.4.1) of the intended settled output. Crash-stop failures
  (§3.4.2) are modeled as freezing part of the schedule — a failed machine's
  operators simply stop being scheduled — so every failed execution is just
  some finite trace of the ordinary step relation, and the theorem is a trace
  induction: steps only ever *concatenate* deltas onto output buffers, and
  natural orders are aligned with concatenation.
-/

namespace HydroLean.Gyatso

universe u

/-- **Def 3.4.1 (Collection Natural Order)**: an optional partial order on a
collection aligned with concatenation: growing a value by any delta moves it
up. Examples: subset order for sets, prefix order for sequences, lattice order
for LVars. -/
structure NaturalOrder (L : Coll.{u}) : Type u where
  /-- The order relation. -/
  le : L.C → L.C → Prop
  /-- Reflexivity. -/
  le_refl : ∀ c, le c c
  /-- Transitivity. -/
  le_trans : ∀ {a b c}, le a b → le b c → le a c
  /-- Alignment with concatenation: `c ≤ c ++ δ` for every delta. -/
  concat_le : ∀ c δ, le c (L.concat c δ)

/-- A tuple of natural orders, one per output port. -/
inductive NatOrders : List Coll.{u} → Type (u + 1) where
  | nil : NatOrders []
  | cons {L : Coll.{u}} {ls : List Coll.{u}} :
      NaturalOrder L → NatOrders ls → NatOrders (L :: ls)

namespace NatOrders

/-- Pointwise order on value tuples. -/
inductive le : {ls : List Coll.{u}} → NatOrders ls → Vals ls → Vals ls → Prop where
  | nil : le .nil .nil .nil
  | cons {L : Coll.{u}} {ls : List Coll.{u}} {no : NaturalOrder L}
      {nos : NatOrders ls} {x y : L.C} {xs ys : Vals ls} :
      no.le x y → le nos xs ys → le (.cons no nos) (.cons x xs) (.cons y ys)

theorem le_refl : ∀ {ls : List Coll.{u}} (nos : NatOrders ls) (v : Vals ls),
    nos.le v v
  | [], .nil, .nil => .nil
  | _ :: _, .cons no nos, .cons x xs => .cons (no.le_refl x) (le_refl nos xs)

theorem le_trans : ∀ {ls : List Coll.{u}} (nos : NatOrders ls)
    {a b c : Vals ls}, nos.le a b → nos.le b c → nos.le a c
  | _, _, _, _, _, .nil, .nil => .nil
  | _, _, _, _, _, .cons h₁ h₁s, .cons h₂ h₂s =>
    .cons (NaturalOrder.le_trans _ h₁ h₂) (le_trans _ h₁s h₂s)

/-- Concatenation moves tuples up in the pointwise natural order. -/
theorem concat_le : ∀ {ls : List Coll.{u}} (nos : NatOrders ls)
    (v δ : Vals ls), nos.le v (v.concat δ)
  | [], .nil, .nil, .nil => .nil
  | _ :: _, .cons no nos, .cons x xs, .cons y ys =>
    .cons (no.concat_le x y) (concat_le nos xs ys)

end NatOrders

/-- **Theorem 3.4.2 (Monotone Outputs)**: along *any* execution trace — in
particular one cut short by crash-stop failures, which merely freeze part of
the schedule (§3.4.2) — the output buffers only grow in their natural orders.
Consequently any side effects driven by emitted outputs are a subset of the
intended ones: the CALM-style safety guarantee of §3.4.4.

Proof: trace induction; the only rule touching output buffers is delta
concatenation, which is aligned with the natural order by Def 3.4.1. -/
theorem monotone_outputs {i o : List Coll.{u}} (nos : NatOrders o)
    {c c' : Graph.Config i o}
    (htrace : Star Graph.CStep c c') :
    nos.le c.O c'.O := by
  induction htrace with
  | refl => exact nos.le_refl _
  | tail _ hstep ih =>
    obtain ⟨δ, _, hO⟩ := hstep
    rw [hO]
    exact nos.le_trans ih (nos.concat_le _ δ)

/-- **Theorem 3.4.1 (Eventual Determinism)**, inherited from Flo: under
liveness (the scheduler eventually performs every available step — the
formal content of "all machines are live and all network messages are
delivered", §3.4.3), a Gyatso program's outputs settle to a *unique* stuck
state, regardless of scheduling, network interleaving, or which cluster member
steps when. This is a restatement of `Graph.unique_stuck` (Lemmas 2.4.2/2.4.3);
Gyatso inherits it unchanged because cluster and network operators are ordinary
lawful Flo operators in our semantics (§3.4.1). -/
theorem eventual_determinism {i o : List Coll.{u}} (g : Graph i o)
    (hl : g.LeavesLawful) (O : Vals o) :
    ∃ f, NormalizesTo Graph.CStep ⟨g, O⟩ f ∧
      ∀ f', NormalizesTo Graph.CStep ⟨g, O⟩ f' → f' = f :=
  Graph.unique_stuck g hl O

/-- Combining both: even if a run is interrupted by failures at an arbitrary
point `c'`, its outputs are below the settled outputs `f.O` of the intended
failure-free execution — the outputs never "go astray" (§3.4.4). Requires the
failure-free continuation from `c'` (liveness after the failure point is
restored), which is `Graph.deterministic_and_eager`'s guarantee. -/
theorem outputs_below_settled {i o : List Coll.{u}} (nos : NatOrders o)
    {g : Graph i o} (hl : g.LeavesLawful) {O : Vals o} {c' : Graph.Config i o}
    (htrace : Star Graph.CStep ⟨g, O⟩ c')
    {f : Graph.Config i o} (hf : NormalizesTo Graph.CStep ⟨g, O⟩ f) :
    nos.le c'.O f.O := by
  -- the failure point c' still reaches the unique stuck state
  obtain ⟨det, _⟩ := Graph.deterministic_and_eager g hl
  obtain ⟨d, hcd, hfd⟩ := det O c' f htrace hf.1
  cases Star.eq_of_stuck hfd hf.2
  exact monotone_outputs nos hcd

end HydroLean.Gyatso
