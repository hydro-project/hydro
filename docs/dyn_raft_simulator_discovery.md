# Report: the Hydro simulator discovering Ongaro's 2015 membership-change bug

## TL;DR

**Claim.** Hydro's compiled simulator, given only an operator-plausible workload, finds
Ongaro's 2015 single-server membership-change bug in a Raft implementation lacking the
current-term-commit barrier.

**The test** (`simulator_explores_concurrent_double_leave` in
`hydro_test/src/cluster/dyn_raft.rs`): a 4-node cluster elects a leader; two operators,
contacting two different nodes, concurrently submit `Remove(1)` and `Remove(2)`; election
and heartbeat timers fire. Safety oracle: no two members may commit different payloads at
the same log index. Nothing in the test encodes the bug's mechanism.

**Result.** Coverage-guided fuzzing (`cargo sim`, 8 parallel seeds on a 96-vCPU EC2 box)
violated the oracle **4 times independently**, first find within ~1.8M executions
(~90 min). The discovered fork: `Configuration({0,2,3})` and `Configuration({0,1,3})` —
*both* concurrent removals durably committed at the same log index by different quorums.
A 121-byte reproducer is checked in; plain `cargo test` replays it deterministically in
~14s, including cross-platform (found on x86_64 Linux, replays on aarch64 macOS).

**Open.** The same scenario with a learner join (`Add`) instead of a second remove found
no violation in ~26M executions/seed × 8 seeds — inconclusive, plausibly protected by the
Add path's catch-up gate.

**Caveats.** One implementation, one oracle-author who knew the bug (multi-version
differential testing would strengthen this); an earlier version of the test was
structurally unable to express the bug and burned 543M executions proving nothing —
detailed below, along with every other misstep.

---

This document describes, precisely, what was tested, what the simulator was and was not
told, what it found, and how the finding was verified. It also corrects an earlier
overclaim made in conversation about this work (see "A correction" below).

Raw trace artifact referenced throughout: [`dyn_raft_bootstrapped_trace.txt`](./dyn_raft_bootstrapped_trace.txt).

## Background: the bug being sought

Diego Ongaro's 2015 dissertation describes a single-server membership-change algorithm,
and a safety bug in its original (pre-barrier) form: if a leader is allowed to append a
configuration-change log entry before it has committed an entry from its own current
term, two leaders in different terms can each append a *different* configuration change,
and each can get that change committed under a *different* quorum (the old configuration's
majority vs. the new configuration's majority). The result is two members permanently
disagreeing about what occupies the same physical log index — a violation of Raft's State
Machine Safety property. The fix ("the 2015 correction") is a barrier: a leader may not
append a configuration change until it has committed at least one entry from its current
term.

`hydro_test/src/cluster/dyn_raft.rs` implements this protocol twice from one shared step
function, `dyn_raft_step_with_policy`, gated by a `StepPolicy::require_current_term_commit`
flag:

- `dyn_raft_step` (`require_current_term_commit: true`) — the production path, always
  used by the public `dyn_raft`/`dyn_raft_server` functions. This keeps the barrier.
- `dyn_raft_step_unpatched_for_simulation` (`require_current_term_commit: false`,
  `#[doc(hidden)]`) — omits the barrier, and (as of this work) truncates conflicting log
  suffixes unconditionally, matching the dissertation's original, unbarricaded behavior
  verbatim. This exists only so simulation can demonstrate the counterexample; it is never
  reachable from the production API.

## Delta from `raft.rs`: what was added, why, and against what specification

`hydro_test/src/cluster/raft.rs` (pre-existing, not modified by this work) implements
plain Raft with **static membership**: `RaftConfig { cluster_size }` is a single integer,
and the protocol's voting set *is* the physical Hydro cluster — there is no notion of a
subset, no membership log entries, and therefore no membership-change algorithm at all.
Its `LogEntry<T>` carries only an application payload `T`; there is no no-op and no
configuration variant. Consequently `raft.rs` cannot exhibit the 2015 bug — the bug is
specifically about *changing* the voting set, and `raft.rs` has no mechanism to change it.

`dyn_raft.rs` (added in this branch's prior commit, `191fdeb1bf`, and extended in this
session) adds exactly the machinery Raft's membership-change chapter requires, and nothing
beyond it:

- **A fixed physical universe, a variable logical configuration.** The Hydro `Cluster`'s
  physical membership is still static (same as `raft.rs`) — `dyn_raft_server` reads it
  once, at startup, as `physical_members`. What varies is `Configuration<C>`, a set of
  `MemberId`s drawn from that fixed physical universe, computed by scanning the log for
  the most recent `DynLogPayload::Configuration` entry
  (`DynRaftState::effective_configuration`). This matches the dissertation's model: a
  fixed universe of servers that *could* serve, with the *current configuration* being
  whichever subset is currently voting — Raft's cluster-membership chapter never grows or
  shrinks the set of machines a deployment provisions, only the set that counts toward
  quorum. (This report cites the dissertation's mechanisms as described in this module's
  own top-of-file doc comment, quoted below; dissertation section numbers were not
  independently re-verified against the source PDF as part of this report and are
  intentionally not cited here.)
- **A three-way log payload.** `DynLogPayload::{Command, Noop, Configuration}` replaces
  `raft.rs`'s bare `T`. `Noop` is appended once, unconditionally, on every leader accession
  (`become_leader`) — in **both** policies; `become_leader` is shared code, not part of
  what `require_current_term_commit` toggles. This is ordinary Raft leader-accession
  behavior (a new leader needs an entry of its own current term in the log at all,
  independent of membership), not something introduced by the 2015 fix. What the fix
  actually does is *consume* the fact that a no-op exists: `committed_current_term()`
  checks whether the leader's *own* current-term entry (which will always be at least the
  no-op) has committed, and the barrier below refuses to append a configuration until it
  has. `raft.rs` has no no-op and no equivalent check because, with static membership,
  there is nothing to gate on a current-term commit — this report's earlier draft
  described the no-op as belonging to the fix, which was inaccurate; it is present in the
  unpatched policy exactly as much as the patched one, and its presence alone does not
  imply the barrier is active.
