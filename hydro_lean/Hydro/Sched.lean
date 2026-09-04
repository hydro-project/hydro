import Hydro.Sem

/-!
# Hydro · the step machine (`SchedSem`)

The concurrent operational semantics: **erased and concrete**. Every
grade executes as plain lists — no `Multiset`, `StutterSeq`, or
`RetryPool` anywhere in the machine; the quotients exist only in the
denotation, and the transfer coupling (`Transfer.lean`) relates this
machine's concrete runs to them *through* the grade types.

**Time**: one global step clock. A live wire is a `StepHist` — its
concrete content *as of each step*, prefix-monotone by construction
(history immutability is physical: buffers only grow). Fan-in
(`values`/`union`) takes no decision: at each step it concatenates the
inputs' step increments, so cross-sender interleaving **emerges from
delivery timing** (within-step simultaneity gets a canonical order,
WLOG — the adversary can split arrivals across steps).

**Where the machine's nondeterminism lives** (its `…Dec` vocabulary):

- `TransportDec` — per-`(receiver, sender)` delivery cursors as
  functions of the step (run through `cumMax`, so delivery is a
  monotone prefix: TCP per-pair FIFO; stalling = unbounded latency;
  `0` forever = silence). Transport never *creates* retries
  (`TCP.fail_stop` preserves the grade): `AtLeastOnce` duplicates are
  content born at sampling sites and ride the wire.
- concrete timing (`SampleDec`/`TimerDec`/`PulseDec`, as in `Values`).
- **tick pacing** — the per-member tick skeleton
  `pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool` (which steps each
  member of each location ticks at), an ambient parameter of the
  instance. Tick presence is independent across cluster members: one
  member may tick with content at a step where a sibling has no tick
  entry at all. Its irreducible content is the presence/absence of
  stutter ticks; batch partitioning is jointly covered with delivery
  bursts.

