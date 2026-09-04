import Hydro.SchedMono

/-!
# Hydro · one tick entry per step (`TPaced`)

**A tick-located wire gains at most one entry per step, and none at a
step where its member is idle.** The machine's ticked carriers are
`Fin n → Nat → Trace σ` (the realized tick sequence as of each step);
every machine op appends to them one entry per realized tick — the
`tickSteps` ops at the tick itself, `fix_tick`'s knot wire one active
step later (the gated diagonal: the body's output of the last active
step). Nothing in the carrier's TYPE says so; this file is the
machine-side fact, per op (`tpaced_<op>`, the `tmono_<op>` pattern of
`SchedMono.lean`), and `Project/Causal.lean` lifts it to every wire of a
compiled graph (`Dag.denote_tpaced`).

It is what justifies the production interpreter's single-slot
increments (D85): `Kind.delta` of a tick-located kind is `Option` —
this step's entry or nothing — and `Kind.incAt` reads it as
`(incTrace …).head?`. `head?` forgets nothing exactly because of this
fact; stated as the lossless corollary `incTrace_eq_toList_head?`.

The statement is **per step**, not per tick: `fix_tick`'s knot wire
gains the tick's entry at the member's next *active* step (a stutter in
the lock-step model; the next tick in production, where every activation
ticks). "At most one per tick" is true of every other op and of every
production run; the per-step form is the one that closes under the knot.
-/

namespace Hydro

/-- **One entry per step, none when idle.** -/
def TPaced {n : Nat} {σ : Type} (act : Fin n → Nat → Bool) (x : Fin n → Nat → Trace σ) : Prop :=
  ∀ i k, (incTrace (x i) k).length ≤ 1 ∧ (act i k = false → incTrace (x i) k = [])

/-- Shaped tick tuples: paced leafwise. -/
@[reducible] def TPacedOf {L : Type} {mem : L → Nat} {ℓ : L} (act : Fin (mem ℓ) → Nat → Bool) :
    (sh : TickShape) →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh → Prop
  | .sing _, x => TPaced act x
  | .stream _ _ _ _, x => TPaced act x
  | .ksing _ _ _ _, x => TPaced act x
  | .kstream _ _ _ _ _ _, x => TPaced act x
  | .pair a b, x => TPacedOf act a x.1 ∧ TPacedOf act b x.2

/-- The trivial pacing fact of a non-tick wire. -/
def TPacedTriv {τ : Sort _} (_x : τ) : Prop := True
theorem TPacedTriv.intro {τ : Sort _} (x : τ) : TPacedTriv x := trivial

/-! ## Pacing is length arithmetic -/

/-- The length form of `TPaced`: the trace length starts at most `1`
(`0` if idle at step `0`), grows by at most `1` per step, and does not
grow at an idle step. -/
def LenPaced {n : Nat} (act : Fin n → Nat → Bool) (len : Fin n → Nat → Nat) : Prop :=
  (∀ i, len i 0 ≤ 1) ∧ (∀ i, act i 0 = false → len i 0 = 0) ∧
  (∀ i k, len i (k + 1) ≤ len i k + 1) ∧ (∀ i k, act i (k + 1) = false → len i (k + 1) ≤ len i k)

theorem TPaced.of_len {n : Nat} {σ : Type} {act : Fin n → Nat → Bool} {x : Fin n → Nat → Trace σ}
    (h : LenPaced act (fun i k => (x i k).length)) : TPaced act x := by
  obtain ⟨h0, h0a, hs, hsa⟩ := h
  simp only [] at h0 h0a hs hsa
  intro i k
  cases k with
  | zero =>
    refine ⟨h0 i, fun ha => ?_⟩
    exact List.eq_nil_of_length_eq_zero (h0a i ha)
  | succ k =>
    refine ⟨?_, fun ha => ?_⟩
    · show ((x i (k + 1)).drop (x i k).length).length ≤ 1
      rw [List.length_drop]; have := hs i k; omega
    · show (x i (k + 1)).drop (x i k).length = []
      exact List.drop_eq_nil_of_le (hsa i k ha)

