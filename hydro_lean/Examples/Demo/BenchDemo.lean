import Hydro
import Examples
import Hydro.Project.Wire
import Hydro.Project.Process
import Hydro.Project.Fused
import Hydro.Project.Shell
import Hydro.HydroStage
import Hydro.Project.Sockets

/-!
# `lake exe bench` — `paxos_bench` as ten processes over local TCP

The whole benchmark — `hydro_test/src/cluster/paxos_bench.rs` as `paxos_bench` — run end
to end: two proposers, three acceptors, two clients, one aggregator and two replicas, each
a process running the compiled activation `benchRun` on its own cells only over TCP
sockets on `127.0.0.1` (process `p` listens on `9500 + p`). The graph has no inputs: the
clients generate the load (`bench_client`'s virtual clients, `inc_i32_workload_generator`),
the protocol sequences it through Paxos and applies it at the replicas (`kv_replica`), the
replicas' checkpoints truncate the acceptors' logs, and the aggregator prints what Rust's
`pretty_print_bench_results` prints — throughput per interval and the latency percentiles
of the merged `HdrHistogram`.

Invocation: `lake exe bench [nClients ckFreq durationMs]` (defaults 4, 100, 5000). The
parent prints the processes' output, then `bench: N activations; T requests completed in
the aggregated intervals, S latency samples` and `bench: OK` (exit 0), or `bench: FAILED
(no request completed)` (exit 1).

**The execution supplies the clock and the timers, read once per activation.** A process
holds no schedule and no timing history: each activation it reads its monotonic clock
(`use::time`), counts the interval timer's fires (`source_interval`) and the proposers'
heartbeat verdicts, and hands those to the body as this tick's timing (`Dag.Timing`).
Content decisions are `Unit` at the machine (each tick consumes one batch); delivery cursors
are read online (`Dag.oracleOf`: everything that has arrived). Only the proof layer builds the lock-step
machine's scripts: a run's per-tick readings form one, and `paxos_bench.dag`
under that script is this run (`Dag.timingAt`).

**Ten OS processes.** The parent spawns `bench --proc P …` for each process, relays their
output and sums their `stats` lines; each process has its own event loop, allocator and
reference counts, and the sockets of `Hydro/Project/Sockets.lean` (`Async` reader chains, one writer
thread, a condition-variable inbox; no thread blocks on a socket). The wire is one
`Wire.frame` per message, `demux` messages routed to the one process their address identifies
(`Shell.routeSendsIdx` over `Dag.wireRoutes`). A process idles between arrivals and timer
fires as DFIR does: an activation is one tick, its knot wires their bodies' outputs of
that tick, so nothing is left for a next tick; a process whose sockets are silent and
whose timers are not due waits.

**The activation is compiled.** `benchRun` is `paxos_bench`'s graph evaluated by the stager
(`hydro_run`, `Hydro/HydroStage.lean`) into one straight-line function, so no graph is
walked at run time; the kernel-checked `benchRun.eq` equates it to `Dag.iterateBytes …
(D cfg) (codecs cfg) …`, the loop body `paxos_safe_wired`'s stack is stated over.
`activate` calls it; the once-built `Graph` serves the start-up tables and the diagnostics.

**This file runs the benchmark only.** The machine takes its activation through one hook
(`Hooks`: `activate` itself here); the instrumented activation, the per-operator profiles,
the activation trace, the idle activation and the wire microbench are a separate entry
point, `lake exe benchprof` (`Examples/Demo/BenchProfile.lean`; `Examples/Demo/Bench.lean` is `bench`'s — both import
this module).
-/

open Hydro Hydro.Project Lean Std Std.Async Sockets

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

/-- A process's timing this activation: its clock read once (µs since the start), and
the previous activation's reading (none at the first). Every clock node reads `nowUs`,
every interval timer whether it fired in between (`TickReadings.ofClock`). The timers are
the program's — the heartbeat interval, the staggered election interval, the measurement
intervals — not the shell's. -/
structure Timing where
  nowUs : Nat
  lastUs : Option Nat

def readings (m : Member PBLoc Mem) (tm : Timing) : TickReadings m :=
  TickReadings.ofClock m tm.nowUs tm.lastUs

/-- The graph's outputs: the aggregator's interval throughputs and latency histograms. -/
abbrev PBOuts : List (Kind PBLoc Mem) :=
  [Kind.stream (mem := Mem) .agg Nat instDecidableEqNat .totalOrder .exactlyOnce,
   Kind.stream (mem := Mem) .agg Histogram Histogram.instDecidableEq .totalOrder .exactlyOnce]

/-- The graph: no decision or schedule baked in. -/
def D (cfg : Cfg) : Dag (mem := Mem) [] PBOuts :=
  paxos_bench.dag (mem := Mem) .guarded .prop .acc .client .agg .rep cfg.ckFreq 1 2 cfg.nClients ⟨1, 5, 10, 15⟩
    cfg.clientIntervalMs cfg.aggIntervalMs

/-- Its wire codecs. -/
def codecs (cfg : Cfg) : Dag.Codecs (D cfg) :=
  paxos_bench.wireCodecs (mem := Mem) .guarded .prop .acc .client .agg .rep cfg.ckFreq 1 2 cfg.nClients ⟨1, 5, 10, 15⟩
    cfg.clientIntervalMs cfg.aggIntervalMs

/-- The graph with its codecs, built once per process for the start-up tables (the routes,
the receivers, the timers) and the diagnostics (`Examples/Demo/BenchProfile.lean`); the activation itself
never touches it (`activate`, below). `hdag` pins it to the configured graph, so the
diagnostics can read the activation's cells. A value a thread never shares keeps
non-atomic reference counts, so the cells travel as a loop argument, never through a
reference another thread created. -/
structure Graph (cfg : Cfg) where
  dag : Dag (mem := Mem) [] PBOuts
  cod : Dag.Codecs dag
  hdag : dag = D cfg

def Graph.ofCfg (cfg : Cfg) : Graph cfg := ⟨D cfg, codecs cfg, rfl⟩

/-- A process's cells: the configured graph's, at its member. -/
abbrev Loc (cfg : Cfg) (p : Fin NProc) := Dag.LocState (selOf p) (D cfg)

/-- Every process starts from the graph's initial cells. -/
def init (cfg : Cfg) (p : Fin NProc) : Loc cfg p :=
  let d := D cfg
  Dag.memoState (selOf p) d (Dag.restrict (selOf p) d (Dag.init d))

/-- The result of one activation. -/
structure Act (cfg : Cfg) (p : Fin NProc) where
  cells : Loc cfg p
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
  ((hostedList (selOf p) .agg).flatMap (fun i => (o.get Var.zero i).toList),
   (hostedList (selOf p) .agg).flatMap (fun i => (o.get Var.zero.succ i).toList))

/-- The activation, compiled: `paxos_bench`'s graph evaluated by the stager (`hydro_run`,
`Hydro/HydroStage.lean`) into one straight-line function — every node of the configured
graph one `let` of its operator's arm, the knot rounds, the knownness masks, the wire
numbers and the environment all resolved at elaboration time, so no graph is walked at run
time. `benchRun.eq` states, checked by the kernel, that the compiled body equals the library's
loop body,
`Dag.iterateBytes … (D cfg) (codecs cfg) (Dag.Timing.ofReadings … r (D cfg))` — the call
`WireTrace.act` makes and `paxos_safe_wired`'s stack is stated over. -/
hydro_run benchRun (cfg : Cfg) (m : Member PBLoc Mem) where
  member := m, graph := D cfg, codecs := codecs cfg

/-- One activation of process `p`: the compiled activation at this tick's readings (the
clock, the interval timers' fire counts); per connection, the kept tail and what the socket
delivered since. -/
def activate (cfg : Cfg) (p : Fin NProc) (tm : Timing) (chans : List ((src : Member PBLoc Mem) × ByteArray))
    (s : Loc cfg p) : Act cfg p :=
  let r := benchRun cfg (selOf p) true (readings (selOf p) tm) chans () s
  let o := outsAt p r.2.1
  ⟨r.1, o.1, o.2, r.2.2.1, r.2.2.2⟩


/-- What a run plugs into the machine: the activation (`activate` itself, or an
instrumented form of the same computation, `Examples/Demo/BenchProfile.lean`) and a report printed with
the process's stats. -/
structure Hooks (cfg : Cfg) (p : Fin NProc) where
  act : Timing → List ((src : Member PBLoc Mem) × ByteArray) → Loc cfg p → IO (Act cfg p) :=
    fun tm chans s => IO.lazyPure fun _ => activate cfg p tm chans s
  report : IO Unit := pure ()

/-- The benchmark's ports: `127.0.0.1:9500 + p`. -/
def basePort : Nat := 9500

/-- The receiver cluster of wire node `k` (the routing table). -/
def recvTable {cfg : Cfg} (g : Graph cfg) : List (Nat × PBLoc) := (Dag.wireReceivers g.dag 0).1

/-- `pretty_print_bench_results` (bench_client/mod.rs): the percentiles in ms (the readings
are µs), and the sample count. -/
def fmtMs (us : Nat) : String :=
  let frac := us % 1000
  let fracS := toString frac
  s!"{us / 1000}.{"".pushn '0' (3 - fracS.length)}{fracS}"

/-- One machine, on its own thread: drain what arrived, read this tick's timing (the clock
reading now; the interval fire count), tick once, flush this activation's sends to the
processes hosting a receiver of each message. The aggregator prints its outputs. -/
partial def machine (cfg : Cfg) (p : Fin NProc) (hooks : Hooks cfg p) (peers : Array (Option TCP.Socket.Client)) (sender : Sender)
    (ridx : Array (Option (Bool × Array (Nat × Option Nat))))
    (inbox : Inbox NProc) (stop : IO.Ref Bool)
    (t0 : Nat) (lastAct : IO.Ref (Option Nat)) (timers : List (Nat × Nat))
    (nAct totalThroughput totalSamples : IO.Ref Nat)
    (tails : Array ByteArray) (nanos : Array Nat) (s : Loc cfg p) : IO (Array Nat) := do
  if (← stop.get) then return nanos
  let drained ← inbox.drain
  -- this tick's timing: the clock reading; the program's interval timers fire off it
  let nowUs := (← IO.monoNanosNow) / 1000 - t0
  let last ← lastAct.get
  let due := timersDue timers last nowUs
  -- nothing arrived, no timer due, not the first activation: wait for the sockets (DFIR
  -- runs a tick only at start and on a socket or timer wake)
  if drained.isEmpty && !due && (← nAct.get) > 0 then
    inbox.waitOnce
    machine cfg p hooks peers sender ridx inbox stop t0 lastAct timers nAct totalThroughput totalSamples tails nanos s
  else
  lastAct.set (some nowUs)
  let tm : Timing := ⟨nowUs, last⟩
  -- per connection, in order of first arrival: the kept tail, then the new bytes
  let grouped := Shell.groupChunks drained
  let chans := Shell.chansOf selOf tails grouped
  let n0 ← IO.monoNanosNow
  let a ← hooks.act tm chans s
  let tails := Shell.storeTails tails grouped a.tails
  -- the activation's nanos travel as a loop argument: an array behind an `IO.Ref` another
  -- thread created is marked multi-threaded, so every `push` would copy it whole and the
  -- `set` re-mark every element — two O(n) walks per activation
  let nanos := nanos.push ((← IO.monoNanosNow) - n0)
  nAct.modify (· + 1)
  if !a.sends.isEmpty then
    -- each connection carries the receiver's messages, a `demux` message its addressee's
    -- only (`Shell.routeSendsIdx` over the per-node peer index resolved once from
    -- `Dag.wireRoutes`)
    let buckets := Shell.routeSendsIdx (n := NProc) ridx a.sends
    let mut out : Array (TCP.Socket.Client × ByteArray) := #[]
    for q in [0:NProc] do
      let mine := buckets.getD q []
      if !mine.isEmpty then
        if q = p.val then inbox.push (p, frameMsgs mine)
        else match peers.getD q none with
          | some c => out := out.push (c, frameMsgs mine)
          | none => pure ()
    sender.push out
  -- the aggregator's outputs: `pretty_print_bench_results`
  for th in a.throughputs do
    totalThroughput.modify (· + th)
    IO.println s!"Throughput: {th * 1000 / cfg.aggIntervalMs} requests/s"
  for h in a.latencies do
    totalSamples.modify (· + h.len)
    IO.println s!"Latency p50: {fmtMs (h.valueAtQuantile 1 2)} | p99 {fmtMs (h.valueAtQuantile 99 100)} | p999 {fmtMs (h.valueAtQuantile 999 1000)} ms ({h.len} samples)"
  machine cfg p hooks peers sender ridx inbox stop t0 lastAct timers nAct totalThroughput totalSamples tails nanos a.cells

def parseCfg : List String → Cfg
  | [n, ck, dur] => { nClients := n.toNat?.getD 4, ckFreq := ck.toNat?.getD 100, durationMs := dur.toNat?.getD 5000 }
  | [n] => { nClients := n.toNat?.getD 4 }
  | _ => {}

/-- The stats a process prints at the end (relayed by the parent): its activations and the
step time of the first and last quarter (flatness). -/
def printStats (p : Fin NProc) (mine : Array Nat) : IO Unit := do
  let n := mine.size
  if n = 0 then return
  let q := max 1 (n / 4)
  let firstQ := (mine.extract 0 q).foldl (· + ·) 0
  let lastQ := (mine.extract (n - q) n).foldl (· + ·) 0
  IO.println s!"  process {p} ({repr (selOf p).1} {(selOf p).2}): {n} activations, step first quarter {firstQ / q / 1000} µs, last quarter {lastQ / q / 1000} µs"

/-- One OS process (`bench --proc P …`): bind its port, accept the peers' connections, dial
the peers, run the machine for the configured duration, print its stats. Each process has
its own event loop, allocator and reference counts, as deployed Hydro processes do. -/
def procMain (p : Fin NProc) (cfg : Cfg) (hooksOf : Graph cfg → IO (Hooks cfg p)) : IO UInt32 := do
  let srv ← TCP.Socket.Server.mk
  srv.bind (addrOf basePort p.val)
  srv.listen 16
  let inbox ← Inbox.new
  let accepts ← IO.asTask (acceptLoop srv inbox (NProc - 1)) (prio := .dedicated)
  let mut peers : Array (Option TCP.Socket.Client) := #[]
  for q in [0:NProc] do
    if q ≠ p.val then
      let c ← dial (addrOf basePort q) p.val 250
      peers := peers.push (some c)
    else peers := peers.push none
  let sender ← Sender.new
  let _ ← IO.asTask (try sender.writer catch _ => pure ()) (prio := .dedicated)
  match ← IO.wait accepts with
  | .ok () => pure ()
  | .error e => throw e
  let stop ← IO.mkRef false
  let t0 := (← IO.monoNanosNow) / 1000
  let nAct ← IO.mkRef 0
  let totalThroughput ← IO.mkRef 0
  let totalSamples ← IO.mkRef 0
  let lastAct ← IO.mkRef (none : Option Nat)
  let g := Graph.ofCfg cfg
  let hooks ← hooksOf g
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
      machine cfg p hooks peers sender (Shell.routeIndex (n := NProc) (Shell.routeTable (Dag.wireRoutes g.dag 0).1) selOf) inbox stop t0 lastAct timers nAct totalThroughput totalSamples
        (Array.replicate NProc ByteArray.empty) #[] (init cfg p))
    (prio := .dedicated)
  -- the run's duration in one sleep (a polling loop here would cost a core fraction per
  -- process)
  IO.sleep cfg.durationMs.toUInt32
  stop.set true
  inbox.cv.notifyAll
  sender.close
  let nanos ← match ← IO.wait task with
    | .ok a => pure a
    | .error e => throw e
  printStats p nanos
  hooks.report
  let sendErrors ← sender.errors.get
  if sendErrors > 0 then
    IO.println s!"  process {p}: {sendErrors} sends failed"
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

/-- `bench [nClients ckFreq durationMs]`: the parent; `bench --proc P …`: one process, the
plain activation. -/
def benchMain (args : List String) : IO UInt32 := do
  if let "--proc" :: pS :: rest := args then
    if h : pS.toNat?.getD 0 < NProc then
      return ← procMain ⟨pS.toNat?.getD 0, h⟩ (parseCfg rest) (fun _ => pure {})
  parentMain (parseCfg args) args

/-- The harness's reader continuations are outstanding on sockets nobody closes; the
runtime would wait for them at exit, so the entry points (`Examples/Demo/Bench.lean`, `Examples/Demo/BenchProfile.lean`)
flush and exit directly. -/
def exitFlushing (code : UInt32) : IO UInt32 := do
  (← IO.getStdout).flush
  (← IO.getStderr).flush
  IO.Process.exit code.toUInt8

end BenchDemo
