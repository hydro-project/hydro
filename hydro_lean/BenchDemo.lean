import Hydro
import Hydro.Project.Wire
import Hydro.Project.Process
import Hydro.Project.Fused
import Hydro.Project.Shell
import Hydro.Project.Profile
import Std.Async.TCP
import Std.Sync.Mutex
import Std.Data.HashMap

/-!
# `bench` — `paxos_bench` as ten processes over local TCP (D83)

The whole benchmark — `hydro_test/src/cluster/paxos_bench.rs` as `paxos_bench` — run end
to end: two proposers, three acceptors, two clients, one aggregator and two replicas, each a
process stepping `Dag.locStep` on **its own cells only** over real TCP sockets on
`127.0.0.1` (the `wired` demo's runtime, `WiredDemo.lean`, for `paxos_core`). The graph has
no inputs: the clients generate the load (`bench_client`'s virtual clients,
`inc_i32_workload_generator`), the protocol sequences it through Paxos and applies it at
the replicas (`kv_replica`), the replicas' checkpoints truncate the acceptors' logs, and the
aggregator prints what Rust's `pretty_print_bench_results` prints — throughput per
interval and the latency percentiles of the merged `HdrHistogram`.

**The clock and the timers are the execution's, read once per activation.** A process
holds no schedule and no timing history (D84): each activation it reads its monotonic
clock (`use::time`), counts the interval timer's fires (`source_interval`) and the proposers'
heartbeat verdicts, and hands exactly those to the loop body as this tick's timing
(`Dag.Timing`). Content decisions are `Unit` at the machine (the ticks are the
batches); delivery cursors are read online (`Dag.oracleOf`: everything that has arrived).
The lock-step machine's scripts are the proof layer's: a run's per-tick readings form
one, and `paxos_bench.dag` under that script is this run (`Dag.timingAt`).

**Ten OS processes** (D86): the parent spawns `bench --proc P …` for each process, relays
their output and sums their `stats` lines; each process has its own event loop, allocator
and reference counts, an `Outbox` writer thread per connection (one vectored `sendAll` per
drain) and a condition-variable inbox the readers and the pulse timer signal. The wire is
one `Wire.frame` per message (D87/D90). Profiling faces: `HYDRO_BENCH_PROFILE=1` (decode+enqueue / step /
encode per activation), `HYDRO_BENCH_NODES=1` (the step by operator, `Project/Profile.lean`),
`HYDRO_BENCH_TRACE=1` (every activation with traffic, first 1.5 s), `bench idle P N` (an
empty activation of process `P`, stage by stage, no sockets), `scripts/bench_perf_role.sh P`
(`perf` on one role, the machine thread separated from the socket threads). FINDINGS D86–D87
and D92 are the measurements: 22.5k → 82k → 120k requests/s at 1000 virtual clients (133k
at 2000); D92's rounds — one graph walk per activation (`@[csimp] Dag.iterateBytes_eq_impl`), decode
buckets, frames and payloads written in place, `demux` messages routed to the one process
their address names (`Shell.routeSends` over `Dag.wireRoutes`; `HYDRO_BENCH_NODEMUX=1` for
the cluster-wide routing) — and the wall: the leader's per-tick re-fold of all in-flight
state, 41 % of its machine thread. Since D93 a process idles between arrivals and timer
fires as DFIR does: an activation is the tick pass and then the settling passes
(`Dag.iterate`: stutters while a knot slot holds feedback, a further tick while a tick input
holds what the stutters put there), so nothing is left for a next tick; a process whose
sockets are silent and whose timers are not due waits.
-/

open Hydro Hydro.Project Lean Std Std.Async

namespace BenchDemo

/-- The benchmark's five clusters (`paxos_bench.rs`: `Proposer`, `Acceptor`, `Client`,
`Aggregator`, `Replica`). -/
inductive PBLoc | prop | acc | client | agg | rep
deriving DecidableEq, Repr

instance : BEq PBLoc := ⟨fun a b => decide (a = b)⟩

/-- Cluster sizes: 2 proposers, 3 acceptors, 2 clients, 1 aggregator, 2 replicas (`f = 1`,
`num_replicas = f + 1`). -/
@[reducible] def pbMem : PBLoc → Nat
  | .prop => 2
  | .acc => 3
  | .client => 2
  | .agg => 1
  | .rep => 2

abbrev Mem := pbMem

abbrev NProc := 10

