import Hydro.Project.Model.StepSpec

/-!
# Hydro · Project — per-operator step lemmas (stateless nodes, folds, timers)

Every per-operator lemma proves `NodeSpec n sch env t`: stepping the node from the
cells the denotation prescribes before `t` (`Node.prevState`), on the denotation's increments
at `t` (`envIncAt env t`), yields the cells it prescribes at `t` (`Node.stateOf`) and the
denotation's output increments at `t`. The context is the lock-step one, `ctxAt pacing t`.
The structural cases in `Project/Model/StepMain.lean` compose these into `Dag.step_spec`.

Stateless operators reduce to one increment lemma from `Project/Model/StepSpec.lean`
(`incView_map`, `incTrace_map`, …). `spec_defer_tick` and `spec_allTicks` need the input to
be tick-monotone (`TMono`) and paced (`TPaced`), which `HTMono`/`HTPaced` supply at the
`Dag` level.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} (pacing : Pacing L mem)

/-- The lock-step context at `t`: step `t`, every member's tick and activity read from
`pacing` at `t`. -/
def ctxAt (t : Nat) : StepCtx L mem := ⟨t, fun ℓ i => pacing.ticks ℓ i t, fun ℓ i => pacing.active ℓ i t⟩

/-- The per-operator specification: `Node.step` at `ctxAt t` from `Node.prevState` on the
increments at `t` equals `Node.stateOf` at `t` paired with the denotation's increments at
`t`. -/
def NodeSpec {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs) (sch : Node.Sched n) (env : Env Γ) (t : Nat) : Prop :=
  Node.step (ctxAt pacing t) n sch (envIncAt env t) (Node.prevState pacing n sch env t)
    = (Node.stateOf pacing n sch env t, envIncAt (Node.denote pacing n sch env) t)

local notation "S" => SchedSem L mem pacing

/-! ## Stateless stream operators

Each is `congr` down to the member function and one `incView_*`/`incTrace_*` lemma; the
nodes whose step and denotation are the same `Array`/`List` function close by `rfl`. -/

theorem spec_map {Γ : Ctx L mem} {ℓ : L} {α β : Type} (iα : DecidableEq α) (iβ : DecidableEq β)
    {ord : StrOrd} (x : Var Γ (.stream ℓ α iα ord .exactlyOnce)) (f : Fin (mem ℓ) → α → β)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.map iα iβ x f) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incView (env.get x i) t).toArray.map (f i) : Fin (mem ℓ) → Array β), ()))
    = ((), ((fun i => (incView ((env.get x i).map (f i)) t).toArray : Fin (mem ℓ) → Array β), ()))
  congr 2
  funext i
  rw [incView_map, List.map_toArray]

theorem spec_filterMap {Γ : Ctx L mem} {ℓ : L} {α β : Type} (iα : DecidableEq α) (iβ : DecidableEq β)
    {ord : StrOrd} (x : Var Γ (.stream ℓ α iα ord .exactlyOnce)) (f : Fin (mem ℓ) → α → Option β)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.filterMap iα iβ x f) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => filterMapB (f i) (incView (env.get x i) t).toArray : Fin (mem ℓ) → Array β), ()))
    = ((), ((fun i => (incView ((env.get x i).filterMap (f i)) t).toArray : Fin (mem ℓ) → Array β), ()))
  congr 2
  funext i
  rw [incView_filterMap, filterMapB_toArray]

