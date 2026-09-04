import Hydro.Graph.IR

/-!
# Hydro · Graph — lag vectors and the knot cascade's lag algebra

This file defines a lag per wire of a context (`Lags`), the arithmetic the lag analysis in
`Graph/Causal.lean` (`Dag.lagOf`) needs, and the **lag algebra of a knot cascade**. A knot
body's knot-output lags are a function `Φ` of its knot-input lags, the externals held at
fixed lags. `Φ` satisfies five laws (`LagLaws`: monotone, sub-additive, max-distributive,
`Φ₀ 0 = 0`, and `Φ m = max (Φ₀ m) (Φ 0)` where `Φ₀` is the same functional with the externals
at lag `0`), and closing the first knot (`LagFun.next`) preserves them.

A cascade is **valid** (`cascadeValid`) when every knot, with the earlier knots closed, is
strict in its own wire. This is Rust's "acyclic through strata": every cycle among the
knots passes a network hop or a `defer_tick`; not every knot output has to. The closure
lags of a valid cascade are `closeLags` (the lags of the `bekic` cascade), bounded by the
externals' maximum (`closeLags_le`). Two facts about a valid cascade carry the interpreter
and the sweep: the Jacobi iteration of its lag functional from "every knot unknown"
reaches "every knot known" in as many rounds as there are knots (`iter_zero`), and the
only post-fixpoint below `1` is `0` (`postfix_zero`).
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-! ## Lag vectors

A lag per wire of a context, `HList (fun _ => Nat) κs`, with the arithmetic the lag
analysis needs: constants, the maximum, a constant added everywhere, the pointwise
order. -/

/-- A lag per wire. -/
abbrev Lags (κs : List (Kind L mem)) : Type := HList (fun _ => Nat) κs

namespace Lags

def const : (κs : List (Kind L mem)) → Nat → Lags κs
  | [], _ => HList.nil
  | _ :: κs, v => HList.cons v (const κs v)

/-- The maximum over the vector; `0` at the empty one. -/
def maxAll : {κs : List (Kind L mem)} → Lags κs → Nat
  | [], _ => 0
  | _ :: _, l => max l.1 (maxAll l.2)

/-- A constant added to every entry. -/
def addC : {κs : List (Kind L mem)} → Lags κs → Nat → Lags κs
  | [], _, _ => HList.nil
  | _ :: _, l, c => HList.cons (l.1 + c) (addC l.2 c)

/-- The pointwise order. -/
def Le : {κs : List (Kind L mem)} → Lags κs → Lags κs → Prop
  | [], _, _ => True
  | _ :: _, l, l' => l.1 ≤ l'.1 ∧ Le l.2 l'.2

/-- Every entry is `0`. -/
def allZero : {κs : List (Kind L mem)} → Lags κs → Bool
  | [], _ => true
  | _ :: _, l => (l.1 == 0) && allZero l.2

theorem get_const : {κs : List (Kind L mem)} → {κ : Kind L mem} → (v : Nat) → (x : Var κs κ) →
    (const κs v).get x = v
  | _ :: _, _, _, .zero => rfl
  | _ :: _, _, v, .succ x => get_const v x

theorem const_append : (κs Γ : List (Kind L mem)) → (v : Nat) →
    (const κs v).append (const Γ v) = const (κs ++ Γ) v
  | [], _, _ => rfl
  | _ :: κs, Γ, v => by
    show HList.cons v ((const κs v).append (const Γ v)) = HList.cons v (const (κs ++ Γ) v)
    rw [const_append κs Γ v]

theorem take_const (κs Γ : List (Kind L mem)) (v : Nat) :
    HList.take (κs := κs) (const (κs ++ Γ) v) = const κs v := by
  rw [← const_append, HList.take_append]

theorem drop_const (κs Γ : List (Kind L mem)) (v : Nat) :
    HList.drop (κs := κs) (const (κs ++ Γ) v) = const Γ v := by
  rw [← const_append, HList.drop_append]

theorem get_addC : {κs : List (Kind L mem)} → {κ : Kind L mem} → (l : Lags κs) → (c : Nat) → (x : Var κs κ) →
    (addC l c).get x = l.get x + c
  | _ :: _, _, _, _, .zero => rfl
  | _ :: _, _, l, c, .succ x => get_addC l.2 c x

theorem addC_const : (κs : List (Kind L mem)) → (v c : Nat) → addC (const κs v) c = const κs (v + c)
  | [], _, _ => rfl
  | _ :: κs, v, c => by
    show HList.cons (v + c) (addC (const κs v) c) = HList.cons (v + c) (const κs (v + c))
    rw [addC_const κs v c]

theorem addC_append : {κs Γ : List (Kind L mem)} → (a : Lags κs) → (b : Lags Γ) → (c : Nat) →
    addC (a.append b) c = (addC a c).append (addC b c)
  | [], _, _, _, _ => rfl
  | _ :: κs, _, a, b, c => by
    show HList.cons (a.1 + c) (addC (a.2.append b) c) = HList.cons (a.1 + c) ((addC a.2 c).append (addC b c))
    rw [addC_append a.2 b c]

theorem take_addC : {κs Γ : List (Kind L mem)} → (l : Lags (κs ++ Γ)) → (c : Nat) →
    HList.take (κs := κs) (addC l c) = addC (HList.take (κs := κs) l) c
  | [], _, _, _ => rfl
  | _ :: κs, Γ, l, c => by
    show HList.cons (l.1 + c) (HList.take (κs := κs) (addC l.2 c)) = HList.cons (l.1 + c) (addC (HList.take (κs := κs) l.2) c)
    rw [take_addC (κs := κs) (Γ := Γ) l.2 c]

