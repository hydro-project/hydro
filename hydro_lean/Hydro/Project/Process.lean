import Hydro.Project.Wire
import Hydro.Project.Sweep
import Hydro.Project.Wired

/-!
# Hydro · Project — the process's loop body

This file defines what a deployed process runs per activation, as one library function. Its
counterpart, `Project/Model/DistributedModel.lean`, defines the model of many processes
running together and proves that the model is this function iterated.

An activation takes the messages that arrived since the last one (`Dag.Inbox`: per network
node, per receiving end, per sender, a per-connection FIFO), appends them to the queues
(`Dag.enqueueInbox`), delivers everything that has arrived at this activation's timing, runs
the sweep of the graph (`Dag.sweepTop`), and hands back the new cells, the outputs and the
sends. The sends, to every peer and to itself, go only to the network; the
activation does not enqueue them. This is `Dag.iterate`, the function the processes run and
the theorems are about. `Dag.pass` is one lock-step step of the same process, `Dag.locStep`
at the online oracle; `Dag.iterate_eq_pass` says the two agree. At the wire the activation is
`Dag.iterateWire`: the messages that arrived, per connection, decoded in one walk
(`Dag.inboxOfWire`), the body, the sends encoded. At the bytes it is `Dag.iterateBytes`:
per connection the unread bytes, parsed into frames (`parseFrames`), the unparsed tails
returned. The TCP shell's only job is to hand it the bytes in arrival order and put its
frames on the connections.

The bridge to the lock-step machine belongs to the proof layer: a lock-step cell at a process
is its loop cell plus the network's in-flight queue toward it (`Dag.arrivedOnly` drops that
queue). The loop body fed the cursor's next messages (`Dag.arrivalsOf`) from the loop cells
computes the lock-step step's outputs and sends, and the loop cells of its result
(`Dag.pass_locStep`). `Project/Model/DistributedModel.lean` lifts this to a whole
interleaving.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} [DecidableEq L]

/-! ## The inbox -/

mutual

