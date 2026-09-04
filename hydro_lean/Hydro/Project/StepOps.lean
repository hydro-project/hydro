import Hydro.Project.StepSpec

/-!
# Hydro · Project — per-operator step lemmas (stateless and simple cells)

Each lemma: stepping the node from its prescribed previous cells on the
denotation's increments at `t` yields the prescribed cells at `t` and
the denotation's output increments at `t`. The context is the lock-step
one: `ctx = ⟨t, pacing · · t⟩`.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} (pacing : Pacing L mem)

/-- The lock-step context at `t`. -/
def ctxAt (t : Nat) : StepCtx L mem := ⟨t, fun ℓ i => pacing.ticks ℓ i t, fun ℓ i => pacing.active ℓ i t⟩

/-- The shape of every per-node lemma. -/
def NodeSpec {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs) (sch : Node.Sched n) (env : Env Γ) (t : Nat) : Prop :=
  Node.step (ctxAt pacing t) n sch (envIncAt env t) (Node.prevState pacing n sch env t)
    = (Node.stateOf pacing n sch env t, envIncAt (Node.denote pacing n sch env) t)

local notation "S" => SchedSem L mem pacing

/-! ## Stateless stream operators -/

theorem spec_map {Γ : Ctx L mem} {ℓ : L} {α β : Type} (iα : DecidableEq α) (iβ : DecidableEq β)
    {ord : StrOrd} (x : Var Γ (.stream ℓ α iα ord .exactlyOnce)) (f : Fin (mem ℓ) → α → β)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.map iα iβ x f) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incView (env.get x i) t).map (f i) : Fin (mem ℓ) → List β), ()))
    = ((), ((fun i => incView ((env.get x i).map (f i)) t : Fin (mem ℓ) → List β), ()))
  congr 2
  funext i
  rw [incView_map]

theorem spec_filterMap {Γ : Ctx L mem} {ℓ : L} {α β : Type} (iα : DecidableEq α) (iβ : DecidableEq β)
    {ord : StrOrd} (x : Var Γ (.stream ℓ α iα ord .exactlyOnce)) (f : Fin (mem ℓ) → α → Option β)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.filterMap iα iβ x f) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incView (env.get x i) t).filterMap (f i) : Fin (mem ℓ) → List β), ()))
    = ((), ((fun i => incView ((env.get x i).filterMap (f i)) t : Fin (mem ℓ) → List β), ()))
  congr 2
  funext i
  rw [incView_filterMap]

theorem spec_values {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V)
    {ord : StrOrd} {ret : Retries} (x : Var Γ (.keyed ℓ K V iK iV ord ret))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.values iK iV x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incView (env.get x i) t).map Prod.snd : Fin (mem ℓ) → List V), ()))
    = ((), ((fun i => incView ((env.get x i).map Prod.snd) t : Fin (mem ℓ) → List V), ()))
  congr 2
  funext i
  rw [incView_map]

theorem spec_into_keyed {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V)
    {ord : StrOrd} (x : Var Γ (.stream ℓ (K × V) inferInstance ord .exactlyOnce))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.into_keyed iK iV x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rfl

theorem spec_kassume_ordering {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K)
    (iV : DecidableEq V) (x : Var Γ (.keyed ℓ K V iK iV .noOrder .exactlyOnce)) (d : Unit)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.kassume_ordering iK iV x d) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rfl

theorem spec_weaken_retries {Γ : Ctx L mem} {ℓ : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
    (x : Var Γ (.stream ℓ α iα ord .exactlyOnce))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.weaken_retries iα x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rfl

theorem spec_assume_ordering {Γ : Ctx L mem} {ℓ : L} {α : Type} (iα : DecidableEq α)
    (x : Var Γ (.stream ℓ α iα .noOrder .exactlyOnce)) (d : Unit)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.assume_ordering iα x d) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rfl

theorem spec_union {Γ : Ctx L mem} {ℓ : L} {α : Type} (iα : DecidableEq α) {ret : Retries}
    (a b : Var Γ (.stream ℓ α iα .noOrder ret))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.union iα a b) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => incView (env.get a i) t ++ incView (env.get b i) t : Fin (mem ℓ) → List α), ()))
    = ((), ((fun i => incView (merge2 (env.get a i) (env.get b i)) t : Fin (mem ℓ) → List α), ()))
  congr 2
  funext i
  rw [incView_merge2]

