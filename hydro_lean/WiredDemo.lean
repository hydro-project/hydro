import Hydro
import Hydro.Project.Wire
import Hydro.Paxos.WiredSafety
import Std.Async.TCP
import Std.Sync.Mutex
import Std.Data.HashSet
import Std.Data.HashMap

/-!
# `wired` — `paxos_core` as five processes over local TCP (D74)

Two proposers and three acceptors, each a process running
`Dag.locStep` on **its own cells only** (`Dag.LocState`), talking over
real TCP sockets on `127.0.0.1`: every process listens on a port, dials
the other four, and each activation's `Sends` go out as JSON lines
(`Dag.wireEncode`, the codecs `paxos_core.wireCodecs` the elaborator
built from the message types' `ToJson`). A receiver drains what has
arrived into its queues (`Dag.enqueueLoc`) and delivers everything that
has arrived (`Dag.oracleAll`); **an activation is one tick** of all its
members. Nothing settles: a knot's pending feedback is state the next tick
consumes, and the next tick comes with the next arrival (every cycle in
`paxos_core` crosses the network) or the idle timer. A process that is not
running is inert, so the run is an `Interleaving` as it stands (D76).

What is checked: the proposers commit a client payload and every commit
any process emits at one slot agrees — the statement `paxos_safe_wired`
makes about **exactly this** execution model. Each process is its own
thread; nothing is shared between them but the sockets (and a stop flag
the harness sets). Each process keeps its own record of what it
committed; the harness reads the records only after the stop.
-/

open Hydro Hydro.Project Lean Std Std.Async

namespace WiredDemo

abbrev Mem := paxMem 2 3

/-- The pacing binder of the graph — unused by the processes (ticks are
their own), present because the module took `H`. -/
def pacing0 : Pacing PaxLoc Mem := fun _ _ _ => .idle

abbrev HS := SchedSem PaxLoc Mem pacing0

/-- Heartbeat timing: proposer 0 times out and pulses at every tick (it
contends and leads); proposer 1 never does (a follower). Gossip samples
every tick. -/
def hbDec : PLHDec HS 2 :=
  { sample := fun _ => List.range 64
    timeout := fun i => if i.val = 0 then List.replicate 64 true else List.replicate 64 false
    interval := fun i => if i.val = 0 then List.replicate 64 true else List.replicate 64 false }

/-- The content decisions at `SchedSem` are all `Unit` (ticks are the
batches); only the heartbeat timing carries data. -/
def sdec : PaxosCoreDec HS 2 3 Nat Nat .totalOrder :=
  { le := { receivedMax := (), hb := hbDec, p1aBatch := (), p1b := { cqwr := (), order := (), snap := () }
            fuelFail := (), fuelIAL := (), fuelLead := () }
    sp := { payloadBatch := (), rc := ⟨()⟩, ap2 := { p2aBatch := (), ckSnap := () }, cqBatch := (), jrBatch := () }
    fuelSeqMax := (), fuelALog := () }

/-- The delivery cursors baked into the graph are the oracle the lock-step
reads; the processes read theirs online (`Dag.oracleAll`), so these are
placeholders — the correspondence theorem's `OracleAt` is about the
cursors that *equal* what the run realized. -/
def ssched : PaxosCoreSched HS 2 3 Nat :=
  { le := { p1aCh := fun _ _ _ => 0, hb := { ial := fun _ _ _ => 0 }, ap1 := { p1bCh := fun _ _ _ => 0 } }
    sp := { p2aCh := fun _ _ _ => 0, ap2 := { p2bCh := fun _ _ _ => 0 } } }

/-- The graph. -/
def D := paxos_core.dag (ckret := .exactlyOnce) pacing0 .guarded .prop .acc 1 sdec ssched

/-- Its wire codecs. -/
def codecs : Dag.Codecs D := paxos_core.wireCodecs (ckret := .exactlyOnce) pacing0 .guarded .prop .acc 1 sdec ssched

/-! ## Five processes -/

abbrev NProc := 5

/-- Process `p` is proposer `p` (p < 2) or acceptor `p − 2`. -/
def memberOf : Fin NProc → Member PaxLoc Mem
  | ⟨0, _⟩ => ⟨.prop, ⟨0, by decide⟩⟩
  | ⟨1, _⟩ => ⟨.prop, ⟨1, by decide⟩⟩
  | ⟨2, _⟩ => ⟨.acc, ⟨0, by decide⟩⟩
  | ⟨3, _⟩ => ⟨.acc, ⟨1, by decide⟩⟩
  | ⟨4, _⟩ => ⟨.acc, ⟨2, by decide⟩⟩

def selOf (p : Fin NProc) : Sel PaxLoc Mem := Sel.single (memberOf p)

abbrev Loc (p : Fin NProc) := Dag.LocState (selOf p) D

/-- The graph's input context: client payloads (at proposers), checkpoints (at acceptors). -/
abbrev Γ0 := [Kind.stream (mem := Mem) .prop Nat inferInstance .totalOrder .exactlyOnce,
  Kind.sing (mem := Mem) .acc Nat (Option Nat) inferInstance .totalOrder .exactlyOnce (.monotonic ValueOrder.optNat)]

/-- No new payloads; checkpoints read `none`. -/
def emptyInp : HList (Kind.delta (mem := Mem)) Γ0 := (fun _ => [], (fun _ => none, ()))

/-- Process `p`'s inputs this activation: its payloads (at a proposer) and its
checkpoint (at an acceptor: the replicas' latest, `paxos_bench.rs:69–98`; a
singleton re-emits its value every step). -/
def inputsFor (p : Fin NProc) (payloads : List Nat) (ck : Option Nat) : HList (Kind.locDelta (selOf p)) Γ0 :=
  (fun _ => payloads, (fun _ => ck, ()))

/-- Every process starts from the graph's initial cells (its `defer_tick`s
fire at its own first tick). -/
def init (p : Fin NProc) : Loc p := Dag.memoState (selOf p) D (Dag.restrict (selOf p) D (Dag.init D))

