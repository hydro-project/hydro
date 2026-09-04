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
| timing decisions | `SampleTimes`/`TimerVerdicts`/`TimingPulses n := Fin n → …` (per member) | per-process timers | **S4** (and S3: F5) |
| per-pair FIFO + prefix delivery | `StepHist.deliver` + `cumMax` | TCP fail_stop TotalOrder per connection | **S4** |
| unbounded delay / silence / crash | cursor stalls; `pacing` all-false; outgoing cursors freeze | fail-stop, arbitrary latency | **S4** |
| partial / divergent fan-out | independent per-recipient cursors, item-granular | independent connections | **S4** |
| consume-all-at-tick | `batchesFrom` | handoff drain per tick | **S4** |
| empty/idle ticks; tick-counting ops | arbitrary pacing; `timeout_snapshot`/`source_interval_batch` take per-tick entries | event-driven ticks incl. timer-only wakes | **S4** |
| `+1` floors (network, knot) | `deliver` reads `view (t-1)`; `shift` at knots | handoffs and network are next-tick at soonest | **S4** |
| duplication sites | transport preserves grade; ALO born at sampling | TCP never retransmits | **S4** |
| static membership | `mem : L → Nat` constant | fixed at deploy | **S4** |
| in-tick unordered consumption | one tick's content is a plain `List` at the machine (`BoundedStream`, D60); the body runs the b-ops on it; every b-op is implemented twice and coupled (`CoBounded`) | `NoOrder` API exposes only order-blind ops | **S4** (§14) |
| fan-in decision-freedom | `values`/`union` take no decision; interleaving from delivery timing; canonical within-step order | arrival order; deterministic in-tick scheduling | **S2** (F4) |
| re-monotonization | `famFreeze` (freeze branch never fires for op-built flows — *unproven*) | n/a (Lean-internal) | **S2** (F3) |
| transport scope | prefix cursors only | `TCP.lossy`, UDP exist | **S1-scope** (F6) |
| input-coupling premise quantifiers | `hcp : ∀ T i, view T <+: cpV i`, `cpV` fixed first | clients send unboundedly | **S1** (F2) |

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
two horizons are the hook) or (b) a generic per-op causality lemma. *Status: OPEN by
ruling.*

### F3 (S2) — `famFreeze` transparency at knots is assumed, not proven

Knot outputs pass raw per-member diagonal views through `famFreeze`, a no-op iff the raw
family is per-member prefix-monotone — true for op-built bodies, proven nowhere. If the
freeze branch fired, the wire would silently stop growing (sound for ⊆-safety; coverage
would shrink and members would couple). Evidence: one positive probe (`TransferChecks`
§6). Fix shape: per-op "machine ops map prefix-monotone families to prefix-monotone
families" + "the Kleene diagonal of a monotone body is monotone". *Status: OPEN.*

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
Both are adversary generosity (safe direction). Same for `source_interval_batch`,
`sample_every`.

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

`TransferChecks §n` = the machine-checked existence witness.

1. **Per-member tick independence** — `pacing` per member; §9. (D35.)
2. **Decision-granularity sweep** — `TransportDec` per (receiver, sender) *and* per
   network edge (distinct fields per edge: two edges between the same pair are
   independent connections); timing families per member. No ambient parameter is coarser
   than its unit of concurrency.
3. **Per-pair FIFO prefix delivery = fail_stop TCP** — `deliver`+`cumMax`; §3. Cursor
   freedoms: unbounded delay (§2), silence forever, stall-at-`k`, partial per-tick
   emissions (item-granular), per-recipient divergence of a broadcast.
4. **Crash realism** — fail-stop = outgoing cursors freeze + pacing all-false; a
   mid-tick crash that flushed some connections = cursors split at different points of
   one tick's emission. No atomic multicast baked in.
5. **Consume-all-at-tick** — `batchesFrom` drops `consumed`, takes the rest; mid-tick
   arrivals go to the next tick = arrival-time refinement, covered by cursor freedom.
6. **Idle/empty and timer-only ticks** — arbitrary `pacing`; §4 shows stutter ticks are
   observable where they should be.
7. **`+1` floors exclude no real behavior** — handoffs and sends are next-tick at
   soonest; the model floors at one *step* (finer than a tick).
8. **Duplication only at sampling** — transport is grade-preserving; ALO born at
   `sample_every` (§5).
9. **Static membership.**
10. **`Trace.zip` truncation is benign** — every zip site pairs carriers of the same
    member on the same skeleton; no cross-skeleton skew exists to drop.
11. **Oblivious schedules cover adaptive adversaries for safety** — cursors/pacing/
    verdicts are functions of the step only; the machine is deterministic given the
    tuple, so any adaptive adversary's realized run *is* some tuple.
12. **Fan-in takes no decision anywhere** — `values`/`union` merge by increments; §1's
    cross-sender interleave witness; the canonical within-step order is F4.
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
    TODO */)` on faith (`paxos.rs:638` the P1b log fold, `:862` `reduce_watermark`'s
    equal-ballot tie) the model *demands* the proof (`FoldOk`/`comm`) — stricter in the
    safe direction; (c) no in-tick op reintroduces order (the pre-D60
    `assume_ordering_batch`, which the corner refused, was deleted in D67; an in-tick
    `assume_ordering` b-op with its own derived selection is the noted follow-up if a
    Rust site ever needs it).
15. **Acceptor log = `across_ticks` + keyed `reduce_watermark`** (D58 audit): modeled as
    accumulate-all-multiset + view-time keyed max (`logView`); insert-time vs view-time
    max are extensionally equal by commutativity; the equal-ballot tie Rust leaves as a
    `manual_proof!` TODO is made *checkable* (conflicts degrade to `none`, unreachable
    under the slot-functional contract). **Watermark GC is safety-neutral within
    `paxos_core`** — the proposer mirrors the watermark discipline, and
    `paxos_safe_sched'` carries no checkpoint premise; early GC is a *liveness* concern
    at whole-system scope (the replica feedback loop, unmodeled), to enter as a premise
    at that rung.
16. **The quorum `max` purge** — a key crossing `min` and `max` in one cut *is* emitted
    (`just_reached_quorum` is computed from `current_responses`; `#eval`-checked, D65).

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
- D20 `nondet_membership`: closed membership is a modeling assumption the Rust
  `nondet!` names explicitly; D22, D26 notes.

## Follow-ups, in order of theorem strength bought

1. **F2** per-horizon input coupling (or the causality lemma) — makes the headline
   apply verbatim to unbounded clients.
2. **F3** `famFreeze`-transparency theorem for op-built bodies.
3. **F6** the scope sentence wherever the theorem is advertised; extend the transport
   model only if/when a mirrored program needs lossy/UDP edges.

None of these change `Sched.lean`'s operational definitions: the machine's op semantics
survived the audit; the findings are about what the premises quantify over and about
unproven no-op beliefs at the model's totality armor.
