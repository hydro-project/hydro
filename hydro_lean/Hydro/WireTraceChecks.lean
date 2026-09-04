import Hydro.HydroGenToy
import Hydro.Project.Model.WireTrace

/-!
# Hydro · the top-level model admits the adversary (`WireTraceChecks`)

`paxos_safe_wired` quantifies over every `WireTrace` — who ran at each activation, what
it was handed, what its clock and timers said, how many bytes it read off each
connection — and `WireTrace.streamOutputsAt_eq` says each trace is some lock-step
`Interleaving`, whose outputs are the denotation's. That makes `WireTrace` and
`Interleaving` (`Project/Model/WireTrace.lean`, `WiredRun.lean`, `DistributedModel.lean`)
the trusted statement surface, and a definitional slip there (a read that never lands,
a cursor that never advances, a timer that is never consulted) would make the headline
quietly vacuous. These `#guard`s are concrete traces over the toy graphs showing the
model *admits* each adversarial behaviour (a `#guard` proves admission of one behaviour;
that the model admits *everything a deployment does* is the theorem, not a check):

1. **cross-connection interleaving** — two traces differing only in which connection a
   member reads first produce different output orders with the same multiset;
2. **silence** — a connection never read: its sender's items never reach the reader,
   everything else proceeds;
3. **mid-frame cuts** — one byte per activation vs everything at once: the same outputs at
   the end, and every intermediate output a prefix of the whole-read one;
4. **inert activation** — nothing to read and no input: no output, nothing sent;
5. **the fixpoint race** — the knot's offspring of a member's own item (its loop
   connection read promptly) overtakes an elder item on a stalled connection from the
   other member (Kleene-stage order is not arrival order);
6. **pacing exposes different cuts** — the same inputs over one activation or over
   several: different per-tick emissions, equal flattening;
7. **timer readings are consulted** — an activation whose readings say the interval did
   not fire drops the timer-gated batch; one whose readings say it fired emits;
8. **per-member independence** — one member of a location activated and producing while
   the other, never activated, has produced nothing.

Scenarios 1, 2, 5 and 8 are then written down a second time as explicit lock-step
`Interleaving`s — `who`, every step a tick, the inputs, and the schedule as hand-written
cursor functions (`TransportDec`: delivered count per receiver, sender, step) — and
checked to produce the same outputs: the lock-step model, in its own vocabulary, admits
the same behaviours the bytes do.

The graphs: `toy_loop` (one location, two members; a knot whose body bumps the wire by
one, batches it on a timer-gated tick, ships it to every member of the location through
`broadcast_closed` and re-enters it merged with the external input — every member has a
connection from every member, itself included) and `toy_relay` (the knot body alone:
timer-gated per-tick emissions). Bodies are the interpreter's, `Body.ofCodecs`.
-/

namespace Hydro

namespace WireTraceChecks

open Project

abbrev Mem : Unit → Nat := fun _ => 2

/-- The knot graph, its codecs and body. -/
abbrev DL := toy_loop.dag (L := Unit) (mem := Mem) ()
abbrev codL : Dag.Codecs DL := toy_loop.wireCodecs (L := Unit) (mem := Mem) ()
abbrev bodyL : WireTrace.Body DL := WireTrace.Body.ofCodecs codL

def m0 : Member Unit Mem := ⟨(), 0⟩
def m1 : Member Unit Mem := ⟨(), 1⟩

/-- A schedule from a list (past its end, member 0 — never reached in these runs). -/
def whoOf (ms : List (Member Unit Mem)) (k : Nat) : Member Unit Mem := ms.getD k m0

/-- The clock: a millisecond an activation, starting at one; the interval timer of the
toy fires every microsecond, so every activation's readings say it fired. -/
def clockUs (k : Nat) : Nat := (k + 1) * 1000

def readingsOf (who : Nat → Member Unit Mem) (k : Nat) : TickReadings (who k) :=
  TickReadings.ofClock (who k) (clockUs k) (((activationsOf who (who k) k).getLast?).map clockUs)

