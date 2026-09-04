import Hydro.Project.IR

/-!
# Hydro · Project — the stateful interpreter (lock-step)

Every `SchedSem` operator (`Sched.lean`) is a fold over steps or over
ticks; none needs the history. This file is the **incremental** reading
of the graph: one cell of state per node, and per global step each node
consumes its inputs' increments and emits its own.

- `Kind.delta`: a wire's increment type — a stream's new items, an
  atomic trace's **single slot** (this step's tick entry, or nothing:
  D85, justified by `TPaced`/`Dag.denote_tpaced`), a singleton's current
  value (what a tick reads), a keyed singleton's current map;
- `Node.State`/`Dag.State`: the cells (a fold's accumulator, a batch's
  pending buffer, a channel's undelivered queue + delivered count, a
  tick block's registers + unmatched input suffixes, a knot's feedback
  increment + its body's cells, a callee's cells);
- `Node.step`/`Dag.step`: one global step, in `let` order, every member
  of every location (`StepCtx.tick` says who ticks; a network node reads
  its cursors at `StepCtx.t`);
- `Dag.run`: the lock-step machine from the initial cells.

`Project/StepSpec.lean` proves this reproduces `Dag.denote` step by step
(`Dag.step_spec`); `Project/Wired.lean` cuts it per location.

Within a step the knot feedback is one step stale (`fixN` releases the
pending body output it stored — `SchedSem`'s gated `shift`), and a
network node delivers before it enqueues this step's sends (the `+1`
delivery floor, `StepHist.deliver`). **A member that is not active
(`StepCtx.act`) is inert**: channels only enqueue at it, knots hold their
pending feedback — so a sleeping process's cells never move
(`Project/WiredRun.lean`); an active member that does not tick
(`StepCtx.tick`, a stutter) receives and propagates knot feedback but
runs no tick body.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-! ## Increments -/

namespace Kind

/-- The per-step increment of a wire of this kind, per member. -/
@[reducible] def delta : Kind L mem → Type
  | .stream ℓ α _ _ _ => Fin (mem ℓ) → List α
  | .keyed ℓ K V _ _ _ _ => Fin (mem ℓ) → List (K × V)
  | .sing ℓ _ σ _ _ _ _ => Fin (mem ℓ) → σ
  | .ksing ℓ _ K V _ _ _ _ _ _ => Fin (mem ℓ) → SchedKMap K V
  | .atomic ℓ σ => Fin (mem ℓ) → Option σ
  | .atomicStream ℓ α _ _ _ => Fin (mem ℓ) → Option (List α)
  | .atomicKeyed ℓ K V _ _ _ _ => Fin (mem ℓ) → Option (List (K × V))
  | .atomicKSing ℓ K V _ _ => Fin (mem ℓ) → Option (SchedKMap K V)

end Kind

/-- The increment of a step history at step `t`: its whole view at `0`,
`view (t+1) − view t` afterwards. -/
def incView {α : Type} (h : StepHist α) : Nat → List α
  | 0 => h.view 0
  | t + 1 => h.inc t

/-- The increment at step `t` of a machine wire. -/
def Kind.incAt : (κ : Kind L mem) → κ.sched → Nat → κ.delta
  | .stream _ _ _ _ _, w, t => fun i => incView (w i) t
  | .keyed _ _ _ _ _ _ _, w, t => fun i => incView (w i) t
  | .sing _ _ _ _ _ _ _, w, t => fun i => w i t
  | .ksing _ _ _ _ _ _ _ _ _ _, w, t => fun i => w i t
  | .atomic _ _, w, t => fun i => (incTrace (w i) t).head?
  | .atomicStream _ _ _ _ _, w, t => fun i => (incTrace (w i) t).head?
  | .atomicKeyed _ _ _ _ _ _ _, w, t => fun i => (incTrace (w i) t).head?
  | .atomicKSing _ _ _ _ _, w, t => fun i => (incTrace (w i) t).head?

/-- The increments of an environment at step `t`. -/
def envIncAt {Γ : Ctx L mem} (env : Env Γ) (t : Nat) : HList (Kind.delta (mem := mem)) Γ :=
  env.map (fun {κ} w => Kind.incAt κ w t)

/-! ## Cells -/

