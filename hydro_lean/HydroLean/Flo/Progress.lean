import HydroLean.Flo.Confluence

/-!
# Streaming progress for graphs (Lemma 2.4.4): infrastructure and core proof

We prove the bounded-outputs-fixed half of **Lemma 2.4.4**: for a well-typed
graph (Fig 2.5) whose bounded inputs are fixed, every reachable stuck
configuration has its bounded outputs fixed.

Following the paper, the proof canonicalizes the reduction: run the left
subgraph to its (unique) stuck state, then the right subgraph. Determinism
(Lemma 2.4.3) guarantees that the given stuck state equals the canonical one,
so it suffices to establish progress along the canonical run, where the
per-operator streaming progress obligation (Def 2.3.3) applies at the leaves.
-/

namespace HydroLean

namespace Graph

universe u

variable {i o : List Coll.{u}}

/-! ### `Bounds.le` and `FixedWhere` interaction -/

/-- Subtype-weakening for `FixedWhere`: if every `mb₂`-bounded port is also
`mb₁`-bounded (which `Bounds.le mb₁ mb₂` guarantees, since `U ≤ B` is
impossible), fixedness transfers (Fig 2.2 / Fig 2.5, seq rule). -/
theorem fixedWhere_mono :
    ∀ {ls : List Coll.{u}} {v : Vals ls} {mb₁ mb₂ : List Boundedness},
      Bounds.le mb₁ mb₂ → v.FixedWhere mb₁ → v.FixedWhere mb₂
  | _, .nil, _, _, _, _ => trivial
  | _ :: _, .cons _ _, [], [], _, _ => trivial
  | _ :: _, .cons x xs, b₁ :: mb₁, b₂ :: mb₂, hle, hfix => by
    refine ⟨fun hb => ?_, fixedWhere_mono hle.2 hfix.2⟩
    subst hb
    cases b₁ with
    | bounded => exact hfix.1 rfl
    | unbounded => exact absurd hle.1 (by simp [LE.le, Boundedness.le])

/-- Split `FixedWhere` across an appended tuple (inverse of
`Vals.fixedWhere_append`). -/
theorem fixedWhere_append_inv :
    ∀ {as bs : List Coll.{u}} (v : Vals as) (w : Vals bs)
      {fa fb : List Boundedness}, fa.length = as.length →
      Vals.FixedWhere (v.append w) (fa ++ fb) →
      v.FixedWhere fa ∧ w.FixedWhere fb
  | [], _, .nil, _, [], _, _, h => ⟨trivial, h⟩
  | _ :: _, _, .cons _ xs, w, _ :: _, _, hlen, h =>
    let ⟨h₁, h₂⟩ := fixedWhere_append_inv xs w (Nat.succ.inj hlen) h.2
    ⟨⟨h.1, h₁⟩, h₂⟩

/-! ### WT inversion (via a shape-indexed characterization, like `StepSpec`) -/

/-- Shape-indexed characterization of the typing judgment `WT` (Fig 2.5). -/
def WTSpec : {i o : List Coll.{u}} → Graph i o →
    List Boundedness → List Boundedness → Prop
  | ins, outs, .node op _ _, ib, ob =>
    op.Lawful ∧ ib = op.inBounds ∧ ob = op.outBounds ∧
      op.inBounds.length = ins.length ∧ op.outBounds.length = outs.length
  | _, _, .seq g₁ g₂, ib, ob =>
    ∃ mb₁ mb₂, WT g₁ ib mb₁ ∧ WT g₂ mb₂ ob ∧ Bounds.le mb₁ mb₂
  | _, _, @Graph.par i₁ o₁ i₂ o₂ g₁ g₂, ib, ob =>
    ∃ ib₁ ob₁ ib₂ ob₂, WT g₁ ib₁ ob₁ ∧ WT g₂ ib₂ ob₂ ∧
      ib = ib₁ ++ ib₂ ∧ ob = ob₁ ++ ob₂ ∧ ib₁.length = i₁.length