theorem spec_values {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V)
    {ord : StrOrd} {ret : Retries} (x : Var Γ (.keyed ℓ K V iK iV ord ret))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.values iK iV x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incView (env.get x i) t).toArray.map Prod.snd : Fin (mem ℓ) → Array V), ()))
    = ((), ((fun i => (incView ((env.get x i).map Prod.snd) t).toArray : Fin (mem ℓ) → Array V), ()))
  congr 2
  funext i
  rw [incView_map, List.map_toArray]

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
  show ((), ((fun i => (incView (env.get a i) t).toArray ++ (incView (env.get b i) t).toArray
      : Fin (mem ℓ) → Array α), ()))
    = ((), ((fun i => (incView (merge2 (env.get a i) (env.get b i)) t).toArray : Fin (mem ℓ) → Array α), ()))
  congr 2
  funext i
  rw [incView_merge2, List.append_toArray]

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
  show ((), ((fun i => (incTrace (env.get x i) t).head?.map List.toArray : Fin (mem ℓ) → Option (Array β)), ()))
    = ((), ((fun i => (incTrace (fun k => (env.get x i k).map List.toArray) t).head? : Fin (mem ℓ) → Option (Array β)), ()))
  congr 2
  funext i
  rw [incTrace_map, List.head?_map]

theorem spec_flattenUnordered {Γ : Ctx L mem} {ℓ : L} {β : Type} (iβ : DecidableEq β)
    (x : Var Γ (.atomic ℓ (List β)))
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.flattenUnordered iβ x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incTrace (env.get x i) t).head?.map List.toArray : Fin (mem ℓ) → Option (Array β)), ()))
    = ((), ((fun i => (incTrace (fun k => (env.get x i k).map List.toArray) t).head? : Fin (mem ℓ) → Option (Array β)), ()))
  congr 2
  funext i
  rw [incTrace_map, List.head?_map]

/-- The increment at step `t` of a trace indexed by the tick skeleton, `(tickSteps p k).map g`:
the entry `g t` when `t` ticks, nothing otherwise. -/
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
  show ((), ((fun i => (incTrace (env.get x i) t).head?.map (fun b => b[0]?) : Fin (mem ℓ) → Option (Option α)), ()))
    = ((), ((fun i => (incTrace (fun k => (env.get x i k).map (fun b => b[0]?)) t).head? : Fin (mem ℓ) → Option (Option α)), ()))
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

/-! ## Folds

The cell is `foldl g init` over the view; `view_succ_inc` splits the view at `t+1` into the
view at `t` and the increment, so folding the increment onto the previous cell is the fold
over the whole view. -/

theorem spec_fold {Γ : Ctx L mem} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd} {ret : Retries}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) (iPre : DecidableRel Pre) (ok : FoldOk ord ret Pre g)
    (x : Var Γ (.stream ℓ α iα ord ret)) (env : Env Γ) (t : Nat) :
    NodeSpec pacing (.fold iα g init Pre iPre ok x) () env t := by
  have hs : ∀ i, foldlB g (Node.prevState pacing (.fold iα g init Pre iPre ok x) () env t i) ((envIncAt env t).get x i)
      = ((env.get x i).view t).foldl g init := by
    intro i
    rw [envIncAt_get]
    cases t with
    | zero => exact foldlB_toArray _ _ _
    | succ t =>
      show foldlB g (((env.get x i).view t).foldl g init) (incView (env.get x i) (t + 1)).toArray = _
      rw [foldlB_toArray, ← List.foldl_append, ← view_succ_inc]
  simp only [NodeSpec, Node.step]
  show ((fun i => foldlB g _ ((envIncAt env t).get x i)), ((fun i => foldlB g _ ((envIncAt env t).get x i)), ())) = _
  simp only [hs]
  rfl

