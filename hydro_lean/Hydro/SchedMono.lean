import Hydro.SchedCausal
import Hydro.Transfer

/-!
# Hydro · tick-trace monotonicity of the machine (`TMono`)

**A realized tick never changes.** The machine's ticked carriers are
`Fin n → Nat → Trace σ` — the realized tick sequence AS OF each step —
and every machine op produces step-monotone traces: the trace as of
step `k` is a prefix of the trace as of any later step. This is true of
every machine run (ticks are atomic commits; the step histories they
read only grow), but the `Atomic` carrier's TYPE does not say so, and
until D71 no coupling proof needed it: every in-tick consumer was
order-insensitive (commutative, or `Pre`-commutative) on the tick's
batch, so per-tick multiset agreement sufficed. A per-tick
`assume_ordering` followed by a non-commutative fold is the first
consumer for which the batch's LIST matters at every horizon — and then
the corner must know that the list it derived the order from at `Td` is
the list the machine held at every `k ≤ Tc`.

This file is the machine-side fact, per op (the `causal_*` pattern of
`SchedCausal.lean`): `tmono_<op>`. The corner records it beside `cpl`
(`mono`, `Couple.lean`), the formers prove it from these, and the tick
knot (`tmono_fix_tick`) spends the body's causality to stabilize the
diagonal and the body's monotonicity to extend it. Sched itself is
untouched — the fact is PROVEN about its definitions, never assumed
(user ruling, D71: Sched stays minimal and auditable; the proofs live in
the coupling).
-/

namespace Hydro

/-- **Tick-trace monotonicity in the machine step**: the trace as of
step `k` is a prefix of the trace as of any later step `k'`. -/
def TMono {n : Nat} {σ : Type} (x : Fin n → Nat → Trace σ) : Prop :=
  ∀ i k k', k ≤ k' → x i k <+: x i k'

/-- Shaped tick tuples: monotone leafwise. -/
@[reducible] def TMonoOf {L : Type} {mem : L → Nat} {ℓ : L} : (sh : TickShape) →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh → Prop
  | .sing _, x => TMono x
  | .stream _ _ _ _, x => TMono x
  | .ksing _ _ _ _, x => TMono x
  | .pair a b, x => TMonoOf a x.1 ∧ TMonoOf b x.2

theorem TMono.bot {n : Nat} {σ : Type} : TMono (fun (_ : Fin n) (_ : Nat) => ([] : Trace σ)) :=
  fun _ _ _ _ => List.nil_prefix

theorem TMonoOf_fst {L : Type} {mem : L → Nat} {ℓ : L} (a b : TickShape)
    {x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ (.pair a b)}
    (hx : TMonoOf (.pair a b) x) : TMonoOf a x.1 := hx.1
theorem TMonoOf_snd {L : Type} {mem : L → Nat} {ℓ : L} (a b : TickShape)
    {x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ (.pair a b)}
    (hx : TMonoOf (.pair a b) x) : TMonoOf b x.2 := hx.2

/-- The trivial monotonicity fact of a non-tick wire (a stream has no
tick trace to be monotone in). The generated `M_smono_i` lemmas take
one hypothesis per wire so every wire binder is pinned by unification
from its hypothesis — this is the stream wires' placeholder, as
`TAgree.refl` is for the causal lemmas' context wires. -/
def TMonoTriv {τ : Sort _} (_x : τ) : Prop := True
theorem TMonoTriv.intro {τ : Sort _} (x : τ) : TMonoTriv x := trivial

variable {L : Type} {mem : L → Nat} {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool}

/-! ## The stream → tick boundaries: the tick steps only grow -/

