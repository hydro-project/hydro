import Hydro.Sched

/-!
# Hydro · Project — the dataflow graph (D74)

**What this is.** A Hydro program is written once against the
`HydroSem` signature; its `Values` and `SchedSem` readings are
denotations — functions of complete input histories. A *production*
reading must be stateful (consume a batch, emit a batch) and must share
work the way the program shares wires (`let a := …; H.fold a …;
H.filterMap a …` is one `a`). A tagless-final instance cannot see that
sharing: it receives `a` twice, as a value (Kiselyov's observable-sharing
problem); nor can it reach the feedback cell of a `fix` body it was
handed as a function. hydro_lang itself reifies (`HydroNode::Tee`,
`CycleSource`/`CycleSink`) through `Rc` identity — pointer identity,
which is `unsafe` in Lean. The one agent that sees the `let`s is the
`hydro def` elaborator. So Project is a **typed dataflow graph** the
elaborator compiles from the program (`HydroGenProject.lean`), with

- `Dag.denote` — the graph read back as a `SchedSem` run (this file);
  per module the generated theorem `M.dag_denote : (M.dag …).denote =
  M (SchedSem pacing) …` pins the compilation;
- `Dag.step` — a stateful, incremental interpreter: one cell per node,
  increments in, increments out (`Project/Step.lean`), proven to
  reproduce `denote` step by step;
- the per-location cut with channels (`Project/Wired.lean`): the
  production machine, proven to be a `SchedSem` run under derived
  pacing and cursors.

`HydroSem` is untouched: the graph lives beside the signature, at
`SchedSem`'s vocabulary (in-tick bodies are the machine's list functions,
decisions are the machine's — content `Unit`, transport cursors, timing
scripts).

**Representation.** A `Kind` is a wire's carrier family at the machine
(location included). A `Node Γ κs` is one operator application whose
wire arguments are typed de Bruijn variables into the context `Γ` and
which binds the kinds `κs` (one for an ordinary operator, several for a
tick block, a callee or a knot). A `Dag Γ outs` is the A-normal-form
`let`-chain: `letN n rest` binds `n`'s outputs on top of `Γ`. Knots are
`fixN`: all the mutually recursive wires of one `fix … complete` block
at once, the body exposing its own `let`s as further outputs so the
continuation after `complete` reads them from the *same* body instance
(the elaborator's Bekić cascade re-applies the body; the graph does
not). Its denotation IS that cascade (`bekic`), so `M.dag_denote` stays
an unfolding.
-/

namespace Hydro

namespace Project

universe u v

variable {L : Type} {mem : L → Nat}

/-! ## Kinds: a wire's family at the machine -/

