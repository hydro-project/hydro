import Hydro.SchedCausal
import Hydro.Transfer

/-!
# Tick-trace monotonicity of the machine (`TMono`)

**A realized tick never changes.** The machine's ticked carriers are `Fin n → Nat → Trace σ`,
the realized tick sequence as of each step, and every machine op produces step-monotone
traces: the trace as of step `k` is a prefix of the trace as of any later step. This holds
of every machine run — ticks are atomic commits, and the step histories they read only grow
— but the `Atomic` carrier's type does not say so, so it is proved here per op (`tmono_<op>`),
following the `causal_*` pattern of `Hydro/SchedCausal.lean`.

The consumer is the coupling corner. A per-tick `assume_ordering` followed by a
non-commutative fold makes the batch's list matter at every horizon, and the corner must then
know that the list it derived the order from at `Td` is the list the machine held at every
`k ≤ Tc`. The corner records `TMono` beside `cpl` (`mono` in `Hydro/Couple.lean`), the
formers prove it from these lemmas, and the tick knot (`tmono_fix_tick`) spends the body's
strictness to stabilize the diagonal and the body's monotonicity to extend it.
`Hydro/Sched.lean` is untouched: the fact is proved about its definitions, not assumed.
-/

namespace Hydro

/-- **Tick-trace monotonicity in the machine step**: the trace as of step `k` is a prefix of
the trace as of any later step `k'`. -/
def TMono {n : Nat} {σ : Type} (x : Fin n → Nat → Trace σ) : Prop :=
  ∀ i k k', k ≤ k' → x i k <+: x i k'

/-- `TMono` leafwise over a shaped tick tuple. -/
@[reducible] def TMonoOf {L : Type} {mem : L → Nat} {ℓ : L} : (sh : TickShape) →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh → Prop
  | .sing _, x => TMono x
  | .stream _ _ _ _, x => TMono x
  | .ksing _ _ _ _, x => TMono x
  | .kstream _ _ _ _ _ _, x => TMono x
  | .pair a b, x => TMonoOf a x.1 ∧ TMonoOf b x.2

/-- The empty trace family is monotone. -/
theorem TMono.bot {n : Nat} {σ : Type} : TMono (fun (_ : Fin n) (_ : Nat) => ([] : Trace σ)) :=
  fun _ _ _ _ => List.nil_prefix

theorem TMonoOf_fst {L : Type} {mem : L → Nat} {ℓ : L} (a b : TickShape)
    {x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ (.pair a b)}
    (hx : TMonoOf (.pair a b) x) : TMonoOf a x.1 := hx.1
theorem TMonoOf_snd {L : Type} {mem : L → Nat} {ℓ : L} (a b : TickShape)
    {x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ (.pair a b)}
    (hx : TMonoOf (.pair a b) x) : TMonoOf b x.2 := hx.2

/-- The trivial monotonicity fact of a non-tick wire: a stream has no tick trace to be
monotone in. The generated `M_smono_i` lemmas take one hypothesis per wire so unification
pins every wire binder from its hypothesis. `TMonoTriv` is the stream wires' placeholder,
as `TAgree.refl` is for the causal lemmas' context wires. -/
def TMonoTriv {τ : Sort _} (_x : τ) : Prop := True
theorem TMonoTriv.intro {τ : Sort _} (x : τ) : TMonoTriv x := trivial

variable {L : Type} {mem : L → Nat} {pacing : Pacing L mem}

/-! ## The stream → tick boundaries: the tick steps only grow -/

