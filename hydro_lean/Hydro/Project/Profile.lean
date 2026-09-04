import Hydro.Project.Wired

/-!
# Hydro · Project — a per-node profile of a process's step (D86)

A diagnostic for the production interpreter: `Dag.locStepProfiled` is `Dag.locStep`'s
walk in `IO`, reading the monotonic clock around each node and recording `(tag, nanos)`
per node — the node's constructor and the location it lives at, callee graphs and knot
bodies walked into. Not part of any theorem: `benchprof nodes` uses it to
attribute an activation's time to the operators.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} [DecidableEq L] [Repr L]

/-- The node's constructor name and location (callees and knots are tagged by their shape;
their nodes are walked separately). -/
def Node.tag : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → String
  | _, _, .map (ℓ := ℓ) .. => s!"map@{repr ℓ}"
  | _, _, .filterMap (ℓ := ℓ) .. => s!"filterMap@{repr ℓ}"
  | _, _, .broadcast_closed (c := c) (p := p) .. => s!"broadcast_closed {repr c}→{repr p}"
  | _, _, .demux (c := c) (p := p) .. => s!"demux {repr c}→{repr p}"
  | _, _, .values (ℓ := ℓ) .. => s!"values@{repr ℓ}"
  | _, _, .into_keyed (ℓ := ℓ) .. => s!"into_keyed@{repr ℓ}"
  | _, _, .kassume_ordering (ℓ := ℓ) .. => s!"kassume_ordering@{repr ℓ}"
  | _, _, .kfold_early_stop (ℓ := ℓ) .. => s!"kfold_early_stop@{repr ℓ}"
  | _, _, .kget_max_key (ℓ := ℓ) .. => s!"kget_max_key@{repr ℓ}"
  | _, _, .weaken_retries (ℓ := ℓ) .. => s!"weaken_retries@{repr ℓ}"
  | _, _, .union (ℓ := ℓ) .. => s!"union@{repr ℓ}"
  | _, _, .assume_ordering (ℓ := ℓ) .. => s!"assume_ordering@{repr ℓ}"
  | _, _, .fold (ℓ := ℓ) .. => s!"fold@{repr ℓ}"
  | _, _, .fold_monotone (ℓ := ℓ) .. => s!"fold_monotone@{repr ℓ}"
  | _, _, .snapshot (ℓ := ℓ) .. => s!"snapshot@{repr ℓ}"
  | _, _, .batch (ℓ := ℓ) .. => s!"batch@{repr ℓ}"
  | _, _, .source_interval (ℓ := ℓ) _ _ .. => s!"source_interval@{repr ℓ}"
  | _, _, .mapTick (ℓ := ℓ) .. => s!"mapTick@{repr ℓ}"
  | _, _, .firstTick (ℓ := ℓ) .. => s!"firstTick@{repr ℓ}"
  | _, _, .zipTick (ℓ := ℓ) .. => s!"zipTick@{repr ℓ}"
  | _, _, .defer_tick (ℓ := ℓ) .. => s!"defer_tick@{repr ℓ}"
  | _, _, .allTicks (ℓ := ℓ) .. => s!"allTicks@{repr ℓ}"
  | _, _, .flattenOrdered (ℓ := ℓ) .. => s!"flattenOrdered@{repr ℓ}"
  | _, _, .flattenUnordered (ℓ := ℓ) .. => s!"flattenUnordered@{repr ℓ}"
  | _, _, .tinto_keyed (ℓ := ℓ) .. => s!"tinto_keyed@{repr ℓ}"
  | _, _, .tassume_ordering (ℓ := ℓ) .. => s!"tassume_ordering@{repr ℓ}"
  | _, _, .reduce_watermark (ℓ := ℓ) .. => s!"reduce_watermark@{repr ℓ}"
  | _, _, .kentries (ℓ := ℓ) .. => s!"kentries@{repr ℓ}"
  | _, _, .kmap (ℓ := ℓ) .. => s!"kmap@{repr ℓ}"
  | _, _, .kallTicks (ℓ := ℓ) .. => s!"kallTicks@{repr ℓ}"
  | _, _, .kfold (ℓ := ℓ) .. => s!"kfold@{repr ℓ}"
  | _, _, .ksnapshot (ℓ := ℓ) .. => s!"ksnapshot@{repr ℓ}"
  | _, _, .tfold (ℓ := ℓ) .. => s!"tfold@{repr ℓ}"
  | _, _, .latest (ℓ := ℓ) .. => s!"latest@{repr ℓ}"
  | _, _, .latest_monotone (ℓ := ℓ) .. => s!"latest_monotone@{repr ℓ}"
  | _, _, .time (ℓ := ℓ) .. => s!"time@{repr ℓ}"
  | _, _, .snapshot_bounded (ℓ := ℓ) .. => s!"snapshot_bounded@{repr ℓ}"
  | _, _, .tick_scan (ℓ := ℓ) .. => s!"tick_scan@{repr ℓ}"
  | _, _, .call .. => "call"
  | _, _, .fixN .. => "fixN"

