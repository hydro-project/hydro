import HydroLean.Flo.Operator
import HydroLean.Gyatso.ARS

/-!
# Single-port operator bridge

Gyatso's network operators (§3.5) are all 1-input/1-output. This file provides
`Op1`, a single-port operator specification on *plain carrier values* (no
`Vals` tuples), together with mirrored definitions of the Flo obligations
(`Lawful1`) and a proof that a lawful `Op1` yields a lawful Flo
`Operator [Lin] [Lout]` (`Op1.toOperator_lawful`). All the heterogeneous-tuple
plumbing is discharged once here, so each operator's proof is a clean argument
about its own carrier.
-/

namespace HydroLean.Gyatso

universe u

variable {L Lin Lout : Coll.{u}}

/-- Extract the single component of a one-port tuple. -/
def uno : Vals [L] → L.C
  | .cons x _ => x

/-- Build a one-port tuple. -/
def one (x : L.C) : Vals [L] := .cons x .nil

@[simp] theorem uno_one (x : L.C) : uno (one x) = x := rfl

@[simp] theorem one_uno : ∀ v : Vals [L], one (uno v) = v
  | .cons _ .nil => rfl

@[simp] theorem uno_concat (v w : Vals [L]) :
    uno (v.concat w) = L.concat (uno v) (uno w) := by
  cases v with
  | cons x xs =>
    cases xs
    cases w with
    | cons y ys => cases ys; rfl

@[simp] theorem one_concat (x y : L.C) :
    (one x).concat (one y) = one (L.concat x y) := rfl

@[simp] theorem uno_empty : uno (Vals.empty [L]) = L.empty := rfl

@[simp] theorem fixAll_one (x : L.C) : (one x).fixAll = one (L.fix x) := rfl

theorem allFixed_one {x : L.C} (h : L.Fixed x) : Vals.AllFixed (one x) :=
  .cons h .nil

theorem fixed_of_allFixed_one {x : L.C} (h : Vals.AllFixed (one x)) : L.Fixed x := by
  cases h with
  | cons hx _ => exact hx