/-- One channel pair's cell: what the sender has produced and the
channel has not yet handed over, and how much it has handed over. -/
structure ChanCell (α : Type) where
  pending : List α
  delivered : Nat

def ChanCell.init {α : Type} : ChanCell α := ⟨[], 0⟩

/-- Deliver per the cursor's value `c` at this step (`cumMax`), then
enqueue this step's sends. -/
def ChanCell.step {α : Type} (c : Nat) (sends : List α) (cell : ChanCell α) :
    ChanCell α × List α :=
  let out := cell.pending.take (c - cell.delivered)
  (⟨cell.pending.drop out.length ++ sends, cell.delivered + out.length⟩, out)

/-- A receiver's queue from a sender, extended by the sender's items
(enqueue without delivering: the receiver does not tick). -/
def ChanCell.enqueue {α : Type} (cell : ChanCell α) (sends : List α) : ChanCell α :=
  ⟨cell.pending ++ sends, cell.delivered⟩

/-- A channel cell's step at a receiver: deliver per the cursor if the
receiver ticks (it drains its inbox when it runs), otherwise only enqueue. -/
def ChanCell.stepG {α : Type} (tick : Bool) (c : Nat) (sends : List α) (cell : ChanCell α) :
    ChanCell α × List α :=
  if tick then cell.step c sends else (cell.enqueue sends, [])

