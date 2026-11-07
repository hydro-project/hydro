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
    show ((batchesFrom ((s i).view) (tickSteps (pacing.ticks ℓ i) k) 0).map List.toArray).length = _
    rw [List.length_map]
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
    show ((t i k).map (fun b => b[0]?)).length = _
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

/-- `defer_tick`: `(init :: t (k-1)).take (released k)` — bounded by the
released count, and by the input's length at the previous step plus one
(D95: release before push; the input must be step-monotone for the released
count to stay within the buffer — `TMono`, every tick wire's fact). -/
theorem tpaced_defer_tick {ℓ : L} {σ : Type} (init : σ)
    {t : (SchedSem L mem pacing).Atomic ℓ σ} (hm : TMono t) (ht : TPaced (pacing.active ℓ) t) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).defer_tick init t) := by
  apply TPaced.of_len
  have hl : ∀ i k, ((SchedSem L mem pacing).defer_tick init t i k).length
      = min (deferReleased (pacing.ticks ℓ i) (fun k => (t i (k - 1)).length) k) ((t i (k - 1)).length + 1) :=
    fun i k => by
    show ((init :: t i (k - 1)).take (deferReleased (pacing.ticks ℓ i) (fun k => (t i (k - 1)).length) k)).length = _
    rw [List.length_take, List.length_cons]
  -- the released count stays within the buffer as of the previous step
  have hbound : ∀ i k, deferReleased (pacing.ticks ℓ i) (fun k => (t i (k - 1)).length) k
      ≤ (t i (k - 1)).length + 1 := fun i k =>
    deferReleased_le_len_succ _ _ (fun k => (hm i (k - 1) (k + 1 - 1) (by omega)).length_le) k
  obtain ⟨a0, a0a, as, asa⟩ := LenPaced.deferReleased (pacing := pacing) ℓ (fun i k => (t i (k - 1)).length)
  obtain ⟨b0, b0a, bs, bsa⟩ := ht.len
  simp only [] at a0 a0a as asa b0 b0a bs bsa
  refine ⟨fun i => ?_, fun i h => ?_, fun i k => ?_, fun i k h => ?_⟩ <;> simp only []
  · rw [hl]; have := a0 i; omega
  · rw [hl]; have := a0a i h; omega
  · rw [hl, hl]
    have h1 := as i k
    cases k with
    | zero => have := hbound i 0; omega
    | succ k =>
      have := bs i k
      simp only [Nat.add_sub_cancel] at *
      omega
  · rw [hl, hl]
    have h1 := asa i k h
    have h2 := hbound i k
    cases k with
    | zero => omega
    | succ k =>
      simp only [Nat.add_sub_cancel] at *
      have := (hm i k (k + 1) (Nat.le_succ k)).length_le
      omega

theorem tpaced_flattenOrdered {ℓ : L} {α : Type} [DecidableEq α]
    {t : (SchedSem L mem pacing).Atomic ℓ (List α)} (ht : TPaced (pacing.active ℓ) t) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).flattenOrdered t) :=
  TPaced.of_len_eq ht (fun i k => by
    show ((t i k).map List.toArray).length = _
    exact List.length_map ..)

theorem tpaced_flattenUnordered {ℓ : L} {α : Type} [DecidableEq α]
    {t : (SchedSem L mem pacing).Atomic ℓ (List α)} (ht : TPaced (pacing.active ℓ) t) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).flattenUnordered t) :=
  TPaced.of_len_eq ht (fun i k => by
    show ((t i k).map List.toArray).length = _
    exact List.length_map ..)

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
      (Trace.zip ((ks i k).map Array.toList) (wm i k))).length = _
    rw [scanAcrossTicksTrace_length, List.length_zip, List.length_map]
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

/-- Every stage of the iteration is paced when the body preserves pacing. -/
theorem TPaced.stages {act : Fin n → Nat → Bool}
    {A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hA : ∀ x, TPaced act x → TPaced act (A x)) :
    ∀ m, TPaced act (iterate A (fun _ _ => ([] : Trace σ)) m)
  | 0 => TPaced.bot act
  | m + 1 => hA _ (TPaced.stages hA m)

/-- The raw tick diagonal of a strict, pacing-preserving body is paced
(D95: the diagonal's entry at `k` is stage `k+2`'s, by stabilization, so
its increment at `k+1` is one stage's increment). -/
theorem tpaced_tick_diag {act : Fin n → Nat → Bool}
    {A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hstrict : TStrict A)
    (hA : ∀ x, TPaced act x → TPaced act (A x)) :
    TPaced act (fun i t => (iterate A (fun _ _ => ([] : Trace σ)) (t + 1)) i t) := by
  intro i k
  let X : Nat → Fin n → Nat → Trace σ := iterate A (fun _ _ => ([] : Trace σ))
  cases k with
  | zero =>
    have hp := TPaced.stages hA 1 i 0
    exact hp
  | succ k =>
    have hstab : X (k + 1) i k = X (k + 2) i k :=
      stab_iterate_tick_strict (h := k) (L := 0) (A := A) (A' := A)
        (fun h' _ x y hxy => hstrict.graded h' x y hxy) k (k + 1) (k + 2) (by omega)
        (Nat.lt_succ_self _) (by omega) i
    have hp := TPaced.stages hA (k + 2) i (k + 1)
    show ((X (k + 2) i (k + 1)).drop (X (k + 1) i k).length).length ≤ 1
      ∧ (act i (k + 1) = false → (X (k + 2) i (k + 1)).drop (X (k + 1) i k).length = [])
    rw [hstab]
    exact hp

end TickKnot

theorem tpaced_fix_tick {ℓ : L} {σ : Type} (d : Unit)
    {body : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ}
    (hstrict : TStrict body)
    (hpaced : ∀ x, TPaced (pacing.active ℓ) x → TPaced (pacing.active ℓ) (body x)) :
    TPaced (pacing.active ℓ) ((SchedSem L mem pacing).fix_tick d body) := by
  rw [fix_tick_eq_diag]
  exact tpaced_tick_diag (A := body) (act := fun j => pacing.active ℓ j) hstrict hpaced

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