theorem spec_fold_monotone {Γ : Ctx L mem} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd}
    {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ) (init : σ)
    (ok : FoldOk ord ret (fun _ _ => True) g) (hinfl : ∀ s x, vo.le s (g s x))
    (x : Var Γ (.stream ℓ α iα ord ret)) (env : Env Γ) (t : Nat) :
    NodeSpec pacing (.fold_monotone iα vo g init ok hinfl x) () env t := by
  have hs : ∀ i, foldlB g
      (Node.prevState pacing (.fold_monotone iα vo g init ok hinfl x) () env t i) ((envIncAt env t).get x i)
      = ((env.get x i).view t).foldl g init := by
    intro i
    rw [envIncAt_get]
    cases t with
    | zero => exact foldlB_toArray _ _ _
    | succ t =>
      show foldlB g (((env.get x i).view t).foldl g init) (incView (env.get x i) (t + 1)).toArray = _
      rw [foldlB_toArray, ← List.foldl_append, ← view_succ_inc]
  simp only [NodeSpec, Node.step]
  show ((fun i => foldlB g _ ((envIncAt env t).get x i)), ((fun i => foldlB g _ ((envIncAt env t).get x i)), ())) = _
  simp only [hs]
  rfl

/-! ## Tick counters and the deferred wire -/

/-- `tickSteps p 0` is empty iff step `0` does not tick. -/
theorem tickSteps_isEmpty_zero (p : Nat → Bool) : (tickSteps p 0).isEmpty = !p 0 := by
  rw [tickSteps_zero]; cases p 0 <;> rfl

/-- `tickSteps p (t+1)` is empty iff `tickSteps p t` is and `t+1` does not tick. -/
theorem tickSteps_isEmpty_succ (p : Nat → Bool) (t : Nat) :
    (tickSteps p (t + 1)).isEmpty = ((tickSteps p t).isEmpty && !p (t + 1)) := by
  rw [tickSteps_succ]
  cases p (t + 1) <;> simp