theorem TPaced.len {n : Nat} {σ : Type} {act : Fin n → Nat → Bool} {x : Fin n → Nat → Trace σ}
    (h : TPaced act x) : LenPaced act (fun i k => (x i k).length) := by
  refine ⟨fun i => (h i 0).1, fun i ha => List.length_eq_zero_iff.mpr ((h i 0).2 ha),
    fun i k => ?_, fun i k ha => ?_⟩ <;> simp only []
  · have := (h i (k + 1)).1
    simp only [incTrace_succ, List.length_drop] at this; omega
  · have := (h i (k + 1)).2 ha
    simp only [incTrace_succ, List.drop_eq_nil_iff] at this; exact this

theorem TPaced.bot {n : Nat} {σ : Type} (act : Fin n → Nat → Bool) :
    TPaced act (fun (_ : Fin n) (_ : Nat) => ([] : Trace σ)) :=
  fun _ k => by cases k <;> exact ⟨Nat.zero_le _, fun _ => rfl⟩

/-- Pacing is a property of the lengths. -/
theorem TPaced.of_len_eq {n : Nat} {σ τ : Type} {act : Fin n → Nat → Bool}
    {x : Fin n → Nat → Trace σ} {y : Fin n → Nat → Trace τ}
    (hx : TPaced act x) (hlen : ∀ i k, (y i k).length = (x i k).length) : TPaced act y :=
  TPaced.of_len (by simpa only [hlen] using hx.len)

/-- Lengths bounded by a paced trace's lengths stay paced when the
difference never grows: `min` with a paced length. -/
theorem LenPaced.min {n : Nat} {act : Fin n → Nat → Bool} {a b : Fin n → Nat → Nat}
    (ha : LenPaced act a) (hb : LenPaced act b) : LenPaced act (fun i k => min (a i k) (b i k)) := by
  obtain ⟨a0, a0a, as, asa⟩ := ha
  obtain ⟨b0, b0a, bs, bsa⟩ := hb
  refine ⟨fun i => ?_, fun i h => ?_, fun i k => ?_, fun i k h => ?_⟩ <;> simp only []
  · have := a0 i; have := b0 i; omega
  · have := a0a i h; have := b0a i h; omega
  · have := as i k; have := bs i k; omega
  · have := asa i k h; have := bsa i k h; omega

variable {L : Type} {mem : L → Nat} {pacing : Pacing L mem}

/-- A ticking member is active. -/
theorem Pacing.active_of_ticks {ℓ : L} {i : Fin (mem ℓ)} {k : Nat}
    (h : pacing.ticks ℓ i k = true) : pacing.active ℓ i k = true := by
  unfold Pacing.ticks Pacing.active at *
  revert h; cases pacing ℓ i k <;> simp [Pace.ticks, Pace.active]

theorem tickSteps_length_zero_le (p : Nat → Bool) : (tickSteps p 0).length ≤ 1 := by
  unfold tickSteps
  simp only [List.range_succ, List.range_zero, List.nil_append, List.filter_cons, List.filter_nil]
  split <;> simp

theorem tickSteps_length_zero_of_not (p : Nat → Bool) (h : p 0 = false) : (tickSteps p 0).length = 0 := by
  unfold tickSteps
  simp [List.range_succ, h]

theorem tickSteps_length_succ_le (p : Nat → Bool) (k : Nat) :
    (tickSteps p (k + 1)).length ≤ (tickSteps p k).length + 1 := by
  unfold tickSteps
  rw [List.range_succ (n := k + 1), List.filter_append, List.length_append]
  simp only [List.filter_cons, List.filter_nil]
  split <;> simp

theorem tickSteps_length_succ_of_not (p : Nat → Bool) (k : Nat) (h : p (k + 1) = false) :
    (tickSteps p (k + 1)).length = (tickSteps p k).length := by
  unfold tickSteps
  rw [List.range_succ (n := k + 1), List.filter_append, List.length_append]
  simp [h]