/-- The messages that arrived at the member's receiving ends since the last activation: per
network node, per receiving end, per sender. These are the receiver-side items; a `demux` is
already filtered by address. -/
def Node.Inbox (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (p := p) (α := α) _ _ => Hosted m p → Fin (mem c) → Array α
  | _, _, .demux (c := c) (p := p) (α := α) _ _ _ => Hosted m p → Fin (mem c) → Array α
  | _, _, .call N _ => Dag.Inbox m N
  | _, _, .fixN _ body => Dag.Inbox m body
  | _, _, _ => Unit

def Dag.Inbox (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.Inbox m n × Dag.Inbox m rest

end

mutual

/-- Append the inbox to the queues. -/
def Node.enqueueInbox (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Inbox m n → Node.LocState m n → Node.LocState m n
  | _, _, .broadcast_closed .., ib, s => fun i j => (s i j).enqueue (ib i j)
  | _, _, .demux .., ib, s => fun i j => (s i j).enqueue (ib i j)
  | _, _, .call N _, ib, s => Dag.enqueueInbox m N ib s
  | _, _, .fixN _ body, ib, s => Dag.enqueueInbox m body ib s
  | _, _, .map .., _, s => s
  | _, _, .filterMap .., _, s => s
  | _, _, .values .., _, s => s
  | _, _, .into_keyed .., _, s => s
  | _, _, .kassume_ordering .., _, s => s
  | _, _, .kfold_early_stop .., _, s => s
  | _, _, .kget_max_key .., _, s => s
  | _, _, .weaken_retries .., _, s => s
  | _, _, .union .., _, s => s
  | _, _, .assume_ordering .., _, s => s
  | _, _, .fold .., _, s => s
  | _, _, .fold_monotone .., _, s => s
  | _, _, .snapshot .., _, s => s
  | _, _, .batch .., _, s => s
  | _, _, .source_interval .., _, s => s
  | _, _, .mapTick .., _, s => s
  | _, _, .firstTick .., _, s => s
  | _, _, .zipTick .., _, s => s
  | _, _, .defer_tick .., _, s => s
  | _, _, .allTicks .., _, s => s
  | _, _, .flattenOrdered .., _, s => s
  | _, _, .flattenUnordered .., _, s => s
  | _, _, .tinto_keyed .., _, s => s
  | _, _, .tassume_ordering .., _, s => s
  | _, _, .kentries .., _, s => s
  | _, _, .kmap .., _, s => s
  | _, _, .kallTicks .., _, s => s
  | _, _, .kfold .., _, s => s
  | _, _, .ksnapshot .., _, s => s
  | _, _, .tfold .., _, s => s
  | _, _, .latest .., _, s => s
  | _, _, .latest_monotone .., _, s => s
  | _, _, .time .., _, s => s
  | _, _, .snapshot_bounded .., _, s => s
  | _, _, .reduce_watermark .., _, s => s
  | _, _, .tick_scan .., _, s => s

def Dag.enqueueInbox (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Inbox m d → Dag.LocState m d → Dag.LocState m d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, ib, s => (Node.enqueueInbox m n ib.1 s.1, Dag.enqueueInbox m rest ib.2 s.2)

end

mutual

/-- The inbox touches only the queues: the timing read at the cells' tick counts is
unchanged by it. -/
theorem Node.timingAt_enqueueInbox (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (sch : Node.Sched n) → (ib : Node.Inbox m n) → (s : Node.LocState m n) →
    Node.timingAt m n sch (Node.enqueueInbox m n ib s) = Node.timingAt m n sch s
  | _, _, .call N _, sch, ib, s => Dag.timingAt_enqueueInbox m N sch ib s
  | _, _, .fixN _ body, sch, ib, s => Dag.timingAt_enqueueInbox m body sch ib s
  | _, _, .broadcast_closed .., _, _, _ => rfl
  | _, _, .demux .., _, _, _ => rfl
  | _, _, .map .., _, _, _ => rfl
  | _, _, .filterMap .., _, _, _ => rfl
  | _, _, .values .., _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _ => rfl
  | _, _, .union .., _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _ => rfl
  | _, _, .fold .., _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _ => rfl
  | _, _, .snapshot .., _, _, _ => rfl
  | _, _, .batch .., _, _, _ => rfl
  | _, _, .source_interval .., _, _, _ => rfl
  | _, _, .mapTick .., _, _, _ => rfl
  | _, _, .firstTick .., _, _, _ => rfl
  | _, _, .zipTick .., _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _ => rfl
  | _, _, .allTicks .., _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _ => rfl
  | _, _, .kentries .., _, _, _ => rfl
  | _, _, .kmap .., _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _ => rfl
  | _, _, .kfold .., _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _ => rfl
  | _, _, .tfold .., _, _, _ => rfl
  | _, _, .latest .., _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _ => rfl
  | _, _, .time .., _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _ => rfl

theorem Dag.timingAt_enqueueInbox (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (sch : Dag.Sched d) → (ib : Dag.Inbox m d) → (s : Dag.LocState m d) →
    Dag.timingAt m d sch (Dag.enqueueInbox m d ib s) = Dag.timingAt m d sch s
  | _, _, .ret _, _, _, _ => rfl
  | _, _, .letN n rest, sch, ib, s => by
    show (Node.timingAt m n sch.1 (Node.enqueueInbox m n ib.1 s.1), Dag.timingAt m rest sch.2 (Dag.enqueueInbox m rest ib.2 s.2))
      = (Node.timingAt m n sch.1 s.1, Dag.timingAt m rest sch.2 s.2)
    rw [Node.timingAt_enqueueInbox m n sch.1 ib.1 s.1, Dag.timingAt_enqueueInbox m rest sch.2 ib.2 s.2]

end

mutual

/-- The loop cells: a queue holds nothing in flight. What has arrived was delivered and
counted; the network holds what is in flight. -/
def Node.arrivedOnly (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.LocState m n → Node.LocState m n
  | _, _, .broadcast_closed .., s => fun i j => ⟨#[], (s i j).delivered⟩
  | _, _, .demux .., s => fun i j => ⟨#[], (s i j).delivered⟩
  | _, _, .call N _, s => Dag.arrivedOnly m N s
  | _, _, .fixN _ body, s => Dag.arrivedOnly m body s
  | _, _, .map .., s => s
  | _, _, .filterMap .., s => s
  | _, _, .values .., s => s
  | _, _, .into_keyed .., s => s
  | _, _, .kassume_ordering .., s => s
  | _, _, .kfold_early_stop .., s => s
  | _, _, .kget_max_key .., s => s
  | _, _, .weaken_retries .., s => s
  | _, _, .union .., s => s
  | _, _, .assume_ordering .., s => s
  | _, _, .fold .., s => s
  | _, _, .fold_monotone .., s => s
  | _, _, .snapshot .., s => s
  | _, _, .batch .., s => s
  | _, _, .source_interval .., s => s
  | _, _, .mapTick .., s => s
  | _, _, .firstTick .., s => s
  | _, _, .zipTick .., s => s
  | _, _, .defer_tick .., s => s
  | _, _, .allTicks .., s => s
  | _, _, .flattenOrdered .., s => s
  | _, _, .flattenUnordered .., s => s
  | _, _, .tinto_keyed .., s => s
  | _, _, .tassume_ordering .., s => s
  | _, _, .kentries .., s => s
  | _, _, .kmap .., s => s
  | _, _, .kallTicks .., s => s
  | _, _, .kfold .., s => s
  | _, _, .ksnapshot .., s => s
  | _, _, .tfold .., s => s
  | _, _, .latest .., s => s
  | _, _, .latest_monotone .., s => s
  | _, _, .time .., s => s
  | _, _, .snapshot_bounded .., s => s
  | _, _, .reduce_watermark .., s => s
  | _, _, .tick_scan .., s => s

def Dag.arrivedOnly (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.LocState m d → Dag.LocState m d
  | _, _, .ret _, _ => ()
  | _, _, .letN n rest, s => (Node.arrivedOnly m n s.1, Dag.arrivedOnly m rest s.2)

end

mutual

/-- The messages the cursor delivers next, read off the lock-step cells: the in-flight prefix
up to the cursor, clamped to what is in flight. -/
def Node.arrivalsOf (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Oracle m n → Node.LocState m n → Node.Inbox m n
  | _, _, .broadcast_closed .., ora, s => fun i j => (s i j).pending.extract 0 (ora i j - (s i j).delivered)
  | _, _, .demux .., ora, s => fun i j => (s i j).pending.extract 0 (ora i j - (s i j).delivered)
  | _, _, .call N _, ora, s => Dag.arrivalsOf m N ora s
  | _, _, .fixN _ body, ora, s => Dag.arrivalsOf m body ora s
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
  | _, _, .source_interval .., _, _ => ()
  | _, _, .mapTick .., _, _ => ()
  | _, _, .firstTick .., _, _ => ()
  | _, _, .zipTick .., _, _ => ()
  | _, _, .defer_tick .., _, _ => ()
  | _, _, .allTicks .., _, _ => ()
  | _, _, .flattenOrdered .., _, _ => ()
  | _, _, .flattenUnordered .., _, _ => ()
  | _, _, .tinto_keyed .., _, _ => ()
  | _, _, .tassume_ordering .., _, _ => ()
  | _, _, .kentries .., _, _ => ()
  | _, _, .kmap .., _, _ => ()
  | _, _, .kallTicks .., _, _ => ()
  | _, _, .kfold .., _, _ => ()
  | _, _, .ksnapshot .., _, _ => ()
  | _, _, .tfold .., _, _ => ()
  | _, _, .latest .., _, _ => ()
  | _, _, .latest_monotone .., _, _ => ()
  | _, _, .time .., _, _ => ()
  | _, _, .snapshot_bounded .., _, _ => ()
  | _, _, .reduce_watermark .., _, _ => ()
  | _, _, .tick_scan .., _, _ => ()

def Dag.arrivalsOf (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Oracle m d → Dag.LocState m d → Dag.Inbox m d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, ora, s => (Node.arrivalsOf m n ora.1 s.1, Dag.arrivalsOf m rest ora.2 s.2)

end

mutual

/-- A sender process's sends, as this process's inbox; a `demux` is filtered by address. -/
def Node.inboxOfSends (m src : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Sends src n → Node.Inbox m n
  | _, _, .broadcast_closed (c := c) .., snd => fun _ j => if h : (⟨c, j⟩ : Member L mem) = src then snd ⟨j, h⟩ else #[]
  | _, _, .demux (c := c) _ _ addr, snd => fun i j =>
    if h : (⟨c, j⟩ : Member L mem) = src then filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (snd ⟨j, h⟩) else #[]
  | _, _, .call N _, snd => Dag.inboxOfSends m src N snd
  | _, _, .fixN _ body, snd => Dag.inboxOfSends m src body snd
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
  | _, _, .source_interval .., _ => ()
  | _, _, .mapTick .., _ => ()
  | _, _, .firstTick .., _ => ()
  | _, _, .zipTick .., _ => ()
  | _, _, .defer_tick .., _ => ()
  | _, _, .allTicks .., _ => ()
  | _, _, .flattenOrdered .., _ => ()
  | _, _, .flattenUnordered .., _ => ()
  | _, _, .tinto_keyed .., _ => ()
  | _, _, .tassume_ordering .., _ => ()
  | _, _, .kentries .., _ => ()
  | _, _, .kmap .., _ => ()
  | _, _, .kallTicks .., _ => ()
  | _, _, .kfold .., _ => ()
  | _, _, .ksnapshot .., _ => ()
  | _, _, .tfold .., _ => ()
  | _, _, .latest .., _ => ()
  | _, _, .latest_monotone .., _ => ()
  | _, _, .time .., _ => ()
  | _, _, .snapshot_bounded .., _ => ()
  | _, _, .reduce_watermark .., _ => ()
  | _, _, .tick_scan .., _ => ()

def Dag.inboxOfSends (m src : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Sends src d → Dag.Inbox m d
  | _, _, .ret _, _ => ()
  | _, _, .letN n rest, snd => (Node.inboxOfSends m src n snd.1, Dag.inboxOfSends m src rest snd.2)

end

mutual

/-- Enqueueing a sender's sends is enqueueing the inbox they make. -/
theorem Node.enqueueLoc_eq (m src : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (snd : Node.Sends src n) → (s : Node.LocState m n) →
    Node.enqueueLoc m src n snd s = Node.enqueueInbox m n (Node.inboxOfSends m src n snd) s
  | _, _, .broadcast_closed .., snd, s => by
    funext i j
    simp only [Node.enqueueLoc, Node.enqueueInbox, Node.inboxOfSends]
    split <;> simp [ChanCell.enqueue]
  | _, _, .demux .., snd, s => by
    funext i j
    simp only [Node.enqueueLoc, Node.enqueueInbox, Node.inboxOfSends]
    split <;> simp [ChanCell.enqueue]
  | _, _, .call N _, snd, s => Dag.enqueueLoc_eq m src N snd s
  | _, _, .fixN _ body, snd, s => Dag.enqueueLoc_eq m src body snd s
  | _, _, .map .., _, _ => rfl
  | _, _, .filterMap .., _, _ => rfl
  | _, _, .values .., _, _ => rfl
  | _, _, .into_keyed .., _, _ => rfl
  | _, _, .kassume_ordering .., _, _ => rfl
  | _, _, .kfold_early_stop .., _, _ => rfl
  | _, _, .kget_max_key .., _, _ => rfl
  | _, _, .weaken_retries .., _, _ => rfl
  | _, _, .union .., _, _ => rfl
  | _, _, .assume_ordering .., _, _ => rfl
  | _, _, .fold .., _, _ => rfl
  | _, _, .fold_monotone .., _, _ => rfl
  | _, _, .snapshot .., _, _ => rfl
  | _, _, .batch .., _, _ => rfl
  | _, _, .source_interval .., _, _ => rfl
  | _, _, .mapTick .., _, _ => rfl
  | _, _, .firstTick .., _, _ => rfl
  | _, _, .zipTick .., _, _ => rfl
  | _, _, .defer_tick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl
  | _, _, .kentries .., _, _ => rfl
  | _, _, .kmap .., _, _ => rfl
  | _, _, .kallTicks .., _, _ => rfl
  | _, _, .kfold .., _, _ => rfl
  | _, _, .ksnapshot .., _, _ => rfl
  | _, _, .tfold .., _, _ => rfl
  | _, _, .latest .., _, _ => rfl
  | _, _, .latest_monotone .., _, _ => rfl
  | _, _, .time .., _, _ => rfl
  | _, _, .snapshot_bounded .., _, _ => rfl
  | _, _, .reduce_watermark .., _, _ => rfl
  | _, _, .tick_scan .., _, _ => rfl

theorem Dag.enqueueLoc_eq (m src : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (snd : Dag.Sends src d) → (s : Dag.LocState m d) →
    Dag.enqueueLoc m src d snd s = Dag.enqueueInbox m d (Dag.inboxOfSends m src d snd) s
  | _, _, .ret _, _, _ => rfl
  | _, _, .letN n rest, snd, s => by
    show (Node.enqueueLoc m src n snd.1 s.1, Dag.enqueueLoc m src rest snd.2 s.2) = _
    rw [Node.enqueueLoc_eq, Dag.enqueueLoc_eq]
    rfl

end

/-! ## The loop body -/

mutual

/-- The empty inbox. -/
def Node.Inbox.empty (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Inbox m n
  | _, _, .broadcast_closed .. => fun _ _ => #[]
  | _, _, .demux .. => fun _ _ => #[]
  | _, _, .call N _ => Dag.Inbox.empty m N
  | _, _, .fixN _ body => Dag.Inbox.empty m body
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

def Dag.Inbox.empty (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Inbox m d
  | _, _, .ret _ => ()
  | _, _, .letN n rest => (Node.Inbox.empty m n, Dag.Inbox.empty m rest)

end

mutual

/-- Two inboxes, the second's messages after the first's, per connection. -/
def Node.Inbox.append (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Inbox m n → Node.Inbox m n → Node.Inbox m n
  | _, _, .broadcast_closed .., a, b => fun i j => a i j ++ b i j
  | _, _, .demux .., a, b => fun i j => a i j ++ b i j
  | _, _, .call N _, a, b => Dag.Inbox.append m N a b
  | _, _, .fixN _ body, a, b => Dag.Inbox.append m body a b
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
  | _, _, .source_interval .., _, _ => ()
  | _, _, .mapTick .., _, _ => ()
  | _, _, .firstTick .., _, _ => ()
  | _, _, .zipTick .., _, _ => ()
  | _, _, .defer_tick .., _, _ => ()
  | _, _, .allTicks .., _, _ => ()
  | _, _, .flattenOrdered .., _, _ => ()
  | _, _, .flattenUnordered .., _, _ => ()
  | _, _, .tinto_keyed .., _, _ => ()
  | _, _, .tassume_ordering .., _, _ => ()
  | _, _, .kentries .., _, _ => ()
  | _, _, .kmap .., _, _ => ()
  | _, _, .kallTicks .., _, _ => ()
  | _, _, .kfold .., _, _ => ()
  | _, _, .ksnapshot .., _, _ => ()
  | _, _, .tfold .., _, _ => ()
  | _, _, .latest .., _, _ => ()
  | _, _, .latest_monotone .., _, _ => ()
  | _, _, .time .., _, _ => ()
  | _, _, .snapshot_bounded .., _, _ => ()
  | _, _, .reduce_watermark .., _, _ => ()
  | _, _, .tick_scan .., _, _ => ()

def Dag.Inbox.append (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Inbox m d → Dag.Inbox m d → Dag.Inbox m d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, a, b => (Node.Inbox.append m n a.1 b.1, Dag.Inbox.append m rest a.2 b.2)

end

/-- The inbox the shell hands over: the sends it decoded from its sockets, `(sender, its
sends)`, one entry per message batch, in arrival order, as this process's inbox. -/
def Dag.inboxOf (m : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (inbox : List ((src : Member L mem) × Dag.Sends src d)) : Dag.Inbox m d :=
  inbox.foldl (fun ib ⟨src, snd⟩ => Dag.Inbox.append m d ib (Dag.inboxOfSends m src d snd)) (Dag.Inbox.empty m d)

/-! ### The inbox straight from the wire, in one walk

`Dag.inboxOf` of the decoded batches walks the graph three times per batch (`wireDecode`,
`inboxOfSends`, `Inbox.append`). `Dag.inboxOfWire` walks once: at each network node
(numbered in wire order as `wireDecode` numbers them) the inbox is the batches' messages for
that node and sender, decoded, in arrival order. `inboxOfWire_eq` says that the one walk
computes the same inbox as `Dag.inboxOf`. -/

mutual

/-- A node's inbox from the raw batches `(sender, its messages)`, numbering network nodes
from `base`. -/
def Node.inboxOfWire (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Codecs n → Nat → List ((src : Member L mem) × List WireMsg) → Node.Inbox m n × Nat
  | _, _, .broadcast_closed (c := c) .., cod, base, batches =>
    (fun _ j => appendFold (fun b =>
      if (⟨c, j⟩ : Member L mem) = b.1 then decodeAt cod b.2 base j.val else #[]) batches, base + 1)
  | _, _, .demux (c := c) _ _ addr, cod, base, batches =>
    (fun i j => appendFold (fun b =>
      if (⟨c, j⟩ : Member L mem) = b.1 then
        filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod b.2 base j.val)
      else #[]) batches, base + 1)
  | _, _, .call N _, cod, base, batches => Dag.inboxOfWire m N cod base batches
  | _, _, .fixN _ body, cod, base, batches => Dag.inboxOfWire m body cod base batches
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
  | _, _, .source_interval .., _, base, _ => ((), base)
  | _, _, .mapTick .., _, base, _ => ((), base)
  | _, _, .firstTick .., _, base, _ => ((), base)
  | _, _, .zipTick .., _, base, _ => ((), base)
  | _, _, .defer_tick .., _, base, _ => ((), base)
  | _, _, .allTicks .., _, base, _ => ((), base)
  | _, _, .flattenOrdered .., _, base, _ => ((), base)
  | _, _, .flattenUnordered .., _, base, _ => ((), base)
  | _, _, .tinto_keyed .., _, base, _ => ((), base)
  | _, _, .tassume_ordering .., _, base, _ => ((), base)
  | _, _, .kentries .., _, base, _ => ((), base)
  | _, _, .kmap .., _, base, _ => ((), base)
  | _, _, .kallTicks .., _, base, _ => ((), base)
  | _, _, .kfold .., _, base, _ => ((), base)
  | _, _, .ksnapshot .., _, base, _ => ((), base)
  | _, _, .tfold .., _, base, _ => ((), base)
  | _, _, .latest .., _, base, _ => ((), base)
  | _, _, .latest_monotone .., _, base, _ => ((), base)
  | _, _, .time .., _, base, _ => ((), base)
  | _, _, .snapshot_bounded .., _, base, _ => ((), base)
  | _, _, .reduce_watermark .., _, base, _ => ((), base)
  | _, _, .tick_scan .., _, base, _ => ((), base)

def Dag.inboxOfWire (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Codecs d → Nat → List ((src : Member L mem) × List WireMsg) → Dag.Inbox m d × Nat
  | _, _, .ret _, _, base, _ => ((), base)
  | _, _, .letN n rest, cod, base, batches =>
    let r := Node.inboxOfWire m n cod.1 base batches
    let r' := Dag.inboxOfWire m rest cod.2 r.2 batches
    ((r.1, r'.1), r'.2)

end

mutual

/-- `Dag.inboxOf` of one batch's decoded sends: the per-connection lists of that batch. -/
theorem Node.inboxOfSends_wireDecode (m src : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) → (msgs : List WireMsg) →
    Node.inboxOfSends m src n (Node.wireDecode src n cod base msgs).1
      = (Node.inboxOfWire m n cod base [⟨src, msgs⟩]).1
    ∧ (Node.wireDecode src n cod base msgs).2 = (Node.inboxOfWire m n cod base [⟨src, msgs⟩]).2
  | _, _, .broadcast_closed (c := c) .., cod, base, msgs => ⟨funext fun _ => funext fun j => by
      show (if h : (⟨c, j⟩ : Member L mem) = src then _ else #[])
        = appendFold (fun b : (src : Member L mem) × List WireMsg => if (⟨c, j⟩ : Member L mem) = b.1 then decodeAt cod b.2 base j.val else #[])
          [⟨src, msgs⟩]
      rw [appendFold_singleton]; split <;> rfl, rfl⟩
  | _, _, .demux (c := c) _ _ addr, cod, base, msgs => ⟨funext fun i => funext fun j => by
      show (if h : (⟨c, j⟩ : Member L mem) = src then _ else #[])
        = appendFold (fun b : (src : Member L mem) × List WireMsg => if (⟨c, j⟩ : Member L mem) = b.1 then
            filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod b.2 base j.val) else #[])
          [⟨src, msgs⟩]
      rw [appendFold_singleton]; split <;> rfl, rfl⟩
  | _, _, .call N _, cod, base, msgs => Dag.inboxOfSends_wireDecode m src N cod base msgs
  | _, _, .fixN _ body, cod, base, msgs => Dag.inboxOfSends_wireDecode m src body cod base msgs
  | _, _, .map .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .filterMap .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .values .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .into_keyed .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .kassume_ordering .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .kfold_early_stop .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .kget_max_key .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .weaken_retries .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .union .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .assume_ordering .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .fold .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .fold_monotone .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .snapshot .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .batch .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .source_interval .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .mapTick .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .firstTick .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .zipTick .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .defer_tick .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .allTicks .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .flattenOrdered .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .flattenUnordered .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .tinto_keyed .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .tassume_ordering .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .kentries .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .kmap .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .kallTicks .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .kfold .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .ksnapshot .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .tfold .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .latest .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .latest_monotone .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .time .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .snapshot_bounded .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .reduce_watermark .., _, _, _ => ⟨rfl, rfl⟩
  | _, _, .tick_scan .., _, _, _ => ⟨rfl, rfl⟩

theorem Dag.inboxOfSends_wireDecode (m src : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) → (msgs : List WireMsg) →
    Dag.inboxOfSends m src d (Dag.wireDecode src d cod base msgs).1
      = (Dag.inboxOfWire m d cod base [⟨src, msgs⟩]).1
    ∧ (Dag.wireDecode src d cod base msgs).2 = (Dag.inboxOfWire m d cod base [⟨src, msgs⟩]).2
  | _, _, .ret _, _, _, _ => ⟨rfl, rfl⟩
  | _, _, .letN n rest, cod, base, msgs => by
    obtain ⟨h1, h2⟩ := Node.inboxOfSends_wireDecode m src n cod.1 base msgs
    show (Node.inboxOfSends m src n (Node.wireDecode src n cod.1 base msgs).1,
        Dag.inboxOfSends m src rest (Dag.wireDecode src rest cod.2 (Node.wireDecode src n cod.1 base msgs).2 msgs).1)
      = ((Node.inboxOfWire m n cod.1 base [⟨src, msgs⟩]).1,
        (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base [⟨src, msgs⟩]).2 [⟨src, msgs⟩]).1)
      ∧ (Dag.wireDecode src rest cod.2 (Node.wireDecode src n cod.1 base msgs).2 msgs).2
      = (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base [⟨src, msgs⟩]).2 [⟨src, msgs⟩]).2
    rw [h1, h2]
    obtain ⟨h3, h4⟩ := Dag.inboxOfSends_wireDecode m src rest cod.2 _ msgs
    rw [h3, h4]
    exact ⟨rfl, rfl⟩

end

mutual

/-- The numbering of the network nodes does not depend on the batches. -/
theorem Node.inboxOfWire_snd (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) → (a b : List ((src : Member L mem) × List WireMsg)) →
    (Node.inboxOfWire m n cod base a).2 = (Node.inboxOfWire m n cod base b).2
  | _, _, .call N _, cod, base, a, b => Dag.inboxOfWire_snd m N cod base a b
  | _, _, .fixN _ body, cod, base, a, b => Dag.inboxOfWire_snd m body cod base a b
  | _, _, .broadcast_closed .., _, _, _, _ => rfl
  | _, _, .demux .., _, _, _, _ => rfl
  | _, _, .map .., _, _, _, _ => rfl
  | _, _, .filterMap .., _, _, _, _ => rfl
  | _, _, .values .., _, _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _, _ => rfl
  | _, _, .union .., _, _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _, _ => rfl
  | _, _, .fold .., _, _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _, _ => rfl
  | _, _, .snapshot .., _, _, _, _ => rfl
  | _, _, .batch .., _, _, _, _ => rfl
  | _, _, .source_interval .., _, _, _, _ => rfl
  | _, _, .mapTick .., _, _, _, _ => rfl
  | _, _, .firstTick .., _, _, _, _ => rfl
  | _, _, .zipTick .., _, _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _, _ => rfl
  | _, _, .allTicks .., _, _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _, _ => rfl
  | _, _, .kentries .., _, _, _, _ => rfl
  | _, _, .kmap .., _, _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _, _ => rfl
  | _, _, .kfold .., _, _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _, _ => rfl
  | _, _, .tfold .., _, _, _, _ => rfl
  | _, _, .latest .., _, _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _, _ => rfl
  | _, _, .time .., _, _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _, _ => rfl

theorem Dag.inboxOfWire_snd (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) → (a b : List ((src : Member L mem) × List WireMsg)) →
    (Dag.inboxOfWire m d cod base a).2 = (Dag.inboxOfWire m d cod base b).2
  | _, _, .ret _, _, _, _, _ => rfl
  | _, _, .letN n rest, cod, base, a, b => by
    show (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base a).2 a).2
      = (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base b).2 b).2
    rw [Node.inboxOfWire_snd m n cod.1 base a b]
    exact Dag.inboxOfWire_snd m rest cod.2 _ a b

end

mutual

/-- Appending the inboxes of two batch lists is the inbox of their concatenation. -/
theorem Node.inboxOfWire_append (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) → (a b : List ((src : Member L mem) × List WireMsg)) →
    Node.Inbox.append m n (Node.inboxOfWire m n cod base a).1 (Node.inboxOfWire m n cod base b).1
      = (Node.inboxOfWire m n cod base (a ++ b)).1
  | _, _, .broadcast_closed .., _, _, a, b => funext fun _ => funext fun _ => (appendFold_append ..).symm
  | _, _, .demux .., _, _, a, b => funext fun _ => funext fun _ => (appendFold_append ..).symm
  | _, _, .call N _, cod, base, a, b => Dag.inboxOfWire_append m N cod base a b
  | _, _, .fixN _ body, cod, base, a, b => Dag.inboxOfWire_append m body cod base a b
  | _, _, .map .., _, _, _, _ => rfl
  | _, _, .filterMap .., _, _, _, _ => rfl
  | _, _, .values .., _, _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _, _ => rfl
  | _, _, .union .., _, _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _, _ => rfl
  | _, _, .fold .., _, _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _, _ => rfl
  | _, _, .snapshot .., _, _, _, _ => rfl
  | _, _, .batch .., _, _, _, _ => rfl
  | _, _, .source_interval .., _, _, _, _ => rfl
  | _, _, .mapTick .., _, _, _, _ => rfl
  | _, _, .firstTick .., _, _, _, _ => rfl
  | _, _, .zipTick .., _, _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _, _ => rfl
  | _, _, .allTicks .., _, _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _, _ => rfl
  | _, _, .kentries .., _, _, _, _ => rfl
  | _, _, .kmap .., _, _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _, _ => rfl
  | _, _, .kfold .., _, _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _, _ => rfl
  | _, _, .tfold .., _, _, _, _ => rfl
  | _, _, .latest .., _, _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _, _ => rfl
  | _, _, .time .., _, _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _, _ => rfl

theorem Dag.inboxOfWire_append (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) → (a b : List ((src : Member L mem) × List WireMsg)) →
    Dag.Inbox.append m d (Dag.inboxOfWire m d cod base a).1 (Dag.inboxOfWire m d cod base b).1
      = (Dag.inboxOfWire m d cod base (a ++ b)).1
  | _, _, .ret _, _, _, _, _ => rfl
  | _, _, .letN n rest, cod, base, a, b => by
    show (Node.Inbox.append m n (Node.inboxOfWire m n cod.1 base a).1 (Node.inboxOfWire m n cod.1 base b).1,
        Dag.Inbox.append m rest (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base a).2 a).1
          (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base b).2 b).1)
      = ((Node.inboxOfWire m n cod.1 base (a ++ b)).1,
        (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base (a ++ b)).2 (a ++ b)).1)
    rw [Node.inboxOfWire_append m n cod.1 base a b, Node.inboxOfWire_snd m n cod.1 base b (a ++ b),
      Node.inboxOfWire_snd m n cod.1 base a (a ++ b), Dag.inboxOfWire_append m rest cod.2 _ a b]

end

mutual

/-- The empty inbox is the inbox of no batches. -/
theorem Node.inboxOfWire_nil (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) →
    (Node.inboxOfWire m n cod base []).1 = Node.Inbox.empty m n
  | _, _, .broadcast_closed .., _, _ => rfl
  | _, _, .demux .., _, _ => rfl
  | _, _, .call N _, cod, base => Dag.inboxOfWire_nil m N cod base
  | _, _, .fixN _ body, cod, base => Dag.inboxOfWire_nil m body cod base
  | _, _, .map .., _, _ => rfl
  | _, _, .filterMap .., _, _ => rfl
  | _, _, .values .., _, _ => rfl
  | _, _, .into_keyed .., _, _ => rfl
  | _, _, .kassume_ordering .., _, _ => rfl
  | _, _, .kfold_early_stop .., _, _ => rfl
  | _, _, .kget_max_key .., _, _ => rfl
  | _, _, .weaken_retries .., _, _ => rfl
  | _, _, .union .., _, _ => rfl
  | _, _, .assume_ordering .., _, _ => rfl
  | _, _, .fold .., _, _ => rfl
  | _, _, .fold_monotone .., _, _ => rfl
  | _, _, .snapshot .., _, _ => rfl
  | _, _, .batch .., _, _ => rfl
  | _, _, .source_interval .., _, _ => rfl
  | _, _, .mapTick .., _, _ => rfl
  | _, _, .firstTick .., _, _ => rfl
  | _, _, .zipTick .., _, _ => rfl
  | _, _, .defer_tick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl
  | _, _, .kentries .., _, _ => rfl
  | _, _, .kmap .., _, _ => rfl
  | _, _, .kallTicks .., _, _ => rfl
  | _, _, .kfold .., _, _ => rfl
  | _, _, .ksnapshot .., _, _ => rfl
  | _, _, .tfold .., _, _ => rfl
  | _, _, .latest .., _, _ => rfl
  | _, _, .latest_monotone .., _, _ => rfl
  | _, _, .time .., _, _ => rfl
  | _, _, .snapshot_bounded .., _, _ => rfl
  | _, _, .reduce_watermark .., _, _ => rfl
  | _, _, .tick_scan .., _, _ => rfl

theorem Dag.inboxOfWire_nil (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) →
    (Dag.inboxOfWire m d cod base []).1 = Dag.Inbox.empty m d
  | _, _, .ret _, _, _ => rfl
  | _, _, .letN n rest, cod, base => by
    show ((Node.inboxOfWire m n cod.1 base []).1, (Dag.inboxOfWire m rest cod.2 _ []).1)
      = (Node.Inbox.empty m n, Dag.Inbox.empty m rest)
    rw [Node.inboxOfWire_nil, Dag.inboxOfWire_nil]

end

/-- **The one-walk inbox is `Dag.inboxOf` of the decoded batches.** -/
theorem Dag.inboxOfWire_eq (m : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (batches : List ((src : Member L mem) × List WireMsg)) :
    (Dag.inboxOfWire m d cod 0 batches).1
      = Dag.inboxOf m d (batches.map fun b => ⟨b.1, (Dag.wireDecode b.1 d cod 0 b.2).1⟩) := by
  unfold Dag.inboxOf
  -- fold the batches in from the left: the accumulated inbox is `inboxOfWire` of the prefix
  suffices h : ∀ (pre : List ((src : Member L mem) × List WireMsg)),
      (Dag.inboxOfWire m d cod 0 (pre ++ batches)).1 =
        (batches.map fun b => (⟨b.1, (Dag.wireDecode b.1 d cod 0 b.2).1⟩ : (src : Member L mem) × Dag.Sends src d)).foldl
          (fun ib ⟨src, snd⟩ => Dag.Inbox.append m d ib (Dag.inboxOfSends m src d snd))
          (Dag.inboxOfWire m d cod 0 pre).1 by
    have := h []
    rw [Dag.inboxOfWire_nil, List.nil_append] at this
    exact this
  intro pre
  induction batches generalizing pre with
  | nil => simp
  | cons b rest ih =>
    rw [List.map_cons, List.foldl_cons]
    show _ = List.foldl _ (Dag.Inbox.append m d (Dag.inboxOfWire m d cod 0 pre).1
      (Dag.inboxOfSends m b.1 d (Dag.wireDecode b.1 d cod 0 b.2).1)) _
    rw [(Dag.inboxOfSends_wireDecode m b.1 d cod 0 b.2).1, Dag.inboxOfWire_append, ← ih, List.append_assoc,
      List.singleton_append]

/-- **One pass of a process**, one lock-step step of its member: append the inbox, deliver
everything that has arrived at this activation's timing (`oracleOf`: the clock reading and
the interval pulses the shell read for this tick, `Dag.Timing`), step the projected graph
with `Dag.locStep` (a tick iff `tk`, else a stutter). Returns the new cells, the outputs at
the member, and the sends to every peer, itself included; they travel through the network
and nothing stays in the process. A process holds no schedule and no timing history: the
lock-step machine's script enters only through `timingAt` in the theorems. `memoState`
materializes the cells once (an identity, `Dag.memoState_eq`). `Dag.pass_eq`
(`Project/Model/DistributedModel.lean`) says a pass matches the lock-step step. -/
def Dag.pass (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (tm : Dag.Timing m d) (ib : Dag.Inbox m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × Dag.Sends m d :=
  let s' := Dag.memoState m d (Dag.enqueueInbox m d ib s)
  Dag.locStep m tk d (Dag.oracleOf m d tm s') Δ s'

/-! ### The activation

A `forward_ref` inside a tick is, in DFIR, a same-tick alias: `CycleSink` emits
`identity()` (`hydro_lang/src/compile/ir/mod.rs:2351`), and within a tick the only edges
excluded from the topological order are into `defer_tick`/`defer_tick_lazy`
(`dfir_lang/src/graph/flat_to_partitioned.rs`), so the consumer runs after the producer in
stratum order, in the same tick. The model's knot wire is likewise its body's output of the
same step, therefore **an activation is one sweep**: `Dag.iterate` is `Dag.pass`'s inbox,
cells and oracle followed by the sweep (`Project/Sweep.lean`) that evaluates each node once
and finds every knot block's increments by its sweeps over the body, each binding the knots
whose outputs came out known, as DFIR's strata do (`knotLoop`). `Dag.iterate_eq_pass`
(`Project/Model/SweepSpec.lean`) says that on a valid graph it is `Dag.pass`. A process that
ticked emits every knot-dependent send in that tick, and nothing is pending after an
activation. -/

/-- **One activation of a process**, the loop body, the function the processes run and the
theorems are about: the inbox appended, the cells materialized, the cursors read, then one
sweep of the graph. Returns the cells, the activation's outputs and its sends. -/
def Dag.iterate (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (tm : Dag.Timing m d) (ib : Dag.Inbox m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × Dag.Sends m d :=
  let s' := Dag.memoState m d (Dag.enqueueInbox m d ib s)
  Dag.sweepTop m tk d tm Δ s'

/-! ### The sender's messages, decoded by the receiver

`Dag.inboxOfWire_wireEncode`: a sender's `wireEncode`, read back by a receiver, is the
receiver's inbox increment of the sender's sends (`inboxOfSends`), through the codecs' round
trip. The two walks number the network nodes alike (`wireReceivers`), and a node's messages
carry its number and no other's. -/

/-- `decodeAt` reads only its node's messages. -/
theorem decodeAt_filter {α : Type} (cod : Codec α) (msgs : List WireMsg) (k j : Nat) :
    decodeAt cod (msgs.filter fun msg => decide (msg.1 = k)) k j = decodeAt cod msgs k j := by
  apply Array.toList_inj.mp
  rw [decodeAt_toList, decodeAt_toList, List.filterMap_filter]
  apply List.filterMap_congr
  intro msg _
  by_cases h : msg.1 = k <;> simp [h]

mutual

theorem Node.inboxOfWire_snd_eq (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) → (b : List ((src : Member L mem) × List WireMsg)) →
    (Node.inboxOfWire m n cod base b).2 = (Node.wireReceivers n base).2
  | _, _, .broadcast_closed .., _, _, _ => rfl
  | _, _, .demux .., _, _, _ => rfl
  | _, _, .call N _, cod, base, b => Dag.inboxOfWire_snd_eq m N cod base b
  | _, _, .fixN _ body, cod, base, b => Dag.inboxOfWire_snd_eq m body cod base b
  | _, _, .map .., _, _, _ => rfl
  | _, _, .filterMap .., _, _, _ => rfl
  | _, _, .values .., _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _ => rfl
  | _, _, .union .., _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _ => rfl
  | _, _, .fold .., _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _ => rfl
  | _, _, .snapshot .., _, _, _ => rfl
  | _, _, .batch .., _, _, _ => rfl
  | _, _, .source_interval .., _, _, _ => rfl
  | _, _, .mapTick .., _, _, _ => rfl
  | _, _, .firstTick .., _, _, _ => rfl
  | _, _, .zipTick .., _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _ => rfl
  | _, _, .allTicks .., _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _ => rfl
  | _, _, .kentries .., _, _, _ => rfl
  | _, _, .kmap .., _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _ => rfl
  | _, _, .kfold .., _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _ => rfl
  | _, _, .tfold .., _, _, _ => rfl
  | _, _, .latest .., _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _ => rfl
  | _, _, .time .., _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _ => rfl

theorem Dag.inboxOfWire_snd_eq (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) → (b : List ((src : Member L mem) × List WireMsg)) →
    (Dag.inboxOfWire m d cod base b).2 = (Dag.wireReceivers d base).2
  | _, _, .ret _, _, _, _ => rfl
  | _, _, .letN n rest, cod, base, b => by
    show (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base b).2 b).2
      = (Dag.wireReceivers rest (Node.wireReceivers n base).2).2
    rw [Node.inboxOfWire_snd_eq, Dag.inboxOfWire_snd_eq]

end

mutual

theorem Node.wireEncode_snd_eq (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) → (snd : Node.Sends m n) →
    (Node.wireEncode m n cod base snd).2 = (Node.wireReceivers n base).2
  | _, _, .broadcast_closed .., _, _, _ => rfl
  | _, _, .demux .., _, _, _ => rfl
  | _, _, .call N _, cod, base, snd => Dag.wireEncode_snd_eq m N cod base snd
  | _, _, .fixN _ body, cod, base, snd => Dag.wireEncode_snd_eq m body cod base snd
  | _, _, .map .., _, _, _ => rfl
  | _, _, .filterMap .., _, _, _ => rfl
  | _, _, .values .., _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _ => rfl
  | _, _, .union .., _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _ => rfl
  | _, _, .fold .., _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _ => rfl
  | _, _, .snapshot .., _, _, _ => rfl
  | _, _, .batch .., _, _, _ => rfl
  | _, _, .source_interval .., _, _, _ => rfl
  | _, _, .mapTick .., _, _, _ => rfl
  | _, _, .firstTick .., _, _, _ => rfl
  | _, _, .zipTick .., _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _ => rfl
  | _, _, .allTicks .., _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _ => rfl
  | _, _, .kentries .., _, _, _ => rfl
  | _, _, .kmap .., _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _ => rfl
  | _, _, .kfold .., _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _ => rfl
  | _, _, .tfold .., _, _, _ => rfl
  | _, _, .latest .., _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _ => rfl
  | _, _, .time .., _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _ => rfl

theorem Dag.wireEncode_snd_eq (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) → (snd : Dag.Sends m d) →
    (Dag.wireEncode m d cod base snd).2 = (Dag.wireReceivers d base).2
  | _, _, .ret _, _, _, _ => rfl
  | _, _, .letN n rest, cod, base, snd => by
    show (Dag.wireEncode m rest cod.2 (Node.wireEncode m n cod.1 base snd.1).2 snd.2).2
      = (Dag.wireReceivers rest (Node.wireReceivers n base).2).2
    rw [Node.wireEncode_snd_eq, Dag.wireEncode_snd_eq]

end

mutual

theorem Node.wireReceivers_le : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (base : Nat) → base ≤ (Node.wireReceivers n base).2
  | _, _, .broadcast_closed .., _ => Nat.le_succ _
  | _, _, .demux .., _ => Nat.le_succ _
  | _, _, .call N _, base => Dag.wireReceivers_le N base
  | _, _, .fixN _ body, base => Dag.wireReceivers_le body base
  | _, _, .map .., _ => Nat.le_refl _
  | _, _, .filterMap .., _ => Nat.le_refl _
  | _, _, .values .., _ => Nat.le_refl _
  | _, _, .into_keyed .., _ => Nat.le_refl _
  | _, _, .kassume_ordering .., _ => Nat.le_refl _
  | _, _, .kfold_early_stop .., _ => Nat.le_refl _
  | _, _, .kget_max_key .., _ => Nat.le_refl _
  | _, _, .weaken_retries .., _ => Nat.le_refl _
  | _, _, .union .., _ => Nat.le_refl _
  | _, _, .assume_ordering .., _ => Nat.le_refl _
  | _, _, .fold .., _ => Nat.le_refl _
  | _, _, .fold_monotone .., _ => Nat.le_refl _
  | _, _, .snapshot .., _ => Nat.le_refl _
  | _, _, .batch .., _ => Nat.le_refl _
  | _, _, .source_interval .., _ => Nat.le_refl _
  | _, _, .mapTick .., _ => Nat.le_refl _
  | _, _, .firstTick .., _ => Nat.le_refl _
  | _, _, .zipTick .., _ => Nat.le_refl _
  | _, _, .defer_tick .., _ => Nat.le_refl _
  | _, _, .allTicks .., _ => Nat.le_refl _
  | _, _, .flattenOrdered .., _ => Nat.le_refl _
  | _, _, .flattenUnordered .., _ => Nat.le_refl _
  | _, _, .tinto_keyed .., _ => Nat.le_refl _
  | _, _, .tassume_ordering .., _ => Nat.le_refl _
  | _, _, .kentries .., _ => Nat.le_refl _
  | _, _, .kmap .., _ => Nat.le_refl _
  | _, _, .kallTicks .., _ => Nat.le_refl _
  | _, _, .kfold .., _ => Nat.le_refl _
  | _, _, .ksnapshot .., _ => Nat.le_refl _
  | _, _, .tfold .., _ => Nat.le_refl _
  | _, _, .latest .., _ => Nat.le_refl _
  | _, _, .latest_monotone .., _ => Nat.le_refl _
  | _, _, .time .., _ => Nat.le_refl _
  | _, _, .snapshot_bounded .., _ => Nat.le_refl _
  | _, _, .reduce_watermark .., _ => Nat.le_refl _
  | _, _, .tick_scan .., _ => Nat.le_refl _

theorem Dag.wireReceivers_le : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (base : Nat) → base ≤ (Dag.wireReceivers d base).2
  | _, _, .ret _, _ => Nat.le_refl _
  | _, _, .letN n rest, base =>
    Nat.le_trans (Node.wireReceivers_le n base) (Dag.wireReceivers_le rest _)

end

mutual

/-- A node's messages carry numbers in its range. -/
theorem Node.wireEncode_mem (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) → (snd : Node.Sends m n) →
    ∀ msg ∈ (Node.wireEncode m n cod base snd).1, base ≤ msg.1 ∧ msg.1 < (Node.wireReceivers n base).2
  | _, _, .broadcast_closed .., _, base, _ => fun msg hmsg => by
    obtain ⟨j, _, hj⟩ := List.mem_flatMap.mp hmsg
    obtain ⟨v, _, rfl⟩ := List.mem_map.mp hj
    exact ⟨Nat.le_refl _, Nat.lt_succ_self _⟩
  | _, _, .demux .., _, base, _ => fun msg hmsg => by
    obtain ⟨j, _, hj⟩ := List.mem_flatMap.mp hmsg
    obtain ⟨v, _, rfl⟩ := List.mem_map.mp hj
    exact ⟨Nat.le_refl _, Nat.lt_succ_self _⟩
  | _, _, .call N _, cod, base, snd => Dag.wireEncode_mem m N cod base snd
  | _, _, .fixN _ body, cod, base, snd => Dag.wireEncode_mem m body cod base snd
  | _, _, .map .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .filterMap .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .values .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .into_keyed .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kassume_ordering .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kfold_early_stop .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kget_max_key .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .weaken_retries .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .union .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .assume_ordering .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .fold .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .fold_monotone .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .snapshot .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .batch .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .source_interval .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .mapTick .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .firstTick .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .zipTick .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .defer_tick .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .allTicks .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .flattenOrdered .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .flattenUnordered .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .tinto_keyed .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .tassume_ordering .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kentries .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kmap .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kallTicks .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kfold .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .ksnapshot .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .tfold .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .latest .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .latest_monotone .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .time .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .snapshot_bounded .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .reduce_watermark .., _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .tick_scan .., _, _, _ => fun _ h => absurd h List.not_mem_nil

theorem Dag.wireEncode_mem (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) → (snd : Dag.Sends m d) →
    ∀ msg ∈ (Dag.wireEncode m d cod base snd).1, base ≤ msg.1 ∧ msg.1 < (Dag.wireReceivers d base).2
  | _, _, .ret _, _, _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .letN n rest, cod, base, snd => fun msg hmsg => by
    have hmsg : msg ∈ (Node.wireEncode m n cod.1 base snd.1).1
        ++ (Dag.wireEncode m rest cod.2 (Node.wireEncode m n cod.1 base snd.1).2 snd.2).1 := hmsg
    show base ≤ msg.1 ∧ msg.1 < (Dag.wireReceivers rest (Node.wireReceivers n base).2).2
    rcases List.mem_append.mp hmsg with h | h
    · obtain ⟨h1, h2⟩ := Node.wireEncode_mem m n cod.1 base snd.1 msg h
      exact ⟨h1, Nat.lt_of_lt_of_le h2 (Dag.wireReceivers_le rest _)⟩
    · obtain ⟨h1, h2⟩ := Dag.wireEncode_mem m rest cod.2 _ snd.2 msg h
      rw [Node.wireEncode_snd_eq] at h1 h2
      exact ⟨Nat.le_trans (Node.wireReceivers_le n base) h1, h2⟩

end

/-- The messages of a node in a list in which all of that node's messages are the
sender's: the sender's. -/
theorem decodeAt_of_filter_eq {α : Type} (cod : Codec α) (all own : List WireMsg) (k j : Nat)
    (h : all.filter (fun msg => decide (msg.1 = k)) = own.filter (fun msg => decide (msg.1 = k))) :
    decodeAt cod all k j = decodeAt cod own k j := by
  rw [← decodeAt_filter cod all, h, decodeAt_filter]

/-- Decoding what one hosted sender encoded. -/
theorem decodeAt_encode {α : Type} (cod : Codec α) (base j : Nat) (vs : Array α) :
    decodeAt cod (vs.toList.map fun v => (base, j, cod.enc v)) base j = vs := by
  apply Array.toList_inj.mp
  rw [decodeAt_toList, List.filterMap_map]
  have : ((fun msg : WireMsg => if msg.1 = base ∧ msg.2.1 = j then (cod.dec msg.2.2).toOption else none)
      ∘ fun v : α => (base, j, cod.enc v)) = fun v => some v := by
    funext v
    simp [cod.dec_enc, Except.toOption]
  rw [this]
  exact List.filterMap_some

/-- Decoding at another sender: nothing. -/
theorem decodeAt_encode_ne {α : Type} (cod : Codec α) (base j j' : Nat) (hj : j' ≠ j) (vs : Array α) :
    decodeAt cod (vs.toList.map fun v => (base, j, cod.enc v)) base j' = #[] := by
  apply Array.toList_inj.mp
  rw [decodeAt_toList, Array.toList_empty, List.filterMap_map, List.filterMap_eq_nil_iff]
  intro v _
  simp [hj.symm]

mutual

theorem Node.inboxOfWire_wireEncode (m src : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) → (snd : Node.Sends src n) → (all : List WireMsg) →
    (∀ k, base ≤ k → k < (Node.wireReceivers n base).2 →
      all.filter (fun msg => decide (msg.1 = k)) = (Node.wireEncode src n cod base snd).1.filter (fun msg => decide (msg.1 = k))) →
    (Node.inboxOfWire m n cod base [⟨src, all⟩]).1 = Node.inboxOfSends m src n snd
  | _, _, .broadcast_closed (c := c) .., cod, base, snd, all, h => funext fun _ => funext fun j => by
    show appendFold (fun b : (src : Member L mem) × List WireMsg => if (⟨c, j⟩ : Member L mem) = b.1 then decodeAt cod b.2 base j.val else #[])
        [⟨src, all⟩]
      = if h : (⟨c, j⟩ : Member L mem) = src then snd ⟨j, h⟩ else #[]
    rw [appendFold_singleton]
    show (if (⟨c, j⟩ : Member L mem) = src then decodeAt cod all base j.val else #[])
      = if h : (⟨c, j⟩ : Member L mem) = src then snd ⟨j, h⟩ else #[]
    split
    · rename_i hj
      rw [decodeAt_of_filter_eq cod all _ base j.val (h base (Nat.le_refl _) (Nat.lt_succ_self _))]
      show decodeAt cod ((hostedList src c).flatMap fun j' => (snd j').toList.map fun v => (base, j'.1.val, cod.enc v)) base j.val = _
      rw [hostedList, hostedSelf_eq ⟨j, hj⟩]
      show decodeAt cod (((snd ⟨j, hj⟩).toList.map fun v => (base, j.val, cod.enc v)) ++ []) base j.val = _
      rw [List.append_nil]
      exact decodeAt_encode (cod : Codec _) base j.val (snd ⟨j, hj⟩)
    · rfl
  | _, _, .demux (c := c) _ _ addr, cod, base, snd, all, h => funext fun i => funext fun j => by
    show appendFold (fun b : (src : Member L mem) × List WireMsg => if (⟨c, j⟩ : Member L mem) = b.1 then
          filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod b.2 base j.val) else #[])
        [⟨src, all⟩]
      = if h : (⟨c, j⟩ : Member L mem) = src then
        filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (snd ⟨j, h⟩) else #[]
    rw [appendFold_singleton]
    show (if (⟨c, j⟩ : Member L mem) = src then
        filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod all base j.val) else #[])
      = if h : (⟨c, j⟩ : Member L mem) = src then
        filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (snd ⟨j, h⟩) else #[]
    split
    · rename_i hj
      rw [decodeAt_of_filter_eq cod all _ base j.val (h base (Nat.le_refl _) (Nat.lt_succ_self _))]
      show filterMapB _ (decodeAt cod ((hostedList src c).flatMap fun j' => (snd j').toList.map fun v => (base, j'.1.val, cod.enc v)) base j.val) = _
      rw [hostedList, hostedSelf_eq ⟨j, hj⟩]
      show filterMapB _ (decodeAt cod (((snd ⟨j, hj⟩).toList.map fun v => (base, j.val, cod.enc v)) ++ []) base j.val) = _
      rw [List.append_nil, decodeAt_encode (cod : Codec _) base j.val (snd ⟨j, hj⟩)]
    · rfl
  | _, _, .call N _, cod, base, snd, all, h => Dag.inboxOfWire_wireEncode m src N cod base snd all h
  | _, _, .fixN _ body, cod, base, snd, all, h => Dag.inboxOfWire_wireEncode m src body cod base snd all h
  | _, _, .map .., _, _, _, _, _ => rfl
  | _, _, .filterMap .., _, _, _, _, _ => rfl
  | _, _, .values .., _, _, _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _, _, _ => rfl
  | _, _, .union .., _, _, _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _, _, _ => rfl
  | _, _, .fold .., _, _, _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _, _, _ => rfl
  | _, _, .snapshot .., _, _, _, _, _ => rfl
  | _, _, .batch .., _, _, _, _, _ => rfl
  | _, _, .source_interval .., _, _, _, _, _ => rfl
  | _, _, .mapTick .., _, _, _, _, _ => rfl
  | _, _, .firstTick .., _, _, _, _, _ => rfl
  | _, _, .zipTick .., _, _, _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _, _, _ => rfl
  | _, _, .allTicks .., _, _, _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _, _, _ => rfl
  | _, _, .kentries .., _, _, _, _, _ => rfl
  | _, _, .kmap .., _, _, _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _, _, _ => rfl
  | _, _, .kfold .., _, _, _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _, _, _ => rfl
  | _, _, .tfold .., _, _, _, _, _ => rfl
  | _, _, .latest .., _, _, _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _, _, _ => rfl
  | _, _, .time .., _, _, _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _, _, _ => rfl

theorem Dag.inboxOfWire_wireEncode (m src : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) → (snd : Dag.Sends src d) → (all : List WireMsg) →
    (∀ k, base ≤ k → k < (Dag.wireReceivers d base).2 →
      all.filter (fun msg => decide (msg.1 = k)) = (Dag.wireEncode src d cod base snd).1.filter (fun msg => decide (msg.1 = k))) →
    (Dag.inboxOfWire m d cod base [⟨src, all⟩]).1 = Dag.inboxOfSends m src d snd
  | _, _, .ret _, _, _, _, _, _ => rfl
  | _, _, .letN n rest, cod, base, snd, all, h => by
    have hown : (Dag.wireEncode src (.letN n rest) cod base snd).1
        = (Node.wireEncode src n cod.1 base snd.1).1
          ++ (Dag.wireEncode src rest cod.2 (Node.wireEncode src n cod.1 base snd.1).2 snd.2).1 := rfl
    have hbase : (Node.wireEncode src n cod.1 base snd.1).2 = (Node.wireReceivers n base).2 :=
      Node.wireEncode_snd_eq src n cod.1 base snd.1
    -- the node's range sees only the node's messages
    have hn : ∀ k, base ≤ k → k < (Node.wireReceivers n base).2 →
        all.filter (fun msg => decide (msg.1 = k)) = (Node.wireEncode src n cod.1 base snd.1).1.filter (fun msg => decide (msg.1 = k)) := by
      intro k hk1 hk2
      have hz : (Dag.wireEncode src rest cod.2 (Node.wireEncode src n cod.1 base snd.1).2 snd.2).1.filter
          (fun msg => decide (msg.1 = k)) = [] := by
        rw [List.filter_eq_nil_iff]
        intro msg hmsg
        obtain ⟨h1, _⟩ := Dag.wireEncode_mem src rest cod.2 _ snd.2 msg hmsg
        rw [hbase] at h1
        simp only [decide_eq_true_eq]
        omega
      rw [h k hk1 (Nat.lt_of_lt_of_le hk2 (Dag.wireReceivers_le rest _)), hown, List.filter_append, hz, List.append_nil]
    -- the rest's range sees only the rest's messages
    have hr : ∀ k, (Node.wireReceivers n base).2 ≤ k → k < (Dag.wireReceivers rest (Node.wireReceivers n base).2).2 →
        all.filter (fun msg => decide (msg.1 = k))
          = (Dag.wireEncode src rest cod.2 (Node.wireReceivers n base).2 snd.2).1.filter (fun msg => decide (msg.1 = k)) := by
      intro k hk1 hk2
      have hz : (Node.wireEncode src n cod.1 base snd.1).1.filter (fun msg => decide (msg.1 = k)) = [] := by
        rw [List.filter_eq_nil_iff]
        intro msg hmsg
        obtain ⟨_, h2⟩ := Node.wireEncode_mem src n cod.1 base snd.1 msg hmsg
        simp only [decide_eq_true_eq]
        omega
      rw [h k (Nat.le_trans (Node.wireReceivers_le n base) hk1) hk2, hown, List.filter_append, hbase, hz, List.nil_append]
    show ((Node.inboxOfWire m n cod.1 base [⟨src, all⟩]).1,
        (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base [⟨src, all⟩]).2 [⟨src, all⟩]).1)
      = (Node.inboxOfSends m src n snd.1, Dag.inboxOfSends m src rest snd.2)
    rw [Node.inboxOfWire_wireEncode m src n cod.1 base snd.1 all hn, Node.inboxOfWire_snd_eq,
      Dag.inboxOfWire_wireEncode m src rest cod.2 _ snd.2 all hr]

end

/-- **A sender's messages, decoded by the receiver, are the receiver's inbox increment of
the sender's sends**, the lock-step model's `inboxOfSends` (what `enqueueLoc` enqueues).
The proof rests on the codecs' round-trip hypothesis (`Codec.dec_enc`). -/
theorem Dag.inboxOfWire_wireEncode_self (m src : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)}
    (d : Dag Γ outs) (cod : Dag.Codecs d) (snd : Dag.Sends src d) :
    (Dag.inboxOfWire m d cod 0 [⟨src, (Dag.wireEncode src d cod 0 snd).1⟩]).1 = Dag.inboxOfSends m src d snd :=
  Dag.inboxOfWire_wireEncode m src d cod 0 snd _ (fun _ _ _ => rfl)

/-! ## The loop body at the wire

What a deployed process runs: bytes in, bytes out. `Dag.iterateWire` is `Dag.iterate` with
the inbox decoded from the received batches in one walk (`inboxOfWire`) and the sends
encoded (`wireEncode`). `Interleaving.Batched.loop_pass_wire`
(`Project/Model/DistributedModel.lean`) proves that a run of its lock-step form
`Dag.passWire` matches the lock-step run, through the codecs' round trip (`Codec.dec_enc`), so a
wrong codec is a failed proof. -/

/-- **One pass of a process, at the wire**: the batches that arrived (per batch, the sender
and its messages), `Dag.pass`, the sends as messages. -/
def Dag.passWire (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (tm : Dag.Timing m d) (batches : List ((src : Member L mem) × List WireMsg))
    (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg :=
  let r := Dag.pass m tk d tm (Dag.inboxOfWire m d cod 0 batches).1 Δ s
  (r.1, r.2.1, (Dag.wireEncode m d cod 0 r.2.2).1)

/-- **One activation of a process, at the wire**: the batches that arrived (per batch, the
sender and its messages), the loop body `Dag.iterate`, the sends as messages. -/
def Dag.iterateWire (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (tm : Dag.Timing m d) (batches : List ((src : Member L mem) × List WireMsg))
    (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg :=
  let r := Dag.iterate m tk d tm (Dag.inboxOfWire m d cod 0 batches).1 Δ s
  (r.1, r.2.1, (Dag.wireEncode m d cod 0 r.2.2).1)

/-- The messages in a connection's unread bytes, the complete frames decoded, and the
incomplete tail to keep. -/
def parseFrames (bytes : ByteArray) : List WireMsg × ByteArray :=
  let r := Wire.frames (α := WireMsg) bytes
  (r.1.toList.filterMap Except.toOption, r.2)

/-! `parseFrames` compiled. `Wire.frames` extracts every frame's body, decodes the message out
of the copy (which extracts the payload again), and pushes an `Except` onto an array that is
then turned into a list and filtered. `parseFramesImpl` reads the frame length, then the
message's fields in place at the frame's offset (`Wire.getK`; the payload is the only copy),
accepts the message iff the reader ends at the frame's end, and conses it onto the list.
`parseFrames_eq_impl` is the `@[csimp]` equation, through `Wire.Restr`: a reader that ends
inside an extract reads the same bytes as the reader on the buffer. -/

/-- `WireMsg`'s reader reads inside an extract as inside the buffer. -/
theorem restr_wireMsg : Wire.Restr (Wire.get (α := WireMsg)) :=
  Wire.restr_getProd Wire.restr_nat (Wire.restr_getProd Wire.restr_nat Wire.restr_bytes)

/-- The frame body's decode, read in place: the message iff the reader ends at the frame's end. -/
theorem decode_extract_toOption (buf : ByteArray) (i' len : Nat) (h : i' + len ≤ buf.size) :
    (Wire.decode (α := WireMsg) (buf.extract i' (i' + len))).toOption
      = match Wire.get (α := WireMsg) buf i' with
        | .ok (x, e) => if e = i' + len then some x else none
        | .error _ => none := by
  have hsz : (buf.extract i' (i' + len)).size = len := by
    rw [ByteArray.size_extract, Nat.min_eq_left h]; omega
  have hr := restr_wireMsg.2 buf i' (i' + len) 0 len
  rcases hd : Wire.decode (α := WireMsg) (buf.extract i' (i' + len)) with er | x
  · simp only [Except.toOption]
    rcases hg : Wire.get (α := WireMsg) buf i' with er' | ⟨x, e⟩
    · rfl
    · simp only
      split
      · rename_i he
        exfalso
        subst he
        have := (hr x h (by omega)).mpr (by rw [Nat.add_zero]; exact hg)
        have hok := (Wire.decode_ok_iff (buf.extract i' (i' + len)) x).mpr (by rw [hsz]; exact this)
        rw [hd] at hok; cases hok
      · rfl
  · simp only [Except.toOption]
    have hok := (Wire.decode_ok_iff _ x).mp hd
    rw [hsz] at hok
    have hg := (hr x h (by omega)).mp hok
    rw [Nat.add_zero] at hg
    rw [hg]
    simp

/-- The frames from offset `i`, read in place, consed in reverse onto `acc`. -/
def parseFramesGo (buf : ByteArray) (i : Nat) (acc : List WireMsg) : List WireMsg × ByteArray :=
  match hg : Wire.getNat buf i with
  | .ok (len, i') =>
    if _h : i' + len ≤ buf.size then
      let j := i' + len
      let acc' := match Wire.getK (α := WireMsg) buf i' (fun x e => if e = j then .ok x else .error "") with
        | .ok x => x :: acc
        | .error _ => acc
      parseFramesGo buf j acc'
    else (acc.reverse, buf.extract i buf.size)
  | .error _ => (acc.reverse, buf.extract i buf.size)
termination_by buf.size - i
decreasing_by
  have := Wire.getNat_ok_bounds buf i 0 0 hg
  omega

def parseFramesImpl (bytes : ByteArray) : List WireMsg × ByteArray := parseFramesGo bytes 0 []

theorem parseFramesGo_eq (buf : ByteArray) : ∀ (i : Nat) (acc : List WireMsg) (arr : Array (Except String WireMsg)),
    acc.reverse = arr.toList.filterMap Except.toOption →
    parseFramesGo buf i acc
      = ((Wire.framesFrom buf i arr).1.toList.filterMap Except.toOption, (Wire.framesFrom buf i arr).2)
  | i, acc, arr, hacc => by
    rw [parseFramesGo, Wire.framesFrom]
    unfold Wire.getBytes
    rcases hg : Wire.getNat buf i with er | ⟨len, i'⟩
    · simp only [bind, Except.bind, hacc]
    · simp only [bind, Except.bind]
      have hb := Wire.getNat_ok_bounds buf i 0 0 hg
      split
      · rename_i h
        simp only
        rw [dif_pos ⟨by omega, h⟩]
        have hacc' : (match Wire.getK (α := WireMsg) buf i' (fun x e => if e = i' + len then .ok x else .error "") with
            | .ok x => x :: acc | .error _ => acc).reverse
            = (arr.push (Wire.decode (buf.extract i' (i' + len)))).toList.filterMap Except.toOption := by
          rw [Array.toList_push, List.filterMap_append, ← hacc, Wire.getK_eq]
          simp only [List.filterMap_cons, List.filterMap_nil, decode_extract_toOption buf i' len h]
          rcases Wire.get (α := WireMsg) buf i' with er | ⟨x, e⟩
          · simp [Wire.bindGet]
          · simp only [Wire.bindGet_ok]
            by_cases he : e = i' + len
            · simp [he]
            · simp [he]
        exact parseFramesGo_eq buf (i' + len) _ _ hacc'
      · simp only [hacc]
termination_by i => buf.size - i
decreasing_by
  omega

@[csimp] theorem parseFrames_eq_impl : @parseFrames = @parseFramesImpl := by
  funext bytes
  unfold parseFrames parseFramesImpl Wire.frames
  exact (parseFramesGo_eq bytes 0 [] #[] (by simp)).symm ▸ rfl

/-- **One activation of a process, at the bytes**: per connection, its unread bytes (the tail
kept from the last activation, then whatever the socket has delivered since, cut anywhere,
mid-frame included). The complete frames are this activation's messages, `Dag.iterateWire`
is the loop body, and the incomplete tails come back, one per connection in order, for the
process to keep. The shells call this; a `WireTrace` (`Project/Model/WireTrace.lean`) runs it
along any byte-level history of the connections. The connections carry only the program's
frames; a harness's own traffic travels on its own sockets. -/
def Dag.iterateBytes (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (tm : Dag.Timing m d) (chans : List ((src : Member L mem) × ByteArray))
    (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg × List ByteArray :=
  let parsed := chans.map fun c => (⟨c.1, (parseFrames c.2).1⟩ : (src : Member L mem) × List WireMsg)
  let r := Dag.iterateWire m tk d cod tm parsed Δ s
  (r.1, r.2.1, r.2.2, chans.map fun c => (parseFrames c.2).2)

/-- One pass of a process at the bytes: the proof layer's unit, a `PassTrace` step
(`Project/Model/WireTrace.lean`). `Dag.iterateBytes` computes the same body through
`Dag.iterate` (the sweep); `Dag.iterate_eq_pass` proves the two equal. -/
def Dag.passBytes (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (tm : Dag.Timing m d) (chans : List ((src : Member L mem) × ByteArray))
    (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg × List ByteArray :=
  let parsed := chans.map fun c => (⟨c.1, (parseFrames c.2).1⟩ : (src : Member L mem) × List WireMsg)
  let r := Dag.passWire m tk d cod tm parsed Δ s
  (r.1, r.2.1, r.2.2, chans.map fun c => (parseFrames c.2).2)

/-- The messages a sender puts on a connection, as bytes: one frame per message. -/
def frameMsgs (msgs : List WireMsg) : ByteArray := Wire.frameAll msgs

/-- One message framed straight onto the buffer: the frame's length, then the node, the
sender, the payload's length, the payload, the fields' `Wire` forms written in place
(`frameOneInto_eq`). `Wire.frame` would copy the payload once per nesting level. -/
def frameOneInto (acc : ByteArray) (msg : WireMsg) : ByteArray :=
  let inner := Wire.natLen msg.1 + (Wire.natLen msg.2.1 + (Wire.natLen msg.2.2.size + msg.2.2.size))
  Wire.pushNat (Wire.pushNat (Wire.pushNat (Wire.pushNat acc inner) msg.1) msg.2.1) msg.2.2.size ++ msg.2.2

theorem frameOneInto_eq (acc : ByteArray) (msg : WireMsg) : frameOneInto acc msg = acc ++ Wire.frame msg := by
  obtain ⟨k, j, p⟩ := msg
  have he : Wire.encode (k, j, p) = Wire.encode k ++ (Wire.encode j ++ (Wire.encode p.size ++ p)) := rfl
  have hsz : (Wire.encode (k, j, p)).size = Wire.natLen k + (Wire.natLen j + (Wire.natLen p.size + p.size)) := by
    rw [he]; simp only [ByteArray.size_append, Wire.size_encode_nat]
  show Wire.pushNat (Wire.pushNat (Wire.pushNat (Wire.pushNat acc _) k) j) p.size ++ p
      = acc ++ (Wire.encode (Wire.encode (k, j, p)).size ++ Wire.encode (k, j, p))
  rw [← hsz, he]
  simp only [Wire.pushNat_eq]
  show _ = acc ++ ((Wire.natBytes _).toByteArray ++ ((Wire.natBytes k).toByteArray
    ++ ((Wire.natBytes j).toByteArray ++ ((Wire.natBytes p.size).toByteArray ++ p))))
  simp only [ByteArray.append_assoc]

def frameMsgsImpl (msgs : List WireMsg) : ByteArray := msgs.foldl frameOneInto .empty

@[csimp] theorem frameMsgs_eq_impl : @frameMsgs = @frameMsgsImpl := by
  funext msgs
  unfold frameMsgs frameMsgsImpl
  have h : frameOneInto = fun acc x => acc ++ Wire.frame x := by
    funext acc x; exact frameOneInto_eq acc x
  rw [h, Wire.frameAllImpl_foldl, ByteArray.empty_append]

/-- The messages of a send list that process `m` receives: those of the network nodes whose
receiver cluster `m` is in, per the routing table `Dag.wireReceivers`. This is what a sender
puts on its connection to `m`. -/
def wireForTable (table : List (Nat × L)) (m : Member L mem) (msgs : List WireMsg) : List WireMsg :=
  msgs.filter fun msg => (table.find? (·.1 = msg.1)).any (fun e => e.2 = m.1)

def Dag.wireFor (m : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (msgs : List WireMsg) : List WireMsg :=
  wireForTable (Dag.wireReceivers d 0).1 m msgs

/-! ### Filtering to the receiver's nodes changes nothing it reads -/

mutual

/-- The routing table's entries carry the node numbers of their range. -/
theorem Node.wireReceivers_mem : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (base : Nat) →
    ∀ e ∈ (Node.wireReceivers n base).1, base ≤ e.1 ∧ e.1 < (Node.wireReceivers n base).2
  | _, _, .broadcast_closed (p := p) .., base => fun e he => by
    have he' : e ∈ [(base, p)] := he
    rw [List.mem_singleton] at he'; subst he'; exact ⟨Nat.le_refl _, Nat.lt_succ_self _⟩
  | _, _, .demux (p := p) .., base => fun e he => by
    have he' : e ∈ [(base, p)] := he
    rw [List.mem_singleton] at he'; subst he'; exact ⟨Nat.le_refl _, Nat.lt_succ_self _⟩
  | _, _, .call N _, base => Dag.wireReceivers_mem N base
  | _, _, .fixN _ body, base => Dag.wireReceivers_mem body base
  | _, _, .map .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .filterMap .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .values .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .into_keyed .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kassume_ordering .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kfold_early_stop .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kget_max_key .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .weaken_retries .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .union .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .assume_ordering .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .fold .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .fold_monotone .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .snapshot .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .batch .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .source_interval .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .mapTick .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .firstTick .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .zipTick .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .defer_tick .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .allTicks .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .flattenOrdered .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .flattenUnordered .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .tinto_keyed .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .tassume_ordering .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kentries .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kmap .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kallTicks .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .kfold .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .ksnapshot .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .tfold .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .latest .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .latest_monotone .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .time .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .snapshot_bounded .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .reduce_watermark .., _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .tick_scan .., _ => fun _ h => absurd h List.not_mem_nil

theorem Dag.wireReceivers_mem : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (base : Nat) →
    ∀ e ∈ (Dag.wireReceivers d base).1, base ≤ e.1 ∧ e.1 < (Dag.wireReceivers d base).2
  | _, _, .ret _, _ => fun _ h => absurd h List.not_mem_nil
  | _, _, .letN n rest, base => fun e he => by
    have he : e ∈ (Node.wireReceivers n base).1 ++ (Dag.wireReceivers rest (Node.wireReceivers n base).2).1 := he
    show base ≤ e.1 ∧ e.1 < (Dag.wireReceivers rest (Node.wireReceivers n base).2).2
    rcases List.mem_append.mp he with h | h
    · obtain ⟨h1, h2⟩ := Node.wireReceivers_mem n base e h
      exact ⟨h1, Nat.lt_of_lt_of_le h2 (Dag.wireReceivers_le rest _)⟩
    · obtain ⟨h1, h2⟩ := Dag.wireReceivers_mem rest _ e h
      exact ⟨Nat.le_trans (Node.wireReceivers_le n base) h1, h2⟩

end

/-- A node's messages in a list filtered by a table that routes the node to the
receiver's cluster: all of them. -/
theorem filter_wireForTable (table : List (Nat × L)) (m : Member L mem) (msgs : List WireMsg) (k : Nat)
    (hk : table.find? (fun e => decide (e.1 = k)) = some (k, m.1)) :
    (wireForTable table m msgs).filter (fun msg => decide (msg.1 = k)) = msgs.filter (fun msg => decide (msg.1 = k)) := by
  unfold wireForTable
  rw [List.filter_filter]
  apply List.filter_congr
  intro msg _
  by_cases h : msg.1 = k
  · rw [h, hk]; simp
  · simp [h]

mutual

theorem Node.inboxOfWire_wireForTable (m src : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) → (table : List (Nat × L)) → (msgs : List WireMsg) →
    (∀ k, base ≤ k → k < (Node.wireReceivers n base).2 →
      table.find? (fun e => decide (e.1 = k)) = (Node.wireReceivers n base).1.find? (fun e => decide (e.1 = k))) →
    (Node.inboxOfWire m n cod base [⟨src, wireForTable table m msgs⟩]).1 = (Node.inboxOfWire m n cod base [⟨src, msgs⟩]).1
  | _, _, .broadcast_closed (c := c) (p := p) .., cod, base, table, msgs, h => funext fun i => funext fun j => by
    have hm : m.1 = p := by rw [← i.2]
    have hk : table.find? (fun e => decide (e.1 = base)) = some (base, m.1) := by
      rw [h base (Nat.le_refl _) (Nat.lt_succ_self _), hm]
      show [(base, p)].find? (fun e => decide (e.1 = base)) = some (base, p)
      simp
    show appendFold (fun b : (src : Member L mem) × List WireMsg => if (⟨c, j⟩ : Member L mem) = b.1 then decodeAt cod b.2 base j.val else #[])
        [⟨src, wireForTable table m msgs⟩]
      = appendFold (fun b : (src : Member L mem) × List WireMsg => if (⟨c, j⟩ : Member L mem) = b.1 then decodeAt cod b.2 base j.val else #[])
        [⟨src, msgs⟩]
    rw [appendFold_singleton, appendFold_singleton]
    show (if _ then decodeAt cod (wireForTable table m msgs) base j.val else #[])
      = (if _ then decodeAt cod msgs base j.val else #[])
    rw [decodeAt_of_filter_eq cod _ msgs base j.val (filter_wireForTable table m msgs base hk)]
  | _, _, .demux (c := c) (p := p) _ _ addr, cod, base, table, msgs, h => funext fun i => funext fun j => by
    have hm : m.1 = p := by rw [← i.2]
    have hk : table.find? (fun e => decide (e.1 = base)) = some (base, m.1) := by
      rw [h base (Nat.le_refl _) (Nat.lt_succ_self _), hm]
      show [(base, p)].find? (fun e => decide (e.1 = base)) = some (base, p)
      simp
    show appendFold (fun b : (src : Member L mem) × List WireMsg => if (⟨c, j⟩ : Member L mem) = b.1 then
          filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod b.2 base j.val) else #[])
        [⟨src, wireForTable table m msgs⟩]
      = appendFold (fun b : (src : Member L mem) × List WireMsg => if (⟨c, j⟩ : Member L mem) = b.1 then
          filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod b.2 base j.val) else #[])
        [⟨src, msgs⟩]
    rw [appendFold_singleton, appendFold_singleton]
    show (if _ then filterMapB _ (decodeAt cod (wireForTable table m msgs) base j.val) else #[])
      = (if _ then filterMapB _ (decodeAt cod msgs base j.val) else #[])
    rw [decodeAt_of_filter_eq cod _ msgs base j.val (filter_wireForTable table m msgs base hk)]
  | _, _, .call N _, cod, base, table, msgs, h => Dag.inboxOfWire_wireForTable m src N cod base table msgs h
  | _, _, .fixN _ body, cod, base, table, msgs, h => Dag.inboxOfWire_wireForTable m src body cod base table msgs h
  | _, _, .map .., _, _, _, _, _ => rfl
  | _, _, .filterMap .., _, _, _, _, _ => rfl
  | _, _, .values .., _, _, _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _, _, _ => rfl
  | _, _, .union .., _, _, _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _, _, _ => rfl
  | _, _, .fold .., _, _, _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _, _, _ => rfl
  | _, _, .snapshot .., _, _, _, _, _ => rfl
  | _, _, .batch .., _, _, _, _, _ => rfl
  | _, _, .source_interval .., _, _, _, _, _ => rfl
  | _, _, .mapTick .., _, _, _, _, _ => rfl
  | _, _, .firstTick .., _, _, _, _, _ => rfl
  | _, _, .zipTick .., _, _, _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _, _, _ => rfl
  | _, _, .allTicks .., _, _, _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _, _, _ => rfl
  | _, _, .kentries .., _, _, _, _, _ => rfl
  | _, _, .kmap .., _, _, _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _, _, _ => rfl
  | _, _, .kfold .., _, _, _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _, _, _ => rfl
  | _, _, .tfold .., _, _, _, _, _ => rfl
  | _, _, .latest .., _, _, _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _, _, _ => rfl
  | _, _, .time .., _, _, _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _, _, _ => rfl

theorem Dag.inboxOfWire_wireForTable (m src : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) → (table : List (Nat × L)) → (msgs : List WireMsg) →
    (∀ k, base ≤ k → k < (Dag.wireReceivers d base).2 →
      table.find? (fun e => decide (e.1 = k)) = (Dag.wireReceivers d base).1.find? (fun e => decide (e.1 = k))) →
    (Dag.inboxOfWire m d cod base [⟨src, wireForTable table m msgs⟩]).1 = (Dag.inboxOfWire m d cod base [⟨src, msgs⟩]).1
  | _, _, .ret _, _, _, _, _, _ => rfl
  | _, _, .letN n rest, cod, base, table, msgs, h => by
    have htab : (Dag.wireReceivers (.letN n rest) base).1
        = (Node.wireReceivers n base).1 ++ (Dag.wireReceivers rest (Node.wireReceivers n base).2).1 := rfl
    have hn : ∀ k, base ≤ k → k < (Node.wireReceivers n base).2 →
        table.find? (fun e => decide (e.1 = k)) = (Node.wireReceivers n base).1.find? (fun e => decide (e.1 = k)) := by
      intro k hk1 hk2
      have hz : (Dag.wireReceivers rest (Node.wireReceivers n base).2).1.find? (fun e => decide (e.1 = k)) = none := by
        rw [List.find?_eq_none]
        intro e he
        obtain ⟨h1, _⟩ := Dag.wireReceivers_mem rest _ e he
        simp only [decide_eq_true_eq]
        omega
      rw [h k hk1 (Nat.lt_of_lt_of_le hk2 (Dag.wireReceivers_le rest _)), htab, List.find?_append, hz, Option.or_none]
    have hr : ∀ k, (Node.wireReceivers n base).2 ≤ k → k < (Dag.wireReceivers rest (Node.wireReceivers n base).2).2 →
        table.find? (fun e => decide (e.1 = k))
          = (Dag.wireReceivers rest (Node.wireReceivers n base).2).1.find? (fun e => decide (e.1 = k)) := by
      intro k hk1 hk2
      have hz : (Node.wireReceivers n base).1.find? (fun e => decide (e.1 = k)) = none := by
        rw [List.find?_eq_none]
        intro e he
        obtain ⟨_, h2⟩ := Node.wireReceivers_mem n base e he
        simp only [decide_eq_true_eq]
        omega
      rw [h k (Nat.le_trans (Node.wireReceivers_le n base) hk1) hk2, htab, List.find?_append, hz, Option.none_or]
    show ((Node.inboxOfWire m n cod.1 base [⟨src, wireForTable table m msgs⟩]).1,
        (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base [⟨src, wireForTable table m msgs⟩]).2
          [⟨src, wireForTable table m msgs⟩]).1)
      = ((Node.inboxOfWire m n cod.1 base [⟨src, msgs⟩]).1,
        (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base [⟨src, msgs⟩]).2 [⟨src, msgs⟩]).1)
    rw [Node.inboxOfWire_wireForTable m src n cod.1 base table msgs hn, Node.inboxOfWire_snd_eq,
      Node.inboxOfWire_snd_eq, Dag.inboxOfWire_wireForTable m src rest cod.2 _ table msgs hr]

end

/-- Filtering a sender's messages to the receiver's nodes changes nothing the receiver
reads. -/
theorem Dag.inboxOfWire_wireFor (m src : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)}
    (d : Dag Γ outs) (cod : Dag.Codecs d) (msgs : List WireMsg) :
    (Dag.inboxOfWire m d cod 0 [⟨src, Dag.wireFor m d msgs⟩]).1 = (Dag.inboxOfWire m d cod 0 [⟨src, msgs⟩]).1 :=
  Dag.inboxOfWire_wireForTable m src d cod 0 _ msgs (fun _ _ _ => rfl)

end Project

end Hydro
