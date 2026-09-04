import Hydro.Project.Model.Causal

/-!
# Hydro · Project — the knot equations

`SchedSem` defines a knot as a Kleene diagonal (`fix_stream`: the
`famHist` of `t ↦ (shift∘body)^[t+1] ⊥ .view t`; `fix_tick`: the
tick-shifted iterate), the shift gated by the member's ticks. For a
causal body these are exactly the tick-gated one-step-delayed feedback
equations

    D.view 0 = []   D.view (t+1) = if tick (t+1) then (body D).view t else D.view t
    D i 0   = []    D i (t+1)   = if tick (t+1) then body D i t else D i t

which is what the stateful interpreter runs (`Node.step` at `fixN`:
release the pending body output at ticking members, step the body on it,
keep the rest and append the body's new output). The Bekić cascade
(`bekic`) satisfies them componentwise against the whole body.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-- A member-wise prefix-monotone raw family is its own freeze. -/
theorem famFreeze_eq_of_mono {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) (hm : ∀ t i, h t i <+: h (t + 1) i) :
    ∀ t, famFreeze h t = h t
  | 0 => rfl
  | t + 1 => by
    show (if (List.finRange n).all
        (fun i => ((famFreeze h t) i).isPrefixOf (h (t + 1) i))
      then h (t + 1) else famFreeze h t) = h (t + 1)
    rw [famFreeze_eq_of_mono h hm t]
    have : (List.finRange n).all (fun i => (h t i).isPrefixOf (h (t + 1) i)) = true := by
      rw [List.all_eq_true]
      intro i _
      exact List.isPrefixOf_iff_prefix.mpr (hm t i)
    rw [this]
    rfl

section Stream
variable {pacing : Pacing L mem}

/-- The stream knot's stages (at the members' tick skeletons `p`). -/
abbrev knotStages {n : Nat} {α : Type} (p : Fin n → Nat → Bool)
    (body : (Fin n → StepHist α) → (Fin n → StepHist α)) :
    Nat → Fin n → StepHist α :=
  iterate (fun x j => ((body x) j).shift (p j)) (fun _ => StepHist.bot)

