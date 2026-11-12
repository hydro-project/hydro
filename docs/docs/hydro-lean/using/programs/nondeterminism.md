---
sidebar_position: 4
title: "Nondeterminism and Decisions"
---

# Nondeterminism and Decisions

A `nondet!` in the Rust becomes an argument of the Lean program, a **decision**, so that "for every batching" is a `∀ dec` in a contract. As in Rust, an algebraic property often removes the need for one: a fold over an unordered stream whose closure is proved commutative takes no decision. Decisions appear at three kinds of site:

1. **Observations.** An operator that looks at one particular history (where a tick's batch ends, what a snapshot has seen, which order an unordered batch is processed in, what the clock reads) cannot compute its result from the stream's value under the denotational semantics `Values`, because that value captures all the histories the stream could have had ([Grades](./signature.md#grades)). These are the Rust `nondet!` sites.
2. **Timing.** The step machine `SchedSem`, the operational semantics that runs the program on a global clock of steps, has to know when each message is delivered and when each timer fires. The denotational semantics never sees either, and the Rust marks neither.
3. **Cycle depth.** `Values` computes a cycle (a `forward_ref`) by iterating its body from the empty collection, and needs to know how many times.

The three kinds are kept apart, because a contract talks about observations and never about timing. Every value of a decision type yields a run, even a value that asks for more than the stream holds ([Total Decision Spaces](#total-decision-spaces)); otherwise every theorem would carry a legality premise, and every theorem about the machine would have to discharge it.

## Decision Families

An operator that needs a choice takes a decision argument whose type is a field of the interpretation (the instance of `HydroSem` the program is read at, see [the signature](./signature.md)):

```lean
    batch : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries},
      Stream ℓ α ord ret →
      (d : BatchDec (mem ℓ) α ord ret) →
      AtomicStream ℓ α ord ret
```

Each instance fills the families in with the freedom it has, and `Unit` where it has none. The framework names the three kinds above **nondet** (an observation, mirroring a Rust `nondet!`), **sched** (a schedule decision) and **fuel**:

| family | kind | at `Values` | at `SchedSem` | Rust site |
|---|---|---|---|---|
| `BatchDec n α ord ret` | nondet | `BatchCutsOf n α ord ret` | `Unit` | `.batch(&tick, nondet!)`, `use::batch` |
| `SnapDec n α ord` | nondet | `SnapshotCuts n α ord` | `Unit` | `.snapshot(&tick, nondet!)` |
| `OrderSelDec n α` | nondet | `OrderSelection n α` | `Unit` | `.assume_ordering::<TotalOrder>(nondet!)` |
| `TickOrderSelDec n α` | nondet | `TickOrderSelection n α` | `Unit` | the same, on one tick's batch |
| `TicksDec n` | nondet | `TickCounts n` | `Unit` | `use::snapshot` of a bounded singleton |
| `ClockDec n` | nondet | `ClockReadings n` | `TickScript n Nat` | `use::time(nondet!)` |
| `TransportDec p c` | sched | `Unit` | `Fin p → Fin c → Nat → Nat` | a network edge (no `nondet!`) |
| `IntervalDec n` | sched | `Unit` | `TickScript n Nat` | `source_interval` (no `nondet!`) |
| `FixDec` | fuel | `UnfoldFuel` | `Unit` | a `forward_ref` cycle |

`Values` takes `Unit` at every network edge and timer because it encodes delay and reordering in the types of the received streams, and resolves them only at observation sites: a broadcast hands each receiver the senders' pools, keyed by sender, and how much of a pool has arrived by a tick is chosen by the `batch` or `snapshot` decision that observes it ([The Network at `Values`](./distributed-programs.mdx#the-network-at-values)).

Because the machine `SchedSem` takes the opposite choices, it connects the denotation to production. Every stream there is a flat, concrete sequence per member (a list that grows step by step; an in-tick batch is an `Array`), so the machine always has one actual order and one actual batching, as a deployed process does. Its observation sites therefore take `Unit` and do nothing: `assume_ordering u _d := u` returns the stream in the order it already has, and a tick's batch is whatever has arrived by then, determined by delivery and by the **pacing** (which members step, and when, a parameter that machine theorems quantify over). What it does take is the schedule: per sender-receiver pair a delivery cursor, the number of the sender's messages the receiver has received by each step, and per tick the fire count of each interval timer. The clock is the one family real in both, because a clock reading is both an observation and a fact about timing.

Because the machine's orders and batchings are not chosen to suit any proof, the denotational semantics has to cover them. The coupling instance `CoupleSem` derives the observation decisions of the denotational run from the machine run, as shown below, so a contract proved for every decision record applies to whatever the machine actually did. A denotational semantics that relied on a convenient order or batching could not be coupled to the machine. `SchedSem` is still a model, less concrete than the processes exchanging bytes ([Processes and the Compiled Step](../../internals/execution/processes.md)), but it is much closer to production than `Values`, and the rest of the chain to the bytes is equations.

The `Values` decision shapes carry domain names (`hydro_lean/Hydro/Decisions.lean`). A batch decision's shape depends on the grade of the stream it cuts:

```lean
@[reducible] def BatchCutsOf (n : Nat) (α : Type) : StrOrd → Retries → Type
  | .noOrder, .exactlyOnce => BatchCuts n α
  | .noOrder, .atLeastOnce => RetryBatchCuts n α
  | .totalOrder, .exactlyOnce => OrderedBatchCuts n
  | .totalOrder, .atLeastOnce => StutterBatchCuts n α
```

`BatchCuts n α` is `Fin n → List (Multiset α)`: per member, per tick, the multiset the tick consumes, a **cut**. An ordered exactly-once stream's cut is a slice length, and an ordered at-least-once stream's is the realized batch. An unordered at-least-once stream's cut is a set drawn from the pool, because a re-delivered element can appear in any later batch, so nothing is used up.

## The `dec` and `sched` Records

The three kinds go to two places in a program's signature. Nondets and fuels go in a `…Dec` record, or a bare binder when there is only one, like `collect_quorum`'s `dec`. Scheds go in a trailing `…Sched` record. `two_pc` (`hydro_lean/Examples/TwoPC.lean`) has both:

```lean
/-- The content decisions of one 2PC execution. -/
structure TwoPCDec (H : HydroSem L mem) (nC nPt : Nat) (P : Type)
    [DecidableEq P] where
  /-- Phase-1 `collect_quorum`'s `use::batch(…, nondet!(…))`. -/
  votes : H.BatchDec nC (P × Except Unit Unit) .noOrder .exactlyOnce
  /-- Phase-2 `collect_quorum`'s `use::batch(…, nondet!(…))`. -/
  acks : H.BatchDec nC (P × Except Unit Unit) .noOrder .exactlyOnce

/-- `two_pc`'s sched-det bundle: `Unit` at `Values`, delivery cursors at
the machine. -/
structure TwoPCSched (H : HydroSem L mem) (nC nPt : Nat) (P : Type)
    [DecidableEq P] where
  /-- `payloads.broadcast(&participants, …)`: the prepare transport. -/
  prepCh : H.TransportDec nPt nC
  /-- The vote echo's `send(coordinator)` transport. -/
  voteCh : H.TransportDec nC nPt
  /-- `c_all_vote_yes.broadcast(&participants, …)`: the commit transport. -/
  commitCh : H.TransportDec nPt nC
  /-- The ack echo's `send(coordinator)` transport. -/
  ackCh : H.TransportDec nC nPt
```

A contract (a module's `ensures` clause, see [Contracts](./contracts.md)) may mention the `dec` record, because its fields are real at `Values`: `CQEnsures` speaks of the consumed pool `cqConsumed (resp i) (dec i)`. A contract never mentions `sched`, whose fields are all `Unit` at `Values`; a consumer passes `TwoPCSched.triv` there, and machine theorems quantify over every schedule. A callee's decisions travel in the caller's record (`dec.votes` is the first `collect_quorum`'s batching), and a cycle's fuel travels in the record too, because the proofs about the cycle's fixpoint iteration consume it.

Each field's docstring names its Rust site. A decision corresponds to a site, not to a guard: `timeout`'s Rust takes one `nondet` guard parameter and forwards it to two `nondet!(nondet)` sites, and the Lean has two decisions, because the two sites choose independently:

```lean
structure TimeoutDec (H : HydroSem L mem) (n : Nat) (α : Type) where
  /-- `use::batch(self, nondet!(/** sampling timing … */ nondet))`. -/
  arrivals : H.BatchDec n α .noOrder .atLeastOnce
  /-- `use::time(nondet!(/** the verdict depends on when the tick runs */ nondet))`. -/
  clock : H.ClockDec n
```

In the other direction, two_pc.rs's `nondet!(/** TODO */)` at its two broadcasts has no decision. That guard is `nondet_membership`: it covers the cluster membership snapshot when members join asynchronously. The model sets membership at deploy time (the receivers are the type `Fin (mem p)`), so the operator is `broadcast_closed`, and what remains of the edge is the delivery cursor, which Rust never marks with `nondet!`.

## Total Decision Spaces

A `BatchCuts` value is any list of multisets, and most lists are not legal: a cut can ask for an element the pool lacks, or for more copies than remain. The obvious design defines legality and states contracts as `∀ d, Legal d → P (prog d)`. That premise must then be discharged in two places. A consumer must prove its own decisions legal before it can use a callee's contract. And the machine theorem, which obtains its decision from a machine run, must prove that decision legal, which is a semantic fact about the program around the site, not a fact about the decision.

The framework makes every value of the decision type yield a run. An illegal cut does not make the program undefined; it ends the run there. A run's batches form a **trace**, the list of per-tick values:

```lean
def batchCuts {α : Type _} [DecidableEq α] (pool : Multiset α)
    (consumed : Multiset α) : (d : List (Multiset α)) →
      Trace (Multiset α)
  | [] => []
  | b :: ds =>
    if consumed + b ≤ pool then b :: batchCuts pool (consumed + b) ds
    else []
```

`consumed` is what earlier ticks took. A tick is realized while what it takes fits in what remains of the pool; at the first cut that does not fit, the trace stops, and the later cuts are ignored even if they would fit. The `Values` `batch` at `.noOrder .exactlyOnce` is `.mk' fun i => batchCuts (s i) 0 (d i)`. Per-tick order selection follows the same pattern:

```lean
def realizeTicks {α : Type _} [DecidableEq α] :
    Trace (Multiset α) → List (List α) → Trace (List α)
  | p :: ps, s :: ss => if Multiset.ofList s = p then s :: realizeTicks ps ss else []
  | _, _ => []
```

A tick is realized only by a selection that is a permutation of its batch, so a realized tick is never a mis-ordered batch, and the first wrong selection cuts the trace. `retryBatchCuts` and `stutterBatchCuts` stop at the first illegal batch of their grades, and `latest_monotone` stops at the first read whose prefix does not ascend. In every case **legality is realizability**: a decision is as legal as the run it produces is long.

Because every decision yields a run:

- **Contracts carry no decision premise.** `collect_quorum.ensures` holds for every input and every `dec`. It stays meaningful at a truncated `dec` because it speaks of the realized run: `cqConsumed pool d` is `(batchCuts pool 0 d).sum`, what the legal prefix consumed.
- **A short run is a run.** A machine observed up to a finite step bound, its **horizon**, has realized finitely many ticks, and its derived decision is a finite list. Truncation is the same shape.
- **The machine theorem picks the decision the run realized.** The coupling instance derives the denotational run's batch cuts from the machine's tick segments, and the derived decision needs no proof of anything:

```lean
def batchDerive {n : Nat} {α : Type} [DecidableEq α]
    (p : Fin n → Nat → Bool)
    (T : Nat) (s : Fin n → StepHist α) : BatchCuts n α :=
  fun i => (batchesFrom ((s i).view) (tickSteps (p i) T) 0).map
    (fun b => Multiset.ofList b)
```

`cq_safe_sched'` in `hydro_lean/Hydro/Std/Quorum.lean` opens with `refine ⟨collect_quorum_vdec (Td := T) (pacing := pacing) () mn mx h, ?_⟩`, the generated derived decision built from `batchDerive`, and closes with `collect_quorum.ensures` at that decision. [Transferring to the Machine and the Network](../proving/transferring-to-the-machine.md) follows the argument.

The decision space of `Values` admits behaviour no machine produces: `assume_ordering` may select any permutation, while a machine realizes one order per schedule. There is therefore no theorem that every decision is realized by some schedule. Safety needs only the inclusion of the machine's behaviour in the behaviour of `Values`, and the surplus makes the contracts stronger, not weaker. A contract over decisions also says nothing about elements a decision never consumes; that a fair machine eventually consumes them is liveness ([Liveness](../../internals/liveness.md)).

## The Census

A port can get the decisions wrong without any proof noticing. Merging two Rust sites into one decision changes no proof and weakens every theorem, because a statement over one shared cut covers fewer behaviours than a statement over two independent ones. Adding a decision the Rust does not have models freedom the program does not have. The **census**, a `#nondet_census` line after each program, makes the count of each kind a build check:

```lean
#nondet_census two_pc (nondets := 2) (scheds := 4) (fuels := 0)
```

`#nondet_census M (nondets := a) (scheds := b) (fuels := c)` (`hydro_lean/Hydro/HydroDef.lean`) counts the **decision leaves** of `M`'s signature. It walks every binder of `M`, recursing through the fields of decision records, and takes as a leaf every binder or field whose type is one of the nine decision families. Each leaf is classified by the head constant of its type:

```lean
private def classifyHead : Name → Option DecKind
  | ``HydroSem.SnapDec | ``HydroSem.BatchDec
  | ``HydroSem.OrderSelDec | ``HydroSem.TickOrderSelDec
  | ``HydroSem.TicksDec | ``HydroSem.ClockDec =>
    some .nondet
  | ``HydroSem.TransportDec | ``HydroSem.IntervalDec => some .sched
  | ``HydroSem.FixDec => some .fuel
  | _ => none
```

The three counts are:

- **nondets**: the content decisions, one per site where an operator observes a particular history: a batch, a snapshot read, an order selection, a tick count of a bounded snapshot, a clock reading.
- **scheds**: the schedule sites, one `TransportDec` per network edge (every `broadcast_closed` and every `demux`, the Lean forms of Rust's `.broadcast`, `.send` and `.demux`) and one `IntervalDec` per interval timer (`source_interval`).
- **fuels**: one `FixDec` per cycle, that is, per wire of a `fix` block (Rust's `forward_ref`).

The counts are transitive. A callee's decisions are fields of the caller's records, so they are leaves of the caller: `paxos_core`'s 16/7/5 is `leader_election`'s 10/5/3 plus `sequence_payload`'s 6/2/0 plus the fuels of its own two knot wires. The build fails unless the three counts equal the stated ones, and the error lists every leaf with its access path. Here is the error for `toy_loop` (`Hydro/HydroGenToy.lean`), a cycle closed through a network hop around a call of the toy relay, checked against a count that forgets the hop:

```text
error: #nondet_census Hydro.toy_loop: found (nondets, scheds, fuels) = (2, (2, 1)), expected (2, (1, 1)); leaves:
  tdec.relay.batch : Hydro.HydroGen.Census.DecKind.nondet
  tdec.relay.gate : Hydro.HydroGen.Census.DecKind.nondet
  tdec.fuel : Hydro.HydroGen.Census.DecKind.fuel
  sched.relay.interval : Hydro.HydroGen.Census.DecKind.sched
  sched.net : Hydro.HydroGen.Census.DecKind.sched
```

The relay's two batch cuts and its timer appear under the callee's records (`tdec.relay`, `sched.relay`), the hop's delivery cursor is `sched.net`, and the cycle's fuel is `tdec.fuel`.

The census is not the Rust `nondet!` tally. Schedule sites and cycles carry no `nondet!` in the Rust: `.send`, `.demux` and `source_interval` take none, and a `forward_ref` takes none. In the other direction, the `nondet_membership` guard of a dynamic `.broadcast(…, nondet!(…))` has no decision, because the model's membership is set at deploy time (above), so the edge counts only as a sched. `two_pc` shows both differences. Its census is 2/4/0: two batch cuts (one per `collect_quorum` call), four network edges (two broadcasts, two sends), no cycle. The Rust has four `nondet!`s: the two membership guards in two_pc.rs and the `use::batch` in quorum.rs, once per call.

| module | nondets | scheds | fuels | what they are |
|---|---|---|---|---|
| `collect_quorum` | 1 | 0 | 0 | the `use::batch` |
| `timeout` | 2 | 0 | 0 | the `use::batch` and the `use::time` |
| `sample_every` | 1 | 1 | 0 | a batch and the interval timer |
| `two_pc` | 2 | 4 | 0 | two `collect_quorum` batches, four edges |
| `paxos_core` | 16 | 7 | 5 | `leader_election`, `sequence_payload`, two knot wires |

A census line follows every program in the repository, and changing one is a claim about how the Lean port corresponds to the Rust. The [fidelity](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/FIDELITY.md#the-census) page records the census of every mirrored module against its Rust.