theorem tickSteps_length_succ_of (p : Nat → Bool) (k : Nat) (h : p (k + 1) = true) :
    (tickSteps p (k + 1)).length = (tickSteps p k).length + 1 := by
  unfold tickSteps
  rw [List.range_succ (n := k + 1), List.filter_append, List.length_append]
  simp [h]

/-- **`defer_tick` on a Rust-expressible input.** When the input is never
more than one tick behind the tick count (every wire a Hydro program can
build: the `tickSteps` ops are exact, the knot wire one behind), the
released count is the tick count — the deferred wire is `(init :: t).take
ticks`, one entry per tick. The `deferReleased` form is the FIFO's own
semantics on *any* input, which is what the production node computes. -/
theorem deferReleased_eq_ticks (p : Nat → Bool) (len : Nat → Nat)
    (h : ∀ k, (tickSteps p k).length ≤ len k + 1) :
    ∀ k, deferReleased p len k = (tickSteps p k).length
  | 0 => by
    show (if p 0 then 1 else 0) = _
    unfold tickSteps
    simp only [List.range_succ, List.range_zero, List.nil_append, List.filter_cons, List.filter_nil]
    split <;> simp
  | k + 1 => by
    rw [deferReleased_succ, deferReleased_eq_ticks p len h k]
    cases hp : p (k + 1)
    · simp only [Bool.false_and, Bool.false_eq_true, if_false]
      exact (tickSteps_length_succ_of_not p k hp).symm
    · have := h (k + 1)
      rw [tickSteps_length_succ_of p k hp] at this ⊢
      simp only [Bool.true_and, decide_eq_true_eq]
      rw [if_pos (by omega)]

/-- The tick count of a member is paced by its activity. -/
theorem LenPaced.ticks (ℓ : L) :
    LenPaced (pacing.active ℓ) (fun i k => (tickSteps (pacing.ticks ℓ i) k).length) := by
  refine ⟨fun i => tickSteps_length_zero_le _, fun i h => ?_, fun i k => tickSteps_length_succ_le _ _,
    fun i k h => ?_⟩
  · apply tickSteps_length_zero_of_not
    cases ht : pacing.ticks ℓ i 0
    · rfl
    · exact absurd (Pacing.active_of_ticks ht) (by simp [h])
  · apply Nat.le_of_eq; apply tickSteps_length_succ_of_not
    cases ht : pacing.ticks ℓ i (k + 1)
    · rfl
    · exact absurd (Pacing.active_of_ticks ht) (by simp [h])

/-- A trace whose length is the tick count. -/
theorem TPaced.of_ticks {ℓ : L} {σ : Type} {x : Fin (mem ℓ) → Nat → Trace σ}
    (hlen : ∀ i k, (x i k).length = (tickSteps (pacing.ticks ℓ i) k).length) :
    TPaced (pacing.active ℓ) x :=
  TPaced.of_len (by simpa only [hlen] using LenPaced.ticks (pacing := pacing) ℓ)

/-! ## The stream → tick boundaries -/

theorem tpaced_snapshot {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {b : SingBound σ} (s : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b) (d : Unit) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).snapshot s d) :=
  TPaced.of_ticks (fun i k => by
    show ((tickSteps (pacing.ticks ℓ i) k).map (fun st => s i st)).length = _
    exact List.length_map ..)

theorem batchesFrom_length' {α : Type} (src : Nat → List α) :
    ∀ (ss : List Nat) (c : Nat), (batchesFrom src ss c).length = ss.length
  | [], _ => rfl
  | _ :: ss, _ => congrArg (· + 1) (batchesFrom_length' src ss _)

theorem tpaced_batch {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (s : (SchedSem L mem pacing).Stream ℓ α ord ret) (d : Unit) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).batch s d) :=
  TPaced.of_ticks (fun i k => by
    show (batchesFrom ((s i).view) (tickSteps (pacing.ticks ℓ i) k) 0).length = _
    exact batchesFrom_length' _ _ _)

theorem tpaced_time {ℓ : L} (d : (SchedSem L mem pacing).ClockDec (mem ℓ)) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).time d) :=
  TPaced.of_ticks (fun i k => by
    show (scriptTrace (d i) (tickSteps (pacing.ticks ℓ i) k).length).length = _
    exact scriptTrace_length _ _)

