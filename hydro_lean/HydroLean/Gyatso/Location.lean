import HydroLean.Flo.Graph

/-!
# Gyatso locations (dissertation §3.3)

Locations are the choreographic layer of Gyatso: every stream is materialized
at a `Loc` (a single `process` or a scale-out `cluster`, §3.3.1), and the type
system tracks placement so that network boundaries are explicit. Crucially,
locations are *erased from the operational semantics* — placement is a typing
judgment over the same Flo graphs (§3.3.2), exactly like the boundedness layer
`Graph.WT`.

The judgment `Graph.LWT` is parameterized by a *leaf signature* `sig` declaring
which port-location assignments each operator supports:
- computational (local) operators are upgraded from Flo with all ports at one
  location (`LocSig.local_`, the paper's upgrade-process rule, Fig 3.3);
- network operators declare an input location and a distinct-or-equal output
  location (the paper's network operator types, §3.5).

Because `LWT` never appears in `Graph.Step`, the semantics of a located program
is literally the Flo semantics — Gyatso preserves all Flo guarantees by
construction (§3.4.1: "we can leverage many of the existing proofs in Flo
as-is"). We also prove that location typing is preserved by execution
(`LWT.preserved`), the formal content of "projection stays valid as the system
runs".
-/

namespace HydroLean

universe u

namespace Gyatso

/-- A distributed location (§3.3.1): a single process or a cluster of machines,
identified by a tag. A process and a cluster with the same tag are unrelated
locations (§3.3.3). -/
inductive Loc where
  | process (tag : String)
  | cluster (tag : String)
deriving DecidableEq, Repr

/-- A located stream type (Fig 3.1): collection, boundedness, and location. -/
structure LocStreamType : Type (u + 1) where
  coll : Coll.{u}
  bound : Boundedness
  loc : Loc

/-- Located subtyping (Fig 3.1): same collection and location, boundedness
weakens (`(C, B, L) ≤ (C, U, L)`). -/
def LocStreamType.le (s₁ s₂ : LocStreamType.{u}) : Prop :=
  s₁.coll = s₂.coll ∧ s₁.bound ≤ s₂.bound ∧ s₁.loc = s₂.loc

/-- A leaf signature: which port-location assignments an operator admits.
This is the parameter of the located typing judgment; it plays the role of the
paper's per-operator location typing rules (`⊢_O` in §3.3.2 and the network
types of §3.5). -/
def LocSig : Type (u + 1) :=
  ∀ {ins outs : List Coll.{u}}, Operator ins outs → List Loc → List Loc → Prop

/-- The location assignment for a *local* (computational) operator: every input
and output port at the same location (the upgrade rules of Fig 3.3 for
processes and Fig 3.5 for clusters — with clusters, the operator itself is the
multibuffer-upgraded `clusterOp`, and this assignment places all its ports at
the cluster location). -/
def LocSig.local_ {ins outs : List Coll.{u}} (loc : Loc) (il ol : List Loc) : Prop :=
  il = List.replicate ins.length loc ∧ ol = List.replicate outs.length loc

end Gyatso

namespace Graph

open Gyatso

/-- The located typing judgment (§3.3.2, Fig 3.2): assigns a location to every
input and output port of a graph, given a leaf signature `sig`. Sequential
composition requires the intermediate locations to agree on both sides (data
does not silently move between machines: only network operators, licensed by
`sig`, may bridge locations). -/
inductive LWT (sig : Gyatso.LocSig.{u}) : {i o : List Coll.{u}} → Graph i o →
    List Gyatso.Loc → List Gyatso.Loc → Prop where
  | node {ins outs : List Coll.{u}} {op : Operator ins outs}
      {st : op.State} {buf : Vals ins} {il ol : List Gyatso.Loc} :
      sig op il ol →
      LWT sig (.node op st buf) il ol
  | seq {i m o : List Coll.{u}} {g₁ : Graph i m} {g₂ : Graph m o}
      {il ml ol : List Gyatso.Loc} :
      LWT sig g₁ il ml → LWT sig g₂ ml ol →
      LWT sig (.seq g₁ g₂) il ol
  | par {i₁ o₁ i₂ o₂ : List Coll.{u}} {g₁ : Graph i₁ o₁} {g₂ : Graph i₂ o₂}
      {il₁ ol₁ il₂ ol₂ : List Gyatso.Loc} :
      LWT sig g₁ il₁ ol₁ → LWT sig g₂ il₂ ol₂ →
      LWT sig (.par g₁ g₂) (il₁ ++ il₂) (ol₁ ++ ol₂)

/-- Location typing ignores buffered inputs, so it is stable under `setInput`. -/
theorem LWT.setInput {sig : Gyatso.LocSig.{u}}
    {i o : List Coll.{u}} {g : Graph i o} {il ol : List Gyatso.Loc}
    (h : LWT sig g il ol) : ∀ I : Vals i, LWT sig (g.setInput I) il ol := by
  induction h with
  | node hsig => exact fun _ => .node hsig
  | seq h₁ h₂ ih₁ _ =>
    intro I
    show LWT sig (.seq (Graph.setInput _ I) _) _ _
    exact .seq (ih₁ I) h₂
  | par h₁ h₂ ih₁ ih₂ =>
    intro I
    show LWT sig (.par (Graph.setInput _ _) (Graph.setInput _ _)) _ _
    exact .par (ih₁ _) (ih₂ _)

/-- Inversion shape for `LWT`, keyed on the graph constructor. Direct `cases`
on an `LWT` derivation over a `.par` graph fails (the `i₁ ++ i₂` index is not
invertible), so we compute the inversion by recursion on the derivation into
this graph-directed motive — the standard shape-function technique. -/
def LWTShape (sig : Gyatso.LocSig.{u}) :
    {i o : List Coll.{u}} → Graph i o → List Gyatso.Loc → List Gyatso.Loc → Prop
  | _, _, .node op _ _, il, ol => sig op il ol
  | _, _, .seq g₁ g₂, il, ol => ∃ ml, LWT sig g₁ il ml ∧ LWT sig g₂ ml ol
  | _, _, .par g₁ g₂, il, ol =>
    ∃ il₁ ol₁ il₂ ol₂, il = il₁ ++ il₂ ∧ ol = ol₁ ++ ol₂ ∧
      LWT sig g₁ il₁ ol₁ ∧ LWT sig g₂ il₂ ol₂

theorem LWT.shape {sig : Gyatso.LocSig.{u}} {i o : List Coll.{u}}
    {g : Graph i o} {il ol : List Gyatso.Loc}
    (h : LWT sig g il ol) : LWTShape sig g il ol := by
  induction h with
  | node hsig => exact hsig
  | seq h₁ h₂ => exact ⟨_, h₁, h₂⟩
  | par h₁ h₂ => exact ⟨_, _, _, _, rfl, rfl, h₁, h₂⟩

/-- **Location preservation**: execution never changes the location assignment
of a graph — the choreographic projection determined at compile time stays
valid throughout the run. (Locations are erased from `Step`, so this is the
formal statement that placement is purely static, §3.3.2.) -/
theorem LWT.preserved {sig : Gyatso.LocSig.{u}}
    {i o : List Coll.{u}} {g g' : Graph i o} {δ : Vals o}
    {il ol : List Gyatso.Loc}
    (hs : Step g g' δ) (h : LWT sig g il ol) : LWT sig g' il ol := by
  induction hs generalizing il ol with
  | node _ =>
    have hsig := h.shape
    exact .node hsig
  | seqLeft _ ih =>
    obtain ⟨ml, h₁, h₂⟩ := h.shape
    exact .seq (ih h₁) (h₂.setInput _)
  | seqRight _ ih =>
    obtain ⟨ml, h₁, h₂⟩ := h.shape
    exact .seq h₁ (ih h₂)
  | parLeft _ ih =>
    obtain ⟨il₁, ol₁, il₂, ol₂, hil, hol, h₁, h₂⟩ := h.shape
    subst hil; subst hol
    exact .par (ih h₁) h₂
  | parRight _ ih =>
    obtain ⟨il₁, ol₁, il₂, ol₂, hil, hol, h₁, h₂⟩ := h.shape
    subst hil; subst hol
    exact .par h₁ (ih h₂)

end Graph

end HydroLean
