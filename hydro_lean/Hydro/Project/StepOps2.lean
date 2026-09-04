import Hydro.Project.StepOps

/-!
# Hydro · Project — per-operator step lemmas (the buffered cells)

`batch`/`batch_ordered` (the pending buffer and the consumed count),
`zipTick` (unmatched suffixes), `reduce_watermark` and `tick_scan`
(unmatched suffixes + the scan register), `sample_every` (the trace so
far + the position in the sample list).
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} (pacing : Pacing L mem)

/-! ## Batches -/

theorem consumedAfter_append_single {α : Type} (src : Nat → List α) :
    ∀ (steps : List Nat) (u c : Nat), consumedAfter src (steps ++ [u]) c = (src u).length
  | [], _, _ => rfl
  | _ :: rest, u, _ => consumedAfter_append_single src rest u _

/-- The consumed count before step `t`. -/
def prevConsumed {α : Type} (p : Nat → Bool) (h : StepHist α) (t : Nat) : Nat :=
  consumedAfter h.view (prevTicks p t) 0

theorem prevConsumed_le {α : Type} (p : Nat → Bool) (h : StepHist α) (t : Nat) :
    prevConsumed p h t ≤ (prevView h t).length := by
  cases t with
  | zero => exact Nat.le_refl _
  | succ t =>
    exact consumedAfter_le h.view (fun s s' hs => h.mono_le hs) _ 0 t (tickSteps_le p t) (Nat.zero_le _)

/-- The batch cell: pending buffer and emitted batch at step `t`. -/
theorem batch_cell_step {α : Type} (p : Nat → Bool) (h : StepHist α) (t : Nat) :
    let pend := (prevView h t).drop (prevConsumed p h t) ++ incView h t
    ((if p t then [] else pend) = (h.view t).drop (consumedAfter h.view (tickSteps p t) 0)) ∧
    ((if p t then [pend] else []) = incTrace (fun k => batchesFrom h.view (tickSteps p k) 0) t) := by
  intro pend
  have hpend : pend = (h.view t).drop (prevConsumed p h t) := by
    show (prevView h t).drop (prevConsumed p h t) ++ incView h t = _
    rw [view_eq_prev_inc h t, List.drop_append_of_le_length (prevConsumed_le p h t)]
  constructor
  · rw [tickSteps_eq_prev]
    split
    · rw [consumedAfter_append_single, List.drop_length]
    · rw [List.append_nil]
      exact hpend
  · cases t with
    | zero =>
      show _ = batchesFrom h.view (tickSteps p 0) 0
      rw [tickSteps_zero]
      split
      · show [pend] = [(h.view 0).drop 0]
        rw [hpend]; rfl
      · rfl
    | succ t =>
      show _ = (batchesFrom h.view (tickSteps p (t + 1)) 0).drop (batchesFrom h.view (tickSteps p t) 0).length
      rw [tickSteps_succ]
      split
      · rw [batchesFrom_append, List.drop_left, hpend]
        rfl
      · rw [List.append_nil, List.drop_length]

theorem spec_batch {Γ : Ctx L mem} {ℓ : L} {α : Type} (iα : DecidableEq α)
    (x : Var Γ (.stream ℓ α iα .noOrder .exactlyOnce)) (d : Unit)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.batch iα x d) () env t := by
  have hp : Node.prevState pacing (.batch iα x d) () env t
      = fun i => (prevView (env.get x i) t).drop (prevConsumed (pacing.ticks ℓ i) (env.get x i) t) := by
    cases t <;> rfl
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rw [hp]
  show ((fun i => if pacing.ticks ℓ i t then [] else (prevView (env.get x i) t).drop (prevConsumed (pacing.ticks ℓ i) (env.get x i) t) ++ incView (env.get x i) t),
      ((fun i => if pacing.ticks ℓ i t then [(prevView (env.get x i) t).drop (prevConsumed (pacing.ticks ℓ i) (env.get x i) t) ++ incView (env.get x i) t] else [] : Fin (mem ℓ) → List (List α)), ()))
    = ((fun i => ((env.get x i).view t).drop (consumedAfter ((env.get x i).view) (tickSteps (pacing.ticks ℓ i) t) 0)),
      ((fun i => incTrace (fun k => batchesFrom ((env.get x i).view) (tickSteps (pacing.ticks ℓ i) k) 0) t : Fin (mem ℓ) → List (List α)), ()))
  congr 2
  · funext i; exact (batch_cell_step (pacing.ticks ℓ i) (env.get x i) t).1
  · funext i; exact (batch_cell_step (pacing.ticks ℓ i) (env.get x i) t).2

