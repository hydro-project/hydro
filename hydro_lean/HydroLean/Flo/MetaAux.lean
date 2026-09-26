import HydroLean.Flo.Graph

/-!
# Auxiliary metatheory for Flo graphs (dissertation §2.4)

Infrastructure for the proofs of Lemmas 2.4.2–2.4.4 in `Flo/Theorems.lean`:

- `setInput` algebra (`setInput_setInput`, `setInput_inputs`) — buffered inputs
  form a "lens" on graph syntax;
- inversion lemmas for the graph small-step at each syntactic head;
- preservation of leaf-lawfulness and of graph size along steps;
- accessibility (`AccG`): a graph with lawful leaves admits no infinite step
  sequences (the well-founded core of Lemma 2.4.2), proven by strong induction
  on graph size with nested well-founded inductions for `seq`/`par` — this is
  the mechanized form of the paper's "the right side may be re-enabled by the
  left, but this cycles back to the left which eventually gets stuck";
- lifting lemmas embedding reductions of a subgraph into reductions of a
  composite graph (`liftLeft`, `liftRight`, `liftParLeft`, `liftParRight`) —
  the formal counterpart of the paper's trace manipulations in §2.4.2.
-/

namespace HydroLean

namespace Graph

universe u

variable {i o : List Coll.{u}}

/-! ## `setInput` algebra -/