/-- Process `p` is proposer `p` (p < 2), acceptor `p − 2` (p < 5), client `p − 5` (p < 7),
the aggregator (p = 7), or replica `p − 8`. -/
def memberOf : Fin NProc → Member PBLoc Mem
  | ⟨0, _⟩ => ⟨.prop, ⟨0, by decide⟩⟩
  | ⟨1, _⟩ => ⟨.prop, ⟨1, by decide⟩⟩
  | ⟨2, _⟩ => ⟨.acc, ⟨0, by decide⟩⟩
  | ⟨3, _⟩ => ⟨.acc, ⟨1, by decide⟩⟩
  | ⟨4, _⟩ => ⟨.acc, ⟨2, by decide⟩⟩
  | ⟨5, _⟩ => ⟨.client, ⟨0, by decide⟩⟩
  | ⟨6, _⟩ => ⟨.client, ⟨1, by decide⟩⟩
  | ⟨7, _⟩ => ⟨.agg, ⟨0, by decide⟩⟩
  | ⟨8, _⟩ => ⟨.rep, ⟨0, by decide⟩⟩
  | ⟨9, _⟩ => ⟨.rep, ⟨1, by decide⟩⟩

def selOf (p : Fin NProc) : Member PBLoc Mem := memberOf p

/-- Which cluster process `q` hosts. -/
def hosts (q : Nat) (ℓ : PBLoc) : Bool :=
  if h : q < NProc then (memberOf ⟨q, h⟩).1 == ℓ else false

/-- Run configuration. -/
structure Cfg where
  /-- Virtual clients per client process (`num_clients_per_node`). -/
  nClients : Nat := 4
  /-- `checkpoint_frequency`: the replicas checkpoint every this many applied slots. -/
  ckFreq : Nat := 100
  /-- The clients' measurement interval (`client_interval_millis`). -/
  clientIntervalMs : Nat := 100
  /-- The aggregator's output interval (`aggregate_interval_millis`). -/
  aggIntervalMs : Nat := 1000
  /-- How long the benchmark runs. -/
  durationMs : Nat := 5000

/-- A process's timing this activation — its clock read once (µs since the start), and
the previous activation's reading (none at the first): every clock node reads `nowUs`,
every interval timer whether it fired in between (`TickReadings.ofClock`, D89). The
timers are the program's — the heartbeat interval, the staggered election interval, the
measurement intervals — not the shell's. -/
structure Timing where
  nowUs : Nat
  lastUs : Option Nat

def readings (m : Member PBLoc Mem) (tm : Timing) : TickReadings m :=
  TickReadings.ofClock m tm.nowUs tm.lastUs

/-- The graph's outputs: the aggregator's interval throughputs and latency histograms. -/
abbrev PBOuts : List (Kind PBLoc Mem) :=
  [Kind.stream (mem := Mem) .agg Nat instDecidableEqNat .totalOrder .exactlyOnce,
   Kind.stream (mem := Mem) .agg Histogram Histogram.instDecidableEq .totalOrder .exactlyOnce]

/-- The graph — no decision or schedule baked in. -/
def D (cfg : Cfg) : Dag (mem := Mem) [] PBOuts :=
  paxos_bench.dag (mem := Mem) .guarded .prop .acc .client .agg .rep cfg.ckFreq 1 2 cfg.nClients ⟨1, 5, 10, 15⟩
    cfg.clientIntervalMs cfg.aggIntervalMs

/-- Its wire codecs. -/
def codecs (cfg : Cfg) : Dag.Codecs (D cfg) :=
  paxos_bench.wireCodecs (mem := Mem) .guarded .prop .acc .client .agg .rep cfg.ckFreq 1 2 cfg.nClients ⟨1, 5, 10, 15⟩
    cfg.clientIntervalMs cfg.aggIntervalMs

/-- The compiled graph with its codecs, **built once per process** and threaded through
the loop as a value (D86): `D cfg` is a function of the configuration, so each call
rebuilt the whole graph; and a value a thread never shares keeps non-atomic reference
counts — the cells travel as a loop argument, never through a reference another thread
created. -/
structure Graph where
  dag : Dag (mem := Mem) [] PBOuts
  cod : Dag.Codecs dag

def Graph.ofCfg (cfg : Cfg) : Graph := ⟨D cfg, codecs cfg⟩

abbrev Loc (g : Graph) (p : Fin NProc) := Dag.LocState (selOf p) g.dag

/-- Every process starts from the graph's initial cells. -/
def init (g : Graph) (p : Fin NProc) : Loc g p :=
  Dag.memoState (selOf p) g.dag (Dag.restrict (selOf p) g.dag (Dag.init g.dag))

/-- The result of one activation. -/
structure Act (g : Graph) (p : Fin NProc) where
  cells : Loc g p
  /-- The aggregator's outputs this tick (at the aggregator process): interval
  throughputs and latency histograms. -/
  throughputs : List Nat
  latencies : List Histogram
  sends : List WireMsg
  /-- The incomplete frame at the end of each connection handed in, to keep. -/
  tails : List ByteArray

