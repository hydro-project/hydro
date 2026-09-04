import Hydro.Project.Step
import Hydro.Project.Knot

/-!
# Hydro · Project — the interpreter reproduces the denotation

`Dag.step_spec`: stepping the graph from the cells the denotation
prescribes for step `t-1`, on the denotation's increments at `t`, yields
the cells it prescribes for `t` and the denotation's output increments
at `t`. `Dag.run_spec` iterates it from the initial cells.

The **state abstraction** `Node.stateOf` says, per node kind, what the
cell must hold at the end of step `t` as a function of the `SchedSem`
wires (a fold's `foldl` over the view, a batch's `drop consumed`, a
channel's undelivered suffix, a knot's increment, …). The per-operator
lemmas are about `Sched.lean`'s definitions; the structural cases
(`letN`, `call`, `fixN`) compose them.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-! ## Views and traces as prefix + increment -/

/-- The view before step `t` (`[]` before step `0`). -/
def prevView {α : Type} (h : StepHist α) : Nat → List α
  | 0 => []
  | t + 1 => h.view t

theorem view_eq_prev_inc {α : Type} (h : StepHist α) (t : Nat) :
    h.view t = prevView h t ++ incView h t := by
  cases t with
  | zero => rfl
  | succ t => exact h.view_succ t

theorem prevView_prefix {α : Type} (h : StepHist α) (t : Nat) : prevView h t <+: h.view t :=
  ⟨incView h t, (view_eq_prev_inc h t).symm⟩

/-- The trace before step `t`. -/
def prevTrace {σ : Type} (tr : Nat → Trace σ) : Nat → Trace σ
  | 0 => []
  | t + 1 => tr t

/-- For a step-monotone trace, the trace at `t` is the previous trace
plus the ticks appended at `t`. -/
theorem trace_eq_prev_inc {σ : Type} (tr : Nat → Trace σ) (hm : ∀ t, tr t <+: tr (t + 1)) (t : Nat) :
    tr t = prevTrace tr t ++ incTrace tr t := by
  cases t with
  | zero => rfl
  | succ t =>
    show tr (t + 1) = tr t ++ (tr (t + 1)).drop (tr t).length
    obtain ⟨e, he⟩ := hm t
    rw [← he, List.drop_left]

theorem incView_map {α β : Type} (h : StepHist α) (f : α → β) (t : Nat) :
    incView (h.map f) t = (incView h t).map f := by
  cases t with
  | zero => rfl
  | succ t =>
    show ((h.view (t + 1)).map f).drop ((h.view t).map f).length = (h.inc t).map f
    rw [List.length_map, ← List.map_drop]
    rfl

theorem incView_filterMap {α β : Type} (h : StepHist α) (f : α → Option β) (t : Nat) :
    incView (h.filterMap f) t = (incView h t).filterMap f := by
  cases t with
  | zero => rfl
  | succ t =>
    show ((h.view (t + 1)).filterMap f).drop ((h.view t).filterMap f).length = (h.inc t).filterMap f
    rw [h.view_succ t, List.filterMap_append, List.drop_left]

theorem incView_merge2 {α : Type} (a b : StepHist α) (t : Nat) :
    incView (merge2 a b) t = incView a t ++ incView b t := by
  cases t with
  | zero => rfl
  | succ t =>
    show (merge2View a b (t + 1)).drop (merge2View a b t).length = a.inc t ++ b.inc t
    show (merge2View a b t ++ a.inc t ++ b.inc t).drop (merge2View a b t).length = _
    rw [List.append_assoc, List.drop_left]

theorem incView_mergeN {m : Nat} {α : Type} (k : Fin m → StepHist α) (t : Nat) :
    incView (mergeN k) t = (List.finRange m).flatMap (fun j => incView (k j) t) := by
  cases t with
  | zero => rfl
  | succ t =>
    show (mergeNView k (t + 1)).drop (mergeNView k t).length = _
    show (mergeNView k t ++ (List.finRange m).flatMap (fun j => (k j).inc t)).drop
      (mergeNView k t).length = _
    rw [List.drop_left]
    rfl

/-- The delivered view at a ticking step `t`: the cursor's prefix of the
sender's previous view. -/
theorem deliver_view_eq {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat)
    (hp : p t = true) : (h.deliver p c).view t = (prevView h t).take (cumMax c t) := by
  cases t with
  | zero => show [] = List.take _ []; rw [List.take_nil]
  | succ t => show gateView p _ (t + 1) = _; rw [gateView_succ, if_pos hp]; rfl