/-- A wire's carrier family at the machine, location included. One
constructor per `HydroSem` carrier family (the eight stream-level ones;
in-tick values are not wires). -/
inductive Kind (L : Type) (mem : L → Nat) : Type 1
  | stream (ℓ : L) (α : Type) (inst : DecidableEq α) (ord : StrOrd) (ret : Retries)
  | keyed (ℓ : L) (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
      (ord : StrOrd) (ret : Retries)
  | sing (ℓ : L) (α σ : Type) (inst : DecidableEq α) (ord : StrOrd) (ret : Retries)
      (b : SingBound σ)
  | ksing (ℓ : L) (α K V : Type) (iα : DecidableEq α) (iK : DecidableEq K)
      (iV : DecidableEq V) (ord : StrOrd) (ret : Retries) (kb : KSBound)
  | atomic (ℓ : L) (σ : Type)
  | atomicStream (ℓ : L) (α : Type) (inst : DecidableEq α) (ord : StrOrd) (ret : Retries)
  | atomicKeyed (ℓ : L) (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
      (ord : StrOrd) (ret : Retries)
  | atomicKSing (ℓ : L) (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)

namespace Kind

/-- The machine carrier of a kind — `(SchedSem L mem pacing).<family> …`
spelled out (definitionally equal to the projections, pacing-free). -/
@[reducible] def sched : Kind L mem → Type
  | .stream ℓ α _ _ _ => Fin (mem ℓ) → StepHist α
  | .keyed ℓ K V _ _ _ _ => Fin (mem ℓ) → StepHist (K × V)
  | .sing ℓ _ σ _ _ _ _ => Fin (mem ℓ) → Nat → σ
  | .ksing ℓ _ K V _ _ _ _ _ _ => Fin (mem ℓ) → Nat → SchedKMap K V
  | .atomic ℓ σ => Fin (mem ℓ) → Nat → Trace σ
  | .atomicStream ℓ α _ _ _ => Fin (mem ℓ) → Nat → Trace (List α)
  | .atomicKeyed ℓ K V _ _ _ _ => Fin (mem ℓ) → Nat → Trace (List (K × V))
  | .atomicKSing ℓ K V _ _ => Fin (mem ℓ) → Nat → Trace (SchedKMap K V)

end Kind

/-- A knot wire's kind: Rust `forward_ref` exists on streams
(`fix_stream`) and on tick wires (`fix_tick`). -/
inductive KnotKind (L : Type) (mem : L → Nat) : Type 1
  | stream (ℓ : L) (α : Type) (inst : DecidableEq α) (ord : StrOrd) (ret : Retries)
  | atomic (ℓ : L) (σ : Type)

def KnotKind.toKind : KnotKind L mem → Kind L mem
  | .stream ℓ α i ord ret => .stream ℓ α i ord ret
  | .atomic ℓ σ => .atomic ℓ σ

/-! ## Contexts, variables, heterogeneous lists -/

abbrev Ctx (L : Type) (mem : L → Nat) : Type 1 := List (Kind L mem)

/-- A typed de Bruijn index. -/
inductive Var : Ctx L mem → Kind L mem → Type 1
  | zero {Γ : Ctx L mem} {κ : Kind L mem} : Var (κ :: Γ) κ
  | succ {Γ : Ctx L mem} {κ κ' : Kind L mem} : Var Γ κ → Var (κ' :: Γ) κ

/-- A heterogeneous list over kinds — a right-nested product computed
from the list (a recursive *type function*, so it lives in the
universe of its elements; an inductive over `Kind` indices would be
forced into `Type 1`). -/
@[reducible] def HList (f : Kind L mem → Type u) : List (Kind L mem) → Type u
  | [] => PUnit
  | κ :: κs => f κ × HList f κs

namespace HList
variable {f : Kind L mem → Type u}

@[reducible] def nil : HList f [] := ⟨⟩
@[reducible] def cons {κ : Kind L mem} {κs : List (Kind L mem)} (x : f κ) (xs : HList f κs) :
    HList f (κ :: κs) := (x, xs)

def head {κ : Kind L mem} {κs : List (Kind L mem)} (xs : HList f (κ :: κs)) : f κ := xs.1
def tail {κ : Kind L mem} {κs : List (Kind L mem)} (xs : HList f (κ :: κs)) : HList f κs := xs.2

def get : {Γ : Ctx L mem} → {κ : Kind L mem} → HList f Γ → Var Γ κ → f κ
  | _ :: _, _, xs, .zero => xs.1
  | _ :: _, _, xs, .succ v => get xs.2 v

def append : {κs Γ : List (Kind L mem)} → HList f κs → HList f Γ → HList f (κs ++ Γ)
  | [], _, _, ys => ys
  | _ :: _, _, xs, ys => (xs.1, append xs.2 ys)

/-- Split at the first `κs`. -/
def take : {κs Γ : List (Kind L mem)} → HList f (κs ++ Γ) → HList f κs
  | [], _, _ => ⟨⟩
  | _ :: _, _, xs => (xs.1, take xs.2)

def drop : {κs Γ : List (Kind L mem)} → HList f (κs ++ Γ) → HList f Γ
  | [], _, xs => xs
  | _ :: _, _, xs => drop xs.2

def map {g : Kind L mem → Type v} (φ : ∀ {κ}, f κ → g κ) :
    {κs : List (Kind L mem)} → HList f κs → HList g κs
  | [], _ => ⟨⟩
  | _ :: _, xs => (φ xs.1, map φ xs.2)

theorem take_append : {κs Γ : List (Kind L mem)} → (xs : HList f κs) → (ys : HList f Γ) →
    HList.take (κs := κs) (xs.append ys) = xs
  | [], _, _, _ => rfl
  | _ :: κs, Γ, xs, ys => by
    show (xs.1, HList.take (κs := κs) (xs.2.append ys)) = xs
    rw [take_append (κs := κs) xs.2 ys]

theorem drop_append : {κs Γ : List (Kind L mem)} → (xs : HList f κs) → (ys : HList f Γ) →
    HList.drop (κs := κs) (xs.append ys) = ys
  | [], _, _, _ => rfl
  | _ :: κs, _, xs, ys => drop_append (κs := κs) xs.2 ys

/-- Build an `HList` from a function on variables. -/
def ofFun : (κs : List (Kind L mem)) → (∀ {κ}, Var κs κ → f κ) → HList f κs
  | [], _ => ⟨⟩
  | _ :: κs, h => (h .zero, ofFun κs (fun v => h (.succ v)))

end HList

/-- The machine environment of a context: a machine wire per variable. -/
abbrev Env (Γ : Ctx L mem) : Type := HList (Kind.sched (mem := mem)) Γ

/-- A substitution: the callee's input variables, in the caller's context. -/
abbrev Subst (Γ' Γ : Ctx L mem) : Type 1 := HList (Var Γ) Γ'

/-! ## Tick shapes as kinds -/

/-- The kinds of a shaped tick tuple at location `ℓ`, left to right. -/
def shapeKinds (ℓ : L) : TickShape → List (Kind L mem)
  | .sing τ => [.atomic ℓ τ]
  | .stream α inst ord ret => [.atomicStream ℓ α inst ord ret]
  | .ksing K V iK iV => [.atomicKSing ℓ K V iK iV]
  | .pair a b => shapeKinds ℓ a ++ shapeKinds ℓ b

/-- The variables feeding a shaped tick input. -/
@[reducible] def VarsOf (Γ : Ctx L mem) (ℓ : L) : TickShape → Type 1
  | .sing τ => Var Γ (.atomic ℓ τ)
  | .stream α inst ord ret => Var Γ (.atomicStream ℓ α inst ord ret)
  | .ksing K V iK iV => Var Γ (.atomicKSing ℓ K V iK iV)
  | .pair a b => VarsOf Γ ℓ a × VarsOf Γ ℓ b

/-- Read a shaped tick input off the environment. -/
def readShape {Γ : Ctx L mem} (env : Env Γ) (ℓ : L) : (sh : TickShape) → VarsOf Γ ℓ sh →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh
  | .sing τ, v => (env.get v : Fin (mem ℓ) → Nat → Trace τ)
  | .stream α _ _ _, v => (env.get v : Fin (mem ℓ) → Nat → Trace (List α))
  | .ksing K V _ _, v => (env.get v : Fin (mem ℓ) → Nat → Trace (SchedKMap K V))
  | .pair a b, (va, vb) => (readShape env ℓ a va, readShape env ℓ b vb)

/-- Flatten a shaped tick tuple into kinds. -/
def shapeToList (ℓ : L) : (sh : TickShape) →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh →
    HList (Kind.sched (mem := mem)) (shapeKinds (mem := mem) ℓ sh)
  | .sing τ, x => HList.cons (x : (Kind.atomic (mem := mem) ℓ τ).sched) HList.nil
  | .stream α i ord ret, x => HList.cons (x : (Kind.atomicStream (mem := mem) ℓ α i ord ret).sched) HList.nil
  | .ksing K V iK iV, x => HList.cons (x : (Kind.atomicKSing (mem := mem) ℓ K V iK iV).sched) HList.nil
  | .pair a b, (xa, xb) => (shapeToList ℓ a xa).append (shapeToList ℓ b xb)

/-! ## Nodes and graphs -/

/-- The machine's in-tick carriers (what a tick body is a function of). -/
abbrev BS := SchedTick.BS
abbrev BStr := SchedTick.BStr
abbrev BKS := SchedTick.BKS

mutual

/-- One operator application, wire arguments as variables, binding the
output kinds `κs`. Data arguments (closures, seeds, decisions, proofs)
are stored verbatim at the machine's types. Constructors mirror
`HydroSem`'s stream-level fields one for one (`Sem.lean`), plus `call`
(a callee module's own graph, inputs substituted) and `fixN` (one
`fix … complete` block: its knot wires `ks` and the body's exposed
`let`s `outs`). -/
inductive Node : Ctx L mem → List (Kind L mem) → Type 1
  | map {Γ} {ℓ : L} {α β : Type} (iα : DecidableEq α) (iβ : DecidableEq β) {ord : StrOrd}
      (x : Var Γ (.stream ℓ α iα ord .exactlyOnce)) (f : Fin (mem ℓ) → α → β) :
      Node Γ [.stream ℓ β iβ ord .exactlyOnce]
  | filterMap {Γ} {ℓ : L} {α β : Type} (iα : DecidableEq α) (iβ : DecidableEq β)
      {ord : StrOrd}
      (x : Var Γ (.stream ℓ α iα ord .exactlyOnce)) (f : Fin (mem ℓ) → α → Option β) :
      Node Γ [.stream ℓ β iβ ord .exactlyOnce]
  | broadcast_closed {Γ} {c p : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
      {ret : Retries} (x : Var Γ (.stream c α iα ord ret)) :
      Node Γ [.keyed p (Fin (mem c)) α inferInstance iα ord ret]
  | demux {Γ} {c p : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
      (x : Var Γ (.stream c (Nat × α) inferInstance ord .exactlyOnce))
      (addr : Fin (mem p) → Nat) :
      Node Γ [.keyed p (Fin (mem c)) α inferInstance iα ord .exactlyOnce]
  | values {Γ} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V) {ord : StrOrd}
      {ret : Retries} (x : Var Γ (.keyed ℓ K V iK iV ord ret)) :
      Node Γ [.stream ℓ V iV .noOrder ret]
  | into_keyed {Γ} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V)
      {ord : StrOrd} (x : Var Γ (.stream ℓ (K × V) inferInstance ord .exactlyOnce)) :
      Node Γ [.keyed ℓ K V iK iV ord .exactlyOnce]
  | kassume_ordering {Γ} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V)
      (x : Var Γ (.keyed ℓ K V iK iV .noOrder .exactlyOnce)) (d : Unit) :
      Node Γ [.keyed ℓ K V iK iV .totalOrder .exactlyOnce]
  | kfold_early_stop {Γ} {ℓ : L} {K V A : Type} (iK : DecidableEq K) (iV : DecidableEq V)
      (iA : DecidableEq A)
      (x : Var Γ (.keyed ℓ K V iK iV .totalOrder .exactlyOnce)) (f : A → V → A × Bool)
      (init : A) :
      Node Γ [.ksing ℓ (K × V) K A inferInstance iK iA .totalOrder .exactlyOnce .boundedValue]
  | kget_max_key {Γ} {ℓ : L} {α K V : Type} (iα : DecidableEq α) (iK : DecidableEq K)
      (iV : DecidableEq V) (lo : LinearOrder K) {ord : StrOrd} {ret : Retries}
      (x : Var Γ (.ksing ℓ α K V iα iK iV ord ret .boundedValue)) :
      Node Γ [.sing ℓ α (Option (K × V)) iα .noOrder ret .unbounded]
  | weaken_retries {Γ} {ℓ : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
      (x : Var Γ (.stream ℓ α iα ord .exactlyOnce)) :
      Node Γ [.stream ℓ α iα ord .atLeastOnce]
  | union {Γ} {ℓ : L} {α : Type} (iα : DecidableEq α) {ret : Retries}
      (a b : Var Γ (.stream ℓ α iα .noOrder ret)) :
      Node Γ [.stream ℓ α iα .noOrder ret]
  | assume_ordering {Γ} {ℓ : L} {α : Type} (iα : DecidableEq α)
      (x : Var Γ (.stream ℓ α iα .noOrder .exactlyOnce)) (d : Unit) :
      Node Γ [.stream ℓ α iα .totalOrder .exactlyOnce]
  | fold {Γ} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd} {ret : Retries}
      (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) (iPre : DecidableRel Pre)
      (ok : FoldOk ord ret Pre g) (x : Var Γ (.stream ℓ α iα ord ret)) :
      Node Γ [.sing ℓ α σ iα ord ret .unbounded]
  | fold_monotone {Γ} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd}
      {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ) (init : σ)
      (ok : FoldOk ord ret (fun _ _ => True) g) (hinfl : ∀ s x, vo.le s (g s x))
      (x : Var Γ (.stream ℓ α iα ord ret)) :
      Node Γ [.sing ℓ α σ iα ord ret (.monotonic vo)]
  | snapshot {Γ} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd} {ret : Retries}
      {b : SingBound σ} (x : Var Γ (.sing ℓ α σ iα ord ret b)) (d : Unit) :
      Node Γ [.atomic ℓ σ]
  | batch {Γ} {ℓ : L} {α : Type} (iα : DecidableEq α)
      (x : Var Γ (.stream ℓ α iα .noOrder .exactlyOnce)) (d : Unit) :
      Node Γ [.atomicStream ℓ α iα .noOrder .exactlyOnce]
  | batch_ordered {Γ} {ℓ : L} {α : Type} (iα : DecidableEq α)
      (x : Var Γ (.stream ℓ α iα .totalOrder .exactlyOnce)) (d : Unit) :
      Node Γ [.atomicStream ℓ α iα .totalOrder .exactlyOnce]
  | sample_every {Γ} {ℓ : L} {α : Type} (iα : DecidableEq α)
      (x : Var Γ (.atomic ℓ (Option α))) :
      Node Γ [.stream ℓ α iα .totalOrder .atLeastOnce]
  | timeout_snapshot {Γ} {ℓ : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
      {ret : Retries} (x : Var Γ (.stream ℓ α iα ord ret)) :
      Node Γ [.atomic ℓ Bool]
  | source_interval_batch {Γ} {ℓ : L} : Node Γ [.atomic ℓ Bool]
  | mapTick {Γ} {ℓ : L} {α β : Type} (x : Var Γ (.atomic ℓ α)) (f : Fin (mem ℓ) → α → β) :
      Node Γ [.atomic ℓ β]
  | zipTick {Γ} {ℓ : L} {α β : Type} (a : Var Γ (.atomic ℓ α)) (b : Var Γ (.atomic ℓ β)) :
      Node Γ [.atomic ℓ (α × β)]
  | defer_tick {Γ} {ℓ : L} {σ : Type} (init : σ) (x : Var Γ (.atomic ℓ σ)) :
      Node Γ [.atomic ℓ σ]
  | allTicks {Γ} {ℓ : L} {β : Type} (iβ : DecidableEq β) {ord : StrOrd}
      (x : Var Γ (.atomicStream ℓ β iβ ord .exactlyOnce)) :
      Node Γ [.stream ℓ β iβ ord .exactlyOnce]
  | flattenOrdered {Γ} {ℓ : L} {β : Type} (iβ : DecidableEq β)
      (x : Var Γ (.atomic ℓ (List β))) :
      Node Γ [.atomicStream ℓ β iβ .totalOrder .exactlyOnce]
  | flattenUnordered {Γ} {ℓ : L} {β : Type} (iβ : DecidableEq β)
      (x : Var Γ (.atomic ℓ (List β))) :
      Node Γ [.atomicStream ℓ β iβ .noOrder .exactlyOnce]
  | tinto_keyed {Γ} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V)
      {ord : StrOrd}
      (x : Var Γ (.atomicStream ℓ (K × V) inferInstance ord .exactlyOnce)) :
      Node Γ [.atomicKeyed ℓ K V iK iV ord .exactlyOnce]
  | tassume_ordering {Γ} {ℓ : L} {α : Type} (iα : DecidableEq α)
      (x : Var Γ (.atomicStream ℓ α iα .noOrder .exactlyOnce)) (d : Unit) :
      Node Γ [.atomicStream ℓ α iα .totalOrder .exactlyOnce]
  | reduce_watermark {Γ} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iH : Hashable K) (iV : DecidableEq V)
      (lo : LinearOrder K) {ord : StrOrd}
      (x : Var Γ (.atomicKeyed ℓ K V iK iV ord .exactlyOnce))
      (wm : Var Γ (.atomic ℓ (Option K))) (f : V → V → V) (Pre : V → V → Prop)
      (iPre : DecidableRel Pre) (ok : FoldOk ord .exactlyOnce Pre (reduceStep f)) :
      Node Γ [.atomicKSing ℓ K V iK iV]
  | tick_scan {Γ} {ℓ : L} (sts ins outs : TickShape) (x : VarsOf Γ ℓ ins)
      (g : Fin (mem ℓ) → BoundedOf BS BStr BKS sts → BoundedOf BS BStr BKS ins →
        BoundedOf BS BStr BKS sts × BoundedOf BS BStr BKS outs)
      (seed : SeedOf sts) :
      Node Γ (shapeKinds ℓ outs)
  /-- A callee module's graph, its inputs substituted. -/
  | call {Γ} {Γ' : Ctx L mem} {outs : List (Kind L mem)} (N : Dag Γ' outs) (args : Subst Γ' Γ) :
      Node Γ outs
  /-- One `fix … complete` block: the knot wires `ks` (every `forward_ref`
  of the block) and the body's exposed `let`s `outs`; the body sees the
  knot wires at indices `0 … |ks|-1` and returns their next values first,
  then the exposed lets. -/
  | fixN {Γ} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
      (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs)) :
      Node Γ (ks.map KnotKind.toKind ++ outs)

