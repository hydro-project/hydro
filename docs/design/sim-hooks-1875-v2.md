# Simulator hooks: scripting the decisions of unsafe operators (v2)

Issue: [#1875](https://github.com/hydro-project/hydro/issues/1875)

This is a design proposal for letting tests take manual control of the non-deterministic
decisions in a Hydro program — when a batch fires and what it contains, which snapshot of
a piece of state a request observes, and so on — so that developers can write
deterministic, readable tests for *specific* distributed-systems scenarios, alongside the
existing fuzzing and exhaustive-search modes of the simulator.

The document is organized so that each section can be read mostly on its own. Sections
1–2 explain what we are trying to achieve and what we care about. Sections 3–4 describe
the API a developer sees. Sections 5–6 define how the simulator executes a scripted test.
Section 7 sketches the implementation, and sections 8–9 cover scope and open questions.
Section 10 is a discussion section: for every part of the design where we considered and
rejected alternatives — or where a simple-looking rule has a subtle justification — there
is a long-form writeup of the reasoning. The main body (sections 3–6) describes only the
final design; if a rule seems arbitrary, the discussion section explains where it came
from.

## 1. Motivation

Hydro programs are deterministic by default. The only places where non-determinism can
enter a program are its **unsafe operators** — operators like `batch` (which splits an
input stream into arbitrarily-sized chunks), `snapshot` (which observes an
arbitrarily-stale version of evolving state), and `assume_ordering` (which declares that
an unordered stream may be consumed in some arbitrary order). Every one of these
operators requires the developer to pass a `NonDet` guard created with `nondet!(...)`,
documenting why the non-determinism is acceptable.

Because these operators are the *only* sources of non-determinism, the simulator can
explore every meaningfully different execution of a program by varying just their
behavior. Today, it does so **autonomously**: in `fuzz` mode a fuzzer picks batch
boundaries, snapshot versions, and orderings; in `exhaustive` mode every combination is
enumerated. This is excellent for finding bugs. But it cannot express a test like:

> "Two increments are sent to a counter. A read request arrives *between* them, and the
> service reveals the state after the first increment. The response must therefore be 1."

To make this concrete, here is a small counter service. It has two unsafe operators: a
`batch` that groups incoming read requests, and a `snapshot` that picks which version of
the count each batch of reads observes.

```rust
pub fn counter_service<'a>(
    increments: Stream<u64, Process<'a, Counter>>,
    get_requests: Stream<u32, Process<'a, Counter>>,
    nondet_batch: NonDet,    // how get requests are batched
    nondet_snapshot: NonDet, // which version of the count each batch observes
) -> Stream<(u32, u64), Process<'a, Counter>, Unbounded, NoOrder> {
    let current_count = increments.fold(q!(|| 0), q!(|acc, v| *acc += v));

    sliced! {
        let request_batch = use::batch(get_requests, nondet!(
            /// batching delays are observable in responses
            nondet_batch
        ));
        let count_snapshot = use::snapshot(current_count, nondet!(
            /// gets may observe any version of the count
            nondet_snapshot
        ));

        request_batch.cross_singleton(count_snapshot)
    }
}
```

A fuzz test today can only assert properties that hold for *every* choice of batching
and snapshot timing:

```rust
flow.sim().fuzz(async || {
    inc_send.send_many([1, 1]);
    get_send.send(0);
    // Which count the get observes (0, 1, or 2) is up to the fuzzer.
    // We can only assert facts that hold in all three cases.
});
```

With this proposal, the test can *script* those two decisions and assert the exact
outcome:

```rust
flow.sim().deterministic(async || {
    inc_send.send_many([1, 1]);
    get_send.send(0);

    snapshot_hook.reveal(1u64).await; // the batch observes count = 1 (not the latest, 2!)
    batch_hook.release(1).await;      // ...and contains exactly the one read request

    out.assert_yields_only_unordered([(0, 1u64)]).await;
});
```

Scripted tests are useful in several distinct ways:

- **Unit tests for specific interleavings.** The scenario prose ("a request arrives
  between two increments") becomes executable, instead of hoping the fuzzer stumbles on
  it or weakening assertions until they hold everywhere.
- **Readable regression tests.** When fuzzing finds a bug, the minimized reproducer is an
  opaque binary blob. A scripted test re-expresses it as a self-documenting scenario.
- **Executable documentation.** A scripted test reads as a specification of intended
  behavior under a particular timing.
- **Coverage for non-determinism the simulator cannot explore autonomously.** Some
  decision spaces are unbounded and have no autonomous exploration at all — most notably
  retries (`AtLeastOnce` delivery), where "which elements are duplicated, how many times"
  cannot be enumerated. A hook lets a test inject a *specific* retry pattern ("this ack
  is delivered twice"), providing the first coverage these operators can get.

## 2. Goals, and what we are careful about

The design decisions in this document are driven by a small set of concerns, listed here
so that later sections can refer back to them.

**G1: A test must never silently skip states the developer believes it covers.** This is
the overriding safety concern, and it is more subtle than it first appears. The obvious
failure is a hook the test *never* drives: its operator never fires, downstream outputs
never appear, and an assertion like "no more output arrives" can succeed for exactly the
wrong reason. The subtler failure is a hook the test drives *too late*: data sat buffered
at an operator through several other steps, the real system could have fired the operator
at any of those moments, and the test only exercises the late firing — the missed early
firing may be precisely the schedule that exposes a bug. Whenever the design has a choice
between silently proceeding and failing loudly, it fails loudly; and every mechanism that
buffers data without acting on it must be an *explicit declaration* in the test, never a
default the developer can stumble into.

**G2: The natural way to write a test must just work.** The style we expect developers to
reach for is sequential: drive some input, script a step, await its output, script the
next step based on what happened. This shape must work without ceremony — no manual
synchronization, no arbitrary sleeps, no defensive declarations — in every mode. Safety
rules that would force ceremony onto this natural style are wrong rules.

**G3: Scripts read as schedules, and errors point at script lines.** A scripted test is a
description of *one particular execution*. Reading the test top to bottom should
correspond to watching the execution unfold step by step. When the script is wrong —
a decision is missing, a step can never happen, two steps are written in an impossible
order — the failure should be reported at the offending line of the test, not as a
confusing downstream assertion failure or a panic deep inside the simulator.

**G4: One set of semantics across all modes.** The same script must mean the same thing
under `fuzz`, `exhaustive`, and `deterministic` execution. The modes may differ in *who
chooses* among the schedules the script leaves open (a fuzzer, an enumerator, or nobody
because nothing is left open) and in *when* a scripting error is detected — but never in
what a script means or which obligations the developer has. A test developed in
deterministic mode must not start failing spuriously when someone reuses its body under
fuzzing, and vice versa.

**G5: Scripting must compose with fuzzing.** Hooking is per-operator, not all-or-nothing.
A test should be able to pin the one operator whose timing matters for the scenario and
let the fuzzer explore everything else. This "pin one, explore the rest" middle ground is
one of the most valuable configurations, so the semantics must keep fuzzed exploration
unbiased around scripted operators (see G1) rather than quietly constraining it.

**G6: Zero impact on existing code.** Existing programs, libraries, and tests — every
`nondet!(...)` call site and every function signature taking `NonDet` — must continue to
compile and behave identically. Hookability must be adoptable one operator at a time,
and a component that exposes hooks must remain usable in production deployments with no
changes at its call sites.

## 3. Attaching a hook to an operator

This section covers the program-side API: how a test gets a handle to one specific unsafe
operator inside a program, in a way that is type-checked and survives refactoring.

### 3.1 `NonDet` learns to carry a hook

Every unsafe operator already takes a `NonDet` guard, and `NonDet` values are already
threaded through component boundaries as parameters — a library function that batches
internally takes a `nondet_batching: NonDet` parameter so its caller must acknowledge the
non-determinism. That existing plumbing is exactly the path a hook needs to travel, so we
extend `NonDet` rather than adding a parallel mechanism:

```rust
// today                       // proposed
pub struct NonDet;             pub struct NonDet<H = ()> {
                                   hook: Option<H>,
                               }
```

`H` identifies the kind of hook the guard can carry, and defaults to `()` so that plain
`NonDet` still means what it always did. The `nondet!` macro keeps its current forms and
gains one optional trailing argument:

```rust
nondet!(/** reason */)                       // as today: no hook attached
nondet!(/// reason
        nondet_parent)                       // as today: forwarded justification
nondet!(/** reason */ hook = my_hook)        // attach a hook handle
nondet!(/// reason
        hook = nondet_parent)                // forward a hook received from the caller
```

The `hook =` argument accepts either a raw handle or another `NonDet<H>` (whose payload —
possibly absent — passes through). That second form is how a *component* opts into
hookability. Taking the counter service from section 1, the only change is to name the
hook types in its signature and forward the guards to the operators they control:

```rust
pub fn counter_service<'a>(
    increments: Stream<u64, Process<'a, Counter>>,
    get_requests: Stream<u32, Process<'a, Counter>>,
    nondet_batch: NonDet<BatchHook<u32>>,       // was: NonDet
    nondet_snapshot: NonDet<SnapshotHook<u64>>, // was: NonDet
) -> Stream<(u32, u64), Process<'a, Counter>, Unbounded, NoOrder> {
    let current_count = increments.fold(q!(|| 0), q!(|acc, v| *acc += v));

    sliced! {
        let request_batch = use::batch(get_requests, nondet!(
            /// batching delays are observable in responses
            hook = nondet_batch
        ));
        let count_snapshot = use::snapshot(current_count, nondet!(
            /// gets may observe any version of the count
            hook = nondet_snapshot
        ));

        request_batch.cross_singleton(count_snapshot)
    }
}
```

Production callers are untouched (G6): `counter_service(incs, gets, nondet!(/** ... */),
nondet!(/** ... */))` still compiles, with the hook type inferred and the payload absent.
Only a test that wants control passes actual handles.

One rule worth stating explicitly: a hook is only ever attached where the test can see it
being attached. Forwarding a guard *without* `hook =` never propagates a binding, even if
the forwarded guard carries one. Guards are frequently forwarded many-to-one (a single
`nondet_raft` justifying four different `batch` calls), so implicit propagation would be
ambiguous — which of the four batches would the hook control? — and would bind hooks at
operators the reader of the code never sees. The `hook =` keyword makes every binding
visible at the exact operator it controls.

### 3.2 Hook handles are typed like the operator they control

Each kind of unsafe operator has a corresponding handle type, and the handle's type
parameters mirror the guarantees of the stream the operator consumes:

| Handle type | Controls | 
|---|---|
| `BatchHook<T, O = TotalOrder, R = ExactlyOnce>` | `Stream::batch`, `use::batch` |
| `KeyedBatchHook<K, V, O = TotalOrder, R = ExactlyOnce>` | `KeyedStream::batch` |
| `SnapshotHook<T>` | `Singleton::snapshot`, `Optional::snapshot`, `use::snapshot` |
| `KeyedSnapshotHook<K, V>` | `KeyedSingleton::snapshot` |

The operator's signature changes from `nondet: NonDet` to (for example)
`nondet: NonDet<BatchHook<T, O, R>>`, which is what makes hooking type-safe end to end:
attaching a `BatchHook<u32>` to a batch of `Stream<String>` is a compile error, attaching
a `SnapshotHook` to a `batch` is a compile error — and, more interestingly, the *set of
decisions the handle offers* is derived from the stream's type. A hook for a
totally-ordered stream offers no ordering decisions (there is nothing to reorder); a hook
for a `NoOrder` stream does. A hook for an exactly-once stream offers no redelivery
decisions; a hook for an `AtLeastOnce` stream lets the test script duplicate deliveries.
The type system guarantees a test can only script behaviors the program's types say are
possible. Section 4.1 lists the decisions themselves.

### 3.3 Minting handles, and bundles of hooks

A handle must exist *before* the program under test is constructed, because it is passed
into the program as part of a `NonDet` argument. Handles are minted from the
`FlowBuilder`:

```rust
let mut flow = FlowBuilder::new();
let batch_hook: BatchHook<u32> = flow.sim_hook();
let snapshot_hook: SnapshotHook<u64> = flow.sim_hook();
```

Each handle wraps a fresh internal ID allocated in the flow's state (the same pattern as
the existing `sim_input` ports). Handles are small and `Copy`: the same value is passed
into the program during construction and used later inside the test body to script
decisions.

A realistic component exposes several hooks, and its set of hooks is effectively part of
its testing interface. To keep that manageable, a derive lets a struct of handles be
minted in one call:

```rust
#[derive(SimHookBundle, Clone, Copy)]
pub struct CounterHooks {
    pub batch: BatchHook<u32>,
    pub snapshot: SnapshotHook<u64>,
}

let hooks: CounterHooks = flow.sim_hook(); // mints every field
```

Bundles nest (a Paxos bundle can contain a leader-election bundle), and since handles are
`Copy` a test can pass the bundle around or destructure it freely.

Misuse is caught early:

- Binding the same handle to two different operators is an error at flow build time,
  reported with both operator locations.
- Minting a handle and never binding it is allowed (a bundle may be only partially used
  by a particular program configuration), but scripting a decision on an unbound handle
  panics at that call.
- Binding hooks in a flow that is then *deployed* rather than simulated is harmless: the
  binding is metadata that non-simulator backends ignore, so shared construction code can
  bind hooks unconditionally.

## 4. Writing a script

This section covers the test-side API: the decisions a handle can make, and how a
sequence of decision calls in the test body maps onto executions of the program.

Some vocabulary used throughout, explained in depth in section 5: the program's tick
regions (a `sliced!` block, or operators placed into a `tick()`) execute repeatedly; each
round of execution is a **tick execution**. Every unsafe operator feeding a tick buffers
its incoming data, and a **decision** tells the operator what to release into the tick's
next execution.

### 4.1 Decisions

For a **batch hook**, a decision says which buffered elements form the next batch. On a
totally-ordered stream the order is fixed, so the decision is just a count; on an
unordered stream the contents are also a choice:

```rust
batch_hook.release(3).await;          // the next batch is the next 3 buffered elements
batch_hook.release_at([0, 2]).await;  // (NoOrder only) select by buffer position
batch_hook.release_all().await;       // the next batch is everything that has arrived
batch_hook.skip().await;              // the next batch is empty; hold everything
```

For streams typed `AtLeastOnce`, the batch hook additionally offers redelivery — a
released element is retained so a later batch can deliver it again, which is how a test
scripts a retry:

```rust
batch_hook.release(2).await;        // deliver the first two requests...
batch_hook.redeliver_at(1).await;   // ...then a later batch re-delivers the second one
```

For a **snapshot hook**, the buffered inputs are the successive versions of a piece of
state, and a decision picks which version the next tick execution observes:

```rust
snapshot_hook.reveal(1u64).await;    // reveal the version equal to 1 (assert + release)
snapshot_hook.reveal_next().await;   // advance to the next buffered version
snapshot_hook.reveal_latest().await; // reveal the current version
snapshot_hook.keep().await;          // observe the previously revealed version again
```

`reveal(value)` deserves special mention because we expect it to be the most-used
decision. It is a combined assertion and release: it scans forward from the
currently-revealed version through the buffered ones and releases the first version equal
to `value`, skipping over earlier versions; if the buffered versions pass `value` by
without matching, it panics and prints the versions it saw. The point is
self-checking scripts. A script written with positional decisions ("advance twice, then
read") breaks silently when the program under test changes how often the state updates; a
script written with `reveal(count == 1)` names the state it means, and any
mis-synchronization fails loudly *at the reveal*, not three assertions later where the
cause is hard to reconstruct.

Keyed hooks take a key as their first argument (`keyed_batch.release(&key, 2)`,
`keyed_snapshot.reveal(&key, value)`), with per-key semantics otherwise identical.

**Exact and loose decisions.** Most decisions above are *exact*: what they release is
fully determined by the decision itself plus a prefix of the buffered input — reading the
script tells you precisely what the batch contains or which version is revealed,
independent of any timing. Two decisions are deliberately *loose*: `release_all()` and
`reveal_latest()` release "whatever has arrived by the time the tick fires". In
deterministic mode that set is still fully determined (section 6 explains why); under
fuzzing it varies with the schedule being explored, which means a loose decision
deliberately leaves part of the outcome to the fuzzer even on a scripted hook. Loose
decisions are convenient for "flush whatever is there" and "assert against the current
state" phases; prefer exact decisions whenever the released contents matter to the
assertion.

### 4.2 The script is a schedule

Decision calls are `async`, and the sequence of calls in the test body is not a bag of
instructions — it is a **schedule**, read in program order:

- Consecutive decisions that target *different hooks of the same tick* form a **group**:
  one execution of that tick will consume all of them together. In the counter example,
  `snapshot_hook.reveal(1).await; batch_hook.release(1).await;` describes a single tick
  execution that reveals count 1 *and* releases one request.
- A decision that targets a **different tick execution** starts a new group. That means
  either a different tick region, or — equally — *the next execution of the same tick*:
  scripting a hook that already has a decision in the current group closes the group and
  begins the next one. So `batch.release(1).await; batch.release(1).await;` describes two
  successive executions of one tick.
- A new group does not become eligible to run until the previous group's tick execution
  has actually happened. The `.await` on the first decision of a new group suspends the
  test until then, so the test body advances in lockstep with the execution it describes.
  (Decisions whose turn has already come return immediately without suspending, so
  writing out a group is never interrupted halfway.)

A two-step script for the counter reads exactly like the execution it produces — here,
two successive executions of the same tick region:

```rust
inc_send.send_many([1, 1, 1]);
get_send.send_many([7, 8]);

snapshot_hook.reveal(1u64).await; // 1st execution: reveal count = 1
batch_hook.release(1).await;      // 1st execution: get 7 → (7, 1)   (same group: no waiting)

snapshot_hook.reveal(3u64).await; // 2nd execution: reveal count = 3 — suspends until the
batch_hook.release(1).await;      // 1st execution has actually run

out.assert_yields_only_unordered([(7, 1u64), (8, 3u64)]).await;
```

Two properties of this model matter enough to call out.

**While the script waits, the rest of the simulation does not.** When the test is
suspended waiting for a previous group's tick to run, the simulator's normal scheduling
continues: in fuzz and exhaustive mode, the fuzzer keeps freely choosing which *other*
ticks run — unhooked ones, and hooked ones whose decisions are ready — in any order, any
number of times. The script constrains only the relative order of the *scripted* tick
executions; everything else remains fully explored (G5). Section 5.4 explains why this
must be a genuinely free-running wait rather than "just run the previous tick now".

**A decision may be scripted before its data exists.** `release(3)` immediately after
`send_many([1, 2, 3])` is valid even though the three elements have not yet propagated
into the operator's buffer — propagation only happens while the test is suspended. The
decision simply waits, and the tick fires at the first moment it can be honored in full.
The test never needs to synchronize with internal propagation before scripting (G2). A
decision that can *never* be honored — the test only ever sent two elements — is an
error, reported when the simulation runs out of other work (section 5.3), attributed to
the script line that is suspended waiting.

**Incomplete groups are rejected at the offending line.** If the test starts scripting a
new group while the previous group's tick *cannot run* — say the previous group released
a batch but forgot to give the snapshot hook a decision, and new state versions are
buffered — the suspended call panics, naming the incomplete tick and hook:

```text
cannot script `batch` on tick B: previously scripted tick A cannot run
--> tests/counter.rs:88:9
   tick A's snapshot hook has 2 newer versions but no decision
help: finish scripting tick A (`reveal(..)`, `keep()`) or `pause()` its hook
```

This one rule catches two very different mistakes at their source (G3). The first is
*mis-pairing*: without it, a decision written for one execution could silently apply to a
later one (the batch pairs with a stale snapshot; the reveal drifts to the next
execution). The second is *causal deadlock*: a script whose decisions are written in an
order the dataflow cannot satisfy — for example, grouping all of tick A's decisions
before tick B's when A's second batch can only be released after B has processed A's
first — fails at the first out-of-order line, with the fix being "write the script in
execution order" rather than a mysterious hang.

### 4.3 Holding data on purpose: `pause`

A hook with buffered data and no decision is an error the simulator will report
(section 5.3) — that is the heart of G1. But sometimes buffering *is* the scenario: let
ten increments flow through while read requests pile up, then release them as one batch.
`pause()` is the explicit declaration that this is intended:

```rust
batch_hook.pause();
get_send.send_many([7, 8]);              // reads pile up at the paused batch
inc_send.send_many([1, 1, 1]);
other_out.assert_yields([3u64]).await;   // increments flow while reads stay buffered
batch_hook.release(2).await;             // resuming implicitly; both reads in one batch
```

While held, a hook is exempt from the missing-decision error, never causes its tick to
run, and — if its tick runs anyway because *other* hooks feed it — contributes its
"nothing new" behavior each time: a held batch hook contributes empty batches while its
buffer grows; a held snapshot hook keeps re-revealing its last version while newer ones
pile up. Scripting any decision implicitly resumes the hook; `resume()` also exists. A
hold takes its place in the script like everything else: called while a decision is still
pending, the hold begins once that decision has been consumed, so `release(2).await;
pause();` reads exactly as it executes — release two, then hold.

Two more ergonomic forms round this out. `pause_while(async || { ... })` pauses exactly
for the duration of a closure (resuming even on panic), so a bracketed buffering phase
cannot leak a paused hook. And `pause_until_count(n)` pauses and returns a future that
resolves once at least `n` elements are buffered — a synchronization point for scripts
where the right decision is not knowable upfront (for instance, when the number of
elements the program will produce into the hook is not statically known and the test
wants to inspect other outputs before deciding):

```rust
batch_hook.pause_until_count(3).await; // wait until 3 requests have reached the batch
// ... inspect other outputs, decide how this step should respond ...
batch_hook.release(3).await;
```

After `pause_until_count` resolves, the hook is unpaused; if the test then wanders off to
await something else without scripting a decision, the ordinary missing-decision error
applies. The pause family is for *declared* waiting, not for switching the safety net
off. (Snapshot hooks have the analogous `pause_until_versions(n)`.)

Finally, for tests where a hook should *only ever* act when scripted — buffering is the
rule, not the exception — `auto_pause()` sets a standing mode: the hook holds
immediately, and every scripted decision leaves a fresh hold in place behind it. An
auto-paused hook is therefore held exactly while it has nothing scripted:

```rust
batch_hook.auto_pause();          // this operator acts only when the script says so
get_send.send_many([7, 8, 9]);
inc_send.send_many([1, 1]);
other_out.assert_yields([2u64]).await; // reads buffer silently; no declaration needed here
batch_hook.release(2).await;           // acts once, then is held again automatically
other_out2.next().await;               // the remaining read stays buffered; still no error
```

This is deliberately nothing more than a standing use of `pause()` — "keep a hold in
place after every decision" — so the simulator core keeps its single simple rule
(buffered data with no decision and no hold = panic) with nothing to special-case; the
mode lives entirely in the handle (section 7). The trade-off is equally simple to state:
an auto-paused hook opts out of the protection described in section 10.1 — if the test
forgets a step, the operator silently holds its data instead of failing — so the one
`auto_pause()` line at the top of a test is the reviewer-visible marker that this hook's
timing is entirely script-driven, missed steps and all.

The obligation `pause()` discharges is the same in every mode, including deterministic
(G4): holding data past a point where the operator could have fired always requires the
declaration. Section 10.1 discusses why this rule is what protects against tests passing
because a hook fired *later* than it could have.

### 4.4 Awaiting outputs

Tests receive outputs through the existing simulator APIs (`out.next().await`,
`assert_yields`, `collect`, and friends). Scripting adds one rule: **an output await
completes only after every decision scripted so far has been consumed**. If the
simulation runs out of work first — some scripted decision can never be honored — the
await panics, attributed to the await's call site, naming the stuck decision.

This makes every point where the test body resumes a clean synchronization point: the
script written so far has fully happened, so follow-up actions (pausing a hook, scripting
the next phase, sending more input) never race against a half-applied script. It also
means a stuck script cannot masquerade as a completed one — the very first thing the test
observes after a stuck decision is a panic pointing at it, rather than an assertion
failing on missing output three lines later. Adaptive tests fall out naturally: script a
phase, await an output, inspect it, script the next phase.

## 5. How the simulator executes a scripted test

This section defines the semantics that sections 3–4 rely on. It applies to all modes;
section 6 covers what is special about deterministic mode.

### 5.1 Background: how the simulator schedules today

A brief recap of the existing simulator, since the scripting semantics slot into it. The
compiled simulation consists of long-running *async dataflows* (which move data between
locations and into buffers) and *tick dataflows* (one per tick region, executed one round
at a time). Every unsafe operator feeding a tick is represented by a **hook object** in
the simulator: it accumulates incoming data in a buffer, and before each execution of the
tick it decides what to release into that execution. Today all hooks decide
*autonomously*, drawing on fuzzer entropy (in exhaustive mode, the same entropy interface
enumerates instead of sampling).

The scheduler alternates between the test body and the simulation using a *biased* loop:
the test body is always polled first, and only when it is blocked (awaiting an output or,
now, a decision) does the simulator take a step — run the async dataflows until they stop
making progress, then pick one ready tick (a fuzzed choice) and execute it, releasing
each of its hooks' decisions into it. When nothing can make progress at all and the test
is still waiting, the simulation is *quiescent*: output streams report that no more
messages can arrive, which is what assertions like `assert_no_more` check.

Because the test body always runs first, anything the test does between two awaits is
atomic with respect to the simulation — no tick runs behind the test's back while it is
actively executing. This existing property does a lot of quiet work in what follows.

### 5.2 What changes for a scripted hook

A hook that has been bound to a handle changes behavior in one fundamental way: **it
never makes a decision on its own.** No fuzzer entropy is ever spent on it; everything it
releases was scripted (with one narrow, precisely-bounded exception described below).
From that principle, three rules:

**Rule 1 — a scripted decision names one specific execution, and the tick waits for it.**
The next queued decision on a hook applies to that tick's *next* execution — never to a
later one. Consequently, while any hook of a tick has a queued decision that cannot be
honored yet, that tick is not allowed to run at all. There are two ways a decision can be
not-yet-honorable: its group hasn't been reached yet (an earlier group's tick execution
hasn't happened — section 4.2), or its data hasn't arrived yet (`release(3)` with two
elements buffered). In both cases the tick simply waits. This is what makes scripts
readable as schedules (G3): a decision you wrote can never be skipped over, silently
dropped, or paired with a different execution than the one your script describes.

**Rule 2 — a hook with data but no decision is an error, checked at every scheduling
boundary, in every mode.** Each time the async dataflows stop making progress and the
scheduler is about to consider running ticks, it first scans all scripted hooks. If any
unpaused hook holds meaningful buffered input and has no queued decision, the simulator
panics, naming the operator and the buffered data:

```text
scripted hook has buffered input but no decision:
--> src/counter.rs:42:31
 |     let request_batch = use::batch(get_requests, nondet!(
 |                         ^ 1 buffered item: [0] (hook: BatchHook<u32>, decisions queued: 0)
help: script a decision (e.g. `hook.release(..)`) or call `hook.pause()` if buffering is intended
```

This is the enforcement of G1, and its placement is deliberate. The scan fires at the
first moment where the missing decision could influence what happens next — the real
system could have fired this operator here, and the simulator refuses to guess whether
the test meant it to. It runs identically in fuzz, exhaustive, and deterministic mode, and
consumes no entropy (so recorded fuzz reproducers replay unchanged). And because the test
body always runs before the scan (biased loop), the natural sequential pattern is never
caught in a transient: when tick A's execution produces both an awaited element and fresh
data for hook B, the test is resumed *first*, scripts B (or pauses it), and only then does
the scheduler reach the next boundary. The scan only ever fires when the test has genuinely
moved on without saying what B should do. The alternatives to this rule — and why they
fail — are discussed at length in section 10.2.

**Rule 3 — a decision that can never be honored is reported at the script line waiting on
it.** If the simulation reaches quiescence while some scripted decision is still waiting
(its data will never arrive, or its group can never be reached), the panic is raised
through whichever decision call or output await the test body is suspended in, so the
report points at the test:

```text
scripted decision can never be satisfied:
--> tests/counter.rs:92:9      <- the batch_hook.release(3) line
   release(3) is waiting on `batch` at src/counter.rs:42:31, which has 2 buffered items
   and can receive no more input
```

### 5.3 The narrow exception: decisions that are the only possibility

There is exactly one situation where a scripted hook acts without a scripted decision:
when a tick executes and one of its hooks has an empty queue, *and there is only one
thing that hook could possibly do*. A batch hook whose buffer is empty can only
contribute an empty batch; a snapshot hook with no newer versions can only re-reveal what
it already revealed. In those situations the hook does that implicitly. Requiring the
test to write `skip()` or `keep()` for every such hook at every execution would be pure
noise — there is no decision being made, because there is nothing to decide.

The boundary between "nothing to decide" and "something to decide" is exactly the rule 2
scan: the moment the hook's buffer holds real data (or newer versions), the implicit
behavior is off the table and the absence of a decision is an error. A scripted hook
therefore never silently holds back data and never silently reveals a stale version —
the only implicit behaviors are the ones that were forced. Whether this narrow implicit
rule can ever interact badly with scheduling was a major focus of the design review; the
full analysis (it cannot, and why) is in sections 10.3 and 10.4.

### 5.4 Why waiting for a group is a free-running wait

Section 4.2 said that when the test starts a new group, its first `.await` suspends until
the previous group's tick execution has happened — and that meanwhile the simulator keeps
scheduling *freely*. It is tempting to implement this wait more directly: the previous
group's decisions are all queued, so why not just run that tick immediately and hand
control back?

Because "immediately" would skip states (G1, G5). Two separate things can legitimately
need to happen before the previous group's tick can — or should — fire:

- **Its decisions may not be honorable yet.** The group's `release(3)` may be waiting on
  data that is produced by *other* ticks — including fuzzed ones — that haven't run yet.
  Forcing the tick to fire now would either violate rule 1 or force a partial batch.
- **Even when its decisions are honorable, firing order matters for loose decisions.**
  Since `release_all()` and `reveal_latest()` release "whatever has arrived", *when* the
  scripted tick fires relative to the fuzzed ticks around it changes what it releases.
  If the scheduler forced the scripted tick to fire at the earliest possible moment, the
  schedules where a fuzzed tick delivers more data first would be systematically
  unexplored — a silent bias, invisible to the developer.

So the wait is implemented as a *subscription*, not an intervention: the suspended
decision call registers to be woken when the previous group's tick execution completes,
and the scheduler goes on making its ordinary (fuzzed or deterministic) choices — any
number of other ticks may run first. The scripted tick fires whenever the ordinary
schedule reaches it with all decisions honorable; the test is then resumed (before the
next scheduling boundary, per the biased loop) and the next group becomes available. In
exhaustive mode this means every legal placement of the scripted execution among the
fuzzed ones is explored; the script pins only what it says — the relative order of
scripted executions — and nothing else.

### 5.5 What scripting removes from the search space — and what it doesn't

A scripted hook with *exact* decisions releases identical contents in every fuzz
instance: its dimension is removed from the search space entirely. If every hook in the
program is scripted with exact decisions and the inputs are fixed, `exhaustive` explores
exactly one execution.

Two deliberate exceptions keep this statement honest. *Loose* decisions
(`release_all()`, `reveal_latest()`) leave their contents co-varying with the fuzzed
schedule around them — no new entropy is drawn, but the script no longer determines the
released set (see 4.1). And a *paused* hook's behavior depends on how many times its tick
fires while paused, which in fuzz mode is a fuzzed quantity. Both are explicit,
per-call/per-declaration choices visible in the test.

## 6. Deterministic mode

`flow.sim().deterministic(async || { ... })` runs the test body against exactly one
execution of the program, with no fuzzer involved anywhere — the promise is "if it passes
once, it passes always, on every machine." Everything in sections 4–5 applies unchanged
(G4); this section explains what the absence of a fuzzer means and the guarantees that
follow.

### 6.1 Every source of variation must be pinned

Three things vary between runs of a fuzzed simulation: the inputs (already scripted, via
`sim_input`), the unsafe operators' decisions, and the scheduler's choice of which ready
tick to run next. Deterministic mode eliminates the first two by requirement and the
third by consequence:

- **Every unsafe operator that receives data must be hooked and scripted.** An unhooked
  operator encountered with meaningful input has no fuzzer to decide for it, and
  silently substituting some fixed behavior would fake coverage — so it panics, naming
  the operator. This is also the answer to "what happens when a program change introduces
  a *new* unsafe operator?": the scripted test fails with an actionable error pointing at
  the operator that now needs a hook (or whose component should expose one), rather than
  silently pinning its behavior with a fixed seed.
- **The scheduler's tick choice disappears structurally**, which is the subtle part,
  covered next.

### 6.2 The invariant: at most one tick is ever runnable

Deterministic mode does not need a tie-breaking policy for "which tick runs next",
because the situation where two ticks are simultaneously runnable cannot arise. This is
worth spelling out, since the whole mode rests on it. Consider any tick and ask what
could make it runnable:

- A tick with an **unhooked** hook holding data has already panicked (6.1). An unhooked
  hook with *no* data offers nothing to run.
- A **scripted hook with no queued decision** never makes its tick runnable: either its
  buffer is empty / has nothing new (the only-possibility case, 5.3 — nothing to run *for*),
  or it holds real data, in which case the boundary scan has already panicked (rule 2).
- A **scripted hook with a queued decision that is not yet honorable** blocks its tick
  outright (rule 1).
- That leaves scripted hooks whose queued decisions are honorable — and decisions only
  become available group by group, in script order (4.2). At most one group is
  "current" at any moment, so at most one tick has honorable queued decisions:
  **the current group's tick, and nothing else**.

So the deterministic scheduler is trivial: run the async dataflows until they stop making
progress; run the boundary scan; if the current group's tick can be honored in full, run
it; otherwise there is nothing to do. There is no choice being made — the *script* is the
schedule, and the only ordering the developer ever needs to reason about is the one they
wrote. (In fuzz mode the same script leaves the placement of scripted executions among
fuzzed ticks open, per 5.4; in deterministic mode there are no fuzzed ticks for them to
interleave with.)

One scheduling detail becomes load-bearing here: ticks fire only after async propagation
has fully quiesced. This is what gives the *loose* decisions (`release_all()`,
`reveal_latest()`) a well-defined meaning in deterministic mode — "everything that has
arrived" is precisely the set of data causally available at this point in the script,
which is a deterministic set. A loose decision under deterministic execution is exactly
as reproducible as an exact one; the difference is only that the script does not state
the contents, the execution does.

### 6.3 Quiescence, and why there is no end-of-test check

When the test is awaiting an output and the simulation has nothing left it can do, it is
quiescent. By the boundary scan, quiescence can never be reached while an unpaused hook
holds undecided data — that panics first. So at quiescence, the pending state can only
be:

- **A scripted decision that can never be honored** — an error, reported at the suspended
  script line (rule 3). We call this *dirty* quiescence.
- **Paused hooks holding data** — declared, expected, fine. This is *clean* quiescence,
  and it is delivered normally: output streams report end-of-input, so `assert_no_more`
  and `collect` complete. Notably, this means quiescence-checking assertions are
  trustworthy by construction: `assert_no_more` cannot succeed while an operator secretly
  holds data the test never accounted for, because either a decision is queued for it
  (the output await would have waited, 4.4), or it is paused (declared), or the scan
  panicked long ago.

A natural question is whether the *end of the test body* needs a final check — could data
quietly arrive at a hook after the last thing the test awaited, leaving an undecided hook
at exit? It could, but no check is needed, and it is worth understanding why. If the
boundary scan never fired during the test, then at every point where the simulator was
about to act, that hook had no undecided data — meaning there was no moment at which the
operator could have fired causally *before* anything the test asserted. The executions
the test checked are exactly the executions its assertions covered. Data arriving after
the final scheduling boundary can only influence outputs the test never looked at, which
is the same harmless category as input the test sent but never awaited results for. (An
earlier iteration of this design had an explicit exit check; section 10.5 walks through
how it became unnecessary.)

## 7. Implementation sketch

This section maps the design onto the existing simulator internals; it assumes
familiarity with `hydro_lang/src/sim/`.

**From the surface API to the IR.** `nondet!(... hook = h)` produces
`NonDet { hook: Some(h) }`. Operators that accept a hookable guard (`batch`, `snapshot`,
and their atomic/keyed/`sliced!` variants) copy the handle's ID into a new
`manual_hook: Option<HookId>` field on their IR node metadata, next to the backtrace
metadata already used for rendering hook locations in logs. Non-simulator backends ignore
the field. `HookId` is a counter in `FlowStateInner`, minted by `flow.sim_hook()` exactly
like `ExternalPortId` is for `sim_input`.

**Runtime wiring.** The sim builder, on seeing `manual_hook`, wraps the operator's normal
hook object (`StreamHook`, `SingletonHook`, ...) in a scripted-mode shell — call it
`Scripted<H>` — registered in a per-instance `HashMap<HookId, ...>` threaded through the
dylib entry point (the same pattern as the `external_in`/`external_out` queues).
Test-side handles resolve their `HookId` through the task-local sim context, the same way
`SimSender::send` resolves its port, so handles work across fuzz iterations without
per-instance setup. Decision variants that carry values (`reveal(v)`) are
bincode-serialized across the dylib boundary and compared after deserialization —
mirroring `sim_input` — which puts `Serialize` / `DeserializeOwned + PartialEq + Debug`
bounds on value-carrying decisions only.

**`Scripted<H>` state: not a queue.** A consequence of the script protocol is that the
scripted state per hook collapses to two fields beside the operator's normal buffer:

```rust
next_decision: Option<Decision>,  // at most one, by construction
hold: bool,                       // pause() / auto_pause() / transient (see below)
```

There is never more than one live decision per hook: decisions are installed through the
async call protocol, and the first decision of a new group installs only *after* the
previous group's tick execution has been consumed — so a hook's earlier decision is
always gone before its next one arrives (installation asserts `next_decision.is_none()`
as an internal invariant). "Hold comes after the decision" needs no ordering structure
either: the fields are consulted in order — decision present → normal decision
semantics; otherwise hold set → held; otherwise data present → the forgotten state.
Cancellation is correspondingly trivial: `resume()` asserts no decision is pending and
clears the bool.

**The decision-call protocol** (all handle-side; the shared state is single-threaded, so
these are plain compound mutations):

1. Set `hold = true` if not already. This covers the suspension that may follow: while
   the call waits for the previous group, its hook may already have buffered data and no
   decision, and the hold is what tells the boundary scan "the test is mid-script for
   this hook" — no special case in the scan needed.
2. Await group activation (resolves immediately, without yielding, if this decision
   belongs to the current group — so writing out a group is atomic).
3. Assert `next_decision.is_none()`; set `hold = false` (unless the hook is in
   `auto_pause` mode); install the decision.

`pause()` sets the bool (taking effect after any pending decision, per the field order
above); `resume()` clears it; `pause_until_count(n)` sets it and returns a future that
clears it when the watermark is reached; `auto_pause()` just makes step 3 leave the bool
set. None of these touch the scheduler.

**The rest of `Scripted<H>` behind the existing `SimHook` trait:**

- `current_decision()` materializes `next_decision` if it is honorable against the
  buffer; otherwise reports none.
- `can_make_nontrivial_decision()` is `next_decision_honorable`. Pending input alone
  never makes a scripted hook offer work; the boundary scan is the mechanism that reacts
  to it.
- A new `blocks_tick()` accessor is true while `next_decision` is present but not yet
  honorable; `SimTick::can_run` requires `!blocks_tick()` for every hook, implementing
  rule 1.
- A new `forgotten()` accessor is `next_decision.is_none() && !hold &&
  has_meaningful_input()`, consulted by the boundary scan.
- `autonomous_decision()` never touches the entropy driver. It produces the
  only-possibility implicit behavior (5.3) when that applies (including while held), and
  otherwise aborts as an internal invariant violation — unreachable, because the scan and
  `blocks_tick()` gate every path that could ask.

**Scheduler changes** (in `LaunchedSim::step` and around it):

- The **boundary scan**: at the existing point where the async dataflows report no
  progress and the scheduler moves to tick selection, iterate scripted hooks and panic on
  any `forgotten()` one. No entropy consumed.
- **Group activation**: a per-instance cell tracks the current group. Consuming a group
  (its tick executed) activates the next and wakes the suspended decision future; the
  biased loop then polls the test body before taking another step, preserving the
  clean-resumption property (4.4, 5.4). Decision futures resolve immediately (without
  yielding) when their group is already current, making group assembly atomic.
- **Quiescence checks**: at `wait_for_resume`, scan for queued decisions that can never
  be honored and unresolved `pause_until_count` waits; raise these through whichever
  script/await future the test is suspended in, using the same caller-tracking technique
  as the existing assertion futures, so reports carry test line numbers.
- **Output-await barriers**: the quiescence-aware receiver streams additionally wait for
  "all scripted decisions consumed" before yielding elements, and panic instead of
  yielding end-of-stream when quiescence is dirty.
- **Deterministic mode** drops the fuzzed tick choice (at most one candidate exists,
  6.2); everything else — the scan, the checks, the biased loop — is shared code.

**Logging.** Scripted releases reuse the existing per-release log lines, tagged as
scripted, so mixed tests show fuzzer-chosen and scripted decisions uniformly:

```text
* --> src/counter.rs:42:31
*  |         let request_batch = use::batch(get_requests, nondet!(
*  |                             ^ releasing items (scripted): [7]
```

## 8. Scope

**v1** covers: `NonDet<H>` and the `nondet!` `hook =` syntax; `BatchHook`,
`KeyedBatchHook`, `SnapshotHook`, `KeyedSnapshotHook` (spanning `batch`/`snapshot` and
their atomic and `sliced!` forms); the decision APIs of section 4.1 including redelivery
for `AtLeastOnce`; the script/group model with its panics; the pause family including
`auto_pause`; `flow.sim_hook()` and `#[derive(SimHookBundle)]`; and deterministic mode.

Follow-ons, roughly in priority order:

- **A hook for `assume_retries` on top-level streams.** Batch-boundary retries are
  covered by `BatchHook`'s `AtLeastOnce` capability, but `assume_retries` on a stream
  outside any tick is a separate operator that would need its own handle (e.g.
  `retries_hook.duplicate_next(2)`). Same motivation: no autonomous exploration exists
  for retries at all.
- **Ordering hooks** (`assume_ordering`, `merge_ordered`, `entries_partially_ordered`):
  decisions are interleavings, and the API needs care to stay readable — one-at-a-time
  "deliver this next" scripting seems more usable than permutation indices.
- **Value-based batch selection** (`release_values([...])`, `release_where(pred)`) for
  unordered streams, using the same serialization channel as `reveal(v)`.
- **Network-level hooks**: message drop and delivery-order control on specific channels,
  which would subsume many uses of ordering hooks at the transport level.

## 9. Open questions

1. **`nondet!` argument order**: should `hook =` come before or after forwarded guards?
   Currently proposed: last, matching "optional trailing argument" intuition.
2. **Scripting inside `fuzz` bodies**: handles resolve per-instance automatically, so
   scripted decisions inside a fuzz closure work today in the design. Should that be
   embraced (it is how "pin one, explore the rest" is written) or should some decision
   kinds be discouraged there? Currently proposed: fully allowed.
3. **Naming**: `sim_hook` vs `hook`; `BatchHook` vs `BatchScript`; `pause` vs `hold`;
   `reveal_latest` vs `reveal_current`. Also whether the doc-facing term should be
   "hook" or "control point".
4. **Group boundaries at awaits**: a group is defined by *consecutive* decision calls.
   Should an interposed output await (or input send) between two decisions for the same
   tick split them into two groups? Currently proposed: yes — awaits are barriers, so
   decisions after an await describe a new step.
5. **`keep_expecting(v)`**: an assert-flavored `keep()` that also verifies the retained
   version equals `v`, for scripts that want reveal-style self-checking on re-observed
   state.

## 10. Discussion

Each subsection here is a long-form account of a part of the design that went through
real deliberation: what the concern was, what alternatives were considered, and why the
final rules look the way they do. The main body describes only the final design; this is
where the reasoning (including abandoned iterations) lives.

### 10.1 Forgotten hooks — and *temporarily* forgotten hooks

The first safety question this design has to answer is: what happens when a test binds a
hook to an operator and then fails to drive it?

The obvious version of the failure is total: the hook is never scripted, so its operator
never fires. Everything downstream of it simply never produces output. The danger is that
this failure mode is *quiet* — output streams don't error, they just end. A test that
asserts "the response is 1" fails visibly (good), but a test that asserts "no duplicate
responses arrive" *passes*, because no responses arrive at all. The developer reads a
green test as evidence about deduplication logic that was, in fact, never executed.

The much subtler version is partial, and it drove more of the design than the total one.
Suppose the script does fire the tick — but later than it could have. Concretely: read
requests arrive at a batch, then several increments are processed by other steps of the
system, and only then does the script release the batch. In the real system, the batch
could have fired while those increments were still in flight; that early firing (a read
racing the increments) may be exactly the interleaving that exposes a bug. The test
passes — not because the code is correct, but because the only schedule it exercised was
the forgiving one. Nothing about the test's assertions or its output is wrong; the gap is
in *which executions were simulated*, which no assertion can see.

This is why the error condition throughout the design is "an unpaused hook holds
meaningful buffered input and has no queued decision" — a statement about a *moment
during execution* — rather than "the hook was never fired," a statement about the end of
the test. Every moment at which the real system could have fired the operator, the
simulated test must either say what the operator does (a decision), or explicitly
acknowledge that it is deliberately doing nothing (`pause()`). Holding data by default
was considered and rejected precisely because of the partial failure: buffering-as-default
converts "I forgot the batch could fire here" into a green test.

This is also why `pause()` is required in *deterministic* mode, which surprises people at
first — after all, in deterministic mode ticks only fire when the script says, so
buffering seems like it could be the harmless default. An earlier iteration of this
design did exactly that. The problem is the same partial failure wearing a different
coat: with buffering-as-default, a deterministic test that scripts the batch late is
indistinguishable from one that scripts it late *on purpose*, and the missed early-firing
schedule leaves no trace. Requiring `pause()` uniformly means every deliberate hold is
visible in the test as a declaration, and every accidental one is a panic — in every
mode, at the same points, per G4. The cost is one extra line in tests that genuinely want
to hold data; the benefit is that "the operator could have fired here and the test chose
not to" is always something a reviewer can see written down.

It is worth noting how `auto_pause()` (4.3) fits this picture, since it is precisely the
buffering-as-default behavior this section argues against — as an engine default. The
difference is that `auto_pause()` is a per-hook, per-test *declaration*, sitting on one
visible line, that this hook's timing is entirely script-driven and that missed firing
points are accepted. That is a legitimate position for many tests (especially ones
narrating a long scenario around a single operator), and expressing it through the hold
mechanism means the engine's core rule — buffered data with no decision and no hold is a
panic — never gains a second mode or a special case. The safety analysis of this whole
design only ever has to reason about holds; `auto_pause()` is just a policy for when the
handle leaves one in place.

### 10.2 How should a forgotten hook be confronted?

Given that an unpaused hook with data and no decision is an error state, there were four
candidate mechanisms for detecting it. Walking through why three of them fail explains
why the boundary scan (rule 2 in 5.2) is shaped the way it is.

**Panic the moment data arrives.** Simplest, and wrong: it makes correct scripts fail.
Decisions are scripted before the test suspends, but data propagates only *while* it is
suspended (the simulator runs only when the test body is blocked). A test that sends
input and then scripts a decision would race its own propagation — whether it panics
would depend on internal buffering detail the test cannot observe. Any rule that forces
tests to synchronize with propagation before scripting violates G2.

**Hide the undecided tick and flag it only at quiescence.** In this formulation, a hook
with no decision reports "nothing to run"; if the whole simulation eventually stalls
while the test is still waiting, report the undecided hook then. This behaves correctly
for *scripted* decisions that aren't honorable yet — the tick waiting is exactly right
(rule 1) — but for the *forgotten* case it is a silent bias, and the bias is worth
understanding in detail because it is invisible in exactly the way G1 forbids. Consider a
hooked tick whose outputs feed one side of a downstream `merge_ordered`, with some other
tick feeding the other side. The merge's own interleaving choice (fuzzed or scripted) can
only interleave elements that have *arrived*. While the hooked tick is hidden, its
outputs never arrive — so every explored interleaving has the other side's elements
first. The test's awaited outputs all show up, its assertions pass, and the executions
where the hooked tick's outputs came first were never simulated. No quiescence is ever
reached (the other side keeps making progress), so the "flag it at quiescence" backstop
never triggers. The developer gets a green, biased test.

**Keep the undecided tick schedulable, and panic if the scheduler picks it.** This was
the design for a while: the forgotten tick stays visible, so the fuzzer can run into it,
and running into it panics. It has the right *semantics* — the error is confronted, not
hidden — but detection is probabilistic in fuzz mode: an instance catches the error only
if the fuzzer happens to pick that tick before the test's awaits resolve. The same
scripting mistake passes some instances and fails others for no semantic reason, and a
`with_instance` run might never catch it.

**The boundary scan (chosen).** Check the error condition directly, at every point where
the scheduler is about to act (async propagation exhausted, ticks about to be
considered), in every mode, consuming no entropy. This is the earliest *sound* detection
point: any earlier races propagation (first alternative); any later either hides the
state (second) or leaves detection to chance (third). Because the test body always runs
before the scheduler takes its next step, the scan also never fires on the transient
state in the natural sequential pattern — when a tick's execution simultaneously
produces an awaited element and data for the next hook, the test is resumed first and
scripts that hook before any boundary is reached. The scan fires only when the test has
demonstrably moved on without saying what the hook should do.

One residual instance-dependence remains in fuzz mode, and it is inherent rather than a
detection gap: whether the data *arrives* before the test's last relevant await resolves
is itself part of the schedule being fuzzed. Instances where it never arrives have
nothing to detect. The scan guarantees that every instance that exposes the error state
reports it.

### 10.3 Are implicit decisions safe? (the T1/T2 discussion)

Section 5.3 allows a scripted hook to act without a decision in exactly one situation:
when only one behavior is possible (empty buffer → empty batch; no newer versions →
re-reveal). This looks harmless — nothing is being decided — but it conditions on the
buffer being empty *at the moment the tick fires*, and "at the moment" is a scheduling
fact. That earned it the most scrutiny of anything in this design. The concern, its
false fixes, and its eventual resolution are worth recording in full.

**The concern.** Call the scripted tick T1, and suppose its snapshot hook H has no queued
decision and an empty buffer, so T1 can fire using the implicit "nothing new" behavior.
Suppose some other tick T2, had it run *first*, would have produced data into H. If the
scheduler eagerly fires T1, then the execution where T1 observes T2's data — an execution
the real system could exhibit — is never simulated, and (worse) nothing reports this: the
implicit behavior was legal precisely because the buffer was empty when the scheduler got
there. In deterministic mode this looked especially alarming, since some fixed internal
order would decide T1-vs-T2 once and for all, silently.

**False fix #1: defer implicitly-deciding ticks.** Make any tick that would rely on an
implicit behavior wait until nothing else can run, so that potential feeders like T2 get
their chance first. This falls apart on its own counterexample: if T1 *and* T2 both rely
on implicit behaviors and each could feed the other, both defer, and the scheduler must
still pick one to go first — the arbitrary silent choice has just been moved into the
deferred set, not eliminated.

**A tempting observation that was not a fix.** Tracing the eager schedule further: after
T1 fires and T2 then feeds H, H is now an unpaused hook with data and no decision — the
forgotten state — so its next confrontation panics, forcing the developer to make H
explicit. Doesn't that resolve the concern retroactively? No, for two reasons. First, the
panic comes one execution too late: T1 already fired with the implicit behavior, so the
"T2 first" pairing was still never simulated; the developer is forced to rewrite, but
never shown the schedule they missed. Second — and this is the sharper point — the panic
can be *masked*: if the test resumes (a group activation, an output await) before the
confrontation, it can script H's *next* decision, converting the forgotten state into a
scripted one. No panic ever fires, and the eager execution stands silently. What this
observation actually contributed was a precise location for the hole: it lives in the
timing of *test resumption* relative to confrontation.

**False fix #2: gate test resumption on quiescence.** If the test can only resume once
the simulation has nothing left to confront, the masking window closes. But this breaks
the most natural test there is (G2): script tick A, await its output, then script tick B
with the data A produced. B's hook holds freshly-produced data and no decision at the
moment the await would resolve — under gating, the await *cannot* resolve until that
state is confronted, and confronting it panics. The natural sequential test becomes
unwritable. Withdrawn.

**The resolution: the scenario cannot arise.** The false fixes were both answering the
question "in which order should the scheduler run T1 and T2?" — and the final script
model makes that a question the scheduler is never asked. Work through who T2 can be:

- *T2 is scripted.* Then T1-vs-T2 order is not a scheduler choice at all: the script
  activates one group at a time (4.2), so whichever group the developer wrote first runs
  first. "T2 feeds H before T1 fires" corresponds to a different script — one the
  developer chose not to write. There is no arbitrary order to leak, in any mode.
- *T2 is unscripted (fuzzed) and holds data.* In deterministic mode this panics
  immediately (unhooked operator with data, 6.1). In fuzz mode T2 is a fuzzed tick, and
  the fuzzer legitimately explores both "T2 before T1" and "T1 before T2" — in the
  former, T1's hook H holds data at the boundary before T1 fires, and the boundary scan
  panics, forcing the developer to script what T1 does with that data. The exploration
  itself surfaces the missed pairing.

So the implicit rule is safe not because of any mechanism aimed at it, but as a
consequence of two structural facts: scripted executions are totally ordered by the
script, and unscripted feeders cannot slip data past the boundary scan. Both false fixes
were withdrawn with nothing replacing them.

### 10.4 Can the implicit rule make a test behave differently between runs, silently?

A related but distinct worry about the implicit rule: whether the implicit behavior
applies depends on whether upstream data has arrived when the tick fires, and in fuzz
mode arrival timing varies between instances. Could the same script therefore *behave*
differently in different instances — pass with meaning A here and meaning B there —
without anyone noticing?

The reassuring answer comes from looking at what the divergence would require. For the
implicit behavior to mean something the developer didn't intend, the developer must have
intended the hook to have data at that execution. But if the data *can* be there under
some schedule, then instances with that schedule have a hook holding data with no
decision — and the boundary scan panics there. So any instance-to-instance behavioral
difference traceable to the implicit rule comes packaged with panicking instances: the
failure mode is a possibly-flaky but loud and precisely-attributed panic, never two
silently different green runs. The only fully silent case is when the intended data can
*never* be present at that execution under any schedule (it is causally produced later) —
in which case the implicit behavior is not a divergence, it is the only possible
behavior, and the developer's misunderstanding is about their own dataflow, which
assertion-style decisions like `reveal(v)` are designed to surface downstream.

In deterministic mode the question dissolves entirely: scripted executions are totally
ordered, ticks fire only after propagation quiesces, and unscripted operators with data
panic — so whether a hook has data at a given execution is a function of the script
alone. The same script either always panics (make the decision explicit) or always
proceeds with the same meaning.

The honest residual cost, in fuzz mode, is flakiness of the *panic*: a mixed test may
pass many instances and then fail one where the fuzzer delivered data earlier than usual.
That is the forcing function working as intended — the failing instance is a real
schedule of the real system — but it will feel like a flaky test to a developer, so the
panic message must make the resolution obvious (script the decision or pause the hook).

### 10.5 The end-of-test check that turned out to be unnecessary

For part of its history this design had a final check at test exit: after the body
completes, propagate once more and verify no unpaused hook holds undecided data and no
scripted decision is left unconsumed. It felt obviously right — a last chance to catch a
forgotten hook whose data arrived after the last thing the test awaited.

It was dropped after working through what such leftover data can actually affect. The
boundary scan runs before every point where the simulator acts. If it never fired during
the test, then at every one of those points the hook in question had no undecided data —
which means there was no moment at which its operator could have fired causally *before*
anything the test observed. Every output the test asserted about was produced by
executions that are exactly the executions the script describes. Data that arrives only
after the final scheduling boundary can influence only outputs the test never looked at —
the same benign category as input sent without ever awaiting its results, which the
simulator has always allowed. Meanwhile the cases that *feel* like they need an exit
check are already covered earlier: a stuck decision panics at the await that depends on
it (4.4), and quiescence-checking assertions cannot succeed over undeclared data (6.3).
The check would have added a failure mode without adding coverage of any assertable
behavior — so it was removed, and the invariant "checks exist exactly where they protect
an assertion" was kept clean.

### 10.6 The prefix invariant: a rule this design needed, then outgrew

Early versions of this design contained a hard invariant: every decision's released
output had to be a deterministic function of the decision plus a *prefix* of the hook's
input buffer. `release(3)` qualifies (the first three elements, whenever they exist);
"release everything buffered right now" does not, because *right now* depends on the
schedule. Under that invariant, `release_all()` and `reveal_latest()` were deliberately
excluded from the API.

The invariant existed to protect a particular scheduler. At the time, deterministic mode
was designed as a *sweep*: at each step, execute every tick that has pending work, in
some fixed but arbitrary internal order. Under a sweep, that arbitrary order is
observable through any buffer-dependent decision — if T1's `release_all()` fires before
vs. after T2 pushed data into its buffer, the batch differs, and which one you get is
decided by an ordering the developer never chose. The prefix restriction made hook
outputs immune to the sweep's internal order, which is what made the sweep sound.

The sweep did not survive the design process. Once the script model settled — one group
active at a time, groups totally ordered by the test body — deterministic mode's
scheduler stopped having any ordering freedom at all: at most one tick is ever runnable
(6.2). With the arbitrary order gone, the invariant's soundness role went with it:

- In deterministic mode, "everything buffered right now" is a script-determined set —
  ticks fire only at propagation-exhausted points, and no other tick can interleave — so
  a loose decision is exactly as reproducible as an exact one.
- In fuzz and exhaustive mode, a loose decision's contents vary with the schedule — but
  varying with the explored schedule is what fuzzing *is*. Every tick that could have
  contributed data either is concurrently ready (both orders explored), cannot yet run
  (so it could not have contributed in any real execution either), or is sitting on
  undecided data (boundary scan). Nothing silent remains.

So the invariant was demoted from a soundness requirement to an API *classification* —
exact vs. loose decisions (4.1) — and the loose operations were reinstated, with the
docs steering toward exact decisions when contents matter to assertions. What remains
load-bearing from that era is one scheduling rule it left behind: deterministic mode
fires ticks only after propagation has quiesced, which is precisely what gives loose
decisions their well-defined deterministic meaning. It is also the reason section 5.4's
free-running wait cannot be replaced by "just run the previous group's tick now": with
loose decisions in the language, *when* a scripted tick fires relative to fuzzed ticks
affects *what* it releases, so the placement must remain the ordinary explored choice
rather than something the script machinery forces.

### 10.7 Reading order is execution order — the cost we accepted

The group model (4.2) means a script must be written in the order things happen: you
cannot group all of tick A's decisions together and all of tick B's afterward if the
execution actually alternates A, B, A, B. Scripts written in the "wrong" order fail — at
the first offending line, with an explanation — rather than being reinterpreted.

We considered being cleverer: letting decision queues per hook fill up in any order and
matching them to executions as they become possible. That direction was rejected because
it reintroduces, in a subtler form, every pairing problem this design exists to prevent:
when queues are decoupled from execution order, a decision can apply to a different
execution than the one the author had in mind while writing it (10.3's mis-pairing, again),
and reading the test no longer tells you which execution a given line describes. The
schedule-like reading — line N of the script corresponds to step N of the execution — is
exactly what makes scripted tests reviewable, and it is what the panics can point into
when something is impossible. The cost is that authors of feedback-heavy topologies must
interleave their script the way the data actually flows; the error message's job is to
make that rewrite obvious.

### 10.8 How `auto_pause` shrank the pause mechanism instead of growing it

`auto_pause()` started as a late feature request with a modest goal: some tests want a
hook that *only ever* acts when scripted, and asking them to `pause()` after every
decision is ceremony. The obvious design was a mode flag layered on the existing pause
flag: pause immediately, and re-pause automatically whenever the hook's decision queue
drains. The stated hope was that this could be pure sugar — the engine's core rule
("buffered data with no decision is a panic") untouched.

The first implementation sketch broke that hope in a small but telling way: *who notices
the queue draining?* The decision queue is consumed by the scheduler, dylib-side; the
pause flag was conceptually owned by the handle, test-side. Re-pausing on drain meant
either the scheduler learns about auto-pause (a second mode in the core — exactly what we
wanted to avoid) or the handle polls for a drain it cannot observe. A first repair moved
the flag fully runtime-side and had the scheduler clear it whenever it dequeued a
decision — workable, but now the scheduler *mutates pause state*, and the invariant
"pause is a test-side declaration the engine merely reads" was gone.

The fix came from turning the question around: instead of a flag that fights the queue,
make the hold *an entry in the queue* — pausing is scripted, like everything else. That
reframing triggered a chain of invariant-hunting that ended up simplifying the core well
below its starting point:

- If a hold is queued behind decisions, it takes effect when it reaches the front — so
  `release(2); pause();` naturally means "release two, *then* hold," which is more
  faithful to the script-as-schedule reading than an instantaneous flag ever was.
- Can a hold ever have anything queued *behind* it? Scripting a decision implicitly
  resumes, i.e. replaces a pending hold — so no: a hold is always the last entry, and a
  hold at the front means the queue is exactly one hold. Cancellation (`resume()`)
  becomes "assert the queue is a single hold, reset it."
- Can two *decisions* ever be queued at once? No — and this was the decisive
  observation: decisions install through the async call protocol, and the first decision
  of a new group installs only after the previous group's tick execution has been
  consumed. A hook's earlier decision is always gone before its next one arrives, so
  installation can *assert* the queue is empty.
- With at most one decision and at most one hold logically after it, the "queue" is not
  a queue: it is `next_decision: Option<Decision>` plus `hold: bool`, consulted in
  order. Every operation is an assert-and-set on two fields.

Two things fell out of the collapsed model for free. First, the mid-script suspension
exemption — the rule that the boundary scan must not fire on a hook while the test is
suspended in a decision call *for that hook* — stopped being a special case: the call
protocol simply sets the hold before suspending and clears it when it installs, so "the
test is currently scripting this hook" is expressed in the same vocabulary as every other
declared buffering. Second, `auto_pause()` itself reduced to a one-line policy: the
install step leaves the hold set instead of clearing it. The scheduler cannot tell an
auto-paused hook from one whose author diligently calls `pause()` after every decision —
which is the precise sense in which the feature is sugar.

The journey is worth recording because its shape is the design's philosophy in
miniature: the feature was accepted only on the condition that it not complicate the
core, the first two designs quietly did, and the pressure of that condition produced a
model simpler than the one we started with.
