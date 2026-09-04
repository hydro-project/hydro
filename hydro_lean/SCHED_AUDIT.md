# SCHED_AUDIT — fidelity of the step machine to the Hydro runtime

**Subject**: `Hydro/Sched.lean` (`SchedSem`) and the statement-level premises of the
machine-run theorems (`paxos_safe_sched'`, `cq_safe_sched'`, `cq_live_chain`) that
quantify over its schedules.

**Question**: is the machine an accurate model of true distributed semantics — can every
behavior a deployed Hydro program can exhibit be represented by some `(pacing, schedule,
decision)` tuple, and do the theorems' premises quantify over all of those tuples? A
gap in either half is a hole in the headline, because `SchedSem`'s definition is part
of the trusted statement (`CORRESPONDENCE.md` §trust base).

**Why this ledger exists**: the per-member ticking gap (D35) was found incidentally.
Its moral — *an ambient parameter whose sharing scope is coarser than the unit of
concurrency is a smell* — is one axis below; the audit swept all of them plus a pass
for Lean-internal accidental holes (blocking branches, freeze branches, quantifier
placement). Names are the anchors; line numbers drift.

## Severity

| rank | meaning | consequence |
|---|---|---|
| **S1** | a real behavior is outside the theorem's quantifiers, visibly | needs a fix or an explicit scope condition |
| **S2** | unreachable in the model, with an argument that no proven property depends on it | document; optional hardening |
| **S3** | the model admits **more** than reality | safe direction for safety; document |
| **S4** | checked faithful | recorded so the next auditor doesn't redo it |

## Ground truth: what the Rust runtime does

- **Ticks are per-process local logical time** — `dfir_rs/src/scheduled/ticks.rs`
  module doc: "Each iteration of a process loop is called a tick … the 'local logical
  time' at the process." No cross-process tick synchronization exists.
- **Ticks are event-driven; empty ticks happen; mid-tick arrivals wait** —
  `dfir_rs/src/scheduled/context.rs`: `run` loops `run_available` then sleeps until a
  waker fires; `run_available` "Always run at least one tick"; arrivals during a tick
  are processed in the next (`run_tick`). Timer wakes are real ticks with no data.
- **A tick drains what is buffered at tick start** (handoff semantics). "Partial
  batches" arise only from arrival timing — `stream/mod.rs::batch`: "batches are
  guaranteed to be contiguous across ticks and preserve the order of the input … batch
  boundaries are non-deterministic."
- **Transports** — `hydro_lang/src/networking/mod.rs`: `TCP.fail_stop()` gives
  `TotalOrder` per connection and stops sending after a failed connection; `TCP.lossy()`
  may drop a message and continue; `lossy_delayed_forever` re-grades to `NoOrder`; UDP
  is `NoOrder`, connectionless. Fail-stop connections never retransmit, so **TCP never
  duplicates**; duplication is born at application-level re-sampling (`sample_every`).
- **The mirrored programs use fail-stop everywhere** (`paxos.rs:316, 442, 521, 748,
  889`; `hydro_std` likewise).
- **Cluster membership is fixed per deployment** (`mem ℓ` constant).
- **Cross-tick state** is either a user cycle (`use::state` in `sliced!` =
  `cycle_with_initial`, `sliced/style.rs:114–127`) or operator state
  (`across_ticks` + `reduce_watermark`, `paxos.rs:851–864`); there is no `persist()`
  (the comment at `paxos.rs:823–825` is stale — upstream report list below).
- **Monotonic singletons exist** (`live_collections/singleton.rs:39–45`,
  `properties/mod.rs:532–541`): a fold with a `monotone(…)`-proven combiner lands in the
  `Monotonic` bound — mirrored by `SingBound.monotonic`/`fold_monotone` (D67).
- **In-tick `NoOrder` consumption**: `Stream<_, Tick, Bounded, NoOrder>` exposes only
  order-blind operators (elementwise `map`/`filter`, `count`, `fold` with
  `commutative = …`, keyed ops); `enumerate`/`first` require `TotalOrder`.

"True distributed semantics" for a deployed program: per-process event-driven local
ticks; per-connection FIFO prefix delivery with fail-stop; independent per-recipient
fan-out; timers on wall-clock; all inter-process influence mediated by messages.

