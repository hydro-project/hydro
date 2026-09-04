import Hydro.Project.Wired
import Hydro.Wire

/-!
# Hydro · Project — the wire: sends as messages, the online oracle

What a running process needs besides `Dag.locStep`:

- `Dag.Codecs d`: one payload codec per network node (the elaborator
  builds `M.wireCodecs` for each module from the payload types' `Json`
  instances — `HydroGenProject.lean`);
- `Dag.wireEncode`/`Dag.wireDecode`: a process's `Sends` as a list of
  `(node, sender, payload)` messages and back (network nodes are numbered
  in graph order, callee graphs inline);
- `Dag.oracleAll`: the online oracle — deliver everything that has
  arrived (feasible by construction: it asks for exactly `delivered +
  pending.length`).
-/

namespace Hydro

namespace Project

open Lean

variable {L : Type} {mem : L → Nat} [DecidableEq L]

/-! ## Codecs along the graph -/

mutual

/-- One codec per network node, along the graph. -/
def Node.Codecs : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (α := α) _ _ => Codec α
  | _, _, .demux (α := α) _ _ _ => Codec (Nat × α)
  | _, _, .call N _ => Dag.Codecs N
  | _, _, .fixN _ body => Dag.Codecs body
  | _, _, _ => Unit

def Dag.Codecs : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.Codecs n × Dag.Codecs rest

end

/-- A message on the wire: network node number, sender member, payload. -/
abbrev WireMsg := Nat × Nat × Json

/-- Every member of a location hosted by the process. -/
def hostedList (m : Member L mem) (ℓ : L) : List (Hosted m ℓ) :=
  (List.finRange (mem ℓ)).filterMap fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then some ⟨i, h⟩ else none

mutual

