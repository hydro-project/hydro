import Hydro.Project.Model.StepOps6
import Hydro.Project.Model.StepAgree

/-!
# Hydro · Project — `Dag.step_spec`: the interpreter reproduces the denotation

Structural induction over the graph. Each operator case is its lemma
(`ProjectStepOps*.lean`); `letN` composes by extending the environment
with the node's denotation, `call` by substitution, `fixN` by the knot
equation (`Project/Model/Knot.lean`, D95): the closed knots' increments at a
step are the body's knot outputs' increments at that step (`bekic_fix`) — a
fixpoint of the body's step in the knot increments —, and the reference
machine's `fixN` iterates the body's step from empty knot increments to that
fixpoint (`Dag.step_knotIter_agree0`, `iter_zero`), so it is the body stepped
at the closed knots' increments (`Node.step_fixN_eq`).

The hypotheses: `Dag.valid d` (every knot body cascade-valid — every cycle
among its knots passes a strict node, what makes each knot its body's
fixpoint), `HTMono env` (every atomic input
is tick-monotone — what the tick-buffer cells need; `Dag.denote_tmono`
propagates it) and `HTPaced pacing env` (single slots).
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} (pacing : Pacing L mem)

/-! ## The structural theorem -/

/-- The cells before step `t` of a graph, structurally. -/
theorem Dag.prevState_letN {Γ : Ctx L mem} {κs outs : List (Kind L mem)} (n : Node Γ κs)
    (rest : Dag (κs ++ Γ) outs) (env : Env Γ) (t : Nat)
    (sch : Dag.Sched (.letN n rest)) :
    Dag.prevState pacing (.letN n rest) sch env t
      = (Node.prevState pacing n sch.1 env t, Dag.prevState pacing rest sch.2 ((Node.denote pacing n sch.1 env).append env) t) := by
  cases t <;> rfl

theorem Node.prevState_call {Γ Γ' : Ctx L mem} {outs : List (Kind L mem)} (N : Dag Γ' outs) (args : Subst Γ' Γ)
    (sch : Node.Sched (.call N args)) (env : Env Γ) (t : Nat) :
    Node.prevState pacing (.call N args) sch env t = Dag.prevState pacing N sch (args.map (fun v => env.get v)) t := by
  cases t <;> rfl

theorem Node.prevState_fixN {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs))
    (sch : Node.Sched (.fixN ks body)) (env : Env Γ) (t : Nat) :
    Node.prevState pacing (.fixN ks body) sch env t
      = Dag.prevState pacing body sch
          ((bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)).append env) t := by
  cases t <;> rfl

mutual