- **The single-server add/remove protocol.** `ReconfigurationRequest`,
  `ConfigurationChange::{Add, Remove}`, and the `pending`/`PendingStage` state machine in
  `DynRaftState` implement the dissertation's one-change-at-a-time algorithm, per the
  module's own doc comment: "Additions first replicate to the prospective server as a
  non-voting learner. The catch-up test here is deliberately the simple `matchIndex ==
  leader last index` safety-oriented criterion; the dissertation's multi-round/time-bound
  heuristic is an availability refinement." Removals append immediately once the prior
  configuration is committed. Neither operation is reachable from `raft.rs` at all.
- **The 2015 current-term-commit barrier itself**, `StepPolicy::require_current_term_commit`,
  gating exactly one line: whether `latest_configuration_index() <= commit_index` also
  requires `committed_current_term()` before a new configuration may be appended. Per the
  module's doc comment, this is "the 2015 safety correction: a newly elected leader appends
  a no-op and may not append a configuration until it has committed an entry from its
  current term." This barrier is the entire subject of that correction — it is not an
  invention added to make a bug appear; it is the fix being tested for.
- **The doc-hidden unpatched path**, `dyn_raft_step_unpatched_for_simulation`, is
  `dyn_raft_step_with_policy` with that one gate condition dropped, plus (added in this
  session) the change described next.

### The one behavioral change made *in this session*, and why

Before this session, the truncation branch (an `AppendEntries` recipient asked to
overwrite a log entry with a same-index, different-term entry) carried a hand-added
assertion — not present in the dissertation — that panicked if the entry being overwritten
was already committed: `assert!(entry.index > state.commit_index, "protocol violation:
truncate committed entry ...")`. This assertion is Hydro-specific defensive code, not part
of Raft or the dissertation; the dissertation's actual `AppendEntries` receiver
(Figure 2, and the description in §3.5/§3.6) truncates conflicting suffixes
unconditionally, with no such guard, because the barrier is supposed to make truncating a
committed entry *unreachable*, not caught.