/-- A step-monotone trace (`hx`) at `t+1` is the trace at `t` plus the increment at `t+1`. -/
theorem trace_prev_append_inc {σ : Type} {x : Nat → Trace σ} (hx : ∀ k k', k ≤ k' → x k <+: x k') (t : Nat) :
    x t ++ incTrace x (t + 1) = x (t + 1) := by
  obtain ⟨e, he⟩ := hx t (t + 1) (Nat.le_succ t)
  show x t ++ (x (t + 1)).drop (x t).length = x (t + 1)
  rw [← he, List.drop_left]

/-- One FIFO step of `defer_tick` as list arithmetic. The buffer is `X'.drop r`, the entries of
`init :: input` beyond the released count `r`; a tick releases its head and the released count
grows by one when something was there. The released head is the increment of `X'.take` by the
new released count, and the remaining buffer is `X'.drop` by it. -/
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

/-- `defer_tick` meets `NodeSpec`. `hx : TMono` makes the released count (which reads the input
one step back) at most the buffer's length, and `hp : TPaced` makes the step's increment at
most one slot, so the step is `defer_fifo_step` on the buffer followed by a push of the slot. -/
theorem spec_defer_tick {Γ : Ctx L mem} {ℓ : L} {σ : Type} (init : σ) (x : Var Γ (.atomic ℓ σ))
    (env : Env Γ) (hx : TMono (env.get x)) (hp : TPaced (pacing.active ℓ) (env.get x)) (t : Nat) :
    NodeSpec pacing (.defer_tick init x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, Node.stateOf, envIncAt_get, Kind.incAt, ctxAt]
  -- the released count never exceeds the buffer `init :: x`, read one step back or at `k`
  have hlen : ∀ i k, (env.get x i (k - 1)).length ≤ (env.get x i (k + 1 - 1)).length :=
    fun i k => (hx i (k - 1) (k + 1 - 1) (by omega)).length_le
  have hr : ∀ i k, deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) k
      ≤ (init :: env.get x i (k - 1)).length :=
    fun i k => by rw [List.length_cons]; exact deferReleased_le_len_succ _ _ (hlen i) k
  have hr2 : ∀ i k, deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) k
      ≤ (init :: env.get x i k).length :=
    fun i k => (hr i k).trans (List.cons_prefix_cons.mpr ⟨rfl, hx i (k - 1) k (Nat.sub_le k 1)⟩).length_le
  -- a paced input's increment is at most one slot, so the slot pushed is the whole increment
  have hslot : ∀ i k, (incTrace (env.get x i) k).head?.toList = incTrace (env.get x i) k :=
    fun i k => incTrace_eq_toList_head? hp i k
  have hD : ∀ i k, (SchedSem L mem pacing).defer_tick init (env.get x) i k
      = (init :: env.get x i (k - 1)).take (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) k) :=
    fun _ _ => rfl
  cases t with
  | zero =>
    show ((fun i => (if pacing.ticks ℓ i 0 then [init].tail else [init]) ++ (incTrace (env.get x i) 0).head?.toList),
        ((fun i => if pacing.ticks ℓ i 0 then [init].head? else none : Fin (mem ℓ) → Option σ), ())) = _
    simp only [hslot]
    simp only [incTrace_zero]
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
    show ((fun i => (if pacing.ticks ℓ i (t + 1)
            then ((init :: env.get x i t).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) t)).tail
            else (init :: env.get x i t).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) t))
              ++ (incTrace (env.get x i) (t + 1)).head?.toList),
        ((fun i => if pacing.ticks ℓ i (t + 1)
            then ((init :: env.get x i t).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) t)).head?
            else none : Fin (mem ℓ) → Option σ), ())) = _
    simp only [hslot]
    have hstep := fun i => defer_fifo_step (init :: env.get x i t) (init :: env.get x i t)
      (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) t)
      (List.prefix_refl _) (hr2 i t) (pacing.ticks ℓ i (t + 1))
    have hr' : ∀ i, deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) (t + 1)
        = if pacing.ticks ℓ i (t + 1) && decide (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) t
            < (init :: env.get x i t).length)
          then deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) t + 1
          else deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) t := by
      intro i
      rw [deferReleased_succ, List.length_cons]
      simp only [Nat.lt_succ_iff, Nat.add_sub_cancel]
    -- the released buffer with this step's slot pushed is `init :: x` at `t + 1` beyond the new
    -- released count
    have hbuf : ∀ i, (init :: env.get x i t).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) (t + 1))
        ++ incTrace (env.get x i) (t + 1)
        = (init :: env.get x i (t + 1)).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) (t + 1)) := by
      intro i
      rw [← List.drop_append_of_le_length (by simpa using hr i (t + 1)), List.cons_append, trace_prev_append_inc (hx i) t]
    congr 1
    · funext i
      rw [(hstep i).2, ← hr' i, hbuf i]
    · congr 1
      funext i
      show _ = (incTrace ((SchedSem L mem pacing).defer_tick init (env.get x) i) (t + 1)).head?
      rw [incTrace_succ, hD, hD, hr' i]
      simp only [Nat.add_sub_cancel]
      -- the input is monotone, so its `take` by the released count is the same at `t - 1` and `t`
      have hprev : (init :: env.get x i (t - 1)).take (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) t)
          = (init :: env.get x i t).take (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) t) := by
        obtain ⟨e, he⟩ : init :: env.get x i (t - 1) <+: init :: env.get x i t :=
          List.cons_prefix_cons.mpr ⟨rfl, hx i (t - 1) t (Nat.sub_le t 1)⟩
        rw [← he, List.take_append_of_le_length (hr i t)]
      rw [hprev]
      exact (hstep i).1

/-! ## Timing scripts along the tick skeleton

A timer node's trace at step `t` is the script's first `(tickSteps p t).length` entries, so
its increment at a tick step is the script's entry at the previous tick count. -/

/-- The increment of a script trace at `t`: the entry at the previous tick count when `t`
ticks, nothing otherwise. -/
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

/-- The tick count at `t` is the previous tick count plus one when `t` ticks. -/
theorem tickSteps_length_ite (p : Nat → Bool) (t : Nat) :
    (tickSteps p t).length = if p t then (prevTicks p t).length + 1 else (prevTicks p t).length := by
  rw [tickSteps_eq_prev, List.length_append]
  split <;> simp