theorem spec_kget_max_key {Γ : Ctx L mem} {ℓ : L} {α K V : Type} (iα : DecidableEq α) (iK : DecidableEq K)
    (iV : DecidableEq V) (lo : LinearOrder K) {ord : StrOrd} {ret : Retries}
    (x : Var Γ (.ksing ℓ α K V iα iK iV ord ret .boundedValue))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.kget_max_key iα iK iV lo x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rfl

theorem spec_tinto_keyed {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V)
    {ord : StrOrd} (x : Var Γ (.atomicStream ℓ (K × V) inferInstance ord .exactlyOnce))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.tinto_keyed iK iV x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rfl

theorem spec_tassume_ordering {Γ : Ctx L mem} {ℓ : L} {α : Type} (iα : DecidableEq α)
    (x : Var Γ (.atomicStream ℓ α iα .noOrder .exactlyOnce)) (d : Unit)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.tassume_ordering iα x d) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rfl

theorem spec_flattenOrdered {Γ : Ctx L mem} {ℓ : L} {β : Type} (iβ : DecidableEq β)
    (x : Var Γ (.atomic ℓ (List β)))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.flattenOrdered iβ x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rfl

theorem spec_flattenUnordered {Γ : Ctx L mem} {ℓ : L} {β : Type} (iβ : DecidableEq β)
    (x : Var Γ (.atomic ℓ (List β)))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.flattenUnordered iβ x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  rfl

theorem spec_mapTick {Γ : Ctx L mem} {ℓ : L} {α β : Type} (x : Var Γ (.atomic ℓ α))
    (f : Fin (mem ℓ) → α → β) (env : Env Γ) (t : Nat) : NodeSpec pacing (.mapTick x f) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incTrace (env.get x i) t).map (f i) : Fin (mem ℓ) → List β), ()))
    = ((), ((fun i => incTrace (fun k => (env.get x i k).map (f i)) t : Fin (mem ℓ) → List β), ()))
  congr 2
  funext i
  rw [incTrace_map]

theorem spec_snapshot {Γ : Ctx L mem} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd}
    {ret : Retries} {b : SingBound σ} (x : Var Γ (.sing ℓ α σ iα ord ret b)) (d : Unit)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.snapshot iα x d) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => if pacing.ticks ℓ i t then [env.get x i t] else [] : Fin (mem ℓ) → List σ), ()))
    = ((), ((fun i => incTrace (fun k => (tickSteps (pacing.ticks ℓ i) k).map (fun st => env.get x i st)) t :
        Fin (mem ℓ) → List σ), ()))
  congr 2
  funext i
  cases t with
  | zero =>
    show _ = (tickSteps (pacing.ticks ℓ i) 0).map (fun st => env.get x i st)
    rw [tickSteps_zero]
    split <;> rfl
  | succ t =>
    show _ = ((tickSteps (pacing.ticks ℓ i) (t + 1)).map (fun st => env.get x i st)).drop
      ((tickSteps (pacing.ticks ℓ i) t).map (fun st => env.get x i st)).length
    rw [tickSteps_succ, List.map_append, List.drop_left]
    split <;> rfl

/-! ## Folds -/

theorem spec_fold {Γ : Ctx L mem} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd} {ret : Retries}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) (iPre : DecidableRel Pre) (ok : FoldOk ord ret Pre g)
    (x : Var Γ (.stream ℓ α iα ord ret)) (env : Env Γ) (t : Nat) :
    NodeSpec pacing (.fold iα g init Pre iPre ok x) () env t := by
  have hs : ∀ i, ((envIncAt env t).get x i).foldl g (Node.prevState pacing (.fold iα g init Pre iPre ok x) () env t i)
      = ((env.get x i).view t).foldl g init := by
    intro i
    rw [envIncAt_get]
    cases t with
    | zero => rfl
    | succ t =>
      show (incView (env.get x i) (t + 1)).foldl g (((env.get x i).view t).foldl g init) = _
      rw [← List.foldl_append, ← view_succ_inc]
  simp only [NodeSpec, Node.step]
  show ((fun i => ((envIncAt env t).get x i).foldl g _), ((fun i => ((envIncAt env t).get x i).foldl g _), ())) = _
  simp only [hs]
  rfl

