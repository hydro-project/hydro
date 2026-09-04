import Examples.Demo.BenchDemo
import Hydro.Project.Profile

/-!
# `lake exe benchprof` — the benchmark instrumented, and two microbenches

This file does not define the benchmark: `Examples/Demo/BenchDemo.lean` (`lake exe bench`) runs `paxos_bench` and
knows one hook (`BenchDemo.Hooks`: the activation and an end-of-run report). This entry
point fills the hook and holds the stand-alone measurements:

- `benchprof stages [nClients ckFreq durationMs]`: the benchmark with the activation in
  stages (decode+enqueue / step / encode — the reference step `Dag.locStep`, the
  interpreter's per-process step, not the compiled activation `bench` runs), messages per
  wire node;
- `benchprof nodes [cfg]`: the above plus the step by operator (`Project/Profile.lean`);
- `benchprof trace [cfg]`: every activation with traffic in the first 1.5 s, one line
  (process, µs, bytes in, every message in and out as `node:sender:[leading naturals]` —
  the slot of a sequenced payload, the ballot of a P1a, the address of a `demux` message);
- `benchprof idle P N`: an empty activation of process `P`, stage by stage, no sockets
  (`benchprof idle P N nodes` prints the node ordinals of the profiled walk; `benchprof idle
  P N full` times the full activation only, plus one sweep over finished parts — the walk's
  per-visit floor);
- `benchprof decode N`: the wire path in isolation (decode, parse, frame per message).

`scripts/bench_perf_role.sh P` records a `perf` profile of one role ( the machine thread separated
from the socket threads).
-/

open Hydro Hydro.Project Lean Std

namespace BenchProfile
open BenchDemo

/-- The activation in stages, timed (`benchprof stages`): decode the inbox and
materialize the cells, step the graph, encode the sends. The same computation as `activate`;
the stage nanos accumulate per process plus the message counts. -/
def activateProfiled {cfg : Cfg} (g : Graph cfg) (p : Fin NProc) (tm : Timing) (chans : List ((src : Member PBLoc Mem) × ByteArray))
    (s : Loc cfg p) (prof : IO.Ref (Array Nat)) (nodes : Option (IO.Ref (Array NodeProfile)))
    (wires : IO.Ref (Std.HashMap Nat (Nat × Nat))) : IO (Act cfg p) := do
  -- the diagnostics walk the interpreter over the once-built graph `g.dag` (= `D cfg`, `g.hdag`)
  let s : Dag.LocState (selOf p) g.dag := cast (congrArg (Dag.LocState (selOf p)) g.hdag.symm) s
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
  let st : Loc cfg p × HList (Kind.locDelta (selOf p)) PBOuts × List WireMsg := (cast (congrArg (Dag.LocState (selOf p)) g.hdag) r.1, r.2.1, sends)
  let o := outsAt p st.2.1
  let nmsg := parsed.foldl (fun n c => n + c.2.1.length) 0
  -- messages per wire node, in and out
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
    a.modify 5 (· + 1)
  pure ⟨st.1, o.1, o.2, sends, parsed.map (·.2.2)⟩

/-- The profile report, printed after the process's stats. -/
def report (nodesProfile : Bool) (pr : IO.Ref (Array Nat)) (np : IO.Ref (Array NodeProfile))
    (wires : IO.Ref (Std.HashMap Nat (Nat × Nat)))
    (recv : List (Nat × PBLoc)) : IO Unit := do
let a ← pr.get
if a.size = 8 ∧ a[5]! > 0 then
  let k := a[5]!
  IO.println s!"    per activation: decode+enqueue {a[0]! / k / 1000} µs, step {a[1]! / k / 1000} µs, encode {a[2]! / k / 1000} µs; {a[3]! * 100 / k / 100}.{a[3]! * 100 / k % 100} msgs in, {a[4]! * 100 / k / 100}.{a[4]! * 100 / k % 100} msgs out; {a[6]! * 100 / k} % of activations had input"
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

/-- The leading LEB128 naturals of a payload (the trace's view of a message: the slot of a
sequenced payload, the address of a `demux` message, …). -/
def leadingNats (b : ByteArray) (k : Nat) : List Nat :=
  let rec go (i : Nat) (k : Nat) (acc : List Nat) (fuel : Nat) : List Nat :=
    match fuel with
    | 0 => acc.reverse
    | fuel + 1 =>
      if k = 0 || i ≥ b.size then acc.reverse
      else
        -- one varint from `i`
        let rec varint (j : Nat) (mul : Nat) (v : Nat) (f : Nat) : Nat × Nat :=
          match f with
          | 0 => (v, j)
          | f + 1 =>
            if h : j < b.size then
              let byte := b[j].toNat
              if byte < 128 then (v + byte * mul, j + 1)
              else varint (j + 1) (mul * 128) (v + (byte - 128) * mul) f
            else (v, j)
        let (v, j) := varint i 1 0 10
        go j (k - 1) (v :: acc) fuel
  go 0 k [] 16

/-- `benchprof trace`: the plain activation, each one with traffic printed (first 1.5 s);
the messages in and out are printed as `node:sender:[leading naturals]`. -/
def traceHooks {cfg : Cfg} (p : Fin NProc) (g : Graph cfg) : IO (Hooks cfg p) := do
  IO.eprintln s!"trace wire nodes → receiver: {(recvTable g).map fun (k, r) => (k, repr r)}"
  return { act := fun tm chans s => do
    let a ← IO.lazyPure fun _ => activate cfg p tm chans s
    let nIn := chans.foldl (fun n c => n + c.2.size) 0
    if (nIn > 0 || !a.sends.isEmpty) && tm.nowUs < 1500000 then
      let parsed := chans.map fun c => (c.1.2.val, (parseFrames c.2).1)
      let fmt (m : WireMsg) : String := s!"{m.1}:{m.2.1}:{leadingNats m.2.2 7}"
      let ins := parsed.flatMap fun c => c.2.map fmt
      IO.eprintln s!"trace {tm.nowUs} p{p} in {nIn} bytes {ins} out {a.sends.length} {a.sends.map fmt}"
    pure a }

/-- The hooks of a profiled process: the instrumented activation and the report (the refs
are this process's; the machine thread is the only writer). -/
def hooks {cfg : Cfg} (nodesProfile : Bool) (p : Fin NProc) (g : Graph cfg) : IO (Hooks cfg p) := do
  let pr ← IO.mkRef (#[] : Array Nat)
  let np ← IO.mkRef (#[] : Array NodeProfile)
  let wires ← IO.mkRef ({} : Std.HashMap Nat (Nat × Nat))
  return {
    act := fun tm chans s => activateProfiled g p tm chans s pr (if nodesProfile then some np else none) wires
    report := report nodesProfile pr np wires (recvTable g) }

-- The fused-sweep line's elaboration unifies `Dag.Part`/`Mask` types over the production
-- graph; the other lines' types match shapes already stored. The override covers the measured cost.
set_option maxHeartbeats 600000 in
/-- `benchprof idle P N`: `N` activations of process `P` with an empty inbox, no sockets — the
per-activation base cost, stage by stage (each stage timed in isolation on the same cells). -/
def idleBench (p : Fin NProc) (n : Nat) (ordinals onlyFull : Bool) : IO Unit := do
  let cfg : Cfg := { nClients := 500, ckFreq := 10 }
  let g := Graph.ofCfg cfg
  let tm : Timing := ⟨1000, some 0⟩
  let mut s := init cfg p
  let time (label : String) (k : Nat) (act : Unit → IO Unit) : IO Unit := do
    let t0 ← IO.monoNanosNow
    for _ in [0:k] do act ()
    let t1 ← IO.monoNanosNow
    IO.println s!"  {label}: {(t1 - t0) / k / 1000}.{((t1 - t0) / k % 1000) / 100} µs"
  -- warm up: a few full activations
  for _ in [0:20] do
    s := (activate cfg p tm [] s).cells
  let s0 : Loc cfg p := s
  IO.println s!"idle activation of process {p} ({repr (selOf p).1} {(selOf p).2}), {n} iterations:"
  IO.println s!"  nodes touched/skipped: {Dag.touchCount (selOf p) g.dag}"
  -- the compiled activation (`benchRun`), what `bench` runs
  time "full activate (compiled, benchRun)" n fun _ => do let _ ← IO.lazyPure fun _ => activate cfg p tm [] s0
  -- the interpreter's stages, on the same cells (the reference `Dag.iterateBytes` walks `g.dag`)
  let s0 : Dag.LocState (selOf p) g.dag := cast (congrArg (Dag.LocState (selOf p)) g.hdag.symm) s0
  time "full activate (interpreter, Dag.iterateBytes)" n fun _ => do
    let _ ← IO.lazyPure fun _ =>
      Dag.iterateBytes (selOf p) true g.dag g.cod (Dag.Timing.ofReadings (selOf p) (readings (selOf p) tm) g.dag) [] () s0
  -- `benchprof idle P N full`: the full activation only (a `perf` target), and the cost of
  -- one walk that steps nothing — a re-sweep of a finished activation's parts (every node
  -- `done`): the per-visit floor of the sweep, paid again on every knot round
  if onlyFull then
    let tmg := Dag.Timing.ofReadings (selOf p) (readings (selOf p) tm) g.dag
    let r0 := Dag.sweepF (selOf p) true g.dag g.cod 0 (fun _ => ([] : List ((_src : Member PBLoc Mem) × List WireMsg))) tmg true true () (Mask.const [] true) s0 (Dag.Part.init (selOf p) g.dag)
    time "sweepF on all-done parts (one walk, no steps)" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.sweepF (selOf p) true g.dag g.cod 0 (fun _ => ([] : List ((_src : Member PBLoc Mem) × List WireMsg))) tmg false true () (Mask.const [] true) r0.cells r0.part
    time "sweepF on all-done parts, no encode" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.sweepF (selOf p) true g.dag g.cod 0 (fun _ => ([] : List ((_src : Member PBLoc Mem) × List WireMsg))) tmg false false () (Mask.const [] true) r0.cells r0.part
    return
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
  time "locStep (the reference: knot bodies stepped twice)" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.locStep (selOf p) true g.dag ora () s2
  let r := Dag.locStep (selOf p) true g.dag ora () s2
  time "sweepTop (the activation as a sweep)" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.sweepTop (selOf p) true g.dag tmg () s2
  time "wireEncode" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.wireEncode (selOf p) g.dag g.cod 0 r.2.2
  time "inboxOfWire (one empty batch)" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.inboxOfWire (selOf p) g.dag g.cod 0 [⟨selOf p, []⟩]
  time "sweepF (the fused sweep)" n fun _ => do let _ ← IO.lazyPure fun _ => Dag.sweepF (selOf p) true g.dag g.cod 0 (fun _ => ([] : List ((_src : Member PBLoc Mem) × List WireMsg))) tmg true true () (Mask.const [] true) s0 (Dag.Part.init (selOf p) g.dag)
  -- `benchprof idle P N nodes`: the node ordinals of the profiled walk with their tags (to
  -- read the per-node profile's `#k` back to an operator)
  if ordinals then
    let (_, np) ← Dag.locStepProfiled (selOf p) true g.dag ora () s2 #[]
    IO.println s!"  node ordinals: {np.toList.map (·.1)}"
  time "Dag.init + restrict + memoState" (max 1 (n / 10)) fun _ => do let _ ← IO.lazyPure fun _ => init cfg p

/-- `benchprof decode N`: the wire path in isolation — `N` rounds of encoding, framing, parsing
and decoding 2000 `P2b` messages (the acceptors' reply, the leader's most frequent arrival),
compiled; the per-message costs of `Wire.decode`, `parseFrames` and
`frameMsgs`. -/
def decodeBench (rounds : Nat) : IO Unit := do
  let mk (r : Nat) : Array (P2b 2) := (Array.range 2000).map fun i =>
    { slot := i * 7 + r, ballot := ⟨i % 5 + 1, ⟨i % 2, by omega⟩⟩,
      res := if i % 4 = 0 then .error (some ⟨9, ⟨1, by omega⟩⟩) else .ok () }
  -- one data set per round (so that nothing is hoisted out of the timed loops)
  let sets := (Array.range rounds).map fun r =>
    let msgs := mk r
    let wire : List WireMsg := msgs.toList.map fun m => (3, 1, Wire.encode m)
    (msgs.map fun m => Wire.encode m, wire, frameMsgs wire)
  let n := 2000
  IO.println s!"decode bench: {n} P2b messages per round, {rounds} rounds, {(sets[0]!).2.2.size} framed bytes"
  let t0 ← IO.monoNanosNow
  let mut ok := 0
  for st in sets do
    for b in st.1 do
      match Wire.decode (α := P2b 2) b with
      | .ok _ => ok := ok + 1
      | .error _ => pure ()
  let t1 ← IO.monoNanosNow
  IO.println s!"  Wire.decode (P2b): {(t1 - t0) / (rounds * n)} ns/msg ({ok} ok)"
  let t2 ← IO.monoNanosNow
  let mut cnt := 0
  for st in sets do
    let p := parseFrames st.2.2
    cnt := cnt + p.1.length
  let t3 ← IO.monoNanosNow
  IO.println s!"  parseFrames: {(t3 - t2) / (rounds * n)} ns/msg ({cnt} frames)"
  let t4 ← IO.monoNanosNow
  let mut sz := 0
  for st in sets do
    let f := frameMsgs st.2.1
    sz := sz + f.size
  let t5 ← IO.monoNanosNow
  IO.println s!"  frameMsgs: {(t5 - t4) / (rounds * n)} ns/msg ({sz} bytes)"

/-- The subcommands; the parent relays `stages`/`nodes` to its children (`--proc P stages …`). -/
def profMain (args : List String) : IO UInt32 := do
  match args with
  | "decode" :: rest =>
    decodeBench ((rest.head?.bind String.toNat?).getD 200)
    return 0
  | "idle" :: pS :: rest =>
    if h : pS.toNat?.getD 0 < NProc then
      idleBench ⟨pS.toNat?.getD 0, h⟩ ((rest.head?.bind String.toNat?).getD 2000) (rest.contains "nodes") (rest.contains "full")
      return 0
    else
      IO.eprintln s!"benchprof idle: process {pS} out of range"; return 1
  | "--proc" :: pS :: mode :: rest =>
    if h : pS.toNat?.getD 0 < NProc then
      let p : Fin NProc := ⟨pS.toNat?.getD 0, h⟩
      procMain p (parseCfg rest) (if mode = "trace" then traceHooks p else hooks (mode = "nodes") p)
    else
      IO.eprintln s!"benchprof: process {pS} out of range"; return 1
  | mode :: rest =>
    if mode = "stages" ∨ mode = "nodes" ∨ mode = "trace" then parentMain (parseCfg rest) args
    else
      IO.eprintln "usage: benchprof (stages | nodes | trace) [nClients ckFreq durationMs] | idle P N [nodes] | decode N"; return 1
  | [] =>
    IO.eprintln "usage: benchprof (stages | nodes | trace) [nClients ckFreq durationMs] | idle P N [nodes] | decode N"; return 1

end BenchProfile

def main (args : List String) : IO UInt32 := do
  BenchDemo.exitFlushing (← BenchProfile.profMain args)