/-- The A-normal-form `let`-chain. -/
inductive Dag : Ctx L mem → List (Kind L mem) → Type 1
  | ret {Γ} {outs : List (Kind L mem)} (xs : HList (Var Γ) outs) : Dag Γ outs
  | letN {Γ} {κs outs : List (Kind L mem)} (n : Node Γ κs) (rest : Dag (κs ++ Γ) outs) :
      Dag Γ outs

end

/-! ## The denotation: the graph read back as a `SchedSem` run -/

/-! ## Schedules: the delivery cursors of a run

The network's behaviour on a graph: per network node (`broadcast_closed`,
`demux`), per receiver, per sender, the cursor at each step — how many of the
sender's items the receiver has read by then (`TransportDec` at `SchedSem`).
Execution data, not program data: the graph is compiled once (`M.dag`) and the
schedule is supplied to its denotation (`Dag.denote d sch`) and its machine
(`Dag.run d sch`); the generated `M.schedOf` packs a module's `…Sched` bundle
into this shape and `M.recordOf` unpacks it (an isomorphism, `M.schedOf_recordOf`). -/

mutual

/-- A node's schedule: the cursors of a network node, the timing verdicts of a
timer node (`sample_every`/`timeout_snapshot`/`source_interval_batch` — per member,
per tick), the body's for a callee or a knot, nothing otherwise. -/
def Node.Sched : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (p := p) _ _ => Fin (mem p) → Fin (mem c) → Nat → Nat
  | _, _, .demux (c := c) (p := p) _ _ _ => Fin (mem p) → Fin (mem c) → Nat → Nat
  | _, _, .sample_every (ℓ := ℓ) _ _ => SampleTimes (mem ℓ)
  | _, _, .timeout_snapshot (ℓ := ℓ) _ _ => TimerVerdicts (mem ℓ)
  | _, _, .source_interval_batch (ℓ := ℓ) => TimingPulses (mem ℓ)
  | _, _, .call N _ => Dag.Sched N
  | _, _, .fixN _ body => Dag.Sched body
  | _, _, _ => Unit

