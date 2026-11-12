---
sidebar_position: 5
title: Wire Codecs and Byte-Level Traces
---

# Wire Codecs and Byte-Level Traces

`paxos_safe_wired` is stated over processes that exchange bytes over TCP connections, each reading whatever prefix of its connections has arrived (cut anywhere, mid-frame included), activated in any order. Its proof goes through the [lock-step machine](./lockstep-machine.md), whose network is typed queues per connection and whose deliveries are chosen by a [schedule](./graph-ir-causality.md#representation): per **network node** (`broadcast_closed` or `demux`, the graph's form of a send to a cluster), receiver and sender, a delivery cursor at every step, and per timer a script of readings. Each process runs the loop body `Dag.iterateBytes`, which equals `Dag.passBytes`, the same body through `Dag.locStep` ([The Activation](./activation.md)).

Two gaps separate a deployment from that machine. The first is representation: bytes, frames and partial reads against typed messages. The codec layer closes it. The second is that a deployment has no schedule, so the proof constructs one from the trace and shows, by induction over the trace, that the machine run under it equals the trace's run, activation for activation. The bytes on these connections carry the network traffic that the denotational semantics `Values` treats as an identity operator ([The Network as Identity](../semantics/values.md#the-network-as-identity)): their timing, framing and partial reads exist only from the step machine down, and at this layer they are bytes.

## The `Wire` Class

The codecs mirror the Rust message structs' `#[derive(Serialize, Deserialize)]` under `bincode`: a value is its fields in order, naturals as LEB128 varints, tags as one byte, lists length-prefixed. A `Wire` instance is an encoder, one reader with two laws, and a buffer writer `put` with its equation `put_eq` to the encoder (`Hydro/Wire.lean`):

```lean
class Wire (α : Type) where
  encode : α → ByteArray
  getK : {β : Type} → ByteArray → Nat → (α → Nat → Except String β) → Except String β
  getK_bind : ∀ {β : Type} (b : ByteArray) (i : Nat) (k : α → Nat → Except String β),
    getK b i k = Wire.bindGet (getK b i fun a j => .ok (a, j)) k
  getK_encode : ∀ (x : α) (pre suf : ByteArray),
    getK (pre ++ encode x ++ suf) pre.size (fun a j => .ok (a, j)) = .ok (x, pre.size + (encode x).size)
  …
```

The reader hands the value and the next offset to a continuation instead of returning `Except String (α × Nat)`, because the pair-returning form allocates an `Except` and a pair for every field of every message (measured on a `P2b`: 136 ns per decode with the pair-returning reader, 101 ns with `getK`). `getK_bind` says the continuation is applied once, to what was read. `getK_encode` states the round trip, and it holds at any offset in any buffer (`pre ++ encode x ++ suf`), because a field sits in the middle of its message and a message in the middle of a connection's bytes.

`getK` is the class's only reader. `Wire.get` (the reader at the identity continuation) and `Wire.decode` (the reader at the continuation that checks the offset reached the end) are definitions over it, so the decoder the theorems mention is the decoder the codecs run, and no second, faster reader needs a compiler seam (a [`@[csimp]` equation](./activation.md#the-csimp-seams)) between the theorem and the binary. Instances are built with `Wire.ofSpec`, which takes a pair-shaped reader `g` to prove the laws about and the continuation reader `K` with `K b i k = bindGet (g b i) k` (`bindGet` applies the continuation to a successful result); the message structs are `Wire.ofEquiv` of their field tuples.

What a network node's payload looks like in bytes is a `Codec`, and every codec the elaborator (the `hydro def` command, which compiles a program to its graph) builds is `Codec.ofWire`:

```lean
structure Codec (α : Type) where
  enc : α → ByteArray
  dec : ByteArray → Except String α
  dec_enc : ∀ x, dec (enc x) = .ok x
```

The round trip is a field of the structure, so a wrong codec is a failed proof downstream, not a dropped message at run time.

## Frames

A connection carries frames: `frame x = encode (encode x : ByteArray)`, the size as a varint followed by the bytes. `Wire.framesFrom` reads every complete frame from an offset and returns the incomplete tail, which the receiver keeps and prepends to its next read. The law that turns "bytes read" into "messages parsed" is:

```lean
theorem framesFrom_prefix_frameAll {α : Type} [Wire α] : ∀ (xs : List α) (b : Nat), b ≤ (frameAll xs).size →
    ∀ (acc : Array (Except String α)),
    framesFrom ((frameAll xs).extract 0 b) 0 acc
      = (acc ++ ((xs.take (splitFrames xs b).1).map .ok).toArray,
         (frameAll (xs.drop (splitFrames xs b).1)).extract 0 (splitFrames xs b).2)
```

`splitFrames xs b` is how many of the framed values fit in the first `b` bytes and how many bytes of the next one: cutting the stream anywhere yields exactly the frames that fit, decoded, and the bytes of the one that does not.

`parseFrames` (in `Hydro/Project/Process.lean`) is `Wire.frames` with the decode errors dropped, the definition the proofs use. Its compiled form `parseFramesImpl` reads the frame length and then the message's fields in place at the frame's offset, with no copy of the frame body. The `@[csimp]` equation between them goes through `Wire.Restr`, a reader that reads inside `b.extract i j` as it reads `b` from `i`:

```lean
def Restr {α : Type} (g : ByteArray → Nat → Except String (α × Nat)) : Prop :=
  (∀ (b : ByteArray) (i : Nat) (x : α) (e : Nat), g b i = .ok (x, e) → i < e ∧ e ≤ b.size) ∧
  ∀ (b : ByteArray) (i j k e : Nat) (x : α), j ≤ b.size → e ≤ j - i →
    (g (b.extract i j) k = .ok (x, e) ↔ g b (i + k) = .ok (x, i + e))
```

It holds for naturals and byte arrays and is closed under products, which covers `WireMsg`.

## Messages, Numbering and Routing

A message is a triple (`Hydro/Project/Wire.lean`):

```lean
abbrev WireMsg := Nat × Nat × ByteArray
```

the network node's number, the sender's member index, the encoded payload. Network nodes are numbered in graph order, the graphs of called modules inline (`Dag.wireReceivers` returns the numbering with each node's receiver location, the routing table). `Dag.wireEncode` turns a process's sends into messages and `Node.inboxOfWire` decodes a node's inbox from the received batches with `decodeAt cod msgs k j`, which reads only node `k`'s messages from sender `j`. The theorem that ties the two ends of a connection states the round trip at the level of whole graphs:

```lean
theorem Dag.inboxOfWire_wireEncode_self (m src : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)}
    (d : Dag Γ outs) (cod : Dag.Codecs d) (snd : Dag.Sends src d) :
    (Dag.inboxOfWire m d cod 0 [⟨src, (Dag.wireEncode src d cod 0 snd).1⟩]).1 = Dag.inboxOfSends m src d snd :=
  Dag.inboxOfWire_wireEncode m src d cod 0 snd _ (fun _ _ _ => rfl)
```

`Dag.inboxOfSends` is what the lock-step machine enqueues at the receiver (`enqueueLoc`). Its proof rests on two facts: the two walks number the nodes alike, and a node's messages carry its number and no other's (`Dag.wireEncode_mem`), so splitting the message list at a node boundary never mixes nodes.

A sender puts on its connection to `m` only the messages of nodes whose receiver cluster `m` is in (`Dag.wireFor`, by the routing table), and `Dag.inboxOfWire_wireFor` says the filter changes nothing `m` reads. A `demux` payload carries its target member's address as its first varint; `Dag.wireRoutes` records each `demux` node's address function so a shell can send a message to the one process its address names. That is a harness optimization: the receiver keeps only the messages addressed to it either way, and the theorems quantify over every byte history.

## Reading the Machine as Processes

The [lock-step machine](./lockstep-machine.md) with one member active per step is an `Interleaving` (`Hydro/Project/Model/WiredRun.lean`; [Processes](./processes.md#interleavings)):

```lean
structure Interleaving {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  who : Nat → Member L mem
  tick : Nat → Bool
  inp : Nat → HList (Kind.delta (mem := mem)) Γ
  sched : Dag.Sched d
```

`I.view k m` is member `m`'s lock-step cells before step `k`. Those cells include the network's in-flight queue toward `m`, which a real process does not hold, so `Hydro/Project/Model/DistributedModel.lean` reads the machine as processes through `loopView k m := Dag.arrivedOnly m d (I.view k m)`, the cells with the in-flight queues emptied. The running process is fed `arrivals k`, the in-flight prefix up to the schedule's cursor, and `timing k`, the schedule's scripts read at its cells' tick counts. Then `loop_pass` says its `Dag.pass` produces the next step's loop cells and the outputs and sends of `I.active k`, the lock-step machine's step at `k`, and `loop_idle` says a process that is not running does not change: its peers' sends are in flight, not in its cells.

An interleaving is **batched** when each step's arrivals decode from some per-sender batches of `WireMsg`s. For a batched interleaving the loop body over encoded messages equals the lock-step step:

```lean
theorem loop_pass_wire (k : Nat) :
    B.loopPassWire k = (I.loopView (k + 1) (I.who k), (I.active k).2.1, I.wireOut cod k) := by
  …
```

(`Interleaving.Batched.loop_pass_wire`, where `wireOut cod k` is `Dag.wireEncode` of the lock-step sends). The codecs' round trip is used here, through `view_idle_wire`: every other process's queues grow by the decode of the active process's bytes.

## Byte-Level Traces

`paxos_safe_wired` quantifies over a `WireTrace`, four functions (`Hydro/Project/Model/WireTrace.lean`):

```lean
structure WireTrace {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  who : Nat → Member L mem
  inp : Nat → HList (Kind.delta (mem := mem)) Γ
  readings : (k : Nat) → TickReadings (who k)
  reads : Nat → List (Member L mem × Nat)
```

which process ran, the inputs it was handed, its clock and timer readings, and how many bytes it asked for on each connection. Every quadruple is a trace; there is nothing to assume. A trace is run by a `WireTrace.Body`, the loop body at the bytes. `Body.ofCodecs cod` is defined as the **interpreter** `Dag.iterateBytes` at the shell's readings, the body that walks the graph as data; a staged activation is another body, equal to it by its generated equation ([Staging](./staging.md)). The run (`WireTrace.run`) keeps per connection the receiver's byte cursor and kept tail. At step `k` the running process gets, per connection it reads, its tail followed by the next `min (requested, available)` bytes of the sender's framed stream so far; its sends go onto its connections as frames (`frameMsgs`).

The induction is not carried on `WireTrace` itself but on `PassTrace`, the same four functions plus a tick flag per step, each step run through `Dag.passBytes`. The flag is there because the interleaving allows stutters (a running member that does not tick); a `WireTrace` always ticks. `WireTrace.toPass` sets the flag to `true` everywhere, and on a valid graph a wire trace's run is `toPass`'s (`WireTrace.run_eq`, from `Dag.iterateBytes_eq_passBytes`).

### Parsing Bytes into Messages

The first invariant is about the run alone:

```lean
structure RunInv (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : Prop where
  cur_le : ∀ m src, τ.cur cod k m src ≤ (frameMsgs (τ.stream cod m src k)).size
  buf_eq : ∀ m src, τ.buf cod k m src = tailOf (τ.stream cod m src k) (τ.cur cod k m src)
```

Every byte cursor is within what the sender has written, and every kept tail is the bytes of the frame the cursor cuts. Under it, `PassTrace.parse_eq` computes what activation `k` parses off a connection: the next messages of the sender's stream, as many as the bytes read complete, with the new tail at the new cursor. The proof rests on the framing law.

### The Induced Schedule

The schedule is read off the trace. `pc k m src` is how many of `src`'s messages to `m` the receiver has parsed before step `k`, and at a network node the cursor is that count, restricted to the node and sender, one step ahead:

```lean
  | _, _, .broadcast_closed (c := c) (p := p) _ _, cod, base => fun i j t =>
    ccount who sent pc (bcSel cod base j.val) ⟨p, i⟩ ⟨c, j⟩ (t + 1)
```

`t + 1` because the lock-step machine delivers at step `t` what the cursor at `t` allows, and the receiver's activation at `t` parses its messages during step `t`; a cursor at `t` that counted only what was parsed before `t` would deliver one activation late. The machine's cursors are cumulative maxima of the schedule's (`cumMax`), so any function is a legal schedule; this one is monotone, so the maximum is itself (`cumMax_eq_of_mono`). At a timer node the script's entry `n` is the reading the member took at its `n`-th tick (`nthTick`), which chooses the step and is noncomputable; the schedule exists only inside the proof, so the noncomputability is harmless.

The second invariant says what the lock-step cells hold under that schedule:

```lean
  | _, _, .broadcast_closed (c := c) _ _, cod, base, s => ∀ i j,
    s i j = ⟨(unconsumed who sent pc (bcSel cod base j.val) m ⟨c, j⟩ k).toArray,
      ccount who sent pc (bcSel cod base j.val) m ⟨c, j⟩ k⟩
  …
  | _, _, .source_interval .., _, _, s => ∀ i, s i = (ticksOf who tick m k).length
```

(`Node.TraceQueues`, shortened): each receiving end's queue holds the connection's messages past the parsed prefix, in order, and its delivered count equals the parsed count; each timer cell equals the member's tick count. The machine's in-flight queue is therefore exactly what has been sent and not yet parsed: bytes sitting in TCP, or a frame cut in half.

The induction carries both, plus the identification of the trace's cells and bytes with the model's:

```lean
structure Inv (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : Prop where
  queues : ∀ m, Dag.TraceQueues τ.who (τ.sent cod) (τ.pc cod) τ.tick k m d cod 0 ((τ.interleaving cod).view k m)
  cells : ∀ m, τ.cells cod k m = (τ.interleaving cod).loopView k m
  sends : ∀ k' < k, τ.sends cod k' = (τ.interleaving cod).wireOut cod k'
```

`PassTrace.pass_eq_of_inv` proves the induction step. Under `Inv` at `k`, the trace's activation (`Dag.pass` on the parsed batches with the shell's readings as timing) equals the model's `loopPass k`. The arrivals match by `Dag.traceQueues_arrivals`. The timing matches at a tick by `Dag.traceQueues_timing`, and at a stutter by `Dag.pass_false_timing`: a pass that does not tick reads no timer, so any timing serves. The idle and active cases of `TraceQueues` (`Dag.traceQueues_idle`, `Dag.traceQueues_active`) advance the invariant: a process that does not run receives the running one's bytes into its in-flight queue, and the running one's queues lose what it parsed.

### The Output Theorem

```lean
theorem streamOutputsAt_eq (hv : d.valid = true) (cod : Dag.Codecs d) (τ : WireTrace d) (T : Nat) (m : Member L mem) {ℓ : L}
    {α : Type} {iα : DecidableEq α} {ord : StrOrd} {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    τ.streamOutputsAt (Body.ofCodecs cod) T m v i = (τ.interleaving cod).streamOutputsAt T m v i := by
  …
```

What a stream output of process `m` has carried by activation `T` under the interpreter's body is what the induced interleaving's output carries by step `T`. `paxos_safe_wired` (`Examples/Paxos/WiredSafety.lean`) opens with this rewrite, after `paxosBody_eq` replaces the staged body by the interpreter's:

```lean
  rw [paxosBody_eq, τ.streamOutputsAt_eq (paxosDag_valid prop acc config)] at hw hw'
```

From there the statement is about an interleaving, and `Interleaving.outputs` with `paxos_core.dag_denote` carry it to the program's `SchedSem` run, where the coupling corner takes over ([Soundness](../soundness.md)).

## Witnesses for the Model

`WireTrace`, `Interleaving` and the definitions under them are trusted statement surface: a read that never lands or a timer that is never consulted would make the headline true and empty. `Hydro/WireTraceChecks.lean` `#guard`s concrete traces over the toy graphs showing the model admits eight behaviours: interleaving across connections, silence, mid-frame cuts, inert activations, the knot race (an item derived from a member's own item through a `forward_ref` cycle overtaking an older item on a stalled connection), the same inputs split over different numbers of activations (different per-tick outputs, the same flattening), consulted timer readings and per-member independence; four of them are then written a second time as hand-built lock-step interleavings with the same outputs. `Examples/Paxos/WireTraceCheck.lean` runs six Paxos traces under the staged body, among them `trickle` (five bytes per connection per activation, every frame cut) and `lazy` (DFIR's wake rule as a schedule: a member runs only when it has unread bytes, a fired timer, or has never run). `lazy` commits and stops, which shows that an activation leaves nothing pending for a tick that nothing wakes. `lake exe wired --traces` evaluates them.
