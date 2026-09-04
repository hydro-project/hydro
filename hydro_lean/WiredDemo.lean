import Hydro
import Hydro.Project.Wire
import Hydro.Project.Shell
import Hydro.Project.Fused
import Hydro.Paxos.WiredSafety
import Hydro.WireTraceCheck
import Std.Async.TCP
import Std.Sync.Mutex
import Std.Data.HashMap

/-!
# `wired` — `paxos_core` as five processes over local TCP (D74)

Two proposers and three acceptors, each a process running
`Dag.locStep` on **its own cells only** (`Dag.LocState`), talking over
real TCP sockets on `127.0.0.1`: every process listens on a port, dials
the other four, and each activation's `Sends` go out as one frame per message
(`Dag.wireEncode`, the codecs `paxos_core.wireCodecs` the elaborator
built from the message types' `Wire` instances; `frameMsgs`). A receiver hands the
library the bytes its sockets delivered since its last activation — cut wherever
TCP cut them — and the library's `Dag.iterateBytes` finds the complete frames, keeps
the tails, enqueues the messages and delivers everything (D90); **an
activation is one tick** of all its members. Nothing settles: a knot's pending feedback is state the next tick
consumes, and the next tick comes with the next arrival (every cycle in
`paxos_core` crosses the network) or the idle timer. A process that is not
running is inert, so the run is an `Interleaving` as it stands (D76).

What is checked: the proposers commit a client payload and every commit
any process emits at one slot agrees — the statement `paxos_safe_wired`
makes about **exactly this** execution model. Each process is its own
thread; nothing is shared between them but the sockets (and a stop flag
the harness sets). Each process keeps its own record of what it
committed; the harness reads the records only after the stop. (`wired` is
the witness, not a benchmark: the open-loop `--bench` mode and its harness
checkpoint channel were removed in D92 — `lake exe bench` is the measured
system, the whole `paxos_bench` with its own clients and replicas.)
-/

open Hydro Hydro.Project Lean Std Std.Async

namespace WiredDemo

abbrev Mem := paxMem 2 3

/-- The graph — no decision or schedule baked in. -/
def D := paxos_core.dag (mem := Mem) (P := Nat) (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce) .guarded PaxLoc.prop PaxLoc.acc ⟨1, 5, 10, 15⟩

/-- **Timing, read per activation** (D84: a process holds no timing history — each
activation is handed this tick's clock reading and, from it and the previous reading,
whether each of the program's interval timers fired; D89). The election timers are the
program's (`PaxosConfig`: heartbeats every 5 s, the check every 10 s, proposer `i`'s
check staggered by `15·i` s): proposer 0's fires at once and it leads, proposer 1's
never within the run. Content decisions are `Unit` at the machine (the ticks are the
batches); delivery cursors are read online (`Dag.oracleOf`: everything that has
arrived). -/
def readings (m : Member PaxLoc Mem) (nowUs : Nat) (lastUs : Option Nat) : TickReadings m :=
  TickReadings.ofClock m nowUs lastUs

/-- Its wire codecs. -/
def codecs : Dag.Codecs D := paxos_core.wireCodecs (mem := Mem) (P := Nat) (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce) .guarded PaxLoc.prop PaxLoc.acc ⟨1, 5, 10, 15⟩

/-! ## Five processes -/

abbrev NProc := 5

/-- Process `p` is proposer `p` (p < 2) or acceptor `p − 2`. -/
def memberOf : Fin NProc → Member PaxLoc Mem
  | ⟨0, _⟩ => ⟨.prop, ⟨0, by decide⟩⟩
  | ⟨1, _⟩ => ⟨.prop, ⟨1, by decide⟩⟩
  | ⟨2, _⟩ => ⟨.acc, ⟨0, by decide⟩⟩
  | ⟨3, _⟩ => ⟨.acc, ⟨1, by decide⟩⟩
  | ⟨4, _⟩ => ⟨.acc, ⟨2, by decide⟩⟩

def selOf (p : Fin NProc) : Member PaxLoc Mem := memberOf p

abbrev Loc (p : Fin NProc) := Dag.LocState (selOf p) D

/-- The graph's input context: client payloads (at proposers), checkpoints (at acceptors). -/
abbrev Γ0 := [Kind.stream (mem := Mem) .prop Nat inferInstance .totalOrder .exactlyOnce,
  Kind.sing (mem := Mem) .acc Nat (Option Nat) inferInstance .totalOrder .exactlyOnce (.monotonic ValueOrder.optNat)]

/-- Process `p`'s inputs this activation: its payloads (at a proposer); no checkpoint ever
arrives (the acceptors' input singleton stays `none` — there are no replicas here). -/
def inputsFor (p : Fin NProc) (payloads : List Nat) : HList (Kind.locDelta (selOf p)) Γ0 :=
  (fun _ => payloads, (fun _ => none, ()))

/-- Every process starts from the graph's initial cells (its `defer_tick`s
fire at its own first tick). -/
def init (p : Fin NProc) : Loc p := Dag.memoState (selOf p) D (Dag.restrict (selOf p) D (Dag.init D))

/-- The result of one activation. -/
structure Act (p : Fin NProc) where
  cells : Loc p
  ballots : List (Ballot 2)
  commits : List (Nat × Option Nat)
  sends : List WireMsg
  /-- The incomplete frame at the end of each connection handed in, to keep. -/
  tails : List ByteArray

/-- The outputs at the process's proposers. -/
def outsAt (p : Fin NProc) (o : HList (Kind.locDelta (selOf p))
    [Kind.stream (mem := Mem) .prop (Ballot 2) instDecidableEqBallot .totalOrder .exactlyOnce,
     Kind.stream (mem := Mem) .prop (Nat × Option Nat) inferInstance .noOrder .exactlyOnce]) :
    List (Ballot 2) × List (Nat × Option Nat) :=
  ((hostedList (selOf p) .prop).flatMap (fun i => o.get Var.zero i),
   (hostedList (selOf p) .prop).flatMap (fun i => o.get Var.zero.succ i))

/-- **One activation of process `p`** — the library's loop body at the bytes
(`Dag.iterateBytes`, D90): per connection, the kept tail and what the socket delivered
since; the complete frames are the messages (appended, all delivered), one tick with the
payloads and then the settling passes (stutters while a knot slot holds feedback, a further
tick while a tick input holds items — D93),
nothing kept in flight; the sends encoded, the new tails returned. The theorem
`paxos_safe_wired` is about exactly this function along any history of the connections. -/
def activate (p : Fin NProc) (nowUs : Nat) (lastUs : Option Nat)
    (chans : List ((src : Member PaxLoc Mem) × ByteArray)) (payloads : List Nat)
    (s : Loc p) : Act p :=
  let r := Dag.iterateBytes (selOf p) true D codecs (Dag.Timing.ofReadings (selOf p) (readings (selOf p) nowUs lastUs) D)
    chans (inputsFor p payloads) s
  let o := outsAt p r.2.1
  ⟨r.1, o.1, o.2, r.2.2.1, r.2.2.2⟩

/-- Array access in `IO` (no `Inhabited` needed). -/
def idx {α : Type} (a : Array α) (i : Nat) : IO α :=
  match a[i]? with
  | some x => pure x
  | none => throw (IO.userError s!"wired: index {i} out of range")

/-! ## The wire -/

def basePort : Nat := 9400

def addrOf (p : Nat) : Net.SocketAddress :=
  .v4 ⟨Net.IPv4Addr.ofParts 127 0 0 1, (basePort + p).toUInt16⟩

/-- A process's own record of one activation: its commits. -/
abbrev Act' := List (Nat × Option Nat)

/-- What the readers push: the connection and a chunk of bytes, as the socket delivered it. -/
abbrev Chunk := Fin NProc × ByteArray

/-- Read a connection's bytes into the inbox, chunk by chunk, as they come. -/
partial def reader (client : TCP.Socket.Client) (inbox : Mutex (Array Chunk)) (fm : Fin NProc) : IO Unit := do
  match ← (client.recv? 65536).block with
  | none => return
  | some bytes =>
    inbox.atomically (modify fun a => a.push (fm, bytes))
    reader client inbox fm

/-- The dialer's first byte names it (`Shell.hello`). -/
partial def recvHello (client : TCP.Socket.Client) : IO Nat := do
  match ← (client.recv? 1).block with
  | none => throw (IO.userError "wired: connection closed before hello")
  | some b =>
    match Shell.helloOf b with
    | some p => pure p
    | none => recvHello client

/-- Accept the four peers and read from each. -/
partial def acceptLoop (server : TCP.Socket.Server) (inbox : Mutex (Array Chunk)) (n : Nat) : IO Unit := do
  if n = 0 then return
  let client ← (server.accept).block
  -- no Nagle: a hop's small message must not wait for the delayed ACK (D86)
  client.noDelay
  let fm ← recvHello client
  if h : fm < NProc then
    let _ ← IO.asTask (reader client inbox ⟨fm, h⟩) (prio := .dedicated)
  else IO.eprintln s!"wired: bad hello {fm}"
  acceptLoop server inbox (n - 1)

/-- Dial a peer, retrying while it is not yet listening; say who is calling. -/
partial def dial (addr : Net.SocketAddress) (hello : Option Nat) (tries : Nat) : IO TCP.Socket.Client := do
  let c ← TCP.Socket.Client.mk
  try
    (c.connect addr).block
    c.noDelay
    if let some p := hello then (c.send (Shell.hello p)).block
    return c
  catch e =>
    if tries = 0 then throw e
    IO.sleep 20
    dial addr hello (tries - 1)

/-- Which cluster process `q` hosts. -/
def hosts (q : Nat) : PaxLoc → Bool
  | .prop => decide (q < 2)
  | .acc => decide (2 ≤ q)

instance : BEq PaxLoc := ⟨fun a b => decide (a = b)⟩

/-- The routing table: wire node → receiver cluster (`Dag.wireFor`'s). -/
def recvTable : List (Nat × PaxLoc) := (Dag.wireReceivers D 0).1

/-- The harness gives up after this long without the commit. -/
def deadlineMs : Nat := 10000

/-- The one client payload the leader proposes once elected. -/
def thePayload : Nat := 1000

/-- **One machine**, on its own thread. Loops independently of every other
process: take whatever bytes have arrived on its sockets, tick once, flush this
activation's sends; records its commits in `rec` (read by the harness only after
the stop). An idle machine — nothing arrived, no interval timer of its program due
(`Dag.timers`, `timersDue`) — waits, as DFIR does (D93); the proposers' election timers
are the program's `source_interval`s and wake it. The leader (the process that saw its
own election on the `ballots` output) injects the one client payload, once. -/
partial def machine (p : Fin NProc) (peers : Array (Nat × TCP.Socket.Client))
    (inbox : Mutex (Array Chunk)) (stop : IO.Ref Bool) (rec : IO.Ref (Array Act'))
    (stRef : IO.Ref (Loc p)) (tails : IO.Ref (Array ByteArray)) (leader injected committed : IO.Ref Nat)
    (clock0 : Nat) (lastAct : IO.Ref (Option Nat)) (timers : List (Nat × Nat)) : IO Unit := do
  if (← stop.get) then return
  let drained ← inbox.atomically (do
    let a : Array Chunk ← get
    set (#[] : Array Chunk)
    pure a)
  -- per connection, in order of first arrival: the kept tail, then the new bytes
  let grouped := Shell.groupChunks drained
  let chans := Shell.chansOf selOf (← tails.get) grouped
  let s ← stRef.get
  let payloads ← do
    if (← leader.get) = 0 || (← injected.get) ≥ 1 then pure []
    else
      injected.set 1
      pure [thePayload]
  let t0 ← IO.monoNanosNow
  -- this tick's clock reading (µs since the start) and the previous one
  let nowUs := t0 / 1000 - clock0
  let last ← lastAct.get
  -- nothing arrived, no payload to inject, none of the program's interval timers due, not
  -- the first activation, nothing left pending by the last activation: idle (DFIR runs a
  -- tick at start, on a socket or timer wake — D93)
  if drained.isEmpty && payloads.isEmpty && last.isSome && !timersDue timers last nowUs
      && !(Dag.settleFlag (selOf p) D s).isSome then
    IO.sleep 1
    machine p peers inbox stop rec stRef tails leader injected committed clock0 lastAct timers
  else
  lastAct.set (some nowUs)
  -- the loop body: cut the bytes into frames, enqueue, deliver all, the tick and its settling passes
  let a ← IO.lazyPure fun _ => activate p nowUs last chans payloads s
  stRef.set a.cells
  tails.modify fun t => Shell.storeTails t grouped a.tails
  let t1 ← IO.monoNanosNow
  let dt := t1 - t0
  if !a.ballots.isEmpty then leader.set 1
  committed.modify (· + a.commits.length)
  -- flush — each connection carries the receiver's messages (`Dag.wireFor`: the nodes
  -- whose receiver cluster it is in); a message to a receiver this process hosts travels
  -- the same way, through its own inbox (the loop body keeps nothing in flight — D78)
  if !a.sends.isEmpty then
    for (q, c) in peers do
      if hq : q < NProc then
        let mine := wireForTable recvTable (selOf ⟨q, hq⟩) a.sends
        if !mine.isEmpty then (c.send (frameMsgs mine)).block
    let mine := wireForTable recvTable (selOf p) a.sends
    if !mine.isEmpty then inbox.atomically (modify (·.push (p, frameMsgs mine)))
  rec.modify (·.push a.commits)
  if !a.ballots.isEmpty || !a.commits.isEmpty || !payloads.isEmpty then
    IO.eprintln s!"wired: process {p}: inbox {drained.size} payloads {payloads} → ballots {repr a.ballots}, commits {repr a.commits} ({dt / 1000} µs)"
  -- the leader has seen its payload committed: tell the harness
  if (← leader.get) = 1 && (← committed.get) ≥ 1 then
    stop.set true
  machine p peers inbox stop rec stRef tails leader injected committed clock0 lastAct timers

/-- `wired --traces`: the scripted byte-level traces of `WireTraceCheck` (round robin reading
everything; five bytes per connection per activation; a silent network; an acceptor that never
runs; proposers ten times as busy as acceptors), run natively through `WireTrace.run` — the
same `Dag.iterateBytes` the sockets feed. -/
def tracesMain : IO UInt32 := do
  let mut ok := true
  for (name, chk) in WireTraceCheck.checks do
    let t0 ← IO.monoMsNow
    let b ← IO.lazyPure fun _ => chk ()
    IO.println s!"wired --traces: {name}: {b} ({(← IO.monoMsNow) - t0} ms)"
    ok := ok && b
  let lz := WireTraceCheck.lazyRun
  IO.println s!"wired --traces: lazy schedule: {lz.1.length} activations, payload at step {lz.2}: {lz.1.map fun m => s!"{match m.1 with | .prop => "p" | .acc => "a"}{m.2.val}"}"
  IO.println s!"wired --traces: {if ok then "OK" else "FAILED"}"
  return (if ok then 0 else 1)

def main (args : List String) : IO UInt32 := do
  -- `lake exe wired -- …` hands the `--` through
  let args := if args.head? = some "--" then args.tail else args
  if args = ["--traces"] then return ← tracesMain
  unless args.isEmpty do
    IO.eprintln "usage: wired [--traces]"
    return 2
  IO.eprintln s!"wired: paxos_core as {NProc} processes (2 proposers, 3 acceptors) over TCP 127.0.0.1:{basePort}+p"
  -- servers first
  let mut servers : Array TCP.Socket.Server := #[]
  for p in [0:NProc] do
    let srv ← TCP.Socket.Server.mk
    srv.bind (addrOf p)
    srv.listen 8
    servers := servers.push srv
  let inboxes ← (List.finRange NProc).toArray.mapM fun _ => Mutex.new (#[] : Array Chunk)
  let accepts ← (List.finRange NProc).toArray.mapM fun p => do
    let srv ← idx servers p.val
    let inb ← idx inboxes p.val
    IO.asTask (acceptLoop srv inb (NProc - 1)) (prio := .dedicated)
  -- dial the peers (saying who is calling)
  let mut peersOf : Array (Array (Nat × TCP.Socket.Client)) := #[]
  for p in [0:NProc] do
    let mut peers : Array (Nat × TCP.Socket.Client) := #[]
    for q in [0:NProc] do
      if q ≠ p then peers := peers.push (q, ← dial (addrOf q) (some p) 100)
    peersOf := peersOf.push peers
  for t in accepts do
    match ← IO.wait t with
    | .ok () => pure ()
    | .error e => throw e
  IO.eprintln "wired: all sockets connected"
  let stop ← IO.mkRef false
  let recs ← (List.finRange NProc).toArray.mapM fun _ => IO.mkRef (#[] : Array Act')
  let t0 ← IO.monoMsNow
  let clock0 := (← IO.monoNanosNow) / 1000
  let tasks ← (List.finRange NProc).toArray.mapM fun p => do
    let stRef ← IO.mkRef (init p)
    let peers ← idx peersOf p.val
    let inb ← idx inboxes p.val
    let rec ← idx recs p.val
    let tails ← IO.mkRef (Array.replicate NProc ByteArray.empty)
    IO.asTask (machine p peers inb stop rec stRef tails (← IO.mkRef 0) (← IO.mkRef 0) (← IO.mkRef 0)
      clock0 (← IO.mkRef none) (Dag.timers (selOf p) D)) (prio := .dedicated)
  -- the harness waits for the leader's signal (or the deadline), then stops every machine
  for _ in [0:deadlineMs] do
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
  let commitsAll := perProc.toList.flatMap (fun (p, r) => r.toList.flatMap (fun e => e.map (fun c => (p, c))))
  -- slot-functional: one value per slot across every process (a hash map of first values)
  let slotFn := (commitsAll.foldl (fun (acc : Std.HashMap Nat (Option Nat) × Bool) (_, c) =>
    match acc.1[c.1]? with
    | some v => (acc.1, acc.2 && v == c.2)
    | none => (acc.1.insert c.1 c.2, acc.2)) (∅, true)).2
  let committed := commitsAll.map (·.2)
  IO.println s!"wired: commits {committed}; slot-functional across processes: {slotFn}"
  let hasPayload := committed.any (·.2 = some thePayload)
  unless slotFn && hasPayload do
    IO.eprintln "wired: FAILED"
    return 1
  IO.println "wired: OK"
  return 0

end WiredDemo

/-- The harness's reader threads block in `recv?` on sockets nobody closes; the
runtime would wait for them at exit, so the harness flushes and exits directly. -/
def main (args : List String) : IO UInt32 := do
  let code ← WiredDemo.main args
  (← IO.getStdout).flush
  (← IO.getStderr).flush
  IO.Process.exit code.toUInt8