/-- The result of one activation. -/
structure Act (p : Fin NProc) where
  cells : Loc p
  ballots : List (Ballot 2)
  commits : List (Nat × Option Nat)
  sends : List WireMsg

/-- The outputs at the process's proposers. -/
def outsAt (p : Fin NProc) (o : HList (Kind.locDelta (selOf p))
    [Kind.stream (mem := Mem) .prop (Ballot 2) instDecidableEqBallot .totalOrder .exactlyOnce,
     Kind.stream (mem := Mem) .prop (Nat × Option Nat) inferInstance .noOrder .exactlyOnce]) :
    List (Ballot 2) × List (Nat × Option Nat) :=
  ((hostedList (selOf p) .prop).flatMap (fun i => o.get Var.zero i),
   (hostedList (selOf p) .prop).flatMap (fun i => o.get Var.zero.succ i))

/-- **One activation of process `p`** — pure: drain the inbox into the
queues, one tick step with the payloads. The oracle delivers everything that
has arrived. -/
def activate (p : Fin NProc) (inbox : List (Fin NProc × List WireMsg)) (payloads : List Nat) (ck : Option Nat)
    (s : Loc p) : Act p :=
  let s := Dag.memoState (selOf p) D (inbox.foldl (fun s (fm, msgs) =>
    Dag.enqueueLoc (selOf p) (selOf fm) D (Dag.wireDecode (selOf fm) D codecs 0 msgs).1 s) s)
  let r := Dag.locStep (selOf p) true D (Dag.oracleAll (selOf p) D s) (inputsFor p payloads ck) s
  let o := outsAt p r.2.1
  ⟨r.1, o.1, o.2, (Dag.wireEncode (selOf p) D codecs 0 r.2.2).1⟩

/-- Array access in `IO` (no `Inhabited` needed). -/
def idx {α : Type} (a : Array α) (i : Nat) : IO α :=
  match a[i]? with
  | some x => pure x
  | none => throw (IO.userError s!"wired: index {i} out of range")

/-! ## The wire -/