/-- The outputs at the process's aggregator member. -/
def outsAt (p : Fin NProc) (o : HList (Kind.locDelta (selOf p)) PBOuts) :
    List Nat × List Histogram :=
  ((hostedList (selOf p) .agg).flatMap (fun i => o.get Var.zero i),
   (hostedList (selOf p) .agg).flatMap (fun i => o.get Var.zero.succ i))

/-- **One activation of process `p`** — the library's loop body at the bytes
(`Dag.iterateBytes`, D90) at this activation's timing: per connection, the kept tail and
what the socket delivered since — the call `WireTrace.act` makes, verbatim: the tick pass,
then the settling passes (D93). Compiled through
`@[csimp] Dag.iterateBytes_eq_impl` (`Project/Fused.lean`): the frames parsed once per
connection and the graph walked once per pass (`Dag.activateFused`, D92). -/
def activate (g : Graph) (p : Fin NProc) (tm : Timing) (chans : List ((src : Member PBLoc Mem) × ByteArray))
    (s : Loc g p) : Act g p :=
  let r := Dag.iterateBytes (selOf p) true g.dag g.cod (Dag.Timing.ofReadings (selOf p) (readings (selOf p) tm) g.dag) chans () s
  let o := outsAt p r.2.1
  ⟨r.1, o.1, o.2, r.2.2.1, r.2.2.2⟩

/-- The activation in stages, timed (`HYDRO_BENCH_PROFILE=1`): decode the inbox and
materialize the cells, step the graph, encode the sends. The same computation as `activate`;
the stage nanos accumulate per process plus the message counts. -/
def activateProfiled (g : Graph) (p : Fin NProc) (tm : Timing) (chans : List ((src : Member PBLoc Mem) × ByteArray))
    (s : Loc g p) (prof : IO.Ref (Array Nat)) (nodes : Option (IO.Ref (Array NodeProfile)))
    (wires : IO.Ref (Std.HashMap Nat (Nat × Nat))) : IO (Act g p) := do
  let t0 ← IO.monoNanosNow
  let parsed ← IO.lazyPure fun _ => chans.map fun c => (⟨c.1, parseFrames c.2⟩ : (src : Member PBLoc Mem) × List WireMsg × ByteArray)
  let ib ← IO.lazyPure fun _ =>
    (Dag.inboxOfWire (selOf p) g.dag g.cod 0 (parsed.map fun c => ⟨c.1, c.2.1⟩)).1
  let s' ← IO.lazyPure fun _ => Dag.memoState (selOf p) g.dag (Dag.enqueueInbox (selOf p) g.dag ib s)
  let t1 ← IO.monoNanosNow
  let ora := Dag.oracleOf (selOf p) g.dag (Dag.Timing.ofReadings (selOf p) (readings (selOf p) tm) g.dag) s'
  let r ← match nodes with
    | none => IO.lazyPure fun _ => Dag.locStep (selOf p) true g.dag ora () s'
    | some nref => do
      let (r, np) ← Dag.locStepProfiled (selOf p) true g.dag ora () s' #[]
      nref.modify (·.push np)
      pure r
  let t2 ← IO.monoNanosNow
  let sends ← IO.lazyPure fun _ => (Dag.wireEncode (selOf p) g.dag g.cod 0 r.2.2).1
  let t3 ← IO.monoNanosNow
  -- the settling passes (D93): the compiled form, as `Dag.iterateBytesImpl` runs them
  let st ← IO.lazyPure fun _ => Dag.settleImpl (selOf p) g.dag g.cod
    (Dag.Timing.quiet (selOf p) g.dag (Dag.Timing.ofReadings (selOf p) (readings (selOf p) tm) g.dag))
    (envQuiet (selOf p) ()) (Dag.settleFuel g.dag) r.1 r.2.1 sends
  let t4 ← IO.monoNanosNow
  let o := outsAt p st.2.1
  let sends := st.2.2
  let nmsg := parsed.foldl (fun n c => n + c.2.1.length) 0
  -- messages per wire node, in and out (`HYDRO_BENCH_PROFILE`)
  wires.modify fun w =>
    let w := parsed.foldl (fun w c => c.2.1.foldl (fun w msg => w.alter msg.1 fun o => some ((o.getD (0, 0)).1 + 1, (o.getD (0, 0)).2)) w) w
    sends.foldl (fun w msg => w.alter msg.1 fun o => some ((o.getD (0, 0)).1, (o.getD (0, 0)).2 + 1)) w
  prof.modify fun a =>
    let a := if a.size < 8 then #[0, 0, 0, 0, 0, 0, 0, 0] else a
    let a := a.modify 0 (· + (t1 - t0))
    let a := a.modify 1 (· + (t2 - t1))
    let a := a.modify 2 (· + (t3 - t2))
    let a := a.modify 3 (· + nmsg)
    let a := a.modify 4 (· + sends.length)
    let a := a.modify 6 (· + (if nmsg > 0 then 1 else 0))
    let a := a.modify 7 (· + (t4 - t3))
    a.modify 5 (· + 1)
  pure ⟨st.1, o.1, o.2, sends, parsed.map (·.2.2)⟩