theorem drop_addC : {κs Γ : List (Kind L mem)} → (l : Lags (κs ++ Γ)) → (c : Nat) →
    HList.drop (κs := κs) (addC l c) = addC (HList.drop (κs := κs) l) c
  | [], _, _, _ => rfl
  | _ :: κs, Γ, l, c => drop_addC (κs := κs) (Γ := Γ) l.2 c

theorem mapSubst_addC {Γ Γ' : Ctx L mem} (l : Lags Γ) (c : Nat) (args : Subst Γ' Γ) :
    args.map (fun v => (addC l c).get v) = addC (args.map (fun v => l.get v)) c := by
  induction Γ' with
  | nil => rfl
  | cons κ κs ih =>
    show HList.cons ((addC l c).get args.1) (args.2.map (fun v => (addC l c).get v)) = _
    rw [ih args.2, get_addC]
    rfl

theorem maxAll_addC_le : {κs : List (Kind L mem)} → (l : Lags κs) → (c : Nat) → maxAll (addC l c) ≤ maxAll l + c
  | [], _, _ => Nat.zero_le _
  | _ :: κs, l, c => by
    show max (l.1 + c) (maxAll (addC l.2 c)) ≤ max l.1 (maxAll l.2) + c
    have := maxAll_addC_le l.2 c
    exact Nat.max_le.mpr ⟨Nat.add_le_add_right (Nat.le_max_left _ _) c,
      this.trans (Nat.add_le_add_right (Nat.le_max_right _ _) c)⟩

theorem max_add_le (a b c : Nat) : max (a + c) (b + c) ≤ max a b + c :=
  Nat.max_le.mpr ⟨Nat.add_le_add_right (Nat.le_max_left _ _) c, Nat.add_le_add_right (Nat.le_max_right _ _) c⟩

theorem le_maxAll : {κs : List (Kind L mem)} → {κ : Kind L mem} → (l : Lags κs) → (x : Var κs κ) → l.get x ≤ maxAll l
  | _ :: _, _, l, .zero => Nat.le_max_left _ _
  | _ :: _, _, l, .succ x => (le_maxAll l.2 x).trans (Nat.le_max_right _ _)

theorem Le.refl : {κs : List (Kind L mem)} → (l : Lags κs) → Le l l
  | [], _ => trivial
  | _ :: _, l => ⟨Nat.le_refl _, Le.refl l.2⟩

theorem Le.trans : {κs : List (Kind L mem)} → {a b c : Lags κs} → Le a b → Le b c → Le a c
  | [], _, _, _, _, _ => trivial
  | _ :: _, _, _, _, h₁, h₂ => ⟨h₁.1.trans h₂.1, Le.trans h₁.2 h₂.2⟩

