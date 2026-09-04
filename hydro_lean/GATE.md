# The gate

Every change to `hydro_lean/` passes all of this before it is accepted. Run from
`hydro_lean/` with the toolchain on `PATH` (`README.md` §Build). Run the executables
one at a time — concurrent `lake exe` invocations race on the build directory.

## 1. Build

```bash
lake build          # the `Hydro` library + the four exes; expect ≈ 826 jobs, ≈ 3 min (16 cores)
```

Green means: every `#guard`/`#eval` passed, every `#nondet_census` matched, every
`hydro def` pipeline generated its artifacts. Per-file envelope (report anything above):
`LeaderElection` ≤ ~90 s, `SequencePayload` ≤ ~60 s, `HydroTickCheck` ≤ ~50 s,
`PaxosCore` ≤ ~45 s, no other file above 30 s. A new file above 15 s is reported with
its `trace.profiler` breakdown and `scripts/heartbeat_table.sh` (D72: two thirds of
`LeaderElection`'s old 320 s was three `#guard_eval`s interpreting a call-by-name `Values`
run; the rest was a failing `apply` paying ~1.7× per shared projection level in every
walk. D73: the guards run at `Values` in milliseconds — member carriers are `MemberMap`s
with a forcing compiled constructor). Executables: each binary runs its scenario in
well under a second (`paxos`/`twopc`/`explore`/`falsify` ≤ 40 ms wall); a program that
takes seconds is a sharing bug to find (`ENGINE_NOTES.md` "Runtime"), not a cost.

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
`MemberMap.mkImpl_eq`/`mk'_eq_mkImpl` (the executable-witness seam), the corner's
`co_fix_cpl`/`co_tick_fix_cpl`/`cc_*`, `paxos_co_sr`/`_rr`, the generated `_co_*`,
`_param` classes, `paxos_core.a_log.inv`, `toy_safe_sched`, `monoLaws`/`causalLaws`).
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
*guarded* variants do not. `paxos`/`twopc` are the D15 non-vacuity witnesses — the
`Values` programs run directly (what they print is the denotation; the one compiled
seam is `MemberMap.mkImpl`, authorized by the `@[csimp]` theorem `mk'_eq_mkImpl`).
Nothing unverified stands between a program and its binary: `grep -rn "implemented_by\|@\[extern\|native_decide\|^unsafe" Hydro/ *.lean` must return only `MemberMap.lean`'s
doc comment saying so. (Queued: once the toolchain is ≥ 4.35, `lake check` — core's
Comparator — replays the headlines through the kernel with statement identity and the
permitted-axiom list, replacing `AxCheck.lean` and the R9 diff rule.)

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
| `recommit_after_leader_election` | 1 / 0 / 0 (D71: the fold's `assume_ordering` `nondet!`) |
| `sequence_payload` | 6 / 2 / 0 |
| `leader_election` | 8 / 3 / 3 |
| `paxos_core` | 14 / 5 / 5 |
| `two_pc` | 2 / 4 / 0 |
| toys (`HydroTickCheck.lean`) `toy_kmax`, `toy_wm`, `toy_pre`, `toy_wm_fold`, `toy_tk_consume`, `toy_tk_order` | 2/0/0, 1/0/0, 1/0/0, 1/0/0, 2/0/0, 2/0/1 |

A census change is a statement about the Rust mirror and is ratified explicitly
(D54, D55, D61).

## 6. Budgets

Heartbeats are a proxy; **proof time is the goal** (user ruling, D72). The engine
(`HydroDef`/`HydroGen*`/`HydroTick`) carries no budget. Two program files carry a
measured `set_option maxHeartbeats … in` on their def — `Paxos/LeaderElection.lean`
(600k) and `Paxos/PaxosCore.lean` (250k) — because one `hydro def` command elaborates
the def AND every declaration it generates (knot stacks, couplings, causality, wf, the
free theorem; ≈500M raw for `leader_election`, no single theorem above 90M) and Lean
counts them together. An override is a measured fact: report it with its
per-declaration table (`scripts/heartbeat_table.sh <file>`), and treat a change that
needs a third site or a higher value as an engine blowup to diagnose (`trace.profiler`;
D56's keyed mono resolver, D72's keyed rule dispatch `hydro_rule_apply`), never a
number to raise.

```bash
grep -rn "^set_option maxHeartbeats" Hydro/ --include="*.lean"     # exactly the two
```

## 7. Statements

The headline statements are byte-identical across refactors unless a checkpoint
ratified a change (`DOCTRINE.md` R9). Diff `paxos_safe_sched'`, `cq_safe_sched'`,
`cq_live`/`cq_live_chain`, `PCEnsures`, and every `…Ensures` that a
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