def idx {α : Type} (a : Array α) (i : Nat) : IO α :=
  match a[i]? with
  | some x => pure x
  | none => throw (IO.userError s!"bench: index {i} out of range")

/-! ## The wire -/

/-- What the readers push: the connection and a chunk of bytes, as the socket delivered it. -/
abbrev Chunk := Fin NProc × ByteArray

/-- A process's inbox: the chunks its readers pulled off the sockets, under a mutex, with a
condition variable the readers (and the pulse timer) signal — an idle process waits here
instead of ticking (D86). -/
structure Inbox where
  mu : BaseMutex
  cv : Condvar
  items : IO.Ref (Array Chunk)

def Inbox.new : IO Inbox := do
  pure ⟨← BaseMutex.new, ← Condvar.new, ← IO.mkRef #[]⟩

def Inbox.push (ib : Inbox) (x : Chunk) : IO Unit := do
  ib.mu.lock
  ib.items.modify (·.push x)
  ib.mu.unlock
  ib.cv.notifyOne

def Inbox.drain (ib : Inbox) : IO (Array Chunk) := do
  ib.mu.lock
  let a ← ib.items.get
  ib.items.set #[]
  ib.mu.unlock
  pure a

/-- Block until a reader pushes or the timer signals (one wake-up; the caller re-checks). -/
def Inbox.waitOnce (ib : Inbox) : IO Unit := do
  ib.mu.lock
  if (← ib.items.get).isEmpty then ib.cv.wait ib.mu
  ib.mu.unlock

/-- A connection's outbox: the machine queues frames and moves on; a writer thread per
connection drains the queue into one vectored `sendAll`. A `send` through the event loop
blocks the caller for ~1 ms (cross-thread hand-off); a hop that sent to five peers inline
spent 2–7 ms per activation in sends (D86). -/
structure Outbox where
  mu : BaseMutex
  cv : Condvar
  items : IO.Ref (Array ByteArray)
  closed : IO.Ref Bool

def Outbox.new : IO Outbox := do
  pure ⟨← BaseMutex.new, ← Condvar.new, ← IO.mkRef #[], ← IO.mkRef false⟩

def Outbox.push (ob : Outbox) (b : ByteArray) : IO Unit := do
  ob.mu.lock
  ob.items.modify (·.push b)
  ob.mu.unlock
  ob.cv.notifyOne

def Outbox.close (ob : Outbox) : IO Unit := do
  ob.mu.lock
  ob.closed.set true
  ob.mu.unlock
  ob.cv.notifyAll

/-- The writer: wait for frames, take them all, write them in one call. -/
partial def Outbox.writer (ob : Outbox) (c : TCP.Socket.Client) : IO Unit := do
  ob.mu.lock
  while (← ob.items.get).isEmpty && !(← ob.closed.get) do
    ob.cv.wait ob.mu
  let batch ← ob.items.get
  ob.items.set #[]
  let closed ← ob.closed.get
  ob.mu.unlock
  if !batch.isEmpty then (c.sendAll batch).block
  if closed && batch.isEmpty then return
  ob.writer c

def basePort : Nat := 9500

def addrOf (p : Nat) : Net.SocketAddress :=
  .v4 ⟨Net.IPv4Addr.ofParts 127 0 0 1, (basePort + p).toUInt16⟩

/-- Read a connection's bytes into the inbox, chunk by chunk, as they come. -/
partial def reader (client : TCP.Socket.Client) (inbox : Inbox) (fm : Fin NProc) : IO Unit := do
  match ← (client.recv? 65536).block with
  | none => return
  | some bytes =>
    inbox.push (fm, bytes)
    reader client inbox fm

/-- The dialer's first byte names it (`Shell.hello`). -/
partial def recvHello (client : TCP.Socket.Client) : IO Nat := do
  match ← (client.recv? 1).block with
  | none => throw (IO.userError "bench: connection closed before hello")
  | some b =>
    match Shell.helloOf b with
    | some p => pure p
    | none => recvHello client

partial def acceptLoop (server : TCP.Socket.Server) (inbox : Inbox)
    (n : Nat) : IO Unit := do
  if n = 0 then return
  let client ← (server.accept).block
  -- no Nagle: a hop's small message must not wait for the delayed ACK (D86)
  client.noDelay
  let fm ← recvHello client
  if h : fm < NProc then
    let _ ← IO.asTask (reader client inbox ⟨fm, h⟩) (prio := .dedicated)
  else IO.eprintln s!"bench: bad hello {fm}"
  acceptLoop server inbox (n - 1)

