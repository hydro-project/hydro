import Hydro.Project.Model.StepOps5

/-!
# Hydro · Project — per-operator step lemmas (D81: the bench client's vocabulary)

The keyed wires (`kentries`/`kmap`/`kallTicks`: stateless casts and maps,
`kfold`: the arrivals so far with the hashed keyed fold recomputed,
`ksnapshot`: a tick read of the keyed singleton's current map), the
across-ticks fold (`tfold`: a scan over the new batches), the `Monotonic`
latest (`latest_monotone`: the latest present value, updated by the new
ticks), and the two `sliced!` sources (`time`: the clock reading at each
tick, `snapshot_bounded`: the constant at each tick).
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} (pacing : Pacing L mem)

/-! ## The keyed wires -/

theorem spec_kentries {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V)
    {ord : StrOrd} (x : Var Γ (.keyed ℓ K V iK iV ord .exactlyOnce))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.kentries iK iV x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rfl

theorem spec_kmap {Γ : Ctx L mem} {ℓ : L} {K V W : Type} (iK : DecidableEq K) (iV : DecidableEq V)
    (iW : DecidableEq W) {ord : StrOrd} (x : Var Γ (.keyed ℓ K V iK iV ord .exactlyOnce))
    (f : Fin (mem ℓ) → V → W) (env : Env Γ) (t : Nat) :
    NodeSpec pacing (.kmap iK iV iW x f) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incView (env.get x i) t).map (fun e => (e.1, f i e.2)) : Fin (mem ℓ) → List (K × W)), ()))
    = ((), ((fun i => incView ((env.get x i).map (fun e => (e.1, f i e.2))) t : Fin (mem ℓ) → List (K × W)), ()))
  congr 2
  funext i
  rw [incView_map]

theorem spec_kallTicks {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V)
    {ord : StrOrd} (x : Var Γ (.atomicKeyed ℓ K V iK iV ord .exactlyOnce)) (env : Env Γ) (t : Nat)
    (hx : TMono (env.get x)) (hpx : TPaced (pacing.active ℓ) (env.get x)) :
    NodeSpec pacing (.kallTicks iK iV x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incTrace (env.get x i) t).head?.getD [] : Fin (mem ℓ) → List (K × V)), ()))
    = ((), ((fun i => incView (famHist (fun step j => (env.get x j step).flatten) i) t : Fin (mem ℓ) → List (K × V)), ()))
  congr 2
  funext i
  rw [head?_getD_eq_flatten _ (hpx i t).1]
  have hraw : ∀ k, famFreeze (fun step j => (env.get x j step).flatten) k = fun j => (env.get x j k).flatten :=
    famFreeze_eq_of_mono _ (fun k j => prefix_flatten (hx j k (k + 1) (Nat.le_succ k)))
  cases t with
  | zero =>
    show (env.get x i 0).flatten = famFreeze (fun step j => (env.get x j step).flatten) 0 i
    rw [hraw]
  | succ t =>
    show ((env.get x i (t + 1)).drop (env.get x i t).length).flatten
      = (famFreeze (fun step j => (env.get x j step).flatten) (t + 1) i).drop
        (famFreeze (fun step j => (env.get x j step).flatten) t i).length
    rw [hraw, hraw]
    show _ = (env.get x i (t + 1)).flatten.drop (env.get x i t).flatten.length
    rw [trace_succ_inc (env.get x i) (fun k => hx i k (k + 1) (Nat.le_succ k)) t, List.flatten_append]
    simp only [List.drop_left]

theorem spec_kfold {Γ : Ctx L mem} {ℓ : L} {K V A : Type} (iK : DecidableEq K) (iH : Hashable K)
    (iV : DecidableEq V) (iA : DecidableEq A) {ord : StrOrd} (g : A → V → A) (init : A)
    (Pre : V → V → Prop) (iPre : DecidableRel Pre) (ok : FoldOk ord .exactlyOnce Pre g)
    (x : Var Γ (.keyed ℓ K V iK iV ord .exactlyOnce)) (env : Env Γ) (t : Nat) :
    NodeSpec pacing (.kfold iK iH iV iA g init Pre iPre ok x) () env t := by
  -- the cell is the fold of the arrivals so far; folding this step's in is the fold of
  -- the view (`kfoldStepH_keyedFoldAssocH`)
  have hs : ∀ i, kfoldStepH g init (Node.prevState pacing (.kfold iK iH iV iA g init Pre iPre ok x) () env t i)
      ((envIncAt env t).get x i) = keyedFoldAssocH g init ((env.get x i).view t) := by
    intro i
    rw [envIncAt_get]
    cases t with
    | zero =>
      show kfoldStepH (K := K) g init (keyedFoldAssocH g init []) _ = _
      rw [kfoldStepH_keyedFoldAssocH, List.nil_append]
      rfl
    | succ t =>
      show kfoldStepH (K := K) g init (keyedFoldAssocH g init ((env.get x i).view t)) (incView (env.get x i) (t + 1)) = _
      rw [kfoldStepH_keyedFoldAssocH, ← view_succ_inc]
  simp only [NodeSpec, Node.step]
  show ((fun i => kfoldStepH g init (Node.prevState pacing (.kfold iK iH iV iA g init Pre iPre ok x) () env t i)
        ((envIncAt env t).get x i)),
      ((fun i => kfoldStepH g init (Node.prevState pacing (.kfold iK iH iV iA g init Pre iPre ok x) () env t i)
        ((envIncAt env t).get x i)), ())) = _
  simp only [hs]
  rfl