theorem spec_fold_monotone {Γ : Ctx L mem} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd}
    {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ) (init : σ)
    (ok : FoldOk ord ret (fun _ _ => True) g) (hinfl : ∀ s x, vo.le s (g s x))
    (x : Var Γ (.stream ℓ α iα ord ret)) (env : Env Γ) (t : Nat) :
    NodeSpec pacing (.fold_monotone iα vo g init ok hinfl x) () env t := by
  have hs : ∀ i, ((envIncAt env t).get x i).foldl g
      (Node.prevState pacing (.fold_monotone iα vo g init ok hinfl x) () env t i)
      = ((env.get x i).view t).foldl g init := by
    intro i
    rw [envIncAt_get]
    cases t with
    | zero => rfl
    | succ t =>
      show (incView (env.get x i) (t + 1)).foldl g (((env.get x i).view t).foldl g init) = _
      rw [← List.foldl_append, ← view_succ_inc]
  simp only [NodeSpec, Node.step]
  show ((fun i => ((envIncAt env t).get x i).foldl g _), ((fun i => ((envIncAt env t).get x i).foldl g _), ())) = _
  simp only [hs]
  rfl

/-! ## Tick counters and the deferred wire -/

theorem tickSteps_isEmpty_zero (p : Nat → Bool) : (tickSteps p 0).isEmpty = !p 0 := by
  rw [tickSteps_zero]; cases p 0 <;> rfl

theorem tickSteps_isEmpty_succ (p : Nat → Bool) (t : Nat) :
    (tickSteps p (t + 1)).isEmpty = ((tickSteps p t).isEmpty && !p (t + 1)) := by
  rw [tickSteps_succ]
  cases p (t + 1) <;> simp

