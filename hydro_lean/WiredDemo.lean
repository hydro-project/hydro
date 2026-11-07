import Hydro
import Hydro.Project.Wire
import Hydro.Project.Shell
import Hydro.Project.Fused
import Hydro.Paxos.WiredSafety
import Hydro.WireTraceCheck
import Sockets
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
activation is one tick** of all its members, the knot wires their bodies' outputs
of that tick (the sweep, D95). The next tick comes with the next arrival (every
cycle in `paxos_core` crosses the network) or the idle timer. A process that is not
running is inert, so the run is an `Interleaving` as it stands (D76). What a process
runs is `paxosBody` — the body the theorem is stated over (D96: the graph compiled by
`hydro_run` into straight-line code, `paxosRun`, with the kernel-checked `paxosRun.eq` to
`Dag.iterateBytes`); `wired --traces` runs scripted traces under both the compiled body
and the interpreter's and compares (`WireTraceCheck`).

What is checked: the proposers commit a client payload and every commit
any process emits at one slot agrees — the statement `paxos_safe_wired`
makes about **exactly this** execution model. Each process is its own
thread; nothing is shared between them but the sockets (`Sockets.lean`, the harnesses'
shared plumbing — `Async` reader chains, one writer per process, a condition-variable inbox
the readers and the timer thread signal; D94) and a stop flag the harness sets. Each process keeps its own record of what it
committed; the harness reads the records only after the stop. (`wired` is
the witness, not a benchmark: the open-loop `--bench` mode and its harness
checkpoint channel were removed in D92 — `lake exe bench` is the measured
system, the whole `paxos_bench` with its own clients and replicas.)
-/

open Hydro Hydro.Project Lean Std Std.Async Sockets

namespace WiredDemo

abbrev Mem := paxMem 2 3

abbrev config : PaxosConfig := ⟨1, 5, 10, 15⟩

/-- The graph — the headline's (`paxosDag`, `Paxos/WiredSafety.lean`): the cells' type, the
initial cells and the start-up tables (routes, receivers, timers) are read off it; the
activation is `paxosBody`, which carries its codecs. -/
abbrev D := paxosDag (mem := Mem) (P := Nat) PaxLoc.prop PaxLoc.acc config

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
  Kind.sing (mem := Mem) .acc (Option Nat) (Option Nat) inferInstance .totalOrder .exactlyOnce (.monotonic ValueOrder.optNat)]

