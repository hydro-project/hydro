import Hydro.Project.StepOps2

/-!
# Hydro · Project — per-operator step lemmas (`sample_every`, `tick_scan`)
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} (pacing : Pacing L mem)

/-! ## Sampling -/

theorem sampleAtOpt_nil_tr {α : Type} (us : List Nat) : sampleAtOpt ([] : Trace (Option α)) us = [] := by
  cases us with
  | nil => rfl
  | cons u us => simp [sampleAtOpt]

theorem samplePos_nil_tr {α : Type} (us : List Nat) : samplePos ([] : Trace (Option α)) us = 0 := by
  cases us with
  | nil => rfl
  | cons u us => simp [samplePos]

/-- Walking from a position is the walk over the suffix. -/
theorem sampleAdvance_spec {α : Type} (tr : Trace (Option α)) (us : List Nat) :
    ∀ pos, sampleAdvance tr us pos = (pos + samplePos tr (us.drop pos), sampleAtOpt tr (us.drop pos)) := by
  intro pos
  generalize hn : us.length - pos = n
  induction n using Nat.strong_induction_on generalizing pos with
  | _ n ih =>
    rw [sampleAdvance]
    split
    · next hnone =>
      have := List.getElem?_eq_none_iff.mp hnone
      rw [List.drop_eq_nil_of_le this]
      rfl
    · next u hu =>
      have hlt := (List.getElem?_eq_some_iff.mp hu).1
      have hdrop : us.drop pos = u :: us.drop (pos + 1) := by
        rw [List.drop_eq_getElem_cons hlt]
        simp [List.getElem?_eq_some_iff.mp hu |>.2]
      rw [hdrop]
      have hrec := ih (us.length - (pos + 1)) (by omega) (pos + 1) rfl
      rcases hx : tr[u]? with _ | _ | x
      · have hnl : ¬ u < tr.length := by
          have := List.getElem?_eq_none_iff.mp hx
          omega
        simp only [sampleAtOpt, samplePos, hx, hnl, if_false]
        rfl
      · have hul : u < tr.length := (List.getElem?_eq_some_iff.mp hx).1
        simp only [sampleAtOpt, samplePos, hx, hul, if_true]
        rw [hrec]
        congr 1; omega
      · have hul : u < tr.length := (List.getElem?_eq_some_iff.mp hx).1
        simp only [sampleAtOpt, samplePos, hx, hul, if_true]
        rw [hrec]
        show (_, x :: _) = (_, x :: _)
        congr 1; omega