/-- A step-monotone trace's view is its previous view plus its increment. -/
theorem trace_prev_append_inc {σ : Type} {x : Nat → Trace σ} (hx : ∀ k k', k ≤ k' → x k <+: x k') (t : Nat) :
    x t ++ incTrace x (t + 1) = x (t + 1) := by
  obtain ⟨e, he⟩ := hx t (t + 1) (Nat.le_succ t)
  show x t ++ (x (t + 1)).drop (x t).length = x (t + 1)
  rw [← he, List.drop_left]

theorem spec_defer_tick {Γ : Ctx L mem} {ℓ : L} {σ : Type} (init : σ) (x : Var Γ (.atomic ℓ σ))
    (env : Env Γ) (hx : TMono (env.get x)) (t : Nat) : NodeSpec pacing (.defer_tick init x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, Node.stateOf, envIncAt_get, Kind.incAt, ctxAt]
  have hD : ∀ i s, (SchedSem L mem pacing).defer_tick init (env.get x) i s
      = if (tickSteps (pacing.ticks ℓ i) s).isEmpty then [] else init :: env.get x i s := fun _ _ => rfl
  cases t with
  | zero =>
    show ((fun i => if false then _ else if pacing.ticks ℓ i 0 then (true, []) else (false, [] ++ incTrace (env.get x i) 0)),
        ((fun i => if false then _ else if pacing.ticks ℓ i 0 then init :: ([] ++ incTrace (env.get x i) 0) else [] :
          Fin (mem ℓ) → List σ), ())) = _
    simp only [if_false, List.nil_append]
    congr 1
    · funext i
      rw [tickSteps_isEmpty_zero]
      cases pacing.ticks ℓ i 0 <;> rfl
    · congr 1
      funext i
      show _ = incTrace ((SchedSem L mem pacing).defer_tick init (env.get x) i) 0
      show _ = (SchedSem L mem pacing).defer_tick init (env.get x) i 0
      rw [hD, tickSteps_isEmpty_zero]
      cases pacing.ticks ℓ i 0 <;> rfl
  | succ t =>
    show ((fun i => if !(tickSteps (pacing.ticks ℓ i) t).isEmpty then
            (!(tickSteps (pacing.ticks ℓ i) t).isEmpty, if (tickSteps (pacing.ticks ℓ i) t).isEmpty then env.get x i t else [])
          else if pacing.ticks ℓ i (t + 1) then (true, [])
          else (false, (if (tickSteps (pacing.ticks ℓ i) t).isEmpty then env.get x i t else []) ++ incTrace (env.get x i) (t + 1))),
        ((fun i => if !(tickSteps (pacing.ticks ℓ i) t).isEmpty then incTrace (env.get x i) (t + 1)
          else if pacing.ticks ℓ i (t + 1) then
            init :: ((if (tickSteps (pacing.ticks ℓ i) t).isEmpty then env.get x i t else []) ++ incTrace (env.get x i) (t + 1))
          else [] : Fin (mem ℓ) → List σ), ())) = _
    congr 1
    · funext i
      rw [tickSteps_isEmpty_succ]
      cases hE : (tickSteps (pacing.ticks ℓ i) t).isEmpty
      · rfl
      · cases pacing.ticks ℓ i (t + 1)
        · simp only [Bool.not_true, Bool.false_eq_true, if_false, Bool.true_and, Bool.not_false, if_true]
          rw [trace_prev_append_inc (hx i) t]
        · simp
    · congr 1
      funext i
      show _ = incTrace ((SchedSem L mem pacing).defer_tick init (env.get x) i) (t + 1)
      show _ = ((SchedSem L mem pacing).defer_tick init (env.get x) i (t + 1)).drop
        ((SchedSem L mem pacing).defer_tick init (env.get x) i t).length
      rw [hD, hD, tickSteps_isEmpty_succ]
      cases hE : (tickSteps (pacing.ticks ℓ i) t).isEmpty
      · rfl
      · cases pacing.ticks ℓ i (t + 1)
        · simp
        · simp only [Bool.not_true, Bool.false_eq_true, if_false, Bool.true_and, Bool.not_true, Bool.and_false,
            if_true, List.length_nil, List.drop_zero]
          rw [trace_prev_append_inc (hx i) t]

theorem spec_timeout_snapshot {Γ : Ctx L mem} {ℓ : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
    {ret : Retries} (x : Var Γ (.stream ℓ α iα ord ret)) (d : TimerVerdicts (mem ℓ))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.timeout_snapshot (ℓ := ℓ) iα x) d env t := by
  simp only [NodeSpec, Node.step, Node.denote]
  have hp : Node.prevState pacing (.timeout_snapshot (ℓ := ℓ) iα x) d env t = fun i => (prevTicks (pacing.ticks ℓ i) t).length := by
    cases t <;> rfl
  rw [hp]
  show ((fun i => if pacing.ticks ℓ i t then (prevTicks (pacing.ticks ℓ i) t).length + 1 else (prevTicks (pacing.ticks ℓ i) t).length),
      ((fun i => if pacing.ticks ℓ i t then ((d i)[(prevTicks (pacing.ticks ℓ i) t).length]?).toList else [] : Fin (mem ℓ) → List Bool), ()))
    = ((fun i => (tickSteps (pacing.ticks ℓ i) t).length),
      ((fun i => incTrace (fun k => (d i).take (tickSteps (pacing.ticks ℓ i) k).length) t : Fin (mem ℓ) → List Bool), ()))
  have hlen : ∀ i, (tickSteps (pacing.ticks ℓ i) t).length
      = if pacing.ticks ℓ i t then (prevTicks (pacing.ticks ℓ i) t).length + 1 else (prevTicks (pacing.ticks ℓ i) t).length := by
    intro i
    rw [tickSteps_eq_prev, List.length_append]
    split <;> simp
  have hinc : ∀ i, incTrace (fun k => (d i).take (tickSteps (pacing.ticks ℓ i) k).length) t
      = if pacing.ticks ℓ i t then ((d i)[(prevTicks (pacing.ticks ℓ i) t).length]?).toList else [] := by
    intro i
    cases t with
    | zero =>
      show (d i).take (tickSteps (pacing.ticks ℓ i) 0).length = _
      rw [tickSteps_zero]
      split
      · simp only [List.length_singleton, prevTicks, List.length_nil]
        cases d i <;> rfl
      · rfl
    | succ t =>
      show ((d i).take (tickSteps (pacing.ticks ℓ i) (t + 1)).length).drop ((d i).take (tickSteps (pacing.ticks ℓ i) t).length).length = _
      rw [tickSteps_succ, List.length_append]
      show _ = if pacing.ticks ℓ i (t + 1) then ((d i)[(tickSteps (pacing.ticks ℓ i) t).length]?).toList else []
      split
      · simp only [List.length_singleton]
        exact take_succ_drop_take (d i) _
      · simp only [List.length_nil, Nat.add_zero, List.drop_length]
  congr 2
  · funext i; exact (hlen i).symm
  · funext i; exact (hinc i).symm

theorem spec_source_interval_batch {Γ : Ctx L mem} {ℓ : L} (d : TimingPulses (mem ℓ))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.source_interval_batch (Γ := Γ) (ℓ := ℓ)) d env t := by
  simp only [NodeSpec, Node.step, Node.denote]
  have hp : Node.prevState pacing (.source_interval_batch (Γ := Γ) (ℓ := ℓ)) d env t
      = fun i => (prevTicks (pacing.ticks ℓ i) t).length := by
    cases t <;> rfl
  rw [hp]
  show ((fun i => if pacing.ticks ℓ i t then (prevTicks (pacing.ticks ℓ i) t).length + 1 else (prevTicks (pacing.ticks ℓ i) t).length),
      ((fun i => if pacing.ticks ℓ i t then ((d i)[(prevTicks (pacing.ticks ℓ i) t).length]?).toList else [] : Fin (mem ℓ) → List Bool), ()))
    = ((fun i => (tickSteps (pacing.ticks ℓ i) t).length),
      ((fun i => incTrace (fun k => (d i).take (tickSteps (pacing.ticks ℓ i) k).length) t : Fin (mem ℓ) → List Bool), ()))
  have hlen : ∀ i, (tickSteps (pacing.ticks ℓ i) t).length
      = if pacing.ticks ℓ i t then (prevTicks (pacing.ticks ℓ i) t).length + 1 else (prevTicks (pacing.ticks ℓ i) t).length := by
    intro i
    rw [tickSteps_eq_prev, List.length_append]
    split <;> simp
  have hinc : ∀ i, incTrace (fun k => (d i).take (tickSteps (pacing.ticks ℓ i) k).length) t
      = if pacing.ticks ℓ i t then ((d i)[(prevTicks (pacing.ticks ℓ i) t).length]?).toList else [] := by
    intro i
    cases t with
    | zero =>
      show (d i).take (tickSteps (pacing.ticks ℓ i) 0).length = _
      rw [tickSteps_zero]
      split
      · simp only [List.length_singleton, prevTicks, List.length_nil]
        cases d i <;> rfl
      · rfl
    | succ t =>
      show ((d i).take (tickSteps (pacing.ticks ℓ i) (t + 1)).length).drop ((d i).take (tickSteps (pacing.ticks ℓ i) t).length).length = _
      rw [tickSteps_succ, List.length_append]
      show _ = if pacing.ticks ℓ i (t + 1) then ((d i)[(tickSteps (pacing.ticks ℓ i) t).length]?).toList else []
      split
      · simp only [List.length_singleton]
        exact take_succ_drop_take (d i) _
      · simp only [List.length_nil, Nat.add_zero, List.drop_length]
  congr 2
  · funext i; exact (hlen i).symm
  · funext i; exact (hinc i).symm

/-! ## `allTicks`: the frozen family is the raw one under tick-monotonicity -/

theorem spec_allTicks {Γ : Ctx L mem} {ℓ : L} {β : Type} (iβ : DecidableEq β) {ord : StrOrd}
    (x : Var Γ (.atomicStream ℓ β iβ ord .exactlyOnce)) (env : Env Γ) (t : Nat)
    (hx : TMono (env.get x)) : NodeSpec pacing (.allTicks iβ x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incTrace (env.get x i) t).flatten : Fin (mem ℓ) → List β), ()))
    = ((), ((fun i => incView (famHist (fun step j => (env.get x j step).flatten) i) t : Fin (mem ℓ) → List β), ()))
  congr 2
  funext i
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

end Project

end Hydro