/-- Per-tick batches over a prefix of the tick steps are a prefix of the batches: the consumed
count threads identically through the shared prefix. -/
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
  show (tickSteps (pacing.ticks ℓ i) k).map (fun st => s i st)
    <+: (tickSteps (pacing.ticks ℓ i) k').map (fun st => s i st)
  exact List.IsPrefix.map _ (tickSteps_le_steps hk)

theorem tmono_batch {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (s : (SchedSem L mem pacing).Stream ℓ α ord ret) (d : Unit) :
    TMono ((SchedSem L mem pacing).batch s d) := by
  intro i k k' hk
  show (batchesFrom ((s i).view) (tickSteps (pacing.ticks ℓ i) k) 0).map List.toArray
    <+: (batchesFrom ((s i).view) (tickSteps (pacing.ticks ℓ i) k') 0).map List.toArray
  exact List.IsPrefix.map _ (batchesFrom_prefix _ (tickSteps_le_steps hk) 0)

theorem tmono_firstTick {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries}
    {t : (SchedSem L mem pacing).AtomicStream ℓ α .totalOrder ret} (ht : TMono t) :
    TMono ((SchedSem L mem pacing).firstTick t) := by
  intro i k k' hk
  show (t i k).map (fun b => b[0]?) <+: (t i k').map (fun b => b[0]?)
  exact List.IsPrefix.map _ (ht i k k' hk)

/-! The tick-located keyed and timing sources. -/

theorem tmono_ksnapshot {ℓ : L} {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} {b : KSBound}
    (ks : (SchedSem L mem pacing).KeyedSingleton ℓ α K V ord ret b) (d : Unit) :
    TMono ((SchedSem L mem pacing).ksnapshot ks d) := by
  intro i k k' hk
  show (tickSteps (pacing.ticks ℓ i) k).map (fun st => ks i st)
    <+: (tickSteps (pacing.ticks ℓ i) k').map (fun st => ks i st)
  exact List.IsPrefix.map _ (tickSteps_le_steps hk)

theorem tmono_tfold {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk ord ret Pre g) {bs : (SchedSem L mem pacing).AtomicStream ℓ α ord ret}
    (hb : TMono bs) :
    TMono ((SchedSem L mem pacing).tfold g init Pre ok bs) := by
  intro i k k' hk
  show tfoldHist g init (bs i k) <+: tfoldHist g init (bs i k')
  exact scanAcrossTicksTrace_prefix _ _ (hb i k k' hk)

theorem tmono_time {ℓ : L} (d : (SchedSem L mem pacing).ClockDec (mem ℓ)) :
    TMono ((SchedSem L mem pacing).time (ℓ := ℓ) d) := by
  intro i k k' hk
  exact scriptTrace_prefix (d i) (tickSteps_le_steps hk).length_le

theorem tmono_snapshot_bounded {ℓ : L} {σ : Type} (x : σ) (d : Unit) :
    TMono ((SchedSem L mem pacing).snapshot_bounded (ℓ := ℓ) x d) := by
  intro i k k' hk
  show List.replicate (tickSteps (pacing.ticks ℓ i) k).length x
    <+: List.replicate (tickSteps (pacing.ticks ℓ i) k').length x
  have hle := (tickSteps_le_steps (p := pacing.ticks ℓ i) hk).length_le
  refine ⟨List.replicate ((tickSteps (pacing.ticks ℓ i) k').length - (tickSteps (pacing.ticks ℓ i) k).length) x, ?_⟩
  rw [← List.replicate_add, Nat.add_sub_cancel' hle]

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

/-- `take n` of a prefix is a prefix of `take n'` of the longer list when `n ≤ n'`. -/
theorem take_prefix_take_of_prefix {α : Type} {l l' : List α} (h : l <+: l') {n n' : Nat}
    (hn : n ≤ n') : l.take n <+: l'.take n' := by
  have h1 : l.take n <+: l' := (List.take_prefix n l).trans h
  rw [List.prefix_iff_eq_take.mp h1]
  apply List.take_prefix_take_left
  simp only [List.length_take]
  exact le_trans (Nat.min_le_left _ _) hn

/-- `defer_tick` is monotone: the buffer grows by prefix with the input and the released count
is monotone (`deferReleased_mono`). -/
theorem tmono_defer_tick {ℓ : L} {σ : Type} (init : σ)
    {t : (SchedSem L mem pacing).Atomic ℓ σ} (ht : TMono t) :
    TMono ((SchedSem L mem pacing).defer_tick init t) := by
  intro i k k' hk
  show (init :: t i (k - 1)).take (deferReleased (pacing.ticks ℓ i) (fun k => (t i (k - 1)).length) k)
    <+: (init :: t i (k' - 1)).take (deferReleased (pacing.ticks ℓ i) (fun k => (t i (k - 1)).length) k')
  exact take_prefix_take_of_prefix (List.cons_prefix_cons.mpr ⟨rfl, ht i (k - 1) (k' - 1) (by omega)⟩)
    (deferReleased_mono _ _ hk)

theorem tmono_flattenOrdered {ℓ : L} {α : Type} [DecidableEq α]
    {t : (SchedSem L mem pacing).Atomic ℓ (List α)} (ht : TMono t) :
    TMono ((SchedSem L mem pacing).flattenOrdered t) := by
  intro i k k' hk
  show (t i k).map List.toArray <+: (t i k').map List.toArray
  exact List.IsPrefix.map _ (ht i k k' hk)

theorem tmono_flattenUnordered {ℓ : L} {α : Type} [DecidableEq α]
    {t : (SchedSem L mem pacing).Atomic ℓ (List α)} (ht : TMono t) :
    TMono ((SchedSem L mem pacing).flattenUnordered t) := by
  intro i k k' hk
  show (t i k).map List.toArray <+: (t i k').map List.toArray
  exact List.IsPrefix.map _ (ht i k k' hk)

theorem tmono_tinto_keyed {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {x : (SchedSem L mem pacing).AtomicStream ℓ (K × V) ord .exactlyOnce} (hx : TMono x) :
    TMono ((SchedSem L mem pacing).tinto_keyed x) := hx

theorem tmono_tassume_ordering {ℓ : L} {α : Type} [DecidableEq α]
    {u : (SchedSem L mem pacing).AtomicStream ℓ α .noOrder .exactlyOnce}
    (d : Unit) (hu : TMono u) :
    TMono ((SchedSem L mem pacing).tassume_ordering u d) := hu

theorem tmono_reduce_watermark {ℓ : L} {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd}
    {ks : (SchedSem L mem pacing).AtomicKeyedStream ℓ K V ord .exactlyOnce}
    {wm : (SchedSem L mem pacing).Atomic ℓ (Option K)}
    (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOk ord .exactlyOnce Pre (reduceStep f))
    (hks : TMono ks) (hwm : TMono wm) :
    TMono ((SchedSem L mem pacing).reduce_watermark ks wm f Pre ok) := by
  intro i k k' hk
  show scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none)
      (Trace.zip ((ks i k).map Array.toList) (wm i k))
    <+: scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none)
      (Trace.zip ((ks i k').map Array.toList) (wm i k'))
  exact scanAcrossTicksTrace_prefix _ _
    (zip_prefix (List.IsPrefix.map _ (hks i k k' hk)) (hwm i k k' hk))

/-! ## The shaped tick former -/

/-- Slicing a leafwise-monotone shaped input gives a monotone trace of tuples. -/
theorem SchedTick.slice_mono {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh) →
    TMonoOf sh x → ∀ i k k', k ≤ k' →
    SchedTick.slice sh x i k <+: SchedTick.slice sh x i k'
  | .sing _, _, hx, i, k, k', hk => hx i k k' hk
  | .stream _ _ _ _, _, hx, i, k, k', hk => hx i k k' hk
  | .ksing _ _ _ _, _, hx, i, k, k', hk => hx i k k' hk
  | .kstream _ _ _ _ _ _, _, hx, i, k, k', hk => hx i k k' hk
  | .pair a b, x, hx, i, k, k', hk => by
    show Trace.zip (SchedTick.slice a x.1 i k) (SchedTick.slice b x.2 i k)
      <+: Trace.zip (SchedTick.slice a x.1 i k') (SchedTick.slice b x.2 i k')
    exact zip_prefix (SchedTick.slice_mono a x.1 hx.1 i k k' hk)
      (SchedTick.slice_mono b x.2 hx.2 i k k' hk)

/-- Unslicing a monotone trace of tuples gives a leafwise-monotone shaped output. -/
theorem SchedTick.unslice_mono {ℓ : L} : (sh : TickShape) →
    (F : Fin (mem ℓ) → Nat → Trace (BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sh)) →
    (∀ i k k', k ≤ k' → F i k <+: F i k') →
    TMonoOf sh (SchedTick.unslice sh F)
  | .sing _, _, hF => hF
  | .stream _ _ _ _, _, hF => hF
  | .ksing _ _ _ _, _, hF => hF
  | .kstream _ _ _ _ _ _, _, hF => hF
  | .pair a b, _, hF =>
    ⟨SchedTick.unslice_mono a _ (fun i k k' hk => List.IsPrefix.map _ (hF i k k' hk)),
     SchedTick.unslice_mono b _ (fun i k k' hk => List.IsPrefix.map _ (hF i k k' hk))⟩

/-- The tick former is monotone: the scan of a prefix is a prefix of the scan. -/
theorem tmono_tick_scan {ℓ : L} (sts ins outs : TickShape)
    {x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ ins}
    (g : Fin (mem ℓ) → BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sts →
      BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr ins →
      BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sts × BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr outs)
    (init : SeedOf sts) (hx : TMonoOf ins x) :
    TMonoOf outs ((SchedSem L mem pacing).tick_scan sts ins outs x g init) :=
  SchedTick.unslice_mono outs _ (fun i k k' hk => by
    show scanAcrossTicksTrace (g i) (SchedTick.seed sts init) (SchedTick.slice ins x i k)
      <+: scanAcrossTicksTrace (g i) (SchedTick.seed sts init) (SchedTick.slice ins x i k')
    exact scanAcrossTicksTrace_prefix _ _ (SchedTick.slice_mono ins x hx i k k' hk))

/-! ## The tick knot

The diagonal is `fix_tick d body i t = (iterate body ⊥ (t+1)) i t`, ungated. Two facts
combine: the body's strictness in the knot wire stabilizes the stages (stage `t+1` and stage
`t'+1` agree at steps `≤ t`, `stab_iterate_tick_strict`), and the body's monotonicity makes
every stage a monotone trace. The diagonal at `t` is therefore stage `t'+1`'s entry at `t`,
a prefix of its entry at `t'`. -/

/-- Every stage of the iteration is monotone when the body preserves monotonicity. -/
theorem TMono.stages {n : Nat} {σ : Type}
    {A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hA : ∀ x, TMono x → TMono (A x)) :
    ∀ m, TMono (iterate A (fun _ _ => ([] : Trace σ)) m)
  | 0 => TMono.bot
  | m + 1 => hA _ (TMono.stages hA m)

/-- The raw tick diagonal of a strict, monotonicity-preserving body is step-monotone. -/
theorem tmono_tick_diag {n : Nat} {σ : Type}
    {A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hstrict : TStrict A)
    (hmono : ∀ x, TMono x → TMono (A x)) :
    TMono (fun i t => (iterate A (fun _ _ => ([] : Trace σ)) (t + 1)) i t) := by
  intro i t t' ht
  let X : Nat → Fin n → Nat → Trace σ := iterate A (fun _ _ => ([] : Trace σ))
  show X (t + 1) i t <+: X (t' + 1) i t'
  have hstab : X (t + 1) i t = X (t' + 1) i t :=
    stab_iterate_tick_strict (h := t) (L := 0) (A := A) (A' := A)
      (fun h' _ x y hxy => hstrict.graded h' x y hxy) t (t + 1) (t' + 1) (by omega)
      (Nat.lt_succ_self _) (by omega) i
  have hm : X (t' + 1) i t <+: X (t' + 1) i t' := TMono.stages hmono (t' + 1) i t t' ht
  rw [hstab]
  exact hm

/-- The tick knot is monotone for a strict, monotonicity-preserving body. -/
theorem tmono_fix_tick {ℓ : L} {σ : Type} (d : Unit)
    {body : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ}
    (hstrict : TStrict body)
    (hmono : ∀ x, TMono x → TMono (body x)) :
    TMono ((SchedSem L mem pacing).fix_tick d body) := by
  rw [fix_tick_eq_diag]
  exact tmono_tick_diag (A := body) hstrict hmono

end Hydro
