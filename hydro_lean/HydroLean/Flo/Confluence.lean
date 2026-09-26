import HydroLean.Flo.MetaAux

/-!
# Local confluence and eager execution for lawful graphs (§2.4.2)

The mechanized core of **Lemma 2.4.3**. We prove, by strong induction on graph
size, that every graph with lawful leaves satisfies:

- **local confluence** (`LC`): one-step peaks join — the formal residue of the
  paper's trace-rewriting argument; and
- **eager execution** (Def 2.4.2, `EagerG`): input deltas commute with steps up
  to joinability.

The interesting interaction is in `seq g₁ g₂`:

- a `seqLeft`/`seqRight` peak is resolved by the *right* subgraph's eager
  execution — the delta emitted by the left subgraph is exactly an input delta
  for the right subgraph (the paper: "the execution of `aᵢ…aⱼ` simply
  introduces additional data for `b` to process");
- a `seqLeft`/`seqLeft` peak is resolved by the *left* subgraph's local
  confluence, observing that the right subgraph's input buffer plays the role
  of the left subgraph's output buffer (`liftLeft`).

Global confluence (Def 2.4.1) then follows via Newman's lemma
(`newman_rooted`) and accessibility (`accG`), assembled in `Flo/Theorems.lean`.
-/

namespace HydroLean

namespace Graph

universe u

variable {i o : List Coll.{u}}

/-- One-step local confluence at graph `g`, for every output buffer. -/
def LC (g : Graph i o) : Prop :=
  ∀ (O : Vals o) (c₁ c₂ : Config i o),
    CStep ⟨g, O⟩ c₁ → CStep ⟨g, O⟩ c₂ → Joinable CStep c₁ c₂

/-! ### Auxiliary constructions for the `seq` cases -/

section Seq

variable {m : List Coll.{u}} {g₁ : Graph i m} {g₂ : Graph m o}

/-- A `seqLeft` `CStep` at the configuration level: the left subgraph steps and
its delta is pushed into the right subgraph's buffer; outputs are unchanged. -/
theorem cstep_seqLeft {a₁ : Graph i m} {δ₁ : Vals m} (h : Step g₁ a₁ δ₁)
    (O : Vals o) :
    CStep (⟨.seq g₁ g₂, O⟩ : Config i o)
      ⟨.seq a₁ (g₂.setInput (g₂.inputs.concat δ₁)), O⟩ :=
  ⟨Vals.empty o, .seqLeft h, (Vals.concat_empty O).symm⟩

/-- A `seqRight` `CStep` at the configuration level. -/
theorem cstep_seqRight {b₂ : Graph m o} {ε : Vals o} (h : Step g₂ b₂ ε)
    (O : Vals o) :
    CStep (⟨.seq g₁ g₂, O⟩ : Config i o) ⟨.seq g₁ b₂, O.concat ε⟩ :=
  ⟨ε, .seqRight h, rfl⟩

/-- The mixed `seq` peak (`seqLeft` vs `seqRight`), resolved by the right
subgraph's eager execution: the delta the left subgraph emitted is an input
delta for the right subgraph, so introducing it before or after the right
subgraph's step joins. This is the crux of the paper's §2.4.2 argument. -/
theorem seq_mixed_join (hE₂ : g₂.EagerG)
    {a₁ : Graph i m} {δ₁ : Vals m} (hL : Step g₁ a₁ δ₁)
    {b₂ : Graph m o} {ε : Vals o} (hR : Step g₂ b₂ ε) (O : Vals o) :
    Joinable CStep
      (⟨.seq a₁ (g₂.setInput (g₂.inputs.concat δ₁)), O⟩ : Config i o)
      ⟨.seq g₁ b₂, O.concat ε⟩ := by
  -- eager execution of g₂: introduce δ₁ before or after its step
  obtain ⟨k, hk₁, hk₂⟩ := hE₂ δ₁ O ⟨b₂, O.concat ε⟩ ⟨ε, hR, rfl⟩
  refine ⟨⟨.seq a₁ k.g, k.O⟩, ?_, ?_⟩
  · -- run the right side from `seq a₁ (g₂ + δ₁)`
    exact liftRight (g₁ := a₁) hk₁
  · -- from `seq g₁ b₂`: replay the left step, then run the right side
    exact Star.head (cstep_seqLeft hL _) (liftRight (g₁ := a₁) hk₂)

end Seq

/-! ### Auxiliary constructions for the `par` cases -/

section Par

variable {i₁ o₁ i₂ o₂ : List Coll.{u}} {g₁ : Graph i₁ o₁} {g₂ : Graph i₂ o₂}

/-- A `parLeft` `CStep` at the configuration level, with outputs in
`append` form. -/
theorem cstep_parLeft {a : Graph i₁ o₁} {δ₁ : Vals o₁} (h : Step g₁ a δ₁)
    (A : Vals o₁) (B : Vals o₂) :
    CStep (⟨.par g₁ g₂, A.append B⟩ : Config (i₁ ++ i₂) (o₁ ++ o₂))
      ⟨.par a g₂, (A.concat δ₁).append B⟩ := by
  refine ⟨δ₁.append (Vals.empty o₂), .parLeft h, ?_⟩
  rw [← Vals.append_concat, Vals.concat_empty]

/-- A `parRight` `CStep` at the configuration level. -/
theorem cstep_parRight {b : Graph i₂ o₂} {δ₂ : Vals o₂} (h : Step g₂ b δ₂)
    (A : Vals o₁) (B : Vals o₂) :
    CStep (⟨.par g₁ g₂, A.append B⟩ : Config (i₁ ++ i₂) (o₁ ++ o₂))
      ⟨.par g₁ b, A.append (B.concat δ₂)⟩ := by
  refine ⟨(Vals.empty o₁).append δ₂, .parRight h, ?_⟩
  rw [← Vals.append_concat, Vals.concat_empty]

/-- The mixed `par` peak: steps on independent halves commute on the nose. -/
theorem par_mixed_join {a : Graph i₁ o₁} {δ₁ : Vals o₁} (hL : Step g₁ a δ₁)
    {b : Graph i₂ o₂} {δ₂ : Vals o₂} (hR : Step g₂ b δ₂)
    (A : Vals o₁) (B : Vals o₂) :
    Joinable CStep
      (⟨.par a g₂, (A.concat δ₁).append B⟩ : Config (i₁ ++ i₂) (o₁ ++ o₂))
      ⟨.par g₁ b, A.append (B.concat δ₂)⟩ :=
  ⟨⟨.par a b, (A.concat δ₁).append (B.concat δ₂)⟩,
    Star.single (cstep_parRight hR _ _), Star.single (cstep_parLeft hL _ _)⟩

end Par

/-! ### The main induction -/

/-- **Local confluence and eager execution for all lawful graphs** (the
inductive core of Lemma 2.4.3), by strong induction on graph size. -/
theorem lc_eager_of_lawful :
    ∀ (n : Nat) {i o : List Coll.{u}} (g : Graph i o),
      g.size ≤ n → g.LeavesLawful → LC g ∧ g.EagerG := by
  intro n
  induction n with
  | zero =>
    intro i o g hs _
    exact absurd (Nat.lt_of_lt_of_le (size_pos g) hs) (Nat.lt_irrefl 0)
  | succ n ihn =>
    intro i o g hs hl
    match g, hs, hl with
    | .node op st buf, _, hop =>
      constructor
      · -- LC via operator confluence
        rintro O c₁ c₂ h₁ h₂
        obtain ⟨d₁, rfl, hd₁⟩ := cstep_nodeCfg_inv (c := ⟨buf, st, O⟩) h₁
        obtain ⟨d₂, rfl, hd₂⟩ := cstep_nodeCfg_inv (c := ⟨buf, st, O⟩) h₂
        obtain ⟨e, he₁, he₂⟩ :=
          hop.confluent _ _ _ (Star.single hd₁) (Star.single hd₂)
        exact ⟨nodeCfg op e, star_cstep_nodeCfg he₁, star_cstep_nodeCfg he₂⟩
      · -- eager via operator eager execution
        rintro Δ O c' h
        obtain ⟨d, rfl, hd⟩ := cstep_nodeCfg_inv (c := ⟨buf, st, O⟩) h
        obtain ⟨e, he₁, he₂⟩ := hop.eager Δ ⟨buf, st, O⟩ d hd
        exact ⟨nodeCfg op e, star_cstep_nodeCfg he₁, star_cstep_nodeCfg he₂⟩
    | .seq (m := mid) g₁ g₂, hs, hl =>
      have hs₁ : g₁.size ≤ n := Nat.le_of_lt_succ
        (Nat.lt_of_lt_of_le (Nat.lt_add_of_pos_right (size_pos g₂)) hs)
      have hs₂ : g₂.size ≤ n := Nat.le_of_lt_succ
        (Nat.lt_of_lt_of_le (Nat.lt_add_of_pos_left (size_pos g₁)) hs)
      obtain ⟨hLC₁, hE₁⟩ := ihn g₁ hs₁ hl.1
      obtain ⟨hLC₂, hE₂⟩ := ihn g₂ hs₂ hl.2
      constructor
      · -- LC for seq
        rintro O ⟨G₁, Oa⟩ ⟨G₂, Ob⟩ ⟨δa, hsa, hOa⟩ ⟨δb, hsb, hOb⟩
        dsimp only at hOa hOb
        subst hOa hOb
        rcases step_seq_inv hsa with ⟨a₁, δ₁, hLa, rfl, rfl⟩ | ⟨b₂, hRa, rfl⟩ <;>
          rcases step_seq_inv hsb with ⟨a₁', δ₁', hLb, rfl, rfl⟩ | ⟨b₂', hRb, rfl⟩ <;>
          try simp only [Vals.concat_empty]
        · -- (L, L): left subgraph local confluence, lifted
          obtain ⟨e, he₁, he₂⟩ := hLC₁ g₂.inputs ⟨a₁, g₂.inputs.concat δ₁⟩
            ⟨a₁', g₂.inputs.concat δ₁'⟩ ⟨δ₁, hLa, rfl⟩ ⟨δ₁', hLb, rfl⟩
          exact ⟨⟨.seq e.g (g₂.setInput e.O), O⟩,
            liftLeft (g₂ := g₂) (O := O) (c := ⟨a₁, g₂.inputs.concat δ₁⟩) he₁,
            liftLeft (g₂ := g₂) (O := O) (c := ⟨a₁', g₂.inputs.concat δ₁'⟩) he₂⟩
        · -- (L, R): mixed peak via right subgraph's eager execution
          exact seq_mixed_join hE₂ hLa hRb O
        · -- (R, L): symmetric mixed peak
          exact (seq_mixed_join hE₂ hLb hRa O).symm
        · -- (R, R): right subgraph local confluence, lifted
          obtain ⟨e, he₁, he₂⟩ := hLC₂ O ⟨b₂, O.concat δa⟩ ⟨b₂', O.concat δb⟩
            ⟨δa, hRa, rfl⟩ ⟨δb, hRb, rfl⟩
          exact ⟨⟨.seq g₁ e.g, e.O⟩,
            liftRight (g₁ := g₁) (c := ⟨b₂, O.concat δa⟩) he₁,
            liftRight (g₁ := g₁) (c := ⟨b₂', O.concat δb⟩) he₂⟩
      · -- eager for seq
        rintro Δ O ⟨G', O'⟩ ⟨δ, hstep, hO⟩
        dsimp only at hO
        subst hO
        rcases step_seq_inv hstep with ⟨a₁, δ₁, hL, rfl, rfl⟩ | ⟨b₂, hR, rfl⟩
        · -- seqLeft: use the left subgraph's eager execution
          simp only [Config.addDelta, addDelta_seq, Vals.concat_empty]
          obtain ⟨e, he₁, he₂⟩ := hE₁ Δ g₂.inputs ⟨a₁, g₂.inputs.concat δ₁⟩
            ⟨δ₁, hL, rfl⟩
          refine ⟨⟨.seq e.g (g₂.setInput e.O), O⟩, ?_, ?_⟩
          · have lift := liftLeft (g₂ := g₂) (O := O)
              (c := ⟨g₁.addDelta Δ, g₂.inputs⟩) (c' := e) he₁
            simpa [setInput_inputs] using lift
          · have lift := liftLeft (g₂ := g₂) (O := O)
              (c := ⟨a₁.addDelta Δ, g₂.inputs.concat δ₁⟩) (c' := e) he₂
            simpa [addDelta_seq] using lift
        · -- seqRight: the same right step is available after the delta
          simp only [Config.addDelta, addDelta_seq]
          exact ⟨⟨.seq (g₁.addDelta Δ) b₂, O.concat δ⟩,
            Star.single (cstep_seqRight hR O), Star.refl _⟩
    | .par (i₁ := pi₁) (o₁ := po₁) (i₂ := pi₂) (o₂ := po₂) g₁ g₂, hs, hl =>
      have hs₁ : g₁.size ≤ n := Nat.le_of_lt_succ
        (Nat.lt_of_lt_of_le (Nat.lt_add_of_pos_right (size_pos g₂)) hs)
      have hs₂ : g₂.size ≤ n := Nat.le_of_lt_succ
        (Nat.lt_of_lt_of_le (Nat.lt_add_of_pos_left (size_pos g₁)) hs)
      obtain ⟨hLC₁, hE₁⟩ := ihn g₁ hs₁ hl.1
      obtain ⟨hLC₂, hE₂⟩ := ihn g₂ hs₂ hl.2
      constructor
      · -- LC for par
        rintro O ⟨G₁, Oa⟩ ⟨G₂, Ob⟩ ⟨δa, hsa, hOa⟩ ⟨δb, hsb, hOb⟩
        dsimp only at hOa hOb
        subst hOa hOb
        obtain ⟨A, B, rfl⟩ : ∃ A B, O = Vals.append A B :=
          ⟨O.split.1, O.split.2, (Vals.append_split O).symm⟩
        rcases step_par_inv hsa with ⟨a, δ₁, hLa, rfl, rfl⟩ | ⟨b, δ₂, hRa, rfl, rfl⟩ <;>
          rcases step_par_inv hsb with ⟨a', δ₁', hLb, rfl, rfl⟩ | ⟨b', δ₂', hRb, rfl, rfl⟩ <;>
          try simp only [← Vals.append_concat, Vals.concat_empty]
        · -- (L, L)
          obtain ⟨e, he₁, he₂⟩ := hLC₁ A ⟨a, A.concat δ₁⟩ ⟨a', A.concat δ₁'⟩
            ⟨δ₁, hLa, rfl⟩ ⟨δ₁', hLb, rfl⟩
          exact ⟨⟨.par e.g g₂, e.O.append B⟩,
            liftParLeft (g₂ := g₂) (O₂ := B) (c := ⟨a, A.concat δ₁⟩) he₁,
            liftParLeft (g₂ := g₂) (O₂ := B) (c := ⟨a', A.concat δ₁'⟩) he₂⟩
        · -- (L, R)
          exact par_mixed_join hLa hRb A B
        · -- (R, L)
          exact (par_mixed_join hLb hRa A B).symm
        · -- (R, R)
          obtain ⟨e, he₁, he₂⟩ := hLC₂ B ⟨b, B.concat δ₂⟩ ⟨b', B.concat δ₂'⟩
            ⟨δ₂, hRa, rfl⟩ ⟨δ₂', hRb, rfl⟩
          exact ⟨⟨.par g₁ e.g, A.append e.O⟩,
            liftParRight (g₁ := g₁) (O₁ := A) (c := ⟨b, B.concat δ₂⟩) he₁,
            liftParRight (g₁ := g₁) (O₁ := A) (c := ⟨b', B.concat δ₂'⟩) he₂⟩
      · -- eager for par
        rintro Δ O ⟨G', O'⟩ ⟨δ, hstep, hO⟩
        dsimp only at hO
        subst hO
        obtain ⟨A, B, rfl⟩ : ∃ A B, O = Vals.append A B :=
          ⟨O.split.1, O.split.2, (Vals.append_split O).symm⟩
        obtain ⟨Δ₁, Δ₂, rfl⟩ : ∃ Δ₁ Δ₂, Δ = Vals.append Δ₁ Δ₂ :=
          ⟨Δ.split.1, Δ.split.2, (Vals.append_split Δ).symm⟩
        rcases step_par_inv hstep with ⟨a, δ₁, hL, rfl, rfl⟩ | ⟨b, δ₂, hR, rfl, rfl⟩
        · -- parLeft: left subgraph's eager execution, lifted
          simp only [Config.addDelta, addDelta_par, ← Vals.append_concat,
            Vals.concat_empty]
          obtain ⟨e, he₁, he₂⟩ := hE₁ Δ₁ A ⟨a, A.concat δ₁⟩ ⟨δ₁, hL, rfl⟩
          exact ⟨⟨.par e.g (g₂.addDelta Δ₂), e.O.append B⟩,
            liftParLeft (g₂ := g₂.addDelta Δ₂) (O₂ := B)
              (c := ⟨g₁.addDelta Δ₁, A⟩) he₁,
            liftParLeft (g₂ := g₂.addDelta Δ₂) (O₂ := B)
              (c := ⟨a.addDelta Δ₁, A.concat δ₁⟩) he₂⟩
        · -- parRight: right subgraph's eager execution, lifted
          simp only [Config.addDelta, addDelta_par, ← Vals.append_concat,
            Vals.concat_empty]
          obtain ⟨e, he₁, he₂⟩ := hE₂ Δ₂ B ⟨b, B.concat δ₂⟩ ⟨δ₂, hR, rfl⟩
          exact ⟨⟨.par (g₁.addDelta Δ₁) e.g, A.append e.O⟩,
            liftParRight (g₁ := g₁.addDelta Δ₁) (O₁ := A)
              (c := ⟨g₂.addDelta Δ₂, B⟩) he₁,
            liftParRight (g₁ := g₁.addDelta Δ₁) (O₁ := A)
              (c := ⟨b.addDelta Δ₂, B.concat δ₂⟩) he₂⟩

end Graph

end HydroLean