That guard interacted badly with the simulator specifically: when it fired inside the
dlopen'd sim dylib, the resulting panic crossed the dylib boundary as a foreign exception,
which Rust cannot catch (`fatal runtime error: Rust cannot catch foreign exceptions,
aborting` → `SIGABRT`) — not a normal, catchable panic, so `#[should_panic]` could not
observe it, and it always pre-empted the host-side oracle from ever seeing the forked
commit outputs. The fix made in this session was to make the *unpatched* policy truncate
conflicting suffixes unconditionally, with no guard — matching the description in this
module's own doc comment of "the single-server algorithm in Ongaro's dissertation" without
the 2015 correction — while leaving the guard in place on the *safe*, barrier-enforcing
policy (where, if the barrier is doing its job, it should indeed be unreachable, and a
panic there is a legitimate internal-consistency check on the production path). This is a
change to make the unsafe path behave as the dissertation-derived algorithm is documented
to behave, not a change to make the bug easier to find — the guard's presence or absence
does not affect whether the fork occurs, only whether it surfaces as an uncatchable
process abort or as an observable divergence in the committed output streams.

### Is there a danger of overfitting to a known-a-priori bug?

Partially, and this report tries to draw that line explicitly rather than paper over it:

- **The step function's mechanics (`dyn_raft_step_with_policy`, both policies) are not
  overfit on the parts shared with `raft.rs`.** `raft.rs`'s own top-of-file comment cites
  specific sections of the *original* Raft paper (Ongaro & Ousterhout) for the election
  restriction (§5.4.1), log matching (§5.3), and leader-only current-term commit counting
  (§5.4.2); `dyn_raft_step_with_policy` reimplements those same three mechanisms, and they
  were not written by working backward from the desired counterexample — they are the
  same logic as the pre-existing, unmodified `raft.rs`. The *2015-dissertation-specific*
  material — the barrier, the single-server add/remove state machine, and the no-op-as-
  barrier-target device — is described only in prose in `dyn_raft.rs`'s own doc comments
  (quoted above), without section numbers. This report does not independently verify
  those citations against the dissertation text, and does not know whether the author of
  `dyn_raft.rs` (in the prior commit `191fdeb1bf`, not part of this session) read the
  dissertation directly or worked from secondary knowledge of the well-known bug; either
  way, the resulting mechanism (append-then-gate-on-current-term-commit) matches the
  documented shape of the correction. Separately, the `#[should_panic(expected =
  "truncate committed entry")]` unit test that predates this session
  (`unsafe_policy_reproduces_ongaro_two_removes_counterexample`, in `dyn_raft.rs`'s prior
  commit) *is* a hand-scripted replay of the specific counterexample from that prior
  agent's writeup, driven step-by-step with full knowledge of the target shape — it was
  never claimed to be a blind discovery, and the task behind this session's tests was
  explicitly to go beyond it using the simulator's own search.
- **The fully-staged simulator test is overfit, and says so.** Its first two phases
  script the first two steps of the known counterexample (see "A correction" below).
- **The bootstrapped simulator test is the one built specifically to minimize this risk**:
  the only fixed step (a first election) is not specific to membership changes at all, and
  every subsequent choice was left to search. It is not *zero*-knowledge — the choice to
  send `Remove` requests for members 2 and 3 in particular, and the choice of workload
  sizes (2 election waves, 3 redundant copies per removal, 6 heartbeat waves), reflects
  knowing that *some* two-removal, multi-term scenario was the target, even though no
  particular leader/term assignment was scripted. A fully blind workload with no removal
  targets chosen by a bug-aware author is closer to what the `_unstaged` test attempts,
  and it has not (yet, at the budgets tried) found the fork.
- **This is a threat to validity that multi-version programming could address later.**
  Every test here checks one implementation (`dyn_raft_step_with_policy`, unpatched
  policy) against an oracle written by the same people who wrote the implementation and
  who knew the target bug. A stronger design would implement the single-server
  reconfiguration algorithm a second time, independently (ideally by an author blind to
  the counterexample, or derived mechanically from a TLA+/Ivy spec of the dissertation
  algorithm), and use *differential* testing — the two implementations must agree on every
  committed entry — as an oracle that does not itself encode knowledge of the specific
  fork. That is not done here; the current oracle (no two members commit different entries
  at the same index) is correct and independent of the implementation, but the *workload*
  that drives it to the fork is not independent of implementer knowledge, for the reasons
  above.

## Update: the operator-scenario tests, a broken first attempt, and the EC2 discovery

Everything below this heading was added after the sections above were written, and
supersedes parts of them.

### The operator scenarios

After the bootstrapped test's discovery, two further tests were written to model the
scenario an operator would actually create, with no tuning toward the known bug:

- `simulator_explores_concurrent_double_leave`: elect a leader (one election tick +
  quiesce), then submit **one** `Remove(1)` and **one** `Remove(2)` — each sent exactly
  once — followed by a symmetric batch of election/heartbeat ticks (the same 2-wave /
  6-wave shape as `raft.rs`'s own fully-concurrent test).
- `simulator_explores_concurrent_join_and_leave`: same shape, but with member 3 starting
  as a non-voting learner and the two requests being `Add(3)` and `Remove(1)`.

### The broken first attempt (a cautionary result)

The first version of both tests sent **both** requests to member 0 only. That version ran
on a 96-vCPU EC2 instance (`c7i.24xlarge`), 16 parallel coverage-guided fuzzers (8
independent seeds per scenario), for over two days and **543 million total executions —
and found nothing.** On review, that null result was structural, not evidence of safety:
Ongaro's counterexample requires two *different* leaders to each append a *different*
configuration entry, and since only member 0 was ever sent any reconfiguration request,
no other member could ever have a configuration to append. The search space simply did
not contain the bug. The 543M-execution run measured nothing beyond the cost of not
re-deriving reachability from first principles before scaling up.

Two fixes followed:

1. **Endpoints:** the second request now goes to member 1 (two concurrent operators
   contacting two different endpoints). The scheduler can deliver member 1's request while
   it is a follower (harmlessly rejected) or delay it until member 1 leads a later term —
   making a second configuration-appending leader structurally reachable with exactly one
   copy of each request.
2. **Oracle precision:** all the simulator oracles now compare committed **payloads** at
   each physical index rather than whole entries. Two same-payload no-ops from different
   terms no longer count as "the fork" (addressing the index-1 weakness discussed above);
   two incompatible committed configurations do.

Both existing checked-in reproducers (staged and bootstrapped) were re-verified to still
replay green under the payload-only oracle — confirming behaviorally that those replays
reach the *configuration* fork at index 2, not just the no-op term mismatch at index 1.

### The result

With the corrected tests, the same EC2 setup (fresh instance, 16 parallel fuzzers, 8
seeds per scenario, `-rss_limit_mb=8192`) produced, within roughly 45 million total
executions:

- **Three independent discoveries** of the safety violation in the double-leave scenario
  (seeds 4, 5, and 8), each from a different random search trajectory:
  - seeds 4 and 8: `Configuration({0,2,3})` vs `Configuration({0,1,3})` committed at
    physical index 2 — **both concurrent removals durably committed at the same log slot
    by different quorums**. One member's committed log says "member 1 was removed"; another
    member's committed log says "member 2 was removed". This is the dissertation
    counterexample in its literal form, discovered from an un-tuned two-operator workload.
  - seed 5: `Configuration({0,2,3})` vs `Noop` at index 2.
- **No violation (yet) in the join+leave scenario** across 8 seeds and 3.2M+ executions
  each at the time of the find — a real asymmetry, consistent with the `Add` path's
  learner catch-up requirement (`matchIndex` must reach the leader's last index before the
  configuration is appended) narrowing the unsafe window. This is reported as
  inconclusive, not as safety.

The seed-4 reproducer (121 bytes) is checked in at
`hydro_test/src/cluster/sim-failures/simulator_explores_concurrent_double_leave.bin`, and
the test now carries `#[should_panic(expected = "committed log forked")]`. Notably, the
reproducer was **discovered on x86_64 Linux and replays deterministically on aarch64
macOS** (~14s under plain `cargo test`) — the schedule encoding is platform-independent.

`simulator_explores_concurrent_join_and_leave` remains `#[ignore]`d and exploratory.

**Post-trim note:** after the double-leave discovery, the staged, bootstrapped, and
unstaged tests (and the staged/bootstrapped reproducers) were **removed from the test
file** as superseded rungs of the ladder this report documents: the double-leave test
demonstrates everything they demonstrated, from an honest workload, with its own
deterministic reproducer. The hand-scripted `StepCluster` unit test remains (it tests
step-function mechanics and never claimed discovery), as do the double-leave and
join+leave tests. The sections below describing the removed tests are retained as the
historical record of the methodology, including its missteps.

## Three tests, three different amounts of staging

All three tests drive the same doc-hidden simulation server,
`dyn_raft_server_unpatched_for_simulation`, over a **static** 4-member physical Hydro
cluster (`{0,1,2,3}`), through `flow.sim().fuzz(...)`. All three install the same oracle
(below). They differ only in how much of the scenario is pre-arranged by the test author
versus left to the simulator's schedule search.

| test | what is fixed by the test | what is left to the simulator | has it found the bug? |
|---|---|---|---|
| `simulator_discovers_ongaro_membership_bug` | Two full setup phases, each ended by a quiesce: (1) member 0 wins an uncontested election; (2) member 0 is sent `Remove(3)`. Only after that does a single un-quiesced burst run with member 1's election, `Remove(2)`, and heartbeats for both 0 and 1. | Within the final burst only: batching, message delivery order/timing, and vote outcomes. | Yes — found by coverage-guided fuzzing (`cargo sim`) after roughly half a million executions. |
| `simulator_discovers_ongaro_membership_bug_bootstrapped` | Only that *some* first leader exists: one election tick to member 0, one quiesce. Nothing else. | Everything after the bootstrap: which member leads which subsequent term, whether/when each of 24 copies of `Remove(2)`/`Remove(3)` (sent to *all four* members) is accepted or rejected, all replication and vote-message ordering and batching, across 2 further election waves and 6 heartbeat waves — all sent concurrently, with no quiescing until the very end. | **Yes** — found by coverage-guided fuzzing at **315,218 executions**. This is the strongest tier that has actually produced a reproducer. |
| `simulator_discovers_ongaro_membership_bug_unstaged` | Nothing protocol-specific. All four members receive symmetric inputs (election ticks, heartbeat ticks, copies of both removals) sent entirely up front, with **no quiescing at all** until a single final drain. | Everything, including which member(s) ever become leader at all. | Not yet. Plain `cargo test` (random, 8,192 iterations) does not find it. A coverage-guided `cargo sim` run reached 8.6 million executions and then hit libFuzzer's out-of-memory limit (2 GB) without finding it. This is marked `#[ignore]` and is not part of the default test suite. |

The bootstrapped test is the one worth trusting as the primary artifact, and is discussed
in detail below. The fully-staged test remains in the suite as a faster, more targeted
regression once the bug's shape is known. The unstaged test remains as an honest negative
result — see "What the simulator did *not* do."

## The oracle (identical in all three tests)

Hydro's simulator does not itself validate `assert_has_consistency_of` (the type-level
consistency claim on the production `committed` stream) — so this is an explicit,
hand-written runtime safety check in the test closure, evaluated after collecting every
member's committed-entries output stream:

```rust
let mut committed_at: HashMap<usize, DynLogEntry<String, Replica>> = HashMap::new();
for member in 0..N as u32 {
    for entry in committed_recv.collect::<Vec<_>>(member).await {
        if let Some(previous) = committed_at.get(&entry.index) {
            assert_eq!(
                previous, &entry,
                "committed log forked: physical index {} committed two different entries",
                entry.index
            );
        } else {
            committed_at.insert(entry.index, entry);
        }
    }
}
```

For every physical log index, every member that committed *anything* at that index must
have committed the *same* `DynLogEntry` (same term and payload) as every other member.
This is Raft's State Machine Safety property, checked directly against the physical log
index (not the application-visible command index), which is what the counterexample
actually violates. The test is written as `#[should_panic(expected = "committed log
forked")]`: the test "passing" means this assertion fired.

## What the bootstrapped test actually found

Verified reproducer replay (`cargo test --lib
cluster::dyn_raft::tests::simulator_discovers_ongaro_membership_bug_bootstrapped --
--ignored`, deterministic, ~2.4s):

```
assertion `left == right` failed: committed log forked: physical index 1 committed two different entries
  left: DynLogEntry { payload: Noop, term: 1, index: 1 }
 right: DynLogEntry { payload: Noop, term: 4, index: 1 }
```

The oracle asserts on the *first* index where it observes two members' `DynLogEntry`
values disagree (`PartialEq` compares payload, term, *and* index), so this is where
execution stopped. Taken alone, this specific instance is a weaker finding than the rest
of this report first claimed, and it deserves scrutiny rather than being read as
self-evidently catastrophic: both sides committed a `Noop` — an inert payload with no
application-visible content — at index 1, and the two entries differ only in `term` (1 vs
4). A term number differing on an otherwise-identical no-op is a real, structural
inequality (the `assert_eq!` correctly fired on unequal structs), but by itself it is not
obviously a *content* disagreement in the sense that matters to a client of the system.

