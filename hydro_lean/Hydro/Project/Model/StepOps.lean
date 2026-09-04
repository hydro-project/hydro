import Hydro.Project.Model.StepSpec

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

/-- The increment of a tick-indexed trace `(tickSteps p k).map g` at step
`t`: the tick's entry, if `t` is a tick. -/
theorem incTrace_ticks_map {σ : Type} (p : Nat → Bool) (g : Nat → σ) (t : Nat) :
    incTrace (fun k => (tickSteps p k).map g) t = if p t then [g t] else [] := by
  cases t with
  | zero =>
    show (tickSteps p 0).map g = _
    rw [tickSteps_zero]; split <;> rfl
  | succ t =>
    show ((tickSteps p (t + 1)).map g).drop ((tickSteps p t).map g).length = _
    rw [tickSteps_succ, List.map_append, List.drop_left]
    split <;> rfl

theorem spec_mapTick {Γ : Ctx L mem} {ℓ : L} {α β : Type} (x : Var Γ (.atomic ℓ α))
    (f : Fin (mem ℓ) → α → β) (env : Env Γ) (t : Nat) : NodeSpec pacing (.mapTick x f) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incTrace (env.get x i) t).head?.map (f i) : Fin (mem ℓ) → Option β), ()))
    = ((), ((fun i => (incTrace (fun k => (env.get x i k).map (f i)) t).head? : Fin (mem ℓ) → Option β), ()))
  congr 2
  funext i
  rw [incTrace_map, List.head?_map]

theorem spec_firstTick {Γ : Ctx L mem} {ℓ : L} {α : Type} (iα : DecidableEq α) (ret : Retries)
    (x : Var Γ (.atomicStream ℓ α iα .totalOrder ret)) (env : Env Γ) (t : Nat) :
    NodeSpec pacing (.firstTick iα ret x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incTrace (env.get x i) t).head?.map List.head? : Fin (mem ℓ) → Option (Option α)), ()))
    = ((), ((fun i => (incTrace (fun k => (env.get x i k).map List.head?) t).head? : Fin (mem ℓ) → Option (Option α)), ()))
  congr 2
  funext i
  rw [incTrace_map, List.head?_map]

theorem spec_snapshot {Γ : Ctx L mem} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd}
    {ret : Retries} {b : SingBound σ} (x : Var Γ (.sing ℓ α σ iα ord ret b)) (d : Unit)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.snapshot iα x d) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => if pacing.ticks ℓ i t then some (env.get x i t) else none : Fin (mem ℓ) → Option σ), ()))
    = ((), ((fun i => (incTrace (fun k => (tickSteps (pacing.ticks ℓ i) k).map (fun st => env.get x i st)) t).head? :
        Fin (mem ℓ) → Option σ), ()))
  congr 2
  funext i
  rw [incTrace_ticks_map]
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