theorem wt_spec {g : Graph i o} {ib ob : List Boundedness}
    (h : g.WT ib ob) : WTSpec g ib ob := by
  cases h with
  | node op st buf hop hin hout => exact ⟨hop, rfl, rfl, hin, hout⟩
  | seq h₁ h₂ hle => exact ⟨_, _, h₁, h₂, hle⟩
  | par h₁ h₂ hlen => exact ⟨_, _, _, _, h₁, h₂, rfl, rfl, hlen⟩

/-- Typing ignores buffered inputs: `setInput` preserves `WT`. -/
theorem wt_setInput :
    ∀ {i o : List Coll.{u}} {g : Graph i o} {ib ob : List Boundedness},
      g.WT ib ob → ∀ I : Vals i, (g.setInput I).WT ib ob := by
  intro i o g
  induction g with
  | node op st buf =>
    intro ib ob h I
    obtain ⟨hop, rfl, rfl, hin, hout⟩ := wt_spec h
    exact .node op st I hop hin hout
  | seq g₁ g₂ ih₁ ih₂ =>
    intro ib ob h I
    obtain ⟨mb₁, mb₂, h₁, h₂, hle⟩ := wt_spec h
    exact .seq (ih₁ h₁ I) h₂ hle
  | par g₁ g₂ ih₁ ih₂ =>
    intro ib ob h I
    obtain ⟨ib₁, ob₁, ib₂, ob₂, h₁, h₂, rfl, rfl, hlen⟩ := wt_spec h
    exact .par (ih₁ h₁ _) (ih₂ h₂ _) hlen

/-- Well-typed graphs have lawful leaves (the `node` rule requires
`op.Lawful`). -/
theorem wt_leavesLawful :
    ∀ {i o : List Coll.{u}} {g : Graph i o} {ib ob : List Boundedness},
      g.WT ib ob → g.LeavesLawful := by
  intro i o g
  induction g with
  | node op st buf =>
    intro ib ob h
    exact (wt_spec h).1
  | seq g₁ g₂ ih₁ ih₂ =>
    intro ib ob h
    obtain ⟨mb₁, mb₂, h₁, h₂, _⟩ := wt_spec h
    exact ⟨ih₁ h₁, ih₂ h₂⟩
  | par g₁ g₂ ih₁ ih₂ =>
    intro ib ob h
    obtain ⟨ib₁, ob₁, ib₂, ob₂, h₁, h₂, _, _, _⟩ := wt_spec h
    exact ⟨ih₁ h₁, ih₂ h₂⟩

/-- Well-typed graphs have flag lists matching their port lists in length. -/
theorem wt_lengths :
    ∀ {i o : List Coll.{u}} {g : Graph i o} {ib ob : List Boundedness},
      g.WT ib ob → ib.length = i.length ∧ ob.length = o.length := by
  intro i o g
  induction g with
  | node op st buf =>
    intro ib ob h
    obtain ⟨_, rfl, rfl, hin, hout⟩ := wt_spec h
    exact ⟨hin, hout⟩
  | seq g₁ g₂ ih₁ ih₂ =>
    intro ib ob h
    obtain ⟨mb₁, mb₂, h₁, h₂, _⟩ := wt_spec h
    exact ⟨(ih₁ h₁).1, (ih₂ h₂).2⟩
  | par g₁ g₂ ih₁ ih₂ =>
    intro ib ob h
    obtain ⟨ib₁, ob₁, ib₂, ob₂, h₁, h₂, rfl, rfl, _⟩ := wt_spec h
    simp [List.length_append, (ih₁ h₁).1, (ih₁ h₁).2, (ih₂ h₂).1, (ih₂ h₂).2]

/-! ### Stuckness decomposition -/

/-- Graph-level stuckness (no step of the syntax, for any delta). -/
def GStuck (g : Graph i o) : Prop := ¬ ∃ g' δ, Step g g' δ

/-- A configuration is `CStep`-stuck iff its graph is `Step`-stuck: the output
buffer never blocks steps. -/
theorem stuck_cstep_iff {g : Graph i o} {O : Vals o} :
    Stuck CStep (⟨g, O⟩ : Config i o) ↔ GStuck g := by
  constructor
  · rintro h ⟨g', δ, hstep⟩
    exact h ⟨⟨g', O.concat δ⟩, δ, hstep, rfl⟩
  · rintro h ⟨⟨g', O'⟩, δ, hstep, -⟩
    exact h ⟨g', δ, hstep⟩