theorem tpaced_snapshot_bounded {ℓ : L} {σ : Type} (x : σ) (d : Unit) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).snapshot_bounded x d) :=
  TPaced.of_ticks (fun i k => by
    show (List.replicate (tickSteps (pacing.ticks ℓ i) k).length x).length = _
    exact List.length_replicate)

theorem tpaced_ksnapshot {ℓ : L} {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} {b : KSBound}
    (ks : (SchedSem L mem pacing).KeyedSingleton ℓ α K V ord ret b) (d : Unit) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).ksnapshot ks d) :=
  TPaced.of_ticks (fun i k => by
    show ((tickSteps (pacing.ticks ℓ i) k).map (fun st => ks i st)).length = _
    exact List.length_map ..)

/-! ## Pointwise tick ops -/

theorem tpaced_mapTick {ℓ : L} {α β : Type} {t : (SchedSem L mem pacing).Atomic ℓ α}
    (f : Fin (mem ℓ) → α → β) (ht : TPaced (pacing.active ℓ) t) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).mapTick t f) :=
  TPaced.of_len_eq ht (fun i k => by
    show ((t i k).map (f i)).length = _
    exact List.length_map ..)

theorem tpaced_firstTick {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries}
    {t : (SchedSem L mem pacing).AtomicStream ℓ α .totalOrder ret}
    (ht : TPaced (pacing.active ℓ) t) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).firstTick t) :=
  TPaced.of_len_eq ht (fun i k => by
    show ((t i k).map List.head?).length = _
    exact List.length_map ..)

theorem tpaced_zipTick {ℓ : L} {α β : Type} {a : (SchedSem L mem pacing).Atomic ℓ α}
    {b : (SchedSem L mem pacing).Atomic ℓ β}
    (ha : TPaced (pacing.active ℓ) a) (hb : TPaced (pacing.active ℓ) b) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).zipTick a b) :=
  TPaced.of_len (by
    have h := LenPaced.min ha.len hb.len
    have hl : ∀ i k, ((SchedSem L mem pacing).zipTick a b i k).length = min (a i k).length (b i k).length :=
      fun i k => List.length_zip
    simpa only [hl] using h)

/-- The released count of a member's deferred wire is paced by its activity. -/
theorem LenPaced.deferReleased (ℓ : L) (len : Fin (mem ℓ) → Nat → Nat) :
    LenPaced (pacing.active ℓ) (fun i k => deferReleased (pacing.ticks ℓ i) (len i) k) := by
  refine ⟨fun i => deferReleased_zero_le _ _, fun i h => ?_, fun i k => deferReleased_succ_le _ _ _,
    fun i k h => ?_⟩
  · apply deferReleased_zero_of_not
    cases ht : pacing.ticks ℓ i 0
    · rfl
    · exact absurd (Pacing.active_of_ticks ht) (by simp [h])
  · apply Nat.le_of_eq; apply deferReleased_of_not
    cases ht : pacing.ticks ℓ i (k + 1)
    · rfl
    · exact absurd (Pacing.active_of_ticks ht) (by simp [h])

