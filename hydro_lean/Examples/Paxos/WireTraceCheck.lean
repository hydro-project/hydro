import Examples.Paxos.WiredSafety

/-!
# Hydro · `WireTrace` scenarios

A `WireTrace` is four functions — who ran, what it was given, what its clock and timers
said, how many bytes it read off each connection — and every such quadruple is a run of
`paxos_core`'s processes at the bytes (`WireTrace.run`). These are a few, written down so
a reader can see the kind of concurrency the headline (`paxos_safe_wired`) ranges over;
`lake exe wired --traces` evaluates them (`checks`). They are witnesses, like
`lake exe explore`; the theorem is over all traces.

- `sync`: round robin over the five members, each activation reads everything queued —
  the lock-step picture.
- `trickle`: the same schedule, each activation reads five bytes per connection, so every
  frame arrives in pieces, cut mid-frame; the commits are the same.
- `silent`: nothing is ever read, so nothing commits.
- `stalled`: acceptor 2 never runs (crash-stop is a trace that stops activating it); a
  majority still commits.
- `lopsided`: the proposers run ten times between the acceptors' activations.
- `lazy`: DFIR's own rule as a schedule. A member is activated only when it has never
  run, has unread bytes on a connection, or one of its interval timers has fired since its
  last activation; the schedule is built greedily, round robin among the members with a
  reason. The first time no member has one the client's payload arrives at proposer 0, the
  second time the schedule stops. It commits, and it stops: an activation is one tick whose
  knot wires are their bodies' outputs of that tick (the sweep `Dag.iterate`), so nothing
  is left for a tick that nothing wakes.

Proposer 0's election timer fires at its first activation (its stagger is 0 s; the clock
runs a millisecond a step), proposer 1's stagger is 15 s and never comes within these runs.
-/

namespace Hydro

namespace WireTraceCheck

open Project

abbrev Mem := paxMem 2 3
abbrev config : PaxosConfig := ⟨1, 5, 10, 15⟩
abbrev D := paxosDag (mem := Mem) (P := Nat) PaxLoc.prop PaxLoc.acc config
abbrev cod : Dag.Codecs D := paxosCodecs (mem := Mem) (P := Nat) PaxLoc.prop PaxLoc.acc config

/-- The body the traces run: the headline's, `paxosBody` — the compiled activation
(`paxosRun`), the function `lake exe wired` calls. -/
abbrev body : WireTrace.Body D := paxosBody (mem := Mem) (P := Nat) PaxLoc.prop PaxLoc.acc config

/-- The interpreter's body, `Dag.iterateBytes` at the codecs. `paxosBody_eq` says it is
`body`; `checks` compares what the two compiled forms commit, because no theorem covers the
`@[csimp]` seams and the stager's compiled output. -/
abbrev bodyI : WireTrace.Body D := WireTrace.Body.ofCodecs cod

def p0 : Member PaxLoc Mem := ⟨.prop, 0⟩
def p1 : Member PaxLoc Mem := ⟨.prop, 1⟩
def a0 : Member PaxLoc Mem := ⟨.acc, 0⟩
def a1 : Member PaxLoc Mem := ⟨.acc, 1⟩
def a2 : Member PaxLoc Mem := ⟨.acc, 2⟩

def members : List (Member PaxLoc Mem) := [p0, p1, a0, a1, a2]

def cycle (ms : List (Member PaxLoc Mem)) (k : Nat) : Member PaxLoc Mem := ms.getD (k % ms.length) p0

/-- The clock: a millisecond a step. -/
def clockUs (k : Nat) : Nat := k * 1000