/-- At a non-ticking step the delivered view holds. -/
theorem deliver_view_eq_prev {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat)
    (hp : p t = false) : (h.deliver p c).view t = prevView (h.deliver p c) t := by
  cases t with
  | zero => rfl
  | succ t => show gateView p _ (t + 1) = _; rw [gateView_succ, if_neg (by simp [hp])]; rfl

/-- The delivered view is a prefix of the sender's previous view. -/
theorem deliver_view_prefix_prev {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) :
    ∀ t, (h.deliver p c).view t <+: prevView h t
  | 0 => List.nil_prefix
  | t + 1 => by
    show gateView p _ (t + 1) <+: h.view t
    rw [gateView_succ]
    split
    · exact List.take_prefix _ _
    · exact (deliver_view_prefix_prev h p c t).trans (prevView_prefix h t)

/-- The pending suffix and delivered count of a channel pair at the end
of step `t`: the sender's view beyond what has been delivered. -/
def chanCellAt {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) : ChanCell α :=
  ⟨(h.view t).drop ((h.deliver p c).view t).length, ((h.deliver p c).view t).length⟩

/-! ## The state abstraction -/

/-- The consumed count after a run of tick steps (the threading of
`batchesFrom`). -/
def consumedAfter {α : Type} (src : Nat → List α) : List Nat → Nat → Nat
  | [], c => c
  | s :: rest, _ => consumedAfter src rest (src s).length

theorem batchesFrom_append {α : Type} (src : Nat → List α) :
    ∀ (steps : List Nat) (u : Nat) (c : Nat),
      batchesFrom src (steps ++ [u]) c
        = batchesFrom src steps c ++ [(src u).drop (consumedAfter src steps c)]
  | [], u, c => rfl
  | s :: rest, u, c => by
    show (src s).drop c :: batchesFrom src (rest ++ [u]) (src s).length = _
    rw [batchesFrom_append src rest u]
    rfl

theorem consumedAfter_le {α : Type} (src : Nat → List α) (hm : ∀ s s', s ≤ s' → src s <+: src s') :
    ∀ (steps : List Nat) (c : Nat) (t : Nat), (∀ s ∈ steps, s ≤ t) → c ≤ (src t).length →
      consumedAfter src steps c ≤ (src t).length
  | [], c, t, _, hc => hc
  | s :: rest, c, t, hs, _ => by
    show consumedAfter src rest (src s).length ≤ (src t).length
    exact consumedAfter_le src hm rest _ t (fun u hu => hs u (List.mem_cons_of_mem _ hu))
      (hm s t (hs s (List.mem_cons_self ..))).length_le

/-- The position reached by `sampleAtOpt`'s walk. -/
def samplePos {α : Type} (tr : Trace (Option α)) : List Nat → Nat
  | [] => 0
  | u :: us => if u < tr.length then samplePos tr us + 1 else 0

/-- One scan of the early-stopping keyed fold over an arrival list. -/
def kfesOf {K V A : Type} [DecidableEq K] (f : A → V → A × Bool) (init : A) (l : List (K × V)) :
    KFESCell K A :=
  l.foldl (kfesInsert f init) ⟨[], List.nodup_nil⟩

/-- The pending feedback of a knot wire at the end of step `t`: the body's
knot output beyond what the knot has released. -/
def KnotKind.pendingAt : (k : KnotKind L mem) → k.toKind.sched → k.toKind.sched → Nat → k.toKind.delta
  | .stream ℓ _ _ _ _, x, y, t => fun (i : Fin (mem ℓ)) => ((y i).view t).drop ((x i).view t).length
  | .atomic ℓ _, x, y, t => fun (i : Fin (mem ℓ)) => (y i t).drop (x i t).length

def knotPendingAt : (ks : List (KnotKind L mem)) →
    HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
    HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) → Nat →
    HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)
  | [], _, _, _ => HList.nil
  | k :: ks, x, y, t => HList.cons (k.pendingAt x.1 y.1 t) (knotPendingAt ks x.2 y.2 t)

