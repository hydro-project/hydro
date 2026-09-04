import Hydro.Project.Model.Causal
import Hydro.Project.Step

/-!
# Every input sequence is the increments of an environment

`Dag.run` takes its inputs as per-step increments (`inp : Nat → HList Kind.delta Γ`);
the denotation takes an environment of step histories (`Env Γ`). `envOfInc` builds the
environment whose increments are a given sequence — the history at step `t` is the
concatenation of the increments up to `t` — so a wired theorem can be stated over an
interleaving's increments alone and recover the environment inside the proof
(`envIncAt_envOfInc`). Every such environment is tick-monotone (`HTMono_envOfInc`).
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-- The concatenation of the increments up to `t`. -/
def accum {α : Type} (inc : Nat → List α) : Nat → List α
  | 0 => inc 0
  | t + 1 => accum inc t ++ inc (t + 1)

theorem accum_prefix {α : Type} (inc : Nat → List α) (t : Nat) : accum inc t <+: accum inc (t + 1) :=
  ⟨inc (t + 1), rfl⟩

theorem accum_prefix_le {α : Type} (inc : Nat → List α) {t t' : Nat} (h : t ≤ t') :
    accum inc t <+: accum inc t' := by
  induction t' with
  | zero => cases h; exact List.prefix_rfl
  | succ t' ih =>
    rcases Nat.lt_or_eq_of_le h with h | h
    · exact (ih (Nat.le_of_lt_succ h)).trans (accum_prefix inc t')
    · subst h; exact List.prefix_rfl

/-- The step history with the given increments. -/
def histOfInc {α : Type} (inc : Nat → List α) : StepHist α :=
  ⟨accum inc, accum_prefix inc⟩

theorem incView_histOfInc {α : Type} (inc : Nat → List α) (t : Nat) : incView (histOfInc inc) t = inc t := by
  cases t with
  | zero => rfl
  | succ t =>
    show ((accum inc t ++ inc (t + 1)).drop (accum inc t).length) = inc (t + 1)
    rw [List.drop_left]

theorem incTrace_accum {σ : Type} (inc : Nat → List σ) (t : Nat) : incTrace (accum inc) t = inc t := by
  cases t with
  | zero => rfl
  | succ t =>
    show ((accum inc t ++ inc (t + 1)).drop (accum inc t).length) = inc (t + 1)
    rw [List.drop_left]

theorem tmono_accum {n : Nat} {σ : Type} (inc : Fin n → Nat → List σ) :
    TMono (fun i => accum (inc i)) := fun i _ _ h => accum_prefix_le (inc i) h

/-- A trace accumulated from single slots is paced: one entry per step
(`TPaced`'s idle clause is vacuous for an input — a production input is
supplied at the steps its process runs; `Interleaving.env` masks the
others, D80). -/
theorem tpaced_accum_slots {n : Nat} {σ : Type} (act : Fin n → Nat → Bool) (inc : Fin n → Nat → Option σ)
    (hidle : ∀ i k, act i k = false → inc i k = none) :
    TPaced act (fun i => accum (fun t => (inc i t).toList)) := by
  intro i k
  rw [incTrace_accum]
  refine ⟨?_, fun h => ?_⟩
  · cases inc i k <;> simp
  · rw [hidle i k h]; rfl

/-- The carrier of a kind whose increments are `inc`. -/
def Kind.ofInc : (κ : Kind L mem) → (Nat → κ.delta) → κ.sched
  | .stream _ _ _ _ _, inc => fun i => histOfInc (fun t => inc t i)
  | .keyed _ _ _ _ _ _ _, inc => fun i => histOfInc (fun t => inc t i)
  | .sing _ _ _ _ _ _ _, inc => fun i t => inc t i
  | .ksing _ _ _ _ _ _ _ _ _ _, inc => fun i t => inc t i
  | .atomic _ _, inc => fun i => accum (fun t => (inc t i).toList)
  | .atomicStream _ _ _ _ _, inc => fun i => accum (fun t => (inc t i).toList)
  | .atomicKeyed _ _ _ _ _ _ _, inc => fun i => accum (fun t => (inc t i).toList)
  | .atomicKSing _ _ _ _ _, inc => fun i => accum (fun t => (inc t i).toList)

theorem Option.toList_head? {α : Type} (o : Option α) : o.toList.head? = o := by cases o <;> rfl

theorem Kind.incAt_ofInc : (κ : Kind L mem) → (inc : Nat → κ.delta) → (t : Nat) →
    Kind.incAt κ (Kind.ofInc κ inc) t = inc t
  | .stream _ _ _ _ _, inc, t => funext fun i => incView_histOfInc _ t
  | .keyed _ _ _ _ _ _ _, inc, t => funext fun i => incView_histOfInc _ t
  | .sing _ _ _ _ _ _ _, _, _ => rfl
  | .ksing _ _ _ _ _ _ _ _ _ _, _, _ => rfl
  | .atomic _ _, inc, t => funext fun i => by
    show (incTrace (accum (fun t => (inc t i).toList)) t).head? = _
    rw [incTrace_accum, Option.toList_head?]
  | .atomicStream _ _ _ _ _, inc, t => funext fun i => by
    show (incTrace (accum (fun t => (inc t i).toList)) t).head? = _
    rw [incTrace_accum, Option.toList_head?]
  | .atomicKeyed _ _ _ _ _ _ _, inc, t => funext fun i => by
    show (incTrace (accum (fun t => (inc t i).toList)) t).head? = _
    rw [incTrace_accum, Option.toList_head?]
  | .atomicKSing _ _ _ _ _, inc, t => funext fun i => by
    show (incTrace (accum (fun t => (inc t i).toList)) t).head? = _
    rw [incTrace_accum, Option.toList_head?]

theorem Kind.tmono_ofInc : (κ : Kind L mem) → (inc : Nat → κ.delta) → Kind.tmono κ (Kind.ofInc κ inc)
  | .stream _ _ _ _ _, _ => trivial
  | .keyed _ _ _ _ _ _ _, _ => trivial
  | .sing _ _ _ _ _ _ _, _ => trivial
  | .ksing _ _ _ _ _ _ _ _ _ _, _ => trivial
  | .atomic _ _, inc => tmono_accum (fun i t => (inc t i).toList)
  | .atomicStream _ _ _ _ _, inc => tmono_accum (fun i t => (inc t i).toList)
  | .atomicKeyed _ _ _ _ _ _ _, inc => tmono_accum (fun i t => (inc t i).toList)
  | .atomicKSing _ _ _ _ _, inc => tmono_accum (fun i t => (inc t i).toList)

/-- An input's slots at a member's idle steps are empty. -/
def Kind.idleSlots (pacing : Pacing L mem) : (κ : Kind L mem) → (Nat → κ.delta) → Prop
  | .atomic ℓ _, inc => ∀ i k, pacing.active ℓ i k = false → inc k i = none
  | .atomicStream ℓ _ _ _ _, inc => ∀ i k, pacing.active ℓ i k = false → inc k i = none
  | .atomicKeyed ℓ _ _ _ _ _ _, inc => ∀ i k, pacing.active ℓ i k = false → inc k i = none
  | .atomicKSing ℓ _ _ _ _, inc => ∀ i k, pacing.active ℓ i k = false → inc k i = none
  | _, _ => True

theorem Kind.tpaced_ofInc (pacing : Pacing L mem) : (κ : Kind L mem) → (inc : Nat → κ.delta) →
    Kind.idleSlots pacing κ inc → Kind.tpaced pacing κ (Kind.ofInc κ inc)
  | .stream _ _ _ _ _, _, _ => trivial
  | .keyed _ _ _ _ _ _ _, _, _ => trivial
  | .sing _ _ _ _ _ _ _, _, _ => trivial
  | .ksing _ _ _ _ _ _ _ _ _ _, _, _ => trivial
  | .atomic _ _, inc, h => tpaced_accum_slots _ (fun i t => inc t i) h
  | .atomicStream _ _ _ _ _, inc, h => tpaced_accum_slots _ (fun i t => inc t i) h
  | .atomicKeyed _ _ _ _ _ _ _, inc, h => tpaced_accum_slots _ (fun i t => inc t i) h
  | .atomicKSing _ _ _ _ _, inc, h => tpaced_accum_slots _ (fun i t => inc t i) h

/-- The environment whose increments are `inp`. -/
def envOfInc : {Γ : Ctx L mem} → (Nat → HList (Kind.delta (mem := mem)) Γ) → Env Γ
  | [], _ => HList.nil
  | κ :: _, inp => HList.cons (f := Kind.sched (mem := mem)) (Kind.ofInc κ (fun t => (inp t).1)) (envOfInc (fun t => (inp t).2))

theorem envIncAt_envOfInc : {Γ : Ctx L mem} → (inp : Nat → HList (Kind.delta (mem := mem)) Γ) → (t : Nat) →
    envIncAt (envOfInc inp) t = inp t
  | [], _, _ => rfl
  | κ :: _, inp, t => by
    show (Kind.incAt κ (Kind.ofInc κ (fun t => (inp t).1)) t, envIncAt (envOfInc (fun t => (inp t).2)) t) = inp t
    rw [Kind.incAt_ofInc, envIncAt_envOfInc]

theorem HTMono_envOfInc : {Γ : Ctx L mem} → (inp : Nat → HList (Kind.delta (mem := mem)) Γ) →
    HTMono (envOfInc inp)
  | [], _ => trivial
  | κ :: _, inp => ⟨Kind.tmono_ofInc κ _, HTMono_envOfInc (fun t => (inp t).2)⟩

/-- Every atomic input's slots are empty at its member's idle steps. -/
def HIdleSlots (pacing : Pacing L mem) : {Γ : Ctx L mem} → (Nat → HList (Kind.delta (mem := mem)) Γ) → Prop
  | [], _ => True
  | κ :: _, inp => Kind.idleSlots pacing κ (fun t => (inp t).1) ∧ HIdleSlots pacing (fun t => (inp t).2)

theorem HTPaced_envOfInc (pacing : Pacing L mem) : {Γ : Ctx L mem} → (inp : Nat → HList (Kind.delta (mem := mem)) Γ) →
    HIdleSlots pacing inp → HTPaced pacing (envOfInc inp)
  | [], _, _ => trivial
  | κ :: _, inp, h => ⟨Kind.tpaced_ofInc pacing κ _ h.1, HTPaced_envOfInc pacing (fun t => (inp t).2) h.2⟩

end Project

end Hydro