/-- `defer_tick`: `(init :: t).take released` — bounded by the released
count, and by the input's length plus one. -/
theorem tpaced_defer_tick {ℓ : L} {σ : Type} (init : σ)
    {t : (SchedSem L mem pacing).Atomic ℓ σ} (ht : TPaced (pacing.active ℓ) t) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).defer_tick init t) := by
  apply TPaced.of_len
  have hl : ∀ i k, ((SchedSem L mem pacing).defer_tick init t i k).length
      = min (deferReleased (pacing.ticks ℓ i) (fun k => (t i k).length) k) ((t i k).length + 1) := fun i k => by
    show ((init :: t i k).take (deferReleased (pacing.ticks ℓ i) (fun k => (t i k).length) k)).length = _
    rw [List.length_take, List.length_cons]
  simp only [hl]
  obtain ⟨a0, a0a, as, asa⟩ := LenPaced.deferReleased (pacing := pacing) ℓ (fun i k => (t i k).length)
  obtain ⟨b0, b0a, bs, bsa⟩ := ht.len
  simp only [] at a0 a0a as asa b0 b0a bs bsa
  refine ⟨fun i => ?_, fun i h => ?_, fun i k => ?_, fun i k h => ?_⟩ <;> simp only []
  · have := a0 i; omega
  · have := a0a i h; omega
  · have := as i k; have := bs i k; omega
  · have := asa i k h; have := bsa i k h; omega

theorem tpaced_flattenOrdered {ℓ : L} {α : Type} [DecidableEq α]
    {t : (SchedSem L mem pacing).Atomic ℓ (List α)} (ht : TPaced (pacing.active ℓ) t) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).flattenOrdered t) := ht

theorem tpaced_flattenUnordered {ℓ : L} {α : Type} [DecidableEq α]
    {t : (SchedSem L mem pacing).Atomic ℓ (List α)} (ht : TPaced (pacing.active ℓ) t) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).flattenUnordered t) := ht

theorem tpaced_tinto_keyed {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {x : (SchedSem L mem pacing).AtomicStream ℓ (K × V) ord .exactlyOnce}
    (hx : TPaced (pacing.active ℓ) x) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).tinto_keyed (K := K) (V := V) x) := hx

theorem tpaced_tassume_ordering {ℓ : L} {α : Type} [DecidableEq α]
    {u : (SchedSem L mem pacing).AtomicStream ℓ α .noOrder .exactlyOnce}
    (d : Unit) (hu : TPaced (pacing.active ℓ) u) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).tassume_ordering u d) := hu

theorem tpaced_reduce_watermark {ℓ : L} {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd}
    {ks : (SchedSem L mem pacing).AtomicKeyedStream ℓ K V ord .exactlyOnce}
    {wm : (SchedSem L mem pacing).Atomic ℓ (Option K)}
    (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOk ord .exactlyOnce Pre (reduceStep f))
    (hks : TPaced (pacing.active ℓ) ks) (hwm : TPaced (pacing.active ℓ) wm) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).reduce_watermark ks wm f Pre ok) := by
  apply TPaced.of_len
  have h := LenPaced.min hks.len hwm.len
  have hl : ∀ i k, ((SchedSem L mem pacing).reduce_watermark ks wm f Pre ok i k).length
      = min (ks i k).length (wm i k).length := fun i k => by
    show (scanAcrossTicksTrace (reduceWatermarkEagerH f) (SchedKMap.empty, none)
      (Trace.zip (ks i k) (wm i k))).length = _
    rw [scanAcrossTicksTrace_length]; exact List.length_zip
  simpa only [hl] using h

theorem tpaced_tfold {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk ord ret Pre g)
    {bs : (SchedSem L mem pacing).AtomicStream ℓ α ord ret} (hbs : TPaced (pacing.active ℓ) bs) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).tfold g init Pre ok bs) :=
  TPaced.of_len_eq hbs (fun i k => by
    show (tfoldHist g init (bs i k)).length = _
    rw [tfoldHist_eq_fold]; exact foldAcrossTicksTrace_length ..)

/-! ## The shaped tick former -/