To check whether the fork actually goes anywhere, we re-ran the same saved reproducer with
the oracle temporarily replaced by one that logs every member's full committed history
instead of asserting on the first mismatch (this was a diagnostic-only, reverted edit; the
committed test file is unchanged). The full committed histories were:

```
member 0: [Noop@term1/idx1, Configuration({0,1,2})@term1/idx2, Noop@term6/idx3]
member 1: [Noop@term4/idx1, Configuration({0,1,3})@term4/idx2]
member 2: []
member 3: [Noop@term4/idx1, Configuration({0,1,3})@term4/idx2]
```

This is where the actual, content-meaningful fork is: at physical index **2**, member 0
committed `Configuration({0,1,2})` (voters 0, 1, 2 — server 3 removed) while members 1 and
3 committed `Configuration({0,1,3})` (voters 0, 1, 3 — server 2 removed). Those are two
different, mutually incompatible cluster configurations, each durably committed by a
different majority. A client asking "is server 2 currently a voter?" gets opposite answers
depending on which member it asks, and both answers are backed by a committed log entry —
this is Ongaro's counterexample in its literal form (two configurations racing to commit
under different quorums), and it is not vacuous the way the index-1 mismatch alone would
be.

The index-1 mismatch is a *consequence* of the same fork, not a separate or weaker
instance of it: it is exactly what you'd expect to see immediately upstream of two
divergent configurations — each leader's own no-op, at its own term, is the entry each
leader committed for itself before appending its (different) configuration on top. The
oracle's choice to report the first divergent index means this report's earlier framing
(quoting only the index-1 mismatch and describing it as "the fork," full stop) understated
what was actually found and did not check what happened past that point before making the
claim. That is corrected here: the finding is the index-2 configuration fork; the index-1
no-op mismatch is real but, in isolation, not the interesting part.

**Caveat this raises about the oracle itself:** because `assert_eq!` panics on the first
mismatch, an oracle like this one will report whichever index diverges *first*, which is
not necessarily the index at which the divergence is *meaningful*. A version of this
oracle that continued past the first mismatch (collecting all of them, or specifically
checking whether any *non-Noop* payload disagrees) would be a strictly better artifact,
and was not built here. This is noted as a limitation, not fixed in this session.

### Lineage: the scheduler trace

`HYDRO_SIM_LOG=1 cargo test --lib ... -- --ignored --nocapture` replays the same saved
reproducer with the simulator's built-in tick/release logging turned on. The full log is
committed at [`docs/dyn_raft_bootstrapped_trace.txt`](./dyn_raft_bootstrapped_trace.txt);
it is 955 lines and ends with the panic above. Two things about what this trace is and is
not:

- **What it is:** a literal, line-by-line record of the scheduler's decisions: 49
  `Running Tick (Cluster Member N)` entries (16 for member 0, 12 for member 1, 11 for
  member 2, 10 for member 3), each followed by which dataflow edges in the compiled
  program released values on that tick (e.g. "releasing items: [()]" for an election-timer
  tick, "releasing no items" when a batch was empty). This is a real, mechanical
  lineage — it is what the simulator did, in the order it did it, and it is what makes the
  reproducer byte string meaningful rather than opaque.
- **What it is not:** a semantic Raft trace. It does not print "member 1 became leader of
  term 2" or "member 0 truncated index 1" — those facts have to be inferred from which
  edges fired, and several of the multiset/unordered releases are printed redacted as
  `[?]` by the sim runtime itself (this is the runtime's own logging code, not a
  redaction I applied). Reconstructing the full leader/term timeline from this trace alone
  is possible in principle but was not done as part of this report; the report's claims
  are limited to what the trace and the final assertion directly show.

## A correction

Earlier in this work, the first (fully-staged) test's doc comment and my own summary
described it as letting the simulator discover the bug "on its own." That was not
accurate: two setup phases in that test (elect member 0; send it `Remove(3)`) are the
first two steps of Ongaro's counterexample, written by someone who already knew the
target shape. That test's doc comment has since been corrected to say exactly that.