theorem Le.get : {κs : List (Kind L mem)} → {κ : Kind L mem} → {l l' : Lags κs} → Le l l' → (x : Var κs κ) →
    l.get x ≤ l'.get x
  | _ :: _, _, _, _, h, .zero => h.1
  | _ :: _, _, _, _, h, .succ x => Le.get h.2 x

theorem Le.const : {κs : List (Kind L mem)} → {a b : Nat} → a ≤ b → Le (const κs a) (const κs b)
  | [], _, _, _ => trivial
  | _ :: _, _, _, h => ⟨h, Le.const h⟩

theorem Le.append : {κs Γ : List (Kind L mem)} → {a a' : Lags κs} → {b b' : Lags Γ} → Le a a' → Le b b' →
    Le (a.append b) (a'.append b')
  | [], _, _, _, _, _, _, hb => hb
  | _ :: _, _, _, _, _, _, ha, hb => ⟨ha.1, Le.append ha.2 hb⟩

theorem Le.take : {κs Γ : List (Kind L mem)} → {l l' : Lags (κs ++ Γ)} → Le l l' →
    Le (HList.take (κs := κs) l) (HList.take (κs := κs) l')
  | [], _, _, _, _ => trivial
  | _ :: κs, Γ, _, _, h => ⟨h.1, Le.take (κs := κs) (Γ := Γ) h.2⟩

theorem Le.drop : {κs Γ : List (Kind L mem)} → {l l' : Lags (κs ++ Γ)} → Le l l' →
    Le (HList.drop (κs := κs) l) (HList.drop (κs := κs) l')
  | [], _, _, _, h => h
  | _ :: κs, Γ, _, _, h => Le.drop (κs := κs) (Γ := Γ) h.2

theorem Le.mapSubst {Γ Γ' : Ctx L mem} {l l' : Lags Γ} (h : Le l l') (args : Subst Γ' Γ) :
    Le (args.map (fun v => l.get v)) (args.map (fun v => l'.get v)) := by
  induction Γ' with
  | nil => trivial
  | cons κ κs ih => exact ⟨h.get args.1, ih args.2⟩

theorem Le.addC : {κs : List (Kind L mem)} → {l l' : Lags κs} → Le l l' → (c : Nat) → Le (addC l c) (addC l' c)
  | [], _, _, _, _ => trivial
  | _ :: _, _, _, h, c => ⟨Nat.add_le_add_right h.1 c, Le.addC h.2 c⟩

theorem le_const_of_maxAll : {κs : List (Kind L mem)} → (l : Lags κs) → {v : Nat} → maxAll l ≤ v → Le l (const κs v)
  | [], _, _, _ => trivial
  | _ :: _, l, v, h => by
    show l.1 ≤ v ∧ Le l.2 (const _ v)
    have h' : max l.1 (maxAll l.2) ≤ v := h
    exact ⟨le_of_max_le_left h', le_const_of_maxAll l.2 (le_of_max_le_right h')⟩

theorem maxAll_le_of_le : {κs : List (Kind L mem)} → {l : Lags κs} → {v : Nat} → Le l (const κs v) → maxAll l ≤ v
  | [], _, _, _ => Nat.zero_le _
  | _ :: _, l, v, h => by
    show max l.1 (maxAll l.2) ≤ v
    exact max_le h.1 (maxAll_le_of_le h.2)

theorem maxAll_mono {κs : List (Kind L mem)} {l l' : Lags κs} (h : Le l l') : maxAll l ≤ maxAll l' :=
  maxAll_le_of_le (h.trans (le_const_of_maxAll l' (Nat.le_refl _)))

theorem allZero_eq : {κs : List (Kind L mem)} → (l : Lags κs) → allZero l = true → l = const κs 0
  | [], _, _ => rfl
  | _ :: κs, l, h => by
    have h' := Bool.and_eq_true_iff.mp h
    have h1 : l.1 = 0 := by simpa using h'.1
    show (l.1, l.2) = ((0 : Nat), const κs 0)
    rw [h1, allZero_eq l.2 h'.2]

theorem eq_allZero : {κs : List (Kind L mem)} → (l : Lags κs) → Le l (const κs 0) → allZero l = true
  | [], _, _ => rfl
  | _ :: κs, l, h => by
    show ((l.1 == 0) && allZero l.2) = true
    rw [Bool.and_eq_true_iff]
    exact ⟨by simpa using Nat.le_zero.mp h.1, eq_allZero l.2 h.2⟩

end Lags

/-! ### The pointwise maximum and the sum -/

namespace Lags

/-- The pointwise maximum. -/
def sup : {κs : List (Kind L mem)} → Lags κs → Lags κs → Lags κs
  | [], _, _ => HList.nil
  | _ :: _, a, b => HList.cons (max a.1 b.1) (sup a.2 b.2)

/-- The sum of the entries. -/
def sum : {κs : List (Kind L mem)} → Lags κs → Nat
  | [], _ => 0
  | _ :: _, l => l.1 + sum l.2

theorem get_sup : {κs : List (Kind L mem)} → {κ : Kind L mem} → (a b : Lags κs) → (x : Var κs κ) →
    (sup a b).get x = max (a.get x) (b.get x)
  | _ :: _, _, _, _, .zero => rfl
  | _ :: _, _, a, b, .succ x => get_sup a.2 b.2 x

theorem sup_comm : {κs : List (Kind L mem)} → (a b : Lags κs) → sup a b = sup b a
  | [], _, _ => rfl
  | _ :: _, a, b => by
    show HList.cons (max a.1 b.1) (sup a.2 b.2) = HList.cons (max b.1 a.1) (sup b.2 a.2)
    rw [Nat.max_comm, sup_comm a.2 b.2]

theorem sup_assoc : {κs : List (Kind L mem)} → (a b c : Lags κs) → sup (sup a b) c = sup a (sup b c)
  | [], _, _, _ => rfl
  | _ :: _, a, b, c => by
    show HList.cons (max (max a.1 b.1) c.1) (sup (sup a.2 b.2) c.2) = HList.cons (max a.1 (max b.1 c.1)) (sup a.2 (sup b.2 c.2))
    rw [Nat.max_assoc, sup_assoc a.2 b.2 c.2]

theorem sup_left_comm {κs : List (Kind L mem)} (a b c : Lags κs) : sup a (sup b c) = sup b (sup a c) := by
  rw [← sup_assoc, sup_comm a b, sup_assoc]

theorem sup_self : {κs : List (Kind L mem)} → (a : Lags κs) → sup a a = a
  | [], _ => rfl
  | _ :: _, a => by
    show HList.cons (max a.1 a.1) (sup a.2 a.2) = a
    rw [Nat.max_self, sup_self a.2]

theorem sup_const_zero_right : {κs : List (Kind L mem)} → (a : Lags κs) → sup a (const κs 0) = a
  | [], _ => rfl
  | _ :: κs, a => by
    show HList.cons (max a.1 0) (sup a.2 (const κs 0)) = a
    rw [Nat.max_zero, sup_const_zero_right a.2]

theorem sup_const_zero_left : {κs : List (Kind L mem)} → (a : Lags κs) → sup (const κs 0) a = a
  | _, a => by rw [sup_comm, sup_const_zero_right]

theorem sup_append : {κs Γ : List (Kind L mem)} → (a a' : Lags κs) → (b b' : Lags Γ) →
    sup (a.append b) (a'.append b') = (sup a a').append (sup b b')
  | [], _, _, _, _, _ => rfl
  | _ :: κs, _, a, a', b, b' => by
    show HList.cons (max a.1 a'.1) (sup (a.2.append b) (a'.2.append b')) = HList.cons (max a.1 a'.1) ((sup a.2 a'.2).append (sup b b'))
    rw [sup_append a.2 a'.2 b b']

theorem take_sup : {κs Γ : List (Kind L mem)} → (a b : Lags (κs ++ Γ)) →
    HList.take (κs := κs) (sup a b) = sup (HList.take (κs := κs) a) (HList.take (κs := κs) b)
  | [], _, _, _ => rfl
  | _ :: κs, Γ, a, b => by
    show HList.cons (max a.1 b.1) (HList.take (κs := κs) (sup a.2 b.2)) = HList.cons (max a.1 b.1) (sup (HList.take (κs := κs) a.2) (HList.take (κs := κs) b.2))
    rw [take_sup (κs := κs) (Γ := Γ) a.2 b.2]

theorem drop_sup : {κs Γ : List (Kind L mem)} → (a b : Lags (κs ++ Γ)) →
    HList.drop (κs := κs) (sup a b) = sup (HList.drop (κs := κs) a) (HList.drop (κs := κs) b)
  | [], _, _, _ => rfl
  | _ :: κs, Γ, a, b => drop_sup (κs := κs) (Γ := Γ) a.2 b.2

theorem const_sup : (κs : List (Kind L mem)) → (u v : Nat) → const κs (max u v) = sup (const κs u) (const κs v)
  | [], _, _ => rfl
  | _ :: κs, u, v => by
    show HList.cons (max u v) (const κs (max u v)) = HList.cons (max u v) (sup (const κs u) (const κs v))
    rw [const_sup κs u v]

theorem maxAll_sup : {κs : List (Kind L mem)} → (a b : Lags κs) → maxAll (sup a b) = max (maxAll a) (maxAll b)
  | [], _, _ => rfl
  | _ :: _, a, b => by
    show max (max a.1 b.1) (maxAll (sup a.2 b.2)) = max (max a.1 (maxAll a.2)) (max b.1 (maxAll b.2))
    rw [maxAll_sup a.2 b.2]
    ac_rfl

theorem maxAll_append : {κs Γ : List (Kind L mem)} → (a : Lags κs) → (b : Lags Γ) →
    maxAll (a.append b) = max (maxAll a) (maxAll b)
  | [], _, _, b => by show maxAll b = max 0 (maxAll b); rw [Nat.zero_max]
  | _ :: κs, Γ, a, b => by
    show max a.1 (maxAll (a.2.append b)) = max (max a.1 (maxAll a.2)) (maxAll b)
    rw [maxAll_append a.2 b]
    ac_rfl

theorem maxAll_const_zero : (κs : List (Kind L mem)) → maxAll (const κs 0) = 0
  | [] => rfl
  | _ :: κs => by show max 0 (maxAll (const κs 0)) = 0; rw [maxAll_const_zero κs]; rfl

theorem mapSubst_sup {Γ Γ' : Ctx L mem} (a b : Lags Γ) (args : Subst Γ' Γ) :
    args.map (fun v => (sup a b).get v) = sup (args.map (fun v => a.get v)) (args.map (fun v => b.get v)) := by
  induction Γ' with
  | nil => rfl
  | cons κ κs ih =>
    show HList.cons ((sup a b).get args.1) (args.2.map (fun v => (sup a b).get v)) = _
    rw [ih args.2, get_sup]
    rfl

theorem mapSubst_const {Γ Γ' : Ctx L mem} (c : Nat) (args : Subst Γ' Γ) :
    args.map (fun v => (const Γ c).get v) = const Γ' c := by
  induction Γ' with
  | nil => rfl
  | cons κ κs ih =>
    show HList.cons ((const Γ c).get args.1) (args.2.map (fun v => (const Γ c).get v)) = HList.cons c (const κs c)
    rw [ih args.2, get_const]

theorem max_sub_one (a b : Nat) : max a b - 1 = max (a - 1) (b - 1) := by omega

theorem max_max_max (a a' b b' : Nat) : max (max a a') (max b b') = max (max a b) (max a' b') := by omega

theorem zero_le : {κs : List (Kind L mem)} → (l : Lags κs) → Le (const κs 0) l
  | [], _ => trivial
  | _ :: _, l => ⟨Nat.zero_le _, zero_le l.2⟩

theorem le_sup_left : {κs : List (Kind L mem)} → (a b : Lags κs) → Le a (sup a b)
  | [], _, _ => trivial
  | _ :: _, a, b => ⟨Nat.le_max_left _ _, le_sup_left a.2 b.2⟩

theorem le_sup_right : {κs : List (Kind L mem)} → (a b : Lags κs) → Le b (sup a b)
  | [], _, _ => trivial
  | _ :: _, a, b => ⟨Nat.le_max_right _ _, le_sup_right a.2 b.2⟩

theorem sup_le : {κs : List (Kind L mem)} → {a b c : Lags κs} → Le a c → Le b c → Le (sup a b) c
  | [], _, _, _, _, _ => trivial
  | _ :: _, _, _, _, ha, hb => ⟨Nat.max_le.mpr ⟨ha.1, hb.1⟩, sup_le ha.2 hb.2⟩

theorem Le.sup : {κs : List (Kind L mem)} → {a a' b b' : Lags κs} → Le a a' → Le b b' → Le (sup a b) (sup a' b')
  | [], _, _, _, _, _, _ => trivial
  | _ :: _, _, _, _, _, ha, hb => ⟨max_le_max ha.1 hb.1, Le.sup ha.2 hb.2⟩

theorem sup_eq_left_of_le : {κs : List (Kind L mem)} → {a b : Lags κs} → Le b a → sup a b = a
  | [], _, _, _ => rfl
  | _ :: _, a, b, h => by
    show HList.cons (max a.1 b.1) (sup a.2 b.2) = a
    rw [Nat.max_eq_left h.1, sup_eq_left_of_le h.2]

theorem sum_le : {κs : List (Kind L mem)} → {a b : Lags κs} → Le a b → sum a ≤ sum b
  | [], _, _, _ => Nat.le_refl _
  | _ :: _, _, _, h => Nat.add_le_add h.1 (sum_le h.2)

theorem sum_lt : {κs : List (Kind L mem)} → {a b : Lags κs} → Le a b → a ≠ b → sum a < sum b
  | [], a, b, _, hne => absurd rfl hne
  | _ :: _, a, b, h, hne => by
    by_cases h1 : a.1 = b.1
    · have h2 : a.2 ≠ b.2 := fun h2 => hne (Prod.ext h1 h2)
      exact Nat.add_lt_add_of_le_of_lt (Nat.le_of_eq h1) (sum_lt h.2 h2)
    · exact Nat.add_lt_add_of_lt_of_le (Nat.lt_of_le_of_ne h.1 h1) (sum_le h.2)

theorem sum_const_one : (κs : List (Kind L mem)) → sum (const κs 1) = κs.length
  | [] => rfl
  | _ :: κs => by show 1 + sum (const κs 1) = κs.length + 1; rw [sum_const_one κs]; omega

theorem eq_const_zero_of_sum : {κs : List (Kind L mem)} → (l : Lags κs) → sum l = 0 → l = const κs 0
  | [], _, _ => rfl
  | _ :: κs, l, h => by
    obtain ⟨a, l⟩ := l
    have h' : (a : Nat) + sum l = 0 := h
    have ha : a = 0 := by omega
    have hl : sum l = 0 := by omega
    show (a, l) = ((0 : Nat), const κs 0)
    rw [eq_const_zero_of_sum l hl, ha]

end Lags

/-! ## The knot cascade's lag algebra

A knot body's knot-output lags are a function `Φ : Lags K → Lags K` of its knot-input
lags, the externals held fixed. Closing the first knot innermost, as the Bekić cascade
`bekic` does, leaves a cascade over the rest whose lag functional is `LagFun.next Φ`: the
first knot's closure lag, given the rest at `m`, is its output lag with its own wire at
`0`. This is sound once the knot is strict in itself, because then the least fixpoint of
`λ ↦ (Φ (λ :: m)).1` is `(Φ (0 :: m)).1` (`LagLaws.head_bound`). -/

namespace LagFun

/-- The lag functional of the rest of the cascade, the first knot closed. -/
def next {κ : Kind L mem} {κs : List (Kind L mem)} (Φ : Lags (κ :: κs) → Lags (κ :: κs)) : Lags κs → Lags κs :=
  fun m => (Φ (HList.cons (Φ (HList.cons 0 m)).1 m)).2

end LagFun

/-- **Cascade validity**: every knot, the earlier knots closed, is strict in its own wire:
its output lag is `0` when its wire is at `1` and every other wire at `0`. -/
def cascadeValid : (κs : List (Kind L mem)) → (Lags κs → Lags κs) → Bool
  | [], _ => true
  | _ :: κs, Φ => ((Φ (HList.cons 1 (Lags.const κs 0))).1 == 0) && cascadeValid κs (LagFun.next Φ)

/-- The closure lags of the cascade: the first knot's, given the rest's. -/
def closeLags : (κs : List (Kind L mem)) → (Lags κs → Lags κs) → Lags κs
  | [], _ => HList.nil
  | _ :: κs, Φ =>
    let rest := closeLags κs (LagFun.next Φ)
    HList.cons (Φ (HList.cons 0 rest)).1 rest

theorem cascadeValid_nil (Φ : Lags ([] : List (Kind L mem)) → Lags []) : cascadeValid [] Φ = true := rfl

theorem cascadeValid_cons {κ : Kind L mem} {κs : List (Kind L mem)} {Φ : Lags (κ :: κs) → Lags (κ :: κs)}
    (h : cascadeValid (κ :: κs) Φ = true) :
    (Φ (HList.cons 1 (Lags.const κs 0))).1 = 0 ∧ cascadeValid κs (LagFun.next Φ) = true := by
  have h' := Bool.and_eq_true_iff.mp h
  exact ⟨by simpa using h'.1, h'.2⟩

/-- One knot of the cascade check on literals: its probe `p`, the body's knot-output lags
`r` at it (the generator's `Dag.lagOf` chain), the head `0`. -/
theorem cascadeValid_cons_lit {κ : Kind L mem} {κs : List (Kind L mem)} (Φ : Lags (κ :: κs) → Lags (κ :: κs))
    (p r : Lags (κ :: κs)) (hp : HList.cons 1 (Lags.const κs 0) = p) (hr : Φ p = r) (h0 : r.1 = 0)
    (hrest : cascadeValid κs (LagFun.next Φ) = true) : cascadeValid (κ :: κs) Φ = true := by
  show (((Φ (HList.cons 1 (Lags.const κs 0))).1 == 0) && cascadeValid κs (LagFun.next Φ)) = true
  rw [hp, hr, h0, hrest]
  rfl

/-- One evaluation of the rest's lag functional on literals: the first knot's closure lag at
`p` (`a`), the probe it closes to (`q`), the body's lags there (`b`), the rest's (`r`). -/
theorem LagFun.next_lit {κ : Kind L mem} {κs : List (Kind L mem)} (Φ : Lags (κ :: κs) → Lags (κ :: κs))
    (p : Lags κs) (a q b : Lags (κ :: κs)) (r : Lags κs)
    (ha : Φ (HList.cons 0 p) = a) (hq : HList.cons a.1 p = q) (hb : Φ q = b) (hr : b.2 = r) :
    LagFun.next Φ p = r := by
  show (Φ (HList.cons (Φ (HList.cons 0 p)).1 p)).2 = r
  rw [ha, hq, hb, hr]

/-- The laws of a knot body's lag functional. `Φ₀` is the body's lag functional with the
externals at lag `0`, `Φ` the one with them at their lags. -/
structure LagLaws {K : List (Kind L mem)} (Φ₀ Φ : Lags K → Lags K) : Prop where
  mono : ∀ {m m' : Lags K}, Lags.Le m m' → Lags.Le (Φ₀ m) (Φ₀ m')
  sub : ∀ (m : Lags K) (c : Nat), Lags.Le (Φ₀ (Lags.addC m c)) (Lags.addC (Φ₀ m) c)
  max : ∀ (m m' : Lags K), Φ₀ (Lags.sup m m') = Lags.sup (Φ₀ m) (Φ₀ m')
  zero : Φ₀ (Lags.const K 0) = Lags.const K 0
  mix : ∀ (m : Lags K), Φ m = Lags.sup (Φ₀ m) (Φ (Lags.const K 0))

namespace LagLaws

variable {K : List (Kind L mem)} {Φ₀ Φ : Lags K → Lags K}

theorem le₀ (h : LagLaws Φ₀ Φ) (m : Lags K) : Lags.Le (Φ₀ m) (Φ m) := by
  rw [h.mix m]; exact Lags.le_sup_left _ _

theorem monoΦ (h : LagLaws Φ₀ Φ) {m m' : Lags K} (hm : Lags.Le m m') : Lags.Le (Φ m) (Φ m') := by
  rw [h.mix m, h.mix m']; exact Lags.Le.sup (h.mono hm) (Lags.Le.refl _)

theorem maxΦ (h : LagLaws Φ₀ Φ) (m m' : Lags K) : Φ (Lags.sup m m') = Lags.sup (Φ m) (Φ m') := by
  rw [h.mix (Lags.sup m m'), h.max, h.mix m, h.mix m']
  rw [Lags.sup_assoc, Lags.sup_assoc]
  congr 1
  rw [Lags.sup_left_comm, Lags.sup_self]

theorem self (h : LagLaws Φ₀ Φ) : LagLaws Φ₀ Φ₀ where
  mono := h.mono
  sub := h.sub
  max := h.max
  zero := h.zero
  mix := fun m => by rw [h.zero, Lags.sup_const_zero_right]

theorem le_const (h : LagLaws Φ₀ Φ) (c : Nat) : Lags.Le (Φ₀ (Lags.const K c)) (Lags.const K c) := by
  have := h.sub (Lags.const K 0) c
  rw [Lags.addC_const, Nat.zero_add, h.zero, Lags.addC_const, Nat.zero_add] at this
  exact this

theorem le_maxAll (h : LagLaws Φ₀ Φ) (m : Lags K) : Lags.Le (Φ₀ m) (Lags.const K (Lags.maxAll m)) :=
  (h.mono (Lags.le_const_of_maxAll m (Nat.le_refl _))).trans (h.le_const _)

/-- `Φ` is bounded by `E` once its value at `0` is. -/
theorem Φ_le_const (h : LagLaws Φ₀ Φ) {E : Nat} (hE : Lags.Le (Φ (Lags.const K 0)) (Lags.const K E)) :
    Lags.Le (Φ (Lags.const K E)) (Lags.const K E) := by
  rw [h.mix]
  exact Lags.sup_le (h.le_const E) hE

end LagLaws

section Cascade

variable {κ : Kind L mem} {κs : List (Kind L mem)}

theorem cons_eq_sup (a : Nat) (m : Lags κs) :
    (HList.cons a m : Lags (κ :: κs)) = Lags.sup (HList.cons a (Lags.const κs 0)) (HList.cons 0 m) := by
  show HList.cons a m = HList.cons (max a 0) (Lags.sup (Lags.const κs 0) m)
  rw [Nat.max_zero, Lags.sup_const_zero_left]

theorem cons_const_zero : (HList.cons 0 (Lags.const κs 0) : Lags (κ :: κs)) = Lags.const (κ :: κs) 0 := rfl

theorem sup_cons (a b : Nat) (m m' : Lags κs) :
    Lags.sup (HList.cons a m : Lags (κ :: κs)) (HList.cons b m') = HList.cons (max a b) (Lags.sup m m') := rfl

theorem addC_cons (a : Nat) (m : Lags κs) (c : Nat) :
    Lags.addC (HList.cons a m : Lags (κ :: κs)) c = HList.cons (a + c) (Lags.addC m c) := rfl

/-- **The Bekić inner step.** A knot strict in itself (`hhead`), with the rest at `m`, has
its output at lag `≤ L` when its own wire is at `L + 1`, for every `L` at or above its
closure lag `(Φ (0 :: m)).1`. -/
theorem LagLaws.head_bound {Φ₀ Φ : Lags (κ :: κs) → Lags (κ :: κs)} (h : LagLaws Φ₀ Φ)
    (hhead : (Φ₀ (HList.cons 1 (Lags.const κs 0))).1 = 0) (m : Lags κs) (L : Nat)
    (hL : (Φ (HList.cons 0 m)).1 ≤ L) : (Φ (HList.cons (L + 1) m)).1 ≤ L := by
  rw [cons_eq_sup, h.maxΦ]
  show Max.max (Φ (HList.cons (L + 1) (Lags.const κs 0))).1 (Φ (HList.cons 0 m)).1 ≤ L
  refine Nat.max_le.mpr ⟨?_, hL⟩
  rw [h.mix]
  show Max.max (Φ₀ (HList.cons (L + 1) (Lags.const κs 0))).1 (Φ (Lags.const (κ :: κs) 0)).1 ≤ L
  refine Nat.max_le.mpr ⟨?_, ?_⟩
  · have h1 : Lags.Le (HList.cons (L + 1) (Lags.const κs 0) : Lags (κ :: κs))
        (Lags.addC (HList.cons 1 (Lags.const κs 0)) L) := by
      rw [addC_cons, Lags.addC_const, Nat.zero_add, Nat.add_comm 1 L]
      exact ⟨Nat.le_refl _, Lags.zero_le _⟩
    have h2 := ((h.mono h1).trans (h.sub _ L)).1
    change (Φ₀ (HList.cons (L + 1) (Lags.const κs 0))).1 ≤ (Φ₀ (HList.cons 1 (Lags.const κs 0))).1 + L at h2
    rw [hhead, Nat.zero_add] at h2
    exact h2
  · exact ((h.monoΦ (m := Lags.const (κ :: κs) 0) (m' := HList.cons 0 m) ⟨Nat.le_refl _, Lags.zero_le m⟩).1).trans hL

/-- Closing the first knot preserves the laws. -/
theorem LagLaws.next {Φ₀ Φ : Lags (κ :: κs) → Lags (κ :: κs)} (h : LagLaws Φ₀ Φ) :
    LagLaws (LagFun.next Φ₀) (LagFun.next Φ) where
  mono := fun {m m'} hm => by
    have h1 : (Φ₀ (HList.cons 0 m)).1 ≤ (Φ₀ (HList.cons 0 m')).1 :=
      (h.mono (m := HList.cons 0 m) (m' := HList.cons 0 m') ⟨Nat.le_refl 0, hm⟩).1
    exact (h.mono (m := HList.cons _ m) (m' := HList.cons _ m') ⟨h1, hm⟩).2
  sub := fun m c => by
    have h1 : (Φ₀ (HList.cons 0 (Lags.addC m c))).1 ≤ (Φ₀ (HList.cons 0 m)).1 + c := by
      have := ((h.mono (m := HList.cons 0 (Lags.addC m c)) (m' := Lags.addC (HList.cons 0 m) c)
        ⟨Nat.zero_le _, Lags.Le.refl _⟩).trans (h.sub (HList.cons 0 m) c)).1
      exact this
    exact ((h.mono (m := HList.cons _ (Lags.addC m c)) (m' := Lags.addC (HList.cons (Φ₀ (HList.cons 0 m)).1 m) c)
      ⟨h1, Lags.Le.refl _⟩).trans (h.sub (HList.cons (Φ₀ (HList.cons 0 m)).1 m) c)).2
  max := fun m m' => by
    have h1 : (Φ₀ (HList.cons 0 (Lags.sup m m'))).1 = Max.max (Φ₀ (HList.cons 0 m)).1 (Φ₀ (HList.cons 0 m')).1 := by
      have e : (HList.cons 0 (Lags.sup m m') : Lags (κ :: κs)) = Lags.sup (HList.cons 0 m) (HList.cons 0 m') := rfl
      rw [e, h.max]; rfl
    show (Φ₀ (HList.cons (Φ₀ (HList.cons 0 (Lags.sup m m'))).1 (Lags.sup m m'))).2 = _
    rw [h1, ← sup_cons, h.max]; rfl
  zero := by
    show (Φ₀ (HList.cons (Φ₀ (HList.cons 0 (Lags.const κs 0))).1 (Lags.const κs 0))).2 = Lags.const κs 0
    rw [cons_const_zero, h.zero]
    show (Φ₀ (HList.cons 0 (Lags.const κs 0))).2 = Lags.const κs 0
    rw [cons_const_zero, h.zero]; rfl
  mix := fun m => by
    -- the closure lag at `m` splits into the self part and the externals' constant
    have hlam : ∀ m, (Φ (HList.cons 0 m)).1 = Max.max (Φ₀ (HList.cons 0 m)).1 (Φ (Lags.const (κ :: κs) 0)).1 := by
      intro m; rw [h.mix (HList.cons 0 m)]; rfl
    have hlam0 : (Φ₀ (HList.cons 0 (Lags.const κs 0))).1 = 0 := by rw [cons_const_zero, h.zero]; rfl
    show (Φ (HList.cons (Φ (HList.cons 0 m)).1 m)).2
      = Lags.sup (Φ₀ (HList.cons (Φ₀ (HList.cons 0 m)).1 m)).2 (Φ (HList.cons (Φ (HList.cons 0 (Lags.const κs 0))).1 (Lags.const κs 0))).2
    rw [hlam m, hlam (Lags.const κs 0), hlam0, Nat.zero_max]
    -- both sides are `max (Φ₀ (λ₀ m :: m)).2 (max (Φ₀ (c₀ :: 0)).2 (Φ 0).2)`
    have e1 : (HList.cons (Max.max (Φ₀ (HList.cons 0 m)).1 (Φ (Lags.const (κ :: κs) 0)).1) m : Lags (κ :: κs))
        = Lags.sup (HList.cons (Φ₀ (HList.cons 0 m)).1 m) (HList.cons (Φ (Lags.const (κ :: κs) 0)).1 (Lags.const κs 0)) := by
      rw [sup_cons, Lags.sup_const_zero_right]
    rw [h.mix (HList.cons _ m), e1, h.max, h.mix (HList.cons (Φ (Lags.const (κ :: κs) 0)).1 (Lags.const κs 0))]
    show Lags.sup (Lags.sup (Φ₀ (HList.cons (Φ₀ (HList.cons 0 m)).1 m)).2 (Φ₀ (HList.cons (Φ (Lags.const (κ :: κs) 0)).1 (Lags.const κs 0))).2) (Φ (Lags.const (κ :: κs) 0)).2
      = Lags.sup (Φ₀ (HList.cons (Φ₀ (HList.cons 0 m)).1 m)).2 (Lags.sup (Φ₀ (HList.cons (Φ (Lags.const (κ :: κs) 0)).1 (Lags.const κs 0))).2 (Φ (Lags.const (κ :: κs) 0)).2)
    rw [Lags.sup_assoc]

/-- The closure lags of a cascade are bounded by any bound `E` of `Φ 0`, the externals'
contribution. -/
theorem closeLags_le : (κs : List (Kind L mem)) → (Φ₀ Φ : Lags κs → Lags κs) → LagLaws Φ₀ Φ → {E : Nat} →
    Lags.Le (Φ (Lags.const κs 0)) (Lags.const κs E) → Lags.Le (closeLags κs Φ) (Lags.const κs E)
  | [], _, _, _, _, _ => trivial
  | κ :: κs, Φ₀, Φ, h, E, hE => by
    have hE' : Lags.Le (LagFun.next Φ (Lags.const κs 0)) (Lags.const κs E) := by
      show Lags.Le (Φ (HList.cons (Φ (HList.cons 0 (Lags.const κs 0))).1 (Lags.const κs 0))).2 (Lags.const κs E)
      have h1 : Lags.Le (HList.cons (Φ (HList.cons 0 (Lags.const κs 0))).1 (Lags.const κs 0) : Lags (κ :: κs)) (Lags.const (κ :: κs) E) :=
        ⟨by rw [cons_const_zero]; exact hE.1, Lags.Le.const (Nat.zero_le E)⟩
      exact ((h.monoΦ h1).trans (h.Φ_le_const hE)).2
    have ih := closeLags_le κs (LagFun.next Φ₀) (LagFun.next Φ) h.next hE'
    refine ⟨?_, ih⟩
    show (Φ (HList.cons 0 (closeLags κs (LagFun.next Φ)))).1 ≤ E
    have h1 : Lags.Le (HList.cons 0 (closeLags κs (LagFun.next Φ)) : Lags (κ :: κs)) (Lags.const (κ :: κs) E) :=
      ⟨Nat.zero_le _, ih⟩
    exact ((h.monoΦ h1).trans (h.Φ_le_const hE)).1

/-- **The only post-fixpoint below `1` of a valid cascade's lag functional is `0`.** A set
of knots each of which reads another of the set at the same step would be a cycle without
a strict node, which `cascadeValid` excludes. -/
theorem postfix_zero : (κs : List (Kind L mem)) → (Φ₀ : Lags κs → Lags κs) → LagLaws Φ₀ Φ₀ →
    cascadeValid κs Φ₀ = true → (m : Lags κs) → Lags.Le m (Φ₀ m) → Lags.Le m (Lags.const κs 1) → m = Lags.const κs 0
  | [], _, _, _, _, _, _ => rfl
  | κ :: κs, Φ₀, h, hv, m, hpost, hle => by
    obtain ⟨hhead, hv'⟩ := cascadeValid_cons hv
    obtain ⟨a, m⟩ := m
    have ha : a ≤ (Φ₀ (HList.cons 0 m)).1 := by
      have h1 : a ≤ (Φ₀ (HList.cons a m)).1 := hpost.1
      rw [cons_eq_sup, h.max] at h1
      change a ≤ max (Φ₀ (HList.cons a (Lags.const κs 0))).1 (Φ₀ (HList.cons 0 m)).1 at h1
      have h2 : (Φ₀ (HList.cons a (Lags.const κs 0))).1 ≤ (Φ₀ (HList.cons 1 (Lags.const κs 0))).1 :=
        (h.mono (m := HList.cons a (Lags.const κs 0)) (m' := HList.cons 1 (Lags.const κs 0)) ⟨hle.1, Lags.Le.refl _⟩).1
      rw [hhead] at h2
      rcases le_max_iff.mp h1 with h3 | h3
      · exact (h3.trans h2).trans (Nat.zero_le _)
      · exact h3
    have hm : Lags.Le m (LagFun.next Φ₀ m) :=
      hpost.2.trans (h.mono (m := HList.cons a m) (m' := HList.cons (Φ₀ (HList.cons 0 m)).1 m) ⟨ha, Lags.Le.refl _⟩).2
    have h2 := postfix_zero κs (LagFun.next Φ₀) h.next hv' m hm hle.2
    subst h2
    rw [cons_const_zero, h.zero] at ha
    have ha0 : a = 0 := Nat.le_zero.mp ha
    subst ha0
    rfl

/-- **The Jacobi iteration from "every knot unknown" reaches "every knot known"** in as many
rounds as there are knots: each round either strictly shrinks the unknown set or has
reached a post-fixpoint, which by `postfix_zero` is `0`. -/
theorem iter_zero_aux (κs : List (Kind L mem)) (Φ₀ : Lags κs → Lags κs) (h : LagLaws Φ₀ Φ₀)
    (hv : cascadeValid κs Φ₀ = true) : ∀ (i : Nat) (m : Lags κs), Lags.Le (Φ₀ m) m → Lags.Le m (Lags.const κs 1) →
    Lags.sum m ≤ i → knotIter Φ₀ i m = Lags.const κs 0
  | 0, m, _, _, hs => Lags.eq_const_zero_of_sum m (Nat.le_zero.mp hs)
  | i + 1, m, hpre, hle, hs => by
    show knotIter Φ₀ i (Φ₀ m) = _
    by_cases heq : Φ₀ m = m
    · have hm := postfix_zero κs Φ₀ h hv m (by rw [heq]; exact Lags.Le.refl _) hle
      rw [hm, h.zero]
      exact knotIter_fixed h.zero i
    · exact iter_zero_aux κs Φ₀ h hv i (Φ₀ m) (h.mono hpre) (hpre.trans hle)
        (by have := Lags.sum_lt hpre heq; omega)

theorem iter_zero (κs : List (Kind L mem)) (Φ₀ : Lags κs → Lags κs) (h : LagLaws Φ₀ Φ₀)
    (hv : cascadeValid κs Φ₀ = true) : knotIter Φ₀ κs.length (Lags.const κs 1) = Lags.const κs 0 :=
  iter_zero_aux κs Φ₀ h hv κs.length (Lags.const κs 1) (h.le_const 1) (Lags.Le.refl _)
    (by rw [Lags.sum_const_one])

end Cascade

end Project

end Hydro