/-- One stream input: member `i` is handed `xs` at activation `k`. -/
def inpOf (hand : Nat → Fin 2 → Array Nat) (k : Nat) :
    HList (Kind.delta (mem := Mem)) [Kind.stream () Nat inferInstance .noOrder .exactlyOnce] :=
  HList.cons (f := Kind.delta (mem := Mem)) (hand k) HList.nil

def traceL (who : List (Member Unit Mem)) (hand : Nat → Fin 2 → Array Nat)
    (reads : Nat → List (Member Unit Mem × Nat)) : WireTrace DL :=
  { who := whoOf who, inp := inpOf hand, readings := readingsOf (whoOf who), reads }

/-- The knot wire's items member `m` has output by activation `T`. -/
def outL (τ : WireTrace DL) (T : Nat) (m : Member Unit Mem) : List Nat :=
  (τ.runOutputs bodyL m T).2.flatMap fun o => (o.get Var.zero ⟨m.2, rfl⟩).toList

/-- Read everything on connection `src`. -/
def all (src : Member Unit Mem) : Member Unit Mem × Nat := (src, 1000000)

/-- Member 0 is handed `10` at its first activation, member 1 `20` at its. -/
def hand12 (k : Nat) (i : Fin 2) : Array Nat :=
  if k = 0 ∧ i.val = 0 then #[10] else if k = 1 ∧ i.val = 1 then #[20] else #[]

/-! ### 1 · cross-connection interleaving is read-order-controlled

Activation 0: member 0 takes `10`, emits `11` to both members. Activation 1: member 1
takes `20`, emits `21` to both. Activations 2–3: member 0 reads its two connections in one
order or the other. (Reading the peer first lets `21`'s offspring `22` land on the own
connection before it is read at activation 3; the three items both traces carry are the
same multiset in different orders.) -/

def ownFirst : WireTrace DL :=
  traceL [m0, m1, m0, m0] hand12 fun k => if k = 2 then [all m0] else if k = 3 then [all m1] else []

def peerFirst : WireTrace DL :=
  traceL [m0, m1, m0, m0] hand12 fun k => if k = 2 then [all m1] else if k = 3 then [all m0] else []

#guard outL ownFirst 4 m0 = [10, 11, 21]
#guard outL peerFirst 4 m0 = [10, 21, 11, 22]
#guard (outL ownFirst 4 m0).idxOf 11 < (outL ownFirst 4 m0).idxOf 21
#guard (outL peerFirst 4 m0).idxOf 21 < (outL peerFirst 4 m0).idxOf 11
#guard Multiset.ofList (outL ownFirst 4 m0) = Multiset.ofList ((outL peerFirst 4 m0).filter (· ≠ 22))

/-! ### 2 · silence: a connection never read -/

/-- Member 0 reads only its own loop connection, forever; member 1's `21` never arrives. -/
def deaf : WireTrace DL :=
  traceL [m0, m1, m0, m0, m0, m0] hand12 fun k => if k ≥ 2 then [all m0] else []

#guard outL deaf 6 m0 = [10, 11, 12, 13, 14]
#guard (outL deaf 6 m0).all (· < 20)
#guard outL deaf 6 m1 = [20]

/-! ### 3 · mid-frame cuts: frames arrive in pieces

Member 0 reads member 1's connection only (its own loop ignored), whole or one byte per
activation; the frame carrying `21` lands when its last byte does. -/

def whole : WireTrace DL :=
  traceL [m0, m1, m0] hand12 fun k => if k = 2 then [all m1] else []

def bytewise : WireTrace DL :=
  traceL ([m0, m1] ++ List.replicate 30 m0) hand12 fun k => if k ≥ 2 then [(m1, 1)] else []

#guard outL whole 3 m0 = [10, 21]
#guard outL bytewise 32 m0 = [10, 21]
#guard (List.range 33).all fun T => (outL bytewise T m0).isPrefixOf (outL whole 3 m0)
-- the frame is not a single byte: some intermediate activation has read part of it
#guard (List.range 33).any fun T => outL bytewise T m0 = [10]

/-! ### 4 · an activation with nothing to read and no input is inert -/

def idleAt3 : WireTrace DL :=
  traceL [m0, m1, m0, m0, m0] hand12 fun k => if k = 2 ∨ k = 4 then [all m0] else []