/-- A `seq` is stuck iff both subgraphs are stuck. -/
theorem gstuck_seq {m : List Coll.{u}} {g₁ : Graph i m} {g₂ : Graph m o} :
    GStuck (Graph.seq g₁ g₂) ↔ GStuck g₁ ∧ GStuck g₂ := by
  constructor
  · intro h
    constructor
    · rintro ⟨g₁', δ, hstep⟩
      exact h ⟨_, _, .seqLeft hstep⟩
    · rintro ⟨g₂', δ, hstep⟩
      exact h ⟨_, _, .seqRight hstep⟩
  · rintro ⟨h₁, h₂⟩ ⟨g', δ, hstep⟩
    rcases step_seq_inv hstep with ⟨g₁', δ₁, hs, -, -⟩ | ⟨g₂', hs, -⟩
    · exact h₁ ⟨_, _, hs⟩
    · exact h₂ ⟨_, _, hs⟩

/-- A `par` is stuck iff both subgraphs are stuck. -/
theorem gstuck_par {i₁ o₁ i₂ o₂ : List Coll.{u}} {g₁ : Graph i₁ o₁}
    {g₂ : Graph i₂ o₂} :
    GStuck (Graph.par g₁ g₂) ↔ GStuck g₁ ∧ GStuck g₂ := by
  constructor
  · intro h
    constructor
    · rintro ⟨g₁', δ, hstep⟩
      exact h ⟨_, _, .parLeft hstep⟩
    · rintro ⟨g₂', δ, hstep⟩
      exact h ⟨_, _, .parRight hstep⟩
  · rintro ⟨h₁, h₂⟩ ⟨g', δ, hstep⟩
    rcases step_par_inv hstep with ⟨g₁', δ₁, hs, -, -⟩ | ⟨g₂', δ₂, hs, -, -⟩
    · exact h₁ ⟨_, _, hs⟩
    · exact h₂ ⟨_, _, hs⟩

/-! ### Node-configuration transfer for normalization -/

/-- Reductions out of a node configuration stay node configurations. -/
theorem star_nodeCfg_inv {ins outs : List Coll.{u}} {op : Operator ins outs}
    {c : op.Config} {f : Config ins outs}
    (h : Star CStep (nodeCfg op c) f) :
    ∃ d : op.Config, f = nodeCfg op d ∧ Star op.OpStep c d := by
  induction h with
  | refl => exact ⟨c, rfl, Star.refl c⟩
  | tail _ hstep ih =>
    obtain ⟨d, rfl, hd⟩ := ih
    obtain ⟨e, rfl, he⟩ := cstep_nodeCfg_inv hstep
    exact ⟨e, rfl, hd.tail he⟩

/-- Stuckness transfers across the node-configuration isomorphism. -/
theorem stuck_nodeCfg {ins outs : List Coll.{u}} {op : Operator ins outs}
    {d : op.Config} (h : Stuck CStep (nodeCfg op d)) : Stuck op.OpStep d := by
  rintro ⟨e, he⟩
  exact h ⟨nodeCfg op e, cstep_nodeCfg he⟩

/-! ### Unique normal form (local assembly, avoiding an import cycle) -/