/-- A key's early-stopping fold state: running (`inl`) or stopped
(`inr`); keys unique — the invariant the per-tick emission reads (D75: the
emission is one linear pass, not a re-`dedup` of every key ever seen). -/
abbrev KFESCell (K A : Type) := {l : List (K × (A ⊕ A)) // (l.map Prod.fst).Nodup}

/-- One arrival into the early-stopping keyed fold: the key's entry is
re-appended last (the machine's association list orders keys by their
LAST arrival — `SchedKMap.ofKeyed`'s `dedup`), running or stopped. -/
def kfesStep {V A : Type} (f : A → V → A × Bool) : A ⊕ A → V → A ⊕ A
  | .inl a, v => let r := f a v; if r.2 then .inr r.1 else .inl r.1
  | .inr a, _ => .inr a

def kfesInsertList {K V A : Type} [DecidableEq K] (f : A → V → A × Bool) (init : A)
    (st : List (K × (A ⊕ A))) (kv : K × V) : List (K × (A ⊕ A)) :=
  let cur := match st.find? (fun e => decide (e.1 = kv.1)) with
    | none => .inl init
    | some e => e.2
  st.filter (fun e => decide (e.1 ≠ kv.1)) ++ [(kv.1, kfesStep f cur kv.2)]

theorem kfesInsertList_nodup {K V A : Type} [DecidableEq K] (f : A → V → A × Bool) (init : A)
    (st : List (K × (A ⊕ A))) (kv : K × V) (h : (st.map Prod.fst).Nodup) :
    ((kfesInsertList f init st kv).map Prod.fst).Nodup := by
  unfold kfesInsertList
  simp only [List.map_append, List.map_cons, List.map_nil]
  rw [List.nodup_append]
  refine ⟨?_, List.nodup_singleton _, ?_⟩
  · have : (st.filter (fun e => decide (e.1 ≠ kv.1))).map Prod.fst
        = (st.map Prod.fst).filter (fun k => decide (k ≠ kv.1)) := by
      rw [List.filter_map]; rfl
    rw [this]
    exact h.filter _
  · intro a ha b hb
    rw [List.mem_singleton] at hb
    subst hb
    rw [List.mem_map] at ha
    obtain ⟨e, he, hke⟩ := ha
    have := (List.mem_filter.mp he).2
    simp only [ne_eq, decide_eq_true_eq] at this
    exact hke ▸ this

def kfesInsert {K V A : Type} [DecidableEq K] (f : A → V → A × Bool) (init : A)
    (st : KFESCell K A) (kv : K × V) : KFESCell K A :=
  ⟨kfesInsertList f init st.1 kv, kfesInsertList_nodup f init st.1 kv st.2⟩

/-- The completed entries. -/
def kfesOut {K A : Type} (st : List (K × (A ⊕ A))) : List (K × A) :=
  st.filterMap fun e => match e.2 with | .inr a => some (e.1, a) | .inl _ => none

theorem kfesOut_keys_sublist {K A : Type} : ∀ (st : List (K × (A ⊕ A))),
    ((kfesOut st).map Prod.fst).Sublist (st.map Prod.fst)
  | [] => List.Sublist.refl _
  | e :: st => by
    unfold kfesOut
    rw [List.filterMap_cons, List.map_cons]
    cases e.2 with
    | inl _ => exact (kfesOut_keys_sublist st).cons _
    | inr a => simp only [List.map_cons]; exact (kfesOut_keys_sublist st).cons₂ _

/-- The completed entries as the machine's keyed map (keys unique by the cell's invariant). -/
def kfesOutMap {K A : Type} [DecidableEq K] (st : KFESCell K A) : SchedKMap K A :=
  ⟨kfesOut st.1, (kfesOut_keys_sublist st.1).nodup st.2⟩

/-- Pending (unmatched) input suffixes of a shaped tick input. -/
@[reducible] def PendingOf : TickShape → Type
  | .sing τ => List τ
  | .stream α _ _ _ => List (List α)
  | .ksing K V _ _ => List (SchedKMap K V)
  | .kstream K V _ _ _ _ => List (List (K × V))
  | .pair a b => PendingOf a × PendingOf b

def PendingOf.empty : (sh : TickShape) → PendingOf sh
  | .sing _ => []
  | .stream _ _ _ _ => []
  | .ksing _ _ _ _ => []
  | .kstream _ _ _ _ _ _ => []
  | .pair a b => (empty a, empty b)

/-- Append this step's leaf increments to the pending suffixes. -/
def PendingOf.push {ℓ : L} : (sh : TickShape) → PendingOf sh →
    (Fin (mem ℓ) → HList (Kind.delta (mem := mem)) (shapeKinds ℓ sh)) → Fin (mem ℓ) → PendingOf sh
  | .sing _, p, Δ, i => p ++ ((Δ i).head i).toList
  | .stream _ _ _ _, p, Δ, i => p ++ ((Δ i).head i).toList
  | .ksing _ _ _ _, p, Δ, i => p ++ ((Δ i).head i).toList
  | .kstream _ _ _ _ _ _, p, Δ, i => p ++ ((Δ i).head i).toList
  | .pair a b, (pa, pb), Δ, i =>
    (push a pa (fun i => (Δ i).take) i, push b pb (fun i => (Δ i).drop) i)

/-- The number of complete tuples available. -/
def PendingOf.avail : (sh : TickShape) → PendingOf sh → Nat
  | .sing _, p => p.length
  | .stream _ _ _ _, p => p.length
  | .ksing _ _ _ _, p => p.length
  | .kstream _ _ _ _ _ _, p => p.length
  | .pair a b, (pa, pb) => min (avail a pa) (avail b pb)

/-- The first `n` tuples, zipped. -/
def PendingOf.takeZip : (sh : TickShape) → PendingOf sh → Nat → List (BoundedOf BS BStr BKS BKStr sh)
  | .sing _, p, n => p.take n
  | .stream _ _ _ _, p, n => p.take n
  | .ksing _ _ _ _, p, n => p.take n
  | .kstream _ _ _ _ _ _, p, n => p.take n
  | .pair a b, (pa, pb), n => List.zip (takeZip a pa n) (takeZip b pb n)

def PendingOf.dropN : (sh : TickShape) → PendingOf sh → Nat → PendingOf sh
  | .sing _, p, n => p.drop n
  | .stream _ _ _ _, p, n => p.drop n
  | .ksing _ _ _ _, p, n => p.drop n
  | .kstream _ _ _ _ _ _, p, n => p.drop n
  | .pair a b, (pa, pb), n => (dropN a pa n, dropN b pb n)

/-- Unzip per-member shaped outputs (this step's tick, if any) into
per-leaf single-slot increments. -/
def unzipShape (ℓ : L) : (sh : TickShape) → (Fin (mem ℓ) → Option (BoundedOf BS BStr BKS BKStr sh)) →
    HList (Kind.delta (mem := mem)) (shapeKinds ℓ sh)
  | .sing _, l => HList.cons (fun i => l i) HList.nil
  | .stream _ _ _ _, l => HList.cons (fun i => l i) HList.nil
  | .ksing _ _ _ _, l => HList.cons (fun i => l i) HList.nil
  | .kstream _ _ _ _ _ _, l => HList.cons (fun i => l i) HList.nil
  | .pair a b, l =>
    (unzipShape ℓ a (fun i => (l i).map Prod.fst)).append (unzipShape ℓ b (fun i => (l i).map Prod.snd))

/-- Run a scan over a list of inputs, returning the final state and the outputs. -/
def scanRun {ι σ β : Type} (g : σ → ι → σ × β) : σ → List ι → σ × List β
  | s, [] => (s, [])
  | s, x :: xs =>
    let (s', y) := g s x
    let (s'', ys) := scanRun g s' xs
    (s'', y :: ys)

/-! ## The step context -/

/-- What a step knows: the global step (read by network nodes for their
cursors) and who ticks. -/
structure StepCtx (L : Type) (mem : L → Nat) where
  t : Nat
  /-- Whose tick bodies run this step. -/
  tick : (ℓ : L) → Fin (mem ℓ) → Bool
  /-- Who is active this step (ticking or stuttering): deliveries land, knot
  feedback propagates. A member that is not active is inert. -/
  act : (ℓ : L) → Fin (mem ℓ) → Bool

/-! ## Cells and steps, per node -/

/-- The pending feedback of a knot block: one per knot wire — the body's
knot output not yet released into the knot; initially empty (`shift`'s
step `0`). -/
def KnotKind.deltaInit : (k : KnotKind L mem) → k.toKind.delta
  | .stream ℓ _ _ _ _ => fun (_ : Fin (mem ℓ)) => []
  | .atomic ℓ _ => fun (_ : Fin (mem ℓ)) => none

def knotDeltaInit : (ks : List (KnotKind L mem)) →
    HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)
  | [] => HList.nil
  | k :: ks => HList.cons k.deltaInit (knotDeltaInit ks)

/-- What a knot wire releases this step: the pending feedback at the
members that tick, nothing at the others (the gated `shift`). -/
def KnotKind.feed (ctx : StepCtx L mem) : (k : KnotKind L mem) → k.toKind.delta → k.toKind.delta
  | .stream ℓ _ _ _ _, d => fun i => if ctx.act ℓ i then d i else []
  | .atomic ℓ _, d => fun i => if ctx.act ℓ i then d i else none

/-- The pending feedback after the step: what a non-ticking member still
holds, then the body's new knot output. An atomic knot's slot is one
entry: an inactive member's body emits nothing (`TPaced`), so the held
slot is never overwritten (`Option.or` keeps it — the spec proof needs
exactly that, `Dag.denote_tpaced`). -/
def KnotKind.keep (ctx : StepCtx L mem) : (k : KnotKind L mem) → k.toKind.delta → k.toKind.delta →
    k.toKind.delta
  | .stream ℓ _ _ _ _, d, inc => fun i => (if ctx.act ℓ i then [] else d i) ++ inc i
  | .atomic ℓ _, d, inc => fun i => if ctx.act ℓ i then inc i else (d i).or (inc i)

def knotFeed (ctx : StepCtx L mem) : (ks : List (KnotKind L mem)) →
    HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind) →
    HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)
  | [], _ => HList.nil
  | k :: ks, d => HList.cons (k.feed ctx d.1) (knotFeed ctx ks d.2)