#guard (idleAt3.runOutputs bodyL m0 4).2.length = 3
#guard outL idleAt3 4 m0 = outL idleAt3 3 m0
#guard (idleAt3.run bodyL 4).sends 3 = []
#guard (idleAt3.run bodyL 3).sends 2 ≠ []

/-! ### 5 · the fixpoint race: offspring overtakes a stalled elder

Member 0 takes `1`, member 1 takes `100`. Member 0 then reads its own loop connection
twice (`2`, then `3` — each an offspring of the last) before it reads member 1's `101`,
which had been on the wire since activation 1, before `3` existed. -/

def hand1_100 (k : Nat) (i : Fin 2) : Array Nat :=
  if k = 0 ∧ i.val = 0 then #[1] else if k = 1 ∧ i.val = 1 then #[100] else #[]

def race : WireTrace DL :=
  traceL [m0, m1, m0, m0, m0] hand1_100 fun k => if k = 2 ∨ k = 3 then [all m0] else if k = 4 then [all m1] else []

#guard outL race 5 m0 = [1, 2, 3, 101]
-- `101` was sent at activation 1, `3` produced at activation 3; `3` is read first
#guard (race.run bodyL 2).sends 1 ≠ [] ∧ (race.run bodyL 3).sends 2 ≠ []
#guard (outL race 5 m0).idxOf 3 < (outL race 5 m0).idxOf 101

/-! ### 6 · pacing exposes different cuts (`toy_relay`, one member) -/

abbrev Mem1 : Unit → Nat := fun _ => 1
abbrev DR := toy_relay.dag (L := Unit) (mem := Mem1) ()
abbrev codR : Dag.Codecs DR := toy_relay.wireCodecs (L := Unit) (mem := Mem1) ()
abbrev bodyR : WireTrace.Body DR := WireTrace.Body.ofCodecs codR

def r0 : Member Unit Mem1 := ⟨(), 0⟩

def inpR (hand : Nat → Array Nat) (k : Nat) :
    HList (Kind.delta (mem := Mem1)) [Kind.stream () Nat inferInstance .noOrder .exactlyOnce] :=
  HList.cons (f := Kind.delta (mem := Mem1)) (fun _ => hand k) HList.nil

def readingsR (fired : Bool) (k : Nat) : TickReadings (mem := Mem1) r0 :=
  if fired then TickReadings.ofClock r0 (clockUs k) (if k = 0 then none else some (clockUs (k - 1)))
  else { clock := fun _ _ => clockUs k, fires := fun _ _ _ _ => 0 }

def traceR (hand : Nat → Array Nat) (fired : Bool) : WireTrace DR :=
  { who := fun _ => r0, inp := inpR hand, readings := fun k => readingsR fired k, reads := fun _ => [] }