## Axis-by-axis verdicts

| axis | model artifact | runtime ground truth | verdict |
|---|---|---|---|
| tick skeleton granularity | `pacing : (ℓ) → Fin (mem ℓ) → Nat → Bool` (per member) | per-process local clocks | **S4** |
| delivery cursor granularity | `TransportDec p c := Fin p → Fin c → Nat → Nat`, one field per network edge in each module's `…Sched` bundle (`LESched.p1aCh`, `SPSched.p2aCh`, …), nested along the call structure into `PaxosCoreSched.le`/`.sp` | one TCP connection per (edge, sender, receiver) | **S4** |
| emission order | none — a `tick` block computes each tick's emissions on the machine's lists; the corner couples them to the quotient per operator (D61) | per-member hashmap iteration order | **S4** |
| timing decisions | at the machine, total per-tick scripts `TickScript n Nat := Fin n → Nat → Nat` (`ClockDec` readings; `IntervalDec` fire counts, D91) indexed by the pacing's tick count (D84: every tick has its verdict/reading; the denotation's `ClockReadings` are derived at the horizon; an interval timer's denotation is the constant class `[()]` (D91)); a process reads its clock and timers once per activation (`Dag.Timing`) | per-process timers, `Instant::now()` per tick | **S4** (and S3: F5) |
| per-pair FIFO + prefix delivery | `StepHist.deliver` + `cumMax` | TCP fail_stop TotalOrder per connection | **S4** |
| unbounded delay / silence / crash | cursor stalls; `pacing` all-false; outgoing cursors freeze | fail-stop, arbitrary latency | **S4** |
| partial / divergent fan-out | independent per-recipient cursors, item-granular | independent connections | **S4** |
| consume-all-at-tick | `batchesFrom` | handoff drain per tick | **S4** |
| empty/idle ticks; tick-counting ops | arbitrary pacing; `time` takes per-tick entries; `source_interval`'s fires arrive at the member's ticks (`IntervalDec` script, D91) | event-driven ticks incl. timer-only wakes | **S4** |
| `+1` floors (network, `defer_tick`) | `deliver` reads `view (t-1)`; `defer_tick` releases what was pushed by `step - 1`; a knot wire is its body's same-step output (D95) | handoffs and network are next-tick at soonest; a `forward_ref` acyclic through strata is a same-tick alias | **S4** |
| duplication sites | transport preserves grade; ALO born at sampling | TCP never retransmits | **S4** |
| static membership | `mem : L → Nat` constant | fixed at deploy | **S4** |
| in-tick unordered consumption | one tick's content is a plain `Array` at the machine (`BoundedStream`, D60 a `List`, D98 an `Array`); the body runs the b-ops on it; every b-op is implemented twice and coupled (`CoBounded`) | `NoOrder` API exposes only order-blind ops | **S4** (§14) |
| keyed content | `KeyedStream` is grouped at `Values` (`KeyedPool`: finite key set, per-key graded content — no cross-key order at any grade) and flat pairs at the machine; `into_keyed` is the identity there, as Rust's `Cast`; the network's keyed output is grouped per sender, `values` merges keys (D70) | `KeyedStream<K, V, …>` = a stream of pairs with per-key ordering guarantees; `broadcast` yields `KeyedStream<MemberId, V>` | **S4** |
| keyed singletons | `KeyedMap` at `Values`, a nodup assoc list (`SchedKMap`) at the machine — the `HashMap` unique-keys invariant, not a `HashMap`; `.boundedValue` reads are increment cuts of the SOURCE pairs with per-key-prefix legality (`legalView`), sound for keyed streams from `kassume_ordering` and from transport (D70) | `KeyedSingleton<K, V, L, BoundedValue>`; snapshot reads of a live map | **S4** (a `.boundedValue` keyed-singleton *batch* op is not modeled — see the "keyed-singleton batch op" item queued in FINDINGS D70) |
| `reduce_watermark` | machine: DFIR's `ReduceKeyedWatermark` fold literally (`reduceWatermarkEager`: payloads then watermark per tick, drop below, insert-or-reduce, eager `retain`); denotation: accumulate + GC-at-read (`reduceWatermarkStep`); `scan_eager_eq_assoc` proves the two output-equal (D70) | `compile/ir/mod.rs:5221–5310`, hydro-project/hydro#3189 | **S4** |
| fan-in decision-freedom | `values`/`union` take no decision; interleaving from delivery timing; canonical within-step order | arrival order; deterministic in-tick scheduling | **S2** (F4) |
| re-monotonization | `famFreeze` (freeze branch never fires for op-built flows — *unproven*) | n/a (Lean-internal) | **S2** (F3) |
| transport scope | prefix cursors only | `TCP.lossy`, UDP exist | **S1-scope** (F6) |
| input-coupling premise quantifiers | `hcp : ∀ T i, view T <+: cpV i`, `cpV` fixed first (`paxos_safe_sched'`); none in `paxos_safe_sched` | clients send unboundedly | closed for Paxos (F2) |