/-- Dial a peer, retrying while it is not yet listening; say who is calling. -/
partial def dial (p q : Nat) (tries : Nat) : IO TCP.Socket.Client := do
  let c ← TCP.Socket.Client.mk
  try
    (c.connect (addrOf q)).block
    c.noDelay
    (c.send (Shell.hello p)).block
    return c
  catch e =>
    if tries = 0 then throw e
    IO.sleep 20
    dial p q (tries - 1)

/-- The receiver cluster of wire node `k` (the routing table). -/
def recvTable (g : Graph) : List (Nat × PBLoc) := (Dag.wireReceivers g.dag 0).1

/-- `pretty_print_bench_results` (bench_client/mod.rs): the percentiles in ms (the readings
are µs), and the sample count. -/
def fmtMs (us : Nat) : String :=
  let frac := us % 1000
  let fracS := toString frac
  s!"{us / 1000}.{"".pushn '0' (3 - fracS.length)}{fracS}"

/-- **One machine**, on its own thread: drain what arrived, read this tick's timing (the
clock reading now; the interval fire count), tick once, flush this activation's sends to the
processes hosting a receiver of each message. The aggregator prints its outputs. -/
partial def machine (cfg : Cfg) (g : Graph) (p : Fin NProc) (peers : Array (Option Outbox))
    (recv : Array (Option (PBLoc × Option (Nat → Option Nat))))
    (inbox : Inbox) (stop : IO.Ref Bool)
    (t0 : Nat) (lastAct : IO.Ref (Option Nat)) (timers : List (Nat × Nat))
    (nAct totalThroughput totalSamples : IO.Ref Nat)
    (prof : Option (IO.Ref (Array Nat))) (nodes : Option (IO.Ref (Array NodeProfile)))
    (wires : IO.Ref (Std.HashMap Nat (Nat × Nat))) (traceOn byAddress : Bool)
    (tails : Array ByteArray) (nanos : Array Nat) (unsettled : Nat) (s : Loc g p) : IO (Array Nat × Nat) := do
  if (← stop.get) then return (nanos, unsettled)
  let drained ← inbox.drain
  -- this tick's timing: the clock reading; the program's interval timers fire off it
  let nowUs := (← IO.monoNanosNow) / 1000 - t0
  let last ← lastAct.get
  let due := timersDue timers last nowUs
  -- nothing arrived, no timer due, not the first activation, nothing left pending by the
  -- last activation (the settling counter ran out): wait for the sockets (DFIR runs a tick
  -- at start, on a socket or timer wake, and for nothing else — D93; the activation settles
  -- its own knots and tick inputs, `Dag.iterate`)
  if drained.isEmpty && !due && (← nAct.get) > 0 && !(Dag.settleFlag (selOf p) g.dag s).isSome then
    inbox.waitOnce
    machine cfg g p peers recv inbox stop t0 lastAct timers nAct totalThroughput totalSamples prof nodes wires traceOn byAddress tails nanos unsettled s
  else
  lastAct.set (some nowUs)
  let tm : Timing := ⟨nowUs, last⟩
  -- per connection, in order of first arrival: the kept tail, then the new bytes
  let grouped := Shell.groupChunks drained
  let chans := Shell.chansOf selOf tails grouped
  let n0 ← IO.monoNanosNow
  let a ← match prof with
    | none => IO.lazyPure fun _ => activate g p tm chans s
    | some pr => activateProfiled g p tm chans s pr nodes wires
  let tails := Shell.storeTails tails grouped a.tails
  if traceOn && (!drained.isEmpty || !a.sends.isEmpty || p.val = 0) && nowUs < 1500000 then
    let nIn := drained.foldl (fun n (_, b) => n + b.size) 0
    IO.eprintln s!"trace {nowUs} p{p} in {nIn} bytes out {a.sends.length} {(a.sends.map (·.1)).eraseDups}"
  -- the activation's nanos travel as a loop argument: an array behind an `IO.Ref` another
  -- thread created is marked multi-threaded, so every `push` copied it whole and the
  -- `set` re-marked every element — two O(n) walks per activation (D92)
  let nanos := nanos.push ((← IO.monoNanosNow) - n0)
  nAct.modify (· + 1)
  -- something still pending after the activation: the counter (`Dag.settleFuel`) ran out
  -- before the settling was done (the client's ramp-up: a virtual client spawned per added
  -- tick) — the next activation starts at once; counted for the stats
  let unsettled := if (Dag.settleFlag (selOf p) g.dag a.cells).isSome then unsettled + 1 else unsettled
  if !a.sends.isEmpty then
    let s0 ← IO.monoNanosNow
    -- each connection carries the receiver's messages, a `demux` message its addressee's
    -- only (`Shell.routeSends` over the table precomputed from `Dag.wireRoutes`, D92)
    let buckets := Shell.routeSends (n := NProc) recv selOf a.sends byAddress
    for q in [0:NProc] do
      let mine := buckets.getD q []
      if !mine.isEmpty then
        if q = p.val then inbox.push (p, frameMsgs mine)
        else match peers.getD q none with
          | some ob => ob.push (frameMsgs mine)
          | none => pure ()
    if traceOn && p.val = 0 then
      IO.eprintln s!"trace {nowUs} p{p} sends took {((← IO.monoNanosNow) - s0) / 1000} µs"
  -- the aggregator's outputs: `pretty_print_bench_results`
  for th in a.throughputs do
    totalThroughput.modify (· + th)
    IO.println s!"Throughput: {th * 1000 / cfg.aggIntervalMs} requests/s"
  for h in a.latencies do
    totalSamples.modify (· + h.len)
    IO.println s!"Latency p50: {fmtMs (h.valueAtQuantile 1 2)} | p99 {fmtMs (h.valueAtQuantile 99 100)} | p999 {fmtMs (h.valueAtQuantile 999 1000)} ms ({h.len} samples)"
  machine cfg g p peers recv inbox stop t0 lastAct timers nAct totalThroughput totalSamples prof nodes wires traceOn byAddress tails nanos unsettled a.cells