/-- `(tag, nanos)` per node, in graph order (the knot's own bookkeeping is one entry). -/
abbrev NodeProfile := Array (String × Nat)

mutual

/-- `Node.locStep` timed; callee graphs and knot bodies walked node by node. -/
partial def Node.locStepProfiled (m : Member L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Oracle m n →
    HList (Kind.locDelta m) Γ → Node.LocState m n → NodeProfile →
    IO ((Node.LocState m n × HList (Kind.locDelta m) κs × Node.Sends m n) × NodeProfile)
  | _, _, .call N args, ora, Δ, s, prof => do
    let (r, prof) ← Dag.locStepProfiled m tk N ora (args.map (fun v => Δ.get v)) s prof
    pure (r, prof)
  | _, _, .fixN ks body, ora, Δ, s, prof => do
    -- the reference's body steps (D95): the knot increments iterated from empty ones, as
    -- many rounds as there are knots, then the step at them
    let mut K := knotFillLoc m ks
    let mut acc := prof
    for _ in [0:ks.length] do
      let (r₁, acc₁) ← Dag.locStepProfiled m tk body ora (K.append Δ) s acc
      let t1 ← IO.monoNanosNow
      K ← IO.lazyPure fun _ => HList.take (κs := ks.map KnotKind.toKind) r₁.2.1
      let t2 ← IO.monoNanosNow
      acc := acc₁.push ("fixN", t2 - t1)
    let (r, acc') ← Dag.locStepProfiled m tk body ora (K.append Δ) s acc
    pure (r, acc')
  | _, _, n, ora, Δ, s, prof => do
    let t0 ← IO.monoNanosNow
    let r ← IO.lazyPure fun _ => Node.locStep m tk n ora Δ s
    let st ← IO.lazyPure fun _ => Node.memoState m n r.1
    let out ← IO.lazyPure fun _ => memoEnv m r.2.1
    let t1 ← IO.monoNanosNow
    -- the tag carries the node's ordinal in the walk, so two nodes of one shape stay apart
    pure ((st, out, r.2.2), prof.push (s!"#{prof.size} {n.tag}", t1 - t0))

/-- `Dag.locStep` timed node by node. -/
partial def Dag.locStepProfiled (m : Member L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Oracle m d →
    HList (Kind.locDelta m) Γ → Dag.LocState m d → NodeProfile →
    IO ((Dag.LocState m d × HList (Kind.locDelta m) outs × Dag.Sends m d) × NodeProfile)
  | _, _, .ret xs, _, Δ, s, prof => pure ((s, xs.map (fun v => Δ.get v), ()), prof)
  | _, _, .letN n rest, ora, Δ, s, prof => do
    let (r, prof) ← Node.locStepProfiled m tk n ora.1 Δ s.1 prof
    let (r', prof) ← Dag.locStepProfiled m tk rest ora.2 (r.2.1.append Δ) s.2 prof
    pure (((r.1, r'.1), r'.2.1, (r.2.2, r'.2.2)), prof)

end

/-- Sum the profiles of many activations by tag (`(tag, total nanos, count)`), most
expensive first. -/
def NodeProfile.summarize (profs : Array NodeProfile) : Array (String × Nat × Nat) :=
  let m : Std.HashMap String (Nat × Nat) := profs.foldl (fun m prof =>
    prof.foldl (fun m (tag, ns) => m.alter tag fun o => some ((o.getD (0, 0)).1 + ns, (o.getD (0, 0)).2 + 1)) m) {}
  (m.toArray.map fun (tag, tot, cnt) => (tag, tot, cnt)).qsort (fun a b => a.2.1 > b.2.1)

/-- How many nodes the process touches (walks) and how many it skips, callees and knots
walked into (a diagnostic for the per-activation floor). -/
partial def Dag.touchCount (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs →
    Nat × Nat
  | _, _, .ret _ => (0, 0)
  | _, _, .letN n rest =>
    let (t, sk) := Dag.touchCount m rest
    match n with
    | .call N _ => let (t', sk') := Dag.touchCount m N; (t + t' + 1, sk + sk')
    | .fixN _ body => let (t', sk') := Dag.touchCount m body; (t + t' + 1, sk + sk')
    | n => if Node.touches m n then (t + 1, sk) else (t, sk + 1)

/-! ## Idling (D86, D92, D95)

A production process idles between arrivals and timer fires: DFIR runs a tick at start, on a
socket or timer wake, and for a non-lazy `defer_tick` — which Hydro never emits
(`hydro_lang/src/compile/ir/mod.rs`, `HydroNode::DeferTick` is `defer_tick_lazy`, whose
buffered items "are buffered until the next tick fires for another reason",
`dfir_lang/src/graph/ops/defer_tick_lazy.rs`). The shells follow that rule: an activation is
`Dag.iterateBytes` — one tick, the knot wires their bodies' outputs of the same tick
(the sweep, `Project/Sweep.lean`) — and a process whose sockets are silent and whose timers
are not due waits. Earlier the shells ticked whenever a reading of the cells
(`Dag.hasPending`, deleted) found a `defer_tick` buffer, a knot's pending feedback or a
hosted tick, at 100 % CPU on every tick-hosting process; later (D93) an activation settled
such feedback with stutter passes — gone with the feedback cells (D95). -/

end Project

end Hydro