/-- A graph's schedule: one component per node of the chain. -/
def Dag.Sched : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.Sched n × Dag.Sched rest

end

section Denote
variable (pacing : Pacing L mem)

/-- Close one knot wire of a kind at the machine. -/
def knotFix : (k : KnotKind L mem) → (k.toKind.sched → k.toKind.sched) → k.toKind.sched
  | .stream ℓ α i ord ret, f => @HydroSem.fix_stream L mem (SchedSem L mem pacing) ℓ α i ord ret () f
  | .atomic ℓ σ, f => @HydroSem.fix_tick L mem (SchedSem L mem pacing) ℓ σ () f

/-- **The Bekić cascade** (the shape the `fix` elaborator emits,
`HydroDef.closeWire`): the first wire is solved innermost — for every
value of the later wires — and each later wire's knot closes over the
earlier wires' closures. `B` is the body's knot outputs as a function of
the knot inputs. -/
def bekic : (ks : List (KnotKind L mem)) →
    (HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)) →
    HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)
  | [], _ => HList.nil
  | k :: ks, B =>
    let inner : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) → k.toKind.sched :=
      fun xs => knotFix pacing k (fun l => (B (HList.cons l xs)).head)
    let rest := bekic ks (fun xs => (B (HList.cons (inner xs) xs)).tail)
    HList.cons (inner rest) rest

