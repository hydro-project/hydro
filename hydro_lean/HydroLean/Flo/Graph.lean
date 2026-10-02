import HydroLean.Flo.Operator

/-!
# Flo graphs (dissertation §2.4)

Graphs compose operators by sequential (`e₁ ; e₂`) and parallel (`e₁ | e₂`)
composition (Fig 2.4). A leaf `{S}[op]` is an operator together with its state
and its *buffered inputs* `S`. Graphs are intrinsically typed by their input
and output port collections; the boundedness layer is a separate typing
judgment `Graph.WT` (Fig 2.5), so the operational semantics (Fig 2.6) is
independent of stream types, exactly as in the paper.

The small-step `Graph.Step g g' δ` emits an output delta `δ` (the paper's
`→Δ`); `Graph.CStep` is the induced relation on configurations `(g, O)` that
concatenates the delta onto the output buffers (the paper's `→`).

The key definitions of §2.4.2 — Determinism (Def 2.4.1) and Eager Execution
(Def 2.4.2) — are stated here; their proofs (Lemma 2.4.2, 2.4.3, 2.4.4) live in
`Flo/Theorems.lean`.
-/

namespace HydroLean

universe u

/-- Flo dataflow graphs (Fig 2.4): `e ::= {S}[op] | e;e | e|e`, intrinsically
typed by input/output port collections. Leaves carry the operator state and the
buffered input collections `S`. -/
inductive Graph : List Coll.{u} → List Coll.{u} → Type (u + 1) where
  | node {ins outs : List Coll.{u}} (op : Operator ins outs)
      (st : op.State) (buf : Vals ins) : Graph ins outs
  | seq {i m o : List Coll.{u}} : Graph i m → Graph m o → Graph i o
  | par {i₁ o₁ i₂ o₂ : List Coll.{u}} :
      Graph i₁ o₁ → Graph i₂ o₂ → Graph (i₁ ++ i₂) (o₁ ++ o₂)

namespace Graph

variable {i o : List Coll.{u}}

/-- The buffered inputs of a graph (Fig 2.6, `inputs`). -/
def inputs : {i o : List Coll.{u}} → Graph i o → Vals i
  | _, _, .node _ _ buf => buf
  | _, _, .seq g₁ _ => g₁.inputs
  | _, _, .par g₁ g₂ => g₁.inputs.append g₂.inputs

/-- Replace the buffered inputs of a graph (Fig 2.6, `setinput`). -/
def setInput : {i o : List Coll.{u}} → Graph i o → Vals i → Graph i o
  | _, _, .node op st _, I => .node op st I
  | _, _, .seq g₁ g₂, I => .seq (g₁.setInput I) g₂
  | _, _, .par g₁ g₂, I =>
    let (l, r) := I.split
    .par (g₁.setInput l) (g₂.setInput r)

@[simp] theorem inputs_setInput :
    ∀ {i o : List Coll.{u}} (g : Graph i o) (I : Vals i), (g.setInput I).inputs = I
  | _, _, .node _ _ _, _ => rfl
  | _, _, .seq g₁ _, I => inputs_setInput g₁ I
  | _, _, .par g₁ g₂, I => by
    simp [setInput, inputs, inputs_setInput g₁, inputs_setInput g₂]

/-- Introduce a delta on the graph's inputs: `{I ++ Δ}g` (Def 2.4.2). -/
def addDelta (g : Graph i o) (Δ : Vals i) : Graph i o :=
  g.setInput (g.inputs.concat Δ)

/-- The graph small-step `g →Δ (g', δ)` (Fig 2.6). Sequential composition
forwards the left side's emitted delta into the right side's input buffers and
emits nothing; parallel composition pads the other side's delta with `∅`. -/
inductive Step : {i o : List Coll.{u}} → Graph i o → Graph i o → Vals o → Prop where
  | node {ins outs : List Coll.{u}} {op : Operator ins outs}
      {st st' : op.State} {buf buf' : Vals ins} {δ : Vals outs} :
      op.step buf st buf' st' δ →
      Step (.node op st buf) (.node op st' buf') δ
  | seqLeft {i m o : List Coll.{u}} {g₁ g₁' : Graph i m} {g₂ : Graph m o}
      {δmid : Vals m} :
      Step g₁ g₁' δmid →
      Step (.seq g₁ g₂) (.seq g₁' (g₂.setInput (g₂.inputs.concat δmid))) (Vals.empty o)
  | seqRight {i m o : List Coll.{u}} {g₁ : Graph i m} {g₂ g₂' : Graph m o}
      {δ : Vals o} :
      Step g₂ g₂' δ →
      Step (.seq g₁ g₂) (.seq g₁ g₂') δ
  | parLeft {i₁ o₁ i₂ o₂ : List Coll.{u}} {g₁ g₁' : Graph i₁ o₁} {g₂ : Graph i₂ o₂}
      {δ₁ : Vals o₁} :
      Step g₁ g₁' δ₁ →
      Step (.par g₁ g₂) (.par g₁' g₂) (δ₁.append (Vals.empty o₂))
  | parRight {i₁ o₁ i₂ o₂ : List Coll.{u}} {g₁ : Graph i₁ o₁} {g₂ g₂' : Graph i₂ o₂}
      {δ₂ : Vals o₂} :
      Step g₂ g₂' δ₂ →
      Step (.par g₁ g₂) (.par g₁ g₂') ((Vals.empty o₁).append δ₂)

/-- A graph configuration: the graph (with its internal buffers) and the
accumulated output buffers. -/
structure Config (i o : List Coll.{u}) : Type (u + 1) where
  g : Graph i o
  O : Vals o

/-- The configuration step (the paper's `→`): take a graph step and concatenate
the emitted delta onto the outputs. -/
def CStep : Config i o → Config i o → Prop :=
  fun c c' => ∃ δ, Step c.g c'.g δ ∧ c'.O = c.O.concat δ

/-- Introduce an input delta into a configuration. -/
def Config.addDelta (c : Config i o) (Δ : Vals i) : Config i o :=
  { c with g := c.g.addDelta Δ }

/-- **Def 2.4.1 (Determinism)** for a graph `g`: from any initial outputs, all
reduction sequences out of `(g, O)` are joinable (whence, with Lemma 2.4.2,
every configuration of `g` has a unique stuck state). -/
def Deterministic (g : Graph i o) : Prop :=
  ∀ (O : Vals o) (c₁ c₂ : Config i o),
    Star CStep ⟨g, O⟩ c₁ → Star CStep ⟨g, O⟩ c₂ → Joinable CStep c₁ c₂

/-- **Def 2.4.2 (Eager Execution)** for a graph `g`: introducing an input delta
before or after a step out of `(g, O)` yields joinable configurations. -/
def EagerG (g : Graph i o) : Prop :=
  ∀ (Δ : Vals i) (O : Vals o) (c' : Config i o), CStep ⟨g, O⟩ c' →
    Joinable CStep ((Config.mk g O).addDelta Δ) (c'.addDelta Δ)

/-- All operator leaves of a graph satisfy the Flo operator obligations. This
is the graph-level precondition for determinism and eager execution
(Lemma 2.4.3); the boundedness layer (`WT`) is only needed for streaming
progress (Lemma 2.4.4). -/
def LeavesLawful : {i o : List Coll.{u}} → Graph i o → Prop
  | _, _, .node op _ _ => op.Lawful
  | _, _, .seq g₁ g₂ => g₁.LeavesLawful ∧ g₂.LeavesLawful
  | _, _, .par g₁ g₂ => g₁.LeavesLawful ∧ g₂.LeavesLawful

/-- Configuration consistency for a graph relative to (downstream) output
buffers: every operator leaf satisfies its consistency invariant
(`Operator.Inv`, the analogue of the paper's configuration typing `⊢→`)
relative to its buffered inputs, its state, and the buffer its emissions feed —
for the rightmost operators that is the configuration's output tuple `O`; for
an operator feeding a sequential successor it is the successor's input buffer.

This is the graph-level premise of streaming progress (Lemma 2.4.4): freshly
constructed programs satisfy it (each operator's `Inv` on its initial buffers),
and Def 2.3.3 is only meaningful on such configurations. -/
def Consistent : {i o : List Coll.{u}} → Graph i o → Vals o → Prop
  | _, _, .node op st buf, O => op.Inv buf st O
  | _, _, .seq g₁ g₂, O => Consistent g₁ g₂.inputs ∧ Consistent g₂ O
  | _, _, .par g₁ g₂, O => Consistent g₁ O.split.1 ∧ Consistent g₂ O.split.2

/-- The boundedness typing judgment for graphs (Fig 2.5), assigning per-port
boundedness flags to the graph's inputs and outputs. Sequential composition
requires the producer's output stream types to be subtypes of the consumer's
input stream types (`Bounds.le`, which encodes `(C,B) ≤ (C,U)` pointwise —
collection agreement is already intrinsic). -/
inductive WT : {i o : List Coll.{u}} → Graph i o →
    List Boundedness → List Boundedness → Prop where
  | node {ins outs : List Coll.{u}} (op : Operator ins outs)
      (st : op.State) (buf : Vals ins) :
      op.Lawful →
      op.inBounds.length = ins.length →
      op.outBounds.length = outs.length →
      WT (.node op st buf) op.inBounds op.outBounds
  | seq {i m o : List Coll.{u}} {g₁ : Graph i m} {g₂ : Graph m o}
      {ib mb₁ mb₂ ob : List Boundedness} :
      WT g₁ ib mb₁ → WT g₂ mb₂ ob → Bounds.le mb₁ mb₂ →
      WT (.seq g₁ g₂) ib ob
  | par {i₁ o₁ i₂ o₂ : List Coll.{u}} {g₁ : Graph i₁ o₁} {g₂ : Graph i₂ o₂}
      {ib₁ ob₁ ib₂ ob₂ : List Boundedness} :
      WT g₁ ib₁ ob₁ → WT g₂ ib₂ ob₂ →
      ib₁.length = i₁.length →
      WT (.par g₁ g₂) (ib₁ ++ ib₂) (ob₁ ++ ob₂)

end Graph

end HydroLean