/-- One line on the wire: the sender and one sub-step's messages. -/
structure Line where
  fm : Nat
  msgs : List WireMsg
  /-- A replica checkpoint for the acceptors (`paxos_bench.rs`: the replicas broadcast
  their checkpointed sequence; the harness's replica is colocated with the leader). -/
  ckpt : Option Nat := none
deriving ToJson, FromJson

def basePort : Nat := 9400

def addrOf (p : Nat) : Net.SocketAddress :=
  .v4 ⟨Net.IPv4Addr.ofParts 127 0 0 1, (basePort + p).toUInt16⟩

/-- A process's own record of one activation: its commits and, for `--bench`,
the step's duration. -/
structure Act' where
  commits : List (Nat × Option Nat)
  nanos : Nat

/-- Read newline-separated JSON lines off a socket into the inbox. -/
partial def reader (client : TCP.Socket.Client) (inbox : Mutex (Array (Fin NProc × List WireMsg)))
    (ckRef : IO.Ref (Option Nat)) (buf : ByteArray) : IO Unit := do
  match ← (client.recv? 65536).block with
  | none => return
  | some bytes =>
    let buf := buf ++ bytes
    let s := String.fromUTF8! buf
    let parts := s.splitOn "\n"
    let fullLines := parts.dropLast
    let rest := parts.getLastD ""
    for line in fullLines do
      if line.isEmpty then continue
      match Json.parse line >>= fromJson? (α := Line) with
      | .ok ln =>
        if let some c := ln.ckpt then
          -- the acceptors reduce the replicas' checkpoints by max (`paxos_bench.rs:72–78`)
          ckRef.modify fun o => some (max c (o.getD 0))
        if h : ln.fm < NProc then
          if !ln.msgs.isEmpty then
            inbox.atomically (modify fun a => a.push (⟨ln.fm, h⟩, ln.msgs))
        else IO.eprintln s!"wired: bad sender {ln.fm}"
      | .error e => IO.eprintln s!"wired: bad line ({e}): {line.take 80}"
    reader client inbox ckRef rest.toUTF8

/-- Accept the four peers and read from each. -/
partial def acceptLoop (server : TCP.Socket.Server) (inbox : Mutex (Array (Fin NProc × List WireMsg)))
    (ckRef : IO.Ref (Option Nat)) (n : Nat) : IO Unit := do
  if n = 0 then return
  let client ← (server.accept).block
  let _ ← IO.asTask (reader client inbox ckRef .empty) (prio := .dedicated)
  acceptLoop server inbox ckRef (n - 1)

/-- Dial a peer, retrying while it is not yet listening. -/
partial def dial (q : Nat) (tries : Nat) : IO TCP.Socket.Client := do
  let c ← TCP.Socket.Client.mk
  try
    (c.connect (addrOf q)).block
    return c
  catch e =>
    if tries = 0 then throw e
    IO.sleep 20
    dial q (tries - 1)

def lineBytes (ln : Line) : ByteArray := ((toJson ln).compress ++ "\n").toUTF8

def sendLine (c : TCP.Socket.Client) (ln : Line) : IO Unit :=
  (c.send (lineBytes ln)).block

/-- Which cluster process `q` hosts. -/
def hosts (q : Nat) : PaxLoc → Bool
  | .prop => decide (q < 2)
  | .acc => decide (2 ≤ q)

instance : BEq PaxLoc := ⟨fun a b => decide (a = b)⟩

/-- The receiver cluster of wire node `k` (the routing table). -/
def recvTable : Array (Option PaxLoc) :=
  let rs := (Dag.wireReceivers D 0).1
  (List.range (Dag.wireReceivers D 0).2).toArray.map fun k => (rs.find? (·.1 = k)).map (·.2)

def recvOf (k : Nat) : Option PaxLoc := (recvTable[k]?).bind id

/-- Run configuration: the demo (the leader proposes one payload once elected)
or a throughput bench (`--bench total batch`: the leader proposes `batch`
fresh payloads per activation until `total` are in flight; the run ends when
it has seen them all committed). -/
structure Cfg where
  total : Nat := 1
  batch : Nat := 1
  bench : Bool := false
  deadlineMs : Nat := 10000
  /-- `checkpoint_frequency` (`paxos_bench.rs`): the replica checkpoints every this many
  applied slots; `0` = no checkpoints (the acceptor log grows without bound). -/
  ckFreq : Nat := 0

/-- **One machine**, on its own thread. Loops independently of every other
process: drain whatever has arrived on its sockets, tick once, flush this
activation's sends; records its commits in `rec` (read by the harness only after
the stop). An idle machine ticks on a local timer (the proposers' timeouts
are timer-driven in the Rust too). The leader (the process that saw its own
election on the `ballots` output) injects client payloads. -/
partial def machine (cfg : Cfg) (p : Fin NProc) (peers : Array (Nat × TCP.Socket.Client))
    (inbox : Mutex (Array (Fin NProc × List WireMsg))) (stop : IO.Ref Bool) (rec : IO.Ref (Array Act'))
    (stRef : IO.Ref (Loc p)) (leader injected committed : IO.Ref Nat)
    (ckRef : IO.Ref (Option Nat)) (replica : IO.Ref (Nat × Nat × Std.HashSet Nat)) : IO Unit := do
  if (← stop.get) then return
  let drained ← inbox.atomically (do
    let a : Array (Fin NProc × List WireMsg) ← get
    set (#[] : Array (Fin NProc × List WireMsg))
    pure a)
  let s ← stRef.get
  let payloads ← do
    let inj ← injected.get
    if (← leader.get) = 0 || inj ≥ cfg.total then pure []
    else
      let k := min cfg.batch (cfg.total - inj)
      injected.set (inj + k)
      pure ((List.range k).map (fun i => 1000 * (p.val + 1) + inj + i))
  let ck ← ckRef.get
  let t0 ← IO.monoNanosNow
  -- phase 1: decode what arrived and enqueue it; phase 2: tick
  let s1 ← IO.lazyPure fun _ => Dag.memoState (selOf p) D (drained.toList.foldl (fun s (fm, msgs) =>
    Dag.enqueueLoc (selOf p) (selOf fm) D (Dag.wireDecode (selOf fm) D codecs 0 msgs).1 s) s)
  let tq ← IO.monoNanosNow
  let a ← IO.lazyPure fun _ => activate p [] payloads ck s1
  stRef.set a.cells
  let t1 ← IO.monoNanosNow
  let dt := t1 - t0
  if cfg.bench && dt > 5000000 then
    IO.eprintln s!"wired: process {p}: decode+enqueue {(tq - t0) / 1000} µs ({drained.size} lines, {drained.foldl (fun a e => a + e.2.length) 0} msgs), step {(t1 - tq) / 1000} µs"
  if !a.ballots.isEmpty then leader.set 1
  committed.modify (· + a.commits.length)
  -- flush — encoded once per receiver cluster and sent only to the processes
  -- hosting a receiver of the message's node (`Dag.wireReceivers`)
  if !a.sends.isEmpty then
    for ℓ in [PaxLoc.prop, PaxLoc.acc] do
      let mine := a.sends.filter fun m => recvOf m.1 == some ℓ
      if mine.isEmpty then continue
      let bytes := lineBytes ⟨p.val, mine, none⟩
      for (q, c) in peers do
        if hosts q ℓ then (c.send bytes).block
  rec.modify (·.push ⟨a.commits, dt⟩)
  -- the harness's replica (colocated with the leader, `kv_replica.rs:81–104`): apply the
  -- committed prefix; every `ckFreq` slots, checkpoint `next_slot` to the acceptors
  if cfg.ckFreq > 0 && !a.commits.isEmpty then
    let (next, lastCk, pending) ← replica.get
    let pending := a.commits.foldl (fun st c => st.insert c.1) pending
    let mut next := next
    let mut pending := pending
    while pending.contains next do
      pending := pending.erase next
      next := next + 1
    let mut lastCk := lastCk
    if next - lastCk ≥ cfg.ckFreq then
      lastCk := next
      for (q, c) in peers do
        if q ≥ 2 then sendLine c ⟨p.val, [], some next⟩
    replica.set (next, lastCk, pending)
  if !cfg.bench && (!a.ballots.isEmpty || !a.commits.isEmpty || !payloads.isEmpty) then
    IO.eprintln s!"wired: process {p}: inbox {drained.size} payloads {payloads} → ballots {repr a.ballots}, commits {repr a.commits} ({dt / 1000} µs)"
  -- the leader has seen everything it proposed committed: tell the harness
  if (← leader.get) = 1 && (← committed.get) ≥ cfg.total then
    stop.set true
  -- idle backoff (a local timer), nothing to do with the other machines
  if drained.isEmpty && payloads.isEmpty then IO.sleep 1
  machine cfg p peers inbox stop rec stRef leader injected committed ckRef replica

def parseCfg : List String → Cfg
  | ["--bench", t, b] => { total := t.toNat?.getD 1000, batch := b.toNat?.getD 10, bench := true, deadlineMs := 60000 }
  | ["--bench", t, b, c] => { total := t.toNat?.getD 1000, batch := b.toNat?.getD 10, bench := true, deadlineMs := 60000,
                              ckFreq := c.toNat?.getD 0 }
  | _ => {}

def main (args : List String) : IO UInt32 := do
  let cfg := parseCfg args
  IO.eprintln s!"wired: paxos_core as {NProc} processes (2 proposers, 3 acceptors) over TCP 127.0.0.1:{basePort}+p{if cfg.bench then s!" — bench: {cfg.total} payloads, batch {cfg.batch}, checkpoint every {cfg.ckFreq}" else ""}"
  -- servers first
  let mut servers : Array TCP.Socket.Server := #[]
  for p in [0:NProc] do
    let srv ← TCP.Socket.Server.mk
    srv.bind (addrOf p)
    srv.listen 8
    servers := servers.push srv
  let inboxes ← (List.finRange NProc).toArray.mapM fun _ => Mutex.new (#[] : Array (Fin NProc × List WireMsg))
  let ckRefs ← (List.finRange NProc).toArray.mapM fun _ => IO.mkRef (none : Option Nat)
  let accepts ← (List.finRange NProc).toArray.mapM fun p => do
    let srv ← idx servers p.val
    let inb ← idx inboxes p.val
    let ckr ← idx ckRefs p.val
    IO.asTask (acceptLoop srv inb ckr (NProc - 1)) (prio := .dedicated)
  -- dial the peers
  let mut peersOf : Array (Array (Nat × TCP.Socket.Client)) := #[]
  for p in [0:NProc] do
    let mut peers : Array (Nat × TCP.Socket.Client) := #[]
    for q in [0:NProc] do
      if q ≠ p then peers := peers.push (q, ← dial q 100)
    peersOf := peersOf.push peers
  for t in accepts do
    match ← IO.wait t with
    | .ok () => pure ()
    | .error e => throw e
  IO.eprintln "wired: all sockets connected"
  let stop ← IO.mkRef false
  let recs ← (List.finRange NProc).toArray.mapM fun _ => IO.mkRef (#[] : Array Act')
  let t0 ← IO.monoMsNow
  let tasks ← (List.finRange NProc).toArray.mapM fun p => do
    let stRef ← IO.mkRef (init p)
    let peers ← idx peersOf p.val
    let inb ← idx inboxes p.val
    let ckr ← idx ckRefs p.val
    let rec ← idx recs p.val
    IO.asTask (machine cfg p peers inb stop rec stRef (← IO.mkRef 0) (← IO.mkRef 0) (← IO.mkRef 0) ckr
      (← IO.mkRef (0, 0, ∅))) (prio := .dedicated)
  -- the harness waits for the leader's signal (or the deadline), then stops every machine
  for _ in [0:cfg.deadlineMs] do
    if (← stop.get) then break
    IO.sleep 1
  let t1 ← IO.monoMsNow
  stop.set true
  for t in tasks do
    match ← IO.wait t with
    | .ok () => pure ()
    | .error e => throw e
  -- after the stop: each process's own record
  let mut perProc : Array (Fin NProc × Array Act') := #[]
  for p in List.finRange NProc do
    perProc := perProc.push (p, ← (← idx recs p.val).get)
  let nAct := perProc.foldl (fun n (_, r) => n + r.size) 0
  IO.println s!"wired: {nAct} activations in {t1 - t0} ms"
  let commitsAll := perProc.toList.flatMap (fun (p, r) => r.toList.flatMap (fun e => e.commits.map (fun c => (p, c))))
  -- slot-functional: one value per slot across every process (a hash map of first values)
  let slotFn := (commitsAll.foldl (fun (acc : Std.HashMap Nat (Option Nat) × Bool) (_, c) =>
    match acc.1[c.1]? with
    | some v => (acc.1, acc.2 && v == c.2)
    | none => (acc.1.insert c.1 c.2, acc.2)) (∅, true)).2
  if cfg.bench then
    let commits := commitsAll.foldl (fun acc (p, _) => if p.val = 0 then acc + 1 else acc) 0
    let wall := t1 - t0
    IO.println s!"wired: {commits} commits at the leader in {wall} ms wall → {if wall = 0 then 0 else commits * 1000 / wall} commits/s ({nAct} activations)"
    for (p, mine) in perProc do
      let n := mine.size
      if n = 0 then continue
      let total := mine.foldl (fun acc e => acc + e.nanos) 0
      let q := max 1 (n / 4)
      let firstQ := (mine.extract 0 q).foldl (fun acc e => acc + e.nanos) 0
      let lastQ := (mine.extract (n - q) n).foldl (fun acc e => acc + e.nanos) 0
      IO.println s!"  process {p}: {n} activations, step mean {total / n / 1000} µs, first quarter {firstQ / q / 1000} µs, last quarter {lastQ / q / 1000} µs"
    IO.println s!"wired: slot-functional across processes: {slotFn}"
    return (if slotFn && commits ≥ cfg.total then 0 else 1)
  let committed := commitsAll.map (·.2)
  IO.println s!"wired: commits {committed}; slot-functional across processes: {slotFn}"
  let has42 := committed.any (·.2 = some 1000)
  unless slotFn && has42 do
    IO.eprintln "wired: FAILED"
    return 1
  IO.println "wired: OK"
  return 0

end WiredDemo

def main (args : List String) : IO UInt32 := WiredDemo.main args