local notation "S" => SchedSem L mem pacing

mutual

/-- A node's outputs at the machine, under the schedule `sch`. -/
def Node.denote : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Sched n → Env Γ →
    HList (Kind.sched (mem := mem)) κs
  | _, _, .map (ℓ := ℓ) (α := α) (β := β) iα iβ (ord := ord) x f, _, env =>
    HList.cons (@HydroSem.map L mem S ℓ α β iα iβ ord (env.get x) f) HList.nil
  | _, _, .filterMap (ℓ := ℓ) (α := α) (β := β) iα iβ (ord := ord) x f, _, env =>
    HList.cons (@HydroSem.filterMap L mem S ℓ α β iα iβ ord (env.get x) f) HList.nil
  | _, _, .broadcast_closed (c := c) (p := p) (α := α) iα (ord := ord) (ret := ret) x, d, env =>
    HList.cons (@HydroSem.broadcast_closed L mem S c p α iα ord ret d (env.get x)) HList.nil
  | _, _, .demux (c := c) (p := p) (α := α) iα (ord := ord) x addr, d, env =>
    HList.cons (@HydroSem.demux L mem S c p α iα ord d (env.get x) addr) HList.nil
  | _, _, .values (ℓ := ℓ) (K := K) (V := V) iK iV (ord := ord) (ret := ret) x, _, env =>
    HList.cons (@HydroSem.values L mem S ℓ K V iK iV ord ret (env.get x)) HList.nil
  | _, _, .into_keyed (ℓ := ℓ) (K := K) (V := V) iK iV (ord := ord) x, _, env =>
    HList.cons (@HydroSem.into_keyed L mem S ℓ K V iK iV ord (env.get x)) HList.nil
  | _, _, .kassume_ordering (ℓ := ℓ) (K := K) (V := V) iK iV x d, _, env =>
    HList.cons (@HydroSem.kassume_ordering L mem S ℓ K V iK iV (env.get x) d) HList.nil
  | _, _, .kfold_early_stop (ℓ := ℓ) (K := K) (V := V) (A := A) iK iV iA x f init, _, env =>
    HList.cons (@HydroSem.kfold_early_stop L mem S ℓ K V A iK iV iA (env.get x) f init) HList.nil
  | _, _, .kget_max_key (ℓ := ℓ) (α := α) (K := K) (V := V) iα iK iV lo (ord := ord) (ret := ret) x, _, env =>
    HList.cons (@HydroSem.kget_max_key L mem S ℓ α K V iα iK iV lo ord ret (env.get x)) HList.nil
  | _, _, .weaken_retries (ℓ := ℓ) (α := α) iα (ord := ord) x, _, env =>
    HList.cons (@HydroSem.weaken_retries L mem S ℓ α iα ord (env.get x)) HList.nil
  | _, _, .union (ℓ := ℓ) (α := α) iα (ret := ret) a b, _, env =>
    HList.cons (@HydroSem.union L mem S ℓ α iα ret (env.get a) (env.get b)) HList.nil
  | _, _, .assume_ordering (ℓ := ℓ) (α := α) iα x d, _, env =>
    HList.cons (@HydroSem.assume_ordering L mem S ℓ α iα (env.get x) d) HList.nil
  | _, _, .fold (ℓ := ℓ) (α := α) (σ := σ) iα (ord := ord) (ret := ret) g init Pre iPre ok x, _, env =>
    HList.cons (@HydroSem.fold L mem S ℓ α σ iα ord ret g init Pre iPre ok (env.get x)) HList.nil
  | _, _, .fold_monotone (ℓ := ℓ) (α := α) (σ := σ) iα (ord := ord) (ret := ret) vo g init ok hinfl x, _, env =>
    HList.cons (@HydroSem.fold_monotone L mem S ℓ α σ iα ord ret vo g init ok hinfl (env.get x)) HList.nil
  | _, _, .snapshot (ℓ := ℓ) (α := α) (σ := σ) iα (ord := ord) (ret := ret) (b := b) x d, _, env =>
    HList.cons (@HydroSem.snapshot L mem S ℓ α σ iα ord ret b (env.get x) d) HList.nil
  | _, _, .batch (ℓ := ℓ) (α := α) iα x d, _, env =>
    HList.cons (@HydroSem.batch L mem S ℓ α iα (env.get x) d) HList.nil
  | _, _, .batch_ordered (ℓ := ℓ) (α := α) iα x d, _, env =>
    HList.cons (@HydroSem.batch_ordered L mem S ℓ α iα (env.get x) d) HList.nil
  | _, _, .sample_every (ℓ := ℓ) (α := α) iα x, d, env =>
    HList.cons (@HydroSem.sample_every L mem S ℓ α iα (env.get x) d) HList.nil
  | _, _, .timeout_snapshot (ℓ := ℓ) (α := α) iα (ord := ord) (ret := ret) x, d, env =>
    HList.cons (@HydroSem.timeout_snapshot L mem S ℓ α iα ord ret (env.get x) d) HList.nil
  | _, _, .source_interval_batch (ℓ := ℓ), d, _ =>
    HList.cons (@HydroSem.source_interval_batch L mem S ℓ d) HList.nil
  | _, _, .mapTick (ℓ := ℓ) (α := α) (β := β) x f, _, env =>
    HList.cons (@HydroSem.mapTick L mem S ℓ α β (env.get x) f) HList.nil
  | _, _, .zipTick (ℓ := ℓ) (α := α) (β := β) a b, _, env =>
    HList.cons (@HydroSem.zipTick L mem S ℓ α β (env.get a) (env.get b)) HList.nil
  | _, _, .defer_tick (ℓ := ℓ) (σ := σ) init x, _, env =>
    HList.cons (@HydroSem.defer_tick L mem S ℓ σ init (env.get x)) HList.nil
  | _, _, .allTicks (ℓ := ℓ) (β := β) iβ (ord := ord) x, _, env =>
    HList.cons (@HydroSem.allTicks L mem S ℓ β iβ ord (env.get x)) HList.nil
  | _, _, .flattenOrdered (ℓ := ℓ) (β := β) iβ x, _, env =>
    HList.cons (@HydroSem.flattenOrdered L mem S ℓ β iβ (env.get x)) HList.nil
  | _, _, .flattenUnordered (ℓ := ℓ) (β := β) iβ x, _, env =>
    HList.cons (@HydroSem.flattenUnordered L mem S ℓ β iβ (env.get x)) HList.nil
  | _, _, .tinto_keyed (ℓ := ℓ) (K := K) (V := V) iK iV (ord := ord) x, _, env =>
    HList.cons (@HydroSem.tinto_keyed L mem S ℓ K V iK iV ord (env.get x)) HList.nil
  | _, _, .tassume_ordering (ℓ := ℓ) (α := α) iα x d, _, env =>
    HList.cons (@HydroSem.tassume_ordering L mem S ℓ α iα (env.get x) d) HList.nil
  | _, _, .reduce_watermark (ℓ := ℓ) (K := K) (V := V) iK iH iV lo (ord := ord) x wm f Pre iPre ok, _, env =>
    HList.cons (@HydroSem.reduce_watermark L mem S ℓ K V iK iH iV lo ord (env.get x) (env.get wm) f Pre iPre ok) HList.nil
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, _, env =>
    shapeToList ℓ outs
      (@HydroSem.tick_scan L mem S ℓ sts ins outs (readShape env ℓ ins x) g seed)
  | _, _, .call N args, sch, env => Dag.denote N sch (args.map (fun v => env.get v))
  | _, _, .fixN ks body, sch, env =>
    let closed := bekic pacing ks (fun xs => (Dag.denote body sch (xs.append env)).take)
    closed.append (Dag.denote body sch (closed.append env)).drop

/-- A graph's outputs at the machine, under the schedule `sch`. -/
def Dag.denote : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Sched d → Env Γ →
    HList Kind.sched outs
  | _, _, .ret xs, _, env => xs.map (fun v => env.get v)
  | _, _, .letN n rest, sch, env => Dag.denote rest sch.2 ((Node.denote n sch.1 env).append env)

end

end Denote

end Project

end Hydro