/-- `bench idle P N`: `N` activations of process `P` with an empty inbox, no sockets — the
per-activation base cost, stage by stage (each stage timed in isolation on the same cells). -/
def idleBench (p : Fin NProc) (n : Nat) : IO Unit := do
  let cfg : Cfg := { nClients := 500, ckFreq := 10 }
  let g := Graph.ofCfg cfg
  let tm : Timing := ⟨1000, some 0⟩
  let mut s := init g p
  let time (label : String) (k : Nat) (act : Unit → IO Unit) : IO Unit := do
    let t0 ← IO.monoNanosNow
    for _ in [0:k] do act ()
    let t1 ← IO.monoNanosNow
    IO.println s!"  {label}: {(t1 - t0) / k / 1000}.{((t1 - t0) / k % 1000) / 100} µs"
  -- warm up: a few full activations
  for _ in [0:20] do
    s := (activate g p tm [] s).cells
  let s0 := s
  IO.println s!"idle activation of process {p} ({repr (selOf p).1} {(selOf p).2}), {n} iterations:"
  IO.println s!"  pending after warm-up: {Dag.settleFlag (selOf p) g.dag s0}; knots: {Dag.knotCount g.dag}, tick inputs: {Dag.batchCount g.dag}, settle counter: {Dag.settleFuel g.dag}; nodes touched/skipped: {Dag.touchCount (selOf p) g.dag}"
  time "full activate" n fun _ => do let _ ← IO.lazyPure fun _ => activate g p tm [] s0
  time "inboxOfWire (no batches)" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.inboxOfWire (selOf p) g.dag g.cod 0 []
  let ib := (Dag.inboxOfWire (selOf p) g.dag g.cod 0 []).1
  time "enqueueInbox" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.enqueueInbox (selOf p) g.dag ib s0
  let s1 := Dag.enqueueInbox (selOf p) g.dag ib s0
  time "memoState" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.memoState (selOf p) g.dag s1
  let s2 := Dag.memoState (selOf p) g.dag s1
  time "Timing.ofReadings" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.Timing.ofReadings (selOf p) (readings (selOf p) tm) g.dag
  let tmg := Dag.Timing.ofReadings (selOf p) (readings (selOf p) tm) g.dag
  time "oracleOf" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.oracleOf (selOf p) g.dag tmg s2
  let ora := Dag.oracleOf (selOf p) g.dag tmg s2
  time "locStep" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.locStep (selOf p) true g.dag ora () s2
  let r := Dag.locStep (selOf p) true g.dag ora () s2
  time "wireEncode" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.wireEncode (selOf p) g.dag g.cod 0 r.2.2
  time "inboxOfWire (one empty batch)" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.inboxOfWire (selOf p) g.dag g.cod 0 [⟨selOf p, []⟩]
  time "activateFused (one walk)" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.activateFused (selOf p) true g.dag g.cod 0 (fun _ => []) (Dag.Timing.ofReadings (selOf p) (readings (selOf p) tm) g.dag) () s0
  time "settle (after the tick walk)" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.settleImpl (selOf p) g.dag g.cod (Dag.Timing.quiet (selOf p) g.dag tmg) (envQuiet (selOf p) ()) (Dag.settleFuel g.dag) r.1 r.2.1 []
  time "Dag.init + restrict + memoState" (max 1 (n / 10)) fun _ => do let _ ← IO.lazyPure fun _ => init g p

def parseCfg : List String → Cfg
  | [n, ck, dur] => { nClients := n.toNat?.getD 4, ckFreq := ck.toNat?.getD 100, durationMs := dur.toNat?.getD 5000 }
  | [n] => { nClients := n.toNat?.getD 4 }
  | _ => {}