theorem Node.step_spec : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (sch : Node.Sched n) →
    Node.valid n = true → ∀ (env : Env Γ), HTMono env → HTPaced pacing env → ∀ (t : Nat), NodeSpec pacing n sch env t
  | _, _, .map iα iβ x f, _, _, env, _, _, t => spec_map pacing iα iβ x f env t
  | _, _, .filterMap iα iβ x f, _, _, env, _, _, t => spec_filterMap pacing iα iβ x f env t
  | _, _, .broadcast_closed iα x, d, _, env, _, _, t => spec_broadcast_closed pacing iα d x env t
  | _, _, .demux iα x addr, d, _, env, _, _, t => spec_demux pacing iα d x addr env t
  | _, _, .values iK iV x, _, _, env, _, _, t => spec_values pacing iK iV x env t
  | _, _, .into_keyed iK iV x, _, _, env, _, _, t => spec_into_keyed pacing iK iV x env t
  | _, _, .kassume_ordering iK iV x d, _, _, env, _, _, t => spec_kassume_ordering pacing iK iV x d env t
  | _, _, .kfold_early_stop iK iV iA x f init, _, _, env, _, _, t => spec_kfold_early_stop pacing iK iV iA x f init env t
  | _, _, .kget_max_key iα iK iV lo x, _, _, env, _, _, t => spec_kget_max_key pacing iα iK iV lo x env t
  | _, _, .weaken_retries iα x, _, _, env, _, _, t => spec_weaken_retries pacing iα x env t
  | _, _, .union iα a b, _, _, env, _, _, t => spec_union pacing iα a b env t
  | _, _, .assume_ordering iα x d, _, _, env, _, _, t => spec_assume_ordering pacing iα x d env t
  | _, _, .fold iα g init Pre iPre ok x, _, _, env, _, _, t => spec_fold pacing iα g init Pre iPre ok x env t
  | _, _, .fold_monotone iα vo g init ok hinfl x, _, _, env, _, _, t =>
    spec_fold_monotone pacing iα vo g init ok hinfl x env t
  | _, _, .snapshot iα x d, _, _, env, _, _, t => spec_snapshot pacing iα x d env t
  | _, _, .batch iα ord ret x d, _, _, env, _, _, t => spec_batch pacing iα ord ret x d env t
  | _, _, .source_interval delay period, d, _, env, _, _, t => spec_source_interval pacing delay period d env t
  | _, _, .mapTick x f, _, _, env, _, _, t => spec_mapTick pacing x f env t
  | _, _, .firstTick iα ret x, _, _, env, _, _, t => spec_firstTick pacing iα ret x env t
  | _, _, .zipTick a b, _, _, env, he, hp, t => spec_zipTick pacing a b env t (he.get a) (he.get b) (hp.get a) (hp.get b)
  | _, _, .defer_tick init x, _, _, env, he, hp, t => spec_defer_tick pacing init x env (he.get x) (hp.get x) t
  | _, _, .allTicks iβ x, _, _, env, he, hp, t => spec_allTicks pacing iβ x env t (he.get x) (hp.get x)
  | _, _, .flattenOrdered iβ x, _, _, env, _, _, t => spec_flattenOrdered pacing iβ x env t
  | _, _, .flattenUnordered iβ x, _, _, env, _, _, t => spec_flattenUnordered pacing iβ x env t
  | _, _, .tinto_keyed iK iV x, _, _, env, _, _, t => spec_tinto_keyed pacing iK iV x env t
  | _, _, .tassume_ordering iα x d, _, _, env, _, _, t => spec_tassume_ordering pacing iα x d env t
  | _, _, .reduce_watermark iK iH iV lo x wm f Pre iPre ok, _, _, env, he, hp, t =>
    spec_reduce_watermark pacing iK iH iV lo x wm f Pre iPre ok env t (he.get x) (he.get wm) (hp.get x) (hp.get wm)
  | _, _, .kentries iK iV x, _, _, env, _, _, t => spec_kentries pacing iK iV x env t
  | _, _, .kmap iK iV iW x f, _, _, env, _, _, t => spec_kmap pacing iK iV iW x f env t
  | _, _, .kallTicks iK iV x, _, _, env, he, hp, t => spec_kallTicks pacing iK iV x env t (he.get x) (hp.get x)
  | _, _, .kfold iK iH iV iA g init Pre iPre ok x, _, _, env, _, _, t =>
    spec_kfold pacing iK iH iV iA g init Pre iPre ok x env t
  | _, _, .ksnapshot iα iK iV x d, _, _, env, _, _, t => spec_ksnapshot pacing iα iK iV x d env t
  | _, _, .tfold iα g init Pre iPre ok x, _, _, env, he, hp, t =>
    spec_tfold pacing iα g init Pre iPre ok x env t (he.get x) (hp.get x)
  | _, _, .latest iσ x, _, _, env, he, hp, t => spec_latest pacing x env t (he.get x) (hp.get x) iσ
  | _, _, .latest_monotone x, _, _, env, he, hp, t => spec_latest_monotone pacing x env t (he.get x) (hp.get x)
  | _, _, .time, d, _, env, _, _, t => spec_time pacing d env t
  | _, _, .snapshot_bounded v d, _, _, env, _, _, t => spec_snapshot_bounded pacing v d env t
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, _, _, env, he, hp, t =>
    spec_tick_scan pacing sts ins outs x g seed env t (readShape_tmono he ℓ ins x) (readShape_tpaced hp ℓ ins x)
  | _, _, .call N args, sch, hv, env, he, hp, t => by
    show Node.step (ctxAt pacing t) (.call N args) sch (envIncAt env t) (Node.prevState pacing (.call N args) sch env t) = _
    rw [Node.prevState_call]
    simp only [Node.step, Node.denote, Node.stateOf]
    rw [← envIncAt_mapSubst]
    exact Dag.step_spec N sch hv _ (he.mapSubst args) (hp.mapSubst args) t
  | _, _, .fixN ks body, sch, hv, env, he, hp, t => by
    show Node.step (ctxAt pacing t) (.fixN ks body) sch (envIncAt env t) (Node.prevState pacing (.fixN ks body) sch env t) = _
    rw [Node.prevState_fixN]
    have hvb := (Node.valid_fixN ks body hv).2
    -- the closed knots: strict body, tick-monotone, single-slot
    have hs := Dag.fixN_strict pacing ks body sch hv env
    have hcl : HTMono (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)) :=
      bekic_tmono pacing ks hs (fun xs hxs => (Dag.denote_tmono pacing body sch hvb _ (hxs.append he)).take)
    have hclp : HTPaced pacing (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)) :=
      bekic_tpaced ks hs
        (fun xs hxs => (Dag.denote_tmono pacing body sch hvb _ (hxs.append he)).take)
        (fun xs hxm hxs => (Dag.denote_tpaced pacing body sch hvb _ (hxm.append he) (hxs.append hp)).take)
    -- the body stepped at the closed knots' increments is the denotation's step
    have ih := Dag.step_spec body sch hvb _ (hcl.append he) (hclp.append hp) t
    -- the knot equation: the closed knots ARE the body's knot outputs
    have hfix : (Dag.denote pacing body sch
        ((bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)).append env)).take
        = bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take) := bekic_fix pacing ks hs
    -- so the closed knots' increments are a fixpoint of the body's step
    have hK : (Dag.step (ctxAt pacing t) body sch
        ((envIncAt (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)) t).append (envIncAt env t))
        (Dag.prevState pacing body sch
          ((bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)).append env) t)).2.take
        = envIncAt (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)) t := by
      rw [← envIncAt_append, ih]
      show HList.take (envIncAt (Dag.denote pacing body sch _) t) = _
      rw [← envIncAt_take]
      exact congrArg (fun w => envIncAt w t) hfix
    rw [Node.step_fixN_eq (ctxAt pacing t) ks body sch hv _ _ _ hK, ← envIncAt_append, ih]
    show (Dag.stateOf pacing body sch _ t, envIncAt (Dag.denote pacing body sch _) t)
      = (Dag.stateOf pacing body sch _ t,
         envIncAt ((bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)).append
           (Dag.denote pacing body sch _).drop) t)
    rw [envIncAt_append, envIncAt_drop]
    conv_lhs => rw [← HList.take_append_drop (κs := ks.map KnotKind.toKind) (envIncAt (Dag.denote pacing body sch _) t)]
    rw [← envIncAt_take, hfix]