theorem SchedTick.slice_lenPaced {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh) →
    TPacedOf (pacing.active ℓ) sh x →
    LenPaced (pacing.active ℓ) (fun i k => (SchedTick.slice sh x i k).length)
  | .sing _, _, hx => hx.len
  | .stream _ _ _ _, _, hx => hx.len
  | .ksing _ _ _ _, _, hx => hx.len
  | .kstream _ _ _ _ _ _, _, hx => hx.len
  | .pair a b, x, hx => by
    have h := LenPaced.min (SchedTick.slice_lenPaced a x.1 hx.1) (SchedTick.slice_lenPaced b x.2 hx.2)
    simpa only [SchedTick.slice, Trace.zip, List.length_zip] using h

theorem SchedTick.unslice_paced {ℓ : L} : (sh : TickShape) →
    (F : Fin (mem ℓ) → Nat → Trace (BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sh)) →
    LenPaced (pacing.active ℓ) (fun i k => (F i k).length) →
    TPacedOf (pacing.active ℓ) sh (SchedTick.unslice sh F)
  | .sing _, _, hF => TPaced.of_len hF
  | .stream _ _ _ _, _, hF => TPaced.of_len hF
  | .ksing _ _ _ _, _, hF => TPaced.of_len hF
  | .kstream _ _ _ _ _ _, _, hF => TPaced.of_len hF
  | .pair a b, F, hF =>
    ⟨SchedTick.unslice_paced a _ (by simpa only [List.length_map] using hF),
     SchedTick.unslice_paced b _ (by simpa only [List.length_map] using hF)⟩

theorem tpaced_tick_scan {ℓ : L} (sts ins outs : TickShape)
    {x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ ins}
    (g : Fin (mem ℓ) → BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sts →
      BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr ins →
      BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sts × BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr outs)
    (init : SeedOf sts) (hx : TPacedOf (pacing.active ℓ) ins x) :
    TPacedOf (pacing.active ℓ) outs ((SchedSem L mem pacing).tick_scan sts ins outs x g init) :=
  SchedTick.unslice_paced outs _ (by
    have h := SchedTick.slice_lenPaced (pacing := pacing) ins x hx
    simpa only [scanAcrossTicksTrace_length] using h)

/-! ## The tick knot

The knot wire is the gated diagonal: at an active step `t+1` it is the
body's output as of step `t`. Two length facts: the body's output is at
most one entry ahead of its gated view (it grows only at active steps,
by one; the gate catches up at the next active step), so the gated view
grows by at most one per step — and, by stabilization (`stab_iterate_tick`),
the diagonal's increment at `t+1` is one stage's increment. -/

section TickKnot

variable {n : Nat} {σ : Type}

/-- The body's output is at most one entry ahead of its gated view. -/
theorem gateView_len_behind (q : Nat → Bool) (f : Nat → List σ)
    (hf0 : (f 0).length ≤ 1) (hfs : ∀ t, (f (t + 1)).length ≤ (f t).length + 1)
    (hfa : ∀ t, q (t + 1) = false → (f (t + 1)).length ≤ (f t).length) :
    ∀ t, (f t).length ≤ (gateView q f t).length + 1
  | 0 => by simpa [gateView] using hf0
  | t + 1 => by
    rw [gateView_succ]
    cases hq : q (t + 1)
    · have := gateView_len_behind q f hf0 hfs hfa t
      have := hfa t hq
      simp only [Bool.false_eq_true, if_false]; omega
    · simp only [if_true]; exact hfs t

theorem gateView_paced {act : Fin n → Nat → Bool} {f : Fin n → Nat → Trace σ}
    (hf : TPaced act f) :
    TPaced act (fun i => gateView (act i) (f i)) := by
  obtain ⟨f0, f0a, fs, fsa⟩ := hf.len
  simp only [] at f0 f0a fs fsa
  refine TPaced.of_len ⟨fun i => ?_, fun i _ => ?_, fun i k => ?_, fun i k h => ?_⟩
  · show (gateView (act i) (f i) 0).length ≤ 1; simp [gateView]
  · show (gateView (act i) (f i) 0).length = 0; simp [gateView]
  · show (gateView (act i) (f i) (k + 1)).length ≤ (gateView (act i) (f i) k).length + 1
    rw [gateView_succ]
    split
    · exact gateView_len_behind (act i) (f i) (f0 i) (fs i) (fsa i) k
    · omega
  · show (gateView (act i) (f i) (k + 1)).length ≤ (gateView (act i) (f i) k).length
    rw [gateView_succ, if_neg (by simp [h])]