/-- Process `p`'s inputs this activation: its payloads (at a proposer); no checkpoint ever
arrives (the acceptors' input singleton stays `none` — there are no replicas here). -/
def inputsFor (p : Fin NProc) (payloads : List Nat) : HList (Kind.locDelta (selOf p)) Γ0 :=
  (fun _ => payloads.toArray, (fun _ => none, ()))

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
  ((hostedList (selOf p) .prop).flatMap (fun i => (o.get Var.zero i).toList),
   (hostedList (selOf p) .prop).flatMap (fun i => (o.get Var.zero.succ i).toList))

/-- **One activation of process `p`** — **`paxosBody`**, the body `paxos_safe_wired` is
stated over (`Paxos/WiredSafety.lean`: the compiled activation `paxosRun`, D96 — the graph
staged by `hydro_run` into straight-line code, equal to the library's loop body
`Dag.iterateBytes` by the kernel-checked `paxosRun.eq`): per connection, the kept tail and
what the socket delivered since; the complete frames are the messages (appended, all
delivered), one tick with the payloads (the activation as a sweep — D95), nothing kept in
flight; the sends encoded, the new tails returned. -/
def activate (p : Fin NProc) (nowUs : Nat) (lastUs : Option Nat)
    (chans : List ((src : Member PaxLoc Mem) × ByteArray)) (payloads : List Nat)
    (s : Loc p) : Act p :=
  let r := paxosBody (mem := Mem) (P := Nat) PaxLoc.prop PaxLoc.acc config (selOf p)
    (readings (selOf p) nowUs lastUs) chans (inputsFor p payloads) s
  let o := outsAt p r.2.1
  ⟨r.1, o.1, o.2, r.2.2.1, r.2.2.2⟩

/-- The witness's ports: `127.0.0.1:9400 + p`. -/
def basePort : Nat := 9400

/-- A process's own record of one activation: its commits. -/
abbrev Act' := List (Nat × Option Nat)

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
partial def machine (p : Fin NProc) (peers : Array (Option TCP.Socket.Client)) (sender : Sender)
    (inbox : Inbox NProc) (stop : IO.Ref Bool) (rec : IO.Ref (Array Act'))
    (leader injected committed : IO.Ref Nat)
    (clock0 : Nat) (lastAct : IO.Ref (Option Nat)) (timers : List (Nat × Nat))
    (tails : Array ByteArray) (s : Loc p) : IO Unit := do
  if (← stop.get) then return
  let drained ← inbox.drain
  -- per connection, in order of first arrival: the kept tail, then the new bytes
  let grouped := Shell.groupChunks drained
  let chans := Shell.chansOf selOf tails grouped
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
  -- the first activation: wait for the sockets or the timer thread (DFIR runs a tick at
  -- start, on a socket or timer wake — D93)
  if drained.isEmpty && payloads.isEmpty && last.isSome && !timersDue timers last nowUs then
    inbox.waitOnce
    machine p peers sender inbox stop rec leader injected committed clock0 lastAct timers tails s
  else
  lastAct.set (some nowUs)
  -- the loop body: cut the bytes into frames, enqueue, deliver all, the tick
  let a ← IO.lazyPure fun _ => activate p nowUs last chans payloads s
  let tails := Shell.storeTails tails grouped a.tails
  let t1 ← IO.monoNanosNow
  let dt := t1 - t0
  if !a.ballots.isEmpty then leader.set 1
  committed.modify (· + a.commits.length)
  -- flush — each connection carries the receiver's messages (`Dag.wireFor`: the nodes
  -- whose receiver cluster it is in, the trace model's own routing); a message to a receiver
  -- this process hosts travels the same way, through its own inbox (the loop body keeps
  -- nothing in flight — D78)
  if !a.sends.isEmpty then
    let mut out : Array (TCP.Socket.Client × ByteArray) := #[]
    for q in [0:NProc] do
      if hq : q < NProc then
        let mine := wireForTable recvTable (selOf ⟨q, hq⟩) a.sends
        if !mine.isEmpty then
          if q = p.val then inbox.push (p, frameMsgs mine)
          else match peers.getD q none with
            | some c => out := out.push (c, frameMsgs mine)
            | none => pure ()
    sender.push out
  rec.modify (·.push a.commits)
  if !a.ballots.isEmpty || !a.commits.isEmpty || !payloads.isEmpty then
    IO.eprintln s!"wired: process {p}: inbox {drained.size} payloads {payloads} → ballots {repr a.ballots}, commits {repr a.commits} ({dt / 1000} µs)"
  -- the leader has seen its payload committed: tell the harness
  if (← leader.get) = 1 && (← committed.get) ≥ 1 then
    stop.set true
  machine p peers sender inbox stop rec leader injected committed clock0 lastAct timers tails a.cells

/-- One process of the witness: its socket, inbox, sender, commit record and last activation. -/
structure Proc where
  p : Fin NProc
  srv : TCP.Socket.Server
  inbox : Inbox NProc
  sender : Sender
  commits : IO.Ref (Array Act')
  lastAct : IO.Ref (Option Nat)

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
  -- servers first, then each process's inbox, sender and records
  let procs ← (List.finRange NProc).toArray.mapM fun p => do
    let srv ← TCP.Socket.Server.mk
    srv.bind (addrOf basePort p.val)
    srv.listen 8
    pure (⟨p, srv, ← Inbox.new, ← Sender.new, ← IO.mkRef #[], ← IO.mkRef none⟩ : Proc)
  let accepts ← procs.mapM fun pr =>
    IO.asTask (acceptLoop pr.srv pr.inbox (NProc - 1)) (prio := .dedicated)
  -- dial the peers (saying who is calling)
  let mut peersOf : Array (Array (Option TCP.Socket.Client)) := #[]
  for p in [0:NProc] do
    let mut peers : Array (Option TCP.Socket.Client) := #[]
    for q in [0:NProc] do
      if q = p then peers := peers.push none
      else peers := peers.push (some (← dial (addrOf basePort q) p 100))
    peersOf := peersOf.push peers
  for t in accepts do
    match ← IO.wait t with
    | .ok () => pure ()
    | .error e => throw e
  IO.eprintln "wired: all sockets connected"
  let stop ← IO.mkRef false
  for pr in procs do
    let _ ← IO.asTask (try pr.sender.writer catch _ => pure ()) (prio := .dedicated)
  let t0 ← IO.monoMsNow
  let clock0 := (← IO.monoNanosNow) / 1000
  -- the timer threads: wake a machine at the next fire instant of any of its program's
  -- interval timers (the proposers' election timers), and at the stop
  for pr in procs do
    let timers := Dag.timers (selOf pr.p) D
    let _ ← IO.asTask (do
      while !(← stop.get) do
        let nowUs := (← IO.monoNanosNow) / 1000 - clock0
        let waitUs := match nextDue timers (← pr.lastAct.get) with
          | some n => min (n - nowUs) 200000
          | none => 200000
        IO.sleep (max 1 (waitUs / 1000)).toUInt32
        pr.inbox.cv.notifyOne) (prio := .dedicated)
  let tasks ← procs.mapM fun pr => do
    IO.asTask (machine pr.p (peersOf.getD pr.p.val #[]) pr.sender pr.inbox stop pr.commits (← IO.mkRef 0) (← IO.mkRef 0) (← IO.mkRef 0)
      clock0 pr.lastAct (Dag.timers (selOf pr.p) D) (Array.replicate NProc ByteArray.empty) (init pr.p)) (prio := .dedicated)
  -- the harness waits for the leader's signal (or the deadline), then stops every machine
  for _ in [0:deadlineMs] do
    if (← stop.get) then break
    IO.sleep 1
  let t1 ← IO.monoMsNow
  stop.set true
  for pr in procs do
    pr.inbox.cv.notifyAll
    pr.sender.close
  for t in tasks do
    match ← IO.wait t with
    | .ok () => pure ()
    | .error e => throw e
  -- after the stop: each process's own record
  let mut perProc : Array (Fin NProc × Array Act') := #[]
  for pr in procs do
    perProc := perProc.push (pr.p, ← pr.commits.get)
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