def knotKeep (ctx : StepCtx L mem) : (ks : List (KnotKind L mem)) →
    HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind) →
    HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind) →
    HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)
  | [], _, _ => HList.nil
  | k :: ks, d, inc => HList.cons (k.keep ctx d.1 inc.1) (knotKeep ctx ks d.2 inc.2)

mutual

/-- The cells of a node. -/
def Node.State : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (p := p) (α := α) _ _ => Fin (mem p) → Fin (mem c) → ChanCell α
  | _, _, .demux (c := c) (p := p) (α := α) _ _ _ => Fin (mem p) → Fin (mem c) → ChanCell α
  | _, _, .kfold_early_stop (ℓ := ℓ) (K := K) (A := A) _ _ _ _ _ _ => Fin (mem ℓ) → KFESCell K A
  | _, _, .fold (ℓ := ℓ) (σ := σ) _ _ _ _ _ _ _ => Fin (mem ℓ) → σ
  | _, _, .fold_monotone (ℓ := ℓ) (σ := σ) _ _ _ _ _ _ _ => Fin (mem ℓ) → σ
  | _, _, .batch (ℓ := ℓ) (α := α) _ _ _ _ _ => Fin (mem ℓ) → List α
  | _, _, .source_interval (ℓ := ℓ) _ _ => Fin (mem ℓ) → Nat
  | _, _, .zipTick (ℓ := ℓ) (α := α) (β := β) _ _ => Fin (mem ℓ) → List α × List β
  | _, _, .defer_tick (ℓ := ℓ) (σ := σ) _ _ => Fin (mem ℓ) → List σ
  | _, _, .reduce_watermark (ℓ := ℓ) (K := K) (V := V) _ _ _ _ _ _ _ _ _ _ =>
    Fin (mem ℓ) → (List (List (K × V)) × List (Option K)) × (SchedKMap K V × Option K)
  | _, _, .kfold (ℓ := ℓ) (K := K) (A := A) _ _ _ _ _ _ _ _ _ _ => Fin (mem ℓ) → SchedKMap K A
  | _, _, .tfold (ℓ := ℓ) (σ := σ) _ _ _ _ _ _ _ => Fin (mem ℓ) → σ
  | _, _, .latest (ℓ := ℓ) (σ := σ) _ _ => Fin (mem ℓ) → Option σ
  | _, _, .latest_monotone (ℓ := ℓ) _ => Fin (mem ℓ) → Option Nat
  | _, _, .time (ℓ := ℓ) => Fin (mem ℓ) → Nat
  | _, _, .snapshot_bounded (ℓ := ℓ) _ _ => Fin (mem ℓ) → Nat
  | _, _, .tick_scan (ℓ := ℓ) sts ins _ _ _ _ => Fin (mem ℓ) → PendingOf ins × BoundedOf BS BStr BKS BKStr sts
  | _, _, .call N _ => Dag.State N
  | _, _, .fixN ks body => Dag.State body × HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)
  | _, _, _ => Unit