/-- Every stage of the gated iteration is paced when the body preserves
pacing. -/
theorem TPaced.tickshift {act : Fin n → Nat → Bool}
    {A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hA : ∀ x, TPaced act x → TPaced act (A x)) :
    ∀ m, TPaced act (iterate (τ := Fin n → Nat → Trace σ)
      (fun x j => gateView (act j) ((A x) j))
      (fun _ _ => ([] : Trace σ)) m)
  | 0 => TPaced.bot act
  | m + 1 => gateView_paced (hA _ (TPaced.tickshift hA m))

/-- The raw tick diagonal of a causal, pacing-preserving body is paced. -/
theorem tpaced_tick_diag {act : Fin n → Nat → Bool}
    {A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hcaus : ∀ h' (x y : Fin n → Nat → Trace σ), TAgree h' x y → TAgree h' (A x) (A y))
    (hA : ∀ x, TPaced act x → TPaced act (A x)) :
    TPaced act (fun i t => (iterate (τ := Fin n → Nat → Trace σ)
      (fun x j => gateView (act j) ((A x) j))
      (fun _ _ => ([] : Trace σ)) (t + 1)) i t) := by
  intro i k
  let X : Nat → Fin n → Nat → Trace σ := iterate (τ := Fin n → Nat → Trace σ)
    (fun x j => gateView (act j) ((A x) j)) (fun _ _ => ([] : Trace σ))
  cases k with
  | zero =>
    show (X 1 i 0).length ≤ 1 ∧ (act i 0 = false → X 1 i 0 = [])
    exact ⟨by simp [X, gateView], fun _ => by simp [X, gateView]⟩
  | succ k =>
    -- the diagonal at `k` is stage `k+2`'s entry at `k` (stabilization)
    have hstab : X (k + 1) i k = X (k + 2) i k :=
      stab_iterate_tick (h := k) (A := A) (A' := A) act
        (fun h' _ x y hxy => hcaus h' x y hxy) k (k + 1) (k + 2) (Nat.le_refl _)
        (Nat.lt_succ_self _) (by omega) i
    have hp := TPaced.tickshift hA (k + 2) i (k + 1)
    show ((X (k + 2) i (k + 1)).drop (X (k + 1) i k).length).length ≤ 1
      ∧ (act i (k + 1) = false → (X (k + 2) i (k + 1)).drop (X (k + 1) i k).length = [])
    rw [hstab]
    exact hp

end TickKnot

theorem tpaced_fix_tick {ℓ : L} {σ : Type} (d : Unit)
    {body : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ}
    (hcaus : ∀ h' (x y : (SchedSem L mem pacing).Atomic ℓ σ),
      TAgree h' x y → TAgree h' (body x) (body y))
    (hpaced : ∀ x, TPaced (pacing.active ℓ) x → TPaced (pacing.active ℓ) (body x)) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).fix_tick d body) :=
  tpaced_tick_diag (A := body) (act := fun j => pacing.active ℓ j) hcaus hpaced

/-! ## The lossless single slot -/

/-- Under pacing, a step's increment is its `head?`, as a list. -/
theorem incTrace_eq_toList_head? {n : Nat} {σ : Type} {act : Fin n → Nat → Bool}
    {x : Fin n → Nat → Trace σ} (h : TPaced act x) (i : Fin n) (k : Nat) :
    (incTrace (x i) k).head?.toList = incTrace (x i) k := by
  have := (h i k).1
  match hl : incTrace (x i) k with
  | [] => rfl
  | [_] => rfl
  | _ :: _ :: _ => simp [hl] at this

end Hydro