/-- The per-tick emissions (the second output, an atomic stream: one array per tick). -/
def ticksR (τ : WireTrace DR) (T : Nat) : List (List Nat) :=
  (τ.runOutputs bodyR r0 T).2.map fun o => ((o.get Var.zero.succ ⟨0, rfl⟩).getD #[]).toList

def atOnce : WireTrace DR := traceR (fun k => if k = 0 then #[3, 5] else #[]) true
def spread : WireTrace DR := traceR (fun k => if k = 0 then #[3] else if k = 1 then #[5] else #[]) true

#guard ticksR atOnce 2 = [[4, 6], []]
#guard ticksR spread 2 = [[4], [6]]
#guard ticksR atOnce 2 ≠ ticksR spread 2
#guard (ticksR atOnce 2).flatten = (ticksR spread 2).flatten

/-! ### 7 · timer readings are consulted: a tick whose interval did not fire drops its batch -/

def noFire : WireTrace DR := traceR (fun k => if k = 0 then #[3, 5] else #[]) false

#guard ticksR noFire 2 = [[], []]
#guard ticksR atOnce 1 = [[4, 6]]

/-! ### 8 · per-member independence: member 1 is never activated -/

def onlyM0 : WireTrace DL :=
  traceL [m0, m0, m0] hand12 fun k => if k ≥ 1 then [all m0] else []

#guard outL onlyM0 3 m0 = [10, 11, 12]
#guard outL onlyM0 3 m1 = []

/-! ## The same behaviours as lock-step interleavings

An `Interleaving` is the model `paxos_safe_wired` is proven through (`WireTrace` →
`PassTrace` → `Interleaving` → `Dag.run` → `Dag.denote`). Here the cursor functions are
written by hand — `cur i j k` = how many of sender `j`'s messages receiver `i` has taken by
step `k` (`cumMax`: a cursor never goes back) — and the outputs must be the traces'. -/

/-- The toy's decisions at the machine: all `Unit`. -/
def tdecS (pacing : Pacing Unit Mem) : ToyLoopDec (SchedSem Unit Mem pacing) 2 := ⟨⟨(), ()⟩, ()⟩

/-- The toy's schedule record from the cursors: the interval fires once per tick. -/
def schedS (pacing : Pacing Unit Mem) (cur : Fin 2 → Fin 2 → Nat → Nat) :
    ToyStepSched (SchedSem Unit Mem pacing) 2 := ⟨⟨fun _ _ => 1⟩, cur⟩

def interOf (who : List (Member Unit Mem)) (hand : Nat → Fin 2 → Array Nat)
    (cur : Fin 2 → Fin 2 → Nat → Nat) : Interleaving DL :=
  let w := whoOf who
  let pacing := pacingOf w (fun _ => true)
  Interleaving.mk w (fun _ => true) (inpOf hand)
    (toy_loop.schedOf pacing () (tdecS pacing) (schedS pacing cur))

def outI (I : Interleaving DL) (T : Nat) (m : Member Unit Mem) : List Nat :=
  I.streamOutputsAt T m Var.zero ⟨m.2, rfl⟩

/-- Scenario 1: member 0 takes its own connection's message at step 2 and member 1's at
step 3, or the other way round. -/
def ownFirstI : Interleaving DL :=
  interOf [m0, m1, m0, m0] hand12 fun i j k =>
    if i.val = 0 ∧ j.val = 0 then (if k ≥ 2 then 1 else 0)
    else if i.val = 0 ∧ j.val = 1 then (if k ≥ 3 then 1 else 0) else 0

def peerFirstI : Interleaving DL :=
  interOf [m0, m1, m0, m0] hand12 fun i j k =>
    if i.val = 0 ∧ j.val = 1 then (if k ≥ 2 then 1 else 0)
    else if i.val = 0 ∧ j.val = 0 then (if k ≥ 3 then 2 else 0) else 0

#guard outI ownFirstI 4 m0 = outL ownFirst 4 m0
#guard outI peerFirstI 4 m0 = outL peerFirst 4 m0

/-- Scenario 2: member 0's cursor on member 1 stays at zero. -/
def deafI : Interleaving DL :=
  interOf [m0, m1, m0, m0, m0, m0] hand12 fun i j k =>
    if i.val = 0 ∧ j.val = 0 then (if k ≥ 2 then k - 1 else 0) else 0

#guard outI deafI 6 m0 = outL deaf 6 m0
#guard outI deafI 6 m1 = outL deaf 6 m1

/-- Scenario 5: member 0's own connection is read at steps 2 and 3, member 1's at step 4. -/
def raceI : Interleaving DL :=
  interOf [m0, m1, m0, m0, m0] hand1_100 fun i j k =>
    if i.val = 0 ∧ j.val = 0 then (if k ≥ 3 then 2 else if k ≥ 2 then 1 else 0)
    else if i.val = 0 ∧ j.val = 1 then (if k ≥ 4 then 1 else 0) else 0

#guard outI raceI 5 m0 = outL race 5 m0

/-- Scenario 8: only member 0 runs. -/
def onlyM0I : Interleaving DL :=
  interOf [m0, m0, m0] hand12 fun i j k =>
    if i.val = 0 ∧ j.val = 0 then (if k ≥ 1 then k else 0) else 0

#guard outI onlyM0I 3 m0 = outL onlyM0 3 m0
#guard outI onlyM0I 3 m1 = []

end WireTraceChecks

end Hydro