Content sites (`batch`/`snapshot`/`assume_ordering`/…) take `Unit`: the
machine consumes its buffers in arrival order — a tick consumes
everything available (hydro's real semantics), a snapshot reads the
fold accumulator's latest value, `assume_ordering` is the identity
(the machine has one real order).

**Time (idle steps and floors)** — the conceptual core of the clock:
idle/empty steps are legal and observationally invisible (observations
are tick- and content-indexed, never step-indexed); they are how the
model represents time passing without progress. The clock is **elastic
above, floored below**, and both directions are load-bearing:

- *elasticity* is adversary coverage — async ticks across locations and
  members are independent skeletons, unbounded lateness is idle steps
  on a path, and granularity refinement (uniform idle-step insertion)
  is why the canonical within-step order and the `+1` floors lose no
  real behaviors;
- the *floors* — network delivery `+1` (`StepHist.deliver`) and the
  knot boundary `+1` (`StepHist.shift` at `fix_stream`, and the
  implicit-defer step shift at `fix_tick`: hydro does not allow
  synchronous in-tick cycles, so the tick feedback wire reads one step
  stale — sound, since staler ⊑ the same denotational knot) — are
  well-definedness: they exclude exactly the Zeno executions
  (zero-time round trips) under which the Kleene diagonal has no
  stable value and a transfer's derived decisions conflict across
  cycle stages.

Quiescence is "all remaining steps idle", so end-of-time equality
reads: once time stops mattering, the machine *is* its denotation.
Breaking either side fails differently: without idle steps, coverage
(asymmetry/latency) is lost; without floors, `fix` is ill-defined.

**Cycles**: `fix` takes `Unit` — cycle unfolding *is* step progression.
The knot is the Kleene diagonal `fun t => (body^[t+1] ⊥) t`; stream
knots re-monotonize through `famFreeze` (a no-op for real, guarded
bodies, but total on arbitrary ones).

**Tick-domain carriers are raw per-step views** (`Nat → Trace σ`, no
monotonicity bundle): every tick op is pointwise per step, and
monotonicity is re-established at the tick→stream boundaries
(`allTicks`/`sample_every`) by `famFreeze` — family-atomically, so a
frozen view is always a *single* raw family view.
-/

namespace Hydro

/-! ## Step histories -/

/-- A prefix-monotone step history: the concrete content of a wire as
of each global step. -/
structure StepHist (α : Type) where
  view : Nat → List α
  mono : ∀ t, view t <+: view (t + 1)

namespace StepHist

theorem mono_le {α : Type} (h : StepHist α) {t t' : Nat} (ht : t ≤ t') :
    h.view t <+: h.view t' := by
  induction t' with
  | zero => cases Nat.le_zero.mp ht; exact List.prefix_refl _
  | succ t' ih =>
    rcases Nat.lt_or_ge t (t' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (h.mono t')
    · cases Nat.le_antisymm ht hge; exact List.prefix_refl _

/-- The empty wire. -/
def bot {α : Type} : StepHist α := ⟨fun _ => [], fun _ => List.prefix_refl _⟩

/-- A wire whose whole content is present from step 0 (a local source). -/
def const {α : Type} (l : List α) : StepHist α :=
  ⟨fun _ => l, fun _ => List.prefix_refl _⟩

def map {α β : Type} (f : α → β) (h : StepHist α) : StepHist β :=
  ⟨fun t => (h.view t).map f, fun t => List.IsPrefix.map f (h.mono t)⟩

def filterMap {α β : Type} (f : α → Option β) (h : StepHist α) :
    StepHist β :=
  ⟨fun t => (h.view t).filterMap f, fun t => prefix_filterMap f (h.mono t)⟩

/-- The step increment: what arrived between steps `t` and `t+1`. -/
def inc {α : Type} (h : StepHist α) (t : Nat) : List α :=
  (h.view (t + 1)).drop (h.view t).length

theorem view_succ {α : Type} (h : StepHist α) (t : Nat) :
    h.view (t + 1) = h.view t ++ h.inc t := by
  obtain ⟨e, he⟩ := h.mono t
  rw [inc, ← he, List.drop_left]

end StepHist

/-! ## Delivery (ordered transport) -/

/-- Cumulative max of a cursor: delivery cursors are used monotonically
(a delivered message stays delivered). -/
def cumMax (c : Nat → Nat) : Nat → Nat
  | 0 => c 0
  | t + 1 => max (cumMax c t) (c (t + 1))

theorem cumMax_mono (c : Nat → Nat) (t : Nat) :
    cumMax c t ≤ cumMax c (t + 1) := Nat.le_max_left _ _

/-- A prefix's take is a prefix of the take. -/
theorem prefix_take_prefix {α : Type _} {l l' : List α} (n : Nat)
    (h : l <+: l') : l.take n <+: l'.take n := by
  rw [List.prefix_iff_eq_take.mp h, List.take_take]
  exact take_prefix_take (Nat.min_le_left _ _)

/-- Deliver a prefix of a wire per a step cursor: what the network has
handed the receiver by each step. **Delivery takes at least one step**
(no same-instant network hop: at step `t` the receiver sees a cursor
prefix of the sender's wire *as of `t - 1`*; step 0 sees nothing).
The floor is structural — the cursor keeps all its freedom above it —
and it is what makes every cycle through the network advance the step
clock, so knot stabilization-in-depth is program-independent. -/
def StepHist.deliver {α : Type} (h : StepHist α) (c : Nat → Nat) :
    StepHist α where
  view
    | 0 => []
    | t + 1 => (h.view t).take (cumMax c (t + 1))
  mono t := by
    cases t with
    | zero => exact List.nil_prefix
    | succ t =>
      exact (prefix_take_prefix _ (h.mono t)).trans
        (take_prefix_take (cumMax_mono c (t + 1)))

/-- Delay a wire by one step (the event-loop pass of a `forward_ref`
knot: the feedback wire presents the body's output shifted by one
step). Structural, so *every* cycle — network-free local loops
included — advances the step clock per unfolding. -/
def StepHist.shift {α : Type} (h : StepHist α) : StepHist α where
  view
    | 0 => []
    | t + 1 => h.view t
  mono t := by
    cases t with
    | zero => exact List.nil_prefix
    | succ t => exact h.mono t

/-! ## Fan-in (interleaving emerges from delivery timing) -/

/-- Merge two wires: at each step, append both inputs' increments. -/
def merge2View {α : Type} (a b : StepHist α) : Nat → List α
  | 0 => a.view 0 ++ b.view 0
  | t + 1 => merge2View a b t ++ a.inc t ++ b.inc t

def merge2 {α : Type} (a b : StepHist α) : StepHist α :=
  ⟨merge2View a b,
   fun t => ⟨a.inc t ++ b.inc t, (List.append_assoc _ _ _).symm⟩⟩

/-- Merge a family of wires (keyed fan-in): at each step, append every
sender's increment, in sender order within the step. -/
def mergeNView {m : Nat} {α : Type} (k : Fin m → StepHist α) :
    Nat → List α
  | 0 => (List.finRange m).flatMap (fun j => (k j).view 0)
  | t + 1 => mergeNView k t
      ++ (List.finRange m).flatMap (fun j => (k j).inc t)

def mergeN {m : Nat} {α : Type} (k : Fin m → StepHist α) : StepHist α :=
  ⟨mergeNView k, fun _t => ⟨_, rfl⟩⟩

/-! ## The tick skeleton -/

/-- The steps at which a location has ticked, as of step `t`. -/
def tickSteps (p : Nat → Bool) (t : Nat) : List Nat :=
  (List.range (t + 1)).filter p

theorem tickSteps_prefix (p : Nat → Bool) (t : Nat) :
    tickSteps p t <+: tickSteps p (t + 1) := by
  unfold tickSteps
  rw [List.range_succ (n := t + 1), List.filter_append]
  exact ⟨_, rfl⟩

/-- Per-tick batches: at each tick step, everything available and not
yet consumed (hydro's real `batch` semantics). -/
def batchesFrom {α : Type} (src : Nat → List α) :
    List Nat → Nat → List (List α)
  | [], _ => []
  | s :: rest, consumed =>
    (src s).drop consumed :: batchesFrom src rest (src s).length

/-! ## Re-monotonization at tick→stream boundaries -/

/-- Freeze a raw per-member view sequence into prefix-monotone
histories, **family-atomically**: advance only while *every* member's
next view extends its current one — so every frozen family view is one
of the raw family views, and the transfer coupling can name the single
step it came from. A no-op on real flows (tick traces grow by prefix,
member-uniformly); total on arbitrary ones. -/
def famFreeze {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) : Nat → Fin n → List β
  | 0 => h 0
  | t + 1 =>
    let prev := famFreeze h t
    if (List.finRange n).all
        (fun i => (prev i).isPrefixOf (h (t + 1) i))
    then h (t + 1) else prev

theorem famFreeze_mono {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) (i : Fin n) (t : Nat) :
    famFreeze h t i <+: famFreeze h (t + 1) i := by
  show famFreeze h t i <+:
    (if (List.finRange n).all
        (fun j => (famFreeze h t j).isPrefixOf (h (t + 1) j))
     then h (t + 1) else famFreeze h t) i
  split
  · next hp =>
    exact List.isPrefixOf_iff_prefix.mp
      (List.all_eq_true.mp hp i (List.mem_finRange i))
  · exact List.prefix_refl _

/-- Every frozen family view is one of the raw family views (no
splicing, no member skew): the coupling names the step it came from. -/
theorem famFreeze_eq_raw {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) :
    ∀ t, ∃ k, k ≤ t ∧ famFreeze h t = h k
  | 0 => ⟨0, Nat.le_refl _, rfl⟩
  | t + 1 => by
    show ∃ k, k ≤ t + 1 ∧
      (if (List.finRange n).all
          (fun i => (famFreeze h t i).isPrefixOf (h (t + 1) i))
       then h (t + 1) else famFreeze h t) = h k
    split
    · exact ⟨t + 1, Nat.le_refl _, rfl⟩
    · obtain ⟨k, hk, he⟩ := famFreeze_eq_raw h t
      exact ⟨k, Nat.le_succ_of_le hk, he⟩

/-- Package a frozen family as per-member step histories. -/
def famHist {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) : Fin n → StepHist β :=
  fun i => ⟨fun t => famFreeze h t i, famFreeze_mono h i⟩

/-! ## The fold singleton: the live value, per step

A folded live singleton at the machine is **its current value as of
each step**: the fold of whatever the source wire has delivered by that
step (arrival order — no commutativity needed to *run*; the grade
obligations are consumed by the transfer proof), and a tick read
(`snapshot`) takes the value at the tick's step. This is the
step-indexed image of hydro-project/hydro#3189's runtime
representation — a root singleton/optional is a pipe of accumulator
updates (tombstones for `None`) whose consumer holds the latest — with
the pipe's successive updates being this function's successive values.
Asynchrony is entirely upstream (the delivery cursors decide how much
has been folded by each step); nothing is stored but the value. -/

/-- The live value of a fold over a source wire, per step. -/
def schedFold {α σ : Type} (g : σ → α → σ) (init : σ) (src : StepHist α) :
    Nat → σ :=
  fun t => (src.view t).foldl g init

/-! ## The instance -/

/-! ### Tick shapes at the machine: per-step slicing of a shaped input
into this tick's list-valued tuples, and back -/

namespace SchedTick
variable {L : Type} {mem : L → Nat}

abbrev Tk (mem : L → Nat) (ℓ : L) (τ : Type) : Type := Fin (mem ℓ) → Nat → Trace τ
abbrev TS (mem : L → Nat) (ℓ : L) (α : Type) [DecidableEq α] (_ord : StrOrd)
    (_ret : Retries) : Type := Fin (mem ℓ) → Nat → Trace (List α)
abbrev TKS (mem : L → Nat) (ℓ : L) (K V : Type) [DecidableEq K] [DecidableEq V] :
    Type := Fin (mem ℓ) → Nat → Trace (SchedKMap K V)
abbrev BS (σ : Type) : Type := σ
abbrev BStr (α : Type) [DecidableEq α] (_ord : StrOrd) (_ret : Retries) : Type := List α
abbrev BKS (K V : Type) [DecidableEq K] [DecidableEq V] : Type := SchedKMap K V

def slice {ℓ : L} : (sh : TickShape) → AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh →
    Fin (mem ℓ) → Nat → Trace (BoundedOf BS BStr BKS sh)
  | .sing _, x, i, k => x i k
  | .stream _ _ _ _, x, i, k => x i k
  | .ksing _ _ _ _, x, i, k => x i k
  | .pair a b, x, i, k => Trace.zip (slice a x.1 i k) (slice b x.2 i k)

def unslice {ℓ : L} : (sh : TickShape) →
    (Fin (mem ℓ) → Nat → Trace (BoundedOf BS BStr BKS sh)) →
    AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ sh
  | .sing _, f => f
  | .stream _ _ _ _, f => f
  | .ksing _ _ _ _, f => f
  | .pair a b, f =>
    (unslice a (fun i k => (f i k).map Prod.fst),
     unslice b (fun i k => (f i k).map Prod.snd))

/-- The initial register of a shape from its seed (a stream register
starts empty). -/
def seed : (sh : TickShape) → SeedOf sh → BoundedOf BS BStr BKS sh
  | .sing _, v => v
  | .stream _ _ _ _, _ => []
  | .ksing _ _ _ _, _ => SchedKMap.empty
  | .pair a b, s => (seed a s.1, seed b s.2)

/-- The tick former at the machine: the same structural fold, on the
runtime's lists, as of each step. -/
def scan {ℓ : L} (sts ins outs : TickShape)
    (x : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS BStr BKS sts → BoundedOf BS BStr BKS ins →
      BoundedOf BS BStr BKS sts × BoundedOf BS BStr BKS outs)
    (init : SeedOf sts) : AtomicOf (Tk mem) (TS mem) (TKS mem) ℓ outs :=
  unslice outs (fun i k => scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i k))

end SchedTick

set_option warn.classDefReducibility false in
/-- The step machine, over a tick skeleton `pacing` (which steps each
member of each location ticks at — tick presence is independent across
cluster members). -/
def SchedSem (L : Type) (mem : L → Nat)
    (pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) :
    HydroSem L mem where
  Stream ℓ α _ _ _ := Fin (mem ℓ) → StepHist α
  -- a keyed stream: the runtime's flat wire of pairs (`into_keyed` is a
  -- cast; the network's is tagged by sender)
  KeyedStream ℓ K V _ _ _ _ := Fin (mem ℓ) → StepHist (K × V)
  -- a folded singleton: its live value as of each step
  Singleton ℓ α σ _ _ _ _b := Fin (mem ℓ) → Nat → σ
  -- a keyed fold's live map as of each step (the runtime's association
  -- list, keys distinct)
  KeyedSingleton ℓ _α K V _ _ _ _ _ _b := Fin (mem ℓ) → Nat → SchedKMap K V
  -- the ticked carrier: per member, the realized tick sequence AS OF
  -- each machine step (prefix-monotone in the step — which step each
  -- tick ran on)
  Atomic ℓ σ := Fin (mem ℓ) → Nat → Trace σ
  -- one tick's bounded stream: the concrete list the runtime holds, at
  -- EVERY grade (the quotient is the denotation's; here order/retries
  -- are unobservable only through the in-tick operators' typing)
  BoundedStream α _ _ _ := List α
  BoundedSingleton σ := σ
  BoundedKeyedStream K V _ _ _ _ := List (K × V)
  BoundedKeyedSingleton K V _ _ := SchedKMap K V
  -- `AtomicStream` is the signature default: `Fin → Nat → Trace (List α)`
  TransportDec p c := Fin p → Fin c → Nat → Nat
  OrderSelDec _ _ := Unit
  TickOrderSelDec _ _ := Unit
  SnapDec _ _ _ := Unit
  BatchDec _ _ := Unit
  OrdBatchDec _ := Unit
  SampleDec n := SampleTimes n
  TimerDec n := TimerVerdicts n
  PulseDec n := TimingPulses n
  FixDec := Unit
  map s f := fun i => (s i).map (f i)
  filterMap s f := fun i => (s i).filterMap (f i)
  broadcast_closed d s := fun i =>
    mergeN (fun j => ((s j).deliver (d i j)).map (fun v => (j, v)))
  demux d s addr := fun i =>
    mergeN (fun j => (((s j).filterMap
      (fun dx => if dx.1 = addr i then some dx.2 else none)).deliver
      (d i j)).map (fun v => (j, v)))
  values k := fun i => (k i).map Prod.snd
  into_keyed s := s
  kassume_ordering u _d := u
  kfold_early_stop ks f init := fun i t =>
    keyedFoldEarlyStopAssoc f init ((ks i).view t)
  kget_max_key ks := fun i t => (ks i t).maxEntry
  weaken_retries s := s
  union a b := fun i => merge2 (a i) (b i)
  assume_ordering u _d := u
  fold g init _Pre _ _ok s := fun i => schedFold g init (s i)
  fold_monotone _vo g init _ok _hinfl s := fun i => schedFold g init (s i)
  snapshot {ℓ _α _σ _ _ord _ret _b} s _d := fun i t =>
    (tickSteps (pacing ℓ i) t).map (fun st => s i st)
  batch {ℓ _α _} s _d := fun i t =>
    batchesFrom ((s i).view) (tickSteps (pacing ℓ i) t) 0
  batch_ordered {ℓ _α _} s _d := fun i t =>
    batchesFrom ((s i).view) (tickSteps (pacing ℓ i) t) 0
  sample_every t d :=
    famHist (fun step i => sampleAtOpt (t i step) (d i))
  timeout_snapshot {ℓ _α _ _ord _ret} _s d := fun i step =>
    (d i).take (tickSteps (pacing ℓ i) step).length
  source_interval_batch {ℓ} d := fun i step =>
    (d i).take (tickSteps (pacing ℓ i) step).length
  mapTick s f := fun i step => (s i step).map (f i)
  zipTick a b := fun i step => Trace.zip (a i step) (b i step)
  defer_tick init t := fun i step => init :: t i step
  allTicks bs := famHist (fun step i => (bs i step).flatten)
  flattenOrdered t := t
  flattenUnordered t := t
  fix_stream _d body :=
    famHist (fun t i =>
      ((iterate (fun x j => ((body x) j).shift)
        (fun _ => StepHist.bot) (t + 1)) i).view t)
  fix_tick _d body := fun i t =>
    (iterate (fun x j u => match u with
        | 0 => ([] : Trace _)
        | u + 1 => (body x) j u)
      (fun _ _ => []) (t + 1)) i t
  tinto_keyed x := x
  tassume_ordering u _d := u
  -- DFIR's `ReduceKeyedWatermark` (`compile/ir/mod.rs:5221–5310`): a
  -- `fold::<'static>` over (map, watermark), this tick's payloads then
  -- its watermark, GC eager — on the association list
  reduce_watermark ks wm f _Pre _ _ok := fun i t =>
    scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none)
      (Trace.zip (ks i t) (wm i t))
  -- in-tick operators on the runtime's plain lists (every grade)
  bmap b f := b.map f
  bfilterMap b f := b.filterMap f
  bflatMapOrdered l f := l.flatMap f
  bflatMapUnordered b f := b.flatMap f
  bofList l := l
  bcount b := b.length
  bfold g init _Pre _ _ok b := b.foldl g init
  benumerate l := listEnumerate l
  bfirst l := l.head?
  bcrossSingleton b s := b.map (fun a => (a, s))
  bchain a b := a ++ b
  bweakenOrder b := b
  bfilter b p := b.filter p
  binto_keyed b := b
  bkfold g init _Pre _ _ok ks := keyedFoldAssoc g init ks
  bksFilter m p := m.filter p
  bksMap m f := m.map f
  bksKeys m := m.keys
  bksEntries m := m.entries
  bjoin a b := listJoin a b
  bantiJoin a ks := a.filter (fun e => !(decide (e.1 ∈ ks)))
  bfilterNotIn a o := a.filter (fun x => !(decide (x ∈ o)))
  bmax b := b.foldl maxStep none
  bfilterIf b flag := if flag then b else []
  bsPure v := v
  bsMap s f := f s
  bsZip a b := (a, b)
  boMap o f := o.map f
  boUnwrapOr o s := o.getD s
  boFilter o p := o.filter p
  boIsSome o := o.isSome
  tick_scan sts ins outs x g init := SchedTick.scan sts ins outs x g init

end Hydro