theorem spec_batch_ordered {Γ : Ctx L mem} {ℓ : L} {α : Type} (iα : DecidableEq α)
    (x : Var Γ (.stream ℓ α iα .totalOrder .exactlyOnce)) (d : Unit)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.batch_ordered iα x d) () env t := by
  have hp : Node.prevState pacing (.batch_ordered iα x d) () env t
      = fun i => (prevView (env.get x i) t).drop (prevConsumed (pacing.ticks ℓ i) (env.get x i) t) := by
    cases t <;> rfl
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rw [hp]
  show ((fun i => if pacing.ticks ℓ i t then [] else (prevView (env.get x i) t).drop (prevConsumed (pacing.ticks ℓ i) (env.get x i) t) ++ incView (env.get x i) t),
      ((fun i => if pacing.ticks ℓ i t then [(prevView (env.get x i) t).drop (prevConsumed (pacing.ticks ℓ i) (env.get x i) t) ++ incView (env.get x i) t] else [] : Fin (mem ℓ) → List (List α)), ()))
    = ((fun i => ((env.get x i).view t).drop (consumedAfter ((env.get x i).view) (tickSteps (pacing.ticks ℓ i) t) 0)),
      ((fun i => incTrace (fun k => batchesFrom ((env.get x i).view) (tickSteps (pacing.ticks ℓ i) k) 0) t : Fin (mem ℓ) → List (List α)), ()))
  congr 2
  · funext i; exact (batch_cell_step (pacing.ticks ℓ i) (env.get x i) t).1
  · funext i; exact (batch_cell_step (pacing.ticks ℓ i) (env.get x i) t).2

/-! ## Zipped traces: the unmatched suffixes -/

/-- The jointly realized length before step `t`. -/
def prevZipLen {α β : Type} (a : Nat → Trace α) (b : Nat → Trace β) : Nat → Nat
  | 0 => 0
  | t + 1 => min (a t).length (b t).length

theorem prevZipLen_le_left {α β : Type} (a : Nat → Trace α) (b : Nat → Trace β)
    (ha : ∀ k, a k <+: a (k + 1)) (t : Nat) : prevZipLen a b t ≤ (a t).length := by
  cases t with
  | zero => exact Nat.zero_le _
  | succ t => exact (Nat.min_le_left _ _).trans (ha t).length_le

theorem prevZipLen_le_right {α β : Type} (a : Nat → Trace α) (b : Nat → Trace β)
    (hb : ∀ k, b k <+: b (k + 1)) (t : Nat) : prevZipLen a b t ≤ (b t).length := by
  cases t with
  | zero => exact Nat.zero_le _
  | succ t => exact (Nat.min_le_right _ _).trans (hb t).length_le

/-- The previous unmatched suffix plus this step's ticks is this step's
trace beyond the previously matched length. -/
theorem pending_push {α : Type} (a : Nat → Trace α) (ha : ∀ k, a k <+: a (k + 1)) (t m : Nat)
    (hm : m ≤ (prevTrace a t).length) :
    (prevTrace a t).drop m ++ incTrace a t = (a t).drop m := by
  rw [trace_eq_prev_inc a ha t, List.drop_append_of_le_length hm]

theorem prevZipLen_le_prev_left {α β : Type} (a : Nat → Trace α) (b : Nat → Trace β) (t : Nat) :
    prevZipLen a b t ≤ (prevTrace a t).length := by
  cases t with
  | zero => exact Nat.le_refl _
  | succ t => exact Nat.min_le_left _ _

theorem prevZipLen_le_prev_right {α β : Type} (a : Nat → Trace α) (b : Nat → Trace β) (t : Nat) :
    prevZipLen a b t ≤ (prevTrace b t).length := by
  cases t with
  | zero => exact Nat.le_refl _
  | succ t => exact Nat.min_le_right _ _

/-- The new zipped pairs at step `t`. -/
theorem zip_inc {α β : Type} (a : Nat → Trace α) (b : Nat → Trace β)
    (ha : ∀ k, a k <+: a (k + 1)) (hb : ∀ k, b k <+: b (k + 1)) (t : Nat) :
    List.zip ((a t).drop (prevZipLen a b t)) ((b t).drop (prevZipLen a b t))
      = incTrace (fun k => List.zip (a k) (b k)) t := by
  cases t with
  | zero => rfl
  | succ t =>
    show List.zip ((a (t + 1)).drop _) ((b (t + 1)).drop _)
      = (List.zip (a (t + 1)) (b (t + 1))).drop (List.zip (a t) (b t)).length
    rw [List.length_zip, drop_zip]
    rfl

theorem zip_drop_len {α β : Type} (a : List α) (b : List β) (m : Nat) :
    min (a.drop m).length (b.drop m).length = min a.length b.length - m := by
  simp only [List.length_drop]
  omega

