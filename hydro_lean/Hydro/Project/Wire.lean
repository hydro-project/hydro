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
  pending.length`);
- `Dag.quietLoc`: no knot feedback pending at the process's members (the
  settle loop's exit test).
-/

namespace Hydro

namespace Project

open Lean

variable {L : Type} {mem : L → Nat}

/-! ## Codecs along the graph -/

mutual

/-- One codec per network node, along the graph. -/
def Node.Codecs : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (α := α) _ _ _ => Codec α
  | _, _, .demux (α := α) _ _ _ _ => Codec (Nat × α)
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
def hostedList (sel : Sel L mem) (ℓ : L) : List (Hosted sel ℓ) :=
  (List.finRange (mem ℓ)).filterMap fun i => if h : sel ℓ i = true then some ⟨i, h⟩ else none

mutual

/-- A process's sends as messages, numbering network nodes from `base`;
returns the messages and the next free number. -/
def Node.wireEncode (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Codecs n → Nat → Node.Sends sel n → List WireMsg × Nat
  | _, _, .broadcast_closed (c := c) .., cod, base, snd =>
    ((hostedList sel c).flatMap (fun j => (snd j).map (fun v => (base, j.1.val, cod.enc v))), base + 1)
  | _, _, .demux (c := c) .., cod, base, snd =>
    ((hostedList sel c).flatMap (fun j => (snd j).map (fun v => (base, j.1.val, cod.enc v))), base + 1)
  | _, _, .call N _, cod, base, snd => Dag.wireEncode sel N cod base snd
  | _, _, .fixN _ body, cod, base, snd => Dag.wireEncode sel body cod base snd
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

def Dag.wireEncode (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Codecs d → Nat → Dag.Sends sel d → List WireMsg × Nat
  | _, _, .ret _, _, base, _ => ([], base)
  | _, _, .letN n rest, cod, base, snd =>
    let r := Node.wireEncode sel n cod.1 base snd.1
    let r' := Dag.wireEncode sel rest cod.2 r.2 snd.2
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
def Node.wireDecode (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Codecs n → Nat → List WireMsg → Node.Sends sel n × Nat
  | _, _, .broadcast_closed .., cod, base, msgs => (fun j => decodeAt cod msgs base j.1.val, base + 1)
  | _, _, .demux .., cod, base, msgs => (fun j => decodeAt cod msgs base j.1.val, base + 1)
  | _, _, .call N _, cod, base, msgs => Dag.wireDecode sel N cod base msgs
  | _, _, .fixN _ body, cod, base, msgs => Dag.wireDecode sel body cod base msgs
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

def Dag.wireDecode (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Codecs d → Nat → List WireMsg → Dag.Sends sel d × Nat
  | _, _, .ret _, _, base, _ => ((), base)
  | _, _, .letN n rest, cod, base, msgs =>
    let r := Node.wireDecode sel n cod.1 base msgs
    let r' := Dag.wireDecode sel rest cod.2 r.2 msgs
    ((r.1, r'.1), r'.2)

end

/-! ## The online oracle and the quiet test -/

mutual

/-- Deliver everything that has arrived: the cursor is the queue's reach. -/
def Node.oracleAll (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.LocState sel n → Node.Oracle sel n
  | _, _, .broadcast_closed .., s => fun i j => (s i j).delivered + (s i j).pending.length
  | _, _, .demux .., s => fun i j => (s i j).delivered + (s i j).pending.length
  | _, _, .call N _, s => Dag.oracleAll sel N s
  | _, _, .fixN _ body, s => Dag.oracleAll sel body s.1
  | _, _, .map .., _ => ()
  | _, _, .filterMap .., _ => ()
  | _, _, .values .., _ => ()
  | _, _, .into_keyed .., _ => ()
  | _, _, .kassume_ordering .., _ => ()
  | _, _, .kfold_early_stop .., _ => ()
  | _, _, .kget_max_key .., _ => ()
  | _, _, .weaken_retries .., _ => ()
  | _, _, .union .., _ => ()
  | _, _, .assume_ordering .., _ => ()
  | _, _, .fold .., _ => ()
  | _, _, .fold_monotone .., _ => ()
  | _, _, .snapshot .., _ => ()
  | _, _, .batch .., _ => ()
  | _, _, .batch_ordered .., _ => ()
  | _, _, .sample_every .., _ => ()
  | _, _, .timeout_snapshot .., _ => ()
  | _, _, .source_interval_batch .., _ => ()
  | _, _, .mapTick .., _ => ()
  | _, _, .zipTick .., _ => ()
  | _, _, .defer_tick .., _ => ()
  | _, _, .allTicks .., _ => ()
  | _, _, .flattenOrdered .., _ => ()
  | _, _, .flattenUnordered .., _ => ()
  | _, _, .tinto_keyed .., _ => ()
  | _, _, .tassume_ordering .., _ => ()
  | _, _, .reduce_watermark .., _ => ()
  | _, _, .tick_scan .., _ => ()

def Dag.oracleAll (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.LocState sel d → Dag.Oracle sel d
  | _, _, .ret _, _ => ()
  | _, _, .letN n rest, s => (Node.oracleAll sel n s.1, Dag.oracleAll sel rest s.2)

end

/-- No stream-knot feedback pending at the hosted members. A tick knot's pending
feedback is this tick's entry — the next tick's input, present after every tick —
so it is not "work": only stream knots are consulted. -/
def KnotKind.locEmpty (sel : Sel L mem) : (k : KnotKind L mem) → k.toKind.locDelta sel → Bool
  | .stream ℓ .., Δ => (hostedList sel ℓ).all fun i => (Δ i).isEmpty
  | .atomic .., _ => true

def knotsLocEmpty (sel : Sel L mem) : (ks : List (KnotKind L mem)) →
    HList (Kind.locDelta sel) (ks.map KnotKind.toKind) → Bool
  | [], _ => true
  | k :: ks, Δ => KnotKind.locEmpty sel k Δ.1 && knotsLocEmpty sel ks Δ.2

mutual

/-- No stream-knot feedback pending at the process's members (a throughput
hint for the harness — tick again before yielding; nothing in the wired
theorem depends on it). -/
def Node.quietLoc (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.LocState sel n → Bool
  | _, _, .call N _, s => Dag.quietLoc sel N s
  | _, _, .fixN ks body, s => Dag.quietLoc sel body s.1 && knotsLocEmpty sel ks s.2
  | _, _, .map .., _ => true
  | _, _, .filterMap .., _ => true
  | _, _, .broadcast_closed .., _ => true
  | _, _, .demux .., _ => true
  | _, _, .values .., _ => true
  | _, _, .into_keyed .., _ => true
  | _, _, .kassume_ordering .., _ => true
  | _, _, .kfold_early_stop .., _ => true
  | _, _, .kget_max_key .., _ => true
  | _, _, .weaken_retries .., _ => true
  | _, _, .union .., _ => true
  | _, _, .assume_ordering .., _ => true
  | _, _, .fold .., _ => true
  | _, _, .fold_monotone .., _ => true
  | _, _, .snapshot .., _ => true
  | _, _, .batch .., _ => true
  | _, _, .batch_ordered .., _ => true
  | _, _, .sample_every .., _ => true
  | _, _, .timeout_snapshot .., _ => true
  | _, _, .source_interval_batch .., _ => true
  | _, _, .mapTick .., _ => true
  | _, _, .zipTick .., _ => true
  | _, _, .defer_tick .., _ => true
  | _, _, .allTicks .., _ => true
  | _, _, .flattenOrdered .., _ => true
  | _, _, .flattenUnordered .., _ => true
  | _, _, .tinto_keyed .., _ => true
  | _, _, .tassume_ordering .., _ => true
  | _, _, .reduce_watermark .., _ => true
  | _, _, .tick_scan .., _ => true

def Dag.quietLoc (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.LocState sel d → Bool
  | _, _, .ret _, _ => true
  | _, _, .letN n rest, s => Node.quietLoc sel n s.1 && Dag.quietLoc sel rest s.2

end

end Project

end Hydro