## Findings

### F1 — CLOSED (historical). The square-era `hsat` premise

The first machine-safety headline took a satisfiability premise `hsat : δ T d` that
turned out to be unsatisfiable for nested knots (D37); the design was dissolved by the
coupling corner (D39–D42). `paxos_safe_sched'` has no such premise. Kept as the origin
of `DOCTRINE.md` R7; details in `CORRESPONDENCE.md` §cautionary.

### F2 (S1) — the input-coupling premises exclude unbounded inputs

`paxos_safe_sched'` takes a **finite** abstract pool `cpV` per member and
`hcp : ∀ T i, (cpS i).view T <+: cpV i` with `cpV` bound before `∀ T`. A machine input
that grows without bound admits no such `cpV`, so those runs are outside the quantifiers
even though each finite prefix is well-behaved. Probably harmless — the conclusion at
horizon `T` depends only on views ≤ `T` — but **machine causality is not a theorem in
the tree**, so the reduction is informal. Fix shapes: (a) per-horizon premise
(`∀ i, Multiset.ofList ((cpS i).view T) ≤ cpV i` at the theorem's own `T`; the corner's
two horizons are the hook) or (b) a generic per-op causality lemma. *Status: CLOSED for
the Paxos headlines (D75): `paxos_safe_sched` (`Paxos/CoupleWf.lean`) takes the
machine inputs alone — any client stream, any checkpoint register that only grows — and
builds the corner's pools inside the proof from the inputs truncated at the horizon,
carrying the commits back by causality through the graph (`paxos_core.dag_denote` +
`Dag.denote_causal`, shape (b)). `paxos_safe_sched'` is kept as the corner-facing form;
`cq_safe_sched'`/`two_pc_*` still carry their pools (same recipe applies).*

### F3 (S2) — `famFreeze` transparency at knots is assumed, not proven

Knot outputs pass raw per-member diagonal views through `famFreeze`, a no-op iff the raw
family is per-member prefix-monotone — true for op-built bodies, proven nowhere. If the
freeze branch fired, the wire would silently stop growing (sound for ⊆-safety; coverage
would shrink and members would couple). Fix shape: per-op "machine ops map prefix-monotone families to prefix-monotone
families" + "the Kleene diagonal of a monotone body is monotone". *Status: OPEN.*

### F2b (closed, D77) — the schedule was a parameter of the graph

Until D77 the compiled graph baked the program's `…Sched`/`…Dec` bundles into its
network and timer nodes, so `paxos_safe_wired` took `ssched`/`sdec` and the processes'
oracle was pinned to the baked cursors — coverage held only through "for every run
there exists a reproducing bundle". Now `Dag.Sched d` (per edge the cursors, per timer
the verdicts) is execution data: an argument of `Dag.denote`/`Dag.run`, a field of
`Interleaving`, and the generated `M.recordOf` with `M.schedOf_recordOf` (kernel `rfl`)
shows every schedule of the graph is some bundles'. The headline quantifies over the
execution's deliveries directly.

### F4 (S2) — canonical within-step fan-in order

`mergeN` appends senders' same-step increments in `finRange` order; `assume_ordering`
is the identity on the machine. Real executions can realize other in-tick production
interleavings. These are invisible to every `NoOrder` consumer and visible only through
`assume_ordering`, whose `Values` semantics quantifies over **all** legal selections —
so every safety fact transported from `Values` holds for the unrealized orders too. The
WLOG is load-bearing only at `assume_ordering`, and every such site in the mirrored
programs has its obligations discharged at `Values` over all selections. *Status:
documented; hardening queued.*