/-- A process's sends as messages, numbering network nodes from `base`;
returns the messages and the next free number. -/
def Node.wireEncode (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Codecs n → Nat → Node.Sends m n → List WireMsg × Nat
  | _, _, .broadcast_closed (c := c) .., cod, base, snd =>
    ((hostedList m c).flatMap (fun j => (snd j).map (fun v => (base, j.1.val, cod.enc v))), base + 1)
  | _, _, .demux (c := c) .., cod, base, snd =>
    ((hostedList m c).flatMap (fun j => (snd j).map (fun v => (base, j.1.val, cod.enc v))), base + 1)
  | _, _, .call N _, cod, base, snd => Dag.wireEncode m N cod base snd
  | _, _, .fixN _ body, cod, base, snd => Dag.wireEncode m body cod base snd
  | _, _, .map .., _, base, _ => ([], base)
  | _, _, .filterMap .., _, base, _ => ([], base)
  | _, _, .values .., _, base, _ => ([], base)
  | _, _, .into_keyed .., _, base, _ => ([], base)
  | _, _, .kassume_ordering .., _, base, _ => ([], base)
  | _, _, .kfold_early_stop .., _, base, _ => ([], base)
  | _, _, .kget_max_key .., _, base, _ => ([], base)
  | _, _, .weaken_retries .., _, base, _ => ([], base)
  | _, _, .union .., _, base, _ => ([], base)
  | _, _, .assume_ordering .., _, base, _ => ([], base)
  | _, _, .fold .., _, base, _ => ([], base)
  | _, _, .fold_monotone .., _, base, _ => ([], base)
  | _, _, .snapshot .., _, base, _ => ([], base)
  | _, _, .batch .., _, base, _ => ([], base)
  | _, _, .batch_ordered .., _, base, _ => ([], base)
  | _, _, .sample_every .., _, base, _ => ([], base)
  | _, _, .timeout_snapshot .., _, base, _ => ([], base)
  | _, _, .source_interval_batch .., _, base, _ => ([], base)
  | _, _, .mapTick .., _, base, _ => ([], base)
  | _, _, .zipTick .., _, base, _ => ([], base)
  | _, _, .defer_tick .., _, base, _ => ([], base)
  | _, _, .allTicks .., _, base, _ => ([], base)
  | _, _, .flattenOrdered .., _, base, _ => ([], base)
  | _, _, .flattenUnordered .., _, base, _ => ([], base)
  | _, _, .tinto_keyed .., _, base, _ => ([], base)
  | _, _, .tassume_ordering .., _, base, _ => ([], base)
  | _, _, .reduce_watermark .., _, base, _ => ([], base)
  | _, _, .tick_scan .., _, base, _ => ([], base)

def Dag.wireEncode (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Codecs d → Nat → Dag.Sends m d → List WireMsg × Nat
  | _, _, .ret _, _, base, _ => ([], base)
  | _, _, .letN n rest, cod, base, snd =>
    let r := Node.wireEncode m n cod.1 base snd.1
    let r' := Dag.wireEncode m rest cod.2 r.2 snd.2
    (r.1 ++ r'.1, r'.2)

end

mutual

/-- The network nodes in wire order with their receiver location (the routing table a
harness uses to send a node's messages only to the processes hosting receivers). -/
def Node.wireReceivers : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Nat → List (Nat × L) × Nat
  | _, _, .broadcast_closed (p := p) .., base => ([(base, p)], base + 1)
  | _, _, .demux (p := p) .., base => ([(base, p)], base + 1)
  | _, _, .call N _, base => Dag.wireReceivers N base
  | _, _, .fixN _ body, base => Dag.wireReceivers body base
  | _, _, _, base => ([], base)

def Dag.wireReceivers : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Nat → List (Nat × L) × Nat
  | _, _, .ret _, base => ([], base)
  | _, _, .letN n rest, base =>
    let r := Node.wireReceivers n base
    let r' := Dag.wireReceivers rest r.2
    (r.1 ++ r'.1, r'.2)

end

/-- The messages of node `k` from sender `j`, decoded (undecodable ones dropped). -/
def decodeAt {α : Type} (cod : Codec α) (msgs : List WireMsg) (k : Nat) (j : Nat) : List α :=
  msgs.filterMap fun m => if m.1 = k ∧ m.2.1 = j then (cod.dec m.2.2).toOption else none

mutual

/-- Messages back into a process's `Sends`. -/
def Node.wireDecode (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Codecs n → Nat → List WireMsg → Node.Sends m n × Nat
  | _, _, .broadcast_closed .., cod, base, msgs => (fun j => decodeAt cod msgs base j.1.val, base + 1)
  | _, _, .demux .., cod, base, msgs => (fun j => decodeAt cod msgs base j.1.val, base + 1)
  | _, _, .call N _, cod, base, msgs => Dag.wireDecode m N cod base msgs
  | _, _, .fixN _ body, cod, base, msgs => Dag.wireDecode m body cod base msgs
  | _, _, .map .., _, base, _ => ((), base)
  | _, _, .filterMap .., _, base, _ => ((), base)
  | _, _, .values .., _, base, _ => ((), base)
  | _, _, .into_keyed .., _, base, _ => ((), base)
  | _, _, .kassume_ordering .., _, base, _ => ((), base)
  | _, _, .kfold_early_stop .., _, base, _ => ((), base)
  | _, _, .kget_max_key .., _, base, _ => ((), base)
  | _, _, .weaken_retries .., _, base, _ => ((), base)
  | _, _, .union .., _, base, _ => ((), base)
  | _, _, .assume_ordering .., _, base, _ => ((), base)
  | _, _, .fold .., _, base, _ => ((), base)
  | _, _, .fold_monotone .., _, base, _ => ((), base)
  | _, _, .snapshot .., _, base, _ => ((), base)
  | _, _, .batch .., _, base, _ => ((), base)
  | _, _, .batch_ordered .., _, base, _ => ((), base)
  | _, _, .sample_every .., _, base, _ => ((), base)
  | _, _, .timeout_snapshot .., _, base, _ => ((), base)
  | _, _, .source_interval_batch .., _, base, _ => ((), base)
  | _, _, .mapTick .., _, base, _ => ((), base)
  | _, _, .zipTick .., _, base, _ => ((), base)
  | _, _, .defer_tick .., _, base, _ => ((), base)
  | _, _, .allTicks .., _, base, _ => ((), base)
  | _, _, .flattenOrdered .., _, base, _ => ((), base)
  | _, _, .flattenUnordered .., _, base, _ => ((), base)
  | _, _, .tinto_keyed .., _, base, _ => ((), base)
  | _, _, .tassume_ordering .., _, base, _ => ((), base)
  | _, _, .reduce_watermark .., _, base, _ => ((), base)
  | _, _, .tick_scan .., _, base, _ => ((), base)

def Dag.wireDecode (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Codecs d → Nat → List WireMsg → Dag.Sends m d × Nat
  | _, _, .ret _, _, base, _ => ((), base)
  | _, _, .letN n rest, cod, base, msgs =>
    let r := Node.wireDecode m n cod.1 base msgs
    let r' := Dag.wireDecode m rest cod.2 r.2 msgs
    ((r.1, r'.1), r'.2)

end

/-! ## The online oracle and the quiet test -/

mutual

/-- Deliver everything that has arrived: the cursor is the queue's reach; the timers
read the schedule's verdicts. -/
def Node.oracleAll (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Sched n → Node.LocState m n → Node.Oracle m n
  | _, _, .broadcast_closed .., _, s => fun i j => (s i j).delivered + (s i j).pending.length
  | _, _, .demux .., _, s => fun i j => (s i j).delivered + (s i j).pending.length
  | _, _, .sample_every .., d, _ => fun i => d i.1
  | _, _, .timeout_snapshot .., d, _ => fun i => d i.1
  | _, _, .source_interval_batch .., d, _ => fun i => d i.1
  | _, _, .call N _, sch, s => Dag.oracleAll m N sch s
  | _, _, .fixN _ body, sch, s => Dag.oracleAll m body sch s.1
  | _, _, .map .., _, _ => ()
  | _, _, .filterMap .., _, _ => ()
  | _, _, .values .., _, _ => ()
  | _, _, .into_keyed .., _, _ => ()
  | _, _, .kassume_ordering .., _, _ => ()
  | _, _, .kfold_early_stop .., _, _ => ()
  | _, _, .kget_max_key .., _, _ => ()
  | _, _, .weaken_retries .., _, _ => ()
  | _, _, .union .., _, _ => ()
  | _, _, .assume_ordering .., _, _ => ()
  | _, _, .fold .., _, _ => ()
  | _, _, .fold_monotone .., _, _ => ()
  | _, _, .snapshot .., _, _ => ()
  | _, _, .batch .., _, _ => ()
  | _, _, .batch_ordered .., _, _ => ()
  | _, _, .mapTick .., _, _ => ()
  | _, _, .zipTick .., _, _ => ()
  | _, _, .defer_tick .., _, _ => ()
  | _, _, .allTicks .., _, _ => ()
  | _, _, .flattenOrdered .., _, _ => ()
  | _, _, .flattenUnordered .., _, _ => ()
  | _, _, .tinto_keyed .., _, _ => ()
  | _, _, .tassume_ordering .., _, _ => ()
  | _, _, .reduce_watermark .., _, _ => ()
  | _, _, .tick_scan .., _, _ => ()

def Dag.oracleAll (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Sched d → Dag.LocState m d → Dag.Oracle m d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, sch, s => (Node.oracleAll m n sch.1 s.1, Dag.oracleAll m rest sch.2 s.2)

end

end Project

end Hydro
