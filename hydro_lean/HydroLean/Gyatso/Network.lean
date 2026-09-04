import HydroLean.Gyatso.LocalColl
import HydroLean.Gyatso.Op1
import HydroLean.Gyatso.Location

/-!
# Gyatso network operators (dissertation §3.5)

Network operators move streams between locations while *modeling* — not
hiding — the nondeterminism of real transports. Each operator keeps the
in-transit messages in its state and takes separate send/receive steps, so
message latency is captured by *when* the scheduler runs those steps relative
to other operators (§3.5's "no-op-free" latency modeling). Each operator is a
Flo `Operator` satisfying every obligation of §2.3.4, so composing it into a
graph preserves eventual determinism end-to-end (§3.4.3).

- `networkO2o` (Fig 3.7): process-to-process FIFO transport (TCP): ordered
  sequences in, ordered sequences out, exactly-once, order preserved.
- `networkO2oUnord` (Fig 3.10): reliable-unordered transport (e.g. QUIC
  streams): the *collection type* `[T]unord` (a quotient by permutation)
  absorbs delivery reordering, so the operator is deterministic *into the
  quotient* — the type-theoretic heart of Gyatso.
- `foldCommutative` (Fig 3.11): the computational operator that consumes an
  unordered stream; its closure must be commutative, which is literally the
  congruence condition making the fold well-defined on the quotient. This is
  the operational-layer justification for Hydro's `fold_commutative` API: the
  Rust `manual_proof!(commutative)` obligation is exactly the `hcomm`
  hypothesis of `foldCommutative_lawful`.

All three follow one design: the carrier is a `flagged` collection (payload +
terminator flag); the operator records terminator-consumption in a `done` bit,
tied to the configuration by an `Inv` ensuring `done` implies the terminator
was really seen and forwarded. Output maximality is discharged generically by
`Op1.outputsMaximal_of_fixDelta` since fixing a flagged input is just
concatenating the terminator delta.
-/

namespace HydroLean.Gyatso

universe u

variable {α : Type u}

/-! ## Computation lemmas for the flagged carriers -/

@[simp] theorem seqC_concat_live (l : List α) (δ : List α × Bool) :
    (seqC α).concat (l, false) δ = (l ++ δ.1, δ.2) := rfl

@[simp] theorem seqC_concat_fixed (l : List α) (δ : List α × Bool) :
    (seqC α).concat (l, true) δ = (l, true) := rfl

@[simp] theorem msetC_concat_live (m : Mset α) (δ : Mset α × Bool) :
    (msetC α).concat (m, false) δ = (m.union δ.1, δ.2) := rfl

@[simp] theorem msetC_concat_fixed (m : Mset α) (δ : Mset α × Bool) :
    (msetC α).concat (m, true) δ = (m, true) := rfl

@[simp] theorem singC_concat_live {β : Type u} (v : Option β) (δ : Option β × Bool) :
    (singC β).concat (v, false) δ = (overwrite v δ.1, δ.2) := rfl

@[simp] theorem singC_concat_fixed {β : Type u} (v : Option β) (δ : Option β × Bool) :
    (singC β).concat (v, true) δ = (v, true) := rfl

@[simp] theorem seqC_concat_emptyδ (c : (seqC α).C) :
    (seqC α).concat c ([], false) = c := (seqC α).concat_empty c

@[simp] theorem msetC_concat_emptyδ (c : (msetC α).C) :
    (msetC α).concat c (Mset.mk [], false) = c := (msetC α).concat_empty c

@[simp] theorem singC_concat_emptyδ {β : Type u} (c : (singC β).C) :
    (singC β).concat c (none, false) = c := (singC β).concat_empty c

/-! ## `networkO2o` (Fig 3.7): FIFO process-to-process transport -/

/-- Network operator state: in-transit messages (FIFO queue) and whether the
terminator has been forwarded. -/
structure O2oSt (α : Type u) : Type u where
  buf : List α
  done : Bool

/-- Small-steps of `networkO2o` (Fig 3.7): `send` moves the oldest unsent input
element into the in-transit queue; `recv` delivers the oldest in-transit
element; `term` forwards the stream terminator once everything is delivered.
Latency is modeled by when the scheduler runs `recv` (§3.5.1). -/
inductive o2oStep :
    (List α × Bool) → O2oSt α → (List α × Bool) → O2oSt α → (List α × Bool) → Prop where
  | send {a : α} {rest : List α} {fl : Bool} {buf : List α} {done : Bool} :
      o2oStep (a :: rest, fl) ⟨buf, done⟩ (rest, fl) ⟨buf ++ [a], done⟩ ([], false)
  | recv {I : List α × Bool} {h : α} {t : List α} {done : Bool} :
      o2oStep I ⟨h :: t, done⟩ I ⟨t, done⟩ ([h], false)
  | term : o2oStep ([], true) ⟨[], false⟩ ([], true) ⟨[], true⟩ ([], true)

/-- Process-to-process FIFO networking (Fig 3.7). Observationally an identity
on ordered sequences (§3.5.1: "observationally equivalent to not having any
operator at all"), so it can be inserted at any collection-compatible edge to
split a graph across locations. -/
def networkO2o (α : Type u) : Op1 (seqC α) (seqC α) where
  State := O2oSt α
  step := o2oStep
  Inv I s O := s.done = true → I = ([], true) ∧ O.2 = true

namespace networkO2o

/-- Termination measure: unsent elements count double (they must be sent then
received), in-transit count once, plus one pending terminator forward. -/
def measure (p : O2oSt α × (List α × Bool)) : Nat :=
  2 * p.2.1.length + p.1.buf.length + (if p.1.done then 0 else 1)

theorem wf_decreasing :
    ∃ r : ((networkO2o α).State × (seqC α).C) → ((networkO2o α).State × (seqC α).C) → Prop,
      WellFounded r ∧
        ∀ {I s I' s' δ}, (networkO2o α).step I s I' s' δ → r (s', I') (s, I) := by
  refine ⟨InvImage (· < ·) measure, InvImage.wf _ (Nat.lt_wfRel.wf), ?_⟩
  intro I s I' s' δ hs
  show measure _ < measure _
  cases hs with
  | @send a rest fl buf done => cases done <;> simp [measure] <;> omega
  | @recv I h t done => cases done <;> simp [measure] <;> omega
  | term => simp [measure]

/-- Configurations are equal when their fields are. -/
theorem config_ext {c c' : (networkO2o α).Config1}
    (hI : c.I = c'.I) (hst : c.st = c'.st) (hO : c.O = c'.O) : c = c' := by
  cases c; cases c'; simp_all

theorem local_confluent : LocallyConfluent (networkO2o α).OpStep1 := by
  rintro ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ ⟨⟨Ip₁, Ifl₁⟩, ⟨buf₁, done₁⟩, O₁⟩ ⟨⟨Ip₂, Ifl₂⟩, ⟨buf₂, done₂⟩, O₂⟩
    ⟨δ₁, hs₁, hO₁⟩ ⟨δ₂, hs₂, hO₂⟩
  simp only at hO₁ hO₂
  cases hs₁ with
  | @send a _ _ _ _ =>
    cases hs₂ with
    | send =>
      subst hO₁ hO₂
      exact joinable_of_eq rfl
    | @recv _ h _ _ =>
      -- send a / recv h commute; the in-transit queue after both is buf₂ ++ [a]
      subst hO₁ hO₂
      refine ⟨⟨(Ip₁, Ifl), ⟨buf₂ ++ [a], done⟩, (seqC α).concat O ([h], false)⟩, ?_, ?_⟩
      · exact Star.single ⟨([h], false), o2oStep.recv, by simp⟩
      · exact Star.single ⟨([], false), o2oStep.send, by simp⟩
  | @recv _ h _ _ =>
    cases hs₂ with
    | @send a _ _ _ _ =>
      subst hO₁ hO₂
      refine ⟨⟨(Ip₂, Ifl), ⟨buf₁ ++ [a], done⟩, (seqC α).concat O ([h], false)⟩, ?_, ?_⟩
      · exact Star.single ⟨([], false), o2oStep.send, by simp⟩
      · exact Star.single ⟨([h], false), o2oStep.recv, by simp⟩
    | recv =>
      subst hO₁ hO₂
      exact joinable_of_eq rfl
  | term =>
    cases hs₂ with
    | term =>
      subst hO₁ hO₂
      exact joinable_of_eq rfl

theorem eager : (networkO2o α).Eager1 := by
  rintro ⟨Δp, Δfl⟩ ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ ⟨⟨Ip', Ifl'⟩, ⟨buf', done'⟩, O'⟩ ⟨δ, hs, hO⟩
  simp only at hO
  cases hs with
  | @send a _ _ _ _ =>
    cases Ifl with
    | false =>
      subst hO
      apply joinable_of_step
      refine ⟨([], false), ?_, rfl⟩
      show o2oStep (a :: (Ip' ++ Δp), Δfl) ⟨buf, done⟩ (Ip' ++ Δp, Δfl)
        ⟨buf ++ [a], done⟩ ([], false)
      exact o2oStep.send
    | true =>
      subst hO
      apply joinable_of_step
      refine ⟨([], false), ?_, rfl⟩
      show o2oStep (a :: Ip', true) ⟨buf, done⟩ (Ip', true) ⟨buf ++ [a], done⟩ ([], false)
      exact o2oStep.send
  | @recv _ h _ _ =>
    subst hO
    apply joinable_of_step
    refine ⟨([h], false), ?_, rfl⟩
    show o2oStep ((seqC α).concat (Ip, Ifl) (Δp, Δfl)) ⟨h :: buf', done⟩
      ((seqC α).concat (Ip, Ifl) (Δp, Δfl)) ⟨buf', done⟩ ([h], false)
    exact o2oStep.recv
  | term =>
    subst hO
    apply joinable_of_step
    refine ⟨([], true), ?_, rfl⟩
    show o2oStep ([], true) ⟨[], false⟩ ([], true) ⟨[], true⟩ ([], true)
    exact o2oStep.term

theorem inv_step {c c' : (networkO2o α).Config1}
    (hinv : (networkO2o α).Inv c.I c.st c.O) (hs : (networkO2o α).OpStep1 c c') :
    (networkO2o α).Inv c'.I c'.st c'.O := by
  obtain ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ := c
  obtain ⟨⟨Ip', Ifl'⟩, ⟨buf', done'⟩, O'⟩ := c'
  obtain ⟨δ, hstep, hO⟩ := hs
  simp only at hO hinv ⊢
  cases hstep with
  | send =>
    intro hdone
    exact absurd (congrArg Prod.fst (hinv hdone).1) (List.cons_ne_nil _ _)
  | recv =>
    intro hdone
    obtain ⟨hI, hOfl⟩ := hinv hdone
    refine ⟨hI, ?_⟩
    subst hO
    cases O with
    | mk v fl => cases fl with
      | true => rfl
      | false => simp_all
  | term =>
    intro _
    refine ⟨rfl, ?_⟩
    subst hO
    cases O with
    | mk v fl => cases fl <;> rfl

theorem inv_delta {I : (seqC α).C} {s : (networkO2o α).State} {O : (seqC α).C}
    (Δ : (seqC α).C) (hinv : (networkO2o α).Inv I s O) :
    (networkO2o α).Inv ((seqC α).concat I Δ) s O := by
  intro hdone
  obtain ⟨hI, hO⟩ := hinv hdone
  subst hI
  exact ⟨rfl, hO⟩

/-- Steps never change the input's fixedness flag. -/
theorem flag_invariant {c c' : (networkO2o α).Config1}
    (hs : (networkO2o α).OpStep1 c c') : c.I.2 = c'.I.2 := by
  obtain ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ := c
  obtain ⟨⟨Ip', Ifl'⟩, ⟨buf', done'⟩, O'⟩ := c'
  obtain ⟨δ, hstep, _⟩ := hs
  cases hstep <;> rfl

theorem flag_invariant_star {c c' : (networkO2o α).Config1}
    (hs : Star (networkO2o α).OpStep1 c c') : c.I.2 = c'.I.2 := by
  induction hs with
  | refl => rfl
  | tail _ hbc ih => exact ih.trans (flag_invariant hbc)

/-- The finisher for generic output maximality: from any consistent stuck
state, delivering the terminator delta forwards it once and fixes the output.
This is the only operator-specific content of Def 2.3.2/2.3.3. -/
theorem finish (f : (networkO2o α).Config1)
    (hinv : (networkO2o α).Inv f.I f.st f.O)
    (hstuck : Stuck (networkO2o α).OpStep1 f) :
    ∃ z : (networkO2o α).Config1,
      NormalizesTo (networkO2o α).OpStep1 (f.addDelta ([], true)) z ∧
        z.O = (seqC α).fix f.O := by
  obtain ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ := f
  simp only at hinv ⊢
  -- the payload must be drained
  cases Ip with
  | cons a rest =>
    exact absurd
      ⟨⟨(rest, Ifl), ⟨buf ++ [a], done⟩, (seqC α).concat O ([], false)⟩,
        ([], false), o2oStep.send, rfl⟩ hstuck
  | nil =>
  -- the in-transit queue must be drained
  cases buf with
  | cons h t =>
    exact absurd
      ⟨⟨([], Ifl), ⟨t, done⟩, (seqC α).concat O ([h], false)⟩,
        ([h], false), o2oStep.recv, rfl⟩ hstuck
  | nil =>
  cases done with
  | true =>
    -- terminator already forwarded: everything is already fixed
    obtain ⟨hI, hO⟩ := hinv rfl
    have hIfl : Ifl = true := congrArg Prod.snd hI
    subst hIfl
    refine ⟨⟨([], true), ⟨[], true⟩, O⟩, ⟨Star.refl _, ?_⟩, ?_⟩
    · show Stuck _ _
      rintro ⟨d, δ, hstep, -⟩
      cases hstep
    · cases hOc : O with
      | mk v fl =>
        have : fl = true := by rw [hOc] at hO; exact hO
        subst this
        rfl
  | false =>
    cases Ifl with
    | true =>
      -- term step was available: contradicts stuckness
      exact absurd
        ⟨⟨([], true), ⟨[], true⟩, (seqC α).concat O ([], true)⟩,
          ([], true), o2oStep.term, rfl⟩ hstuck
    | false =>
      -- deliver the terminator, then forward it
      refine ⟨⟨([], true), ⟨[], true⟩, (seqC α).concat O ([], true)⟩,
        ⟨Star.single ⟨([], true), ?_, rfl⟩, ?_⟩, ?_⟩
      · show o2oStep (([] : List α) ++ [], true) ⟨[], false⟩ ([], true) ⟨[], true⟩ ([], true)
        exact o2oStep.term
      · rintro ⟨d, δ, hstep, -⟩
        cases hstep
      · cases O with
        | mk v fl => cases fl <;> (simp; rfl)

theorem progress (b : Boundedness) : (networkO2o α).Progress1 b b := by
  intro c f hinv hfix hnorm
  have hfinv : (networkO2o α).Inv f.I f.st f.O :=
    Op1.inv_star (fun h hs => inv_step h hs) hnorm.1 hinv
  constructor
  · exact Op1.outputsMaximal_of_fixDelta eager
      (newman (Op1.sn_of_wf wf_decreasing) local_confluent)
      (fun h hs => inv_step h hs)
      (fun I => flagged_fix_eq_concat_term I) finish hinv hnorm
  · intro hb
    -- bounded ports: the input is fixed, so the run must end terminated
    have hcfix : (seqC α).Fixed c.I := hfix hb
    have hflag : c.I.2 = true := (flagged_fixed_iff c.I).mp hcfix
    have hfflag : f.I.2 = true := (flag_invariant_star hnorm.1) ▸ hflag
    -- at the stuck state, done must be true (else term or send applies)
    obtain ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ := f
    simp only at hfflag hfinv
    subst hfflag
    have hstuck := hnorm.2
    cases Ip with
    | cons a rest =>
      exact absurd
        ⟨⟨(rest, true), ⟨buf ++ [a], done⟩, (seqC α).concat O ([], false)⟩,
          ([], false), o2oStep.send, rfl⟩ hstuck
    | nil =>
    cases buf with
    | cons h t =>
      exact absurd
        ⟨⟨([], true), ⟨t, done⟩, (seqC α).concat O ([h], false)⟩,
          ([h], false), o2oStep.recv, rfl⟩ hstuck
    | nil =>
    cases done with
    | false =>
      exact absurd
        ⟨⟨([], true), ⟨[], true⟩, (seqC α).concat O ([], true)⟩,
          ([], true), o2oStep.term, rfl⟩ hstuck
    | true =>
      have := (hfinv rfl).2
      exact (flagged_fixed_iff O).mpr this

/-- `networkO2o` satisfies every Flo operator obligation, at matching
boundedness `b ↩→ b` (Fig 3.7 uses `U ↩→ U`; the bounded instance also holds
because the operator forwards termination). -/
theorem lawful (b : Boundedness) : (networkO2o α).Lawful1 b b where
  wf_decreasing := wf_decreasing
  confluent := newman (Op1.sn_of_wf wf_decreasing) local_confluent
  eager := eager
  progress := progress b
  inv_step := fun h hs => inv_step h hs
  inv_delta := fun Δ h => inv_delta Δ h

end networkO2o

/-- The Flo operator for FIFO process-to-process networking, with its full
lawfulness certificate: composable into any graph with all guarantees
(Lemmas 2.4.2–2.4.4) preserved. -/
theorem networkO2o_lawful (b : Boundedness) :
    ((networkO2o α).toOperator b b).Lawful :=
  Op1.toOperator_lawful (networkO2o.lawful b)



/-! ## `foldCommutative` (Fig 3.11): aggregating an unordered stream -/

/-- Fold state: the running accumulator and whether the terminator has been
consumed and forwarded. -/
structure FoldSt (β : Type u) : Type u where
  acc : β
  done : Bool

/-- Small-steps of `foldCommutative` (Fig 3.11): `consume` folds one
(nondeterministically chosen) element of the unordered input into the
accumulator and publishes the new running value; `term` forwards the
terminator, fixing the output singleton. Element choice is phrased by an
equational premise exposing a list representative of the multiset. -/
inductive foldStep {α β : Type u} (f : β → α → β) :
    (Mset α × Bool) → FoldSt β → (Mset α × Bool) → FoldSt β →
    (Option β × Bool) → Prop where
  | consume {a : α} {l : List α} {m : Mset α} {fl : Bool} {acc : β} {done : Bool} :
      m = Mset.mk (a :: l) →
      foldStep f (m, fl) ⟨acc, done⟩ (Mset.mk l, fl) ⟨f acc a, done⟩
        (some (f acc a), false)
  | term {acc : β} :
      foldStep f (Mset.mk [], true) ⟨acc, false⟩ (Mset.mk [], true) ⟨acc, true⟩
        (none, true)

/-- The `fold_commutative` operator (Fig 3.11): folds an unordered stream into
an overwrite singleton. **This is the operational-layer justification for
Hydro's `Stream::fold_commutative` API**: the commutativity `manual_proof!`
obligation in Rust is exactly the `hcomm` hypothesis of
`foldCommutative.lawful`, and it is what makes the fold a *congruence on the
permutation quotient*, so upstream network nondeterminism cannot leak into the
settled output (§3.5.2). -/
def foldCommutative {α β : Type u} (f : β → α → β) : Op1 (msetC α) (singC β) where
  State := FoldSt β
  step := foldStep f
  Inv I s O := s.done = true → I = (Mset.mk [], true) ∧ O.2 = true

namespace foldCommutative

variable {α β : Type u} {f : β → α → β}

def measure (p : FoldSt β × (Mset α × Bool)) : Nat :=
  Mset.card p.2.1 + (if p.1.done then 0 else 1)

theorem wf_decreasing :
    ∃ r : ((foldCommutative (α := α) f).State × (msetC α).C) →
          ((foldCommutative (α := α) f).State × (msetC α).C) → Prop,
      WellFounded r ∧
        ∀ {I s I' s' δ}, (foldCommutative f).step I s I' s' δ → r (s', I') (s, I) := by
  refine ⟨InvImage (· < ·) measure, InvImage.wf _ (Nat.lt_wfRel.wf), ?_⟩
  intro I s I' s' δ hs
  show measure _ < measure _
  cases hs with
  | @consume a l m fl acc done hm =>
    subst hm
    cases done <;> simp [measure] <;> omega
  | term => simp [measure]

theorem local_confluent [DecidableEq α] (hcomm : ∀ s a b, f (f s a) b = f (f s b) a) :
    LocallyConfluent (foldCommutative (α := α) f).OpStep1 := by
  rintro ⟨⟨Ip, Ifl⟩, ⟨acc, done⟩, O⟩ ⟨⟨Ip₁, Ifl₁⟩, ⟨acc₁, done₁⟩, O₁⟩
    ⟨⟨Ip₂, Ifl₂⟩, ⟨acc₂, done₂⟩, O₂⟩ ⟨δ₁, hs₁, hO₁⟩ ⟨δ₂, hs₂, hO₂⟩
  simp only at hO₁ hO₂
  cases hs₁ with
  | @consume a l₁ _ _ _ _ hm₁ =>
    cases hs₂ with
    | @consume b l₂ _ _ _ _ hm₂ =>
      subst hO₁ hO₂
      rcases Mset.cons_diamond (hm₁ ▸ hm₂) with ⟨rfl, hll⟩ | ⟨l'', h₁, h₂⟩
      · exact joinable_of_eq (by rw [hll])
      · -- consume the other element on each side
        refine ⟨⟨(Mset.mk l'', Ifl), ⟨f (f acc a) b, done⟩,
          (singC β).concat ((singC β).concat O (some (f acc a), false))
            (some (f (f acc a) b), false)⟩, ?_, ?_⟩
        · exact Star.single ⟨_, foldStep.consume h₁, rfl⟩
        · rw [hcomm acc a b]
          refine Star.single ⟨_, foldStep.consume h₂, ?_⟩
          cases O with
          | mk v ofl => cases ofl <;> rfl
    | term =>
      exact absurd hm₁.symm Mset.mk_cons_ne_nil
  | term =>
    cases hs₂ with
    | @consume b l₂ _ _ _ _ hm₂ =>
      exact absurd hm₂.symm Mset.mk_cons_ne_nil
    | term =>
      subst hO₁ hO₂
      exact joinable_of_eq rfl

theorem eager : (foldCommutative (α := α) f).Eager1 := by
  rintro ⟨Δm, Δfl⟩ ⟨⟨Ip, Ifl⟩, ⟨acc, done⟩, O⟩ ⟨⟨Ip', Ifl'⟩, ⟨acc', done'⟩, O'⟩ ⟨δ, hs, hO⟩
  simp only at hO
  cases hs with
  | @consume a l _ _ _ _ hm =>
    cases Ifl with
    | false =>
      subst hO hm
      apply joinable_of_step
      induction Δm using Mset.ind with
      | h δrep =>
        refine ⟨_, ?_, rfl⟩
        show foldStep f (Mset.union (Mset.mk (a :: l)) (Mset.mk δrep), Δfl) ⟨acc, done⟩
          (Mset.union (Mset.mk l) (Mset.mk δrep), Δfl) ⟨f acc a, done⟩ (some (f acc a), false)
        rw [Mset.union_mk, Mset.union_mk]
        exact foldStep.consume rfl
    | true =>
      subst hO
      exact joinable_of_step ⟨_, foldStep.consume hm, rfl⟩
  | term =>
    subst hO
    exact joinable_of_step ⟨_, foldStep.term, rfl⟩

theorem inv_step {c c' : (foldCommutative (α := α) f).Config1}
    (hinv : (foldCommutative f).Inv c.I c.st c.O)
    (hs : (foldCommutative f).OpStep1 c c') :
    (foldCommutative f).Inv c'.I c'.st c'.O := by
  obtain ⟨⟨Ip, Ifl⟩, ⟨acc, done⟩, O⟩ := c
  obtain ⟨⟨Ip', Ifl'⟩, ⟨acc', done'⟩, O'⟩ := c'
  obtain ⟨δ, hstep, hO⟩ := hs
  simp only at hO hinv ⊢
  cases hstep with
  | @consume a l _ _ _ _ hm =>
    intro hdone
    have := (hinv hdone).1
    rw [hm] at this
    exact absurd (congrArg Prod.fst this) Mset.mk_cons_ne_nil
  | term =>
    intro _
    refine ⟨rfl, ?_⟩
    subst hO
    cases O with
    | mk v ofl => cases ofl <;> rfl

theorem inv_delta {I : (msetC α).C} {s : (foldCommutative (α := α) f).State}
    {O : (singC β).C} (Δ : (msetC α).C)
    (hinv : (foldCommutative f).Inv I s O) :
    (foldCommutative f).Inv ((msetC α).concat I Δ) s O := by
  intro hdone
  obtain ⟨hI, hO⟩ := hinv hdone
  subst hI
  exact ⟨rfl, hO⟩

theorem flag_invariant {c c' : (foldCommutative (α := α) f).Config1}
    (hs : (foldCommutative f).OpStep1 c c') : c.I.2 = c'.I.2 := by
  obtain ⟨⟨Ip, Ifl⟩, ⟨acc, done⟩, O⟩ := c
  obtain ⟨⟨Ip', Ifl'⟩, ⟨acc', done'⟩, O'⟩ := c'
  obtain ⟨δ, hstep, _⟩ := hs
  cases hstep <;> rfl

theorem flag_invariant_star {c c' : (foldCommutative (α := α) f).Config1}
    (hs : Star (foldCommutative f).OpStep1 c c') : c.I.2 = c'.I.2 := by
  induction hs with
  | refl => rfl
  | tail _ hbc ih => exact ih.trans (flag_invariant hbc)

theorem finish (fc : (foldCommutative (α := α) f).Config1)
    (hinv : (foldCommutative f).Inv fc.I fc.st fc.O)
    (hstuck : Stuck (foldCommutative f).OpStep1 fc) :
    ∃ z : (foldCommutative f).Config1,
      NormalizesTo (foldCommutative f).OpStep1 (fc.addDelta (Mset.mk [], true)) z ∧
        z.O = (singC β).fix fc.O := by
  obtain ⟨⟨Ip, Ifl⟩, ⟨acc, done⟩, O⟩ := fc
  simp only at hinv ⊢
  -- the payload must be drained
  rcases Mset.eq_nil_or_cons Ip with rfl | ⟨a, l, rfl⟩
  case inr =>
    exact absurd
      ⟨⟨(Mset.mk l, Ifl), ⟨f acc a, done⟩, (singC β).concat O (some (f acc a), false)⟩,
        _, foldStep.consume rfl, rfl⟩ hstuck
  cases done with
  | true =>
    obtain ⟨hI, hO⟩ := hinv rfl
    have hIfl : Ifl = true := congrArg Prod.snd hI
    subst hIfl
    refine ⟨⟨(Mset.mk [], true), ⟨acc, true⟩, O⟩, ⟨Star.refl _, ?_⟩, ?_⟩
    · rintro ⟨⟨⟨dIp, dIfl⟩, ⟨dacc, ddone⟩, dO⟩, δ, hstep, -⟩
      cases hstep with
      | consume hm => exact absurd hm.symm Mset.mk_cons_ne_nil
    · cases hOc : O with
      | mk v ofl =>
        have : ofl = true := by rw [hOc] at hO; exact hO
        subst this
        rfl
  | false =>
    cases Ifl with
    | true =>
      exact absurd
        ⟨⟨(Mset.mk [], true), ⟨acc, true⟩, (singC β).concat O (none, true)⟩,
          _, foldStep.term, rfl⟩ hstuck
    | false =>
      refine ⟨⟨(Mset.mk [], true), ⟨acc, true⟩, (singC β).concat O (none, true)⟩,
        ⟨Star.single ⟨(none, true), ?_, rfl⟩, ?_⟩, ?_⟩
      · show foldStep f (Mset.union (Mset.mk []) (Mset.mk []), true) ⟨acc, false⟩
          (Mset.mk [], true) ⟨acc, true⟩ (none, true)
        rw [Mset.union_mk]
        exact foldStep.term
      · rintro ⟨⟨⟨dIp, dIfl⟩, ⟨dacc, ddone⟩, dO⟩, δ, hstep, -⟩
        cases hstep with
        | consume hm => exact absurd hm.symm Mset.mk_cons_ne_nil
      · cases O with
        | mk v ofl => cases ofl <;> rfl

theorem progress [DecidableEq α] (hcomm : ∀ s a b, f (f s a) b = f (f s b) a)
    (b : Boundedness) : (foldCommutative (α := α) f).Progress1 b b := by
  intro c fc hinv hfix hnorm
  have hfinv : (foldCommutative f).Inv fc.I fc.st fc.O :=
    Op1.inv_star (fun h hs => inv_step h hs) hnorm.1 hinv
  constructor
  · exact Op1.outputsMaximal_of_fixDelta eager
      (newman (Op1.sn_of_wf wf_decreasing) (local_confluent hcomm))
      (fun h hs => inv_step h hs)
      (fun I => flagged_fix_eq_concat_term I) finish hinv hnorm
  · intro hb
    have hcfix : (msetC α).Fixed c.I := hfix hb
    have hflag : c.I.2 = true := (flagged_fixed_iff c.I).mp hcfix
    have hfflag : fc.I.2 = true := (flag_invariant_star hnorm.1) ▸ hflag
    obtain ⟨⟨Ip, Ifl⟩, ⟨acc, done⟩, O⟩ := fc
    simp only at hfflag hfinv
    subst hfflag
    have hstuck := hnorm.2
    rcases Mset.eq_nil_or_cons Ip with rfl | ⟨a, l, rfl⟩
    case inr =>
      exact absurd
        ⟨⟨(Mset.mk l, true), ⟨f acc a, done⟩, (singC β).concat O (some (f acc a), false)⟩,
          _, foldStep.consume rfl, rfl⟩ hstuck
    cases done with
    | false =>
      exact absurd
        ⟨⟨(Mset.mk [], true), ⟨acc, true⟩, (singC β).concat O (none, true)⟩,
          _, foldStep.term, rfl⟩ hstuck
    | true =>
      exact (flagged_fixed_iff O).mpr (hfinv rfl).2

/-- `foldCommutative` satisfies every Flo operator obligation given
commutativity of the closure — the formal counterpart of Hydro's
`fold_commutative` with its `manual_proof!(/** commutative */)`. -/
theorem lawful [DecidableEq α] (hcomm : ∀ s a b, f (f s a) b = f (f s b) a)
    (b : Boundedness) : (foldCommutative (α := α) f).Lawful1 b b where
  wf_decreasing := wf_decreasing
  confluent := newman (Op1.sn_of_wf wf_decreasing) (local_confluent hcomm)
  eager := eager
  progress := progress hcomm b
  inv_step := fun h hs => inv_step h hs
  inv_delta := fun Δ h => inv_delta Δ h

end foldCommutative

/-- The Flo operator for commutative folds over unordered streams, fully
lawful given commutativity. -/
theorem foldCommutative_lawful {α β : Type u} [DecidableEq α] {f : β → α → β}
    (hcomm : ∀ s a b, f (f s a) b = f (f s b) a) (b : Boundedness) :
    ((foldCommutative (α := α) f).toOperator b b).Lawful :=
  Op1.toOperator_lawful (foldCommutative.lawful hcomm b)

/-! ## `networkO2oUnord` (Fig 3.10): reliable-unordered transport -/

/-- Unordered network state: in-transit messages as a *multiset* (delivery
order is unobservable downstream, so we quotient it away here too) and the
terminator bit. -/
structure UnordSt (α : Type u) : Type u where
  buf : Mset α
  done : Bool

/-- Small-steps of `networkO2oUnord` (Fig 3.10): `send` nondeterministically
picks any element of the unordered input to put in flight; `recv` delivers any
in-flight element; `term` forwards the terminator once drained. -/
inductive unordStep {α : Type u} :
    (Mset α × Bool) → UnordSt α → (Mset α × Bool) → UnordSt α →
    (Mset α × Bool) → Prop where
  | send {a : α} {l : List α} {m : Mset α} {fl : Bool} {buf : Mset α} {done : Bool} :
      m = Mset.mk (a :: l) →
      unordStep (m, fl) ⟨buf, done⟩ (Mset.mk l, fl)
        ⟨buf.union (Mset.mk [a]), done⟩ (Mset.mk [], false)
  | recv {I : Mset α × Bool} {h : α} {t : List α} {buf : Mset α} {done : Bool} :
      buf = Mset.mk (h :: t) →
      unordStep I ⟨buf, done⟩ I ⟨Mset.mk t, done⟩ (Mset.mk [h], false)
  | term : unordStep (Mset.mk [], true) ⟨Mset.mk [], false⟩ (Mset.mk [], true)
      ⟨Mset.mk [], true⟩ (Mset.mk [], true)

/-- Reliable-unordered process-to-process networking (Fig 3.10). The
*collection type* `[T]unord` (a permutation quotient) absorbs the delivery
reordering, so this operator is deterministic *into the quotient* — the sender
and receiver nondeterminism (`send`/`recv` picking any element) is provably
unobservable (§3.5.2). -/
def networkO2oUnord (α : Type u) : Op1 (msetC α) (msetC α) where
  State := UnordSt α
  step := unordStep
  Inv I s O := s.done = true → I = (Mset.mk [], true) ∧ O.2 = true

namespace networkO2oUnord

variable {α : Type u}

def measure (p : UnordSt α × (Mset α × Bool)) : Nat :=
  2 * Mset.card p.2.1 + Mset.card p.1.buf + (if p.1.done then 0 else 1)

theorem wf_decreasing :
    ∃ r : ((networkO2oUnord α).State × (msetC α).C) →
          ((networkO2oUnord α).State × (msetC α).C) → Prop,
      WellFounded r ∧
        ∀ {I s I' s' δ}, (networkO2oUnord α).step I s I' s' δ → r (s', I') (s, I) := by
  refine ⟨InvImage (· < ·) measure, InvImage.wf _ (Nat.lt_wfRel.wf), ?_⟩
  intro I s I' s' δ hs
  show measure _ < measure _
  cases hs with
  | @send a l m fl buf done hm =>
    subst hm
    have : Mset.card (buf.union (Mset.mk [a])) = Mset.card buf + 1 := by
      induction buf using Mset.ind with
      | h bl => simp
    cases done <;> simp [measure, this] <;> omega
  | @recv I h t buf done hbuf =>
    subst hbuf
    cases done <;> simp [measure] <;> omega
  | term => simp [measure]

/-- Output-buffer commutation for unordered emissions: emitting two multiset
deltas in either order yields the same buffer (union is commutative on the
quotient) — the counting argument of §3.5.2. -/
theorem out_comm (O : (msetC α).C) (x y : Mset α) :
    (msetC α).concat ((msetC α).concat O (x, false)) (y, false) =
      (msetC α).concat ((msetC α).concat O (y, false)) (x, false) := by
  cases O with
  | mk v ofl =>
    cases ofl with
    | true => rfl
    | false =>
      show ((v.union x).union y, false) = ((v.union y).union x, false)
      rw [Mset.union_right_comm]

theorem local_confluent [DecidableEq α] :
    LocallyConfluent (networkO2oUnord α).OpStep1 := by
  rintro ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ ⟨⟨Ip₁, Ifl₁⟩, ⟨buf₁, done₁⟩, O₁⟩
    ⟨⟨Ip₂, Ifl₂⟩, ⟨buf₂, done₂⟩, O₂⟩ ⟨δ₁, hs₁, hO₁⟩ ⟨δ₂, hs₂, hO₂⟩
  simp only at hO₁ hO₂
  cases hs₁ with
  | @send a l₁ _ _ _ _ hm₁ =>
    cases hs₂ with
    | @send b l₂ _ _ _ _ hm₂ =>
      subst hO₁ hO₂
      rcases Mset.cons_diamond (hm₁ ▸ hm₂) with ⟨rfl, hll⟩ | ⟨l'', h₁, h₂⟩
      · exact joinable_of_eq (by rw [hll])
      · -- send the other element on each side; in-flight multisets agree
        refine ⟨⟨(Mset.mk l'', Ifl), ⟨(buf.union (Mset.mk [a])).union (Mset.mk [b]), done⟩,
          (msetC α).concat ((msetC α).concat O (Mset.mk [], false)) (Mset.mk [], false)⟩,
          ?_, ?_⟩
        · exact Star.single ⟨_, unordStep.send h₁, rfl⟩
        · rw [Mset.union_right_comm buf (Mset.mk [a]) (Mset.mk [b])]
          exact Star.single ⟨_, unordStep.send h₂, rfl⟩
    | @recv _ h t _ _ hbuf =>
      subst hO₁ hO₂
      -- send a / recv h commute: buf ∪ {a} still exposes h
      refine ⟨⟨(Mset.mk l₁, Ifl), ⟨(Mset.mk t).union (Mset.mk [a]), done⟩,
        (msetC α).concat ((msetC α).concat O (Mset.mk [], false)) (Mset.mk [h], false)⟩,
        ?_, ?_⟩
      · refine Star.single ⟨_, unordStep.recv (h := h) (t := t ++ [a]) ?_, rfl⟩
        rw [hbuf]
        exact rfl
      · rw [out_comm]
        exact Star.single ⟨_, unordStep.send hm₁, rfl⟩
    | term =>
      exact absurd hm₁.symm Mset.mk_cons_ne_nil
  | @recv _ h t _ _ hbuf₁ =>
    cases hs₂ with
    | @send b l₂ _ _ _ _ hm₂ =>
      subst hO₁ hO₂
      refine ⟨⟨(Mset.mk l₂, Ifl), ⟨(Mset.mk t).union (Mset.mk [b]), done⟩,
        (msetC α).concat ((msetC α).concat O (Mset.mk [h], false)) (Mset.mk [], false)⟩,
        ?_, ?_⟩
      · exact Star.single ⟨_, unordStep.send hm₂, rfl⟩
      · rw [out_comm]
        refine Star.single ⟨_, unordStep.recv (h := h) (t := t ++ [b]) ?_, rfl⟩
        rw [hbuf₁]
        exact rfl
    | @recv _ h₂ t₂ _ _ hbuf₂ =>
      subst hO₁ hO₂
      rcases Mset.cons_diamond (hbuf₁ ▸ hbuf₂) with ⟨rfl, hll⟩ | ⟨t'', hd₁, hd₂⟩
      · exact joinable_of_eq (by rw [hll])
      · refine ⟨⟨(Ip, Ifl), ⟨Mset.mk t'', done⟩,
          (msetC α).concat ((msetC α).concat O (Mset.mk [h], false)) (Mset.mk [h₂], false)⟩,
          ?_, ?_⟩
        · exact Star.single ⟨_, unordStep.recv hd₁, rfl⟩
        · rw [out_comm]
          exact Star.single ⟨_, unordStep.recv hd₂, rfl⟩
    | term =>
      exact absurd hbuf₁.symm Mset.mk_cons_ne_nil
  | term =>
    cases hs₂ with
    | send hm => exact absurd hm.symm Mset.mk_cons_ne_nil
    | recv hbuf => exact absurd hbuf.symm Mset.mk_cons_ne_nil
    | term =>
      subst hO₁ hO₂
      exact joinable_of_eq rfl

theorem eager : (networkO2oUnord α).Eager1 := by
  rintro ⟨Δm, Δfl⟩ ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ ⟨⟨Ip', Ifl'⟩, ⟨buf', done'⟩, O'⟩ ⟨δ, hs, hO⟩
  simp only at hO
  cases hs with
  | @send a l _ _ _ _ hm =>
    cases Ifl with
    | false =>
      subst hO hm
      apply joinable_of_step
      induction Δm using Mset.ind with
      | h δrep =>
        refine ⟨_, ?_, rfl⟩
        show unordStep (Mset.union (Mset.mk (a :: l)) (Mset.mk δrep), Δfl) ⟨buf, done⟩
          (Mset.union (Mset.mk l) (Mset.mk δrep), Δfl)
          ⟨buf.union (Mset.mk [a]), done⟩ (Mset.mk [], false)
        rw [Mset.union_mk, Mset.union_mk]
        exact unordStep.send rfl
    | true =>
      subst hO
      exact joinable_of_step ⟨_, unordStep.send hm, rfl⟩
  | recv hbuf =>
    subst hO
    exact joinable_of_step ⟨_, unordStep.recv hbuf, rfl⟩
  | term =>
    subst hO
    exact joinable_of_step ⟨_, unordStep.term, rfl⟩

theorem inv_step {c c' : (networkO2oUnord α).Config1}
    (hinv : (networkO2oUnord α).Inv c.I c.st c.O)
    (hs : (networkO2oUnord α).OpStep1 c c') :
    (networkO2oUnord α).Inv c'.I c'.st c'.O := by
  obtain ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ := c
  obtain ⟨⟨Ip', Ifl'⟩, ⟨buf', done'⟩, O'⟩ := c'
  obtain ⟨δ, hstep, hO⟩ := hs
  simp only at hO hinv ⊢
  cases hstep with
  | @send a l _ _ _ _ hm =>
    intro hdone
    have := (hinv hdone).1
    rw [hm] at this
    exact absurd (congrArg Prod.fst this) Mset.mk_cons_ne_nil
  | recv hbuf =>
    intro hdone
    obtain ⟨hI, hOfl⟩ := hinv hdone
    refine ⟨hI, ?_⟩
    subst hO
    cases O with
    | mk v ofl => cases ofl with
      | true => rfl
      | false => simp_all
  | term =>
    intro _
    refine ⟨rfl, ?_⟩
    subst hO
    cases O with
    | mk v ofl => cases ofl <;> rfl

theorem inv_delta {I : (msetC α).C} {s : (networkO2oUnord α).State}
    {O : (msetC α).C} (Δ : (msetC α).C)
    (hinv : (networkO2oUnord α).Inv I s O) :
    (networkO2oUnord α).Inv ((msetC α).concat I Δ) s O := by
  intro hdone
  obtain ⟨hI, hO⟩ := hinv hdone
  subst hI
  exact ⟨rfl, hO⟩

theorem flag_invariant {c c' : (networkO2oUnord α).Config1}
    (hs : (networkO2oUnord α).OpStep1 c c') : c.I.2 = c'.I.2 := by
  obtain ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ := c
  obtain ⟨⟨Ip', Ifl'⟩, ⟨buf', done'⟩, O'⟩ := c'
  obtain ⟨δ, hstep, _⟩ := hs
  cases hstep <;> rfl

theorem flag_invariant_star {c c' : (networkO2oUnord α).Config1}
    (hs : Star (networkO2oUnord α).OpStep1 c c') : c.I.2 = c'.I.2 := by
  induction hs with
  | refl => rfl
  | tail _ hbc ih => exact ih.trans (flag_invariant hbc)

theorem finish (fc : (networkO2oUnord α).Config1)
    (hinv : (networkO2oUnord α).Inv fc.I fc.st fc.O)
    (hstuck : Stuck (networkO2oUnord α).OpStep1 fc) :
    ∃ z : (networkO2oUnord α).Config1,
      NormalizesTo (networkO2oUnord α).OpStep1 (fc.addDelta (Mset.mk [], true)) z ∧
        z.O = (msetC α).fix fc.O := by
  obtain ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ := fc
  simp only at hinv ⊢
  rcases Mset.eq_nil_or_cons Ip with rfl | ⟨a, l, rfl⟩
  case inr =>
    exact absurd
      ⟨⟨(Mset.mk l, Ifl), ⟨buf.union (Mset.mk [a]), done⟩,
        (msetC α).concat O (Mset.mk [], false)⟩,
        _, unordStep.send rfl, rfl⟩ hstuck
  rcases Mset.eq_nil_or_cons buf with rfl | ⟨h, t, rfl⟩
  case inr =>
    exact absurd
      ⟨⟨(Mset.mk [], Ifl), ⟨Mset.mk t, done⟩,
        (msetC α).concat O (Mset.mk [h], false)⟩,
        _, unordStep.recv rfl, rfl⟩ hstuck
  cases done with
  | true =>
    obtain ⟨hI, hO⟩ := hinv rfl
    have hIfl : Ifl = true := congrArg Prod.snd hI
    subst hIfl
    refine ⟨⟨(Mset.mk [], true), ⟨Mset.mk [], true⟩, O⟩, ⟨Star.refl _, ?_⟩, ?_⟩
    · rintro ⟨⟨⟨dIp, dIfl⟩, ⟨dbuf, ddone⟩, dO⟩, δ, hstep, -⟩
      cases hstep with
      | send hm => exact absurd hm.symm Mset.mk_cons_ne_nil
      | recv hbuf => exact absurd hbuf.symm Mset.mk_cons_ne_nil
    · cases hOc : O with
      | mk v ofl =>
        have : ofl = true := by rw [hOc] at hO; exact hO
        subst this
        rfl
  | false =>
    cases Ifl with
    | true =>
      exact absurd
        ⟨⟨(Mset.mk [], true), ⟨Mset.mk [], true⟩, (msetC α).concat O (Mset.mk [], true)⟩,
          _, unordStep.term, rfl⟩ hstuck
    | false =>
      refine ⟨⟨(Mset.mk [], true), ⟨Mset.mk [], true⟩,
        (msetC α).concat O (Mset.mk [], true)⟩,
        ⟨Star.single ⟨(Mset.mk [], true), ?_, rfl⟩, ?_⟩, ?_⟩
      · show unordStep (Mset.union (Mset.mk []) (Mset.mk []), true) ⟨Mset.mk [], false⟩
          (Mset.mk [], true) ⟨Mset.mk [], true⟩ (Mset.mk [], true)
        rw [Mset.union_mk]
        exact unordStep.term
      · rintro ⟨⟨⟨dIp, dIfl⟩, ⟨dbuf, ddone⟩, dO⟩, δ, hstep, -⟩
        cases hstep with
        | send hm => exact absurd hm.symm Mset.mk_cons_ne_nil
        | recv hbuf => exact absurd hbuf.symm Mset.mk_cons_ne_nil
      · cases O with
        | mk v ofl => cases ofl <;> (simp; rfl)

theorem progress [DecidableEq α] (b : Boundedness) :
    (networkO2oUnord α).Progress1 b b := by
  intro c fc hinv hfix hnorm
  have hfinv : (networkO2oUnord α).Inv fc.I fc.st fc.O :=
    Op1.inv_star (fun h hs => inv_step h hs) hnorm.1 hinv
  constructor
  · exact Op1.outputsMaximal_of_fixDelta eager
      (newman (Op1.sn_of_wf wf_decreasing) local_confluent)
      (fun h hs => inv_step h hs)
      (fun I => flagged_fix_eq_concat_term I) finish hinv hnorm
  · intro hb
    have hcfix : (msetC α).Fixed c.I := hfix hb
    have hflag : c.I.2 = true := (flagged_fixed_iff c.I).mp hcfix
    have hfflag : fc.I.2 = true := (flag_invariant_star hnorm.1) ▸ hflag
    obtain ⟨⟨Ip, Ifl⟩, ⟨buf, done⟩, O⟩ := fc
    simp only at hfflag hfinv
    subst hfflag
    have hstuck := hnorm.2
    rcases Mset.eq_nil_or_cons Ip with rfl | ⟨a, l, rfl⟩
    case inr =>
      exact absurd
        ⟨⟨(Mset.mk l, true), ⟨buf.union (Mset.mk [a]), done⟩,
          (msetC α).concat O (Mset.mk [], false)⟩,
          _, unordStep.send rfl, rfl⟩ hstuck
    rcases Mset.eq_nil_or_cons buf with rfl | ⟨h, t, rfl⟩
    case inr =>
      exact absurd
        ⟨⟨(Mset.mk [], true), ⟨Mset.mk t, done⟩,
          (msetC α).concat O (Mset.mk [h], false)⟩,
          _, unordStep.recv rfl, rfl⟩ hstuck
    cases done with
    | false =>
      exact absurd
        ⟨⟨(Mset.mk [], true), ⟨Mset.mk [], true⟩, (msetC α).concat O (Mset.mk [], true)⟩,
          _, unordStep.term, rfl⟩ hstuck
    | true =>
      exact (flagged_fixed_iff O).mpr (hfinv rfl).2

/-- `networkO2oUnord` satisfies every Flo operator obligation: the delivery
nondeterminism is fully absorbed by the `[T]unord` quotient (§3.5.2). -/
theorem lawful [DecidableEq α] (b : Boundedness) :
    (networkO2oUnord α).Lawful1 b b where
  wf_decreasing := wf_decreasing
  confluent := newman (Op1.sn_of_wf wf_decreasing) local_confluent
  eager := eager
  progress := progress b
  inv_step := fun h hs => inv_step h hs
  inv_delta := fun Δ h => inv_delta Δ h

end networkO2oUnord

/-- The Flo operator for reliable-unordered networking, fully lawful. -/
theorem networkO2oUnord_lawful {α : Type u} [DecidableEq α] (b : Boundedness) :
    ((networkO2oUnord α).toOperator b b).Lawful :=
  Op1.toOperator_lawful (networkO2oUnord.lawful b)

/-! ## Locating the network operators (§3.3.2, §3.5)

A leaf signature declaring which operators may appear where: any lawful
operator may be placed locally (all ports at one location), and the network
operators may bridge a source and destination location. This is the concrete
instantiation of the located typing judgment `Graph.LWT` for programs built
from this file's operators. -/

/-- A located leaf signature: `net op src dst` declares which operators are
network operators from `src` to `dst`; everything else must be local. -/
def netSig (net : ∀ {ins outs : List Coll.{u}}, Operator ins outs → Loc → Loc → Prop) :
    LocSig.{u} :=
  fun {ins outs} op il ol =>
    (∃ loc, il = List.replicate ins.length loc ∧ ol = List.replicate outs.length loc) ∨
    (∃ src dst, net op src dst ∧ il = List.replicate ins.length src ∧
      ol = List.replicate outs.length dst)

/-- Example: `networkO2o` typed as a bridge between two process locations
(Fig 3.7's `([T], U, Process[L₁]) ↩→ ([T], U, Process[L₂])`). -/
example (src dst : Loc) (b : Boundedness) (st : O2oSt α) (buf : Vals [seqC α]) :
    Graph.LWT
      (netSig fun {ins outs} _op s d =>
        ins = [seqC α] ∧ outs = [seqC α] ∧ s = src ∧ d = dst)
      (.node ((networkO2o α).toOperator b b) st buf) [src] [dst] :=
  .node (Or.inr ⟨src, dst, ⟨rfl, rfl, rfl, rfl⟩, rfl, rfl⟩)

end HydroLean.Gyatso