/-- Below a prefix's length, the samples agree. -/
theorem sampleAtOpt_split {α : Type} {tr tr' : Trace (Option α)} (h : tr <+: tr') :
    ∀ us, sampleAtOpt tr' us = sampleAtOpt tr us ++ sampleAtOpt tr' (us.drop (samplePos tr us)) ∧
      samplePos tr' us = samplePos tr us + samplePos tr' (us.drop (samplePos tr us))
  | [] => by simp [sampleAtOpt, samplePos]
  | u :: us => by
    obtain ⟨ih1, ih2⟩ := sampleAtOpt_split h us
    by_cases hu : u < tr.length
    · have hu' : u < tr'.length := Nat.lt_of_lt_of_le hu h.length_le
      have hget : tr'[u]? = tr[u]? := by
        rw [List.getElem?_eq_getElem hu, List.getElem?_eq_getElem hu']
        exact congrArg some (h.getElem hu).symm
      rcases hc : tr[u]? with _ | _ | x
      · exact absurd (List.getElem?_eq_none_iff.mp hc) (Nat.not_le.mpr hu)
      · simp only [sampleAtOpt, samplePos, hget, hc, hu, hu', if_true, List.drop_succ_cons]
        exact ⟨ih1, by omega⟩
      · simp only [sampleAtOpt, samplePos, hget, hc, hu, hu', if_true, List.drop_succ_cons]
        exact ⟨by rw [ih1]; rfl, by omega⟩
    · have hnone : tr[u]? = none := List.getElem?_eq_none_iff.mpr (Nat.le_of_not_lt hu)
      refine ⟨?_, ?_⟩ <;> simp [sampleAtOpt, samplePos, hu]

theorem sampleAtOpt_prefix {α : Type} {tr tr' : Trace (Option α)} (h : tr <+: tr') (us : List Nat) :
    sampleAtOpt tr us <+: sampleAtOpt tr' us :=
  ⟨_, ((sampleAtOpt_split h us).1).symm⟩

theorem spec_sample_every {Γ : Ctx L mem} {ℓ : L} {α : Type} (iα : DecidableEq α)
    (x : Var Γ (.atomic ℓ (Option α))) (d : SampleTimes (mem ℓ)) (env : Env Γ) (t : Nat)
    (hx : TMono (env.get x)) : NodeSpec pacing (.sample_every iα x d) env t := by
  have hp : Node.prevState pacing (.sample_every iα x d) env t
      = fun i => (prevTrace (env.get x i) t, samplePos (prevTrace (env.get x i) t) (d i)) := by
    cases t with
    | zero => funext i; show ([], 0) = ([], samplePos [] (d i)); rw [samplePos_nil_tr]
    | succ t => rfl
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get, Kind.incAt]
  rw [hp]
  have htr : ∀ i, prevTrace (env.get x i) t ++ incTrace (env.get x i) t = env.get x i t :=
    fun i => (trace_eq_prev_inc _ (fun k => hx i k (k + 1) (Nat.le_succ k)) t).symm
  have hpre : ∀ i, prevTrace (env.get x i) t <+: env.get x i t := fun i => ⟨_, htr i⟩
  simp only [htr, sampleAdvance_spec]
  have hraw : ∀ k, famFreeze (fun step j => sampleAtOpt (env.get x j step) (d j)) k
      = fun j => sampleAtOpt (env.get x j k) (d j) :=
    famFreeze_eq_of_mono _ (fun k j => sampleAtOpt_prefix (hx j k (k + 1) (Nat.le_succ k)) (d j))
  show ((fun i => (env.get x i t, samplePos (prevTrace (env.get x i) t) (d i)
          + samplePos (env.get x i t) ((d i).drop (samplePos (prevTrace (env.get x i) t) (d i))))),
      ((fun i => sampleAtOpt (env.get x i t) ((d i).drop (samplePos (prevTrace (env.get x i) t) (d i))) :
        Fin (mem ℓ) → List α), ()))
    = ((fun i => (env.get x i t, samplePos (env.get x i t) (d i))),
      ((fun i => incView (famHist (fun step j => sampleAtOpt (env.get x j step) (d j)) i) t : Fin (mem ℓ) → List α), ()))
  congr 2
  · funext i
    rw [(sampleAtOpt_split (hpre i) (d i)).2]
  · funext i
    have hsplit := (sampleAtOpt_split (hpre i) (d i)).1
    cases t with
    | zero =>
      show _ = famFreeze (fun step j => sampleAtOpt (env.get x j step) (d j)) 0 i
      rw [hraw]
      show _ = sampleAtOpt (env.get x i 0) (d i)
      rw [hsplit]
      rw [show prevTrace (env.get x i) 0 = [] from rfl, sampleAtOpt_nil_tr, samplePos_nil_tr, List.nil_append]
    | succ t =>
      show _ = (famFreeze (fun step j => sampleAtOpt (env.get x j step) (d j)) (t + 1) i).drop
        (famFreeze (fun step j => sampleAtOpt (env.get x j step) (d j)) t i).length
      rw [hraw, hraw]
      show _ = (sampleAtOpt (env.get x i (t + 1)) (d i)).drop (sampleAtOpt (env.get x i t) (d i)).length
      rw [hsplit]
      show _ = (sampleAtOpt (env.get x i t) (d i) ++ _).drop (sampleAtOpt (env.get x i t) (d i)).length
      rw [List.drop_left]

/-! ## Tick blocks: shaped unmatched suffixes + the register -/

/-- Leaf traces of a shaped input are step-monotone. -/
theorem TMonoOf_leaf_mono {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) → TMonoOf sh x →
    ∀ (i : Fin (mem ℓ)) (k : Nat), SchedTick.slice sh x i k <+: SchedTick.slice sh x i (k + 1)
  | sh, x, hx, i, k => SchedTick.slice_mono sh x hx i k (k + 1) (Nat.le_succ k)

/-- The per-leaf increments of a shaped input, read off the increment environment. -/
theorem readShapeDelta_envIncAt {Γ : Ctx L mem} (env : Env Γ) (t : Nat) (ℓ : L) :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) →
    readShapeDelta (envIncAt env t) ℓ sh vs = envIncAt (shapeToList ℓ sh (readShape env ℓ sh vs)) t
  | .sing _, v => by show (_, ()) = (_, ()); rw [envIncAt_get]; rfl
  | .stream _ _ _ _, v => by show (_, ()) = (_, ()); rw [envIncAt_get]; rfl
  | .ksing _ _ _ _, v => by show (_, ()) = (_, ()); rw [envIncAt_get]; rfl
  | .pair a b, (va, vb) => by
    show (readShapeDelta (envIncAt env t) ℓ a va).append (readShapeDelta (envIncAt env t) ℓ b vb) = _
    rw [readShapeDelta_envIncAt env t ℓ a va, readShapeDelta_envIncAt env t ℓ b vb]
    show _ = envIncAt ((shapeToList ℓ a _).append (shapeToList ℓ b _)) t
    rw [envIncAt_append]

/-- The shaped family of previous traces (leafwise `prevTrace`). -/
def shapePrev {ℓ : L} : (sh : TickShape) →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh
  | .sing _, x => fun i k => prevTrace (x i) k
  | .stream _ _ _ _, x => fun i k => prevTrace (x i) k
  | .ksing _ _ _ _, x => fun i k => prevTrace (x i) k
  | .pair a b, x => (shapePrev a x.1, shapePrev b x.2)

theorem slice_shapePrev {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) →
    (i : Fin (mem ℓ)) → (k : Nat) →
    SchedTick.slice sh (shapePrev sh x) i k = prevTrace (fun k => SchedTick.slice sh x i k) k
  | .sing _, _, _, _ => rfl
  | .stream _ _ _ _, _, _, _ => rfl
  | .ksing _ _ _ _, _, _, _ => rfl
  | .pair a b, x, i, k => by
    show Trace.zip (SchedTick.slice a (shapePrev a x.1) i k) (SchedTick.slice b (shapePrev b x.2) i k) = _
    rw [slice_shapePrev a x.1 i k, slice_shapePrev b x.2 i k]
    cases k <;> rfl

theorem pendingAt_zero {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) → (i : Fin (mem ℓ)) →
    pendingAt sh (shapePrev sh x) i 0 0 = PendingOf.empty sh
  | .sing _, _, _ => rfl
  | .stream _ _ _ _, _, _ => rfl
  | .ksing _ _ _ _, _, _ => rfl
  | .pair a b, x, i => by
    show (pendingAt a (shapePrev a x.1) i 0 0, pendingAt b (shapePrev b x.2) i 0 0) = _
    rw [pendingAt_zero a x.1 i, pendingAt_zero b x.2 i]
    rfl

/-- The slice's length bounds every leaf's. -/
theorem slice_length_le_leaf {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) →
    (i : Fin (mem ℓ)) → (k : Nat) → PendingOf.avail sh (pendingAt sh x i k 0) = (SchedTick.slice sh x i k).length
  | .sing _, _, _, _ => rfl
  | .stream _ _ _ _, _, _, _ => rfl
  | .ksing _ _ _ _, _, _, _ => rfl
  | .pair a b, x, i, k => by
    show min (PendingOf.avail a (pendingAt a x.1 i k 0)) (PendingOf.avail b (pendingAt b x.2 i k 0))
      = (Trace.zip (SchedTick.slice a x.1 i k) (SchedTick.slice b x.2 i k)).length
    rw [slice_length_le_leaf a x.1 i k, slice_length_le_leaf b x.2 i k, List.length_zip]

/-- Pushing this step's leaf increments onto the previous unmatched
suffixes gives this step's leaves beyond the previously matched length. -/
theorem pendingAt_push {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) → TMonoOf sh x →
    (i : Fin (mem ℓ)) → (t m : Nat) → m ≤ (prevTrace (fun k => SchedTick.slice sh x i k) t).length →
    PendingOf.push sh (pendingAt sh (shapePrev sh x) i t m)
      (fun _ => envIncAt (shapeToList ℓ sh x) t) i = pendingAt sh x i t m
  | .sing _, x, hx, i, t, m, hm =>
    pending_push (x i) (fun k => hx i k (k + 1) (Nat.le_succ k)) t m hm
  | .stream _ _ _ _, x, hx, i, t, m, hm =>
    pending_push (x i) (fun k => hx i k (k + 1) (Nat.le_succ k)) t m hm
  | .ksing _ _ _ _, x, hx, i, t, m, hm =>
    pending_push (x i) (fun k => hx i k (k + 1) (Nat.le_succ k)) t m hm
  | .pair a b, x, hx, i, t, m, hm => by
    have hlen : (prevTrace (fun k => SchedTick.slice (.pair a b) x i k) t).length
        = min (prevTrace (fun k => SchedTick.slice a x.1 i k) t).length
            (prevTrace (fun k => SchedTick.slice b x.2 i k) t).length := by
      cases t with
      | zero => rfl
      | succ t => exact List.length_zip
    rw [hlen] at hm
    have hma := (Nat.le_min.mp hm).1
    have hmb := (Nat.le_min.mp hm).2
    show (PendingOf.push a (pendingAt a (shapePrev a x.1) i t m)
        (fun _ => HList.take (envIncAt ((shapeToList ℓ a x.1).append (shapeToList ℓ b x.2)) t)) i,
      PendingOf.push b (pendingAt b (shapePrev b x.2) i t m)
        (fun _ => HList.drop (envIncAt ((shapeToList ℓ a x.1).append (shapeToList ℓ b x.2)) t)) i)
      = (pendingAt a x.1 i t m, pendingAt b x.2 i t m)
    rw [envIncAt_append, HList.take_append, HList.drop_append,
      pendingAt_push a x.1 hx.1 i t m hma, pendingAt_push b x.2 hx.2 i t m hmb]

/-- The available tuple count beyond `m` is the slice's length beyond `m`. -/
theorem pendingAt_avail {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) →
    (i : Fin (mem ℓ)) → (t m : Nat) →
    PendingOf.avail sh (pendingAt sh x i t m) = (SchedTick.slice sh x i t).length - m
  | .sing _, _, _, _, _ => List.length_drop
  | .stream _ _ _ _, _, _, _, _ => List.length_drop
  | .ksing _ _ _ _, _, _, _, _ => List.length_drop
  | .pair a b, x, i, t, m => by
    show min (PendingOf.avail a (pendingAt a x.1 i t m)) (PendingOf.avail b (pendingAt b x.2 i t m))
      = (Trace.zip (SchedTick.slice a x.1 i t) (SchedTick.slice b x.2 i t)).length - m
    rw [pendingAt_avail a x.1 i t m, pendingAt_avail b x.2 i t m, List.length_zip]
    omega

theorem pendingAt_takeZip {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) →
    (i : Fin (mem ℓ)) → (t m n : Nat) →
    PendingOf.takeZip sh (pendingAt sh x i t m) n = ((SchedTick.slice sh x i t).drop m).take n
  | .sing _, _, _, _, _, _ => rfl
  | .stream _ _ _ _, _, _, _, _, _ => rfl
  | .ksing _ _ _ _, _, _, _, _, _ => rfl
  | .pair a b, x, i, t, m, n => by
    show List.zip (PendingOf.takeZip a (pendingAt a x.1 i t m) n) (PendingOf.takeZip b (pendingAt b x.2 i t m) n)
      = ((Trace.zip (SchedTick.slice a x.1 i t) (SchedTick.slice b x.2 i t)).drop m).take n
    rw [pendingAt_takeZip a x.1 i t m n, pendingAt_takeZip b x.2 i t m n, ← List.take_zip', ← drop_zip]

theorem pendingAt_dropN {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) →
    (i : Fin (mem ℓ)) → (t m n : Nat) →
    PendingOf.dropN sh (pendingAt sh x i t m) n = pendingAt sh x i t (m + n)
  | .sing _, _, _, _, _, _ => List.drop_drop
  | .stream _ _ _ _, _, _, _, _, _ => List.drop_drop
  | .ksing _ _ _ _, _, _, _, _, _ => List.drop_drop
  | .pair a b, x, i, t, m, n => by
    show (PendingOf.dropN a (pendingAt a x.1 i t m) n, PendingOf.dropN b (pendingAt b x.2 i t m) n) = _
    rw [pendingAt_dropN a x.1 i t m n, pendingAt_dropN b x.2 i t m n]
    rfl

/-- The outputs' leaf increments are the unzipped scan increments. -/
theorem envIncAt_unslice {ℓ : L} : (sh : TickShape) →
    (F : Fin (mem ℓ) → Nat → Trace (BoundedOf BS BStr BKS sh)) → (t : Nat) →
    envIncAt (shapeToList ℓ sh (SchedTick.unslice sh F)) t = unzipShape ℓ sh (fun i => incTrace (F i) t)
  | .sing _, _, _ => rfl
  | .stream _ _ _ _, _, _ => rfl
  | .ksing _ _ _ _, _, _ => rfl
  | .pair a b, F, t => by
    show envIncAt ((shapeToList ℓ a (SchedTick.unslice a (fun i k => (F i k).map Prod.fst))).append
        (shapeToList ℓ b (SchedTick.unslice b (fun i k => (F i k).map Prod.snd)))) t = _
    rw [envIncAt_append, envIncAt_unslice a _ t, envIncAt_unslice b _ t]
    show _ = (unzipShape ℓ a (fun i => (incTrace (F i) t).map Prod.fst)).append
      (unzipShape ℓ b (fun i => (incTrace (F i) t).map Prod.snd))
    congr 1
    · congr 1; funext i; exact incTrace_map (F i) Prod.fst t
    · congr 1; funext i; exact incTrace_map (F i) Prod.snd t

theorem pendingAt_shapePrev_succ {ℓ : L} : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) →
    (i : Fin (mem ℓ)) → (t m : Nat) →
    pendingAt sh (shapePrev sh x) i (t + 1) m = pendingAt sh x i t m
  | .sing _, _, _, _, _ => rfl
  | .stream _ _ _ _, _, _, _, _ => rfl
  | .ksing _ _ _ _, _, _, _, _ => rfl
  | .pair a b, x, i, t, m => by
    show (pendingAt a (shapePrev a x.1) i (t + 1) m, pendingAt b (shapePrev b x.2) i (t + 1) m) = _
    rw [pendingAt_shapePrev_succ a x.1 i t m, pendingAt_shapePrev_succ b x.2 i t m]
    rfl

theorem spec_tick_scan {Γ : Ctx L mem} {ℓ : L} (sts ins outs : TickShape) (x : VarsOf Γ ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS BStr BKS sts → BoundedOf BS BStr BKS ins →
      BoundedOf BS BStr BKS sts × BoundedOf BS BStr BKS outs)
    (seed : SeedOf sts) (env : Env Γ) (t : Nat) (hx : TMonoOf ins (readShape env ℓ ins x)) :
    NodeSpec pacing (.tick_scan sts ins outs x g seed) env t := by
  have hp : Node.prevState pacing (.tick_scan sts ins outs x g seed) env t
      = fun i => (pendingAt ins (shapePrev ins (readShape env ℓ ins x)) i t
                    (prevTrace (fun k => SchedTick.slice ins (readShape env ℓ ins x) i k) t).length,
                  (scanRun (g i) (SchedTick.seed sts seed)
                    (prevTrace (fun k => SchedTick.slice ins (readShape env ℓ ins x) i k) t)).1) := by
    cases t with
    | zero =>
      funext i
      show (PendingOf.empty ins, SchedTick.seed sts seed) = (pendingAt ins (shapePrev ins _) i 0 0, _)
      rw [pendingAt_zero]
      rfl
    | succ t =>
      funext i
      show (pendingAt ins (readShape env ℓ ins x) i t _, _) = _
      rw [pendingAt_shapePrev_succ]
      rfl
  simp only [NodeSpec, Node.step, Node.denote, Node.stateOf]
  rw [hp, readShapeDelta_envIncAt]
  generalize readShape env ℓ ins x = X at hx ⊢
  have hSl : ∀ i k, SchedTick.slice ins X i k <+: SchedTick.slice ins X i (k + 1) :=
    fun i k => TMonoOf_leaf_mono ins X hx i k
  have hpush : ∀ i, PendingOf.push ins (pendingAt ins (shapePrev ins X) i t
      (prevTrace (fun k => SchedTick.slice ins X i k) t).length)
      (fun _ => envIncAt (shapeToList ℓ ins X) t) i
      = pendingAt ins X i t (prevTrace (fun k => SchedTick.slice ins X i k) t).length :=
    fun i => pendingAt_push ins X hx i t _ (Nat.le_refl _)
  have hprefix : ∀ i, prevTrace (fun k => SchedTick.slice ins X i k) t <+: SchedTick.slice ins X i t := fun i => by
    cases t with
    | zero => exact List.nil_prefix
    | succ t => exact hSl i t
  have hle : ∀ i, (prevTrace (fun k => SchedTick.slice ins X i k) t).length ≤ (SchedTick.slice ins X i t).length :=
    fun i => (hprefix i).length_le
  simp only [hpush, pendingAt_avail, pendingAt_takeZip, pendingAt_dropN]
  have htake : ∀ i, ((SchedTick.slice ins X i t).drop (prevTrace (fun k => SchedTick.slice ins X i k) t).length).take
      ((SchedTick.slice ins X i t).length - (prevTrace (fun k => SchedTick.slice ins X i k) t).length)
      = (SchedTick.slice ins X i t).drop (prevTrace (fun k => SchedTick.slice ins X i k) t).length := fun i => by
    rw [← List.length_drop]
    exact List.take_length
  have hrun : ∀ i, scanRun (g i) (scanRun (g i) (SchedTick.seed sts seed) (prevTrace (fun k => SchedTick.slice ins X i k) t)).1
      ((SchedTick.slice ins X i t).drop (prevTrace (fun k => SchedTick.slice ins X i k) t).length)
      = ((scanRun (g i) (SchedTick.seed sts seed) (SchedTick.slice ins X i t)).1,
         (scanRun (g i) (SchedTick.seed sts seed) (SchedTick.slice ins X i t)).2.drop
           (scanRun (g i) (SchedTick.seed sts seed) (prevTrace (fun k => SchedTick.slice ins X i k) t)).2.length) :=
    fun i => scanRun_prefix_inc _ _ _ _ (hprefix i)
  simp only [htake, hrun, Nat.add_sub_cancel' (hle _)]
  show _ = (_, envIncAt (shapeToList ℓ outs (SchedTick.unslice outs
    (fun i k => scanAcrossTicksTrace (g i) (SchedTick.seed sts seed) (SchedTick.slice ins X i k)))) t)
  rw [envIncAt_unslice]
  congr 1
  congr 1
  funext i
  have hsc : (fun k => scanAcrossTicksTrace (g i) (SchedTick.seed sts seed) (SchedTick.slice ins X i k))
      = fun k => (scanRun (g i) (SchedTick.seed sts seed) (SchedTick.slice ins X i k)).2 := by
    funext k; rw [scanRun_eq]
  rw [hsc]
  cases t with
  | zero => rfl
  | succ t => rfl

end Project

end Hydro