### F5 (S3) — timing decisions are freer than reality

`timeout_snapshot` verdicts are `(d i).take #ticks`, uncorrelated with delivery; a
decision may supply fewer verdicts than ticks (a timer that stops being consulted).
Both are adversary generosity (safe direction). Same for `source_interval` (D91: its
fire-count script is a sched-det; the denotation is the one class `[()]`, so no script
can exceed it) and `sample_every`.

### F6 (S1-scope) — the transport model is fail-stop only

The cursor model is exactly `TCP.fail_stop`: per-connection FIFO, prefix delivery,
silence-forever expressible. It does **not** model `TCP.lossy` (drop then continue — a
non-prefix subsequence) or `lossy_delayed_forever`/UDP (arbitrary reordering within a
pair). This is fine for the mirrored programs (all edges fail-stop), but any prose of
the form "safety under any schedule" carries the scope condition **"for programs whose
channels are all `TCP.fail_stop`"**. Lossy edges need skipping cursors (a `Sched`
extension with its own safety re-check) before any program that uses them is mirrored
(`LIVENESS.md` rung b′).

## The S4 ledger — checked and found faithful

Coverage of the deployment's behaviours is a theorem since D90/D95 (`WireTrace.streamOutputsAt_eq`:
every byte-level trace of the executable is some machine run; `paxos_safe_wired` quantifies
over all of them), so the items below are audited against the Rust runtime by reading. The
existence witnesses live at the top-level model since D100 (`WireTraceChecks.lean`: traces
and lock-step interleavings over the toy graphs, `#guard`-enforced) — they show the trusted
statement surface admits each behaviour, not that `SchedSem` does.

1. **Per-member tick independence** — `pacing` per member; a `WireTrace`'s `who`. (D35.)
2. **Decision-granularity sweep** — `TransportDec` per (receiver, sender) *and* per
   network edge (distinct fields per edge: two edges between the same pair are
   independent connections); timing families per member. No ambient parameter is coarser
   than its unit of concurrency.
3. **Per-pair FIFO prefix delivery = fail_stop TCP** — `deliver`+`cumMax`. Cursor
   freedoms: unbounded delay, silence forever, stall-at-`k`, partial per-tick
   emissions (item-granular), per-recipient divergence of a broadcast.
4. **Crash realism** — fail-stop = outgoing cursors freeze + pacing all-false; a
   mid-tick crash that flushed some connections = cursors split at different points of
   one tick's emission. No atomic multicast baked in.
5. **Consume-all-at-tick** — `batchesFrom` drops `consumed`, takes the rest; mid-tick
   arrivals go to the next tick = arrival-time refinement, covered by cursor freedom.
6. **Idle/empty and timer-only ticks** — arbitrary `pacing`; at the bytes an activation
   with nothing to read is an empty tick (D93/D95).
7. **`+1` floors exclude no real behavior** — handoffs and sends are next-tick at
   soonest; the model floors at one *step* (finer than a tick). The knot has no floor
   (D95): the wire is its body's output of the same step, as DFIR's `forward_ref`; the
   generated `K_strict₁`/`M.dag_valid` prove every cycle passes a hop or a `defer_tick`.
8. **Duplication only at sampling** — transport is grade-preserving; ALO born at
   `sample_every`.
9. **Static membership.**
10. **`Trace.zip` truncation is benign** — every zip site pairs carriers of the same
    member on the same skeleton; no cross-skeleton skew exists to drop.
11. **Oblivious schedules cover adaptive adversaries for safety** — cursors/pacing/
    verdicts are functions of the step only; the machine is deterministic given the
    tuple, so any adaptive adversary's realized run *is* some tuple.
12. **Fan-in takes no decision anywhere** — `values`/`union` merge by increments; the
    canonical within-step order is F4.
13. **Unordered emissions take no decision** — since D61 a program's per-tick
    emissions are its `tick` block run on the machine's own lists (quorum.rs line for
    line); the corner proves them coupled to the denotation's multisets per operator
    (`CoTick.body_comm`/`scan_comm`). The former `EmitDec`/`emitLin` linearization
    claims are gone (D60–D61).