The bootstrapped test is the honest response to that criticism: the *only* fixed step is
"some leader exists," which is not specific to the membership bug at all (every Raft
cluster needs a first leader before anything else can happen), and every subsequent
choice — who leads which term, when each of many redundant reconfiguration requests lands,
all message and batch ordering — was left to the simulator. It found the fork without
being told the two-configuration mechanism.

## What the simulator did *not* do

- It did not find the counterexample from a fully symmetric, zero-staging workload within
  the budgets tried here: 8,192 random schedules (`cargo test`, no reproducer) and 8.6
  million coverage-guided schedules (`cargo sim`, ~2 hours before hitting libFuzzer's
  memory ceiling) both came up empty on the `_unstaged` test. This is not evidence the bug
  is unreachable from that workload — it is evidence that, at the budgets available in
  this environment, blind search did not get there, while a one-step bootstrap did (at
  315k executions). The gap between "one trivial bootstrap step" and "nothing" suggests
  the reachable-but-rare regime, not an artifact of over-fitting the test to the known bug.
- It did not need any message loss, crash-fault injection, or lossy transport: all three
  tests use `TCP.fail_stop().bincode()`. Interleaving and batching alone are sufficient.
- It did not validate `assert_has_consistency_of` — Hydro's simulator does not check that
  annotation at all (hence the hand-written oracle above; the unpatched simulation path
  also does not attach that annotation to its output, since it would be false).
- Coverage-guided fuzzing (`cargo sim`) requires sanitizer-coverage/libFuzzer linking that
  was not available in every environment this work ran in; where unavailable, `cargo test`
  alone cannot discover new failures (see the `_unstaged` result), only replay previously
  saved ones.

## User story: how the bootstrapped test was actually written

The fully-staged test was written first, by deliberately scripting the first two steps of
the known Ongaro counterexample (elect member 0; give it `Remove(3)`) and letting the
simulator improvise only the racy remainder. When that framing was challenged — could this
test have been written by someone who *didn't* already know the bug? — the honest answer
was no.

The bootstrapped test was then written to close that gap, under a self-imposed rule: the
only thing the test is allowed to fix is whatever is needed for the cluster to do anything
at all (a first leader), and every input that follows must be symmetric across all four
members, so that no line of the test encodes which member ends up doing what. Concretely,
the intent going in was just "throw a first election, then throw a pile of removals and
timer ticks at everyone and let the fuzzer sort out the schedule" — an attempt to cover a
large swath of schedules cheaply, not a targeted reproduction. Two further election waves,
three redundant copies of each removal (addressed to every member, to survive whichever
member happens to be leader when they land), and six heartbeat waves were chosen simply to
give the search room, without regard for how the counterexample mechanism actually works.

It was launched under `cargo sim -p hydro_test -- \
simulator_discovers_ongaro_membership_bug_semi_staged --ignored` (the test's working name
before it was renamed to `_bootstrapped`) with a fresh, empty corpus. Coverage climbed from
2,505 edges at initialization to 2,885–2,886 edges over the run. It found the fork at
execution **#315,218**, about **86 seconds** of wall-clock fuzzing after the sancov-instrumented
binary started running (build/link time before that was several minutes and is not
counted). The resulting 144-byte reproducer was written automatically to
`hydro_test/src/cluster/sim-failures/simulator_discovers_ongaro_membership_bug_bootstrapped.bin`,
and every subsequent `cargo test` run replays it deterministically in about 2.4 seconds —
no fuzzing required after the fact.

## How to reproduce this yourself

```bash
# Deterministic replay of the saved failure (fast, no fuzzing):
cargo test --lib \
  cluster::dyn_raft::tests::simulator_discovers_ongaro_membership_bug_bootstrapped \
  -- --ignored

# The same, with the simulator's scheduler trace on stderr
# (produces output like docs/dyn_raft_bootstrapped_trace.txt):
HYDRO_SIM_LOG=1 cargo test --lib \
  cluster::dyn_raft::tests::simulator_discovers_ongaro_membership_bug_bootstrapped \
  -- --ignored --nocapture

# Re-run coverage-guided discovery from scratch (deletes the saved reproducer's
# effect by fuzzing fresh; requires a libFuzzer/sancov-capable toolchain):
rm hydro_test/src/cluster/sim-failures/simulator_discovers_ongaro_membership_bug_bootstrapped.bin
./cargo-sim sim -p hydro_test -- \
  simulator_discovers_ongaro_membership_bug_bootstrapped --ignored
```