/-- The cells of a graph. -/
def Dag.State : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.State n × Dag.State rest

end

mutual

/-- The initial cells. -/
def Node.init : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.State n
  | _, _, .broadcast_closed _ _ => fun _ _ => ChanCell.init
  | _, _, .demux _ _ _ => fun _ _ => ChanCell.init
  | _, _, .kfold_early_stop _ _ _ _ _ _ => fun _ => ⟨[], List.nodup_nil⟩
  | _, _, .fold _ _ init _ _ _ _ => fun _ => init
  | _, _, .fold_monotone _ _ _ init _ _ _ => fun _ => init
  | _, _, .batch _ _ _ _ _ => fun _ => []
  | _, _, .source_interval _ _ => fun _ => 0
  | _, _, .zipTick _ _ => fun _ => ([], [])
  | _, _, .defer_tick init _ => fun _ => [init]
  | _, _, .reduce_watermark _ _ _ _ _ _ _ _ _ _ => fun _ => (([], []), (SchedKMap.empty, none))
  | _, _, .kfold _ _ _ _ _ _ _ _ _ _ => fun _ => SchedKMap.empty
  | _, _, .tfold _ _ init _ _ _ _ => fun _ => init
  | _, _, .latest _ _ => fun _ => none
  | _, _, .latest_monotone _ => fun _ => none
  | _, _, .time => fun _ => 0
  | _, _, .snapshot_bounded _ _ => fun _ => 0
  | _, _, .kentries .. => ()
  | _, _, .kmap .. => ()
  | _, _, .kallTicks .. => ()
  | _, _, .ksnapshot .. => ()
  | _, _, .tick_scan sts ins _ _ _ seed => fun _ => (PendingOf.empty ins, SchedTick.seed sts seed)
  | _, _, .call N _ => Dag.init N
  | _, _, .fixN ks body => (Dag.init body, knotDeltaInit ks)
  | _, _, .map .. => ()
  | _, _, .filterMap .. => ()
  | _, _, .values .. => ()
  | _, _, .into_keyed .. => ()
  | _, _, .kassume_ordering .. => ()
  | _, _, .kget_max_key .. => ()
  | _, _, .weaken_retries .. => ()
  | _, _, .union .. => ()
  | _, _, .assume_ordering .. => ()
  | _, _, .snapshot .. => ()
  | _, _, .mapTick .. => ()
  | _, _, .firstTick .. => ()
  | _, _, .allTicks .. => ()
  | _, _, .flattenOrdered .. => ()
  | _, _, .flattenUnordered .. => ()
  | _, _, .tinto_keyed .. => ()
  | _, _, .tassume_ordering .. => ()