/-- The stats a process prints at the end (relayed by the parent). -/
def printStats (cfg : Cfg) (p : Fin NProc) (profile nodesProfile : Bool) (mine : Array Nat)
    (pr : IO.Ref (Array Nat)) (np : IO.Ref (Array NodeProfile)) (wires : IO.Ref (Std.HashMap Nat (Nat × Nat)))
    (recv : List (Nat × PBLoc)) : IO Unit := do
  let _ := cfg
  let n := mine.size
  if n = 0 then return
  let q := max 1 (n / 4)
  let firstQ := (mine.extract 0 q).foldl (· + ·) 0
  let lastQ := (mine.extract (n - q) n).foldl (· + ·) 0
  IO.println s!"  process {p} ({repr (selOf p).1} {(selOf p).2}): {n} activations, step first quarter {firstQ / q / 1000} µs, last quarter {lastQ / q / 1000} µs"
  if profile then
    let a ← pr.get
    if a.size = 8 ∧ a[5]! > 0 then
      let k := a[5]!
      IO.println s!"    per activation: decode+enqueue {a[0]! / k / 1000} µs, step {a[1]! / k / 1000} µs, encode {a[2]! / k / 1000} µs, settle {a[7]! / k / 1000} µs; {a[3]! * 100 / k / 100}.{a[3]! * 100 / k % 100} msgs in, {a[4]! * 100 / k / 100}.{a[4]! * 100 / k % 100} msgs out; {a[6]! * 100 / k} % of activations had input"
      let w := (← wires.get).toArray.qsort (fun (a b : Nat × Nat × Nat) => a.2.1 + a.2.2 > b.2.1 + b.2.2)
      let wl := w.toList.map fun ((node, i, o) : Nat × Nat × Nat) =>
        let dst : String := match recv.find? (fun (e : Nat × PBLoc) => e.1 = node) with
          | some e => toString (repr e.2)
          | none => "?"
        s!"#{node}→{dst}: {i * 10 / k / 10}.{i * 10 / k % 10} in {o * 10 / k / 10}.{o * 10 / k % 10} out"
      IO.println s!"    per activation by wire node: {wl.take 8}"
    if nodesProfile then
      let nps ← np.get
      let k := max 1 nps.size
      let summary := NodeProfile.summarize nps
      let total := summary.foldl (fun t e => t + e.2.1) 0
      IO.println s!"    {(nps[0]?.map (·.size)).getD 0} nodes; per activation by node (top 14 of {summary.size} tags, {total / k / 1000} µs total):"
      for (tag, tot, cnt) in summary.extract 0 14 do
        IO.println s!"      {tot / k / 1000} µs  {tag}  (×{cnt / k})"