/-- One FIFO step of `defer_tick`, as pure list arithmetic: from the buffer
`X'.drop r` (the entries of `init :: input` beyond the released count),
a tick releases the head — exactly the increment of `take` by the
released count. -/
theorem defer_fifo_step {σ : Type} (X X' : List σ) (r : Nat) (hX : X <+: X') (hr : r ≤ X.length) (tick : Bool) :
    (if tick then (X'.drop r).head? else none)
        = ((X'.take (if tick && decide (r < X'.length) then r + 1 else r)).drop (X.take r).length).head?
      ∧ (if tick then (X'.drop r).tail else X'.drop r)
        = X'.drop (if tick && decide (r < X'.length) then r + 1 else r) := by
  have hlen : (X.take r).length = r := by rw [List.length_take]; omega
  rw [hlen]
  have hXX' : X.length ≤ X'.length := hX.length_le
  cases tick with
  | false =>
    refine ⟨?_, rfl⟩
    show none = ((X'.take r).drop r).head?
    rw [List.drop_eq_nil_of_le (by rw [List.length_take]; omega)]; rfl
  | true =>
    by_cases hlt : r < X'.length
    · have hc : (true && decide (r < X'.length)) = true := by simp [hlt]
      simp only [hc, if_true]
      refine ⟨?_, List.tail_drop⟩
      rw [List.drop_take, Nat.add_sub_cancel_left]
      cases X'.drop r <;> rfl
    · have hc : (true && decide (r < X'.length)) = false := by simp [hlt]
      simp only [hc, Bool.false_eq_true, if_false, if_true]
      have hr' : X'.length ≤ r := Nat.le_of_not_lt hlt
      have hd : X'.drop r = [] := List.drop_eq_nil_of_le hr'
      rw [hd]
      refine ⟨?_, rfl⟩
      rw [List.take_of_length_le hr', List.drop_eq_nil_of_le (by omega)]

theorem spec_defer_tick {Γ : Ctx L mem} {ℓ : L} {σ : Type} (init : σ) (x : Var Γ (.atomic ℓ σ))
    (env : Env Γ) (hx : TMono (env.get x)) (hp : TPaced (pacing.active ℓ) (env.get x)) (t : Nat) :
    NodeSpec pacing (.defer_tick init x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, Node.stateOf, envIncAt_get, Kind.incAt, ctxAt]
  -- the released count never exceeds the buffer
  have hlen : ∀ i k, (env.get x i k).length ≤ (env.get x i (k + 1)).length :=
    fun i k => (hx i k (k + 1) (Nat.le_succ k)).length_le
  have hr : ∀ i k, deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) k ≤ (init :: env.get x i k).length :=
    fun i k => by rw [List.length_cons]; exact deferReleased_le_len_succ _ _ (hlen i) k
  -- a step's single slot is the whole increment
  have hslot : ∀ i k, (incTrace (env.get x i) k).head?.toList = incTrace (env.get x i) k :=
    fun i k => incTrace_eq_toList_head? hp i k
  have hD : ∀ i k, (SchedSem L mem pacing).defer_tick init (env.get x) i k
      = (init :: env.get x i k).take (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) k) :=
    fun _ _ => rfl
  cases t with
  | zero =>
    show ((fun i => if pacing.ticks ℓ i 0 then ([init] ++ (incTrace (env.get x i) 0).head?.toList).tail
            else [init] ++ (incTrace (env.get x i) 0).head?.toList),
        ((fun i => if pacing.ticks ℓ i 0 then ([init] ++ (incTrace (env.get x i) 0).head?.toList).head? else none :
          Fin (mem ℓ) → Option σ), ())) = _
    simp only [hslot]
    simp only [incTrace_zero, List.singleton_append]
    congr 1
    · funext i
      show _ = (init :: env.get x i 0).drop (if pacing.ticks ℓ i 0 then 1 else 0)
      split <;> rfl
    · congr 1
      funext i
      show _ = (incTrace ((SchedSem L mem pacing).defer_tick init (env.get x) i) 0).head?
      rw [incTrace_zero, hD]
      show _ = ((init :: env.get x i 0).take (if pacing.ticks ℓ i 0 then 1 else 0)).head?
      split <;> rfl
  | succ t =>
    show ((fun i => if pacing.ticks ℓ i (t + 1)
            then ((init :: env.get x i t).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) t)
              ++ (incTrace (env.get x i) (t + 1)).head?.toList).tail
            else (init :: env.get x i t).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) t)
              ++ (incTrace (env.get x i) (t + 1)).head?.toList),
        ((fun i => if pacing.ticks ℓ i (t + 1)
            then ((init :: env.get x i t).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) t)
              ++ (incTrace (env.get x i) (t + 1)).head?.toList).head? else none : Fin (mem ℓ) → Option σ), ())) = _
    simp only [hslot]
    -- the buffer is the next step's `init :: x` beyond the released count
    have hbuf : ∀ i, (init :: env.get x i t).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) t)
        ++ incTrace (env.get x i) (t + 1)
        = (init :: env.get x i (t + 1)).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) t) := by
      intro i
      rw [← List.drop_append_of_le_length (hr i t), List.cons_append, trace_prev_append_inc (hx i) t]
    simp only [hbuf]
    have hstep := fun i => defer_fifo_step (init :: env.get x i t) (init :: env.get x i (t + 1))
      (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) t)
      (List.cons_prefix_cons.mpr ⟨rfl, hx i t (t + 1) (Nat.le_succ t)⟩) (hr i t) (pacing.ticks ℓ i (t + 1))
    have hr' : ∀ i, deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) (t + 1)
        = if pacing.ticks ℓ i (t + 1) && decide (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) t
            < (init :: env.get x i (t + 1)).length)
          then deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) t + 1
          else deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i k).length) t := by
      intro i
      rw [deferReleased_succ, List.length_cons]
      simp only [Nat.lt_succ_iff]
    congr 1
    · funext i
      rw [hr' i]
      exact (hstep i).2
    · congr 1
      funext i
      show _ = (incTrace ((SchedSem L mem pacing).defer_tick init (env.get x) i) (t + 1)).head?
      rw [incTrace_succ, hD, hD, hr' i]
      exact (hstep i).1

/-! ## Timing scripts along the tick skeleton (D84)

A timer node's trace at step `t` is the script's first `ticks-by-t` entries; its
increment at a tick step is the entry at the previous tick count. -/

theorem scriptTrace_ticks_inc {β : Type} (sc : Nat → β) (p : Nat → Bool) (t : Nat) :
    incTrace (fun k => scriptTrace sc (tickSteps p k).length) t
      = if p t then [sc (prevTicks p t).length] else [] := by
  cases t with
  | zero =>
    show scriptTrace sc (tickSteps p 0).length = _
    rw [tickSteps_zero]
    split <;> rfl
  | succ t =>
    show (scriptTrace sc (tickSteps p (t + 1)).length).drop (scriptTrace sc (tickSteps p t).length).length = _
    rw [tickSteps_succ, List.length_append, scriptTrace_length]
    show _ = if p (t + 1) then [sc (tickSteps p t).length] else []
    split
    · rw [List.length_singleton, scriptTrace_succ, List.drop_left' (scriptTrace_length _ _)]
    · rw [List.length_nil, Nat.add_zero, List.drop_eq_nil_of_le (scriptTrace_length _ _).le]

theorem tickSteps_length_ite (p : Nat → Bool) (t : Nat) :
    (tickSteps p t).length = if p t then (prevTicks p t).length + 1 else (prevTicks p t).length := by
  rw [tickSteps_eq_prev, List.length_append]
  split <;> simp

/-- The interval stream's increment at step `t` (D91): the fire count of the tick, as
`()`s, when `t` is a tick; nothing otherwise. -/
theorem intervalHist_incView (p : Nat → Bool) (sc : Nat → Nat) (t : Nat) :
    incView (intervalHist p sc) t
      = if p t then List.replicate (sc (prevTicks p t).length) () else [] := by
  cases t with
  | zero =>
    show List.replicate (scriptSum sc (tickSteps p 0).length) () = _
    rw [tickSteps_zero]
    split
    · rw [List.length_singleton, scriptSum_succ, show scriptSum sc 0 = 0 from rfl, Nat.zero_add]; rfl
    · rfl
  | succ t =>
    show (List.replicate (scriptSum sc (tickSteps p (t + 1)).length) ()).drop
        (List.replicate (scriptSum sc (tickSteps p t).length) ()).length = _
    rw [List.length_replicate, List.drop_replicate, tickSteps_succ, List.length_append]
    show _ = if p (t + 1) then List.replicate (sc (tickSteps p t).length) () else []
    split
    · rw [List.length_singleton, scriptSum_succ, Nat.add_sub_cancel_left]
    · rw [List.length_nil, Nat.add_zero, Nat.sub_self]; rfl

theorem spec_source_interval {Γ : Ctx L mem} {ℓ : L} (delay : Fin (mem ℓ) → Nat) (period : Nat)
    (d : TickScript (mem ℓ) Nat)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.source_interval (Γ := Γ) (ℓ := ℓ) delay period) d env t := by
  simp only [NodeSpec, Node.step, Node.denote]
  have hp : Node.prevState pacing (.source_interval (Γ := Γ) (ℓ := ℓ) delay period) d env t
      = fun i => (prevTicks (pacing.ticks ℓ i) t).length := by
    cases t <;> rfl
  rw [hp]
  show ((fun i => if pacing.ticks ℓ i t then (prevTicks (pacing.ticks ℓ i) t).length + 1 else (prevTicks (pacing.ticks ℓ i) t).length),
      ((fun i => if pacing.ticks ℓ i t then List.replicate (d i (prevTicks (pacing.ticks ℓ i) t).length) () else [] : Fin (mem ℓ) → List Unit), ()))
    = ((fun i => (tickSteps (pacing.ticks ℓ i) t).length),
      ((fun i => incView (intervalHist (pacing.ticks ℓ i) (d i)) t : Fin (mem ℓ) → List Unit), ()))
  congr 2
  · funext i; exact (tickSteps_length_ite _ t).symm
  · funext i; rw [intervalHist_incView]
/-! ## `allTicks`: the frozen family is the raw one under tick-monotonicity -/

/-- A single slot's `getD []` is the flatten of the one-entry increment. -/
theorem head?_getD_eq_flatten {β : Type} (l : List (List β)) (h : l.length ≤ 1) :
    l.head?.getD [] = l.flatten := by
  match l, h with
  | [], _ => rfl
  | [b], _ => simp
  | _ :: _ :: _, h => simp at h

theorem spec_allTicks {Γ : Ctx L mem} {ℓ : L} {β : Type} (iβ : DecidableEq β) {ord : StrOrd}
    (x : Var Γ (.atomicStream ℓ β iβ ord .exactlyOnce)) (env : Env Γ) (t : Nat)
    (hx : TMono (env.get x)) (hp : TPaced (pacing.active ℓ) (env.get x)) :
    NodeSpec pacing (.allTicks iβ x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incTrace (env.get x i) t).head?.getD [] : Fin (mem ℓ) → List β), ()))
    = ((), ((fun i => incView (famHist (fun step j => (env.get x j step).flatten) i) t : Fin (mem ℓ) → List β), ()))
  congr 2
  funext i
  rw [head?_getD_eq_flatten _ (hp i t).1]
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
