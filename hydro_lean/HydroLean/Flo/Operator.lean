import HydroLean.Flo.Collection

/-!
# Flo operators (dissertation §2.3.4–§2.3.6)

An operator language `L_O = (L_C, E_O, →δ, ORD_O, ⊢_O)` gives single-operator
Flo programs. We model an operator intrinsically typed by its input and output
port collections (`ins outs : List Coll`); the operator carries:

- a `State` type (the paper's operator *expressions* `E_O`, which "may carry
  state" — we keep exactly the stateful content);
- the small-step `→δ`: `step I s I' s' δ` relates input tuple and state to
  updated input, state, and an emitted *delta* tuple that will be concatenated
  onto the output buffers;
- boundedness declarations for each port (the operator's stream type).

The paper's derived relation `→_O` acting on configurations `(I, e, O)`
(concatenating the delta to the outputs) is `Operator.OpStep`.

The proof obligations on operators (§2.3.4 well-formedness, Def 2.3.1 eager
execution, Def 2.3.2 output maximality, Def 2.3.3 streaming progress) are
packaged as `Operator.Lawful`. Per DESIGN.md, the paper's "finite and
downwards-closed partial order" `≺` becomes a well-founded relation that steps
decrease — exactly what Lemma 2.3.1 needs.
-/

namespace HydroLean

universe u

/-- A Flo operator with input ports `ins` and output ports `outs`.

The data of the paper's operator language for a single operator: state (operator
expressions), small-step `→δ`, and the stream-type boundedness annotations. -/
structure Operator (ins outs : List Coll.{u}) : Type (u + 1) where
  /-- Internal state: the paper's operator expression `e_o ∈ E_O`. -/
  State : Type u
  /-- The small-step `(I, e) →δ (I', e', O_δ)`: consume from inputs, update
  state, emit a delta for the outputs. -/
  step : Vals ins → State → Vals ins → State → Vals outs → Prop
  /-- Declared boundedness flag for each input port (positional; the paper's
  stream-type annotations `(τ, B|U)` on operator inputs). -/
  inBounds : List Boundedness
  /-- Declared boundedness flag for each output port. -/
  outBounds : List Boundedness
  /-- Value-level consistency invariant on configurations — the analogue of the
  paper's configuration typing `⊢→ (I, e, O) : τ` (§2.3.4), strengthened to
  relate the operator state to the input value and accumulated output history.

  Rationale: `Progress` (Def 2.3.3) is false for *inconsistent* configurations
  that no execution can reach — e.g. a `fold : B ↩→ B` whose input records a
  consumed terminator paired with an output buffer into which the final
  aggregate was never emitted. The paper implicitly assumes reachable
  configurations; we make the assumption explicit and operator-supplied.
  Defaults to trivial (sufficient for `U ↩→ U` operators that keep all
  determinism-relevant history in the input collection value). -/
  Inv : Vals ins → State → Vals outs → Prop := fun _ _ _ => True

namespace Operator

variable {ins outs : List Coll.{u}}

/-- A small-step configuration `(I, e, O)` (§2.3.4). -/
structure Config (op : Operator ins outs) : Type (u + 1) where
  I : Vals ins
  st : op.State
  O : Vals outs

variable {op : Operator ins outs}

/-- The paper's `→_O`: run `→δ` and concatenate the emitted delta onto the
output buffers. -/
def OpStep (op : Operator ins outs) : op.Config → op.Config → Prop :=
  fun c c' => ∃ δ, op.step c.I c.st c'.I c'.st δ ∧ c'.O = c.O.concat δ

/-- Introduce an input delta `Δ` into a configuration: `(I, e, O) ↦ (I ++ Δ, e, O)`.
This is the event-loop action of new data arriving (Fig 2.1). -/
def Config.addDelta (Δ : Vals ins) (c : op.Config) : op.Config :=
  { c with I := c.I.concat Δ }

/-- Fix all inputs of a configuration (used by Output Maximality, Def 2.3.2). -/
def Config.fixInputs (c : op.Config) : op.Config :=
  { c with I := c.I.fixAll }

/-- **Def 2.3.1 (Eager Execution)**, in joinability form: introducing an input
delta before or after a step leads to joinable configurations. Together with
confluence and strong normalization this is equivalent to the paper's
formulation via a common stuck state (see `Theorems.lean` for the derivation),
and inductively extends to deltas interleaved arbitrarily with steps. -/
def Eager (op : Operator ins outs) : Prop :=
  ∀ (Δ : Vals ins) (c c' : op.Config), op.OpStep c c' →
    Joinable op.OpStep (c.addDelta Δ) (c'.addDelta Δ)

/-- **Def 2.3.2 (Output Maximality)** for a stuck state `f` reached from `c`:
running with all inputs fixed from the start reaches a stuck state whose outputs
are exactly `f.O`, componentwise fixed. -/
def OutputsMaximal (op : Operator ins outs) (c f : op.Config) : Prop :=
  ∃ (I'' : Vals ins) (st'' : op.State),
    NormalizesTo op.OpStep c.fixInputs ⟨I'', st'', f.O.fixAll⟩

/-- **Def 2.3.3 (Streaming Progress)**: for any *consistent* configuration
(satisfying the operator's `Inv`) whose bounded inputs are fixed, every
reachable stuck state has maximal outputs, and its bounded outputs are fixed.

The restriction to `Inv`-configurations makes explicit the paper's implicit
assumption that configurations arise from actual executions: without it,
adversarial pairings of operator state and output history (unreachable from any
run) falsify Def 2.3.3 for e.g. `fold : B ↩→ B`. -/
def Progress (op : Operator ins outs) : Prop :=
  ∀ c f : op.Config,
    op.Inv c.I c.st c.O →
    c.I.FixedWhere op.inBounds →
    NormalizesTo op.OpStep c f →
    op.OutputsMaximal c f ∧ f.O.FixedWhere op.outBounds

/-- The complete proof obligations for a Flo operator (§2.3.4 well-formedness
plus Defs 2.3.1–2.3.3). An operator that is `Lawful` may be composed into
graphs while preserving all of Flo's guarantees. -/
structure Lawful (op : Operator ins outs) : Prop where
  /-- §2.3.4: there is a well-founded order that each step decreases on the
  (state, inputs) pair — the paper's "finite, downwards-closed `≺`" — so the
  operator always reaches a stuck state in finitely many steps (Lemma 2.3.1). -/
  wf_decreasing : ∃ r : (op.State × Vals ins) → (op.State × Vals ins) → Prop,
    WellFounded r ∧
      ∀ {I s I' s' δ}, op.step I s I' s' δ → r (s', I') (s, I)
  /-- §2.3.4: `→_O` is confluent. -/
  confluent : Confluent op.OpStep
  /-- Def 2.3.1: eager execution. -/
  eager : op.Eager
  /-- Def 2.3.3: streaming progress. -/
  progress : op.Progress
  /-- The consistency invariant is preserved by steps. -/
  inv_step : ∀ {c c' : op.Config}, op.Inv c.I c.st c.O → op.OpStep c c' →
    op.Inv c'.I c'.st c'.O
  /-- The consistency invariant is preserved by input deltas (new data arriving
  never breaks state/history consistency). -/
  inv_delta : ∀ {I s O} (Δ : Vals ins), op.Inv I s O → op.Inv (I.concat Δ) s O

/-- **Lemma 2.3.1 (Operator Stuck State)**: a lawful operator's step relation is
strongly normalizing — only finitely many steps can be taken from any
configuration. The proof pulls the well-founded order back along the projection
`(I, e, O) ↦ (e, I)`; the output buffer does not influence applicability of
steps (steps only concatenate to it). -/
theorem Lawful.sn {op : Operator ins outs} (h : op.Lawful) : SN op.OpStep := by
  obtain ⟨r, hwf, hdec⟩ := h.wf_decreasing
  have : ∀ c : op.Config, Acc (fun b a => op.OpStep a b) c := by
    intro c
    have acc := hwf.apply (c.st, c.I)
    -- strong induction on the accessibility of the measure
    generalize hm : (c.st, c.I) = m at acc
    induction acc generalizing c with
    | intro m _ ih =>
      subst hm
      constructor
      intro c' hstep
      obtain ⟨δ, hδ, _⟩ := hstep
      exact ih _ (hdec hδ) c' rfl
  exact ⟨this⟩

/-- Every configuration of a lawful operator reaches a stuck state
(Lemma 2.3.1 packaged with existence). -/
theorem Lawful.exists_stuck {op : Operator ins outs} (h : op.Lawful)
    (c : op.Config) : ∃ f, NormalizesTo op.OpStep c f :=
  exists_normalizesTo h.sn c

/-- With confluence, the stuck state of Lemma 2.3.1 is unique. -/
theorem Lawful.stuck_unique {op : Operator ins outs} (h : op.Lawful)
    {c f₁ f₂ : op.Config} (h₁ : NormalizesTo op.OpStep c f₁)
    (h₂ : NormalizesTo op.OpStep c f₂) : f₁ = f₂ :=
  h₁.unique h.confluent h₂

end Operator

end HydroLean