/-- Per-tick batches over a prefix of the tick steps are a prefix of the
batches (the consumed count threads identically). -/
theorem batchesFrom_prefix {α : Type} (src : Nat → List α) :
    ∀ {steps steps' : List Nat}, steps <+: steps' → ∀ (c : Nat),
      batchesFrom src steps c <+: batchesFrom src steps' c
  | [], _, _, _ => List.nil_prefix
  | s :: rest, [], h, _ => absurd h (by simp)
  | s :: rest, s' :: rest', h, c => by
    obtain ⟨rfl, h'⟩ := List.cons_prefix_cons.mp h
    show (src s).drop c :: batchesFrom src rest (src s).length
      <+: (src s).drop c :: batchesFrom src rest' (src s).length
    exact List.cons_prefix_cons.mpr ⟨rfl, batchesFrom_prefix src h' _⟩

theorem tmono_snapshot {ℓ : L} {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {b : SingBound σ}
    (s : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b) (d : Unit) :
    TMono ((SchedSem L mem pacing).snapshot s d) := by
  intro i k k' hk
  show (tickSteps (pacing ℓ i) k).map (fun st => s i st)
    <+: (tickSteps (pacing ℓ i) k').map (fun st => s i st)
  exact List.IsPrefix.map _ (tickSteps_le_steps hk)

theorem tmono_batch {ℓ : L} {α : Type} [DecidableEq α]
    (s : (SchedSem L mem pacing).Stream ℓ α .noOrder .exactlyOnce) (d : Unit) :
    TMono ((SchedSem L mem pacing).batch s d) := by
  intro i k k' hk
  show batchesFrom ((s i).view) (tickSteps (pacing ℓ i) k) 0
    <+: batchesFrom ((s i).view) (tickSteps (pacing ℓ i) k') 0
  exact batchesFrom_prefix _ (tickSteps_le_steps hk) 0

theorem tmono_batch_ordered {ℓ : L} {α : Type} [DecidableEq α]
    (s : (SchedSem L mem pacing).Stream ℓ α .totalOrder .exactlyOnce) (d : Unit) :
    TMono ((SchedSem L mem pacing).batch_ordered s d) := by
  intro i k k' hk
  show batchesFrom ((s i).view) (tickSteps (pacing ℓ i) k) 0
    <+: batchesFrom ((s i).view) (tickSteps (pacing ℓ i) k') 0
  exact batchesFrom_prefix _ (tickSteps_le_steps hk) 0

theorem tmono_timeout_snapshot {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (s : (SchedSem L mem pacing).Stream ℓ α ord ret) (d : (SchedSem L mem pacing).TimerDec (mem ℓ)) :
    TMono ((SchedSem L mem pacing).timeout_snapshot s d) := by
  intro i k k' hk
  show (d i).take (tickSteps (pacing ℓ i) k).length
    <+: (d i).take (tickSteps (pacing ℓ i) k').length
  exact List.take_prefix_take_left (tickSteps_le_steps hk).length_le

theorem tmono_source_interval_batch {ℓ : L}
    (d : (SchedSem L mem pacing).PulseDec (mem ℓ)) :
    TMono ((SchedSem L mem pacing).source_interval_batch (ℓ := ℓ) d) := by
  intro i k k' hk
  show (d i).take (tickSteps (pacing ℓ i) k).length
    <+: (d i).take (tickSteps (pacing ℓ i) k').length
  exact List.take_prefix_take_left (tickSteps_le_steps hk).length_le

/-! ## The tick-to-tick operators -/

theorem tmono_mapTick {ℓ : L} {α β : Type} {t : (SchedSem L mem pacing).Atomic ℓ α}
    (f : Fin (mem ℓ) → α → β) (ht : TMono t) :
    TMono ((SchedSem L mem pacing).mapTick t f) := by
  intro i k k' hk
  show (t i k).map (f i) <+: (t i k').map (f i)
  exact List.IsPrefix.map _ (ht i k k' hk)

theorem tmono_zipTick {ℓ : L} {α β : Type} {a : (SchedSem L mem pacing).Atomic ℓ α}
    {b : (SchedSem L mem pacing).Atomic ℓ β} (ha : TMono a) (hb : TMono b) :
    TMono ((SchedSem L mem pacing).zipTick a b) := by
  intro i k k' hk
  show Trace.zip (a i k) (b i k) <+: Trace.zip (a i k') (b i k')
  exact zip_prefix (ha i k k' hk) (hb i k k' hk)

theorem tmono_defer_tick {ℓ : L} {σ : Type} (init : σ)
    {t : (SchedSem L mem pacing).Atomic ℓ σ} (ht : TMono t) :
    TMono ((SchedSem L mem pacing).defer_tick init t) := by
  intro i k k' hk
  show init :: t i k <+: init :: t i k'
  exact List.cons_prefix_cons.mpr ⟨rfl, ht i k k' hk⟩

theorem tmono_flattenOrdered {ℓ : L} {α : Type} [DecidableEq α]
    {t : (SchedSem L mem pacing).Atomic ℓ (List α)} (ht : TMono t) :
    TMono ((SchedSem L mem pacing).flattenOrdered t) := ht

theorem tmono_flattenUnordered {ℓ : L} {α : Type} [DecidableEq α]
    {t : (SchedSem L mem pacing).Atomic ℓ (List α)} (ht : TMono t) :
    TMono ((SchedSem L mem pacing).flattenUnordered t) := ht

theorem tmono_tinto_keyed {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {x : (SchedSem L mem pacing).AtomicStream ℓ (K × V) ord .exactlyOnce} (hx : TMono x) :
    TMono ((SchedSem L mem pacing).tinto_keyed x) := hx

theorem tmono_tassume_ordering {ℓ : L} {α : Type} [DecidableEq α]
    {u : (SchedSem L mem pacing).AtomicStream ℓ α .noOrder .exactlyOnce}
    (d : Unit) (hu : TMono u) :
    TMono ((SchedSem L mem pacing).tassume_ordering u d) := hu

theorem tmono_reduce_watermark {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd}
    {ks : (SchedSem L mem pacing).AtomicKeyedStream ℓ K V ord .exactlyOnce}
    {wm : (SchedSem L mem pacing).Atomic ℓ (Option K)}
    (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOk ord .exactlyOnce Pre (reduceStep f))
    (hks : TMono ks) (hwm : TMono wm) :
    TMono ((SchedSem L mem pacing).reduce_watermark ks wm f Pre ok) := by
  intro i k k' hk
  show scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none)
      (Trace.zip (ks i k) (wm i k))
    <+: scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none)
      (Trace.zip (ks i k') (wm i k'))
  exact scanAcrossTicksTrace_prefix _ _ (zip_prefix (hks i k k' hk) (hwm i k k' hk))

/-! ## The shaped tick former -/

theorem SchedTick.slice_mono {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) →
    TMonoOf sh x → ∀ i k k', k ≤ k' →
    SchedTick.slice sh x i k <+: SchedTick.slice sh x i k'
  | .sing _, _, hx, i, k, k', hk => hx i k k' hk
  | .stream _ _ _ _, _, hx, i, k, k', hk => hx i k k' hk
  | .ksing _ _ _ _, _, hx, i, k, k', hk => hx i k k' hk
  | .pair a b, x, hx, i, k, k', hk => by
    show Trace.zip (SchedTick.slice a x.1 i k) (SchedTick.slice b x.2 i k)
      <+: Trace.zip (SchedTick.slice a x.1 i k') (SchedTick.slice b x.2 i k')
    exact zip_prefix (SchedTick.slice_mono a x.1 hx.1 i k k' hk)
      (SchedTick.slice_mono b x.2 hx.2 i k k' hk)

theorem SchedTick.unslice_mono {ℓ : L} : (sh : TickShape) →
    (F : Fin (mem ℓ) → Nat → Trace (BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sh)) →
    (∀ i k k', k ≤ k' → F i k <+: F i k') →
    TMonoOf sh (SchedTick.unslice sh F)
  | .sing _, _, hF => hF
  | .stream _ _ _ _, _, hF => hF
  | .ksing _ _ _ _, _, hF => hF
  | .pair a b, _, hF =>
    ⟨SchedTick.unslice_mono a _ (fun i k k' hk => List.IsPrefix.map _ (hF i k k' hk)),
     SchedTick.unslice_mono b _ (fun i k k' hk => List.IsPrefix.map _ (hF i k k' hk))⟩

theorem tmono_tick_scan {ℓ : L} (sts ins outs : TickShape)
    {x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ ins}
    (g : Fin (mem ℓ) → BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sts →
      BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS ins →
      BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sts × BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS outs)
    (init : SeedOf sts) (hx : TMonoOf ins x) :
    TMonoOf outs ((SchedSem L mem pacing).tick_scan sts ins outs x g init) :=
  SchedTick.unslice_mono outs _ (fun i k k' hk => by
    show scanAcrossTicksTrace (g i) (SchedTick.seed sts init) (SchedTick.slice ins x i k)
      <+: scanAcrossTicksTrace (g i) (SchedTick.seed sts init) (SchedTick.slice ins x i k')
    exact scanAcrossTicksTrace_prefix _ _ (SchedTick.slice_mono ins x hx i k k' hk))

/-! ## The tick knot

The diagonal `fix_tick d body i t = (iterate shift ⊥ (t+1)) i t`. Two
facts combine: the body's causality stabilizes the stages (stage `t+1`
and stage `t'+1` agree at steps `≤ t`, `stab_iterate_tick`), and the
body's monotonicity makes every stage a monotone trace — so the
diagonal at `t` is stage `t'+1`'s entry at `t`, a prefix of its entry
at `t'`. -/

theorem TMono.tickshift {n : Nat} {σ : Type}
    {A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hA : ∀ x, TMono x → TMono (A x)) :
    ∀ m, TMono (iterate (τ := Fin n → Nat → Trace σ)
      (fun x j u => match u with
        | 0 => ([] : Trace σ)
        | u + 1 => (A x) j u)
      (fun _ _ => ([] : Trace σ)) m)
  | 0 => TMono.bot
  | m + 1 => by
    intro i k k' hk
    have hm := hA _ (TMono.tickshift hA m)
    cases k with
    | zero => exact List.nil_prefix
    | succ u =>
      cases k' with
      | zero => exact absurd hk (by omega)
      | succ u' => exact hm i u u' (Nat.le_of_succ_le_succ hk)

/-- The raw tick diagonal of a causal, monotonicity-preserving body is
step-monotone. -/
theorem tmono_tick_diag {n : Nat} {σ : Type}
    {A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hcaus : ∀ h' (x y : Fin n → Nat → Trace σ), TAgree h' x y → TAgree h' (A x) (A y))
    (hmono : ∀ x, TMono x → TMono (A x)) :
    TMono (fun i t => (iterate (τ := Fin n → Nat → Trace σ)
      (fun x j u => match u with
        | 0 => ([] : Trace σ)
        | u + 1 => (A x) j u)
      (fun _ _ => ([] : Trace σ)) (t + 1)) i t) := by
  intro i t t' ht
  -- the stages, spelled once
  let X : Nat → Fin n → Nat → Trace σ := iterate (τ := Fin n → Nat → Trace σ)
    (fun x j u => match u with
      | 0 => ([] : Trace σ)
      | u + 1 => (A x) j u) (fun _ _ => ([] : Trace σ))
  show X (t + 1) i t <+: X (t' + 1) i t'
  -- stage `t+1`'s entry at `t` is stage `t'+1`'s entry at `t` (stabilization)
  have hstab : X (t + 1) i t = X (t' + 1) i t :=
    stab_iterate_tick (h := t) (A := A) (A' := A)
      (fun h' _ x y hxy => hcaus h' x y hxy) t (t + 1) (t' + 1) (Nat.le_refl _)
      (Nat.lt_succ_self _) (by omega) i
  -- and stage `t'+1` is a monotone trace
  have hm : X (t' + 1) i t <+: X (t' + 1) i t' := TMono.tickshift hmono (t' + 1) i t t' ht
  rw [hstab]
  exact hm

theorem tmono_fix_tick {ℓ : L} {σ : Type} (d : Unit)
    {body : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ}
    (hcaus : ∀ h' (x y : (SchedSem L mem pacing).Atomic ℓ σ),
      TAgree h' x y → TAgree h' (body x) (body y))
    (hmono : ∀ x, TMono x → TMono (body x)) :
    TMono ((SchedSem L mem pacing).fix_tick d body) :=
  tmono_tick_diag (A := body) hcaus hmono

end Hydro