/-- A single-port Flo operator specification on plain carrier values. -/
structure Op1 (Lin Lout : Coll.{u}) : Type (u + 1) where
  /-- Operator state (the paper's stateful operator expression). -/
  State : Type u
  /-- The small-step `(I, e) →δ (I', e', δ)` on plain carrier values. -/
  step : Lin.C → State → Lin.C → State → Lout.C → Prop
  /-- Consistency invariant (see `Operator.Inv`); trivial by default. -/
  Inv : Lin.C → State → Lout.C → Prop := fun _ _ _ => True

namespace Op1

variable {op : Op1 Lin Lout}

/-- Single-port configuration `(I, e, O)`. -/
structure Config1 (op : Op1 Lin Lout) : Type u where
  I : Lin.C
  st : op.State
  O : Lout.C

/-- The configuration step: perform `step` and concatenate the delta. -/
def OpStep1 (op : Op1 Lin Lout) : op.Config1 → op.Config1 → Prop :=
  fun c c' => ∃ δ, op.step c.I c.st c'.I c'.st δ ∧ c'.O = Lout.concat c.O δ

/-- Introduce an input delta. -/
def Config1.addDelta (Δ : Lin.C) (c : op.Config1) : op.Config1 :=
  { c with I := Lin.concat c.I Δ }

/-- Fix the input. -/
def Config1.fixInput (c : op.Config1) : op.Config1 :=
  { c with I := Lin.fix c.I }

/-- Def 2.3.1 (eager execution) at the single-port level. -/
def Eager1 (op : Op1 Lin Lout) : Prop :=
  ∀ (Δ : Lin.C) (c c' : op.Config1), op.OpStep1 c c' →
    Joinable op.OpStep1 (c.addDelta Δ) (c'.addDelta Δ)

/-- Def 2.3.2 (output maximality) at the single-port level. -/
def OutputsMaximal1 (op : Op1 Lin Lout) (c f : op.Config1) : Prop :=
  ∃ (I'' : Lin.C) (st'' : op.State),
    NormalizesTo op.OpStep1 c.fixInput ⟨I'', st'', Lout.fix f.O⟩

/-- Def 2.3.3 (streaming progress) at the single-port level, for declared
boundedness flags `ib`, `ob`, restricted to `Inv`-consistent configurations. -/
def Progress1 (op : Op1 Lin Lout) (ib ob : Boundedness) : Prop :=
  ∀ c f : op.Config1,
    op.Inv c.I c.st c.O →
    (ib = .bounded → Lin.Fixed c.I) →
    NormalizesTo op.OpStep1 c f →
    op.OutputsMaximal1 c f ∧ (ob = .bounded → Lout.Fixed f.O)

/-- The complete single-port obligations, mirroring `Operator.Lawful`. -/
structure Lawful1 (op : Op1 Lin Lout) (ib ob : Boundedness) : Prop where
  wf_decreasing : ∃ r : (op.State × Lin.C) → (op.State × Lin.C) → Prop,
    WellFounded r ∧
      ∀ {I s I' s' δ}, op.step I s I' s' δ → r (s', I') (s, I)
  confluent : Confluent op.OpStep1
  eager : op.Eager1
  progress : op.Progress1 ib ob
  inv_step : ∀ {c c' : op.Config1}, op.Inv c.I c.st c.O → op.OpStep1 c c' →
    op.Inv c'.I c'.st c'.O
  inv_delta : ∀ {I s O} (Δ : Lin.C), op.Inv I s O → op.Inv (Lin.concat I Δ) s O

/-- The Flo operator induced by a single-port specification. -/
def toOperator (op : Op1 Lin Lout) (ib ob : Boundedness) : Operator [Lin] [Lout] where
  State := op.State
  step I s I' s' δ := op.step (uno I) s (uno I') s' (uno δ)
  inBounds := [ib]
  outBounds := [ob]
  Inv I s O := op.Inv (uno I) s (uno O)

section Transfer

variable {ib ob : Boundedness}

/-- Project a Flo configuration of `toOperator` to a single-port one. -/
def c1of (c : (op.toOperator ib ob).Config) : op.Config1 :=
  ⟨uno c.I, c.st, uno c.O⟩

/-- Inject a single-port configuration into `toOperator`'s. -/
def cOf (c : op.Config1) : (op.toOperator ib ob).Config :=
  ⟨one c.I, c.st, one c.O⟩

@[simp] theorem c1of_cOf (c : op.Config1) : c1of (cOf (ib := ib) (ob := ob) c) = c := rfl

@[simp] theorem cOf_c1of (c : (op.toOperator ib ob).Config) : cOf (c1of c) = c := by
  cases c with
  | mk I st O => simp [c1of, cOf]

theorem step_iff {c c' : (op.toOperator ib ob).Config} :
    (op.toOperator ib ob).OpStep c c' ↔ op.OpStep1 (c1of c) (c1of c') := by
  constructor
  · rintro ⟨δ, hs, hO⟩
    exact ⟨uno δ, hs, by simp [c1of, hO]⟩
  · rintro ⟨δ, hs, hO⟩
    refine ⟨one δ, ?_, ?_⟩
    · exact hs
    · calc c'.O = one (uno c'.O) := (one_uno _).symm
        _ = one (Lout.concat (uno c.O) δ) := congrArg one hO
        _ = (one (uno c.O)).concat (one δ) := (one_concat _ _).symm
        _ = c.O.concat (one δ) := by rw [one_uno]

theorem star_of_star1 {c c' : op.Config1} (h : Star op.OpStep1 c c') :
    Star (op.toOperator ib ob).OpStep (cOf c) (cOf c') := by
  induction h with
  | refl => exact Star.refl _
  | tail _ hbc ih =>
    exact ih.tail (step_iff.mpr (by simpa using hbc))

theorem star1_of_star {c c' : (op.toOperator ib ob).Config}
    (h : Star (op.toOperator ib ob).OpStep c c') :
    Star op.OpStep1 (c1of c) (c1of c') := by
  induction h with
  | refl => exact Star.refl _
  | tail _ hbc ih => exact ih.tail (step_iff.mp hbc)

theorem stuck_iff {c : (op.toOperator ib ob).Config} :
    Stuck (op.toOperator ib ob).OpStep c ↔ Stuck op.OpStep1 (c1of c) := by
  constructor
  · rintro h ⟨d, hd⟩
    exact h ⟨cOf d, step_iff.mpr (by simpa using hd)⟩
  · rintro h ⟨d, hd⟩
    exact h ⟨c1of d, step_iff.mp hd⟩

theorem normalizesTo_iff {c f : op.Config1} :
    NormalizesTo (op.toOperator ib ob).OpStep (cOf c) (cOf f) ↔
      NormalizesTo op.OpStep1 c f := by
  constructor
  · rintro ⟨hstar, hstuck⟩
    exact ⟨by simpa using star1_of_star hstar, by simpa using stuck_iff.mp hstuck⟩
  · rintro ⟨hstar, hstuck⟩
    refine ⟨star_of_star1 hstar, ?_⟩
    rw [stuck_iff]
    simpa using hstuck

/-- **Transfer theorem**: a lawful single-port specification yields a lawful
Flo operator. Discharges all `Vals`-tuple plumbing once and for all. -/
theorem toOperator_lawful (h : op.Lawful1 ib ob) : (op.toOperator ib ob).Lawful := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · -- well-founded decrease: pull back along `uno`
    obtain ⟨r, hwf, hdec⟩ := h.wf_decreasing
    refine ⟨InvImage r (fun p => (p.1, uno p.2)), InvImage.wf _ hwf, ?_⟩
    intro I s I' s' δ hs
    exact hdec hs
  · -- confluence
    intro a b c hab hac
    obtain ⟨d, hbd, hcd⟩ := h.confluent _ _ _ (star1_of_star hab) (star1_of_star hac)
    refine ⟨cOf d, ?_, ?_⟩
    · have := star_of_star1 (op := op) (ib := ib) (ob := ob) hbd
      simpa using this
    · have := star_of_star1 (op := op) (ib := ib) (ob := ob) hcd
      simpa using this
  · -- eager execution
    intro Δ c c' hstep
    have h1 : op.OpStep1 (c1of c) (c1of c') := step_iff.mp hstep
    obtain ⟨d, hd₁, hd₂⟩ := h.eager (uno Δ) _ _ h1
    have e₁ : (c.addDelta Δ) = cOf ((c1of c).addDelta (uno Δ)) := by
      cases c with
      | mk I st O =>
        simp [Operator.Config.addDelta, Config1.addDelta, cOf, c1of, ← uno_concat]
    have e₂ : (c'.addDelta Δ) = cOf ((c1of c').addDelta (uno Δ)) := by
      cases c' with
      | mk I st O =>
        simp [Operator.Config.addDelta, Config1.addDelta, cOf, c1of, ← uno_concat]
    exact ⟨cOf d, e₁ ▸ star_of_star1 hd₁, e₂ ▸ star_of_star1 hd₂⟩
  · -- progress
    intro c f hinv hfix hnorm
    -- reduce all tuples to ground form
    obtain ⟨I, st, O⟩ := c
    obtain ⟨fI, fst, fO⟩ := f
    cases I with | cons x xs => ?_
    cases xs
    cases O with | cons y ys => ?_
    cases ys
    cases fI with | cons fx fxs => ?_
    cases fxs
    cases fO with | cons fy fys => ?_
    cases fys
    have hfix1 : ib = .bounded → Lin.Fixed x := fun hb => hfix.1 hb
    have hnorm1 : NormalizesTo op.OpStep1 ⟨x, st, y⟩ ⟨fx, fst, fy⟩ := by
      have := normalizesTo_iff (op := op) (ib := ib) (ob := ob)
        (c := ⟨x, st, y⟩) (f := ⟨fx, fst, fy⟩)
      exact this.mp hnorm
    obtain ⟨⟨I'', st'', hmax⟩, hofix⟩ :=
      h.progress ⟨x, st, y⟩ ⟨fx, fst, fy⟩ hinv hfix1 hnorm1
    constructor
    · refine ⟨one I'', st'', ?_⟩
      exact normalizesTo_iff.mpr hmax
    · exact ⟨fun hb => hofix hb, trivial⟩
  · -- inv_step
    intro c c' hinv hstep
    exact h.inv_step (c := c1of c) (c' := c1of c') hinv (step_iff.mp hstep)
  · -- inv_delta
    intro I s O Δ hinv
    cases I with | cons x xs => ?_
    cases xs
    cases Δ with | cons dx dxs => ?_
    cases dxs
    exact h.inv_delta (uno (one dx)) hinv

end Transfer

section Generic

/-- Strong normalization from a step-decreasing well-founded order (the
single-port analogue of `Operator.Lawful.sn`, Lemma 2.3.1). -/
theorem sn_of_wf (hwf : ∃ r : (op.State × Lin.C) → (op.State × Lin.C) → Prop,
    WellFounded r ∧ ∀ {I s I' s' δ}, op.step I s I' s' δ → r (s', I') (s, I)) :
    SN op.OpStep1 := by
  obtain ⟨r, hwfr, hdec⟩ := hwf
  have : ∀ c : op.Config1, Acc (fun b a => op.OpStep1 a b) c := by
    intro c
    have acc := hwfr.apply (c.st, c.I)
    generalize hm : (c.st, c.I) = m at acc
    induction acc generalizing c with
    | intro m _ ih =>
      subst hm
      constructor
      intro c' hstep
      obtain ⟨δ, hδ, _⟩ := hstep
      exact ih _ (hdec hδ) c' rfl
  exact ⟨this⟩

/-! ## Generic progress machinery

The following lemmas reduce the streaming-progress obligation (the hardest of
the §2.3.4 obligations) to a *single-configuration* argument, for any operator
whose input collection has a **terminator delta**: a delta `fixδ` with
`fix I = I ++ fixδ` (true of every `flagged` collection, where `fixδ = (∅, ⊗)`).

The key observation: `c.fixInput = c.addDelta fixδ`, so by eager execution
(extended to traces, `eager_star`) the fixed-input run joins the run that
already happened, and confluence then routes it to any stuck finisher we can
exhibit *from the reached stuck state `f` itself*. No invariants about
intermediate trace structure are ever needed — this is Def 2.3.1's "deltas can
be introduced at any time" doing all the work, exactly as the paper intends. -/

/-- Eager execution extends from single steps to whole traces: an input delta
can be introduced before or after any finite execution, with joinable results.
(The inductive extension noted after Def 2.3.1.) -/
theorem eager_star (heager : op.Eager1) (hconf : Confluent op.OpStep1)
    {c f : op.Config1} (h : Star op.OpStep1 c f) (Δ : Lin.C) :
    Joinable op.OpStep1 (c.addDelta Δ) (f.addDelta Δ) := by
  induction h with
  | refl => exact Joinable.refl _ _
  | tail hab hbc ih =>
    obtain ⟨d₁, hd₁, hd₂⟩ := ih
    obtain ⟨d₂, he₁, he₂⟩ := heager Δ _ _ hbc
    obtain ⟨e, hf₁, hf₂⟩ := hconf _ _ _ hd₂ he₁
    exact ⟨e, hd₁.trans hf₁, he₂.trans hf₂⟩

/-- The consistency invariant is preserved along traces. -/
theorem inv_star (hinv_step : ∀ {c c' : op.Config1},
      op.Inv c.I c.st c.O → op.OpStep1 c c' → op.Inv c'.I c'.st c'.O)
    {c f : op.Config1} (h : Star op.OpStep1 c f)
    (hc : op.Inv c.I c.st c.O) : op.Inv f.I f.st f.O := by
  induction h with
  | refl => exact hc
  | tail _ hbc ih => exact hinv_step ih hbc

/-- **Generic output maximality**: if fixing the input is the same as
concatenating a terminator delta `fixδ`, and from every consistent stuck state
`f` the configuration `f.addDelta fixδ` normalizes to a stuck state with
outputs `fix f.O`, then output maximality (Def 2.3.2) holds for every
consistent configuration. -/
theorem outputsMaximal_of_fixDelta
    (heager : op.Eager1) (hconf : Confluent op.OpStep1)
    (hinv_step : ∀ {c c' : op.Config1},
      op.Inv c.I c.st c.O → op.OpStep1 c c' → op.Inv c'.I c'.st c'.O)
    {fixδ : Lin.C} (hfixδ : ∀ I : Lin.C, Lin.fix I = Lin.concat I fixδ)
    (hfinish : ∀ f : op.Config1, op.Inv f.I f.st f.O → Stuck op.OpStep1 f →
      ∃ z : op.Config1, NormalizesTo op.OpStep1 (f.addDelta fixδ) z ∧
        z.O = Lout.fix f.O)
    {c f : op.Config1} (hcinv : op.Inv c.I c.st c.O)
    (hnorm : NormalizesTo op.OpStep1 c f) :
    op.OutputsMaximal1 c f := by
  -- `c.fixInput = c.addDelta fixδ` joins with `f.addDelta fixδ` by eager_star
  have hfinv : op.Inv f.I f.st f.O := inv_star hinv_step hnorm.1 hcinv
  obtain ⟨z, hznorm, hzO⟩ := hfinish f hfinv hnorm.2
  obtain ⟨d, hd₁, hd₂⟩ := eager_star heager hconf hnorm.1 fixδ
  -- z is stuck and reachable from `f.addDelta fixδ`; route d to z by confluence
  have hdz : Star op.OpStep1 d z := by
    obtain ⟨e, he₁, he₂⟩ := hconf _ _ _ hd₂ hznorm.1
    cases Star.eq_of_stuck he₂ hznorm.2
    exact he₁
  have : NormalizesTo op.OpStep1 (c.addDelta fixδ) z :=
    ⟨hd₁.trans hdz, hznorm.2⟩
  refine ⟨z.I, z.st, ?_⟩
  have hfixc : c.fixInput = c.addDelta fixδ := by
    simp [Config1.fixInput, Config1.addDelta, hfixδ]
  rw [hfixc]
  have hz : z = ⟨z.I, z.st, Lout.fix f.O⟩ := by
    cases z; simp_all
  exact hz ▸ this

end Generic

end Op1

end HydroLean.Gyatso
