import Examples.Tutorials.Quorum
import Hydro.Project.Wire
import Hydro.Project.Shell
import Hydro.Project.Sockets

/-!
# `lake exe quorum` — `vote_quorum` as four processes over local TCP

One coordinator and three participants (`QLoc`, `qMem`, `qVote` from
`Examples/Tutorials/Quorum.lean`, quorum size 2), each a process on its own thread running
`quorumBody`, the body `vote_quorum_safe_wired` is stated over, on its own cells only. Process
`p` listens on `127.0.0.1:9600 + p` and dials the others; the processes share nothing but the
sockets. The coordinator is handed the requests `10`, `20`, `30` at its first activation.
A process that has nothing to do waits for its sockets.

The harness stops once the coordinator has decided `10` and `30` (or after 10 s), then checks
the property `vote_quorum_safe_wired` states for this execution: every decision has at least 2
yes voters, and nothing is decided twice. It exits 0 on `quorum: OK`.
-/

open Hydro Hydro.Tutorials Hydro.Project Lean Std Std.Async Sockets

namespace QuorumDemo

abbrev D := vote_quorum.dag (mem := qMem) QLoc.coord QLoc.part 2 qVote

abbrev NProc := 4

/-- Process 0 is the coordinator; processes 1 to 3 are participants 0 to 2. -/
def memberOf : Fin NProc → Member QLoc qMem
  | ⟨0, _⟩ => ⟨.coord, ⟨0, by decide⟩⟩
  | ⟨1, _⟩ => ⟨.part, ⟨0, by decide⟩⟩
  | ⟨2, _⟩ => ⟨.part, ⟨1, by decide⟩⟩
  | ⟨3, _⟩ => ⟨.part, ⟨2, by decide⟩⟩

abbrev Loc (p : Fin NProc) := Dag.LocState (memberOf p) D

abbrev Γ0 := [Kind.stream (mem := qMem) QLoc.coord Nat inferInstance .noOrder .exactlyOnce]

/-- Process `p`'s input this activation: the requests (only the coordinator hosts the input). -/
def inputsFor (p : Fin NProc) (reqs : List Nat) : HList (Kind.locDelta (memberOf p)) Γ0 :=
  (fun _ => reqs.toArray, ())

/-- Every process starts from the graph's initial cells. -/
def init (p : Fin NProc) : Loc p :=
  Dag.memoState (memberOf p) D (Dag.restrict (memberOf p) D (Dag.init D))

--[activate]--
/-- One activation of process `p`: `quorumBody` on the bytes that arrived, the input, its cells.
Returns its cells, its decisions, its sends and the tails to keep. -/
def activate (p : Fin NProc) (nowUs : Nat) (lastUs : Option Nat)
    (chans : List ((src : Member QLoc qMem) × ByteArray)) (reqs : List Nat) (s : Loc p) :
    Loc p × List Nat × List WireMsg × List ByteArray :=
  let r := quorumBody (mem := qMem) QLoc.coord QLoc.part 2 qVote (memberOf p)
    (TickReadings.ofClock (memberOf p) nowUs lastUs) chans (inputsFor p reqs) s
  let decided := (hostedList (memberOf p) .coord).flatMap (fun i => (r.2.1.get Var.zero i).toList)
  (r.1, decided, r.2.2.1, r.2.2.2)
--[/activate]--

def basePort : Nat := 9600

def deadlineMs : Nat := 10000

def theRequests : List Nat := [10, 20, 30]

instance : BEq QLoc := ⟨fun a b => decide (a = b)⟩

/-- The routing table: wire node → receiver cluster. -/
def recvTable : List (Nat × QLoc) := (Dag.wireReceivers D 0).1