theorem spec_ksnapshot {Γ : Ctx L mem} {ℓ : L} {α K V : Type} (iα : DecidableEq α) (iK : DecidableEq K)
    (iV : DecidableEq V) {ord : StrOrd} {ret : Retries} {b : KSBound}
    (x : Var Γ (.ksing ℓ α K V iα iK iV ord ret b)) (d : Unit)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.ksnapshot iα iK iV x d) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => if pacing.ticks ℓ i t then some (env.get x i t) else none : Fin (mem ℓ) → Option (SchedKMap K V)), ()))
    = ((), ((fun i => (incTrace (fun k => (tickSteps (pacing.ticks ℓ i) k).map (fun st => env.get x i st)) t).head? :
        Fin (mem ℓ) → Option (SchedKMap K V)), ()))
  congr 2
  funext i
  rw [incTrace_ticks_map]
  split <;> rfl

/-! ## The across-ticks fold -/

theorem spec_tfold {Γ : Ctx L mem} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) (iPre : DecidableRel Pre)
    (ok : FoldOk ord ret Pre g) (x : Var Γ (.atomicStream ℓ α iα ord ret)) (env : Env Γ) (t : Nat)
    (hx : TMono (env.get x)) (hpx : TPaced (pacing.active ℓ) (env.get x)) :
    NodeSpec pacing (.tfold iα g init Pre iPre ok x) () env t := by
  have hp : Node.prevState pacing (.tfold iα g init Pre iPre ok x) () env t
      = fun i => (scanRun (tfoldStep g) init (prevTrace (env.get x i) t)).1 := by
    cases t <;> rfl
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get, Kind.incAt]
  simp only [incTrace_eq_toList_head? hpx]
  rw [hp]
  have hrun : ∀ i, scanRun (tfoldStep g) (scanRun (tfoldStep g) init (prevTrace (env.get x i) t)).1
      (incTrace (env.get x i) t)
      = ((scanRun (tfoldStep g) init (env.get x i t)).1,
         (scanRun (tfoldStep g) init (env.get x i t)).2.drop
           (scanRun (tfoldStep g) init (prevTrace (env.get x i) t)).2.length) := by
    intro i
    have hinc : incTrace (env.get x i) t = (env.get x i t).drop (prevTrace (env.get x i) t).length := by
      cases t <;> rfl
    rw [hinc]
    exact scanRun_prefix_inc _ _ _ _ (by
      cases t with
      | zero => exact List.nil_prefix
      | succ t => exact hx i t (t + 1) (Nat.le_succ t))
  simp only [hrun]
  show ((fun i => (scanRun (tfoldStep g) init (env.get x i t)).1),
      ((fun i => ((scanRun (tfoldStep g) init (env.get x i t)).2.drop _).head? : Fin (mem ℓ) → Option σ), ()))
    = ((fun i => (scanRun (tfoldStep g) init (env.get x i t)).1),
      ((fun i => (incTrace (fun k => tfoldHist g init (env.get x i k)) t).head? : Fin (mem ℓ) → Option σ), ()))
  congr 2
  funext i
  have hsc : (fun k => tfoldHist g init (env.get x i k)) = fun k => (scanRun (tfoldStep g) init (env.get x i k)).2 := by
    funext k; rw [scanRun_eq]; rfl
  rw [hsc]
  cases t with
  | zero =>
    show ((scanRun _ _ (env.get x i 0)).2.drop (scanRun _ _ []).2.length).head? = ((scanRun _ _ (env.get x i 0)).2).head?
    rfl
  | succ t => rfl

/-! ## The `Monotonic` latest -/

/-- A single slot's `latest` update: the slot's value replaces the cell. -/
theorem slot_lastUpdate {σ : Type} (l : List (Option σ)) (h : l.length ≤ 1) (s : Option σ) :
    (match l.head? with
      | some o => o
      | none => s) = l.foldl (fun _acc o => o) s := by
  match l, h with
  | [], _ => rfl
  | [o], _ => rfl
  | _ :: _ :: _, h => simp at h

