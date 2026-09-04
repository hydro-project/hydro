# The gate

Every change to `hydro_lean/` passes all of this before it is accepted. Run from
`hydro_lean/` with the toolchain on `PATH` (`README.md` §Build). Run the executables
one at a time — concurrent `lake exe` invocations race on the build directory.

## 1. Build

```bash
lake build          # the `Hydro` library + the four exes; expect ≈ 783 jobs, ≈ 6 min
```

Green means: every `#guard`/`#eval` passed, every `#nondet_census` matched, every
`hydro def` pipeline generated its artifacts. Per-file budget (report anything above):
`LeaderElection` ≤ ~275 s, `SequencePayload` ≤ ~60 s, `PaxosCore` ≤ ~90 s, no other
file above 30 s. A new file above 15 s is reported with its `trace.profiler` breakdown.

## 2. Zero sorries

```bash
grep -rn "sorry" Hydro/ *.lean --include="*.lean" | grep -v -i "vacuous\|-- \|prose"
```

must print nothing (comments discussing the word are excluded by the filter; a hit in
code is a failure).

## 3. Axioms

`Hydro/AxCheck.lean` is the sole axiom oracle. It `#print axioms` every headline
(`paxos_core`, `paxos_core.a_log.inv`, `collect_quorum`, `collect_quorum_with_response`,
`join_responses`, `paxos_co_wf`, `paxos_co_cpl`, `paxos_safe_sched'`, `cq_safe_sched'`,
`paxos_eager_den`, `paxos_eager_den_ballots`, `paxos_eager_commits`, the corner's
`co_fix_cpl`/`co_tick_fix_cpl`/`cc_*`, `paxos_co_sr`/`_rr`, the generated `_co_*`,
`_param` classes, `paxos_core.a_log.inv`, `toy_safe_sched`, `eagLaws`/`monoLaws`/`causalLaws`).
Every report must be a subset of `[propext, Classical.choice, Quot.sound]`; `sorryAx`
anywhere is a failure. Check from the build log:

```bash
grep -i "depends on axioms" build.log | grep -v -E "\[propext(, Classical\.choice)?(, Quot\.sound)?\]|\[Classical\.choice(, Quot\.sound)?\]|\[propext, Quot\.sound\]|\[Quot\.sound\]"
grep -ci sorryAx build.log     # must be 0
```

A new headline theorem is added to `AxCheck.lean` in the same change.

## 4. Executables (sequentially)

```bash
lake exe falsify   # last line: "All falsification checks PASSED."
lake exe explore   # "acausal script agreement: true" and "causal control agreement: true"
lake exe paxos     # last line: "paxos: OK"
lake exe twopc     # last line: "twopc: OK"
```

`falsify` is the proof that the premises bite: the *faithful* paxos.rs variants commit
two values at one slot (B1 at `paxos.rs:311–317`, B2 the per-tick rebase — at module
scope on `sequence_payload`'s output and at whole-program scope on `paxos_core`), the
*guarded* variants do not. `paxos`/`twopc` are the D15 non-vacuity witnesses at `Eager`
(what they print is the `Values` run by `paxos_eager_den`).

## 5. Censuses

Build-enforced (`#nondet_census` fails the build on mismatch). Expected values —
(nondets / scheds / fuels), each equal to the Rust `nondet!` tally at that function
plus its cycles:

| module | census |
|---|---|
| `collect_quorum`, `collect_quorum_with_response`, `join_responses` | 1 / 0 / 0 |
| `acceptor_p1` | 0 / 1 / 0 |
| `acceptor_p2` | 2 / 1 / 0 |
| `p_p1b` | 3 / 0 / 0 |
| `p_leader_heartbeat` | 3 / 1 / 0 |
| `sequence_payload` | 5 / 2 / 0 |
| `leader_election` | 8 / 3 / 3 |
| `paxos_core` | 13 / 5 / 5 |
| `two_pc` | 2 / 4 / 0 |

A census change is a statement about the Rust mirror and is ratified explicitly
(D54, D55, D61).

## 6. Budgets

Exactly two `set_option maxHeartbeats … in` sites exist: `Paxos/LeaderElection.lean`
(1.6M) and `Paxos/PaxosCore.lean` (400k), measured at power-of-two by
`scripts/budget_probe.sh <file> <value>` (`0` deletes the option). A change that needs
a third site, or a higher value, is diagnosed with `trace.profiler` first; the engine
(`HydroDef`/`HydroGen*`/`HydroTick`) never carries a budget.

```bash
grep -rn "^set_option maxHeartbeats" Hydro/ --include="*.lean"     # exactly the two
```

## 7. Statements

The headline statements are byte-identical across refactors unless a checkpoint
ratified a change (`DOCTRINE.md` R9). Diff `paxos_safe_sched'`, `cq_safe_sched'`,
`paxos_eager_den*`, `cq_live`/`cq_live_chain`, `PCEnsures`, and every `…Ensures` that a
*different* module consumes.

## 8. Ledger

A `FINDINGS.md` entry (next D-number) records: the question, the measured cause, the
ruling(s) with the user's words, what landed, before/after numbers (lines, build time,
census), the gate result, and gotchas for `ENGINE_NOTES.md`. Paths in entries before
D68 read `HydroV2/…`; the library is `Hydro/` since D68.

## Governance

The user ratifies: any `HydroSem` field change (per op), any statement-shape change to a
headline or a consumed face, any engine extension, any census change, any budget
change. Work proceeds by checkpoints (design → ratification → build → gate); a thread
holds open for explicit approval before closing. Clean handoff beats degraded work:
back out a half-landed route rather than leave it in the tree.