/-- One process, on its own thread: take the bytes that arrived, tick once, flush the sends.
It records its decisions in `rec`; the harness reads them only after the stop. -/
partial def machine (p : Fin NProc) (peers : Array (Option TCP.Socket.Client)) (sender : Sender)
    (inbox : Inbox NProc) (stop : IO.Ref Bool) (rec : IO.Ref (Array Nat)) (injected : IO.Ref Bool)
    (clock0 : Nat) (lastAct : IO.Ref (Option Nat)) (tails : Array ByteArray) (s : Loc p) : IO Unit := do
  if (← stop.get) then return
  let drained ← inbox.drain
  let grouped := Shell.groupChunks drained
  let chans := Shell.chansOf memberOf tails grouped
  let reqs ← do
    if p.val = 0 && !(← injected.get) then
      injected.set true
      pure theRequests
    else pure []
  let nowUs := (← IO.monoNanosNow) / 1000 - clock0
  let last ← lastAct.get
  -- nothing arrived and nothing to inject: wait for the sockets
  if drained.isEmpty && reqs.isEmpty && last.isSome then
    inbox.waitOnce
    machine p peers sender inbox stop rec injected clock0 lastAct tails s
  else
  lastAct.set (some nowUs)
  let (cells, decided, sends, newTails) ← IO.lazyPure fun _ => activate p nowUs last chans reqs s
  let tails := Shell.storeTails tails grouped newTails
  if !sends.isEmpty then
    let mut out : Array (TCP.Socket.Client × ByteArray) := #[]
    for q in [0:NProc] do
      if hq : q < NProc then
        let mine := wireForTable recvTable (memberOf ⟨q, hq⟩) sends
        if !mine.isEmpty then
          if q = p.val then inbox.push (p, frameMsgs mine)
          else match peers.getD q none with
            | some c => out := out.push (c, frameMsgs mine)
            | none => pure ()
    sender.push out
  rec.modify (· ++ decided.toArray)
  IO.eprintln s!"quorum: process {p}: {drained.size} chunks in, requests {reqs} → decided {decided}, {sends.length} messages out"
  let mine ← rec.get
  if p.val = 0 && mine.contains 10 && mine.contains 30 then stop.set true
  machine p peers sender inbox stop rec injected clock0 lastAct tails cells

/-- One process of the run: its socket, inbox, sender and decision record. -/
structure Proc where
  p : Fin NProc
  srv : TCP.Socket.Server
  inbox : Inbox NProc
  sender : Sender
  decisions : IO.Ref (Array Nat)

def main : IO UInt32 := do
  IO.eprintln s!"quorum: vote_quorum as {NProc} processes (1 coordinator, 3 participants) over TCP 127.0.0.1:{basePort}+p"
  let procs ← (List.finRange NProc).toArray.mapM fun p => do
    let srv ← TCP.Socket.Server.mk
    srv.bind (addrOf basePort p.val)
    srv.listen 8
    pure (⟨p, srv, ← Inbox.new, ← Sender.new, ← IO.mkRef #[]⟩ : Proc)
  let accepts ← procs.mapM fun pr =>
    IO.asTask (acceptLoop pr.srv pr.inbox (NProc - 1)) (prio := .dedicated)
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
  IO.eprintln "quorum: all sockets connected"
  let stop ← IO.mkRef false
  for pr in procs do
    let _ ← IO.asTask (try pr.sender.writer catch _ => pure ()) (prio := .dedicated)
  let clock0 := (← IO.monoNanosNow) / 1000
  let t0 ← IO.monoMsNow
  let tasks ← procs.mapM fun pr => do
    IO.asTask (machine pr.p (peersOf.getD pr.p.val #[]) pr.sender pr.inbox stop pr.decisions
      (← IO.mkRef false) clock0 (← IO.mkRef none) (Array.replicate NProc ByteArray.empty) (init pr.p))
      (prio := .dedicated)
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
  let mut decided : List Nat := []
  for pr in procs do
    if pr.p.val = 0 then decided := (← pr.decisions.get).toList
  IO.println s!"quorum: coordinator decided {decided} in {t1 - t0} ms"
  let voters := fun r => ((List.finRange 3).filter (fun j => qVote j r)).length
  let quorumOk := decided.all (fun r => 2 ≤ voters r)
  let onceOk := decided.all (fun r => decided.count r = 1)
  IO.println s!"quorum: every decision has ≥ 2 yes voters: {quorumOk} ({decided.map fun r => (r, voters r)})"
  IO.println s!"quorum: nothing decided twice: {onceOk}"
  unless quorumOk && onceOk && decided.contains 10 && decided.contains 30 do
    IO.eprintln "quorum: FAILED"
    return 1
  IO.println "quorum: OK"
  return 0

end QuorumDemo

/-- The reader threads block on sockets nobody closes, so the harness exits directly. -/
def main : IO UInt32 := do
  let code ← QuorumDemo.main
  (← IO.getStdout).flush
  (← IO.getStderr).flush
  IO.Process.exit code.toUInt8