theorem spec_zipTick {Γ : Ctx L mem} {ℓ : L} {α β : Type} (a : Var Γ (.atomic ℓ α))
    (b : Var Γ (.atomic ℓ β)) (env : Env Γ) (t : Nat)
    (ha : TMono (env.get a)) (hb : TMono (env.get b)) :
    NodeSpec pacing (.zipTick a b) () env t := by
  have hp : Node.prevState pacing (.zipTick a b) () env t
      = fun i => ((prevTrace (env.get a i) t).drop (prevZipLen (env.get a i) (env.get b i) t),
                  (prevTrace (env.get b i) t).drop (prevZipLen (env.get a i) (env.get b i) t)) := by
    cases t <;> rfl
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get, Kind.incAt]
  rw [hp]
  have hpa : ∀ i, (prevTrace (env.get a i) t).drop (prevZipLen (env.get a i) (env.get b i) t) ++ incTrace (env.get a i) t
      = (env.get a i t).drop (prevZipLen (env.get a i) (env.get b i) t) := fun i =>
    pending_push _ (fun k => ha i k (k + 1) (Nat.le_succ k)) t _ (prevZipLen_le_prev_left _ _ t)
  have hpb : ∀ i, (prevTrace (env.get b i) t).drop (prevZipLen (env.get a i) (env.get b i) t) ++ incTrace (env.get b i) t
      = (env.get b i t).drop (prevZipLen (env.get a i) (env.get b i) t) := fun i =>
    pending_push _ (fun k => hb i k (k + 1) (Nat.le_succ k)) t _ (prevZipLen_le_prev_right _ _ t)
  simp only [hpa, hpb]
  show ((fun i => (((env.get a i t).drop _).drop _, ((env.get b i t).drop _).drop _)),
      ((fun i => List.zip ((env.get a i t).drop _) ((env.get b i t).drop _) : Fin (mem ℓ) → List (α × β)), ()))
    = ((fun i => ((env.get a i t).drop (min (env.get a i t).length (env.get b i t).length),
                  (env.get b i t).drop (min (env.get a i t).length (env.get b i t).length))),
      ((fun i => incTrace (fun k => Trace.zip (env.get a i k) (env.get b i k)) t : Fin (mem ℓ) → List (α × β)), ()))
  congr 2
  · funext i
    have hle : prevZipLen (env.get a i) (env.get b i) t ≤ min (env.get a i t).length (env.get b i t).length :=
      Nat.le_min.mpr ⟨prevZipLen_le_left _ _ (fun k => ha i k (k + 1) (Nat.le_succ k)) t,
        prevZipLen_le_right _ _ (fun k => hb i k (k + 1) (Nat.le_succ k)) t⟩
    rw [zip_drop_len, List.drop_drop, List.drop_drop, Nat.add_sub_cancel' hle]
  · funext i
    exact zip_inc _ _ (fun k => ha i k (k + 1) (Nat.le_succ k)) (fun k => hb i k (k + 1) (Nat.le_succ k)) t

/-! ## The watermark reduce: zipped inputs + a scan register -/

/-- Scanning a prefix then its continuation. -/
theorem scanRun_prefix_inc {ι σ β : Type} (g : σ → ι → σ × β) (s : σ) (l l' : List ι)
    (h : l <+: l') :
    scanRun g (scanRun g s l).1 (l'.drop l.length) = ((scanRun g s l').1, (scanRun g s l').2.drop (scanRun g s l).2.length) := by
  obtain ⟨e, rfl⟩ := h
  rw [List.drop_left, scanRun_append]
  show _ = (_, ((scanRun g s l).2 ++ _).drop (scanRun g s l).2.length)
  rw [List.drop_left]

theorem spec_reduce_watermark {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iH : Hashable K) (iV : DecidableEq V)
    (lo : LinearOrder K) {ord : StrOrd} (x : Var Γ (.atomicKeyed ℓ K V iK iV ord .exactlyOnce))
    (wm : Var Γ (.atomic ℓ (Option K))) (f : V → V → V) (Pre : V → V → Prop)
    (iPre : DecidableRel Pre) (ok : FoldOk ord .exactlyOnce Pre (reduceStep f))
    (env : Env Γ) (t : Nat) (hx : TMono (env.get x)) (hw : TMono (env.get wm)) :
    NodeSpec pacing (.reduce_watermark iK iH iV lo x wm f Pre iPre ok) () env t := by
  -- the zipped input traces, as of each step
  let Z : Fin (mem ℓ) → Nat → Trace (List (K × V) × Option K) :=
    fun i k => Trace.zip (env.get x i k) (env.get wm i k)
  have hZmono : ∀ i k, Z i k <+: Z i (k + 1) := fun i k =>
    zip_prefix (hx i k (k + 1) (Nat.le_succ k)) (hw i k (k + 1) (Nat.le_succ k))
  have hp : Node.prevState pacing (.reduce_watermark iK iH iV lo x wm f Pre iPre ok) () env t
      = fun i => (((prevTrace (env.get x i) t).drop (prevZipLen (env.get x i) (env.get wm i) t),
                   (prevTrace (env.get wm i) t).drop (prevZipLen (env.get x i) (env.get wm i) t)),
                  (scanRun (reduceWatermarkEager f) (SchedKMap.empty, none) (prevTrace (Z i) t)).1) := by
    cases t <;> rfl
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get, Kind.incAt]
  rw [hp]
  have hpa : ∀ i, (prevTrace (env.get x i) t).drop (prevZipLen (env.get x i) (env.get wm i) t) ++ incTrace (env.get x i) t
      = (env.get x i t).drop (prevZipLen (env.get x i) (env.get wm i) t) := fun i =>
    pending_push _ (fun k => hx i k (k + 1) (Nat.le_succ k)) t _ (prevZipLen_le_prev_left _ _ t)
  have hpb : ∀ i, (prevTrace (env.get wm i) t).drop (prevZipLen (env.get x i) (env.get wm i) t) ++ incTrace (env.get wm i) t
      = (env.get wm i t).drop (prevZipLen (env.get x i) (env.get wm i) t) := fun i =>
    pending_push _ (fun k => hw i k (k + 1) (Nat.le_succ k)) t _ (prevZipLen_le_prev_right _ _ t)
  simp only [hpa, hpb]
  have hle : ∀ i, prevZipLen (env.get x i) (env.get wm i) t ≤ min (env.get x i t).length (env.get wm i t).length :=
    fun i => Nat.le_min.mpr ⟨prevZipLen_le_left _ _ (fun k => hx i k (k + 1) (Nat.le_succ k)) t,
        prevZipLen_le_right _ _ (fun k => hw i k (k + 1) (Nat.le_succ k)) t⟩
  -- the zipped new pairs are the zipped trace beyond the previous zipped length
  have hzip : ∀ i, List.zip ((env.get x i t).drop (prevZipLen (env.get x i) (env.get wm i) t))
      ((env.get wm i t).drop (prevZipLen (env.get x i) (env.get wm i) t))
      = (Z i t).drop (prevTrace (Z i) t).length := by
    intro i
    rw [← drop_zip]
    cases t with
    | zero => rfl
    | succ t => show (Z i (t + 1)).drop (min _ _) = (Z i (t + 1)).drop (Z i t).length; rw [List.length_zip]
  have hrun : ∀ i, scanRun (reduceWatermarkEager f) (scanRun (reduceWatermarkEager f) (SchedKMap.empty, none) (prevTrace (Z i) t)).1
      (List.zip ((env.get x i t).drop (prevZipLen (env.get x i) (env.get wm i) t))
        ((env.get wm i t).drop (prevZipLen (env.get x i) (env.get wm i) t)))
      = ((scanRun (reduceWatermarkEager f) (SchedKMap.empty, none) (Z i t)).1,
         (scanRun (reduceWatermarkEager f) (SchedKMap.empty, none) (Z i t)).2.drop
           (scanRun (reduceWatermarkEager f) (SchedKMap.empty, none) (prevTrace (Z i) t)).2.length) := by
    intro i
    rw [hzip]
    exact scanRun_prefix_inc _ _ _ _ (by
      cases t with
      | zero => exact List.nil_prefix
      | succ t => exact hZmono i t)
  unfold reduceWatermarkEagerH
  simp only [hrun]
  show ((fun i => ((((env.get x i t).drop _).drop _, ((env.get wm i t).drop _).drop _), _)),
      ((fun i => (scanRun (reduceWatermarkEager f) (SchedKMap.empty, none) (Z i t)).2.drop _ : Fin (mem ℓ) → List (SchedKMap K V)), ()))
    = ((fun i => (((env.get x i t).drop (min (env.get x i t).length (env.get wm i t).length),
                   (env.get wm i t).drop (min (env.get x i t).length (env.get wm i t).length)),
                  (scanRun (reduceWatermarkEager f) (SchedKMap.empty, none) (Z i t)).1)),
      ((fun i => incTrace (fun k => scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none) (Z i k)) t :
        Fin (mem ℓ) → List (SchedKMap K V)), ()))
  congr 2
  · funext i
    rw [zip_drop_len, List.drop_drop, List.drop_drop, Nat.add_sub_cancel' (hle i)]
  · funext i
    have hsc : (fun k => scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none) (Z i k))
        = fun k => (scanRun (reduceWatermarkEager f) (SchedKMap.empty, none) (Z i k)).2 := by
      funext k; rw [scanRun_eq]
    rw [hsc]
    cases t with
    | zero =>
      show (scanRun _ _ (Z i 0)).2.drop (scanRun _ _ []).2.length = (scanRun _ _ (Z i 0)).2
      rfl
    | succ t => rfl

end Project

end Hydro