/-- **One OS process** (`bench --proc P …`): bind its port, accept the peers' connections,
dial the peers, run the machine for the configured duration, print its stats. Each process
has its own event loop, allocator and reference counts — as deployed Hydro processes do;
ten machines in one OS process shared one event loop (D86). -/
def procMain (p : Fin NProc) (cfg : Cfg) : IO UInt32 := do
  let traceOn := (← IO.getEnv "HYDRO_BENCH_TRACE").isSome
  -- `HYDRO_BENCH_NODEMUX=1`: send every `demux` message to every host of the receiver
  -- cluster (the pre-D92 routing), for A/B measurement
  let byAddress := (← IO.getEnv "HYDRO_BENCH_NODEMUX").isNone
  let nodesProfile := (← IO.getEnv "HYDRO_BENCH_NODES").isSome
  let profile := (← IO.getEnv "HYDRO_BENCH_PROFILE").isSome || nodesProfile
  let srv ← TCP.Socket.Server.mk
  srv.bind (addrOf p.val)
  srv.listen 16
  let inbox ← Inbox.new
  let accepts ← IO.asTask (acceptLoop srv inbox (NProc - 1)) (prio := .dedicated)
  let mut peers : Array (Option Outbox) := #[]
  let mut outboxes : Array Outbox := #[]
  for q in [0:NProc] do
    if q ≠ p.val then
      let c ← dial p.val q 250
      let ob ← Outbox.new
      let _ ← IO.asTask (try ob.writer c catch _ => pure ()) (prio := .dedicated)
      peers := peers.push (some ob)
      outboxes := outboxes.push ob
    else peers := peers.push none
  match ← IO.wait accepts with
  | .ok () => pure ()
  | .error e => throw e
  if traceOn && p.val = 0 then
    let g := Graph.ofCfg cfg
    IO.eprintln s!"trace wire nodes → receiver: {(recvTable g).map fun (k, r) => (k, repr r)}"
  let stop ← IO.mkRef false
  let t0 := (← IO.monoNanosNow) / 1000
  let nAct ← IO.mkRef 0
  let totalThroughput ← IO.mkRef 0
  let totalSamples ← IO.mkRef 0
  let pr ← IO.mkRef (#[] : Array Nat)
  let np ← IO.mkRef (#[] : Array NodeProfile)
  let wires ← IO.mkRef ({} : Std.HashMap Nat (Nat × Nat))
  let lastAct ← IO.mkRef (none : Option Nat)
  let g := Graph.ofCfg cfg
  let timers := Dag.timers (selOf p) g.dag
  -- the timer thread: wake the machine at the next fire instant of any of its timers
  let _ ← IO.asTask (do
    while !(← stop.get) do
      let nowUs := (← IO.monoNanosNow) / 1000 - t0
      let waitUs := match nextDue timers (← lastAct.get) with
        | some n => min (n - nowUs) 200000
        | none => 200000
      IO.sleep (max 1 (waitUs / 1000)).toUInt32
      inbox.cv.notifyOne) (prio := .dedicated)
  let task ← IO.asTask (do
      machine cfg g p peers (Shell.routeTable (Dag.wireRoutes g.dag 0).1) inbox stop t0 lastAct timers nAct totalThroughput totalSamples
        (if profile then some pr else none) (if nodesProfile then some np else none) wires traceOn byAddress
        (Array.replicate NProc ByteArray.empty) #[] 0 (init g p))
    (prio := .dedicated)
  for _ in [0:cfg.durationMs] do
    if (← stop.get) then break
    IO.sleep 1
  stop.set true
  inbox.cv.notifyAll
  for ob in outboxes do ob.close
  let (nanos, unsettled) ← match ← IO.wait task with
    | .ok a => pure a
    | .error e => throw e
  printStats cfg p profile nodesProfile nanos pr np wires (recvTable g)
  if unsettled > 0 then
    IO.println s!"  process {p}: {unsettled} activations ended with work still pending (the settling counter ran out; the next activation started at once)"
  let th ← totalThroughput.get
  let smp ← totalSamples.get
  IO.println s!"stats {p} {← nAct.get} {th} {smp}"
  return 0

/-- The parent: spawn the ten processes, relay their output, sum their stats. -/
def parentMain (cfg : Cfg) (cfgArgs : List String) : IO UInt32 := do
  IO.eprintln s!"bench: paxos_bench as {NProc} OS processes (2 proposers, 3 acceptors, 2 clients × {cfg.nClients} virtual clients, 1 aggregator, 2 replicas) over TCP 127.0.0.1:{basePort}+p, checkpoint every {cfg.ckFreq} slots, {cfg.durationMs} ms"
  let exe ← IO.appPath
  let mut children : Array (IO.Process.Child { stdout := .piped, stderr := .inherit, stdin := .inherit }) := #[]
  for p in [0:NProc] do
    let child ← IO.Process.spawn {
      cmd := exe.toString
      args := #["--proc", toString p] ++ cfgArgs.toArray
      stdout := .piped
      stderr := .inherit }
    children := children.push child
  let mut acts := 0
  let mut th := 0
  let mut smp := 0
  let mut outputs : Array String := #[]
  for child in children do
    let out ← child.stdout.readToEnd
    let _ ← child.wait
    outputs := outputs.push out
  for out in outputs do
    for line in out.splitOn "\n" do
      if line.startsWith "stats " then
        match ((line.drop 6).toString.splitOn " ").map String.toNat? with
        | [some _, some a, some t, some m] => acts := acts + a; th := th + t; smp := smp + m
        | _ => pure ()
      else if !line.isEmpty then IO.println line
  IO.println s!"bench: {acts} activations; {th} requests completed in the aggregated intervals, {smp} latency samples"
  if th = 0 then
    IO.eprintln "bench: FAILED (no request completed)"
    return 1
  IO.println "bench: OK"
  return 0

def main (args : List String) : IO UInt32 := do
  if let "idle" :: pS :: rest := args then
    if h : pS.toNat?.getD 0 < NProc then
      idleBench ⟨pS.toNat?.getD 0, h⟩ ((rest.head?.bind String.toNat?).getD 2000)
      return 0
  if let "--proc" :: pS :: rest := args then
    if h : pS.toNat?.getD 0 < NProc then
      return ← procMain ⟨pS.toNat?.getD 0, h⟩ (parseCfg rest)
  parentMain (parseCfg args) args

end BenchDemo

/-- The harness's reader threads block in `recv?` on sockets nobody closes; the
runtime would wait for them at exit, so the harness flushes and exits directly. -/
def main (args : List String) : IO UInt32 := do
  let code ← BenchDemo.main args
  (← IO.getStdout).flush
  (← IO.getStderr).flush
  IO.Process.exit code.toUInt8
