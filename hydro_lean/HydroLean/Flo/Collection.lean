import HydroLean.Prelude

/-!
# Flo collection languages (dissertation §2.3.2)

A collection language `L_C = (C, ++, E_C, T_C, ⟦⟧, ⌊⌋, type_C, fix)` provides the
data domain that streams evolve over. Per DESIGN.md we collapse collection
*expressions* into collection *values* (the paper's well-formedness laws
`⟦e⟧ ∈ type e` and `⌊⟦e⟧⌋ = e` make the syntactic layer isomorphic to the value
layer wherever the semantics consults it), and we replace the set-of-values
"collection types" `T_C` with the carrier type itself: refinements (e.g. dup-free
adjacency for `[T]dup`) are expressed as subtypes or quotients of the carrier.

What remains is the algebraic core: a carrier `C`, a concatenation `++` (with
right identity `∅`), the `fixed` predicate (`fixed c ↔ ∀ δ, c ++ δ = c`, §2.3.2),
and the `fix` transformation producing an equivalent-but-fixed value.

Stream types (§2.3.3) layer a boundedness flag on top: `Bounded` streams are
guaranteed to eventually become fixed, `Unbounded` streams may never be. The
subtyping `(C, B) ≤ (C, U)` (Fig 2.2) is reflected in `Boundedness.le`.
-/

namespace HydroLean

universe u

/-- A collection language: the algebraic core of the paper's `L_C` (§2.3.2).

Fields correspond to: carrier `C`, concatenation `++`, right identity `∅`
(`c ++ ∅ = c`), and `fix : C → C` mapping a value to an equivalent value that is
fixed (`∀ c, fixed (fix c)`). -/
structure Coll : Type (u + 1) where
  /-- The type of collection values. -/
  C : Type u
  /-- Concatenation `++`: how new data (the right argument, a "delta") is added. -/
  concat : C → C → C
  /-- The empty delta, a right identity for `concat`. -/
  empty : C
  /-- Transformation into an equivalent value that is fixed (e.g. appending a
  stream terminator `⊗`). -/
  fix : C → C
  /-- `c ++ ∅ = c` (§2.3.2 well-formedness). -/
  concat_empty : ∀ c, concat c empty = c
  /-- `fixed (fix c)` (§2.3.2 well-formedness): no delta affects a fixed value. -/
  fix_fixed : ∀ c δ, concat (fix c) δ = fix c

namespace Coll

variable (L : Coll.{u})

/-- `fixed c ≜ ∀ c', c ++ c' = c` (§2.3.2): no more data can be added. -/
def Fixed (c : L.C) : Prop := ∀ δ, L.concat c δ = c

theorem fixed_fix (c : L.C) : L.Fixed (L.fix c) := L.fix_fixed c

end Coll

/-- Boundedness flag on stream types (§2.3.3): `bounded` streams eventually
become fixed; `unbounded` streams may never. -/
inductive Boundedness where
  | bounded
  | unbounded
deriving DecidableEq, Repr

namespace Boundedness

/-- Subtyping on boundedness (Fig 2.2): `bounded ≤ unbounded` (a stream known to
terminate can be used where termination is not assumed), plus reflexivity. -/
def le : Boundedness → Boundedness → Prop
  | .bounded, _ => True
  | .unbounded, b => b = .unbounded

instance : LE Boundedness := ⟨le⟩

theorem le_refl (b : Boundedness) : b ≤ b := by cases b <;> trivial

theorem le_trans {a b c : Boundedness} (h₁ : a ≤ b) (h₂ : b ≤ c) : a ≤ c := by
  cases a <;> cases b <;> cases c <;> simp_all [LE.le, le]

theorem bounded_le (b : Boundedness) : Boundedness.bounded ≤ b := trivial

end Boundedness

/-- A stream type (§2.3.3): a collection together with a boundedness flag. -/
structure StreamType : Type (u + 1) where
  coll : Coll.{u}
  bound : Boundedness

/-- Stream subtyping (Fig 2.2): same collection, boundedness weakens. -/
def StreamType.le (s₁ s₂ : StreamType.{u}) : Prop :=
  s₁.coll = s₂.coll ∧ s₁.bound ≤ s₂.bound

/-! ## Tuples of ports