/-- The inputs: proposer 0 is handed one client payload at step `payloadStep` (an activation
of its, once it leads — `paxos_core` drops payloads handed to a proposer that is not the
leader; `paxos_with_client` routes them); no checkpoints. -/
def inpAt (payloadStep k : Nat) :=
  ((fun (i : Fin 2) => if k = payloadStep ∧ i.val = 0 then #[1000] else #[]), ((fun (_ : Fin 3) => (none : Option Nat)), ()))

/-- A trace from a schedule, a read policy and the payload's step; the readings are the clock
at the step and the member's previous activation (`TickReadings.ofClock`). -/
def traceOf (who : Nat → Member PaxLoc Mem) (reads : Nat → List (Member PaxLoc Mem × Nat)) (payloadStep : Nat) :
    WireTrace D :=
  { who
    inp := inpAt payloadStep
    readings := fun k => TickReadings.ofClock (who k) (clockUs k) (((activationsOf who (who k) k).getLast?).map clockUs)
    reads }

/-- Read `n` bytes off every other member's connection. -/
def readAll (n : Nat) (m : Member PaxLoc Mem) : List (Member PaxLoc Mem × Nat) :=
  (members.filter (· ≠ m)).map fun src => (src, n)

def sync : WireTrace D := traceOf (cycle members) (fun k => readAll 1000000 (cycle members k)) 20
def trickle : WireTrace D := traceOf (cycle members) (fun k => readAll 5 (cycle members k)) 60
def silent : WireTrace D := traceOf (cycle members) (fun _ => []) 20
def stalled : WireTrace D := traceOf (cycle [p0, p1, a0, a1]) (fun k => readAll 1000000 (cycle [p0, p1, a0, a1] k)) 16
def lopsided : WireTrace D :=
  let who := fun k => if k % 13 < 10 then (if k % 2 = 0 then p0 else p1) else cycle [a0, a1, a2] (k % 13 - 10)
  traceOf who (fun k => readAll 1000000 (who k)) 28

/-! ### The DFIR rule as a schedule -/

/-- Whether member `m` has a reason to run at step `k` of a run whose state is `r`: never
run yet; unread bytes on a connection (what its peers have put on the wire to it beyond
its cursor); an interval timer of its own (`Dag.timers`) fired since its last activation. -/
def hasReason (who : Nat → Member PaxLoc Mem) (r : RunState D) (k : Nat) (m : Member PaxLoc Mem) : Bool :=
  let acts := activationsOf who m k
  acts.isEmpty
  || members.any (fun src => src ≠ m && (PassTrace.bytesOn (d := D) who r.sends m src k).size > r.cur m src)
  || timersDue (Dag.timers m D) (acts.getLast?.map clockUs) (clockUs k)

/-- The schedule written so far, as a function (padding past its end is never run). -/
def whoOf (ms : List (Member PaxLoc Mem)) (k : Nat) : Member PaxLoc Mem := ms.getD k p0

/-- The greedy schedule under the DFIR rule, at most `n` more steps: at each step, the run so
far (`WireTrace.run`), then the first member with a reason, round robin from the step. When
no member has one — the network quiet, every timer silent — the client's payload arrives at
proposer 0 if it has not yet (an external input, the third kind of wake), and the second
time the schedule ends. Returns the schedule and the payload's step. -/
def lazySchedule : Nat → List (Member PaxLoc Mem) → Option Nat → List (Member PaxLoc Mem) × Nat
  | 0, acc, ps => (acc, ps.getD acc.length)
  | n + 1, acc, ps =>
    let who := whoOf acc
    let k := acc.length
    let τ := traceOf who (fun k => readAll 1000000 (who k)) (ps.getD (k + 1))
    let r := τ.run body k
    match (members.rotate (k % members.length)).find? (hasReason who r k), ps with
    | some m, _ => lazySchedule n (acc ++ [m]) ps
    | none, none => lazySchedule n (acc ++ [p0]) (some k)
    | none, some ps => (acc, ps)

/-- The lazy schedule, up to 200 steps, and its payload step. -/
def lazyRun : List (Member PaxLoc Mem) × Nat := lazySchedule 200 [] none

def lazyWho : List (Member PaxLoc Mem) := lazyRun.1

def lazy : WireTrace D := traceOf (whoOf lazyWho) (fun k => readAll 1000000 (whoOf lazyWho k)) lazyRun.2

/-- Proposer 0's `sequenced` commits by step `T` under a body (the headline's quantity at
`body`). -/
def commitsAt (b : WireTrace.Body D) (τ : WireTrace D) (T : Nat) : List (Nat × Option Nat) :=
  τ.streamOutputsAt b T p0 Var.zero.succ ⟨0, rfl⟩

/-- The same, computed in one pass over the run (`streamOutputsAt` recomputes the run at
every step; `checks` evaluates this). -/
def commitsAt' (b : WireTrace.Body D) (τ : WireTrace D) (T : Nat) : List (Nat × Option Nat) :=
  (τ.runOutputs b p0 T).2.flatMap fun o => (o.get Var.zero.succ ⟨0, rfl⟩).toList

theorem commitsAt_eq (b : WireTrace.Body D) (τ : WireTrace D) (T : Nat) : commitsAt b τ T = commitsAt' b τ T :=
  τ.streamOutputsAt_eq_runOutputs b T p0 Var.zero.succ ⟨0, rfl⟩

/-- What `lake exe wired --traces` checks (natively compiled; evaluated in the elaborator
these runs take minutes): proposer 0 commits its payload on `sync`, `stalled`, `lopsided`,
`trickle` and `lazy`, never on `silent`; the committed log is the same whether the frames
came whole or in five-byte pieces; the lazy schedule ends — all under the compiled body
`body` — and the compiled body and the interpreter's commit the same logs and put the same
bytes on the wire on `trickle` and `lazy`. -/
def checks : List (String × (Unit → Bool)) :=
  [("sync commits 1000", fun _ => (commitsAt' body sync 40).any (·.2 = some 1000)),
   ("silent commits nothing", fun _ => commitsAt' body silent 60 = []),
   ("stalled (acceptor 2 never runs) commits 1000", fun _ => (commitsAt' body stalled 40).any (·.2 = some 1000)),
   ("lopsided (proposers 10× acceptors) commits 1000", fun _ => (commitsAt' body lopsided 60).any (·.2 = some 1000)),
   ("trickle (5 bytes per connection per activation) commits 1000", fun _ => (commitsAt' body trickle 100).any (·.2 = some 1000)),
   ("trickle and sync commit the same log",
     fun _ => (commitsAt' body trickle 100).filter (·.2.isSome) = (commitsAt' body sync 40).filter (·.2.isSome)),
   ("lazy (the DFIR rule: a member runs only on its first activation, unread bytes, a due timer; the payload when all is quiet) commits 1000",
     fun _ => (commitsAt' body lazy lazyWho.length).any (·.2 = some 1000)),
   ("lazy stops: after the commit no member has a reason to run",
     fun _ => lazyWho.length < 200 && (commitsAt' body lazy lazyWho.length).any (·.2 = some 1000)),
   ("compiled body (paxosRun) and interpreter (Dag.iterateBytes) commit the same on trickle",
     fun _ => commitsAt' body trickle 100 = commitsAt' bodyI trickle 100),
   ("compiled body and interpreter send the same bytes on lazy",
     fun _ => (List.range lazyWho.length).all fun k => (lazy.run body (k + 1)).sends k == (lazy.run bodyI (k + 1)).sends k)]

end WireTraceCheck

end Hydro