theorem lastUpdate_append {σ : Type} (l l' : Trace (Option σ)) :
    lastUpdate (l ++ l') = l'.foldl (fun _acc o => o) (lastUpdate l) := by
  induction l' generalizing l with
  | nil => simp
  | cons o l' ih =>
    rw [show l ++ o :: l' = (l ++ [o]) ++ l' from by simp, ih (l ++ [o]), List.foldl_cons]
    congr 1
    unfold lastUpdate
    simp [List.getLast?_append]

theorem spec_latest {Γ : Ctx L mem} {ℓ : L} {σ : Type} (x : Var Γ (.atomic ℓ (Option σ)))
    (env : Env Γ) (t : Nat) (hx : TMono (env.get x)) (hpx : TPaced (pacing.active ℓ) (env.get x)) :
    ∀ iσ, NodeSpec pacing (.latest iσ x) () env t := by
  intro iσ
  have hraw : ∀ k, famFreeze (fun s j => env.get x j s) k = fun j => env.get x j k :=
    famFreeze_eq_of_mono _ (fun k j => hx j k (k + 1) (Nat.le_succ k))
  have hs : ∀ i, (incTrace (env.get x i) t).foldl (fun _acc o => o)
      (Node.prevState pacing (.latest iσ x) () env t i) = lastUpdate (env.get x i t) := by
    intro i
    cases t with
    | zero =>
      show (incTrace (env.get x i) 0).foldl (fun _acc o => o) none = lastUpdate (env.get x i 0)
      have h0 : (none : Option σ) = lastUpdate [] := rfl
      rw [h0, ← lastUpdate_append]
      rfl
    | succ t =>
      show (incTrace (env.get x i) (t + 1)).foldl (fun _acc o => o) (lastUpdate (env.get x i t)) = _
      rw [← lastUpdate_append, ← trace_succ_inc (env.get x i) (fun k => hx i k (k + 1) (Nat.le_succ k)) t]
  simp only [NodeSpec, Node.step, Node.denote, Kind.incAt, envIncAt_get]
  have hm : ∀ i, (match (incTrace (env.get x i) t).head? with
      | some o => o
      | none => Node.prevState pacing (.latest iσ x) () env t i)
      = lastUpdate (env.get x i t) := fun i => by rw [slot_lastUpdate _ (hpx i t).1, hs]
  show ((fun i => match (incTrace (env.get x i) t).head? with
        | some o => o
        | none => Node.prevState pacing (.latest iσ x) () env t i),
      ((fun i => match (incTrace (env.get x i) t).head? with
        | some o => o
        | none => Node.prevState pacing (.latest iσ x) () env t i), ()))
    = ((fun i => lastUpdate (env.get x i t)),
      ((fun i => lastUpdateHist (env.get x) i t), ()))
  simp only [hm]
  congr 2
  funext i
  show lastUpdate (env.get x i t) = lastUpdate ((famHist (fun s j => env.get x j s) i).view t)
  show lastUpdate (env.get x i t) = lastUpdate (famFreeze (fun s j => env.get x j s) t i)
  rw [hraw]

theorem spec_latest_monotone {Γ : Ctx L mem} {ℓ : L} (x : Var Γ (.atomic ℓ (Option Nat)))
    (env : Env Γ) (t : Nat) (hx : TMono (env.get x)) (hpx : TPaced (pacing.active ℓ) (env.get x)) :
    NodeSpec pacing (.latest_monotone x) () env t := by
  have hraw : ∀ k, famFreeze (fun s j => env.get x j s) k = fun j => env.get x j k :=
    famFreeze_eq_of_mono _ (fun k j => hx j k (k + 1) (Nat.le_succ k))
  have hs : ∀ i, (incTrace (env.get x i) t).foldl (fun _acc o => o)
      (Node.prevState pacing (.latest_monotone x) () env t i) = lastUpdate (env.get x i t) := by
    intro i
    cases t with
    | zero =>
      show (incTrace (env.get x i) 0).foldl (fun _acc o => o) none = lastUpdate (env.get x i 0)
      have h0 : (none : Option Nat) = lastUpdate [] := rfl
      rw [h0, ← lastUpdate_append]
      rfl
    | succ t =>
      show (incTrace (env.get x i) (t + 1)).foldl (fun _acc o => o) (lastUpdate (env.get x i t)) = _
      rw [← lastUpdate_append, ← trace_succ_inc (env.get x i) (fun k => hx i k (k + 1) (Nat.le_succ k)) t]
  simp only [NodeSpec, Node.step, Node.denote, Kind.incAt, envIncAt_get]
  have hm : ∀ i, (match (incTrace (env.get x i) t).head? with
      | some o => o
      | none => Node.prevState pacing (.latest_monotone x) () env t i)
      = lastUpdate (env.get x i t) := fun i => by
    have h1 := (hpx i t).1
    rw [← hs i]
    rcases hl : incTrace (env.get x i) t with _ | ⟨o, rest⟩
    · rfl
    · rw [hl] at h1
      cases rest with
      | nil => rfl
      | cons _ _ => simp at h1
  show ((fun i => match (incTrace (env.get x i) t).head? with
        | some o => o
        | none => Node.prevState pacing (.latest_monotone x) () env t i),
      ((fun i => match (incTrace (env.get x i) t).head? with
        | some o => o
        | none => Node.prevState pacing (.latest_monotone x) () env t i), ()))
    = ((fun i => lastUpdate (env.get x i t)),
      ((fun i => lastUpdateHist (env.get x) i t), ()))
  simp only [hm]
  congr 2
  funext i
  show lastUpdate (env.get x i t) = lastUpdate ((famHist (fun s j => env.get x j s) i).view t)
  show lastUpdate (env.get x i t) = lastUpdate (famFreeze (fun s j => env.get x j s) t i)
  rw [hraw]

/-! ## The two `sliced!` sources -/

theorem spec_time {Γ : Ctx L mem} {ℓ : L} (d : TickScript (mem ℓ) Nat)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.time (Γ := Γ) (ℓ := ℓ)) d env t := by
  simp only [NodeSpec, Node.step, Node.denote]
  have hp : Node.prevState pacing (.time (Γ := Γ) (ℓ := ℓ)) d env t
      = fun i => (prevTicks (pacing.ticks ℓ i) t).length := by
    cases t <;> rfl
  rw [hp]
  show ((fun i => if pacing.ticks ℓ i t then (prevTicks (pacing.ticks ℓ i) t).length + 1 else (prevTicks (pacing.ticks ℓ i) t).length),
      ((fun i => if pacing.ticks ℓ i t then some (d i (prevTicks (pacing.ticks ℓ i) t).length) else none : Fin (mem ℓ) → Option Nat), ()))
    = ((fun i => (tickSteps (pacing.ticks ℓ i) t).length),
      ((fun i => (incTrace (fun k => scriptTrace (d i) (tickSteps (pacing.ticks ℓ i) k).length) t).head? : Fin (mem ℓ) → Option Nat), ()))
  congr 2
  · funext i; exact (tickSteps_length_ite _ t).symm
  · funext i; rw [scriptTrace_ticks_inc]; split <;> rfl

theorem spec_snapshot_bounded {Γ : Ctx L mem} {ℓ : L} {σ : Type} (v : σ) (d : Unit)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.snapshot_bounded (Γ := Γ) (ℓ := ℓ) v d) () env t := by
  simp only [NodeSpec, Node.step, Node.denote]
  have hp : Node.prevState pacing (.snapshot_bounded (Γ := Γ) (ℓ := ℓ) v d) () env t
      = fun i => (prevTicks (pacing.ticks ℓ i) t).length := by
    cases t <;> rfl
  rw [hp]
  show ((fun i => if pacing.ticks ℓ i t then (prevTicks (pacing.ticks ℓ i) t).length + 1 else (prevTicks (pacing.ticks ℓ i) t).length),
      ((fun i => if pacing.ticks ℓ i t then some v else none : Fin (mem ℓ) → Option σ), ()))
    = ((fun i => (tickSteps (pacing.ticks ℓ i) t).length),
      ((fun i => (incTrace (fun k => List.replicate (tickSteps (pacing.ticks ℓ i) k).length v) t).head? : Fin (mem ℓ) → Option σ), ()))
  have hlen : ∀ i, (tickSteps (pacing.ticks ℓ i) t).length
      = if pacing.ticks ℓ i t then (prevTicks (pacing.ticks ℓ i) t).length + 1 else (prevTicks (pacing.ticks ℓ i) t).length := by
    intro i
    rw [tickSteps_eq_prev, List.length_append]
    split <;> simp
  have hinc : ∀ i, incTrace (fun k => List.replicate (tickSteps (pacing.ticks ℓ i) k).length v) t
      = if pacing.ticks ℓ i t then [v] else [] := by
    intro i
    cases t with
    | zero =>
      show List.replicate (tickSteps (pacing.ticks ℓ i) 0).length v = _
      rw [tickSteps_zero]
      split <;> rfl
    | succ t =>
      show (List.replicate (tickSteps (pacing.ticks ℓ i) (t + 1)).length v).drop
        (List.replicate (tickSteps (pacing.ticks ℓ i) t).length v).length = _
      rw [tickSteps_succ, List.length_append, List.length_replicate, List.replicate_add,
        List.drop_left' (List.length_replicate ..)]
      split <;> rfl
  congr 2
  · funext i; exact (hlen i).symm
  · funext i; rw [hinc i]; split <;> rfl

end Project

end Hydro