theorem Dag.step_spec : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    Dag.valid d = true → ∀ (env : Env Γ), HTMono env → HTPaced pacing env → ∀ (t : Nat),
    Dag.step (ctxAt pacing t) d sch (envIncAt env t) (Dag.prevState pacing d sch env t)
      = (Dag.stateOf pacing d sch env t, envIncAt (Dag.denote pacing d sch env) t)
  | _, _, .ret xs, sch, _, env, _, _, t => by
    show (Dag.prevState pacing (.ret xs) sch env t, xs.map (fun v => (envIncAt env t).get v))
      = ((), envIncAt (xs.map (fun v => env.get v)) t)
    rw [envIncAt_mapSubst]
    cases t <;> rfl
  | _, _, .letN n rest, sch, hv, env, he, hp, t => by
    rw [Dag.prevState_letN]
    simp only [Dag.step, Dag.denote, Dag.stateOf]
    have hn := Node.step_spec n sch.1 (Dag.valid_letN n rest hv).1 env he hp t
    simp only [NodeSpec] at hn
    rw [hn, ← envIncAt_append]
    rw [Dag.step_spec rest sch.2 (Dag.valid_letN n rest hv).2 _
      ((Node.denote_tmono pacing n sch.1 (Dag.valid_letN n rest hv).1 env he).append he)
      ((Node.denote_tpaced pacing n sch.1 (Dag.valid_letN n rest hv).1 env he hp).append hp) t]

end

/-! ## The lock-step run -/

/-- **The lock-step machine reproduces the machine semantics**: after
step `t` its cells are the prescribed ones and its outputs at `t` are
the denotation's increments at `t`. -/
theorem Dag.run_spec {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) (sch : Dag.Sched d)
    (hv : Dag.valid d = true) (env : Env Γ)
    (he : HTMono env) (hp : HTPaced pacing env) (t : Nat) :
    Dag.run d sch pacing (fun t => envIncAt env t) t
      = (Dag.stateOf pacing d sch env t, envIncAt (Dag.denote pacing d sch env) t) := by
  induction t with
  | zero => exact Dag.step_spec pacing d sch hv env he hp 0
  | succ t ih =>
    show Dag.step (ctxAt pacing (t + 1)) d sch (envIncAt env (t + 1)) (Dag.run d sch pacing _ t).1 = _
    rw [ih]
    exact Dag.step_spec pacing d sch hv env he hp (t + 1)

end Project

end Hydro