/-- Normal forms of lawful-graph configurations are unique (consequence of
Lemma 2.4.3, assembled from `lc_eager_of_lawful` + `newman_rooted`). -/
theorem normalizesTo_unique {g : Graph i o} (hl : g.LeavesLawful)
    {O : Vals o} {f f' : Config i o}
    (h : NormalizesTo CStep ⟨g, O⟩ f) (h' : NormalizesTo CStep ⟨g, O⟩ f') :
    f = f' := by
  have hjoin : Joinable CStep f f' := by
    refine newman_rooted (fun c : Config i o => c.g.LeavesLawful) ?_ ?_ ?_
      ⟨g, O⟩ hl f f' h.1 h'.1
    · rintro x y hx ⟨δ, hs, -⟩
      exact leavesLawful_step hs hx
    · intro x hx
      exact acc_cstep (accG x.g hx) x.O
    · intro x y z hx hxy hxz
      exact (lc_eager_of_lawful x.g.size x.g (Nat.le_refl _) hx).1 x.O y z hxy hxz
  obtain ⟨d, hd₁, hd₂⟩ := hjoin
  exact (Star.eq_of_stuck hd₁ h.2).trans (Star.eq_of_stuck hd₂ h'.2).symm

/-! ### Consistency threading (`Graph.Consistent` along runs)

`Operator.Inv` (the configuration-typing analogue) lifts structurally to graphs
via `Graph.Consistent`. For the canonical-run argument we need it preserved
when new input deltas arrive (leaves' `Lawful.inv_delta`) and, consequently,
along an upstream producer's entire run. -/

/-- Adding an input delta preserves graph consistency (lifting of
`Operator.Lawful.inv_delta`). -/
theorem consistent_addDelta :
    ∀ {i o : List Coll.{u}} (g : Graph i o), g.LeavesLawful →
      ∀ {O : Vals o}, g.Consistent O → ∀ Δ : Vals i,
        (g.setInput (g.inputs.concat Δ)).Consistent O
  | _, _, .node op st buf, hl, _, hc, Δ => hl.inv_delta Δ hc
  | _, _, .seq g₁ g₂, hl, O, hc, Δ =>
    ⟨consistent_addDelta g₁ hl.1 hc.1 Δ, hc.2⟩
  | _, _, .par g₁ g₂, hl, O, hc, Δ => by
    have hsplit := Vals.split_concat (g₁.inputs.append g₂.inputs) Δ
    rw [Vals.split_append] at hsplit
    show Graph.Consistent
      (.par (g₁.setInput ((g₁.inputs.append g₂.inputs).concat Δ).split.1)
            (g₂.setInput ((g₁.inputs.append g₂.inputs).concat Δ).split.2)) O
    rw [hsplit]
    exact ⟨consistent_addDelta g₁ hl.1 hc.1 _, consistent_addDelta g₂ hl.2 hc.2 _⟩

/-- Consistency of a downstream consumer is preserved along the entire run of
an upstream producer whose output buffer feeds the consumer's inputs: every
step only concatenates a delta onto the buffer, which `inv_delta` absorbs. -/
theorem consistent_along_run {i m o : List Coll.{u}} {g₂ : Graph m o}
    (hl₂ : g₂.LeavesLawful) {O : Vals o} (hc : g₂.Consistent O)
    {c f : Config i m} (hrun : Star CStep c f) (hstart : c.O = g₂.inputs) :
    (g₂.setInput f.O).Consistent O := by
  induction hrun with
  | refl =>
    rw [hstart, setInput_inputs]
    exact hc
  | tail hcb hstep ih =>
    obtain ⟨δ, -, hO⟩ := hstep
    have := consistent_addDelta (g₂.setInput _) (leavesLawful_setInput _ _ hl₂)
      ih δ
    rw [setInput_setInput, inputs_setInput] at this
    rw [hO]
    exact this

/-! ### The main induction for streaming progress -/

/-- **Streaming progress, inductive core** (Lemma 2.4.4): by strong induction
on graph size, canonicalizing the run as left-to-stuck then right-to-stuck. -/
theorem progress_aux :
    ∀ (n : Nat) {i o : List Coll.{u}} (g : Graph i o), g.size ≤ n →
      ∀ {ib ob : List Boundedness}, g.WT ib ob →
        g.inputs.FixedWhere ib →
        ∀ {O : Vals o}, g.Consistent O → ∀ {f : Config i o},
          NormalizesTo CStep ⟨g, O⟩ f → f.O.FixedWhere ob := by
  intro n
  induction n with
  | zero =>
    intro i o g hs
    exact absurd (Nat.lt_of_lt_of_le (size_pos g) hs) (Nat.lt_irrefl 0)
  | succ n ihn =>
    intro i o g hs ib ob hwt hfix O hcons f hnorm
    match g, hs, hwt, hfix, hcons, hnorm with
    | .node op st buf, _, hwt, hfix, hcons, hnorm =>
      obtain ⟨hop, rfl, rfl, -, -⟩ := wt_spec hwt
      -- transfer the run to operator configurations
      obtain ⟨d, rfl, hd⟩ := star_nodeCfg_inv (c := ⟨buf, st, O⟩) hnorm.1
      have hstuck : Stuck op.OpStep d := stuck_nodeCfg hnorm.2
      exact (hop.progress ⟨buf, st, O⟩ d hcons hfix ⟨hd, hstuck⟩).2
    | .seq (m := mid) g₁ g₂, hs, hwt, hfix, hcons, hnorm =>
      obtain ⟨mb₁, mb₂, h₁, h₂, hle⟩ := wt_spec hwt
      have hs₁ : g₁.size ≤ n := Nat.le_of_lt_succ
        (Nat.lt_of_lt_of_le (Nat.lt_add_of_pos_right (size_pos g₂)) hs)
      have hs₂ : g₂.size ≤ n := Nat.le_of_lt_succ
        (Nat.lt_of_lt_of_le (Nat.lt_add_of_pos_left (size_pos g₁)) hs)
      have hl₁ : g₁.LeavesLawful := wt_leavesLawful h₁
      have hl₂ : g₂.LeavesLawful := wt_leavesLawful h₂
      -- 1. run g₁ to its stuck state, with g₂'s buffer as its output buffer
      obtain ⟨f₁, hf₁⟩ :=
        exists_normalizesTo_of_acc (acc_cstep (accG g₁ hl₁) g₂.inputs)
      -- 2. its final outputs (= g₂'s new inputs) have bounded ports fixed
      have hmid : f₁.O.FixedWhere mb₁ := ihn g₁ hs₁ h₁ hfix hcons.1 hf₁
      -- 3. run g₂ (with the new buffer) to its stuck state
      have hl₂' : (g₂.setInput f₁.O).LeavesLawful := leavesLawful_setInput _ _ hl₂
      have hcons₂ : (g₂.setInput f₁.O).Consistent O :=
        consistent_along_run hl₂ hcons.2 hf₁.1 rfl
      obtain ⟨f₂, hf₂⟩ :=
        exists_normalizesTo_of_acc (acc_cstep (accG _ hl₂') O)
      have hout : f₂.O.FixedWhere ob := by
        refine ihn (g₂.setInput f₁.O) ?_ (wt_setInput h₂ _) ?_ hcons₂ hf₂
        · simpa [size_setInput] using hs₂
        · simpa [inputs_setInput] using fixedWhere_mono hle hmid
      -- 4. assemble the canonical run and its stuckness
      have canonical : NormalizesTo CStep (⟨.seq g₁ g₂, O⟩ : Config i o)
          ⟨.seq f₁.g f₂.g, f₂.O⟩ := by
        constructor
        · have lift₁ : Star CStep (⟨.seq g₁ g₂, O⟩ : Config i o)
              ⟨.seq f₁.g (g₂.setInput f₁.O), O⟩ := by
            have := liftLeft (g₂ := g₂) (O := O) (c := ⟨g₁, g₂.inputs⟩) hf₁.1
            simpa [setInput_inputs] using this
          have lift₂ : Star CStep (⟨.seq f₁.g (g₂.setInput f₁.O), O⟩ : Config i o)
              ⟨.seq f₁.g f₂.g, f₂.O⟩ :=
            liftRight (g₁ := f₁.g) (c := ⟨g₂.setInput f₁.O, O⟩) hf₂.1
          exact lift₁.trans lift₂
        · rw [stuck_cstep_iff, gstuck_seq]
          exact ⟨(stuck_cstep_iff (O := f₁.O)).1
              (show Stuck CStep (⟨f₁.g, f₁.O⟩ : Config i mid) from hf₁.2),
            (stuck_cstep_iff (O := f₂.O)).1
              (show Stuck CStep (⟨f₂.g, f₂.O⟩ : Config mid o) from hf₂.2)⟩
      -- 5. the given stuck state equals the canonical one (Lemma 2.4.3)
      have : f = ⟨.seq f₁.g f₂.g, f₂.O⟩ :=
        normalizesTo_unique
          (show (Graph.seq g₁ g₂).LeavesLawful from ⟨hl₁, hl₂⟩) hnorm canonical
      rw [this]
      exact hout
    | .par (i₁ := pi₁) (o₁ := po₁) (i₂ := pi₂) (o₂ := po₂) g₁ g₂, hs, hwt, hfix, hcons, hnorm =>
      obtain ⟨ib₁, ob₁, ib₂, ob₂, h₁, h₂, rfl, rfl, hlen⟩ := wt_spec hwt
      have hs₁ : g₁.size ≤ n := Nat.le_of_lt_succ
        (Nat.lt_of_lt_of_le (Nat.lt_add_of_pos_right (size_pos g₂)) hs)
      have hs₂ : g₂.size ≤ n := Nat.le_of_lt_succ
        (Nat.lt_of_lt_of_le (Nat.lt_add_of_pos_left (size_pos g₁)) hs)
      have hl₁ : g₁.LeavesLawful := wt_leavesLawful h₁
      have hl₂ : g₂.LeavesLawful := wt_leavesLawful h₂
      -- split the fixed inputs and the output buffer
      obtain ⟨hfix₁, hfix₂⟩ :=
        fixedWhere_append_inv g₁.inputs g₂.inputs hlen hfix
      obtain ⟨A, B, rfl⟩ : ∃ A B, O = Vals.append A B :=
        ⟨O.split.1, O.split.2, (Vals.append_split O).symm⟩
      -- consistency splits componentwise (split ∘ append = id)
      have hconsAB : g₁.Consistent A ∧ g₂.Consistent B := by
        have h := hcons
        simp only [Graph.Consistent, Vals.split_append] at h
        exact h
      -- run both subgraphs to their stuck states
      obtain ⟨f₁, hf₁⟩ := exists_normalizesTo_of_acc (acc_cstep (accG g₁ hl₁) A)
      obtain ⟨f₂, hf₂⟩ := exists_normalizesTo_of_acc (acc_cstep (accG g₂ hl₂) B)
      have hout₁ : f₁.O.FixedWhere ob₁ := ihn g₁ hs₁ h₁ hfix₁ hconsAB.1 hf₁
      have hout₂ : f₂.O.FixedWhere ob₂ := ihn g₂ hs₂ h₂ hfix₂ hconsAB.2 hf₂
      have canonical : NormalizesTo CStep
          (⟨.par g₁ g₂, A.append B⟩ : Config (pi₁ ++ pi₂) (po₁ ++ po₂))
          ⟨.par f₁.g f₂.g, f₁.O.append f₂.O⟩ := by
        constructor
        · have lift₁ : Star CStep
              (⟨.par g₁ g₂, A.append B⟩ : Config (pi₁ ++ pi₂) (po₁ ++ po₂))
              ⟨.par f₁.g g₂, f₁.O.append B⟩ :=
            liftParLeft (g₂ := g₂) (O₂ := B) (c := ⟨g₁, A⟩) hf₁.1
          have lift₂ : Star CStep
              (⟨.par f₁.g g₂, f₁.O.append B⟩ : Config (pi₁ ++ pi₂) (po₁ ++ po₂))
              ⟨.par f₁.g f₂.g, f₁.O.append f₂.O⟩ :=
            liftParRight (g₁ := f₁.g) (O₁ := f₁.O) (c := ⟨g₂, B⟩) hf₂.1
          exact lift₁.trans lift₂
        · rw [stuck_cstep_iff, gstuck_par]
          exact ⟨(stuck_cstep_iff (O := f₁.O)).1
              (show Stuck CStep (⟨f₁.g, f₁.O⟩ : Config pi₁ po₁) from hf₁.2),
            (stuck_cstep_iff (O := f₂.O)).1
              (show Stuck CStep (⟨f₂.g, f₂.O⟩ : Config pi₂ po₂) from hf₂.2)⟩
      have : f = ⟨.par f₁.g f₂.g, f₁.O.append f₂.O⟩ :=
        normalizesTo_unique
          (show (Graph.par g₁ g₂).LeavesLawful from ⟨hl₁, hl₂⟩) hnorm canonical
      rw [this]
      exact Vals.fixedWhere_append f₁.O f₂.O (wt_lengths h₁).2 hout₁ hout₂

end Graph

end HydroLean
