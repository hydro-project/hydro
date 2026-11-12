import Hydro.Project.Wire

/-!
# Hydro · Project — the activation as a sweep

A knot wire is its body's knot output of the same step (`SchedSem`'s ungated diagonal). The
lock-step machines (`Node.step`/`Node.locStep` at `fixN`) compute it by iterating the body's
step, as many rounds as there are knots; a process computes it in **one evaluation per
node** by sweeping over the body with a **knownness** mask on the wires.

- A node whose inputs are all known is stepped and marked `done`; its outputs and sends are
  stored.
- A **strict** node (`broadcast_closed`, `demux`, `defer_tick`) whose input is not yet known
  is released (`Node.release`: delivery from the queue or the deferred slot, a function of
  its cell alone) and marked `released`; its push (the sends it reports or the slot it
  buffers, `Node.pushStep`) waits for the input.
- Any other node with an unknown input is skipped and stays `todo`. A typed filler stands
  for its outputs; no node that is stepped reads it.

At a knot block the body is swept with its knot wires unknown. The knot outputs that come
out known, those reading no knot wire at the same step, are bound, and the body is swept
again with them known, which determines the knots reading only those, and so on
(`knotLoop`). On a valid body (`Dag.valid`, the lag analysis of `Graph/Causal.lean`: every
cycle among the knots passes a strict node, DFIR's "acyclic through strata") every knot is
bound within as many sweeps as there are knots, and one final sweep with every knot known
steps the `todo` nodes and pushes the `released` ones; `done` nodes are skipped throughout.
A nested block whose externals are partly unknown is swept partially and completed when the
enclosing sweep knows them.

The top-level activation is the sweep from a fresh `Dag.Part` on a fully known environment
(`Dag.sweepTop`); `Dag.iterate_eq_pass` (`Project/Model/SweepSpec.lean`) says the
activation equals `Dag.pass`, which runs the lock-step step `Dag.locStep`. The executed form, with inbox, materialization and encoding fused per node,
is `Dag.sweepF` (`Project/Fused.lean`).
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-! ## Knownness -/

/-- A knownness flag per wire. -/
abbrev Mask (κs : List (Kind L mem)) : Type := HList (fun _ => Bool) κs

namespace Mask

def const : (κs : List (Kind L mem)) → Bool → Mask κs
  | [], _ => HList.nil
  | _ :: κs, b => HList.cons b (const κs b)

def all : {κs : List (Kind L mem)} → Mask κs → Bool
  | [], _ => true
  | _ :: _, mk => mk.1 && all mk.2

/-- The pointwise disjunction: known in either. -/
def or : {κs : List (Kind L mem)} → Mask κs → Mask κs → Mask κs
  | [], _, _ => HList.nil
  | _ :: _, a, b => HList.cons (a.1 || b.1) (or a.2 b.2)

/-- Boolean equality of two masks. -/
def beq : {κs : List (Kind L mem)} → Mask κs → Mask κs → Bool
  | [], _, _ => true
  | _ :: _, a, b => (a.1 == b.1) && beq a.2 b.2

/-- Keep the entries the mask knows, take the others from the second list. -/
def select {f : Kind L mem → Type} : {κs : List (Kind L mem)} → Mask κs → HList f κs → HList f κs → HList f κs
  | [], _, _, _ => HList.nil
  | _ :: _, mk, a, b => HList.cons (if mk.1 then a.1 else b.1) (select mk.2 a.2 b.2)

end Mask

/-- Whether every leaf a shaped tick input reads is known. -/
def shapeKnown {Γ : Ctx L mem} {ℓ : L} (mk : Mask Γ) : (sh : TickShape) → VarsOf Γ ℓ sh → Bool
  | .sing _, v => mk.get v
  | .stream _ _ _ _, v => mk.get v
  | .ksing _ _ _ _, v => mk.get v
  | .kstream _ _ _ _ _ _, v => mk.get v
  | .pair a b, (va, vb) => shapeKnown mk a va && shapeKnown mk b vb

/-- A strict node: its outputs at a step are a function of its cell, because the network
nodes deliver from their queues and `defer_tick` releases its head; its input enters only
the push. -/
def Node.isStrict {Γ : Ctx L mem} {κs : List (Kind L mem)} : Node Γ κs → Bool
  | .broadcast_closed .. => true
  | .demux .. => true
  | .defer_tick .. => true
  | _ => false

/-- Whether every wire a plain node reads is known. -/
def Node.inputsKnown {Γ : Ctx L mem} {κs : List (Kind L mem)} : Node Γ κs → Mask Γ → Bool
  | .map _ _ x _, mk => mk.get x
  | .filterMap _ _ x _, mk => mk.get x
  | .broadcast_closed _ x, mk => mk.get x
  | .demux _ x _, mk => mk.get x
  | .values _ _ x, mk => mk.get x
  | .into_keyed _ _ x, mk => mk.get x
  | .kassume_ordering _ _ x _, mk => mk.get x
  | .kfold_early_stop _ _ _ x _ _, mk => mk.get x
  | .kget_max_key _ _ _ _ x, mk => mk.get x
  | .weaken_retries _ x, mk => mk.get x
  | .union _ a b, mk => mk.get a && mk.get b
  | .assume_ordering _ x _, mk => mk.get x
  | .fold _ _ _ _ _ _ x, mk => mk.get x
  | .fold_monotone _ _ _ _ _ _ x, mk => mk.get x
  | .snapshot _ x _, mk => mk.get x
  | .batch _ _ _ x _, mk => mk.get x
  | .source_interval _ _, _ => true
  | .firstTick _ _ x, mk => mk.get x
  | .mapTick x _, mk => mk.get x
  | .zipTick a b, mk => mk.get a && mk.get b
  | .defer_tick _ x, mk => mk.get x
  | .allTicks _ x, mk => mk.get x
  | .flattenOrdered _ x, mk => mk.get x
  | .flattenUnordered _ x, mk => mk.get x
  | .tinto_keyed _ _ x, mk => mk.get x
  | .tassume_ordering _ x _, mk => mk.get x
  | .reduce_watermark _ _ _ _ x wm _ _ _ _, mk => mk.get x && mk.get wm
  | .kentries _ _ x, mk => mk.get x
  | .kmap _ _ _ x _, mk => mk.get x
  | .kallTicks _ _ x, mk => mk.get x
  | .kfold _ _ _ _ _ _ _ _ _ x, mk => mk.get x
  | .ksnapshot _ _ _ x _, mk => mk.get x
  | .tfold _ _ _ _ _ _ x, mk => mk.get x
  | .latest _ x, mk => mk.get x
  | .latest_monotone x, mk => mk.get x
  | .time, _ => true
  | .snapshot_bounded _ _, _ => true
  | .tick_scan _ ins _ x _ _, mk => shapeKnown mk ins x
  | .call _ args, mk => Mask.all (args.map (fun v => mk.get v))
  | .fixN _ _, mk => mk.all

/-! ## What a sweep has done at a node -/

/-- The state of a plain node across the sweeps of one step. -/
inductive NodePart (O S : Type) where
  | todo
  | released (outs : O) (snd : S)
  | done (outs : O) (snd : S)

/-- The sends a plain node's phase holds; `nil` for a `todo` node. -/
def NodePart.sends {O S : Type} (nil : S) : NodePart O S → S
  | .todo => nil
  | .released _ snd => snd
  | .done _ snd => snd

mutual

/-- A node's sweep state beyond its own phase: a callee's parts; a knot block's knot
increments with their knownness (the knots bound so far) and its body's parts. -/
def Node.Sub (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .call N _ => Dag.Part m N
  | _, _, .fixN ks body => (Mask (ks.map KnotKind.toKind) × HList (Kind.locDelta m) (ks.map KnotKind.toKind)) × Dag.Part m body
  | _, _, _ => Unit

/-- The sweep state of a graph: per node its phase with its stored outputs and sends, and its
sub-parts. -/
def Dag.Part (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN (κs := κs) n rest => (NodePart (HList (Kind.locDelta m) κs) (Node.Sends m n) × Node.Sub m n) × Dag.Part m rest

end

/-- A node's sweep state. -/
abbrev Node.Part (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs) : Type :=
  NodePart (HList (Kind.locDelta m) κs) (Node.Sends m n) × Node.Sub m n

/-- A plain node: neither a call nor a knot block. -/
def Node.isPlain {Γ : Ctx L mem} {κs : List (Kind L mem)} : Node Γ κs → Bool
  | .call .. => false
  | .fixN .. => false
  | _ => true

mutual

/-- Nothing done yet. -/
def Node.Sub.init (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Sub m n
  | _, _, .call N _ => Dag.Part.init m N
  | _, _, .fixN ks body => ((Mask.const _ false, knotFillLoc m ks), Dag.Part.init m body)
  | _, _, .map .. => ()
  | _, _, .filterMap .. => ()
  | _, _, .broadcast_closed .. => ()
  | _, _, .demux .. => ()
  | _, _, .values .. => ()
  | _, _, .into_keyed .. => ()
  | _, _, .kassume_ordering .. => ()
  | _, _, .kfold_early_stop .. => ()
  | _, _, .kget_max_key .. => ()
  | _, _, .weaken_retries .. => ()
  | _, _, .union .. => ()
  | _, _, .assume_ordering .. => ()
  | _, _, .fold .. => ()
  | _, _, .fold_monotone .. => ()
  | _, _, .snapshot .. => ()
  | _, _, .batch .. => ()
  | _, _, .source_interval .. => ()
  | _, _, .mapTick .. => ()
  | _, _, .firstTick .. => ()
  | _, _, .zipTick .. => ()
  | _, _, .defer_tick .. => ()
  | _, _, .allTicks .. => ()
  | _, _, .flattenOrdered .. => ()
  | _, _, .flattenUnordered .. => ()
  | _, _, .tinto_keyed .. => ()
  | _, _, .tassume_ordering .. => ()
  | _, _, .kentries .. => ()
  | _, _, .kmap .. => ()
  | _, _, .kallTicks .. => ()
  | _, _, .kfold .. => ()
  | _, _, .ksnapshot .. => ()
  | _, _, .tfold .. => ()
  | _, _, .latest .. => ()
  | _, _, .latest_monotone .. => ()
  | _, _, .time .. => ()
  | _, _, .snapshot_bounded .. => ()
  | _, _, .reduce_watermark .. => ()
  | _, _, .tick_scan .. => ()

def Dag.Part.init (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Part m d
  | _, _, .ret _ => ()
  | _, _, .letN n rest => ((.todo, Node.Sub.init m n), Dag.Part.init m rest)

end

/-! ## The strict nodes' push, and the fillers -/

/-- The push half of a strict node, once its input is known: a network node reports its
sends, `defer_tick` buffers the slot; every other node is left as it is. -/
def Node.pushStep (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    HList (Kind.locDelta m) Γ → Node.LocState m n → Node.Sends m n → Node.LocState m n × Node.Sends m n
  | _, _, .broadcast_closed _ x, Δ, s, _ => (s, fun j => Δ.get x j)
  | _, _, .demux _ x _, Δ, s, _ => (s, fun j => Δ.get x j)
  | _, _, .defer_tick _ x, Δ, s, snd => (fun i => s i ++ (Δ.get x i).toList, snd)
  | _, _, _, _, s, snd => (s, snd)

mutual

/-- The empty sends of a node. -/
def Node.sendsNil (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Sends m n
  | _, _, .broadcast_closed .. => fun _ => #[]
  | _, _, .demux .. => fun _ => #[]
  | _, _, .call N _ => Dag.sendsNil m N
  | _, _, .fixN _ body => Dag.sendsNil m body
  | _, _, .map .. => ()
  | _, _, .filterMap .. => ()
  | _, _, .values .. => ()
  | _, _, .into_keyed .. => ()
  | _, _, .kassume_ordering .. => ()
  | _, _, .kfold_early_stop .. => ()
  | _, _, .kget_max_key .. => ()
  | _, _, .weaken_retries .. => ()
  | _, _, .union .. => ()
  | _, _, .assume_ordering .. => ()
  | _, _, .fold .. => ()
  | _, _, .fold_monotone .. => ()
  | _, _, .snapshot .. => ()
  | _, _, .batch .. => ()
  | _, _, .source_interval .. => ()
  | _, _, .mapTick .. => ()
  | _, _, .firstTick .. => ()
  | _, _, .zipTick .. => ()
  | _, _, .defer_tick .. => ()
  | _, _, .allTicks .. => ()
  | _, _, .flattenOrdered .. => ()
  | _, _, .flattenUnordered .. => ()
  | _, _, .tinto_keyed .. => ()
  | _, _, .tassume_ordering .. => ()
  | _, _, .kentries .. => ()
  | _, _, .kmap .. => ()
  | _, _, .kallTicks .. => ()
  | _, _, .kfold .. => ()
  | _, _, .ksnapshot .. => ()
  | _, _, .tfold .. => ()
  | _, _, .latest .. => ()
  | _, _, .latest_monotone .. => ()
  | _, _, .time .. => ()
  | _, _, .snapshot_bounded .. => ()
  | _, _, .reduce_watermark .. => ()
  | _, _, .tick_scan .. => ()

def Dag.sendsNil (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Sends m d
  | _, _, .ret _ => ()
  | _, _, .letN n rest => (Node.sendsNil m n, Dag.sendsNil m rest)

end

mutual

/-- The sends a sweep state holds. -/
def Node.partSends (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Part m n → Node.Sends m n
  | _, _, .call N _, part => Dag.partSends m N part.2
  | _, _, .fixN _ body, part => Dag.partSends m body part.2.2
  | _, _, n, part => part.1.sends (Node.sendsNil m n)

def Dag.partSends (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Part m d → Dag.Sends m d
  | _, _, .ret _, _ => ()
  | _, _, .letN n rest, part => (Node.partSends m n part.1, Dag.partSends m rest part.2)

end

/-- The empty increments of a knot block's wires at the process. -/
def knotFillLocOuts (m : Member L mem) (ks : List (KnotKind L mem)) {Γ : Ctx L mem} :
    HList (Kind.locDelta m) (ks.map KnotKind.toKind ++ Γ) → HList (Kind.locDelta m) (ks.map KnotKind.toKind) :=
  fun l => HList.take l

mutual

/-- A filler for the outputs of a node that is not stepped this sweep: the empty increment
for the list-like and slot kinds, the cell's value for the registers (a `sing` has no
increment of its own). No node that is stepped reads it. -/
def Node.fillerLoc (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    HList (Kind.locDelta m) Γ → Node.LocState m n → HList (Kind.locDelta m) κs
  | _, _, .map .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .filterMap .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .broadcast_closed .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .demux .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .values .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .into_keyed .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .kassume_ordering .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .kfold_early_stop .., _, _ => HList.cons (fun _ => SchedKMap.empty) HList.nil
  | _, _, .kget_max_key .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .weaken_retries .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .union .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .assume_ordering .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .fold .., _, s => HList.cons s HList.nil
  | _, _, .fold_monotone .., _, s => HList.cons s HList.nil
  | _, _, .snapshot .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .batch .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .source_interval .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .mapTick .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .firstTick .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .zipTick .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .defer_tick .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .allTicks .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .flattenOrdered .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .flattenUnordered .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .tinto_keyed .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .tassume_ordering .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .kentries .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .kmap .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .kallTicks .., _, _ => HList.cons (fun _ => #[]) HList.nil
  | _, _, .kfold .., _, _ => HList.cons (fun _ => SchedKMap.empty) HList.nil
  | _, _, .ksnapshot .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .tfold .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .latest .., _, s => HList.cons s HList.nil
  | _, _, .latest_monotone .., _, s => HList.cons s HList.nil
  | _, _, .time .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .snapshot_bounded .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .reduce_watermark .., _, _ => HList.cons (fun _ => none) HList.nil
  | _, _, .tick_scan (ℓ := ℓ) _ _ outs _ _ _, _, _ => unzipShapeLoc m ℓ outs (fun _ => none)
  | _, _, .call N args, Δ, s => Dag.fillerLoc m N (args.map (fun v => Δ.get v)) s
  | _, _, .fixN ks body, Δ, s => Dag.fillerLoc m body ((knotFillLoc m ks).append Δ) s

def Dag.fillerLoc (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    HList (Kind.locDelta m) Γ → Dag.LocState m d → HList (Kind.locDelta m) outs
  | _, _, .ret xs, Δ, _ => xs.map (fun v => Δ.get v)
  | _, _, .letN n rest, Δ, s => Dag.fillerLoc m rest ((Node.fillerLoc m n Δ s.1).append Δ) s.2

end

/-! ## The sweep -/

section Sweep
variable [DecidableEq L]

/-- The release half of a strict node: its outputs and cells, a function of its cells alone.
A network node delivers per the oracle and reports no sends; `defer_tick` releases its head
and buffers nothing. Every other node's arm is its step, which the sweep never uses because
it releases strict nodes only. -/
def Node.release (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Oracle m n → HList (Kind.locDelta m) Γ → Node.LocState m n →
    Node.LocState m n × HList (Kind.locDelta m) κs × Node.Sends m n
  | _, _, .broadcast_closed (c := c) _ _, ora, _, s =>
    let cells := memo2 fun i j => (s i j).step (ora i j) #[]
    (fun i j => (cells i j).1,
     HList.cons (fun i => tagConcat (fun j => (cells i j).2)) HList.nil,
     fun _ => #[])
  | _, _, .demux (c := c) _ _ _, ora, _, s =>
    let cells := memo2 fun i j => (s i j).step (ora i j) #[]
    (fun i j => (cells i j).1,
     HList.cons (fun i => tagConcat (fun j => (cells i j).2)) HList.nil,
     fun _ => #[])
  | _, _, .defer_tick _ _, _, _, s =>
    (fun i => if tk then (s i).tail else s i, HList.cons (fun i => if tk then (s i).head? else none) HList.nil, ())
  | _, _, n, ora, Δ, s => Node.locStep m tk n ora Δ s

/-- One sweep over a plain node (neither a call nor a knot block): step it if its inputs are
known, release it if it is strict, skip it otherwise. -/
def Node.sweepPlain (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (tm : Node.Timing m n) (Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ) (s : Node.LocState m n) (part : Node.Part m n) :
    (Node.LocState m n × Node.Part m n) × (HList (Kind.locDelta m) κs × Mask κs × Node.Sends m n) :=
  match part.1 with
  | .done o snd => ((s, part), (o, Mask.const κs true, snd))
  | .released o snd =>
    if Node.inputsKnown n mk then
      let r := Node.pushStep m n Δ s snd
      ((r.1, (.done o r.2, part.2)), (o, Mask.const κs true, r.2))
    else ((s, part), (o, Mask.const κs true, snd))
  | .todo =>
    if Node.inputsKnown n mk then
      let r := Node.locStep m tk n (Node.oracleOf m n tm s) Δ s
      ((r.1, (.done r.2.1 r.2.2, part.2)), (r.2.1, Mask.const κs true, r.2.2))
    else if Node.isStrict n then
      let r := Node.release m tk n (Node.oracleOf m n tm s) Δ s
      ((r.1, (.released r.2.1 r.2.2, part.2)), (r.2.1, Mask.const κs true, r.2.2))
    else ((s, part), (Node.fillerLoc m n Δ s, Mask.const κs false, Node.sendsNil m n))

/-- The result of a knot block's last body sweep `r`, from the knots `(b, Kv)` it was swept
with: the knots that came out known are bound, their increments taken from the sweep, the
ones already bound kept. -/
def knotFinalize {K outs : List (Kind L mem)} {S P Snd : Type}
    (r : (S × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd))
    (b : Mask K) (Kv : HList (Kind.locDelta m) K) :
    (S × (Mask K × HList (Kind.locDelta m) K) × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd) :=
  ((r.1.1, (Mask.or b (HList.take (κs := K) r.2.2.1), Mask.select b Kv (HList.take (κs := K) r.2.1)), r.1.2), r.2)

/-- **The knot loop.** A knot block's body is swept with the knots bound so far (`b`, their
increments `Kv`; the others unknown, fillers); the knots whose outputs came out known are
bound, and the body is swept again, until a sweep binds nothing new or the fuel runs out. A
sweep binds nothing new when every knot was already known, so it was the final sweep that
stepped the `todo` nodes, or when the block is stuck because an external is unknown. On a
valid body with every external known the knots bound grow at every sweep but the last (the
lag analysis's Jacobi iteration, `iter_zero`), therefore a fuel of the knot count, at most
one sweep more than there are knots, reaches the final sweep; a `done` node costs a visit,
not a step. `sw Kv b s part` is one body sweep at the knots `Kv` known per `b`. -/
def knotLoop {K outs : List (Kind L mem)} {S P Snd : Type}
    (sw : HList (Kind.locDelta m) K → Mask K → S → P → (S × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd)) :
    Nat → Mask K → HList (Kind.locDelta m) K → S → P →
    (S × (Mask K × HList (Kind.locDelta m) K) × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd)
  | 0, b, Kv, s, bp => knotFinalize (sw Kv b s bp) b Kv
  | fuel + 1, b, Kv, s, bp =>
    let r := sw Kv b s bp
    let b' := Mask.or b (HList.take (κs := K) r.2.2.1)
    if Mask.beq b' b then knotFinalize r b Kv
    else knotLoop sw fuel b' (Mask.select b Kv (HList.take (κs := K) r.2.1)) r.1.1 r.1.2

mutual

/-- One sweep over a node: step it if its inputs are known, release it if it is strict, skip
it otherwise; walk into callees and knot blocks. Returns the cells and parts, the node's
outputs with their knownness, and its sends (the stored ones for a node not stepped now).
The delivery cursors are read off the cells at the step (`Node.oracleOf`): a node is stepped
or released once, before its cells change. -/
def Node.sweep (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Timing m n → HList (Kind.locDelta m) Γ → Mask Γ → Node.LocState m n → Node.Part m n →
    (Node.LocState m n × Node.Part m n) × (HList (Kind.locDelta m) κs × Mask κs × Node.Sends m n)
  | _, _, .call N args, tm, Δ, mk, s, part =>
    let r := Dag.sweep m tk N tm (args.map (fun v => Δ.get v)) (args.map (fun v => mk.get v)) s part.2
    ((r.1.1, (part.1, r.1.2)), r.2)
  | _, _, .fixN ks body, tm, Δ, mk, s, part =>
    -- the knot loop: the body swept with the knots bound so far, the knots that come out
    -- known bound, until a sweep binds nothing new
    let r := knotLoop (fun Kv b s bp => Dag.sweep m tk body tm (Kv.append Δ) (b.append mk) s bp)
      ks.length part.2.1.1 part.2.1.2 s part.2.2
    ((r.1.1, (part.1, r.1.2)), r.2)
  | _, _, n, tm, Δ, mk, s, part => Node.sweepPlain m tk n tm Δ mk s part

/-- One sweep over a graph. -/
def Dag.sweep (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Timing m d → HList (Kind.locDelta m) Γ → Mask Γ → Dag.LocState m d → Dag.Part m d →
    (Dag.LocState m d × Dag.Part m d) × (HList (Kind.locDelta m) outs × Mask outs × Dag.Sends m d)
  | _, _, .ret xs, _, Δ, mk, s, part => ((s, part), (xs.map (fun v => Δ.get v), xs.map (fun v => mk.get v), ()))
  | _, _, .letN n rest, tm, Δ, mk, s, part =>
    let r := Node.sweep m tk n tm.1 Δ mk s.1 part.1
    -- materialize this node's cells and increments before the rest reads them
    let st := Node.memoState m n r.1.1
    let out := memoEnv m r.2.1
    let r' := Dag.sweep m tk rest tm.2 (out.append Δ) (r.2.2.1.append mk) s.2 part.2
    (((st, r'.1.1), (r.1.2, r'.1.2)), (r'.2.1, r'.2.2.1, (r.2.2.2, r'.2.2.2)))

end

/-- A knot loop ends with some body sweep, finalized. -/
theorem knotLoop_final {K outs : List (Kind L mem)} {S P Snd : Type}
    (sw : HList (Kind.locDelta m) K → Mask K → S → P → (S × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd)) :
    ∀ (fuel : Nat) (b : Mask K) (Kv : HList (Kind.locDelta m) K) (s : S) (bp : P),
      ∃ b₁ Kv₁ s₁ bp₁, knotLoop sw fuel b Kv s bp = knotFinalize (sw Kv₁ b₁ s₁ bp₁) b₁ Kv₁
  | 0, b, Kv, s, bp => ⟨b, Kv, s, bp, rfl⟩
  | fuel + 1, b, Kv, s, bp => by
    unfold knotLoop
    dsimp only
    split
    · exact ⟨b, Kv, s, bp, rfl⟩
    · exact knotLoop_final sw fuel _ _ _ _

/-- The sends a knot loop returns are its last sweep's, hence its parts'. -/
theorem knotLoop_sends {K outs : List (Kind L mem)} {S P Snd : Type}
    (sw : HList (Kind.locDelta m) K → Mask K → S → P → (S × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd))
    (f : P → Snd) (hsw : ∀ Kv b s bp, (sw Kv b s bp).2.2.2 = f (sw Kv b s bp).1.2)
    (fuel : Nat) (b : Mask K) (Kv : HList (Kind.locDelta m) K) (s : S) (bp : P) :
    (knotLoop sw fuel b Kv s bp).2.2.2 = f (knotLoop sw fuel b Kv s bp).1.2.2 := by
  obtain ⟨b₁, Kv₁, s₁, bp₁, e⟩ := knotLoop_final sw fuel b Kv s bp
  rw [e]
  exact hsw _ _ _ _

/-- The sends a sweep returns are its parts'. -/
theorem Node.sweepPlain_sends (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (tm : Node.Timing m n) (Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ) (s : Node.LocState m n) (part : Node.Part m n) :
    (Node.sweepPlain m tk n tm Δ mk s part).2.2.2 = (Node.sweepPlain m tk n tm Δ mk s part).1.2.1.sends (Node.sendsNil m n) := by
  obtain ⟨ph, sub⟩ := part
  cases ph <;> simp only [Node.sweepPlain] <;> (try split) <;> (try split) <;> rfl

mutual

theorem Node.sweep_sends (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (tm : Node.Timing m n) → (Δ : HList (Kind.locDelta m) Γ) → (mk : Mask Γ) → (s : Node.LocState m n) → (part : Node.Part m n) →
    (Node.sweep m tk n tm Δ mk s part).2.2.2 = Node.partSends m n (Node.sweep m tk n tm Δ mk s part).1.2
  | _, _, .call N args, tm, Δ, mk, s, part => Dag.sweep_sends m tk N tm _ _ s part.2
  | _, _, .fixN ks body, tm, Δ, mk, s, part =>
    knotLoop_sends _ _ (fun Kv b s bp => Dag.sweep_sends m tk body tm (Kv.append Δ) (b.append mk) s bp)
      ks.length part.2.1.1 part.2.1.2 s part.2.2
  | _, _, .map .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .filterMap .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .broadcast_closed .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .demux .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .values .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .into_keyed .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .kassume_ordering .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .kfold_early_stop .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .kget_max_key .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .weaken_retries .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .union .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .assume_ordering .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .fold .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .fold_monotone .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .snapshot .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .batch .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .source_interval .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .firstTick .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .mapTick .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .zipTick .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .defer_tick .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .allTicks .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .flattenOrdered .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .flattenUnordered .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .tinto_keyed .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .tassume_ordering .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .reduce_watermark .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .kentries .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .kmap .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .kallTicks .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .kfold .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .ksnapshot .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .tfold .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .latest .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .latest_monotone .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .time .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .snapshot_bounded .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part
  | _, _, .tick_scan .., tm, Δ, mk, s, part => Node.sweepPlain_sends m tk _ tm Δ mk s part

theorem Dag.sweep_sends (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (tm : Dag.Timing m d) → (Δ : HList (Kind.locDelta m) Γ) → (mk : Mask Γ) → (s : Dag.LocState m d) → (part : Dag.Part m d) →
    (Dag.sweep m tk d tm Δ mk s part).2.2.2 = Dag.partSends m d (Dag.sweep m tk d tm Δ mk s part).1.2
  | _, _, .ret _, _, _, _, _, _ => rfl
  | _, _, .letN n rest, tm, Δ, mk, s, part => by
    show (_, _) = (_, _)
    rw [Node.sweep_sends m tk n tm.1 Δ mk s.1 part.1, Dag.sweep_sends m tk rest tm.2 _ _ s.2 part.2]
    rfl

end

/-- **The activation as a sweep**: from fresh parts on a fully known environment, every node
stepped once, the knot blocks' increments found by their knot loops. `Dag.iterate_eq_pass`
says this is `Dag.locStep`. -/
def Dag.sweepTop (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (tm : Dag.Timing m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × Dag.Sends m d :=
  let r := Dag.sweep m tk d tm Δ (Mask.const Γ true) s (Dag.Part.init m d)
  (r.1.1, r.2.1, r.2.2.2)

end Sweep

end Project

end Hydro
