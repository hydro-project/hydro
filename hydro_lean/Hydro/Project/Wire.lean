import Hydro.Project.Wired
import Hydro.Wire

/-!
# Hydro · Project — the wire: sends as messages, the online oracle

What a running process needs besides `Dag.locStep`:

- `Dag.Codecs d`: one payload codec per network node. The elaborator builds `M.wireCodecs`
  for each module from the payload types' `Wire` instances (`HydroGenProject.lean`).
- `Dag.wireEncode`/`Dag.wireDecode`: a process's `Sends` as a list of
  `(node, sender, payload)` messages and back. Network nodes are numbered in graph order,
  callee graphs inline.
- `Dag.Timing`/`Dag.oracleOf`: this activation's timing, handed over by the shell, and the
  online oracle, which delivers everything that has arrived at that timing. The oracle is
  feasible by construction: it asks for `delivered + pending.size`. `Dag.timingAt` reads
  the lock-step script at the node counters; in the proof layer it connects the
  script to the readings.
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
abbrev WireMsg := Nat × Nat × ByteArray

/-- A list without duplicates filtered to one of its members: that member's image alone. -/
theorem filterMap_ite_eq_singleton {α β : Type} [DecidableEq α] (c : β) :
    ∀ (l : List α) (j : α), l.Nodup → j ∈ l →
      l.filterMap (fun i => if i = j then some c else none) = [c]
  | [], j, _, hj => absurd hj List.not_mem_nil
  | a :: l, j, hnd, hj => by
    rw [List.nodup_cons] at hnd
    by_cases haj : a = j
    · subst haj
      rw [List.filterMap_cons_some (f := fun i => if i = a then some c else none) (if_pos rfl),
        List.filterMap_eq_nil_iff.mpr]
      intro i hi
      rw [if_neg (fun h : i = a => hnd.1 (h ▸ hi))]
    · rw [List.filterMap_cons_none (f := fun i => if i = j then some c else none) (if_neg haj)]
      exact filterMap_ite_eq_singleton c l j hnd.2 (by
        rcases List.mem_cons.mp hj with h | h
        · exact absurd h.symm haj
        · exact h)

/-- Every member of a location hosted by the process: the one `hostedSelf` finds, if any. -/
def hostedList (m : Member L mem) (ℓ : L) : List (Hosted m ℓ) :=
  (hostedSelf m ℓ).toList