/-- The interval stream's increment at step `t`: the script's fire count at the previous tick
count, as `()`s, when `t` ticks; nothing otherwise. -/
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

/-- `source_interval` meets `NodeSpec`: the cell is the tick count (`tickSteps_length_ite`) and
`intervalHist_incView` gives the output. -/
theorem spec_source_interval {Γ : Ctx L mem} {ℓ : L} (delay : Fin (mem ℓ) → Nat) (period : Nat)
    (d : TickScript (mem ℓ) Nat)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.source_interval (Γ := Γ) (ℓ := ℓ) delay period) d env t := by
  simp only [NodeSpec, Node.step, Node.denote]
  have hp : Node.prevState pacing (.source_interval (Γ := Γ) (ℓ := ℓ) delay period) d env t
      = fun i => (prevTicks (pacing.ticks ℓ i) t).length := by
    cases t <;> rfl
  rw [hp]
  show ((fun i => if pacing.ticks ℓ i t then (prevTicks (pacing.ticks ℓ i) t).length + 1 else (prevTicks (pacing.ticks ℓ i) t).length),
      ((fun i => if pacing.ticks ℓ i t then Array.replicate (d i (prevTicks (pacing.ticks ℓ i) t).length) () else #[]
        : Fin (mem ℓ) → Array Unit), ()))
    = ((fun i => (tickSteps (pacing.ticks ℓ i) t).length),
      ((fun i => (incView (intervalHist (pacing.ticks ℓ i) (d i)) t).toArray : Fin (mem ℓ) → Array Unit), ()))
  congr 2
  · funext i; exact (tickSteps_length_ite _ t).symm
  · funext i
    apply Array.toList_inj.mp
    rw [List.toList_toArray, intervalHist_incView, apply_ite Array.toList, Array.toList_replicate, Array.toList_empty]
/-! ## `allTicks`: the frozen family is the raw one under tick-monotonicity -/

/-- A list of at most one entry: the head's default `[]` is the flatten. -/
theorem head?_getD_eq_flatten {β : Type} (l : List (List β)) (h : l.length ≤ 1) :
    l.head?.getD [] = l.flatten := by
  match l, h with
  | [], _ => rfl
  | [b], _ => simp
  | _ :: _ :: _, h => simp at h

/-- `getD #[]` then `toList` is `map toList` then `getD []`. -/
theorem toList_getD_empty {β : Type} (o : Option (Array β)) :
    (o.getD #[]).toList = (o.map Array.toList).getD [] := by
  cases o <;> rfl

/-- `allTicks` meets `NodeSpec`. `hx : TMono` makes the raw family prefix-monotone, so the
denotation's `famFreeze` is the identity (`famFreeze_eq_of_mono`); `hp : TPaced` bounds the
step's increment to one slot, so the slot's batch is the increment's flatten. -/
theorem spec_allTicks {Γ : Ctx L mem} {ℓ : L} {β : Type} (iβ : DecidableEq β) {ord : StrOrd}
    (x : Var Γ (.atomicStream ℓ β iβ ord .exactlyOnce)) (env : Env Γ) (t : Nat)
    (hx : TMono (env.get x)) (hp : TPaced (pacing.active ℓ) (env.get x)) :
    NodeSpec pacing (.allTicks iβ x) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incTrace (env.get x i) t).head?.getD #[] : Fin (mem ℓ) → Array β), ()))
    = ((), ((fun i => (incView (famHist (fun step j => ((env.get x j step).map Array.toList).flatten) i) t).toArray
        : Fin (mem ℓ) → Array β), ()))
  congr 2
  funext i
  apply Array.toList_inj.mp
  rw [List.toList_toArray, toList_getD_empty]
  rw [← List.head?_map, ← incTrace_map, head?_getD_eq_flatten _ (by
    rw [incTrace_map, List.length_map]; exact (hp i t).1)]
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

end Project

end Hydro