14. **In-tick consumption of unordered batches = Rust's `NoOrder` API surface** (D60).
    The machine never erases order in its state; the erasure is in the body's *view*:
    the in-tick operators are implemented twice (list / quotient) and coupled per
    operator, so a body cannot observe the schedule's order through any operator the
    `NoOrder` grade exposes — exactly what `Stream<_, Tick, Bounded, NoOrder>` enforces.
    **Dependencies stated**: (a) hydro_lang's `NoOrder` API leaks no order (a claim about
    the Rust library surface); (b) where Rust accepts `commutative = manual_proof!(/**
    TODO */)` on faith the model *demands* the proof — stricter in the safe direction:
    `paxos.rs:862` (`reduce_watermark`'s equal-ballot tie) is `FoldOk … LogTie
    logMerge` with the content half (`P2aFunctional`) proven by `ghost pre` from
    `acceptor_p2`'s `requires` (D71, R3); `paxos.rs:638` (the recommit `(count,
    champion)` fold) turned out NOT to be commutative — the Rust now says so
    (`.assume_ordering::<TotalOrder>(nondet!(…))` before the fold, the `commutative`
    claim removed) and the model runs the literal closure on a per-tick order
    decision (`tassume_ordering`, item 17); (c) no in-tick op reintroduces order
    except `tassume_ordering` on the tick-located wire, whose selection the corner
    derives from the machine's arrival order (`tkOrdSelDerive`) — the body's view of
    the order is the schedule's, never a free choice (D71).
15. **Acceptor log = `across_ticks` + keyed `reduce_watermark`** (D58 audit, redone
    D70–D71): the machine is DFIR's `ReduceKeyedWatermark` fold literally (`logMerge`
    in arrival order, keys below the watermark collected); the denotation is the
    permutation-invariant per-key reduce under the tie prerequisite `LogTie`; the
    equal-ballot tie Rust leaves as a `manual_proof!` TODO is `acceptor_p2`'s
    `requires` (`P2aFunctional`), discharged by `sequence_payload`'s once-per-key
    sending. **Watermark GC is modeled** (`KeyedMap.gcBelow`), and the checkpoint input
    is typed `.monotonic`: the running watermark IS the published checkpoint, so a
    collected slot sits below the view's own checkpoint — what the proposer's recommit
    discipline (skip ≤ checkpoint, holes and fresh slots above it, `rebaseAboveCheckpoint`
    under `.guarded`) consumes. The `.faithful` rebase from the view's max slot below
    the checkpoint is B4.
17. **The recommit fold's order** (D71): paxos.rs:617–640's `(count, champion)` closure
    is order-dependent (one slot, f = 1, X = (1, v), Z = (2, w), Y = (3, v): X·Y·Z skips
    the slot, X·Z·Y recommits it — same multiset). The order is a per-tick content
    decision (`RCDec.order : TickOrderSelDec`, Rust's new `assume_ordering` `nondet!`);
    the proof holds for every order (safety: `rcFold_champ` — the champion is a view
    entry dominating all ballots and the count never exceeds its value's holders;
    `RCTick.skipped_holders`/`value_best`), the machine's order is the arrival order
    and the corner derives the denotation's selection from it. Only efficiency (a
    redundant recommit) is order-dependent. The pre-D71 model (`logView` + a
    commutative count) was an "honest refinement" that HID the divergence.
16. **The quorum `max` purge** — a key crossing `min` and `max` in one cut *is* emitted
    (`just_reached_quorum` is computed from `current_responses`; `#eval`-checked, D65).
18. **`defer_tick` is DFIR's FIFO** (D85): `dfir_lang/src/graph/ops/defer_tick.rs:6`
    "buffers all input items and releases them at the next time boundary"; on a
    `Singleton<Tick>` that is one buffered value released at the next tick. The
    machine's `defer_tick init t = fun i step => (init :: t i (step - 1)).take (deferReleased …)`
    is that step — a tick releases the buffered entry if there is one, from the buffer as of
    the previous step (release before push, D95: what the input pushes at a tick is released
    at the next one, so the operator is strict in its input) — and the production cell is
    the buffer itself (`[init]`; `s ++ Δ.toList`; a tick emits the head). On inputs at
    most one tick behind the tick count (every Rust-expressible wire) the released
    count is the tick count (`deferReleased_eq_ticks`), so the slot wire reads the
    previous tick's value. The previous machine definition released the current
    tick's write in the same step; the `take ticks` form that would have fixed only
    the machine disagrees with a one-slot FIFO on Sched-admissible inputs more than
    one tick behind, and the model must capture every production run with no side
    condition (R13) — hence the step form. `toy_defer_next_tick` (`HydroTickCheck`)
    runs both on one input. **Single slots**: an atomic wire's production increment
    is an `Option` (one value per tick); the fact that makes this lossless, `TPaced`
    (≤ 1 entry per step, none when idle), is proved per operator (`SchedPaced.lean`)
    and is a hypothesis of `Dag.run_spec`, discharged once for every interleaving
    (`Interleaving.wires_paced`; `Interleaving.outputs` takes no hypotheses). The `sample_every` oracle is a `Bool` per tick.

## Upstream report list (open, for the Rust tree)

- **B1** `paxos.rs:311–317`: P1a re-sent every tick of a ballot; the faithful variant
  commits two values at one slot (`lake exe falsify`). Guard: send each ballot's P1a
  once (`OnceInv` register).
- **B2** `paxos.rs:782–804` × `:678–734`: the per-tick rebase lets two fresh payloads on
  two reign ticks both be indexed at max+1 → two values at one slot (`lake exe
  falsify`, module and whole-program scope). Guard: the recommit gate (`SequencePayload`).
- **B3** `quorum.rs` `collect_quorum_with_response` at `min < max`: late stragglers of
  already-emitted keys are batching-dependent (`FINDINGS.md` B3); the contract makes the
  caveat visible as the usage cap `cqKeyCount … ≤ max` on `emit_count`.
- `paxos.rs:823–825`: the `nondet!` doc comment says "we use `persist()`"; no such
  operator exists — the code is `across_ticks` + `reduce_watermark`.
- `quorum.rs` `max` purge: if **more** than `max` responses for a key arrive (only via
  upstream duplicates — B1), the key is re-emitted (cuts `{7×3},{7×2}` at `(2,3)` emit
  `{7,7}`); a doc comment on the exactly-once assumption would help.
- `paxos.rs:386–390`: `p_has_largest_ballot` is provably identically `true` (D26).
- **B4** `paxos.rs:655` × `:788`: with GC, `p_max_slot` is the champions' max slot,
  which can sit BELOW the view checkpoint; `index_payloads` rebases fresh payloads from
  it, re-proposing at a collected (executed) slot (`lake exe falsify`, B4). Guard:
  rebase from `max(max_slot, checkpoint)` (`rcMaxSlot`, `.guarded`).
- `paxos.rs:617–640` (**landed in this tree**, D71): the recommit fold is not
  commutative; the `commutative = manual_proof!(/** TODO */)` is replaced by
  `.into_keyed().assume_ordering::<TotalOrder>(nondet!(/** X·Y·Z + safety argument */))`
  before the `.fold(…)`; the `paxos_ir` snapshots gained the `ObserveNonDet` node.
- `paxos.rs:862` (`reduce_watermark`'s `manual_proof!(/** max by ballot (TODO: not if
  two entries with same ballot, need assume) */)`): the needed assume is "the P2a pool
  is `(slot, ballot) ↦ value` functional"; a `manual_proof!(requires: …)` form would
  let the library demand it (D70/D71 suggestion).
- `optional.rs`: `Optional` has no `Monotonic` bound, so `acceptor_p2`'s
  `a_checkpoint: Optional<usize, …, Unbounded>` cannot say what the protocol relies on
  (the checkpoint only grows); the Lean types it `.monotonic` and the safety proof uses
  it. Upstream: strengthen the bound (D71).
- **B5** (D95, liveness) `kv_replica/mod.rs` `r_checkpoint_seq_new = next_slot` ×
  `paxos.rs:673` (`slot <= checkpoint → None`) and `:687` (`(checkpoint + 1)..max_slot`):
  the replica's checkpoint `C` is the first UNEXECUTED slot (and `reduce_watermark`,
  `keyed_stream/mod.rs:2135`, collects keys `< C`), but the recommit treats slot `C` as
  executed — a re-election after a checkpoint never recommits an in-flight slot `C` (its
  log entries fail `slot <= checkpoint`) and fills no hole there, so the replicas wait for
  `C` forever (latent in the benchmark, which never re-elects). Guard: `slot < checkpoint`,
  holes from `checkpoint`. The Lean's B4 lift inherited the convention; D95's same-tick
  knot let the replicas' startup checkpoint `0` race the P1a and trip its `C = 0` instance
  (the leader based its first payloads at slot 1). Guarded in the Lean (`.guarded`,
  `PaxosVariant.checkpointIsNextSlot`): the recommit drops `slot < C` (`rcSettled`), fills
  holes from `C` (`rcHoleStart`), and lifts the max slot to `lastSettled C` so fresh
  payloads start at `max (max_slot + 1) C`; `falsify`'s B5 scenario shows the faithful
  variant losing the in-flight slot `C = 1` and the guarded one recommitting it.
- **B6** (liveness) `paxos.rs:666–668` (`if count > f { return None; }` in
  `recommit_after_leader_election`): the new leader skips a slot when more than `f`
  acceptors in its P1b quorum hold the champion's value, on the assumption that the value
  is chosen and the replicas have it. Neither follows. (i) The count is over holders at
  ANY ballot: f = 1, acceptors X, Y, Z; P1@1 gets A accepted at X only; P2@2 (quorum
  {Y, Z}, sees nothing) gets B accepted at Y only; P1@3 (quorum {X, Z}) recommits A,
  accepted at Z only; P2@4 (quorum {X, Z}) sees X:(1, A), Z:(3, A) — two holders, two
  ballots, no single-ballot quorum, A NOT chosen — and skips; P1@5 (quorum {X, Y}) sees
  Y:(2, B) as the max ballot and recommits B. Safety holds (the skip sends nothing;
  `p_to_replicas` is gated by the `(slot, ballot)` P2b quorum, `paxos.rs:777–790`,
  `:906–916`, so A is never delivered; the Lean headline uses only `RCTick.value_best`),
  but slot `s` is a hole for P2's whole tenure and the replicas apply slots contiguously
  (`kv_replica/sequence_payloads.rs:32–38`): every slot P2 commits above `s` stalls until a
  later leader recommits `s`. (ii) Even when the value WAS chosen (two holders at one
  ballot), the previous leader may have crashed between receiving the P2b quorum and
  forwarding to the replicas; the skip then makes the hole permanent under the new leader.
  The `nondet!` comment's former justification ("already chosen, by quorum intersection")
  was false and is corrected in this tree; the `skipped_holders` docstring likewise.
  Guard: drop the skip — recommit the champion unconditionally (safe by `value_best`,
  idempotent at the acceptors, and it refreshes delivery). Scenario from a review of the
  D71 finding by the Paxos code's author. **Queued for the parent** (not fixed here): the
  guard in `paxos.rs` and the Lean mirror, a `falsify` B6 scenario showing the replicas'
  `r_next_slot` stuck at `s` under the ballot-4 leader in the faithful variant, and the
  `RCTick` face reshaped to the unconditional recommit.
- D20 `nondet_membership`: closed membership is a modeling assumption the Rust
  `nondet!` names explicitly; D22, D26 notes.
- `paxos_bench.rs:86–88` (D82): the acceptor checkpoint gate is `key_count() == f + 1`.
  With MORE than `f + 1` replicas it closes forever once all have reported (liveness), and
  before that a late replica's lower checkpoint could lower the minimum (the acceptors'
  `Monotonic` read of the checkpoint would be false). The Lean `paxos_protocol` `requires
  mem replicas = f + 1`; `acceptor_checkpoint_from_replicas.ensures` proves the ascent
  under it. Upstream: `>=` with a monotone guard, or document the exact-count assumption.
- `paxos_bench.rs:69–96` × `paxos.rs` `a_checkpoint: Optional<usize, …, Unbounded>` (D82):
  the checkpoint `Optional` has no `Monotonic` bound while `paxos_core` relies on it; the
  Lean knot invariant is exactly the missing bound's content (`AscendingLatest`).
- `kv_replica/mod.rs:34–44` (D82): `impl Ord for SequencedKv` compares `seq` only, so it is
  not antisymmetric against the derived `Eq`; `.sort()` on it is well-defined only because
  the Paxos log is slot-functional. The Lean order is lexicographic (`seq`, then the payload).
- `bench_client/mod.rs:100–128` (**landed in this tree**, D81 ruling C1): `Instant::now()`
  inside a `fold` closure and inside a `map` were wall-clock reads invisible to the types;
  replaced by `use::time(nondet!(…))` (new `sliced!` style over `Tick::current_tick_instant`,
  the slice half of GH #2393) in one `sliced!` block; the overwrite-`reduce`'s `commutative =
  manual_proof!` by `assume_ordering::<TotalOrder>(nondet!(/** one in flight per virtual
  client */))`.

## Follow-ups, in order of theorem strength bought

1. **F2** — done for Paxos (`paxos_safe_sched`); apply the same recipe to
   `cq_safe_sched'` and `two_pc_safe_sched'`.
2. **F3** `famFreeze`-transparency theorem for op-built bodies.
3. **F6** the scope sentence wherever the theorem is advertised; extend the transport
   model only if/when a mirrored program needs lossy/UDP edges.

4. **`SchedSem` against production DFIR** — `Project` (D74) is the vehicle: the
   `hydro def` body is now also a typed dataflow graph with a per-process step
   (`Dag.locStep`) proven equal to the machine's (`Dag.step_wired`), and `lake exe wired`
   runs `paxos_core` as five TCP processes. What the audit still owes is the *cross*
   check — record a production DFIR run's deliveries and pacing, replay them as the
   graph's oracle (`Dag.Oracle`: one cursor per hosted receiver per sender) and ticks,
   compare per-tick outputs. The execution model surfaced one fact about the
   semantics a production comparison must respect, and D76 made the semantics say it:
   a member that is not active is inert — delivery and `defer_tick` are gated by the
   member's activity (a knot wire is its body's same-step output, D95) (`Pace`: idle / stutter / tick; tick bodies
   by its ticks), so a process that is not running is exactly an idle member, and
   every run from `Dag.init` is an interleaving with no settling or prefix condition.
   One DFIR tick is one tick in the model (D95; D93 had "a tick, then settling passes"
   and D76 the reverse order): a forward_ref inside a tick is a same-tick alias in DFIR
   (stratum order), and the model's knot wire is its body's output of the same step —
   no knot shift, no stutter the tick needs, no
   tick production does not trigger. Audited against DFIR (D76): one tick closure per
   process, all `Tick` locations on one counter, intra-tick cycles rejected,
   `DeferTick` → `defer_tick_lazy`, sources polled inside the tick, sinks flushed by a
   task after it, `current_thread` + `LocalSet`, the closure never awaits — every model
   assumption is exact or a superset of the runtime (a process is one cluster member,
   `Member`). **The one load-bearing assumption is
   the `+1` floor**: an executor that let reader tasks run mid-tick (multi-threaded, or a
   tick that awaits) could produce a same-tick round trip the model excludes. Re-check if
   DFIR's tick closure ever gains an await.

5. **Idle ticks at a process (D92, resolved D93).** DFIR runs a tick at start, on a
   socket or timer wake, and for a non-lazy `defer_tick` — which Hydro never emits
   (`compile/ir/mod.rs:4819` → `defer_tick_lazy`; `dfir_rs/src/scheduled/context.rs`,
   `meta_graph.rs`): a production process is idle between arrivals and a buffered item
   waits for the next wake. The model allows that pacing (an idle member is inert, D76).
   The harnesses follow it: an activation is `Dag.iterate` — the inbox, then one sweep of
   the graph in dependency order, a `fix … complete` block swept until its knots are known
   (`knotLoop`, ≤ |ks|+1 sweeps of the body, the model of DFIR's strata; D95) — and a
   process waits unless bytes arrived or a timer is due. Under D93's one-step knot floor
   the activation needed settling passes (stutters while a knot slot held feedback, a
   further tick while a tick input held what they put there), which were not DFIR's tick
   counts; with the knot a same-step binding the activation is one pass and DFIR's. The
   `WireTrace` headline ranges over DFIR's own schedules (`WireTraceCheck.lazy`). Nothing
   to report upstream; the semantics are not at issue.

None of these change `Sched.lean`'s operational definitions: the machine's op semantics
survived the audit; the findings are about what the premises quantify over and about
unproven no-op beliefs at the model's totality armor.