/-- Stage `m > t` shows the diagonal's view at `t`. -/
theorem knotStages_stab {n : Nat} {α : Type} {p : Fin n → Nat → Bool}
    {body : (Fin n → StepHist α) → (Fin n → StepHist α)}
    (hb : ∀ h' (x y : Fin n → StepHist α), SAgree h' x y → SAgree h' (body x) (body y))
    {t m : Nat} (hm : t < m) (i : Fin n) :
    (knotStages p body m i).view t = (knotStages p body (t + 1) i).view t :=
  stab_iterate_shift (h := t) (A := body) (A' := body) p (fun h' _ x y hxy => hb h' x y hxy)
    t m (t + 1) (Nat.le_refl _) hm (Nat.lt_succ_self _) i

/-- The raw diagonal is member-wise prefix-monotone. -/
theorem knotDiag_mono {n : Nat} {α : Type} {p : Fin n → Nat → Bool}
    {body : (Fin n → StepHist α) → (Fin n → StepHist α)}
    (hb : ∀ h' (x y : Fin n → StepHist α), SAgree h' x y → SAgree h' (body x) (body y))
    (t : Nat) (i : Fin n) :
    (knotStages p body (t + 1) i).view t <+: (knotStages p body (t + 2) i).view (t + 1) := by
  rw [← knotStages_stab hb (m := t + 2) (by omega) i]
  exact (knotStages p body (t + 2) i).mono t

/-- The knot's view at `t` is the raw diagonal's. -/
theorem fix_stream_view {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {body : (SchedSem L mem pacing).Stream ℓ α ord ret → (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hb : ∀ h' x y, SAgree h' x y → SAgree h' (body x) (body y)) (i : Fin (mem ℓ)) (t : Nat) :
    (((SchedSem L mem pacing).fix_stream () body) i).view t
      = (knotStages (fun j => pacing.active ℓ j) body (t + 1) i).view t := by
  show famFreeze (fun t j => (knotStages (fun j => pacing.active ℓ j) body (t + 1) j).view t) t i = _
  rw [famFreeze_eq_of_mono _ (fun t j => knotDiag_mono hb t j) t]

theorem fix_stream_view_zero {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {body : (SchedSem L mem pacing).Stream ℓ α ord ret → (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hb : ∀ h' x y, SAgree h' x y → SAgree h' (body x) (body y)) (i : Fin (mem ℓ)) :
    (((SchedSem L mem pacing).fix_stream () body) i).view 0 = [] := by
  rw [fix_stream_view hb]
  rfl

/-- **The feedback equation**: at a ticking step the knot is the body on
the knot one step earlier; at a non-ticking step it holds. -/
theorem fix_stream_view_succ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {body : (SchedSem L mem pacing).Stream ℓ α ord ret → (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hb : ∀ h' x y, SAgree h' x y → SAgree h' (body x) (body y)) (i : Fin (mem ℓ)) (t : Nat) :
    (((SchedSem L mem pacing).fix_stream () body) i).view (t + 1)
      = if pacing.active ℓ i (t + 1) then ((body ((SchedSem L mem pacing).fix_stream () body)) i).view t
        else (((SchedSem L mem pacing).fix_stream () body) i).view t := by
  rw [fix_stream_view hb]
  show gateView (pacing.active ℓ i) (body (knotStages (fun j => pacing.active ℓ j) body (t + 1)) i).view (t + 1) = _
  rw [gateView_succ]
  -- stage `t+1` agrees with the knot up to `t`
  have hag : SAgree t (knotStages (fun j => pacing.active ℓ j) body (t + 1))
      ((SchedSem L mem pacing).fix_stream () body) := by
    intro j s hs
    rw [fix_stream_view hb]
    exact knotStages_stab hb (by omega) j
  split
  · exact hb t _ _ hag i t (Nat.le_refl _)
  · rw [fix_stream_view hb]
    show (knotStages (fun j => pacing.active ℓ j) body (t + 2) i).view t = _
    exact knotStages_stab hb (m := t + 2) (by omega) i

end Stream

section Tick
variable {pacing : Pacing L mem}

abbrev tickStages {n : Nat} {σ : Type} (p : Fin n → Nat → Bool)
    (body : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)) :
    Nat → Fin n → Nat → Trace σ :=
  iterate (τ := Fin n → Nat → Trace σ)
    (fun x j => gateView (p j) ((body x) j))
    (fun _ _ => ([] : Trace σ))

theorem tickStages_stab {n : Nat} {σ : Type} {p : Fin n → Nat → Bool}
    {body : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hb : ∀ h' (x y : Fin n → Nat → Trace σ), TAgree h' x y → TAgree h' (body x) (body y))
    {t m : Nat} (hm : t < m) (i : Fin n) :
    tickStages p body m i t = tickStages p body (t + 1) i t :=
  stab_iterate_tick (h := t) (A := body) (A' := body) p (fun h' _ x y hxy => hb h' x y hxy)
    t m (t + 1) (Nat.le_refl _) hm (Nat.lt_succ_self _) i

theorem fix_tick_eq {ℓ : L} {σ : Type}
    {body : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ}
    (i : Fin (mem ℓ)) (t : Nat) :
    ((SchedSem L mem pacing).fix_tick () body) i t = tickStages (fun j => pacing.active ℓ j) body (t + 1) i t := rfl

theorem fix_tick_zero {ℓ : L} {σ : Type}
    {body : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ}
    (i : Fin (mem ℓ)) : ((SchedSem L mem pacing).fix_tick () body) i 0 = [] := rfl

/-- **The feedback equation** for tick knots. -/
theorem fix_tick_succ {ℓ : L} {σ : Type}
    {body : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ}
    (hb : ∀ h' x y, TAgree h' x y → TAgree h' (body x) (body y)) (i : Fin (mem ℓ)) (t : Nat) :
    ((SchedSem L mem pacing).fix_tick () body) i (t + 1)
      = if pacing.active ℓ i (t + 1) then (body ((SchedSem L mem pacing).fix_tick () body)) i t
        else ((SchedSem L mem pacing).fix_tick () body) i t := by
  rw [fix_tick_eq]
  show gateView (pacing.active ℓ i) (body (tickStages (fun j => pacing.active ℓ j) body (t + 1)) i) (t + 1) = _
  rw [gateView_succ]
  have hag : TAgree t (tickStages (fun j => pacing.active ℓ j) body (t + 1))
      ((SchedSem L mem pacing).fix_tick () body) := by
    intro j s hs
    rw [fix_tick_eq]
    exact tickStages_stab hb (by omega) j
  split
  · exact hb t _ _ hag i t (Nat.le_refl _)
  · rw [fix_tick_eq]
    show tickStages (fun j => pacing.active ℓ j) body (t + 2) i t = _
    exact tickStages_stab hb (m := t + 2) (by omega) i

end Tick

/-! ## The equations, per knot kind, and for the cascade -/

section Delayed
variable (pacing : Pacing L mem)

/-- `x` is the tick-gated one-step-delayed `y`: empty at `0`; at a ticking
`t+1`, `y`'s step-`t` value; at a non-ticking `t+1`, its own step-`t` value. -/
def KnotKind.delayed : (k : KnotKind L mem) → k.toKind.sched → k.toKind.sched → Prop
  | .stream ℓ _ _ _ _, x, y => ∀ (i : Fin (mem ℓ)), (x i).view 0 = [] ∧
      ∀ t, (x i).view (t + 1) = if pacing.active ℓ i (t + 1) then (y i).view t else (x i).view t
  | .atomic ℓ _, x, y => ∀ (i : Fin (mem ℓ)), x i 0 = [] ∧
      ∀ t, x i (t + 1) = if pacing.active ℓ i (t + 1) then y i t else x i t

/-- Componentwise. -/
def HDelayed : (ks : List (KnotKind L mem)) →
    HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
    HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) → Prop
  | [], _, _ => True
  | k :: ks, x, y => k.delayed pacing x.1 y.1 ∧ HDelayed ks x.2 y.2

end Delayed

section Cascade
variable (pacing : Pacing L mem)

theorem knotFix_delayed (k : KnotKind L mem) {f : k.toKind.sched → k.toKind.sched}
    (hf : ∀ h' x y, Kind.agree h' k.toKind x y → Kind.agree h' k.toKind (f x) (f y)) :
    k.delayed pacing (knotFix pacing k f) (f (knotFix pacing k f)) := by
  cases k with
  | stream ℓ α i ord ret =>
    exact fun j => ⟨fix_stream_view_zero hf j, fun t => fix_stream_view_succ hf j t⟩
  | atomic ℓ σ =>
    exact fun j => ⟨fix_tick_zero j, fun t => fix_tick_succ hf j t⟩

/-- **The cascade solves the simultaneous feedback equations** against
the whole body. -/
theorem bekic_delayed : (ks : List (KnotKind L mem)) →
    {B : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)} →
    (∀ h' x y, HAgree h' x y → HAgree h' (B x) (B y)) →
    HDelayed pacing ks (bekic pacing ks B) (B (bekic pacing ks B))
  | [], _, _ => trivial
  | k :: ks, B, hc => by
    -- the cascade's shape
    let inner : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) → k.toKind.sched :=
      fun xs => knotFix pacing k (fun l => (B (HList.cons l xs)).head)
    let B' := fun xs => (B (HList.cons (inner xs) xs)).tail
    let rest := bekic pacing ks B'
    have hshape : bekic pacing ks B' = rest := rfl
    show k.delayed pacing (inner rest) (B (HList.cons (inner rest) rest)).head ∧
      HDelayed pacing ks rest (B (HList.cons (inner rest) rest)).tail
    constructor
    · exact knotFix_delayed pacing k (fun h' x y hxy => (hc h' _ _ ⟨hxy, HAgree.refl h' rest⟩).head)
    · have hc' : ∀ h' x y, HAgree h' x y → HAgree h' (B' x) (B' y) := by
        intro h' x y hxy
        exact (hc h' _ _ ⟨knotFix_agree pacing k (fun h'' hh'' a b hab =>
          (hc h'' _ _ ⟨hab, hxy.mono hh''⟩).head), hxy⟩).tail
      exact bekic_delayed ks hc'

end Cascade

end Project

end Hydro