def Dag.init : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.State d
  | _, _, .ret _ => ()
  | _, _, .letN n rest => (Node.init n, Dag.init rest)

end

/-- The increments feeding a shaped tick input. -/
def readShapeDelta {Γ : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ) (ℓ : L) :
    (sh : TickShape) → VarsOf Γ ℓ sh → HList (Kind.delta (mem := mem)) (shapeKinds ℓ sh)
  | .sing _, v => HList.cons (Δ.get v) HList.nil
  | .stream _ _ _ _, v => HList.cons (Δ.get v) HList.nil
  | .ksing _ _ _ _, v => HList.cons (Δ.get v) HList.nil
  | .kstream _ _ _ _ _ _, v => HList.cons (Δ.get v) HList.nil
  | .pair a b, (va, vb) => (readShapeDelta Δ ℓ a va).append (readShapeDelta Δ ℓ b vb)

mutual

/-- One global step of a node: consume this step's input increments,
update the cells, emit this step's output increments. -/
def Node.step (ctx : StepCtx L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → Node.Sched n → HList (Kind.delta (mem := mem)) Γ → Node.State n →
    Node.State n × HList (Kind.delta (mem := mem)) κs
  | _, _, .map _ _ x f, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map (f i)) HList.nil)
  | _, _, .filterMap _ _ x f, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).filterMap (f i)) HList.nil)
  | _, _, .broadcast_closed (c := c) (p := p) _ x, d, Δ, s =>
    let cells := fun i j => (s i j).stepG (ctx.act p i) (cumMax (d i j) ctx.t) (Δ.get x j)
    (fun i j => (cells i j).1,
     HList.cons (fun i => (List.finRange (mem c)).flatMap (fun j => ((cells i j).2).map (fun v => (j, v)))) HList.nil)
  | _, _, .demux (c := c) (p := p) _ x addr, d, Δ, s =>
    let cells := fun i j => (s i j).stepG (ctx.act p i) (cumMax (d i j) ctx.t)
      ((Δ.get x j).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none))
    (fun i j => (cells i j).1,
     HList.cons (fun i => (List.finRange (mem c)).flatMap (fun j => ((cells i j).2).map (fun v => (j, v)))) HList.nil)
  | _, _, .values _ _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map Prod.snd) HList.nil)
  | _, _, .into_keyed (ℓ := ℓ) (K := K) (V := V) _ _ (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.stream (mem := mem) ℓ (K × V) inferInstance ord .exactlyOnce).delta) i) HList.nil)
  | _, _, .kassume_ordering (ℓ := ℓ) (K := K) (V := V) iK iV x _, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.keyed (mem := mem) ℓ K V iK iV .noOrder .exactlyOnce).delta) i) HList.nil)
  | _, _, .kfold_early_stop _ _ _ x f init, _, Δ, s =>
    let s' := fun i => (Δ.get x i).foldl (kfesInsert f init) (s i)
    (s', HList.cons (fun i => kfesOutMap (s' i)) HList.nil)
  | _, _, .kget_max_key _ _ _ _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).maxEntry) HList.nil)
  | _, _, .weaken_retries (ℓ := ℓ) (α := α) iα (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.stream (mem := mem) ℓ α iα ord .exactlyOnce).delta) i) HList.nil)
  | _, _, .union _ a b, _, Δ, s => (s, HList.cons (fun i => Δ.get a i ++ Δ.get b i) HList.nil)
  | _, _, .assume_ordering (ℓ := ℓ) (α := α) iα x _, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.stream (mem := mem) ℓ α iα .noOrder .exactlyOnce).delta) i) HList.nil)
  | _, _, .fold _ g _ _ _ _ x, _, Δ, s =>
    let s' := fun i => (Δ.get x i).foldl g (s i)
    (s', HList.cons s' HList.nil)
  | _, _, .fold_monotone _ _ g _ _ _ x, _, Δ, s =>
    let s' := fun i => (Δ.get x i).foldl g (s i)
    (s', HList.cons s' HList.nil)
  | _, _, .snapshot (ℓ := ℓ) _ x _, _, Δ, s =>
    (s, HList.cons (fun i => if ctx.tick ℓ i then some (Δ.get x i) else none) HList.nil)
  | _, _, .batch (ℓ := ℓ) _ _ _ x _, _, Δ, s =>
    let p := fun i => s i ++ Δ.get x i
    (fun i => if ctx.tick ℓ i then [] else p i,
     HList.cons (fun i => if ctx.tick ℓ i then some (p i) else none) HList.nil)
  -- the interval timer (D91): this tick's fires, as `()`s, arrive at the tick; the cell
  -- counts ticks (the script's index)
  | _, _, .source_interval (ℓ := ℓ) _ _, d, _, s =>
    (fun i => if ctx.tick ℓ i then s i + 1 else s i,
     HList.cons (fun i => if ctx.tick ℓ i then List.replicate (d i (s i)) () else []) HList.nil)
  | _, _, .firstTick _ _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map List.head?) HList.nil)
  | _, _, .mapTick x f, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map (f i)) HList.nil)
  | _, _, .zipTick a b, _, Δ, s =>
    let pa := fun i => (s i).1 ++ (Δ.get a i).toList
    let pb := fun i => (s i).2 ++ (Δ.get b i).toList
    let n := fun i => min (pa i).length (pb i).length
    (fun i => ((pa i).drop (n i), (pb i).drop (n i)),
     HList.cons (fun i => (List.zip (pa i) (pb i)).head?) HList.nil)
  -- DFIR's `defer_tick` (dfir_lang/src/graph/ops/defer_tick.rs:6, "buffers
  -- all input items and releases them at the next time boundary"): the
  -- cell is the buffer of entries not yet released (the seed before the
  -- first tick); a tick releases its head. One slot leaves per tick; the
  -- buffer holds at most one entry between steps on every Rust-expressible
  -- input (`deferReleased_eq_ticks`) — the FIFO is kept so the node is
  -- DFIR's on *any* input, and so is its machine semantics (`Sched.lean`).
  | _, _, .defer_tick (ℓ := ℓ) _ x, _, Δ, s =>
    let buf := fun i => s i ++ (Δ.get x i).toList
    (fun i => if ctx.tick ℓ i then (buf i).tail else buf i,
     HList.cons (fun i => if ctx.tick ℓ i then (buf i).head? else none) HList.nil)
  | _, _, .allTicks _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).getD []) HList.nil)
  | _, _, .flattenOrdered (ℓ := ℓ) (β := β) _ x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomic (mem := mem) ℓ (List β)).delta) i) HList.nil)
  | _, _, .flattenUnordered (ℓ := ℓ) (β := β) _ x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomic (mem := mem) ℓ (List β)).delta) i) HList.nil)
  | _, _, .tinto_keyed (ℓ := ℓ) (K := K) (V := V) _ _ (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomicStream (mem := mem) ℓ (K × V) inferInstance ord .exactlyOnce).delta) i) HList.nil)
  | _, _, .tassume_ordering (ℓ := ℓ) (α := α) iα x _, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomicStream (mem := mem) ℓ α iα .noOrder .exactlyOnce).delta) i) HList.nil)
  | _, _, .reduce_watermark _ iH _ _ x wm f _ _ _, _, Δ, s =>
    let pa := fun i => (s i).1.1 ++ (Δ.get x i).toList
    let pb := fun i => (s i).1.2 ++ (Δ.get wm i).toList
    let n := fun i => min (pa i).length (pb i).length
    let run := fun i => scanRun (reduceWatermarkEagerH (K := _) f) (s i).2 (List.zip (pa i) (pb i))
    (fun i => (((pa i).drop (n i), (pb i).drop (n i)), (run i).1),
     HList.cons (fun i => (run i).2.head?) HList.nil)
  | _, _, .kentries (ℓ := ℓ) (K := K) (V := V) iK iV (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.keyed (mem := mem) ℓ K V iK iV ord .exactlyOnce).delta) i) HList.nil)
  | _, _, .kmap _ _ _ x f, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x i).map (fun e => (e.1, f i e.2))) HList.nil)
  | _, _, .kallTicks _ _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).getD []) HList.nil)
  | _, _, .kfold _ _ _ _ g init _ _ _ x, _, Δ, s =>
    -- DFIR's `fold_keyed` keeps its map across ticks: the cell is the fold's map, this
    -- step's arrivals folded in (`kfoldStepH`; = the recomputed fold, D86)
    let s' := fun i => kfoldStepH g init (s i) (Δ.get x i)
    (s', HList.cons s' HList.nil)
  | _, _, .ksnapshot (ℓ := ℓ) _ _ _ x _, _, Δ, s =>
    (s, HList.cons (fun i => if ctx.tick ℓ i then some (Δ.get x i) else none) HList.nil)
  | _, _, .tfold _ g _ _ _ _ x, _, Δ, s =>
    let run := fun i => scanRun (tfoldStep g) (s i) (Δ.get x i).toList
    (fun i => (run i).1, HList.cons (fun i => (run i).2.head?) HList.nil)
  | _, _, .latest _ x, _, Δ, s =>
    let s' := fun i => match Δ.get x i with
      | some o => o
      | none => s i
    (s', HList.cons s' HList.nil)
  | _, _, .latest_monotone x, _, Δ, s =>
    let s' := fun i => match Δ.get x i with
      | some o => o
      | none => s i
    (s', HList.cons s' HList.nil)
  | _, _, .time (ℓ := ℓ), d, _, s =>
    (fun i => if ctx.tick ℓ i then s i + 1 else s i,
     HList.cons (fun i => if ctx.tick ℓ i then some (d i (s i)) else none) HList.nil)
  | _, _, .snapshot_bounded (ℓ := ℓ) v _, _, _, s =>
    (fun i => if ctx.tick ℓ i then s i + 1 else s i,
     HList.cons (fun i => if ctx.tick ℓ i then some v else none) HList.nil)
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g _, _, Δ, s =>
    let leaves : Fin (mem ℓ) → HList (Kind.delta (mem := mem)) (shapeKinds ℓ ins) :=
      fun _ => readShapeDelta Δ ℓ ins x
    let p := fun i => PendingOf.push ins (s i).1 leaves i
    let n := fun i => PendingOf.avail ins (p i)
    let run := fun i => scanRun (g i) (s i).2 (PendingOf.takeZip ins (p i) (n i))
    (fun i => (PendingOf.dropN ins (p i) (n i), (run i).1), unzipShape ℓ outs (fun i => (run i).2.head?))
  | _, _, .call N args, sch, Δ, s => Dag.step ctx N sch (args.map (fun v => Δ.get v)) s
  | _, _, .fixN ks body, sch, Δ, s =>
    let fed := knotFeed ctx ks s.2
    let r := Dag.step ctx body sch (fed.append Δ) s.1
    ((r.1, knotKeep ctx ks s.2 r.2.take), fed.append r.2.drop)