/-- The members hosted at `ℓ` by the member test over all of `ℓ`'s members: the same list,
because the process is one member. -/
theorem hostedList_eq (m : Member L mem) (ℓ : L) :
    hostedList m ℓ = (List.finRange (mem ℓ)).filterMap
      (fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then some ⟨i, h⟩ else none) := by
  unfold hostedList
  obtain ⟨ℓ', j⟩ := m
  by_cases h : ℓ' = ℓ
  · subst h
    rw [hostedSelf_eq ⟨j, rfl⟩]
    -- one index passes the test: `j`
    have : ∀ i : Fin (mem ℓ'), (if h : (⟨ℓ', i⟩ : Member L mem) = ⟨ℓ', j⟩ then some (⟨i, h⟩ : Hosted ⟨ℓ', j⟩ ℓ') else none)
        = if i = j then some ⟨j, rfl⟩ else none := by
      intro i
      by_cases hij : i = j
      · subst hij; simp
      · rw [dif_neg (fun h : (⟨ℓ', i⟩ : Member L mem) = ⟨ℓ', j⟩ => hij (eq_of_heq (Sigma.mk.inj_iff.mp h).2)),
          if_neg hij]
    simp only [this, Option.toList_some]
    rw [filterMap_ite_eq_singleton _ _ _ (List.nodup_finRange _) (List.mem_finRange j)]
  · have hnone : hostedSelf (⟨ℓ', j⟩ : Member L mem) ℓ = none := by simp [hostedSelf, h]
    rw [hnone, Option.toList_none, eq_comm, List.filterMap_eq_nil_iff]
    intro i _
    rw [dif_neg]
    intro he
    exact h (Sigma.mk.inj_iff.mp he).1.symm

mutual

/-- A process's sends as messages, numbering network nodes from `base`; returns the messages
and the next free number. -/
def Node.wireEncode (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Codecs n → Nat → Node.Sends m n → List WireMsg × Nat
  | _, _, .broadcast_closed (c := c) .., cod, base, snd =>
    ((hostedList m c).flatMap (fun j => (snd j).toList.map (fun v => (base, j.1.val, cod.enc v))), base + 1)
  | _, _, .demux (c := c) .., cod, base, snd =>
    ((hostedList m c).flatMap (fun j => (snd j).toList.map (fun v => (base, j.1.val, cod.enc v))), base + 1)
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
  | _, _, .source_interval .., _, base, _ => ([], base)
  | _, _, .mapTick .., _, base, _ => ([], base)
  | _, _, .firstTick .., _, base, _ => ([], base)
  | _, _, .zipTick .., _, base, _ => ([], base)
  | _, _, .defer_tick .., _, base, _ => ([], base)
  | _, _, .allTicks .., _, base, _ => ([], base)
  | _, _, .flattenOrdered .., _, base, _ => ([], base)
  | _, _, .flattenUnordered .., _, base, _ => ([], base)
  | _, _, .tinto_keyed .., _, base, _ => ([], base)
  | _, _, .tassume_ordering .., _, base, _ => ([], base)
  | _, _, .kentries .., _, base, _ => ([], base)
  | _, _, .kmap .., _, base, _ => ([], base)
  | _, _, .kallTicks .., _, base, _ => ([], base)
  | _, _, .kfold .., _, base, _ => ([], base)
  | _, _, .ksnapshot .., _, base, _ => ([], base)
  | _, _, .tfold .., _, base, _ => ([], base)
  | _, _, .latest .., _, base, _ => ([], base)
  | _, _, .latest_monotone .., _, base, _ => ([], base)
  | _, _, .time .., _, base, _ => ([], base)
  | _, _, .snapshot_bounded .., _, base, _ => ([], base)
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

/-- The network nodes in wire order with their receiver location: the routing table a
harness uses to send a node's messages only to the processes hosting receivers. -/
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

/-! The number of wire nodes, `(Dag.wireReceivers d 0).2`, without the table. An activation
needs it to size its decode buckets (`Dag.iterateBytesImpl`), and the table's walk would
append a list per node on every activation. -/
mutual

def Node.wireCount : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Nat → Nat
  | _, _, .broadcast_closed .., base => base + 1
  | _, _, .demux .., base => base + 1
  | _, _, .call N _, base => Dag.wireCount N base
  | _, _, .fixN _ body, base => Dag.wireCount body base
  | _, _, _, base => base

def Dag.wireCount : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Nat → Nat
  | _, _, .ret _, base => base
  | _, _, .letN n rest, base => Dag.wireCount rest (Node.wireCount n base)

end

section
omit [DecidableEq L]

mutual

theorem Node.wireCount_eq : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (base : Nat) →
    Node.wireCount n base = (Node.wireReceivers n base).2
  | _, _, .broadcast_closed .., _ => rfl
  | _, _, .demux .., _ => rfl
  | _, _, .call N _, base => Dag.wireCount_eq N base
  | _, _, .fixN _ body, base => Dag.wireCount_eq body base
  | _, _, .map .., _ => rfl
  | _, _, .filterMap .., _ => rfl
  | _, _, .values .., _ => rfl
  | _, _, .into_keyed .., _ => rfl
  | _, _, .kassume_ordering .., _ => rfl
  | _, _, .kfold_early_stop .., _ => rfl
  | _, _, .kget_max_key .., _ => rfl
  | _, _, .weaken_retries .., _ => rfl
  | _, _, .union .., _ => rfl
  | _, _, .assume_ordering .., _ => rfl
  | _, _, .fold .., _ => rfl
  | _, _, .fold_monotone .., _ => rfl
  | _, _, .snapshot .., _ => rfl
  | _, _, .batch .., _ => rfl
  | _, _, .source_interval .., _ => rfl
  | _, _, .mapTick .., _ => rfl
  | _, _, .firstTick .., _ => rfl
  | _, _, .zipTick .., _ => rfl
  | _, _, .defer_tick .., _ => rfl
  | _, _, .allTicks .., _ => rfl
  | _, _, .flattenOrdered .., _ => rfl
  | _, _, .flattenUnordered .., _ => rfl
  | _, _, .tinto_keyed .., _ => rfl
  | _, _, .tassume_ordering .., _ => rfl
  | _, _, .kentries .., _ => rfl
  | _, _, .kmap .., _ => rfl
  | _, _, .kallTicks .., _ => rfl
  | _, _, .kfold .., _ => rfl
  | _, _, .ksnapshot .., _ => rfl
  | _, _, .tfold .., _ => rfl
  | _, _, .latest .., _ => rfl
  | _, _, .latest_monotone .., _ => rfl
  | _, _, .time .., _ => rfl
  | _, _, .snapshot_bounded .., _ => rfl
  | _, _, .reduce_watermark .., _ => rfl
  | _, _, .tick_scan .., _ => rfl

theorem Dag.wireCount_eq : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (base : Nat) →
    Dag.wireCount d base = (Dag.wireReceivers d base).2
  | _, _, .ret _, _ => rfl
  | _, _, .letN n rest, base => by
    show Dag.wireCount rest (Node.wireCount n base) = (Dag.wireReceivers rest (Node.wireReceivers n base).2).2
    rw [Node.wireCount_eq n base, Dag.wireCount_eq rest]

end

end

/-! ### The routing table with the `demux` addresses

A `demux` message carries its target member's address as the first field of its payload:
`Node.Sends` of a `demux` is `Array (Nat × α)`, encoded by the `Prod` instance as the
address's varint, then the value. The receiver keeps only the messages addressed to it
(`Node.inboxOfWire`), so a harness that sends every `demux` message to every process hosting
the receiver cluster makes each of them decode messages it then drops. `wireRoutes` is
`wireReceivers` with, at a `demux` node, the address of each member of the receiver cluster,
so a shell can send a message to the one process its address names (`Shell.routeSendsIdx`).
What a receiver reads is unchanged, and the headline theorems quantify over every byte
history. -/

mutual

def Node.wireRoutes : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Nat →
    List (Nat × L × Option (Nat → Option Nat)) × Nat
  | _, _, .broadcast_closed (p := p) .., base => ([(base, p, none)], base + 1)
  | _, _, .demux (p := p) _ _ addr, base =>
    ([(base, p, some fun i => if h : i < mem p then some (addr ⟨i, h⟩) else none)], base + 1)
  | _, _, .call N _, base => Dag.wireRoutes N base
  | _, _, .fixN _ body, base => Dag.wireRoutes body base
  | _, _, _, base => ([], base)

def Dag.wireRoutes : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Nat →
    List (Nat × L × Option (Nat → Option Nat)) × Nat
  | _, _, .ret _, base => ([], base)
  | _, _, .letN n rest, base =>
    let r := Node.wireRoutes n base
    let r' := Dag.wireRoutes rest r.2
    (r.1 ++ r'.1, r'.2)

end

/-- The address a `demux` payload names: its first varint. -/
def demuxTarget (bytes : ByteArray) : Option Nat :=
  match Hydro.Wire.getNat bytes 0 with
  | .ok (t, _) => some t
  | .error _ => none

/-- The peek reads the address. A `demux` payload is a `Codec.ofWire` of the `Prod` instance
(`HydroGenProject`: every network payload codec is `Codec.ofWire`), whose bytes are the
address's varint then the value's (`Wire.getNat_encode_cont`). Which member each process
hosts remains a harness fact (`Shell.routeIndex`'s table). -/
theorem demuxTarget_ofWire {α : Type} [Wire α] (a : Nat) (x : α) :
    demuxTarget ((Codec.ofWire (α := Nat × α)).enc (a, x)) = some a := by
  rw [Codec.ofWire_enc]
  show demuxTarget (Wire.encode a ++ Wire.encode x) = some a
  have h := Wire.getNat_encode_cont a ByteArray.empty (Wire.encode x) ByteArray.empty
  rw [ByteArray.empty_append, ByteArray.append_empty, ByteArray.size_empty] at h
  unfold demuxTarget
  rw [h]

/-- `List.filterMap` straight into a batch, with no intermediate list. -/
def filterMapToArray {α β : Type} (f : α → Option β) (l : List α) : Array β :=
  l.foldl (fun acc x => match f x with | some y => acc.push y | none => acc) #[]

theorem foldl_filterMap_push {α β : Type} (f : α → Option β) : ∀ (l : List α) (acc : Array β),
    (l.foldl (fun acc x => match f x with | some y => acc.push y | none => acc) acc).toList
      = acc.toList ++ l.filterMap f
  | [], acc => by simp
  | x :: l, acc => by
    rw [List.foldl_cons, List.filterMap_cons]
    cases f x with
    | none => simp only; rw [foldl_filterMap_push f l acc]
    | some y => simp only; rw [foldl_filterMap_push f l, Array.toList_push, List.append_assoc, List.singleton_append]

@[simp] theorem toList_filterMapToArray {α β : Type} (f : α → Option β) (l : List α) :
    (filterMapToArray f l).toList = l.filterMap f := by
  unfold filterMapToArray
  rw [foldl_filterMap_push, Array.toList_empty, List.nil_append]

/-- The messages of node `k` from sender `j`, decoded; undecodable ones are dropped. -/
def decodeAt {α : Type} (cod : Codec α) (msgs : List WireMsg) (k : Nat) (j : Nat) : Array α :=
  filterMapToArray (fun m => if m.1 = k ∧ m.2.1 = j then (cod.dec m.2.2).toOption else none) msgs

@[simp] theorem decodeAt_toList {α : Type} (cod : Codec α) (msgs : List WireMsg) (k : Nat) (j : Nat) :
    (decodeAt cod msgs k j).toList
      = msgs.filterMap fun m => if m.1 = k ∧ m.2.1 = j then (cod.dec m.2.2).toOption else none :=
  toList_filterMapToArray _ _

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

def Dag.wireDecode (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Codecs d → Nat → List WireMsg → Dag.Sends m d × Nat
  | _, _, .ret _, _, base, _ => ((), base)
  | _, _, .letN n rest, cod, base, msgs =>
    let r := Node.wireDecode m n cod.1 base msgs
    let r' := Dag.wireDecode m rest cod.2 r.2 msgs
    ((r.1, r'.1), r'.2)

end

/-! ## The online oracle -/

/-! ## This activation's timing, and the online oracle

A process is handed, per activation, this tick's timing for every timer node at a member it
hosts (`Dag.Timing`): the shell reads its clock and timers once per activation and never
keeps a history. The oracle a step runs with is then the network part read off the queues
(deliver everything that has arrived) plus that timing (`oracleOf`). The lock-step
machine's script meets the process in one definition: `timingAt` reads each script at the
node's own tick count, and `oracleAll sch s = oracleOf (timingAt sch s) s` by unfolding, so
every theorem about `Dag.iterate` at `timingAt` is a theorem about a process whose per-tick
readings form that script, and any run's readings form some script. -/

mutual

/-- The per-activation timing of a timer node at the hosted members: the clock reading,
the interval timer's fire count. -/
def Node.Timing (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .source_interval (ℓ := ℓ) _ _ => Hosted m ℓ → Nat
  | _, _, .time (ℓ := ℓ) => Hosted m ℓ → Nat
  | _, _, .call N _ => Dag.Timing m N
  | _, _, .fixN _ body => Dag.Timing m body
  | _, _, _ => Unit

def Dag.Timing (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.Timing m n × Dag.Timing m rest

end

/-- This activation's timing from per-location readings, the values a shell has in hand
after reading its clock and timers once: the clock reading, and for every interval timer
(`source_interval`, identified by its delay and period at the member) how many times it
fired since the last activation. Two interval nodes with the same delay and period at one
location read the same timer; `paxos_bench`'s heartbeat and election intervals at a
proposer differ in both. -/
structure TickReadings (m : Member L mem) where
  clock : (ℓ : L) → Hosted m ℓ → Nat
  fires : (ℓ : L) → (i : Hosted m ℓ) → (delay period : Nat) → Nat

/-! ### A shell's timers

An interval timer `source_interval_delayed(delay, period)` fires at `delay, delay + period,
delay + 2·period, …` on the process's clock; `source_interval(period)` is `delay = 0`, firing
at once. The tick's batch of it holds one `()` per fire instant in `(last, now]`, `last` the
previous activation's reading, none at the first: `tokio::time::interval`'s default
`MissedTickBehavior::Burst` delivers every missed instant at once. The readings a shell has
after reading its clock once are then a function of `now` and `last`. -/

/-- The number of fire instants at or before `t`: none before `delay`; `period = 0`
degenerates to one. -/
def firesUpTo (t delay period : Nat) : Nat :=
  if t < delay then 0 else (t - delay) / period + 1

/-- How many times the interval timer fired in `(last, now]`; without a `last`, at or before
`now`. -/
def fireCount (last : Option Nat) (now delay period : Nat) : Nat :=
  match last with
  | none => firesUpTo now delay period
  | some l => firesUpTo now delay period - firesUpTo l delay period

/-- Whether the interval timer fired in `(last, now]`; without a `last`, at or before `now`. -/
def intervalFired (last : Option Nat) (now delay period : Nat) : Bool :=
  fireCount last now delay period != 0

/-- The timer's first fire instant after `last`, at or after `delay`. -/
def nextFire (last : Option Nat) (delay period : Nat) : Nat :=
  match last with
  | none => delay
  | some l => if l < delay then delay else delay + ((l - delay) / period + 1) * period

/-- This activation's readings from the process's clock: the reading `now` at every clock
node, and at every interval timer how many times it fired since the previous activation. -/
def TickReadings.ofClock (m : Member L mem) (now : Nat) (last : Option Nat) : TickReadings m :=
  { clock := fun _ _ => now
    fires := fun _ _ delay period => fireCount last now delay period }

mutual

/-- The interval timers of the member's hosted `source_interval` nodes: each hosted
member's delay, and the period. -/
def Node.timers (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → List (Nat × Nat)
  | _, _, .source_interval (ℓ := ℓ) delay period => (hostedList m ℓ).map fun i => (delay i.1, period)
  | _, _, .call N _ => Dag.timers m N
  | _, _, .fixN _ body => Dag.timers m body
  | _, _, _ => []

def Dag.timers (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → List (Nat × Nat)
  | _, _, .ret _ => []
  | _, _, .letN n rest => Node.timers m n ++ Dag.timers m rest

end

/-- Whether any of the timers fired in `(last, now]`. -/
def timersDue (timers : List (Nat × Nat)) (last : Option Nat) (now : Nat) : Bool :=
  timers.any fun (d, p) => intervalFired last now d p

/-- The earliest next fire among the timers; `none` without timers. -/
def nextDue (timers : List (Nat × Nat)) (last : Option Nat) : Option Nat :=
  timers.foldl (fun acc (d, p) =>
    let n := nextFire last d p
    match acc with | none => some n | some a => some (min a n)) none

mutual

def Node.Timing.ofReadings (m : Member L mem) (r : TickReadings m) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Timing m n
  | _, _, .source_interval (ℓ := ℓ) delay period => fun i => r.fires ℓ i (delay i.1) period
  | _, _, .time (ℓ := ℓ) => r.clock ℓ
  | _, _, .call N _ => Dag.Timing.ofReadings m r N
  | _, _, .fixN _ body => Dag.Timing.ofReadings m r body
  | _, _, .broadcast_closed .. => ()
  | _, _, .demux .. => ()
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
  | _, _, .mapTick .. => ()
  | _, _, .firstTick .. => ()
  | _, _, .zipTick .. => ()
  | _, _, .defer_tick .. => ()
  | _, _, .allTicks .. => ()
  | _, _, .flattenOrdered .. => ()
  | _, _, .flattenUnordered .. => ()
  | _, _, .tinto_keyed .. => ()
  | _, _, .tassume_ordering .. => ()
  | _, _, .reduce_watermark .. => ()
  | _, _, .kentries .. => ()
  | _, _, .kmap .. => ()
  | _, _, .kallTicks .. => ()
  | _, _, .kfold .. => ()
  | _, _, .ksnapshot .. => ()
  | _, _, .tfold .. => ()
  | _, _, .latest .. => ()
  | _, _, .latest_monotone .. => ()
  | _, _, .snapshot_bounded .. => ()
  | _, _, .tick_scan .. => ()

def Dag.Timing.ofReadings (m : Member L mem) (r : TickReadings m) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Timing m d
  | _, _, .ret _ => ()
  | _, _, .letN n rest => (Node.Timing.ofReadings m r n, Dag.Timing.ofReadings m r rest)

end

mutual

/-- The online oracle: deliver everything that has arrived, so the cursor is the queue's
reach and feasible by construction, and this activation's timing at the timer nodes. -/
def Node.oracleOf (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Timing m n → Node.LocState m n → Node.Oracle m n
  | _, _, .broadcast_closed .., _, s => fun i j => (s i j).delivered + (s i j).pending.size
  | _, _, .demux .., _, s => fun i j => (s i j).delivered + (s i j).pending.size
  | _, _, .source_interval .., tm, _ => tm
  | _, _, .time .., tm, _ => tm
  | _, _, .call N _, tm, s => Dag.oracleOf m N tm s
  | _, _, .fixN _ body, tm, s => Dag.oracleOf m body tm s
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
  | _, _, .mapTick .., _, _ => ()
  | _, _, .firstTick .., _, _ => ()
  | _, _, .zipTick .., _, _ => ()
  | _, _, .defer_tick .., _, _ => ()
  | _, _, .allTicks .., _, _ => ()
  | _, _, .flattenOrdered .., _, _ => ()
  | _, _, .flattenUnordered .., _, _ => ()
  | _, _, .tinto_keyed .., _, _ => ()
  | _, _, .tassume_ordering .., _, _ => ()
  | _, _, .reduce_watermark .., _, _ => ()
  | _, _, .kentries .., _, _ => ()
  | _, _, .kmap .., _, _ => ()
  | _, _, .kallTicks .., _, _ => ()
  | _, _, .kfold .., _, _ => ()
  | _, _, .ksnapshot .., _, _ => ()
  | _, _, .tfold .., _, _ => ()
  | _, _, .latest .., _, _ => ()
  | _, _, .latest_monotone .., _, _ => ()
  | _, _, .snapshot_bounded .., _, _ => ()
  | _, _, .tick_scan .., _, _ => ()

def Dag.oracleOf (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Timing m d → Dag.LocState m d → Dag.Oracle m d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, tm, s => (Node.oracleOf m n tm.1 s.1, Dag.oracleOf m rest tm.2 s.2)

end

mutual

/-- The lock-step machine's script read at this activation: each timer node's script at the
node's tick count, which is its cell. This is the only place the schedule meets a process. -/
def Node.timingAt (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Sched n → Node.LocState m n → Node.Timing m n
  | _, _, .source_interval .., d, s => fun i => d i.1 (s i)
  | _, _, .time .., d, s => fun i => d i.1 (s i)
  | _, _, .call N _, sch, s => Dag.timingAt m N sch s
  | _, _, .fixN _ body, sch, s => Dag.timingAt m body sch s
  | _, _, .broadcast_closed .., _, _ => ()
  | _, _, .demux .., _, _ => ()
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
  | _, _, .mapTick .., _, _ => ()
  | _, _, .firstTick .., _, _ => ()
  | _, _, .zipTick .., _, _ => ()
  | _, _, .defer_tick .., _, _ => ()
  | _, _, .allTicks .., _, _ => ()
  | _, _, .flattenOrdered .., _, _ => ()
  | _, _, .flattenUnordered .., _, _ => ()
  | _, _, .tinto_keyed .., _, _ => ()
  | _, _, .tassume_ordering .., _, _ => ()
  | _, _, .reduce_watermark .., _, _ => ()
  | _, _, .kentries .., _, _ => ()
  | _, _, .kmap .., _, _ => ()
  | _, _, .kallTicks .., _, _ => ()
  | _, _, .kfold .., _, _ => ()
  | _, _, .ksnapshot .., _, _ => ()
  | _, _, .tfold .., _, _ => ()
  | _, _, .latest .., _, _ => ()
  | _, _, .latest_monotone .., _, _ => ()
  | _, _, .snapshot_bounded .., _, _ => ()
  | _, _, .tick_scan .., _, _ => ()

def Dag.timingAt (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Sched d → Dag.LocState m d → Dag.Timing m d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, sch, s => (Node.timingAt m n sch.1 s.1, Dag.timingAt m rest sch.2 s.2)

end

/-- The oracle of the lock-step machine's schedule at a process: the queues' reach and the
script at the counters. It belongs to the proof layer; a process never computes it. -/
@[reducible] def Node.oracleAll (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (sch : Node.Sched n) (s : Node.LocState m n) : Node.Oracle m n :=
  Node.oracleOf m n (Node.timingAt m n sch s) s

@[reducible] def Dag.oracleAll (m : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (sch : Dag.Sched d) (s : Dag.LocState m d) : Dag.Oracle m d :=
  Dag.oracleOf m d (Dag.timingAt m d sch s) s

theorem Dag.oracleAll_letN (m : Member L mem) {Γ : Ctx L mem} {κs outs : List (Kind L mem)} (n : Node Γ κs)
    (rest : Dag (κs ++ Γ) outs) (sch : Dag.Sched (.letN n rest)) (s : Dag.LocState m (.letN n rest)) :
    Dag.oracleAll m (.letN n rest) sch s = (Node.oracleAll m n sch.1 s.1, Dag.oracleAll m rest sch.2 s.2) := rfl

end Project

end Hydro
