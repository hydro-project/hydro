# Simulator hooks: scripting the decisions of unsafe operators (v2)

Issue: [#1875](https://github.com/hydro-project/hydro/issues/1875)

> **Status: implemented.**

This is a design document for letting tests take manual control of the non-deterministic
decisions in a Hydro program — when a batch fires and what it contains, which snapshot of
a piece of state a request observes, in which order unordered data is consumed, and so
on — so that developers can write deterministic, readable tests for *specific*
distributed-systems scenarios, alongside the existing fuzzing and exhaustive-search modes
of the simulator.

The document is organized so that each section can be read mostly on its own. Sections
1–2 explain what we are trying to achieve and what we care about. Sections 3–4 describe
the API a developer sees. Sections 5–6 define how the simulator executes a scripted test.
Section 7 sketches the implementation, and sections 8–9 cover scope and resolved
questions. Section 10 is a discussion section: for every part of the
design where we considered and rejected alternatives — or where a simple-looking rule has
a subtle justification — there is a long-form writeup of the reasoning. The main body
(sections 3–6) describes only the final design; if a rule seems arbitrary, the discussion
section explains where it came from.

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
pub struct NonDet<H = ()> {
    hook: H,
}
```

`H` is the **hook payload** the guard can carry, and defaults to `()` so that plain
`NonDet` still means what it always did. Payload types implement `Default` ("no hook"),
and the payload itself carries the optionality: an operator's guard type is
`NonDet<Option<BatchHook<T>>>`, and a component can expose several operators through one
guard with a tuple payload such as `NonDet<(Option<BatchHook<T>>,
Option<SnapshotHook<S>>)>`.

The `nondet!` macro keeps its pre-hook forms and gains one optional trailing argument:

```rust
nondet!(/** reason */)                       // no hook attached
nondet!(/// reason
        nondet_parent)                       // forwarded justification, no hook
nondet!(/** reason */ hook = my_hook)        // attach a hook handle
nondet!(/** reason */ hook = part)           // attach a payload split off a composite guard
```

The `hook =` expression is converted with `Into`, so a raw handle can be passed where an
optional payload is expected.

A *component* opts into hookability by typing its `NonDet` parameters with the payload of
the operator each one controls, and passing each guard **directly** to its operator. The
justification lives in a `# Non-Determinism` section of the function's Rustdoc, following
the existing convention for forwarded non-determinism. Taking the counter service from
section 1:

```rust
/// A counter service that responds to read requests with the current count.
///
/// # Non-Determinism
/// - `nondet_batch`: how read requests are batched is observable in responses
/// - `nondet_snapshot`: reads may observe any version of the count
pub fn counter_service<'a>(
    increments: Stream<u64, Process<'a, Counter>>,
    get_requests: Stream<u32, Process<'a, Counter>>,
    nondet_batch: NonDet<Option<BatchHook<u32>>>,
    nondet_snapshot: NonDet<Option<SnapshotHook<u64>>>,
) -> Stream<(u32, u64), Process<'a, Counter>, Unbounded, NoOrder> {
    let current_count = increments.fold(q!(|| 0), q!(|acc, v| *acc += v));

    sliced! {
        let request_batch = use::batch(get_requests, nondet_batch);
        let count_snapshot = use::snapshot(current_count, nondet_snapshot);

        request_batch.cross_singleton(count_snapshot)
    }
}
```

Production callers are untouched (G6): `counter_service(incs, gets, nondet!(/** ... */),
nondet!(/** ... */))` still compiles, with the payload defaulting to "no hook". Only a
test that wants control passes actual handles, discharging the guard at the root with
`nondet!(/** scripted by the test */ hook = batch_hook)`.

One rule worth stating explicitly: a hook is only ever attached where the code makes the
attachment visible. Forwarding a guard through `nondet!` *without* `hook =` never
propagates a binding, even if the forwarded guard carries one. Guards are frequently
forwarded many-to-one (a single `nondet_raft` justifying four different `batch` calls),
so implicit propagation through the macro would be ambiguous — which of the four batches
would the hook control? A guard whose *type* names exactly one operator's payload is
passed to that operator directly (the signature is the visible binding); a composite
payload is split with `NonDet::take_hook()`, which removes the payload from the guard,
and each part is attached with `hook =` at the operator it controls. Section 10.10
records how this settled.

### 3.2 Hook handles are typed like the operator they control

Each kind of unsafe operator has a corresponding handle type, and the handle's type
parameters mirror the guarantees of the collection the operator consumes:

| Handle type | Controls |
|---|---|
| `BatchHook<T, O = TotalOrder, R = ExactlyOnce>` | `Stream::batch`, `use::batch` |
| `SnapshotHook<T>` | `Singleton::snapshot`, `use::snapshot` |
| `OrderingHook<T, B = Unbounded>` | `assume_ordering::<TotalOrder>`, and commutativity proofs on `fold`/`reduce` (section 4.5) |

(Keyed operators are follow-on work, see section 8.)

The operator's signature is (for example)
`nondet: NonDet<Option<BatchHook<T, O, R>>>`, which is what makes hooking type-safe end
to end: attaching a `BatchHook<u32>` to a batch of `Stream<String>` is a compile error,
attaching a `SnapshotHook` to a `batch` is a compile error — and, more interestingly, the
*set of decisions the handle offers* is derived from the collection's type. A batch hook
for a totally-ordered stream releases prefixes (there is nothing to reorder); a hook for
a `NoOrder` stream selects contents. An `OrderingHook<T, Unbounded>` (top-level) releases
one element at a time; an `OrderingHook<T, Bounded>` (inside a tick, where the input is
the tick's complete batch) orders a whole batch at once. The type system guarantees a
test can only script behaviors the program's types say are possible. Section 4.1 lists
the decisions themselves.

### 3.3 Creating handles, and structs of hooks

A handle must exist *before* the program under test is constructed, because it is passed
into the program as part of a `NonDet` argument. Handles are created from the
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
its testing interface. To keep that manageable, the `SimHook` trait lets a struct of
handles be created in one call (`flow.sim_hook()` is generic over the trait; individual
handle types implement it too). Fields are `Option`s, so the struct doubles as a
composite hook payload — its `Default` ("no hooks") is what a plain guard carries, while
`flow.sim_hook()` fills in every handle:

```rust
#[derive(Clone, Copy, Default)]
pub struct CounterHooks {
    pub batch: Option<BatchHook<u32>>,
    pub snapshot: Option<SnapshotHook<u64>>,
}

impl SimHook for CounterHooks {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        CounterHooks {
            batch: SimHook::create(next_id),
            snapshot: SimHook::create(next_id),
        }
    }
}

let hooks: CounterHooks = flow.sim_hook(); // creates every field
```

Such structs nest (a Paxos struct can contain a leader-election struct), and since
handles are `Copy` a test can pass the struct around or destructure it freely.

Misuse is caught early:

- Binding the same handle to two different operators is an error at flow build time,
  reported with both operator locations.
- Creating a handle and never binding it is allowed (a struct of hooks may be only
  partially used by a particular program configuration), but scripting a decision on an
  unbound handle panics at that call.
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
totally-ordered stream the order is fixed, so a decision names a prefix; on an unordered
stream the contents are also a choice:

```rust
batch_hook.release(3).await;              // (TotalOrder) the next 3 buffered elements
batch_hook.release_values([7, 8]).await;  // exactly these values (assert + release);
                                          // ordered: the next prefix must match;
                                          // NoOrder: a multiset of buffered values
batch_hook.release_all().await;           // everything that has arrived
batch_hook.release_empty().await;         // the next batch is empty; hold everything
```

`release_values` is the value-named form: on an ordered stream a mismatching available
prefix value panics immediately, while a matching but incomplete prefix waits for the
remaining values to arrive; on an unordered stream the values are matched independently
of incidental buffer order. There is deliberately no positional selection
(`release_at([0, 2])`-style): naming values gives scripts the same self-checking quality
as `reveal(value)` below. (Redelivery decisions for `AtLeastOnce` streams are follow-on
work, see section 8.)

For a **snapshot hook**, the buffered inputs are the successive versions of a piece of
state, and a decision picks which version the next tick execution observes:

```rust
snapshot_hook.reveal(1u64).await;    // reveal the version equal to 1 (assert + release)
snapshot_hook.reveal_next().await;   // advance to the next buffered version
snapshot_hook.reveal_latest().await; // reveal the newest version that has arrived
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

For an **ordering hook**, the decisions depend on where the operator sits. A top-level
`assume_ordering` (outside any tick) releases one named element per decision; an in-tick
`assume_ordering` consumes its complete tick-local input in one scripted order:

```rust
ordering.next(2).await;         // (top-level) release the buffered element equal to 2
ordering.order([2, 3, 1]).await; // (in-tick) a permutation of the tick's whole input
```

Why one element at a time at the top level: between two releases, ticks and network
feedback can interleave with the remaining buffered input, so a script can express causal
patterns like "the reply to the first message overtakes the second message". A
whole-buffer permutation decision would flatten those interleavings away. Inside a tick
the input is already a closed batch, so a single permutation is exactly the decision
space. Section 4.5 covers how ordering hooks also attach to commutativity proofs.

**Exact and schedule-dependent decisions.** Most decisions above are *exact*: what they
release is fully determined by the decision itself plus a prefix of the buffered input —
reading the script tells you precisely what the batch contains or which version is
revealed, independent of any timing. Two decisions are deliberately schedule-dependent:
`release_all()` and `reveal_latest()` release "whatever has arrived by the time the tick
fires". In deterministic mode that set is still fully determined (section 6 explains
why); under fuzzing it co-varies with the schedule being explored, which means such a
decision deliberately leaves part of the outcome to the fuzzer even on a scripted hook.
They are convenient for "flush whatever is there" and "assert against the current state"
phases; prefer exact decisions whenever the released contents matter to the assertion
(see 10.6 for how this classification earned its place).

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

**Incomplete or impossible groups are rejected loudly.** If the test starts scripting a
new group while the previous group's tick *cannot run*, the failure surfaces through
whichever safeguard the incomplete state trips. If the previous group left one of its
tick's hooks with buffered data and no decision — say it released a batch but forgot to
give the snapshot hook a decision while new state versions are buffered — the boundary
scan (rule 2 in 5.2) reports that hook directly:

```text
scripted hook has buffered input but no decision:
--> src/counter.rs:45:31
 |     let count_snapshot = use::snapshot(current_count, nondet!(
 |                          ^ 2 newer versions buffered
help: script a decision (e.g. `hook.reveal(..)`) or call `hook.pause()` if buffering is intended
```

If instead the previous group's decision can never be honored (its data will never
arrive, or the dataflow cannot satisfy the order the script demands), the suspended call
waiting on the new group panics once the simulation runs out of work:

```text
a previously scripted decision can never be satisfied
--> tests/counter.rs:88:9      <- the suspended decision on the new group
```

Together these catch two very different mistakes at their source (G3). The first is
*mis-pairing*: without the group model, a decision written for one execution could
silently apply to a later one (the batch pairs with a stale snapshot; the reveal drifts
to the next execution). The second is *causal deadlock*: a script whose decisions are
written in an order the dataflow cannot satisfy — for example, grouping all of tick A's
decisions before tick B's when A's second batch can only be released after B has
processed A's first — fails at the first out-of-order line, with the fix being "write the
script in execution order" rather than a mysterious hang.

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

Two more ergonomic forms round this out. `pause_while(fut)` pauses exactly for the
duration of a future (resuming even on panic), so a bracketed buffering phase cannot leak
a paused hook. And `pause_until(predicate)` pauses and returns a future that resolves
once the hook's pending-input status satisfies the predicate — a synchronization point
for scripts where the right decision is not knowable upfront (for instance, when the
number of elements the program will produce into the hook is not statically known and the
test wants to inspect other outputs before deciding). The shorthands `pause_until_count(n)`
(batches and top-level orderings) and `pause_until_versions(n)` (snapshots) cover the
common cases:

```rust
batch_hook.pause_until_count(3).await; // wait until 3 requests have reached the batch
// ... inspect other outputs, decide how this step should respond ...
batch_hook.release(3).await;
```

After `pause_until` resolves, the hook is unpaused; if the test then wanders off to
await something else without scripting a decision, the ordinary missing-decision error
applies. The pause family is for *declared* waiting, not for switching the safety net
off. Like an output await, a `pause_until` wait is a script barrier: it does not resolve
while an earlier decision group is unconsumed, and if that group is stuck, the wait
reports the stuck group as the root cause instead of blaming its own predicate.

One boundary of the pause family is important enough to state in the main body: **a
decision that is waiting for its turn does not pause its hook.** If the test scripts a
decision for hook B while an earlier group is still outstanding, and B *already* holds
buffered input, that input is buffered across scheduling boundaries with neither a
decision installed nor a declared pause — and the forgotten-hook error fires. The queued
decision describes what B will do *when its turn comes*; it says nothing about the data
sitting at B *until* then, and silently exempting it would prune schedules without a
trace (the full discussion is in 10.9). When pre-feeding a hook behind a waiting group is
intended, declare it: `pause()` spans the wait and is released only when the queued
decision actually installs, which is strictly after the previous group has executed.

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

### 4.5 Ordering hooks and commutativity proofs

`OrderingHook` extends scripting beyond `assume_ordering` to the other place where the
simulator explores orderings: **commutativity proofs**. When a developer passes
`commutative = manual_proof!(...)` to `fold` or `reduce`, they are asserting that the
combinator tolerates any consumption order. The simulator deliberately does not trust
that assertion — it explores orders to check the claim — and a hook lets a test *script*
those orders instead. `ManualProof` is generic over a hook payload exactly like `NonDet`,
and the `manual_proof!` macro accepts the same `hook =` argument:

```rust
b.fold(
    q!(|| Vec::new()),
    q!(
        |acc, v| acc.push(v),
        commutative = manual_proof!(
            /// scripted by the test
            hook = ordering
        )
    ),
)
```

Where the hook binds determines its decision shape, mirroring `assume_ordering`:

- A fold **inside a tick** is simulated through an inline shuffle of its batch; the hook
  scripts that shuffle with `order(values)`, and the decision joins the same group as the
  batch decisions feeding the tick.
- A fold **outside any tick** normally benefits from a search-space optimization (the
  simulator explores subsets and permutations of input wholesale, snapshotting
  accumulator states). Binding a hook *disables* that optimization for the fold: each
  `next(value)` decision feeds exactly one named element into the fold, making
  intermediate accumulator states observable at exactly the script's release points —
  typically paired with a `SnapshotHook` downstream that reveals the accumulator after
  each step.

The important property in both cases: a "commutative" fold whose combinator is *not*
actually commutative will produce visibly order-dependent output under a scripted hook
(and under autonomous exploration), because the proof never exempts the operator from
simulation. Section 10.12 discusses why proofs carry hooks rather than being trusted.

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
making progress, then pick one ready action (a fuzzed choice) and execute it. An action
is either a tick execution (releasing each of the tick's hooks' decisions into it) or a
**top-level observation** — an operator like a top-level `assume_ordering` or an
unbounded `fold` releasing buffered input outside any tick. Each top-level hook is its
own observation, even when several live at the same location: unlike a tick's hooks,
which one atomic execution consumes together, co-located top-level hooks are causally
independent, and a picked observation always releases data (staying silent
is expressed by the scheduler picking something else). Observations are ordinary
scheduler peers of ticks; nothing gives either priority (see 10.11). When nothing can make progress at all and the test
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
(Scripted *top-level observations* follow the same rule with a simpler shape: a group
targeting an observation holds exactly one decision, and the observation fires only when
that decision is honorable.)

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
test to write `release_empty()` or `keep()` for every such hook at every execution would
be pure
noise — there is no decision being made, because there is nothing to decide.

The boundary between "nothing to decide" and "something to decide" is exactly the rule 2
scan: the moment the hook's buffer holds real data (or newer versions), the implicit
behavior is off the table and the absence of a decision is an error. A scripted hook
therefore never silently holds back data and never silently reveals a stale version —
the only implicit behaviors are the ones that were forced. Note that the implicit path
exists only for *tick* hooks, because only a tick can be forced to run by hooks other
than the one in question; a top-level observation consists of exactly one hook, so it
can never run without a scripted decision, and its implicit path is an internal
invariant violation (the simulator aborts if it is ever reached). Whether this narrow
implicit rule can ever interact badly with scheduling was a major focus of the design
review; the full analysis (it cannot, and why) is in sections 10.3 and 10.4.

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
- **Even when its decisions are honorable, firing order matters for schedule-dependent
  decisions.** Since `release_all()` and `reveal_latest()` release "whatever has
  arrived", *when* the scripted tick fires relative to the fuzzed ticks around it changes
  what it releases. If the scheduler forced the scripted tick to fire at the earliest
  possible moment, the schedules where a fuzzed tick delivers more data first would be
  systematically unexplored — a silent bias, invisible to the developer.

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

Two deliberate exceptions keep this statement honest. Schedule-dependent decisions
(`release_all()`, `reveal_latest()`) leave their contents co-varying with the fuzzed
schedule around them — no new entropy is drawn, but the script no longer determines the
released set (see 4.1). And a *paused* hook's behavior depends on how many times its tick
fires while paused, which in fuzz mode is a fuzzed quantity. Both are explicit,
per-call/per-declaration choices visible in the test.

## 6. Deterministic mode

`flow.sim().deterministic(async || { ... })` runs the test body against exactly one
execution of the program, with no fuzzer involved anywhere — the promise is "if it passes
once, it passes always, on every machine." The implementation enforces this structurally:
deterministic mode installs no entropy scope at all, so any code path that would draw
randomness fails immediately rather than silently sampling. Everything in sections 4–5
applies unchanged (G4); this section explains what the absence of a fuzzer means and the
guarantees that follow.

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

### 6.2 The invariant: at most one action is ever runnable

Deterministic mode does not need a tie-breaking policy for "which action runs next",
because the situation where two actions (tick executions or top-level observations) are
simultaneously runnable cannot arise. This is worth spelling out, since the whole mode
rests on it — and the implementation asserts it on every step. Consider any tick and ask
what could make it runnable (the same reasoning applies to observations):

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
has fully quiesced. This is what gives the schedule-dependent decisions (`release_all()`,
`reveal_latest()`) a well-defined meaning in deterministic mode — "everything that has
arrived" is precisely the set of data causally available at this point in the script,
which is a deterministic set. Such a decision under deterministic execution is exactly
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
`NonDet { hook: h.into() }`. Operators that accept a hookable guard (`batch`, `snapshot`,
`assume_ordering`, `fold`/`reduce` via `ManualProof`, and their atomic/`sliced!`
variants) take the payload out of the guard and copy the handle's ID into a new
`sim_hook_id: Option<usize>` field on their IR node metadata, next to the backtrace
metadata already used for rendering hook locations in logs. Non-simulator backends ignore
the field. Hook IDs are a counter in the flow state, allocated by `flow.sim_hook()`
exactly like external port IDs are for `sim_input`.

**Runtime wiring.** The sim builder, on seeing `sim_hook_id`, wraps the operator's
normal runtime hook object (`StreamHook`, `SingletonHook`, ...) in a scripted-mode
shell — `Scripted<H>`, generic over a small `ScriptableHook` trait (`type Decision`,
`is_honorable`, `apply`, `implicit`), plus per-kind classification traits
(`ScriptableTickInputHook::decision_triggers_tick`, and a marker for observation kinds) —
registered in a per-instance registry
threaded through the dylib entry point (the same pattern as the
`external_in`/`external_out` queues). Test-side handles resolve their ID through the
task-local sim context, the same way `SimSender::send` resolves its port, so handles work
across fuzz iterations without per-instance setup. Decisions are bincode-serialized
across the dylib boundary and compared after deserialization — mirroring `sim_input` —
which puts `Serialize` / `DeserializeOwned + PartialEq` bounds on value-carrying
decisions. One boundary rule shapes the error paths: panics cannot unwind across the
dylib boundary (Rust aborts on foreign exceptions), so anything that can fail inside
generated code — a mismatched `release_values`, a missing in-tick ordering decision —
returns an error *value* to the host side, which renders and panics there with proper
attribution.

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

1. Ask the script coordinator to schedule the decision. If it belongs to the current
   group (or opens a new one with no group outstanding), it installs immediately —
   without yielding, so writing out a group is atomic. Otherwise the call returns the
   decision to the future, which suspends and retries after every scheduler step.
2. Installation asserts `next_decision.is_none()`, sets `hold = false` (unless the hook
   is in `auto_pause` mode), and stores the decision.

Note what step 1 does *not* do: a decision waiting for its turn leaves its hook's state
completely untouched — in particular, it does not pause the hook (10.9 discusses why it
must not). The natural sequential pattern works without any such shield because
installation happens at the first poll after the previous group executes — before the
scheduler reaches its next boundary — so a hook whose data arrives *from* that execution
is scripted before any scan can see it.

`pause()` sets the bool (taking effect after any pending decision, per the field order
above); `resume()` clears it; `pause_until(..)` sets it and returns a future that clears
it when the predicate is satisfied (barriering on any unconsumed group first, like an
output await); `auto_pause()` just makes installation leave the bool set. None of these
touch the scheduler.

**The rest of `Scripted<H>` behind the `RuntimeHook` trait family** (the erased traits
the scheduler drives all runtime hook objects through — `RuntimeHook` is the shared
base carrying `has_pending_input()` and the choice question
`only_one_possible_decision()`; `TickInputHook` and `ObservationHook` refine it with
each kind's autonomous decision surface, which scripted hooks simply do not implement;
`ScriptedTickInputHook` / `ScriptedObservationHook` are the scripted mirrors of
that split, blanket-implemented on `Scripted<H>`):

- `has_pending_input()` and `only_one_possible_decision()` delegate to the wrapped
  hook: scripting changes who answers a choice, not whether one exists. The boundary
  scan asks the core's `has_pending_input()`.
- `can_trigger_tick()` (respectively `can_fire()` for observations) is true when the
  queued decision is honorable and releases data
  (`ScriptableTickInputHook::decision_triggers_tick`; observation decisions always
  release). Pending input alone never makes a scripted hook trigger; the boundary scan
  is the mechanism that reacts to it.
- A `blocks_tick()` accessor is true while `next_decision` is present but not yet
  honorable; `SimTick::can_run` requires `!blocks_tick()` for every hook, implementing
  rule 1.
- A `boundary_check()` reports the forgotten state (`next_decision.is_none() && !hold &&
  has_pending_input()`), consulted by the boundary scan.
- There is no autonomous path at all: `autonomous_decision` lives on the unscripted
  refinements (`TickInputHook` / `ObservationHook`), which `Scripted<H>` does not
  implement, so the entropy driver is structurally unreachable — no runtime guard
  needed. The only-possibility implicit behavior (5.3) is produced through
  `ScriptableHook::implicit` when co-located hooks force a tick to run.

**Scheduler changes** (in `LaunchedSim::step` and around it):

- The **script coordinator** tracks at most *one* outstanding group:
  `current: Option<CurrentGroup>`, where a group records its target (a tick, or a
  top-level observation) and its member decisions. The group is *sealed* at the start of
  the next scheduler step — after that, scripting the same target again must wait. There
  is no queue of groups: one outstanding group mirrors the one-decision-per-hook
  invariant (see the state model above).
- The **boundary scan**: at the existing point where the async dataflows report no
  progress and the scheduler moves to action selection, iterate scripted hooks and panic
  on any whose `boundary_check()` reports the forgotten state. No entropy consumed.
- **Action selection**: the current group's target, once all its decisions are honorable,
  competes as an *ordinary candidate* alongside every unhooked ready tick and
  observation — it gets no priority (10.11 explains why it must not). Consuming the group
  (its target executed) wakes the suspended decision future; the biased loop then polls
  the test body before taking another step, preserving the clean-resumption property
  (4.4, 5.4).
- **Quiescence checks**: at `wait_for_resume`, scan for queued decisions that can never
  be honored and unresolved `pause_until` waits; raise these through whichever
  script/await future the test is suspended in, using the same caller-tracking technique
  as the existing assertion futures, so reports carry test line numbers.
- **Output-await barriers**: the quiescence-aware receiver streams additionally wait for
  "all scripted decisions consumed" before yielding elements, and panic instead of
  yielding end-of-stream when quiescence is dirty.
- **Deterministic mode** drops the fuzzed action choice (at most one candidate exists,
  6.2, asserted on every step); everything else — the scan, the checks, the biased
  loop — is shared code.

**Logging.** Scripted releases reuse the existing per-release log lines, tagged as
scripted, so mixed tests show fuzzer-chosen and scripted decisions uniformly:

```text
* --> src/counter.rs:42:31
*  |         let request_batch = use::batch(get_requests, nondet!(
*  |                             ^ releasing items (scripted): [7]
```

## 8. Scope

**Implemented**: `NonDet<H>` and the `nondet!` `hook =` syntax, plus the analogous
`ManualProof<H>` and `manual_proof!` for commutativity proofs; `BatchHook`,
`SnapshotHook`, and `OrderingHook` (spanning `batch`, `snapshot`, non-keyed
`assume_ordering::<TotalOrder>`, and `fold`/`reduce` commutativity proofs, including the
atomic and `sliced!` forms); the decision APIs of section 4.1; the script/group model
with its panics; the pause family including `auto_pause` and `pause_until`;
`flow.sim_hook()` and the `SimHook` trait; and deterministic mode.

Follow-ons, roughly in priority order:

- **Keyed hooks** (`KeyedBatchHook`, `KeyedSnapshotHook`, and keyed ordering): decisions
  take a key as their first argument (`keyed_batch.release(&key, 2).await`), with per-key
  semantics otherwise identical.
- **Retry hooks.** No autonomous exploration exists for retries at all (`AtLeastOnce`
  duplication is unbounded), so hooks are the only way these operators can get coverage:
  redelivery decisions on `AtLeastOnce` batch hooks (a released element is retained so a
  later batch can deliver it again), and a handle for top-level `assume_retries` (e.g.
  `retries_hook.duplicate_next(2).await`).
- **Ordering hooks for the remaining ordering operators** (`merge_ordered`,
  `entries_partially_ordered`), reusing the one-at-a-time `next(value)` shape.
- **Network-level hooks**: message drop and delivery-order control on specific channels,
  which would subsume many uses of ordering hooks at the transport level.
- **A derive macro for `SimHook`**, replacing the (small) manual impl for structs of
  handles.

## 9. Resolved questions

Questions that were open during the design phase, and how they settled:

1. **`nondet!` argument order**: `hook =` is the trailing argument, matching "optional
   trailing argument" intuition.
2. **Scripting inside `fuzz` bodies**: fully allowed — handles resolve per-instance
   automatically, and this is precisely how "pin one, explore the rest" (G5) is written.
3. **Naming**: `sim_hook`, `BatchHook`, `pause`, and `reveal_latest` all kept. The
   handle-creation trait is `SimHook` (with method `create`); the internal erased trait
   the scheduler drives is `RuntimeHook`, so the public name stays unambiguous.
4. **Group boundaries at awaits**: yes — output awaits are barriers that wait for all
   scripted decisions to be consumed, so decisions after an await always start a new
   group. No extra rule was needed.
5. **`keep_expecting(v)`** (an assert-flavored `keep()`): deferred. `reveal(v)` covers
   most self-checking needs, since re-revealing an expected value is expressible as
   `reveal(v)` against a hook whose buffer has not advanced.

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
  a schedule-dependent decision is exactly as reproducible as an exact one.
- In fuzz and exhaustive mode, its contents vary with the schedule — but
  varying with the explored schedule is what fuzzing *is*. Every tick that could have
  contributed data either is concurrently ready (both orders explored), cannot yet run
  (so it could not have contributed in any real execution either), or is sitting on
  undecided data (boundary scan). Nothing silent remains.

So the invariant was demoted from a soundness requirement to an API *classification* —
exact vs. schedule-dependent decisions (4.1) — and the operations were reinstated, with
the docs steering toward exact decisions when contents matter to assertions. (For a
while the classification carried the label "loose decisions"; the label was dropped once
the soundness analysis above settled — these decisions behave like every other decision
and need no warning-flavored vocabulary, only a description of what they release.) What
remains load-bearing from that era is one scheduling rule it left behind: deterministic
mode fires ticks only after propagation has quiesced, which is precisely what gives
schedule-dependent decisions their well-defined deterministic meaning. It is also the
reason section 5.4's free-running wait cannot be replaced by "just run the previous
group's tick now": with such decisions in the language, *when* a scripted tick fires
relative to fuzzed ticks affects *what* it releases, so the placement must remain the
ordinary explored choice rather than something the script machinery forces.

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
  `release(2).await; pause();` naturally means "release two, *then* hold," which is more
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

Two things fell out of the collapsed model for free. First, a mid-script suspension
exemption seemed to stop being a special case: the call protocol could simply set the
hold before suspending and clear it when it installs, expressing "the test is currently
scripting this hook" in the same vocabulary as every other declared buffering. (This
particular free lunch turned out to be unsound and was later removed — section 10.9.)
Second, `auto_pause()` itself reduced to a one-line policy: the install step leaves the
hold set instead of clearing it. The scheduler cannot tell an auto-paused hook from one
whose author diligently calls `pause()` after every decision — which is the precise sense
in which the feature is sugar.

The journey is worth recording because its shape is the design's philosophy in
miniature: the feature was accepted only on the condition that it not complicate the
core, the first two designs quietly did, and the pressure of that condition produced a
model simpler than the one we started with.

### 10.9 A queued decision is not a pause: removing the wait-path hold

The decision-call protocol once had a step that 10.8 celebrated as a special case
eliminated: when a decision call had to suspend because an earlier group was still
outstanding, it set the hook's hold for the duration of the wait. The reasoning was
ergonomic — "the test is mid-script for this hook, so the boundary scan should not
bother it" — and it made the mid-script exemption fall out of the ordinary hold
vocabulary. It survived until a failing test was written against it deliberately.

The scenario that kills it: tick A's group is outstanding, and the test scripts a
decision for tick B's hook — while B *already* holds buffered input. The queued decision
describes what B will do *when its turn comes*, which by script order is after A. But
B's input was present at scheduling boundaries *before* A ran, and at every one of those
boundaries the real system could have fired B. The wait-path hold silently exempted
exactly that input from the boundary scan: no decision, no declared pause, and yet no
panic — the precise combination the whole of 10.1 exists to forbid.

What makes this consequential rather than pedantic is the exploration it prunes. Consider
the case where tick A is *partially unscripted* — say a fuzzed batch on A is fed by a
network cycle out of tick B. Scripting A's group before B's forces A to execute first,
so A's fuzzed hook can never observe B's cycled-back value; the executions where it does
are simply absent from the search space. With the hold in place, that pruning left no
trace: every instance passed, exploring fewer schedules than the developer had any way to
notice. This is G1's "subtler failure" (10.1) — not a wrong answer, but a silently
narrower question.

The resolution is that a waiting decision leaves its hook's state completely untouched.
If pre-fed input sits at a hook whose decision is queued behind an outstanding group, the
forgotten-hook error fires, and the developer must resolve the conflict explicitly, in
the test, in one of two visible ways:

- **`pause()` the hook** — declaring that its input intentionally waits behind the
  outstanding group, and accepting (visibly) that the earlier firings are out of scope
  for this test; or
- **reorder the script** — script B's group first, so that exploration can cover the
  schedules where B acts before A.

Removing the hold cost nothing for the natural sequential pattern (G2), which is why the
hold was never actually load-bearing for correct tests: when the data a decision responds
to is *produced by the previous group's execution*, it arrives while the test is
suspended, and the waiting decision installs at the first poll after that execution —
before the scheduler reaches its next boundary scan. A shield is only ever observable
when data was buffered *before* the wait began, and that is exactly the case that must be
confronted rather than shielded.

Two invariants came out of this discussion and are now load-bearing:

- **No implicit holds, anywhere.** Holds are set only by the pause family — every hold in
  the system corresponds to a `pause`-flavored line in the test. (Implicit *releases*
  still exist — installing a decision resumes a paused hook — and every one of them errs
  on the loud side.)
- **A pause spanning a waiting group is released only when the queued decision
  *installs*** — strictly after the previous group has executed — not when the decision
  is first scripted. Releasing at script time would re-open the gap this section closes,
  one boundary earlier; a test pins the install-time behavior specifically.

### 10.10 Forwarding hooks through components: types over ceremony

How does a hook travel from a test, through a component's signature, to the operator it
controls? The design went through three shapes, and the differences are instructive.

The first shape let `hook =` accept another guard: a component holding
`nondet_batch: NonDet<BatchHook<u32>>` would write `nondet!(/** reason */ hook =
nondet_batch)` at its `batch` call, and the payload (possibly absent) passed through.
This conflated two different things — justifying non-determinism and routing a hook —
and it made `hook =` polymorphic between handles and guards for no user-visible benefit.

The second shape separated them: the component calls `nondet_batch.take_hook()` to
extract the payload, then attaches it with `nondet!(/** reason */ hook = payload)`. This
is honest about the two roles, and `take_hook` (which removes the payload rather than
copying it) enforces that a binding happens at most once. But in the common case — one
parameter controlling one operator — it produced pure ceremony: a `take_hook` line plus a
re-wrap whose doc comment has nothing to say (`/** scripted by the test */` conveys no
information at the operator; the *caller* decided that).

The final shape lets the type system carry the routing. An operator's guard parameter is
typed with its exact payload (`NonDet<Option<BatchHook<u32>>>`); a component that
declares a parameter of that type passes it **directly** to the operator. The
justification obligation is discharged where the existing `NonDet` conventions already
put it — a `# Non-Determinism` section in the component's Rustdoc — and the signature
itself is the visible binding: a reader sees `nondet_batch` flow into `use::batch` the
same way they see any other argument. `take_hook` remains for the genuinely plural case
(a composite tuple payload split across several operators), where per-part `hook =`
attachments are exactly as visible as they need to be.

The rule that survived all three shapes unchanged: forwarding a guard through `nondet!`
*without* `hook =` never propagates a binding. Many-to-one forwarding (one `nondet_raft`
justifying four batches) stays unambiguous, and no hook is ever bound at an operator the
reader cannot see it reaching.

### 10.11 Ordering hooks: one element at a time, and no scheduler priority

Ordering decisions raised two questions the batch/snapshot hooks never had to face.

**What is a decision?** For an in-tick `assume_ordering` the answer is easy: the input is
the tick's closed batch, so one decision orders the whole batch (`order(values)`), and it
joins the tick's group like any other decision. For a *top-level* `assume_ordering` the
tempting answer is the same — script a permutation of the buffer — but that flattens away
exactly the scenarios ordering tests are written for. Between two releases of a top-level
ordering, ticks can fire and network feedback can deliver: "the reply to message 1
overtakes message 2" requires message 1 to be released, *processed*, its reply delivered,
and only then the reply released ahead of message 2. So a top-level decision releases
exactly **one named element** (`next(value)`), and the schedule between two `next` calls
remains ordinary explored (or scripted) simulation. A `next(value)` may even name a value
that does not exist yet — one that an unhooked tick will produce by cycling data through
the network — and the observation simply waits for it, like any other not-yet-honorable
decision.

**When does a scripted observation fire?** Top-level observations are scheduler actions
in their own right, and the tempting design was to give a pending scripted observation
priority — or to let it block ticks until served. Both bias the search (G1, G5): a tick
must be able to fire *between* two scripted releases, and must be able to beat buffered
observation input, because the real system exhibits both interleavings. So a scripted
observation competes as an ordinary candidate, exactly like a scripted tick: exhaustive
mode explores every legal placement of the observation among the ticks around it. The
forgotten-hook protection is unchanged by this freedom — an ordering hook with buffered
input and no decision is confronted at the boundary scan, and a test that wants ticks to
run past a buffered ordering hook without deciding for it declares that with `pause()`
(or `pause_until_count(n)` when it is waiting for feedback to accumulate).

### 10.12 Commutativity proofs are hooked, not trusted

`fold` and `reduce` accept commutativity assertions (`commutative = manual_proof!(...)`),
and the question was what the simulator should do with a *manual* proof. One position:
the developer asserted commutativity, so the simulator may consume input in one arbitrary
order — exploring orders is wasted work on an operator whose output provably cannot
depend on them. The design takes the opposite position: a manual proof is an unverified
claim, and an incorrect one is precisely the bug class a simulator should catch. Proofs
gate the *type system* (which APIs are available); they do not exempt anything from
*simulation*. A fold with a manual proof gets its consumption orders explored exactly
like an `assume_ordering` would be — and, consequently, it can be scripted.

That decided where the hook attaches: to the proof itself. `ManualProof<H>` is generic
over a hook payload exactly like `NonDet<H>`, and `manual_proof!` accepts the same
`hook =` argument — the proof is the artifact claiming order-insensitivity, so it is the
natural carrier for the handle that tests the claim, and components expose it the same
way they expose hookable guards (a `ManualProof<Option<OrderingHook<T, B>>>` parameter,
documented and passed through). The canonical demonstration test binds a hook to a fold
whose combinator is `Vec::push` behind a (false) commutativity proof: the scripted order
comes back out in the accumulated `Vec`, visible proof that the proof was not trusted.

One implementation consequence is worth its own note. A top-level fold is normally
simulated through a search-space optimization that explores subsets and permutations of
its input wholesale, snapshotting accumulator states — efficient, but incompatible with a
script that wants to name individual steps. Binding a hook therefore switches the fold
off that optimization and onto the ordinary one-element-per-decision path (`next(value)`),
which is what makes intermediate accumulator states observable at exactly the script's
release points. The optimization is a fidelity-preserving accelerant for *autonomous*
exploration; scripting asks a different question ("walk this exact path"), and the
switch between them is per-operator and visible in the test.