/-- One global step of a graph. -/
def Dag.step (ctx : StepCtx L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → Dag.Sched d → HList (Kind.delta (mem := mem)) Γ → Dag.State d →
    Dag.State d × HList (Kind.delta (mem := mem)) outs
  | _, _, .ret xs, _, Δ, s => (s, xs.map (fun v => Δ.get v))
  | _, _, .letN n rest, sch, Δ, s =>
    let r := Node.step ctx n sch.1 Δ s.1
    let r' := Dag.step ctx rest sch.2 (r.2.append Δ) s.2
    ((r.1, r'.1), r'.2)

end

/-! ## The lock-step machine -/

/-- The lock-step run: cells after step `t` and the outputs emitted at
step `t`, from input increments `inp` per step. -/
def Dag.run {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) (sch : Dag.Sched d)
    (pacing : Pacing L mem)
    (inp : Nat → HList (Kind.delta (mem := mem)) Γ) :
    Nat → Dag.State d × HList (Kind.delta (mem := mem)) outs
  | 0 => Dag.step ⟨0, fun ℓ i => pacing.ticks ℓ i 0, fun ℓ i => pacing.active ℓ i 0⟩ d sch (inp 0) (Dag.init d)
  | t + 1 => Dag.step ⟨t + 1, fun ℓ i => pacing.ticks ℓ i (t + 1), fun ℓ i => pacing.active ℓ i (t + 1)⟩ d sch
      (inp (t + 1)) (Dag.run d sch pacing inp t).1

end Project

end Hydro
