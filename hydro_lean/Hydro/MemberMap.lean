import Hydro.SimpAttr

/-!
# `MemberMap` — the member-indexed carrier of a cluster-located collection

At the denotation a cluster-located collection has one cell per member of
the cluster: mathematically a function `Fin n → C`. `MemberMap n C` is
that function in a structure, for one reason — **evaluation sharing**.

Lean's compiler eta-expands every declaration to the arity of its *type*
(`Lean.Compiler.LCNF.ToDecl`, `Decl.etaExpand`), so a value of function
type can never be pre-forced: a `let v := Vector.ofFn f; fun i => v.get i`
has its `let` pushed inside the lambda and recomputed per call. Member-
indexed carriers as bare functions are therefore call-by-name — every
read re-runs the producing chain, compounding through module composition
and knot iteration (FINDINGS D14: hours for `paxos_core`; D73: one output
leg of the `leader_election` witness at one member read its `p2b` input
4,775,952 times). A *structure* return type is not eta-expanded, so a
constructor can force the member cells once at construction.

The two constructors are the seam:

- `MemberMap.mk'` is what the kernel sees — `⟨f, ()⟩`, `@[reducible]`, so
  `(mk' f) i` is `f i` by projection-of-constructor and β (`rfl`; the
  `den` simp set stays definitional);
- `MemberMap.mkImpl` is what the compiler runs — `Vector.ofFn f` forced
  once, cells read by index;
- `MemberMap.mkImpl_eq` proves them equal, once, for every program: the
  executed data IS the denotation (what `Eager`'s per-carrier `agree`
  field and per-program `*_eager_den` pins used to say — D73 retires
  that interpretation).
-/

namespace Hydro

/-- `Vector.ofFn` reads back: the materialization is the function. -/
theorem vget_ofFn {n : Nat} {α : Type _} (f : Fin n → α) (i : Fin n) :
    (Vector.ofFn f).get i = f i := by
  rcases i with ⟨iv, hi⟩
  exact Vector.getElem_ofFn hi

/-- One cell per cluster member: a cluster-located value at the
denotation. Reads are `m i` (`CoeFun`) or `m.get i`.

`boxed` is the representation pin. A structure with exactly one relevant
field is erased by the compiler to that field (`hasTrivialStructure?`),
so with `cells` alone a member map would be a bare function at runtime
again, and the mono-phase LCNF passes would treat it as one — local
lambdas returning it are eta-expanded (`Simp`: `isEtaExpandCandidate` on
the erased type; D73 saw `fix_stream` become `fun fuel body i => iterate
body seed fuel i`, the whole Kleene iteration re-run per read) and a
let-bound map in head position is merged into its read as an
over-application (`simpAppApp?`: `let s := mkImpl n g; s i ⟹ mkImpl n g i`,
inside the downstream op's cell lambda). Both destroy the forcing `mkImpl`
exists for, exponentially in composition depth (D73: `leader_election`
alone, 1→4 proposers, 852 → 14k → 344k → 3.9M map constructions). With a
second relevant field the map is a heap object: no arrow type to expand,
reads are a projection then an application, and `let`s stay where the
program put them. `Unit` is the cheapest such field (one pointer). -/
structure MemberMap (n : Nat) (C : Type u) where
  /-- The cells, as a function. -/
  cells : Fin n → C
  /-- Representation pin (see above); always `()`. -/
  boxed : Unit := ()

namespace MemberMap

variable {n : Nat} {C : Type u}

/-- The read `m.get i`. A `def` (regular definitional-height hints), not
the projection `cells` itself: the unifier's same-head heuristic
(`isDefEqArgs` before unfolding) is skipped for projection functions,
whose `abbrev` hints send `get A i =?= get A' i` straight to `.proj`
nodes — and a stuck projection makes `isDefEq` `whnf` its structure
argument WITH delta, i.e. the whole program upstream of the wire
(FINDINGS D56's projection cliff; D73 measured it at 34M heartbeats for
one `exact` in a knot's `wf`). Through `get`, two reads of the same wire
compare their carriers structurally first. Reducible, so `(mk' f) i`
is `f i` by `whnfR`. -/
@[reducible] def get (m : MemberMap n C) (i : Fin n) : C := m.cells i

instance : CoeFun (MemberMap n C) (fun _ => Fin n → C) := ⟨MemberMap.get⟩

/-- The compiled constructor: force every member's cell once. The return
type is a structure, so the compiler does not eta-expand this
declaration (`isEtaExpandCandidateCore`: type arity = parameter count)
and the `let` is evaluated at construction, not per read. -/
def mkImpl (f : Fin n → C) : MemberMap n C :=
  let v := Vector.ofFn f
  ⟨fun i => v.get i, ()⟩

/-- The denotational constructor: the function itself. `@[reducible]` so
every read `(mk' f) i` is `f i` definitionally. Compiled as `mkImpl`
through `mk'_eq_mkImpl` below. -/
@[reducible]
def mk' (f : Fin n → C) : MemberMap n C := ⟨f, ()⟩

/-- The compiled constructor IS the denotational one. -/
theorem mkImpl_eq (f : Fin n → C) : mkImpl f = mk' f := by
  show (⟨fun i => (Vector.ofFn f).get i, ()⟩ : MemberMap n C) = ⟨f, ()⟩
  congr 1
  funext i
  exact vget_ofFn f i

/-- The single executable-witness seam, **checked by Lean**: `@[csimp]`
lets the compiler replace `mk'` by `mkImpl` only through this
kernel-checked equation (the mechanism core uses for `List.map` →
`List.mapTR`). There is no `implemented_by` anywhere in `Hydro/` —
nothing unverified stands between a `Values` program and its executable.
Delete this theorem and the executables merely run the logical `mk'`
(slowly; the gate's timing envelope catches it) — the denotation is
unaffected. -/
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

/-- Reads at the denotation are the function's: the coercion is `get`. -/
theorem coe_get (m : MemberMap n C) : (m : Fin n → C) = m.get := rfl
theorem get_eq_cells (m : MemberMap n C) (i : Fin n) : m.get i = m.cells i := rfl

end MemberMap

end Hydro

namespace Hydro.MemberMap

/-- Pointwise consequence of a carrier equality (the generator's
`congrFun` at a member). -/
theorem get_congr {n : Nat} {C : Type u} {a b : MemberMap n C} (h : a = b) (i : Fin n) :
    a i = b i := h ▸ rfl

end Hydro.MemberMap
