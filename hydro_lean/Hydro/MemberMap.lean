import Hydro.SimpAttr

/-!
# `MemberMap` — the member-indexed carrier of a cluster-located collection

At the denotation a cluster-located collection has one cell per member of the cluster:
mathematically a function `Fin n → C`. `MemberMap n C` wraps that function in a structure
for one reason, **evaluation sharing**.

Lean's compiler eta-expands every declaration to the arity of its type
(`Lean.Compiler.LCNF.ToDecl`, `Decl.etaExpand`), so a value of function type is never
pre-forced: in `let v := Vector.ofFn f; fun i => v.get i` the `let` is pushed inside the
lambda and recomputed per call. A member-indexed carrier as a bare function is therefore
call-by-name, and every read re-runs the producing chain, compounding through module
composition and knot iteration. A structure return type is not eta-expanded, so a
constructor can force the member cells once at construction.

The seam between kernel and compiler consists of two constructors:

- `MemberMap.mk'` is what the kernel sees: `⟨f, ()⟩`, `@[reducible]`, so `(mk' f) i` is
  `f i` by projection-of-constructor and β (`rfl`; the `den` simp set stays definitional);
- `MemberMap.mkImpl` is what the compiler runs: `Vector.ofFn f` forced once, cells read by
  index;
- `MemberMap.mkImpl_eq` proves them equal, once, for every program, so the executed data is
  the denotation.
-/

namespace Hydro

/-- `Vector.ofFn` reads back: reading the materialization returns the function. -/
theorem vget_ofFn {n : Nat} {α : Type _} (f : Fin n → α) (i : Fin n) :
    (Vector.ofFn f).get i = f i := by
  rcases i with ⟨iv, hi⟩
  exact Vector.getElem_ofFn hi

/-- One cell per cluster member: a cluster-located value at the denotation. Reads are `m i`
(`CoeFun`) or `m.get i`.

`boxed` pins the representation. The compiler erases a structure with exactly one
relevant field to that field (`hasTrivialStructure?`), so with `cells` alone a member map
would be a bare function at runtime again and the mono-phase LCNF passes would treat it as
one: local lambdas returning it are eta-expanded (`Simp`: `isEtaExpandCandidate` on the
erased type, which turns a `fix_stream` into `fun fuel body i => iterate body seed fuel i`
and re-runs the whole Kleene iteration per read), and a let-bound map in head position is
merged into its read as an over-application (`simpAppApp?`: `let s := mkImpl n g; s i ⟹
mkImpl n g i`, inside the downstream operator's cell lambda). Both destroy the forcing
`mkImpl` exists for, exponentially in composition depth. With a second relevant field the
map is a heap object: there is no arrow type to expand, a read is a projection then an
application, and `let`s stay where the program put them. `Unit` is the cheapest such field
(one pointer). -/
structure MemberMap (n : Nat) (C : Type u) where
  /-- The cells, as a function. -/
  cells : Fin n → C
  /-- Representation pin (see above); always `()`. -/
  boxed : Unit := ()

namespace MemberMap

variable {n : Nat} {C : Type u}

/-- The read `m.get i`. A `def` with regular definitional-height hints, not the projection
`cells` itself: the unifier's same-head heuristic (`isDefEqArgs` before unfolding) is skipped
for projection functions, whose `abbrev` hints send `get A i =?= get A' i` straight to
`.proj` nodes, and a stuck projection makes `isDefEq` `whnf` its structure argument with
delta — the whole program upstream of the wire. Through `get`, two reads of the same wire
compare their carriers structurally first. Reducible, so `(mk' f) i` is `f i` by `whnfR`. -/
@[reducible] def get (m : MemberMap n C) (i : Fin n) : C := m.cells i

instance : CoeFun (MemberMap n C) (fun _ => Fin n → C) := ⟨MemberMap.get⟩

/-- The compiled constructor: force every member's cell once. The return type is a
structure, so the compiler does not eta-expand this declaration (`isEtaExpandCandidateCore`:
type arity = parameter count) and the `let` is evaluated at construction, not per read. -/
def mkImpl (f : Fin n → C) : MemberMap n C :=
  let v := Vector.ofFn f
  ⟨fun i => v.get i, ()⟩

/-- The denotational constructor: the function itself. `@[reducible]`, so every read
`(mk' f) i` is `f i` definitionally. Compiled as `mkImpl` through `mk'_eq_mkImpl` below. -/
@[reducible]
def mk' (f : Fin n → C) : MemberMap n C := ⟨f, ()⟩

/-- The compiled constructor `mkImpl` equals the denotational one `mk'`. -/
theorem mkImpl_eq (f : Fin n → C) : mkImpl f = mk' f := by
  show (⟨fun i => (Vector.ofFn f).get i, ()⟩ : MemberMap n C) = ⟨f, ()⟩
  congr 1
  funext i
  exact vget_ofFn f i

/-- The executable-witness seam, checked by Lean: `@[csimp]` lets the compiler replace `mk'`
by `mkImpl` only through this kernel-checked equation (the mechanism core uses for
`List.map` → `List.mapTR`). Every compiled/logical split in `Hydro/` is such a `@[csimp]`
theorem; there is no `implemented_by`, `@[extern …]`, `native_decide` or `unsafe def` in
`Hydro/`, so nothing unverified stands between a `Values` program and its executable.
Without this theorem the executables run the logical `mk'` (slowly; the gate's timing
envelope catches it) and the denotation is unchanged. -/
@[csimp] theorem mk'_eq_mkImpl : @mk' = @mkImpl := by
  funext n C f
  exact (mkImpl_eq f).symm

@[ext] theorem ext {a b : MemberMap n C} (h : ∀ i, a i = b i) : a = b := by
  cases a; cases b
  congr 1
  exact funext h

@[simp] theorem cells_eq_get (m : MemberMap n C) : m.cells = m.get := rfl

theorem ext_iff' {a b : MemberMap n C} : a = b ↔ ∀ i, a i = b i :=
  ⟨fun h _ => h ▸ rfl, ext⟩

@[simp, den] theorem get_mk' (f : Fin n → C) (i : Fin n) : (mk' f) i = f i := rfl
@[simp, den] theorem get_mk (f : Fin n → C) (i : Fin n) : (⟨f, ()⟩ : MemberMap n C) i = f i := rfl
@[simp] theorem mk'_get (m : MemberMap n C) : mk' (fun i => m i) = m := rfl

instance [Inhabited C] : Inhabited (MemberMap n C) := ⟨mk' fun _ => default⟩

/-- The coercion is `get`: reads at the denotation are the function's. -/
theorem coe_get (m : MemberMap n C) : (m : Fin n → C) = m.get := rfl
theorem get_eq_cells (m : MemberMap n C) (i : Fin n) : m.get i = m.cells i := rfl

end MemberMap

end Hydro

namespace Hydro.MemberMap

/-- Pointwise consequence of a carrier equality: the generator's `congrFun` at a member. -/
theorem get_congr {n : Nat} {C : Type u} {a b : MemberMap n C} (h : a = b) (i : Fin n) :
    a i = b i := h ▸ rfl

end Hydro.MemberMap
