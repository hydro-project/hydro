import Hydro.Project.Causal
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

/-- The carrier of a kind whose increments are `inc`. -/
def Kind.ofInc : (κ : Kind L mem) → (Nat → κ.delta) → κ.sched
  | .stream _ _ _ _ _, inc => fun i => histOfInc (fun t => inc t i)
  | .keyed _ _ _ _ _ _ _, inc => fun i => histOfInc (fun t => inc t i)
  | .sing _ _ _ _ _ _ _, inc => fun i t => inc t i
  | .ksing _ _ _ _ _ _ _ _ _ _, inc => fun i t => inc t i
  | .atomic _ _, inc => fun i => accum (fun t => inc t i)
  | .atomicStream _ _ _ _ _, inc => fun i => accum (fun t => inc t i)
  | .atomicKeyed _ _ _ _ _ _ _, inc => fun i => accum (fun t => inc t i)
  | .atomicKSing _ _ _ _ _, inc => fun i => accum (fun t => inc t i)

theorem Kind.incAt_ofInc : (κ : Kind L mem) → (inc : Nat → κ.delta) → (t : Nat) →
    Kind.incAt κ (Kind.ofInc κ inc) t = inc t
  | .stream _ _ _ _ _, inc, t => funext fun i => incView_histOfInc _ t
  | .keyed _ _ _ _ _ _ _, inc, t => funext fun i => incView_histOfInc _ t
  | .sing _ _ _ _ _ _ _, _, _ => rfl
  | .ksing _ _ _ _ _ _ _ _ _ _, _, _ => rfl
  | .atomic _ _, inc, t => funext fun i => incTrace_accum (fun t => inc t i) t
  | .atomicStream _ _ _ _ _, inc, t => funext fun i => incTrace_accum (fun t => inc t i) t
  | .atomicKeyed _ _ _ _ _ _ _, inc, t => funext fun i => incTrace_accum (fun t => inc t i) t
  | .atomicKSing _ _ _ _ _, inc, t => funext fun i => incTrace_accum (fun t => inc t i) t

theorem Kind.tmono_ofInc : (κ : Kind L mem) → (inc : Nat → κ.delta) → Kind.tmono κ (Kind.ofInc κ inc)
  | .stream _ _ _ _ _, _ => trivial
  | .keyed _ _ _ _ _ _ _, _ => trivial
  | .sing _ _ _ _ _ _ _, _ => trivial
  | .ksing _ _ _ _ _ _ _ _ _ _, _ => trivial
  | .atomic _ _, inc => tmono_accum (fun i t => inc t i)
  | .atomicStream _ _ _ _ _, inc => tmono_accum (fun i t => inc t i)
  | .atomicKeyed _ _ _ _ _ _ _, inc => tmono_accum (fun i t => inc t i)
  | .atomicKSing _ _ _ _ _, inc => tmono_accum (fun i t => inc t i)

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

end Project

end Hydro
