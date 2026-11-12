import Hydro.Graph.Causal

/-!
# Hydro · Project — the knot equations

`SchedSem` defines a knot as the ungated Kleene diagonal (`fix_stream`: the
`famHist` of `t ↦ (body^[t+1] ⊥).view t`; `fix_tick`: `t ↦ (body^[t+1] ⊥) t`).
For a body **strict** in its knot wire — every path from the wire to the
knot's output passes a network hop or a `defer_tick`, both of which read one
step back — the diagonal is a fixpoint of the body
(`SchedCausal.fix_stream_eq`/`fix_tick_eq`):

    D = body D

The Bekić cascade `bekic` closes the knots one at a time, each against the
body with the earlier knots closed; `bekic_fix` shows it solves the
simultaneous equations, `B (bekic B) = bekic B`, when the body is `HStrict`:
graded-causal at a lawful, **cascade-valid** lag functional, so each knot is
strict in its own wire once the earlier ones are closed (every cycle among the
knots passes a strict node, though a knot output may read another knot wire at
the same step).

These two fixpoint equations are what the stateful interpreter's knot cases
need: the knot wires' increments at a step are the body's knot outputs'
increments at that step (`Dag.step_spec` in `Project/Model/StepMain.lean`),
which `Dag.step` computes by iterating the body on the knot increments
(`Project/Step.lean`) and `Dag.sweep` by sweeping the body until every knot is
known (`Project/Sweep.lean`).
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-- A raw family whose every member is prefix-monotone step to step is its own freeze. -/
theorem famFreeze_eq_of_mono {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) (hm : ∀ t i, h t i <+: h (t + 1) i) :
    ∀ t, famFreeze h t = h t :=
  fun t => famFreeze_eq_of_ascending' h t (fun t' _ i => hm t' i)

section Fix
variable (pacing : Pacing L mem)

/-- Two step histories with the same view function are equal. -/
theorem StepHist.ext' {α : Type} {a b : StepHist α} (h : a.view = b.view) : a = b := by
  cases a; cases b; cases h; rfl

/-- Two families of stream wires that agree view by view are equal. -/
theorem stream_ext {n : Nat} {α : Type} {x y : Fin n → StepHist α}
    (h : ∀ i t, (x i).view t = (y i).view t) : x = y :=
  funext fun i => StepHist.ext' (funext fun t => h i t)

/-- **One knot wire is its body's fixpoint.** The hypothesis `hs : k.strictOf f` feeds
`fix_stream_eq`/`fix_tick_eq`; the conclusion is `f (knotFix f) = knotFix f`. -/
theorem knotFix_fix (k : KnotKind L mem) {f : k.toKind.sched → k.toKind.sched} (hs : k.strictOf f) :
    f (knotFix pacing k f) = knotFix pacing k f := by
  cases k with
  | stream ℓ α i ord ret => exact stream_ext (fun j t => (fix_stream_eq (pacing := pacing) hs j t).symm)
  | atomic ℓ σ => exact funext fun j => funext fun t => (fix_tick_eq (pacing := pacing) hs j t).symm

/-- **The cascade solves the simultaneous knot equations**: `B (bekic B) = bekic B` for a body
graded-causal at a lawful, cascade-valid lag functional. Induction on the knot list: the head
knot's cascade body is strict once the tail is closed (`KnotKind.strictOf_of`), so `knotFix_fix`
closes it; the tail body `B'` is causal at the next lag functional (`BodyCausal.tail`), so the
induction hypothesis closes the rest. -/
theorem bekic_fix_aux : (ks : List (KnotKind L mem)) →
    {B : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)} →
    (Φ₀ : Lags (ks.map KnotKind.toKind) → Lags (ks.map KnotKind.toKind)) →
    LagLaws Φ₀ Φ₀ → cascadeValid (ks.map KnotKind.toKind) Φ₀ = true → BodyCausal Φ₀ B →
    B (bekic pacing ks B) = bekic pacing ks B
  | [], B, _, _, _, _ => by
    show B HList.nil = HList.nil
    rfl
  | k :: ks, B, Φ₀, hl, hv, hs => by
    obtain ⟨hhead, hv'⟩ := cascadeValid_cons hv
    let inner : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) → k.toKind.sched :=
      fun xs => knotFix pacing k (fun l => (B (HList.cons l xs)).head)
    let B' := fun xs => (B (HList.cons (inner xs) xs)).tail
    let rest := bekic pacing ks B'
    have hrest : B' rest = rest := bekic_fix_aux ks (LagFun.next Φ₀) hl.next hv' (BodyCausal.tail pacing hl hhead hs)
    have hinner : (B (HList.cons (inner rest) rest)).head = inner rest :=
      knotFix_fix pacing k (KnotKind.strictOf_of hs hhead rest)
    show B (HList.cons (inner rest) rest) = HList.cons (inner rest) rest
    exact Prod.ext hinner hrest

/-- `bekic_fix_aux` with the lag functional packed into `hs : HStrict ks B`. -/
theorem bekic_fix (ks : List (KnotKind L mem))
    {B : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)}
    (hs : HStrict ks B) : B (bekic pacing ks B) = bekic pacing ks B :=
  let ⟨Φ₀, hl, hv, hc⟩ := hs
  bekic_fix_aux pacing ks Φ₀ hl hv hc

end Fix

end Project

end Hydro