/-- The pending suffixes of a shaped input at step `t`: each leaf beyond
the jointly realized ticks. -/
def pendingAt {ℓ : L} : (sh : TickShape) →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh →
    Fin (mem ℓ) → Nat → Nat → PendingOf sh
  | .sing _, x, i, t, m => (x i t).drop m
  | .stream _ _ _ _, x, i, t, m => (x i t).drop m
  | .ksing _ _ _ _, x, i, t, m => (x i t).drop m
  | .pair a b, x, i, t, m => (pendingAt a x.1 i t m, pendingAt b x.2 i t m)

section StateOf
variable (pacing : Pacing L mem)

local notation "S" => SchedSem L mem pacing

mutual

/-- What a node's cells hold at the end of step `t`, read off the
machine wires. -/
def Node.stateOf : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Sched n → Env Γ → Nat →
    Node.State n
  | _, _, .broadcast_closed (p := p) _ x, d, env, t => fun i j => chanCellAt (env.get x j) (pacing.active p i) (d i j) t
  | _, _, .demux (p := p) _ x addr, d, env, t => fun i j =>
    chanCellAt ((env.get x j).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none))
      (pacing.active p i) (d i j) t
  | _, _, .kfold_early_stop _ _ _ x f init, _, env, t => fun i => kfesOf f init ((env.get x i).view t)
  | _, _, .fold _ g init _ _ _ x, _, env, t => fun i => ((env.get x i).view t).foldl g init
  | _, _, .fold_monotone _ _ g init _ _ x, _, env, t => fun i => ((env.get x i).view t).foldl g init
  | _, _, .batch (ℓ := ℓ) _ x _, _, env, t => fun i =>
    ((env.get x i).view t).drop (consumedAfter ((env.get x i).view) (tickSteps (pacing.ticks ℓ i) t) 0)
  | _, _, .batch_ordered (ℓ := ℓ) _ x _, _, env, t => fun i =>
    ((env.get x i).view t).drop (consumedAfter ((env.get x i).view) (tickSteps (pacing.ticks ℓ i) t) 0)
  | _, _, .sample_every _ x, d, env, t => fun i => (env.get x i t, samplePos (env.get x i t) (d i))
  | _, _, .timeout_snapshot (ℓ := ℓ) _ _, _, _, t => fun i => (tickSteps (pacing.ticks ℓ i) t).length
  | _, _, .source_interval_batch (ℓ := ℓ), _, _, t => fun i => (tickSteps (pacing.ticks ℓ i) t).length
  | _, _, .zipTick a b, _, env, t => fun i =>
    ((env.get a i t).drop (min (env.get a i t).length (env.get b i t).length),
     (env.get b i t).drop (min (env.get a i t).length (env.get b i t).length))
  | _, _, .defer_tick (ℓ := ℓ) _ x, _, env, t => fun i =>
    (!(tickSteps (pacing.ticks ℓ i) t).isEmpty, if (tickSteps (pacing.ticks ℓ i) t).isEmpty then env.get x i t else [])
  | _, _, .reduce_watermark _ _ _ _ x wm f _ _ _, _, env, t => fun i =>
    (((env.get x i t).drop (min (env.get x i t).length (env.get wm i t).length),
      (env.get wm i t).drop (min (env.get x i t).length (env.get wm i t).length)),
     (scanRun (reduceWatermarkEager f) (SchedKMap.empty, none)
       (Trace.zip (env.get x i t) (env.get wm i t))).1)
  | _, _, .tick_scan (ℓ := ℓ) sts ins _ x g seed, _, env, t => fun i =>
    (pendingAt ins (readShape env ℓ ins x) i t
      (SchedTick.slice ins (readShape env ℓ ins x) i t).length,
     (scanRun (g i) (SchedTick.seed sts seed) (SchedTick.slice ins (readShape env ℓ ins x) i t)).1)
  | _, _, .call N args, sch, env, t => Dag.stateOf N sch (args.map (fun v => env.get v)) t
  | _, _, .fixN ks body, sch, env, t =>
    let closed := bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)
    (Dag.stateOf body sch (closed.append env) t,
     knotPendingAt ks closed (Dag.denote pacing body sch (closed.append env)).take t)
  | _, _, .map .., _, _, _ => ()
  | _, _, .filterMap .., _, _, _ => ()
  | _, _, .values .., _, _, _ => ()
  | _, _, .into_keyed .., _, _, _ => ()
  | _, _, .kassume_ordering .., _, _, _ => ()
  | _, _, .kget_max_key .., _, _, _ => ()
  | _, _, .weaken_retries .., _, _, _ => ()
  | _, _, .union .., _, _, _ => ()
  | _, _, .assume_ordering .., _, _, _ => ()
  | _, _, .snapshot .., _, _, _ => ()
  | _, _, .mapTick .., _, _, _ => ()
  | _, _, .allTicks .., _, _, _ => ()
  | _, _, .flattenOrdered .., _, _, _ => ()
  | _, _, .flattenUnordered .., _, _, _ => ()
  | _, _, .tinto_keyed .., _, _, _ => ()
  | _, _, .tassume_ordering .., _, _, _ => ()

