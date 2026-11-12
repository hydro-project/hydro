import Hydro.Project.Model.StepOps5

/-!
# Hydro · Project — per-operator step lemmas (keyed wires, `tfold`, `latest`, tick sources)

`NodeSpec` for the keyed wires (`kentries`/`kmap`/`kallTicks`: stateless casts and maps;
`kfold`: the arrivals so far with the hashed keyed fold recomputed; `ksnapshot`: a tick read
of the keyed singleton's current map), the across-ticks fold (`tfold`: a scan over the new
batches), the two latest-value cells (`latest`, `latest_monotone`: the last present value,
replaced by each new tick) and the two tick sources (`time`: the script's reading at each tick;
`snapshot_bounded`: the constant at each tick). The tick-trace nodes need `TMono` and `TPaced`
of their input, which make the step's increment one slot.
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
  show ((), ((fun i => (incView (env.get x i) t).toArray.map (fun e => (e.1, f i e.2)) : Fin (mem ℓ) → Array (K × W)), ()))
    = ((), ((fun i => (incView ((env.get x i).map (fun e => (e.1, f i e.2))) t).toArray : Fin (mem ℓ) → Array (K × W)), ()))
  congr 2
  funext i
  rw [incView_map, List.map_toArray]

/-- `kallTicks` meets `NodeSpec`; the proof is `spec_allTicks` at the keyed wire. -/
theorem spec_kallTicks {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V)
    {ord : StrOrd} (x : Var Γ (.atomicKeyed ℓ K V iK iV ord .exactlyOnce)) (env : Env Γ) (t : Nat)
    (hx : TMono (env.get x)) (hpx : TPaced (pacing.active ℓ) (env.get x)) :
    NodeSpec pacing (.kallTicks iK iV x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incTrace (env.get x i) t).head?.getD #[] : Fin (mem ℓ) → Array (K × V)), ()))
    = ((), ((fun i => (incView (famHist (fun step j => ((env.get x j step).map Array.toList).flatten) i) t).toArray
        : Fin (mem ℓ) → Array (K × V)), ()))
  congr 2
  funext i
  apply Array.toList_inj.mp
  rw [List.toList_toArray, toList_getD_empty]
  rw [← List.head?_map, ← incTrace_map, head?_getD_eq_flatten _ (by
    rw [incTrace_map, List.length_map]; exact (hpx i t).1)]
  have hraw : ∀ k, famFreeze (fun step j => ((env.get x j step).map Array.toList).flatten) k
      = fun j => ((env.get x j k).map Array.toList).flatten :=
    famFreeze_eq_of_mono _ (fun k j => prefix_flatten (List.IsPrefix.map _ (hx j k (k + 1) (Nat.le_succ k))))
  cases t with
  | zero =>
    show ((env.get x i 0).map Array.toList).flatten
      = famFreeze (fun step j => ((env.get x j step).map Array.toList).flatten) 0 i
    rw [hraw]
  | succ t =>
    show (((env.get x i (t + 1)).map Array.toList).drop ((env.get x i t).map Array.toList).length).flatten
      = (famFreeze (fun step j => ((env.get x j step).map Array.toList).flatten) (t + 1) i).drop
        (famFreeze (fun step j => ((env.get x j step).map Array.toList).flatten) t i).length
    rw [hraw, hraw]
    show _ = ((env.get x i (t + 1)).map Array.toList).flatten.drop ((env.get x i t).map Array.toList).flatten.length
    rw [trace_succ_inc (env.get x i) (fun k => hx i k (k + 1) (Nat.le_succ k)) t, List.map_append,
      List.flatten_append]
    simp only [List.drop_left]