@[simp] theorem setInput_setInput :
    ∀ {i o : List Coll.{u}} (g : Graph i o) (I I' : Vals i),
      (g.setInput I).setInput I' = g.setInput I'
  | _, _, .node _ _ _, _, _ => rfl
  | _, _, .seq g₁ g₂, I, I' => by simp [setInput, setInput_setInput g₁]
  | _, _, .par g₁ g₂, I, I' => by
    simp [setInput, setInput_setInput g₁, setInput_setInput g₂]

@[simp] theorem setInput_inputs :
    ∀ {i o : List Coll.{u}} (g : Graph i o), g.setInput g.inputs = g
  | _, _, .node _ _ _ => rfl
  | _, _, .seq g₁ g₂ => by simp [setInput, inputs, setInput_inputs g₁]
  | _, _, .par g₁ g₂ => by
    simp [setInput, inputs, setInput_inputs g₁, setInput_inputs g₂]

/-- `setInput` on a parallel composition, phrased via `Vals.append`. -/
theorem setInput_par_append {i₁ o₁ i₂ o₂ : List Coll.{u}}
    (g₁ : Graph i₁ o₁) (g₂ : Graph i₂ o₂) (X : Vals i₁) (Y : Vals i₂) :
    (Graph.par g₁ g₂).setInput (X.append Y) = .par (g₁.setInput X) (g₂.setInput Y) := by
  simp [setInput]

/-- Introducing a delta on a `seq` acts on the left subgraph. -/
theorem addDelta_seq {i m o : List Coll.{u}} (g₁ : Graph i m) (g₂ : Graph m o)
    (Δ : Vals i) : (Graph.seq g₁ g₂).addDelta Δ = .seq (g₁.addDelta Δ) g₂ := by
  simp [addDelta, inputs, setInput]

/-- Introducing a delta on a `par` splits it across the two subgraphs. -/
theorem addDelta_par {i₁ o₁ i₂ o₂ : List Coll.{u}}
    (g₁ : Graph i₁ o₁) (g₂ : Graph i₂ o₂) (Δ₁ : Vals i₁) (Δ₂ : Vals i₂) :
    (Graph.par g₁ g₂).addDelta (Δ₁.append Δ₂) = .par (g₁.addDelta Δ₁) (g₂.addDelta Δ₂) := by
  simp [addDelta, inputs, ← Vals.append_concat, setInput_par_append]

/-- Introducing a delta on a `node` concatenates onto its buffer. -/
theorem addDelta_node {ins outs : List Coll.{u}} (op : Operator ins outs)
    (st : op.State) (buf : Vals ins) (Δ : Vals ins) :
    (Graph.node op st buf).addDelta Δ = .node op st (buf.concat Δ) := rfl

/-! ## Step inversion

`cases` on a hypothesis `Step (.par g₁ g₂) g' δ` fails dependent elimination
because the port indices `i₁ ++ i₂` are not constructor-headed. We instead
prove a *shape-indexed characterization* `StepSpec` by case analysis on an
unconstrained step (no index unification needed), and derive the per-shape
inversion lemmas definitionally. -/

/-- Characterization of step targets per syntactic shape of the source. -/
def StepSpec : {I O : List Coll.{u}} → Graph I O → Graph I O → Vals O → Prop
  | _, _, .node op st buf, g', δ =>
    ∃ st' buf', g' = .node op st' buf' ∧ op.step buf st buf' st' δ
  | _, o, .seq g₁ g₂, g', δ =>
    (∃ g₁' δm, Step g₁ g₁' δm ∧
        g' = .seq g₁' (g₂.setInput (g₂.inputs.concat δm)) ∧ δ = Vals.empty o) ∨
    (∃ g₂', Step g₂ g₂' δ ∧ g' = .seq g₁ g₂')
  | _, _, @Graph.par _ o₁ _ o₂ g₁ g₂, g', δ =>
    (∃ g₁' δ₁, Step g₁ g₁' δ₁ ∧ g' = .par g₁' g₂ ∧ δ = δ₁.append (Vals.empty o₂)) ∨
    (∃ g₂' δ₂, Step g₂ g₂' δ₂ ∧ g' = .par g₁ g₂' ∧ δ = (Vals.empty o₁).append δ₂)

/-- Every step satisfies the shape characterization. -/
theorem step_spec {I O : List Coll.{u}} {G G' : Graph I O} {δ : Vals O}
    (h : Step G G' δ) : StepSpec G G' δ := by
  cases h with
  | node hop => exact ⟨_, _, rfl, hop⟩
  | seqLeft hstep => exact .inl ⟨_, _, hstep, rfl, rfl⟩
  | seqRight hstep => exact .inr ⟨_, hstep, rfl⟩
  | parLeft hstep => exact .inl ⟨_, _, hstep, rfl, rfl⟩
  | parRight hstep => exact .inr ⟨_, _, hstep, rfl, rfl⟩

theorem step_node_inv {ins outs : List Coll.{u}} {op : Operator ins outs}
    {st : op.State} {buf : Vals ins} {g' : Graph ins outs} {δ : Vals outs}
    (h : Step (.node op st buf) g' δ) :
    ∃ st' buf', g' = .node op st' buf' ∧ op.step buf st buf' st' δ :=
  step_spec h

theorem step_seq_inv {i m o : List Coll.{u}} {g₁ : Graph i m} {g₂ : Graph m o}
    {g' : Graph i o} {δ : Vals o} (h : Step (.seq g₁ g₂) g' δ) :
    (∃ g₁' δm, Step g₁ g₁' δm ∧
        g' = .seq g₁' (g₂.setInput (g₂.inputs.concat δm)) ∧ δ = Vals.empty o) ∨
    (∃ g₂', Step g₂ g₂' δ ∧ g' = .seq g₁ g₂') :=
  step_spec h

theorem step_par_inv {i₁ o₁ i₂ o₂ : List Coll.{u}} {g₁ : Graph i₁ o₁}
    {g₂ : Graph i₂ o₂} {g' : Graph (i₁ ++ i₂) (o₁ ++ o₂)} {δ : Vals (o₁ ++ o₂)}
    (h : Step (.par g₁ g₂) g' δ) :
    (∃ g₁' δ₁, Step g₁ g₁' δ₁ ∧ g' = .par g₁' g₂ ∧ δ = δ₁.append (Vals.empty o₂)) ∨
    (∃ g₂' δ₂, Step g₂ g₂' δ₂ ∧ g' = .par g₁ g₂' ∧ δ = (Vals.empty o₁).append δ₂) :=
  step_spec h

/-! ## Preservation of lawfulness and size -/

/-- `setInput` does not change the operators at the leaves. -/
theorem leavesLawful_setInput :
    ∀ {i o : List Coll.{u}} (g : Graph i o) (I : Vals i),
      g.LeavesLawful → (g.setInput I).LeavesLawful
  | _, _, .node _ _ _, _, h => h
  | _, _, .seq g₁ _, I, h => ⟨leavesLawful_setInput g₁ I h.1, h.2⟩
  | _, _, .par g₁ g₂, _, h =>
    ⟨leavesLawful_setInput g₁ _ h.1, leavesLawful_setInput g₂ _ h.2⟩

/-- Graph steps preserve leaf lawfulness (they only change buffers/state). -/
theorem leavesLawful_step {g g' : Graph i o} {δ : Vals o}
    (hs : Step g g' δ) (hl : g.LeavesLawful) : g'.LeavesLawful := by
  induction hs with
  | node _ => exact hl
  | seqLeft _ ih => exact ⟨ih hl.1, leavesLawful_setInput _ _ hl.2⟩
  | seqRight _ ih => exact ⟨hl.1, ih hl.2⟩
  | parLeft _ ih => exact ⟨ih hl.1, hl.2⟩
  | parRight _ ih => exact ⟨hl.1, ih hl.2⟩

/-- The number of operator leaves in a graph; invariant under steps. -/
def size : {i o : List Coll.{u}} → Graph i o → Nat
  | _, _, .node _ _ _ => 1
  | _, _, .seq g₁ g₂ => g₁.size + g₂.size
  | _, _, .par g₁ g₂ => g₁.size + g₂.size

@[simp] theorem size_setInput :
    ∀ {i o : List Coll.{u}} (g : Graph i o) (I : Vals i), (g.setInput I).size = g.size
  | _, _, .node _ _ _, _ => rfl
  | _, _, .seq g₁ _, I => by simp [setInput, size, size_setInput g₁]
  | _, _, .par g₁ g₂, I => by
    simp [setInput, size, size_setInput g₁, size_setInput g₂]

theorem size_pos : ∀ {i o : List Coll.{u}} (g : Graph i o), 0 < g.size
  | _, _, .node _ _ _ => Nat.zero_lt_one
  | _, _, .seq g₁ _ => Nat.lt_of_lt_of_le (size_pos g₁) (Nat.le_add_right _ _)
  | _, _, .par g₁ _ => Nat.lt_of_lt_of_le (size_pos g₁) (Nat.le_add_right _ _)

theorem size_step {g g' : Graph i o} {δ : Vals o} (hs : Step g g' δ) :
    g'.size = g.size := by
  induction hs with
  | node _ => rfl
  | seqLeft _ ih => simp [size, ih]
  | seqRight _ ih => simp [size, ih]
  | parLeft _ ih => simp [size, ih]
  | parRight _ ih => simp [size, ih]

/-! ## Accessibility: no infinite step sequences (core of Lemma 2.4.2) -/

/-- Accessibility of a graph under the (inverse) step relation: every step
sequence out of `g` is finite. -/
def AccG (g : Graph i o) : Prop :=
  Acc (fun g' g => ∃ δ, Step g g' δ) g

/-- A single lawful operator node is accessible: its steps decrease the
operator's well-founded order (Lemma 2.3.1 at the leaf). -/
theorem accG_node {ins outs : List Coll.{u}} (op : Operator ins outs)
    (hop : op.Lawful) (st : op.State) (buf : Vals ins) :
    AccG (.node op st buf) := by
  obtain ⟨r, hwf, hdec⟩ := hop.wf_decreasing
  have h := hwf.apply (st, buf)
  generalize hm : (st, buf) = p at h
  induction h generalizing st buf with
  | intro p _ ih =>
    subst hm
    constructor
    rintro g' ⟨δ, hstep⟩
    obtain ⟨st', buf', rfl, hop'⟩ := step_node_inv hstep
    exact ih _ (hdec hop') _ _ rfl

/-- **Graph accessibility** (engine of Lemma 2.4.2): every graph with lawful
leaves is accessible. Strong induction on `size` (steps and `setInput` preserve
size and lawfulness, so all graphs reachable in the sub-inductions stay in the
inductive class), with nested well-founded inductions for the composite cases:
for `seq g₁ g₂`, a lexicographic descent on (accessibility of `g₁`,
accessibility of `g₂`) — a `seqLeft` step decreases the first component (and may
reset the second, whose accessibility is re-derived from the size induction),
while a `seqRight` step decreases the second. -/
theorem accG_of_lawful :
    ∀ (n : Nat) {i o : List Coll.{u}} (g : Graph i o),
      g.size ≤ n → g.LeavesLawful → AccG g := by
  intro n
  induction n with
  | zero =>
    intro i o g hs _
    exact absurd (Nat.lt_of_lt_of_le (size_pos g) hs) (Nat.lt_irrefl 0)
  | succ n ihn =>
    intro i o g hs hl
    match g, hs, hl with
    | .node op st buf, _, hl => exact accG_node op hl st buf
    | .seq (m := mid) g₁ g₂, hs, hl =>
      have hs₁ : g₁.size ≤ n :=
        Nat.le_of_lt_succ (Nat.lt_of_lt_of_le
          (Nat.lt_add_of_pos_right (size_pos g₂)) hs)
      have hs₂ : g₂.size ≤ n :=
        Nat.le_of_lt_succ (Nat.lt_of_lt_of_le
          (Nat.lt_add_of_pos_left (size_pos g₁)) hs)
      -- outer WF induction on accessibility of the left subgraph
      have main : ∀ (h₁ : Graph i mid), AccG h₁ → h₁.LeavesLawful → h₁.size ≤ n →
          ∀ (h₂ : Graph mid o), h₂.LeavesLawful → h₂.size ≤ n →
            AccG (.seq h₁ h₂) := by
        intro h₁ A₁
        induction A₁ with
        | intro h₁ _ ih₁ =>
          intro hl₁ hsz₁ h₂ hl₂ hsz₂
          -- inner WF induction on accessibility of the right subgraph
          have A₂ : AccG h₂ := ihn h₂ hsz₂ hl₂
          revert hl₂ hsz₂
          induction A₂ with
          | intro h₂ A₂' ih₂ =>
            intro hl₂ hsz₂
            constructor
            rintro g' ⟨δ, hstep⟩
            rcases step_seq_inv hstep with
              ⟨h₁', δm, hs₁', rfl, -⟩ | ⟨h₂', hs₂', rfl⟩
            · exact ih₁ h₁' ⟨δm, hs₁'⟩ (leavesLawful_step hs₁' hl₁)
                (size_step hs₁' ▸ hsz₁) (h₂.setInput (h₂.inputs.concat δm))
                (leavesLawful_setInput _ _ hl₂)
                (size_setInput h₂ _ ▸ hsz₂)
            · exact ih₂ h₂' ⟨δ, hs₂'⟩ (leavesLawful_step hs₂' hl₂)
                ((size_step hs₂') ▸ hsz₂)
      exact main g₁ (ihn g₁ hs₁ hl.1) hl.1 hs₁ g₂ hl.2 hs₂
    | .par (i₁ := pi₁) (o₁ := po₁) (i₂ := pi₂) (o₂ := po₂) g₁ g₂, hs, hl =>
      have hs₁ : g₁.size ≤ n :=
        Nat.le_of_lt_succ (Nat.lt_of_lt_of_le
          (Nat.lt_add_of_pos_right (size_pos g₂)) hs)
      have hs₂ : g₂.size ≤ n :=
        Nat.le_of_lt_succ (Nat.lt_of_lt_of_le
          (Nat.lt_add_of_pos_left (size_pos g₁)) hs)
      have main : ∀ (h₁ : Graph pi₁ po₁), AccG h₁ → h₁.LeavesLawful → h₁.size ≤ n →
          ∀ (h₂ : Graph pi₂ po₂), h₂.LeavesLawful → h₂.size ≤ n →
            AccG (.par h₁ h₂) := by
        intro h₁ A₁
        induction A₁ with
        | intro h₁ _ ih₁ =>
          intro hl₁ hsz₁ h₂ hl₂ hsz₂
          have A₂ : AccG h₂ := ihn h₂ hsz₂ hl₂
          revert hl₂ hsz₂
          induction A₂ with
          | intro h₂ A₂' ih₂ =>
            intro hl₂ hsz₂
            constructor
            rintro g' ⟨δ, hstep⟩
            rcases step_par_inv hstep with
              ⟨h₁', δ₁, hs₁', rfl, -⟩ | ⟨h₂', δ₂, hs₂', rfl, -⟩
            · exact ih₁ h₁' ⟨δ₁, hs₁'⟩ (leavesLawful_step hs₁' hl₁)
                ((size_step hs₁') ▸ hsz₁) h₂ hl₂ hsz₂
            · exact ih₂ h₂' ⟨δ₂, hs₂'⟩ (leavesLawful_step hs₂' hl₂)
                ((size_step hs₂') ▸ hsz₂)
      exact main g₁ (ihn g₁ hs₁ hl.1) hl.1 hs₁ g₂ hl.2 hs₂

/-- Every graph with lawful leaves is accessible. -/
theorem accG (g : Graph i o) (hl : g.LeavesLawful) : AccG g :=
  accG_of_lawful g.size g (Nat.le_refl _) hl

/-- Lift graph accessibility to configuration accessibility: `CStep` only adds
the output buffer, which never blocks steps. -/
theorem acc_cstep {g : Graph i o} (h : AccG g) (O : Vals o) :
    Acc (fun c' c => CStep c c') (⟨g, O⟩ : Config i o) := by
  induction h generalizing O with
  | intro g _ ih =>
    constructor
    rintro ⟨g', O'⟩ ⟨δ, hstep, hO⟩
    exact ih g' ⟨δ, hstep⟩ O'

/-! ## The node-configuration isomorphism

A configuration of a single-operator graph is the same thing as an operator
configuration (§2.3.4); steps correspond exactly. -/

/-- View an operator configuration as a graph configuration of the
corresponding single-node graph. -/
def nodeCfg {ins outs : List Coll.{u}} (op : Operator ins outs)
    (c : op.Config) : Config ins outs :=
  ⟨.node op c.st c.I, c.O⟩

theorem cstep_nodeCfg {ins outs : List Coll.{u}} {op : Operator ins outs}
    {c d : op.Config} (h : op.OpStep c d) :
    CStep (nodeCfg op c) (nodeCfg op d) := by
  obtain ⟨δ, hstep, hO⟩ := h
  exact ⟨δ, .node hstep, hO⟩

theorem star_cstep_nodeCfg {ins outs : List Coll.{u}} {op : Operator ins outs}
    {c d : op.Config} (h : Star op.OpStep c d) :
    Star CStep (nodeCfg op c) (nodeCfg op d) := by
  induction h with
  | refl => exact Star.refl _
  | tail _ hstep ih => exact ih.tail (cstep_nodeCfg hstep)

theorem cstep_nodeCfg_inv {ins outs : List Coll.{u}} {op : Operator ins outs}
    {c : op.Config} {c' : Config ins outs} (h : CStep (nodeCfg op c) c') :
    ∃ d : op.Config, c' = nodeCfg op d ∧ op.OpStep c d := by
  obtain ⟨δ, hstep, hO⟩ := h
  obtain ⟨st', buf', hg, hop⟩ := step_node_inv hstep
  refine ⟨⟨buf', st', c'.O⟩, ?_, ⟨δ, hop, hO⟩⟩
  simp [nodeCfg, ← hg]

/-! ## Lifting subgraph reductions into composite graphs (§2.4.2)

These lemmas embed a reduction of a subgraph into a reduction of the composite,
mirroring the paper's trace manipulations. Crucially, for `seq g₁ g₂`, the
output buffer of the *left* configuration is exactly the input buffer of the
right subgraph (`liftLeft`). -/

/-- Lift a single step of the left subgraph of a `seq`: the left's outputs are
threaded into the right subgraph's input buffer, and the composite step emits
`∅` at the outer outputs (`Vals.concat_empty`). -/
theorem liftLeft_single {i m o : List Coll.{u}} {g₂ : Graph m o} {O : Vals o}
    {c c' : Config i m} (h : CStep c c') :
    CStep (⟨.seq c.g (g₂.setInput c.O), O⟩ : Config i o)
      ⟨.seq c'.g (g₂.setInput c'.O), O⟩ := by
  obtain ⟨δ, hstep', hO⟩ := h
  refine ⟨Vals.empty o, ?_, (Vals.concat_empty O).symm⟩
  have step := Step.seqLeft (g₂ := g₂.setInput c.O) hstep'
  -- `setInput (setInput g₂ M) (inputs (setInput g₂ M) ++ δ) = setInput g₂ (M ++ δ)`
  simpa [setInput_setInput, inputs_setInput, hO] using step

theorem liftLeft {i m o : List Coll.{u}} {g₂ : Graph m o} {O : Vals o}
    {c c' : Config i m} (h : Star CStep c c') :
    Star CStep (⟨.seq c.g (g₂.setInput c.O), O⟩ : Config i o)
      ⟨.seq c'.g (g₂.setInput c'.O), O⟩ := by
  induction h with
  | refl => exact Star.refl _
  | tail _ hstep ih => exact ih.tail (liftLeft_single hstep)

/-- Lift a reduction of the right subgraph of a `seq`: composite outputs are
the right subgraph's outputs. -/
theorem liftRight {i m o : List Coll.{u}} {g₁ : Graph i m}
    {c c' : Config m o} (h : Star CStep c c') :
    Star CStep (⟨.seq g₁ c.g, c.O⟩ : Config i o) ⟨.seq g₁ c'.g, c'.O⟩ := by
  induction h with
  | refl => exact Star.refl _
  | tail _ hstep ih =>
    obtain ⟨δ, hstep', hO⟩ := hstep
    exact ih.tail ⟨δ, .seqRight hstep', hO⟩

/-- Lift a reduction of the left subgraph of a `par`; the right half of the
output buffer is untouched. -/
theorem liftParLeft {i₁ o₁ i₂ o₂ : List Coll.{u}} {g₂ : Graph i₂ o₂}
    {O₂ : Vals o₂} {c c' : Config i₁ o₁} (h : Star CStep c c') :
    Star CStep (⟨.par c.g g₂, c.O.append O₂⟩ : Config (i₁ ++ i₂) (o₁ ++ o₂))
      ⟨.par c'.g g₂, c'.O.append O₂⟩ := by
  induction h with
  | refl => exact Star.refl _
  | tail _ hstep ih =>
    obtain ⟨δ, hstep', hO⟩ := hstep
    refine ih.tail ⟨δ.append (Vals.empty o₂), .parLeft hstep', ?_⟩
    rw [hO, ← Vals.append_concat, Vals.concat_empty]

/-- Lift a reduction of the right subgraph of a `par`. -/
theorem liftParRight {i₁ o₁ i₂ o₂ : List Coll.{u}} {g₁ : Graph i₁ o₁}
    {O₁ : Vals o₁} {c c' : Config i₂ o₂} (h : Star CStep c c') :
    Star CStep (⟨.par g₁ c.g, O₁.append c.O⟩ : Config (i₁ ++ i₂) (o₁ ++ o₂))
      ⟨.par g₁ c'.g, O₁.append c'.O⟩ := by
  induction h with
  | refl => exact Star.refl _
  | tail _ hstep ih =>
    obtain ⟨δ, hstep', hO⟩ := hstep
    refine ih.tail ⟨(Vals.empty o₁).append δ, .parRight hstep', ?_⟩
    rw [hO, ← Vals.append_concat, Vals.concat_empty]

end Graph

end HydroLean