def Dag.stateOf : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Sched d → Env Γ → Nat →
    Dag.State d
  | _, _, .ret _, _, _, _ => ()
  | _, _, .letN n rest, sch, env, t =>
    (Node.stateOf n sch.1 env t, Dag.stateOf rest sch.2 ((Node.denote pacing n sch.1 env).append env) t)

end

/-- The cells before step `t`. -/
def Dag.prevState {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) (sch : Dag.Sched d) (env : Env Γ) :
    Nat → Dag.State d
  | 0 => Dag.init d
  | t + 1 => Dag.stateOf pacing d sch env t

def Node.prevState {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs) (sch : Node.Sched n) (env : Env Γ) :
    Nat → Node.State n
  | 0 => Node.init n
  | t + 1 => Node.stateOf pacing n sch env t

end StateOf

/-! ## Increment environments -/

theorem envIncAt_get (t : Nat) : {Γ : Ctx L mem} → {κ : Kind L mem} → (env : Env Γ) → (v : Var Γ κ) →
    (envIncAt env t).get v = Kind.incAt κ (env.get v) t
  | _, _, _, .zero => rfl
  | _, _, env, .succ v => envIncAt_get t env.2 v

theorem envIncAt_append : {κs Γ : List (Kind L mem)} → (xs : HList (Kind.sched (mem := mem)) κs) →
    (ys : HList (Kind.sched (mem := mem)) Γ) → (t : Nat) →
    envIncAt (xs.append ys) t = (envIncAt xs t).append (envIncAt ys t)
  | [], _, _, _, _ => rfl
  | _ :: _, _, xs, ys, t => by
    show (Kind.incAt _ xs.1 t, envIncAt (xs.2.append ys) t) = (Kind.incAt _ xs.1 t, (envIncAt xs.2 t).append (envIncAt ys t))
    rw [envIncAt_append xs.2 ys t]

theorem envIncAt_take : {κs Γ : List (Kind L mem)} → (xs : HList (Kind.sched (mem := mem)) (κs ++ Γ)) →
    (t : Nat) → envIncAt (HList.take (κs := κs) xs) t = HList.take (κs := κs) (envIncAt xs t)
  | [], _, _, _ => rfl
  | _ :: κs, Γ, xs, t => by
    show (Kind.incAt _ xs.1 t, envIncAt (HList.take (κs := κs) xs.2) t) = (Kind.incAt _ xs.1 t, HList.take (κs := κs) (envIncAt xs.2 t))
    rw [envIncAt_take (κs := κs) (Γ := Γ) xs.2 t]

theorem envIncAt_drop : {κs Γ : List (Kind L mem)} → (xs : HList (Kind.sched (mem := mem)) (κs ++ Γ)) →
    (t : Nat) → envIncAt (HList.drop (κs := κs) xs) t = HList.drop (κs := κs) (envIncAt xs t)
  | [], _, _, _ => rfl
  | _ :: κs, Γ, xs, t => envIncAt_drop (κs := κs) (Γ := Γ) xs.2 t

theorem envIncAt_mapSubst {Γ Γ' : Ctx L mem} (env : Env Γ) (args : Subst Γ' Γ) (t : Nat) :
    envIncAt (args.map (fun v => env.get v)) t = args.map (fun v => (envIncAt env t).get v) := by
  induction Γ' with
  | nil => rfl
  | cons κ κs ih =>
    show (Kind.incAt _ (env.get args.1) t, envIncAt (args.2.map (fun v => env.get v)) t)
      = ((envIncAt env t).get args.1, args.2.map (fun v => (envIncAt env t).get v))
    rw [ih args.2, envIncAt_get]