/-- `kfold` meets `NodeSpec`: the cell is `keyedFoldAssocH` of the view, and
`kfoldStepH_keyedFoldAssocH` folds this step's arrivals into it. -/
theorem spec_kfold {Γ : Ctx L mem} {ℓ : L} {K V A : Type} (iK : DecidableEq K) (iH : Hashable K)
    (iV : DecidableEq V) (iA : DecidableEq A) {ord : StrOrd} (g : A → V → A) (init : A)
    (Pre : V → V → Prop) (iPre : DecidableRel Pre) (ok : FoldOk ord .exactlyOnce Pre g)
    (x : Var Γ (.keyed ℓ K V iK iV ord .exactlyOnce)) (env : Env Γ) (t : Nat) :
    NodeSpec pacing (.kfold iK iH iV iA g init Pre iPre ok x) () env t := by
  have hs : ∀ i, kfoldStepH g init (Node.prevState pacing (.kfold iK iH iV iA g init Pre iPre ok x) () env t i)
      ((envIncAt env t).get x i) = keyedFoldAssocH g init ((env.get x i).view t) := by
    intro i
    rw [envIncAt_get]
    cases t with
    | zero =>
      show kfoldStepH (K := K) g init (keyedFoldAssocH g init []) _ = _
      rw [kfoldStepH_keyedFoldAssocH, List.nil_append, List.toList_toArray]
      rfl
    | succ t =>
      show kfoldStepH (K := K) g init (keyedFoldAssocH g init ((env.get x i).view t))
        (incView (env.get x i) (t + 1)).toArray = _
      rw [kfoldStepH_keyedFoldAssocH, List.toList_toArray, ← view_succ_inc]
  simp only [NodeSpec, Node.step]
  show ((fun i => kfoldStepH g init (Node.prevState pacing (.kfold iK iH iV iA g init Pre iPre ok x) () env t i)
        ((envIncAt env t).get x i)),
      ((fun i => kfoldStepH g init (Node.prevState pacing (.kfold iK iH iV iA g init Pre iPre ok x) () env t i)
        ((envIncAt env t).get x i)), ())) = _
  simp only [hs]
  rfl

/-- `ksnapshot` meets `NodeSpec`: the tick read is `incTrace_ticks_map`. -/
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

/-- `tfold` meets `NodeSpec`: the register is `scanRun` over the input trace, so `TMono`
(`scanRun_prefix_inc`) advances it by the new batches and `TPaced` makes the emitted value the
single new scan output. -/
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

/-! ## The latest-value cells -/

/-- Replacing the cell by a single slot's value, when there is one, is folding the slot in. -/
theorem slot_lastUpdate {σ : Type} (l : List (Option σ)) (h : l.length ≤ 1) (s : Option σ) :
    (match l.head? with
      | some o => o
      | none => s) = l.foldl (fun _acc o => o) s := by
  match l, h with
  | [], _ => rfl
  | [o], _ => rfl
  | _ :: _ :: _, h => simp at h

/-- `lastUpdate` of an appended trace folds the suffix's values over `lastUpdate` of the prefix. -/
theorem lastUpdate_append {σ : Type} (l l' : Trace (Option σ)) :
    lastUpdate (l ++ l') = l'.foldl (fun _acc o => o) (lastUpdate l) := by
  induction l' generalizing l with
  | nil => simp
  | cons o l' ih =>
    rw [show l ++ o :: l' = (l ++ [o]) ++ l' from by simp, ih (l ++ [o]), List.foldl_cons]
    congr 1
    unfold lastUpdate
    simp [List.getLast?_append]

/-- `latest` meets `NodeSpec`: the cell is `lastUpdate` of the trace, `TMono` makes
`lastUpdate_append` extend it by the increment and `famFreeze` transparent, and `TPaced` makes
the increment one slot (`slot_lastUpdate`). -/
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

/-- `spec_latest` for the `Monotonic` latest over `Option Nat`. -/
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

/-! ## The two tick sources -/

/-- `time` meets `NodeSpec`: the cell is the tick count (`tickSteps_length_ite`) and the output
is the script's entry at the previous tick count (`scriptTrace_ticks_inc`). -/
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

/-- `snapshot_bounded` meets `NodeSpec`: the cell is the tick count and the output is `v` at each
tick. -/
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