Operators and graphs have several input and output ports (the paper's `[C]`).
We model a family of ports as a `List Coll` and a tuple of values as a
heterogeneous list `Vals`. -/

/-- A heterogeneous tuple of collection values, one per port. -/
inductive Vals : List Coll.{u} → Type (u + 1) where
  | nil : Vals []
  | cons {L : Coll.{u}} {ls : List Coll.{u}} : L.C → Vals ls → Vals (L :: ls)

namespace Vals

/-- Pointwise concatenation of a delta tuple (the lifting of `++` to `[C]`). -/
def concat : {ls : List Coll.{u}} → Vals ls → Vals ls → Vals ls
  | [], .nil, .nil => .nil
  | _ :: _, .cons x xs, .cons y ys => .cons (Coll.concat _ x y) (concat xs ys)

/-- The tuple of empty collections. -/
def empty : (ls : List Coll.{u}) → Vals ls
  | [] => .nil
  | L :: ls => .cons L.empty (empty ls)

/-- Pointwise `fix`. -/
def fixAll : {ls : List Coll.{u}} → Vals ls → Vals ls
  | [], .nil => .nil
  | _ :: _, .cons x xs => .cons (Coll.fix _ x) (fixAll xs)

/-- All components of a tuple are fixed. -/
inductive AllFixed : {ls : List Coll.{u}} → Vals ls → Prop where
  | nil : AllFixed .nil
  | cons {L : Coll.{u}} {ls : List Coll.{u}} {x : L.C} {xs : Vals ls} :
      L.Fixed x → AllFixed xs → AllFixed (.cons x xs)

/-- Concatenating the empty tuple is the identity (lifted `concat_empty`). -/
theorem concat_empty : ∀ {ls : List Coll.{u}} (v : Vals ls), v.concat (empty ls) = v
  | [], .nil => rfl
  | _ :: _, .cons x xs => by simp [concat, empty, Coll.concat_empty, concat_empty xs]

/-- `fixAll` produces a tuple in which every component is fixed. -/
theorem allFixed_fixAll : ∀ {ls : List Coll.{u}} (v : Vals ls), AllFixed v.fixAll
  | [], .nil => .nil
  | _ :: _, .cons x xs => .cons (Coll.fixed_fix _ x) (allFixed_fixAll xs)

/-- Fixed tuples absorb all deltas. -/
theorem concat_of_allFixed :
    ∀ {ls : List Coll.{u}} {v : Vals ls}, AllFixed v → ∀ δ, v.concat δ = v
  | _, _, .nil, .nil => rfl
  | _, _, .cons hx hxs, .cons y ys => by
    simp [concat, hx y, concat_of_allFixed hxs ys]

/-- Append two tuples (for parallel composition `e₁ | e₂`). -/
def append : {as bs : List Coll.{u}} → Vals as → Vals bs → Vals (as ++ bs)
  | [], _, .nil, w => w
  | _ :: _, _, .cons x xs, w => .cons x (append xs w)

/-- Split a tuple over an appended port list into its two halves. -/
def split : {as bs : List Coll.{u}} → Vals (as ++ bs) → Vals as × Vals bs
  | [], _, w => (.nil, w)
  | _ :: _, _, .cons x xs =>
    let (l, r) := split xs
    (.cons x l, r)

@[simp] theorem split_append :
    ∀ {as bs : List Coll.{u}} (v : Vals as) (w : Vals bs), split (append v w) = (v, w)
  | [], _, .nil, _ => rfl
  | _ :: _, _, .cons x xs, w => by simp [append, split, split_append xs w]

/-- `append` is a section of `split`. -/
@[simp] theorem append_split :
    ∀ {as bs : List Coll.{u}} (v : Vals (as ++ bs)), append (v.split.1) (v.split.2) = v
  | [], _, _ => rfl
  | _ :: as, bs, .cons x xs => by simp [split, append, append_split xs]

/-- `split` commutes with pointwise concatenation. -/
theorem split_concat :
    ∀ {as bs : List Coll.{u}} (v δ : Vals (as ++ bs)),
      (v.concat δ).split =
        ((v.split.1.concat δ.split.1 : Vals as), (v.split.2.concat δ.split.2 : Vals bs))
  | [], _, v, δ => rfl
  | _ :: as, bs, .cons x xs, .cons y ys => by
    simp [split, concat, split_concat xs ys]

/-- Appending distributes over pointwise concatenation. -/
theorem append_concat :
    ∀ {as bs : List Coll.{u}} (v₁ v₂ : Vals as) (w₁ w₂ : Vals bs),
      (v₁.concat v₂).append (w₁.concat w₂) = (v₁.append w₁).concat (v₂.append w₂)
  | [], _, .nil, .nil, _, _ => rfl
  | _ :: _, _, .cons x xs, .cons y ys, w₁, w₂ => by
    simp [append, concat, append_concat xs ys w₁ w₂]

@[simp] theorem empty_append :
    ∀ (as bs : List Coll.{u}), (empty as).append (empty bs) = empty (as ++ bs)
  | [], _ => rfl
  | _ :: as, bs => by simp [append, empty, empty_append as bs]

/-- `FixedWhere v bs`: every component of `v` whose corresponding flag in `bs`
is `bounded` is a fixed collection value. Used to state streaming progress
(Def 2.3.3): "the bounded inputs are fixed". Missing flags (length mismatch)
impose no constraint. -/
def FixedWhere : {ls : List Coll.{u}} → Vals ls → List Boundedness → Prop
  | [], .nil, _ => True
  | _ :: _, .cons _ _, [] => True
  | _ :: _, .cons x xs, b :: bs =>
    (b = .bounded → Coll.Fixed _ x) ∧ FixedWhere xs bs

theorem fixedWhere_of_allFixed :
    ∀ {ls : List Coll.{u}} {v : Vals ls}, AllFixed v →
      ∀ bs, FixedWhere v bs
  | _, _, .nil => fun bs => by cases bs <;> trivial
  | _, _, .cons hx hxs => fun bs => by
    cases bs with
    | nil => trivial
    | cons b bs => exact ⟨fun _ => hx, fixedWhere_of_allFixed hxs bs⟩

/-- `FixedWhere` splits across appended tuples when the flag list splits
compatibly (lengths matching the left tuple). -/
theorem fixedWhere_append :
    ∀ {as bs : List Coll.{u}} (v : Vals as) (w : Vals bs)
      {fa fb : List Boundedness}, fa.length = as.length →
      FixedWhere v fa → FixedWhere w fb → FixedWhere (v.append w) (fa ++ fb)
  | [], _, .nil, _, [], _, _, _, hw => hw
  | _ :: _, _, .cons _ xs, w, _ :: _, _, hlen, hv, hw =>
    ⟨hv.1, fixedWhere_append xs w (Nat.succ.inj hlen) hv.2 hw⟩

end Vals

/-- Pointwise subtyping on boundedness flag lists (used at sequential
composition: the producer's output stream types must be subtypes of the
consumer's input stream types, Fig 2.5). -/
def Bounds.le : List Boundedness → List Boundedness → Prop
  | [], [] => True
  | a :: as, b :: bs => a ≤ b ∧ Bounds.le as bs
  | _, _ => False

end HydroLean