/-! ## List and skeleton lemmas -/

theorem take_succ_drop_take {α : Type} (l : List α) (n : Nat) :
    (l.take (n + 1)).drop (l.take n).length = (l[n]?).toList := by
  induction l generalizing n with
  | nil => simp
  | cons a l ih =>
    cases n with
    | zero => simp
    | succ n =>
      simp only [List.take_succ_cons, List.length_cons, List.drop_succ_cons, List.getElem?_cons_succ]
      exact ih n

theorem drop_zip {α β : Type} (a : List α) (b : List β) (n : Nat) :
    (List.zip a b).drop n = List.zip (a.drop n) (b.drop n) := by
  induction a generalizing b n with
  | nil => simp
  | cons x xs ih =>
    cases b with
    | nil => simp
    | cons y ys =>
      cases n with
      | zero => rfl
      | succ n => simp only [List.zip_cons_cons, List.drop_succ_cons]; exact ih ys n

theorem tickSteps_succ (p : Nat → Bool) (t : Nat) :
    tickSteps p (t + 1) = tickSteps p t ++ (if p (t + 1) then [t + 1] else []) := by
  unfold tickSteps
  rw [List.range_succ, List.filter_append]
  congr 1
  simp only [List.filter_cons, List.filter_nil]

theorem tickSteps_zero (p : Nat → Bool) : tickSteps p 0 = (if p 0 then [0] else []) := by
  unfold tickSteps
  simp only [List.range_succ, List.range_zero, List.nil_append, List.filter_cons, List.filter_nil]

theorem tickSteps_le (p : Nat → Bool) (t : Nat) : ∀ s ∈ tickSteps p t, s ≤ t :=
  fun _ hs => tickSteps_mem_le hs

/-- The skeleton before step `t`. -/
def prevTicks (p : Nat → Bool) : Nat → List Nat
  | 0 => []
  | t + 1 => tickSteps p t

theorem tickSteps_eq_prev (p : Nat → Bool) (t : Nat) :
    tickSteps p t = prevTicks p t ++ (if p t then [t] else []) := by
  cases t with
  | zero => exact tickSteps_zero p
  | succ t => exact tickSteps_succ p t

theorem view_succ_inc {α : Type} (h : StepHist α) (t : Nat) :
    h.view (t + 1) = h.view t ++ incView h (t + 1) := h.view_succ t

theorem incTrace_map {σ τ : Type} (tr : Nat → Trace σ) (f : σ → τ) (t : Nat) :
    incTrace (fun k => (tr k).map f) t = (incTrace tr t).map f := by
  cases t with
  | zero => rfl
  | succ t =>
    show ((tr (t + 1)).map f).drop ((tr t).map f).length = ((tr (t + 1)).drop (tr t).length).map f
    rw [List.length_map, List.map_drop]

/-- For a monotone trace, the increment at `t+1` is the suffix beyond the trace at `t`. -/
theorem trace_succ_inc {σ : Type} (tr : Nat → Trace σ) (hm : ∀ t, tr t <+: tr (t + 1)) (t : Nat) :
    tr (t + 1) = tr t ++ incTrace tr (t + 1) := trace_eq_prev_inc tr hm (t + 1)

theorem scanRun_eq {ι σ β : Type} (g : σ → ι → σ × β) (s : σ) (l : List ι) :
    (scanRun g s l).2 = scanAcrossTicksTrace g s l := by
  induction l generalizing s with
  | nil => rfl
  | cons x xs ih =>
    show ((scanRun g (g s x).1 xs).1, (g s x).2 :: (scanRun g (g s x).1 xs).2).2
      = (g s x).2 :: scanAcrossTicksTrace g (g s x).1 xs
    rw [← ih]

theorem scanRun_append {ι σ β : Type} (g : σ → ι → σ × β) (s : σ) (l l' : List ι) :
    scanRun g s (l ++ l') = ((scanRun g (scanRun g s l).1 l').1,
      (scanRun g s l).2 ++ (scanRun g (scanRun g s l).1 l').2) := by
  induction l generalizing s with
  | nil => rfl
  | cons x xs ih =>
    show ((scanRun g (g s x).1 (xs ++ l')).1, (g s x).2 :: (scanRun g (g s x).1 (xs ++ l')).2) = _
    rw [ih]
    rfl

end Project

end Hydro
