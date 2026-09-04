# The gate

Every change to `hydro_lean/` passes all of this before it is accepted. Run from
`hydro_lean/` with the toolchain on `PATH` (`README.md` §Build). Run the executables
one at a time — concurrent `lake exe` invocations race on the build directory.

## 1. Build

```bash
lake build          # the `Hydro` library + the exes; expect ≈ 853 jobs, ≈ 5 min (16 cores)
```

Green means: every `#guard`/`#guard_eval` passed, every `#nondet_census` matched, every
`hydro def` pipeline generated its artifacts. Per-file envelope (report anything above):
`LeaderElection` ≤ ~90 s, `SequencePayload` ≤ ~60 s, `HydroTickCheck` ≤ ~50 s,
`PaxosCore` ≤ ~50 s, `Project/Model/WiredMain` ≤ ~75 s (D74: the 32-constructor `step_wired`
mutual block), `Paxos/PaxosBench` ≤ ~150 s (D82: the deepest nesting — `paxos_protocol`
44 s, `paxos_bench` 96 s, half of it the production-graph lemmas; queued in D83),
`Paxos/PaxosWithClient` ≤ ~35 s, no other file above 30 s. A new file above 15 s is reported with
its `trace.profiler` breakdown and `scripts/heartbeat_table.sh` (D72: two thirds of
`LeaderElection`'s old 320 s was three `#guard_eval`s interpreting a call-by-name `Values`
run; the rest was a failing `apply` paying ~1.7× per shared projection level in every
walk. D73: the guards run at `Values` in milliseconds — member carriers are `MemberMap`s
with a forcing compiled constructor). Executables: each binary runs its scenario in
well under a second (`paxos`/`twopc`/`explore`/`falsify` ≤ 40 ms wall, `wired` ≤ ~1 s
including socket setup and 60 activations); a program that takes seconds is a sharing
bug to find (`ENGINE_NOTES.md` "Runtime"), not a cost.

## 2. Zero sorries

```bash
grep -rn "sorry" Hydro/ *.lean --include="*.lean" | grep -v -i "vacuous\|-- \|prose"
```

must print nothing (comments discussing the word are excluded by the filter; a hit in
code is a failure).

## 3. Axioms

`Hydro/AxCheck.lean` is the sole axiom oracle. It `#print axioms` every headline
(`paxos_core`, `paxos_core.a_log.inv`, `collect_quorum`, `collect_quorum_with_response`,
`join_responses`, `paxos_co_wf`, `paxos_co_cpl`, `paxos_safe_sched'`, `paxos_safe_sched`, `cq_safe_sched'`/`cq_safe_sched`,
`two_pc_safe_sched'`/`two_pc_safe_sched`, `two_pc_once_sched'`/`two_pc_once_sched` (D80),
`MemberMap.mkImpl_eq`/`mk'_eq_mkImpl`, `memoBox_eq_memoBoxImpl`/`memoBox2_eq_memoBox2Impl`
(the executable-witness seams), `paxos_core.dag_denote`/`two_pc.dag_denote`/
`collect_quorum.dag_denote`, `Dag.run_spec`, `Dag.step_wired`, `Dag.step_drained`, `Interleaving.step_wired`,
`Interleaving.view_active`/`view_idle`/`outputs`, `paxos_safe_wired` (D74),
`paxos_core.schedOf_recordOf`/`two_pc.schedOf_recordOf` (D77), `Dag.pass_eq`,
`Interleaving.loop_pass`/`loop_idle` (D78), `WireTrace.expands`/`streamOutputsAt_expand`/
`streamOutputsAt_eq`, `Dag.settleImpl_eq`, `Dag.iterateBytes_eq_impl`,
`Dag.Timing.ofReadings_quiet` (D93), the corner's
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
lake exe wired     # last line: "wired: OK" (five independent TCP processes on 127.0.0.1:9400–9404; the ports must be free); "17 activations in 6 ms" (D93: a process idles unless bytes arrived or a timer is due — before D93 it ticked every millisecond). The witness only: its open-loop `--bench` mode and harness checkpoint channel were removed in D92 — `bench` is the measured system
lake exe wired --traces   # D90/D93: the six scripted byte-level traces of `WireTraceCheck` (sync / trickle at 5 bytes per read / silent / stalled acceptor / lopsided schedule / lazy = DFIR's wake rule as a greedy schedule) run natively through `WireTrace.run`; eight lines "… : true", then "lazy schedule: 15 activations, payload at step 7: [p0, p1, a0, a1, a2, p0, p1, p0, a1, a2, p0, p1, a0, p0, p1]", last line "wired --traces: OK", ≈ 3 s
lake build bench && lake exe bench 500 10 6000   # D83/D87: paxos_bench as ten OS processes over TCP (127.0.0.1:9500–9509; the parent spawns `bench --proc P` and relays), 1000 virtual clients in flight, checkpoints every 10 slots, 6 s; "Throughput: … requests/s" / "Latency p50 … ms" lines (expect ≈ 85–95k requests/s, p50 ≈ 11.5 ms on a 32-core box at the Rust's 100 ms client / 1000 ms aggregate windows and real 5 s/10 s election timers, every process idle between arrivals — D93, ≈ 475 % CPU for the ten processes; 115–122k / 8 ms at D92 with every tick-hosting process spinning at 100 %, the drop being the settling tick per burst at the leader; 65–70k / 13 ms at D89; 82k at D87 was a 200 ms window with scripted timers; 22.5k / 43 ms at D83; `HYDRO_BENCH_NODEMUX=1` sends `demux` messages to the whole receiver cluster, the pre-D92 routing, same throughput), last line "bench: OK" (a run that completes no request prints "bench: FAILED" and exits 1 — a previous run's ports still bound does that). D93: each client reports "4 activations ended with work still pending (the settling counter ran out; the next activation started at once)" — its ramp-up, one virtual client spawned per added tick; any other process reporting it, or a larger count, is a finding
/usr/bin/time -v .lake/build/bin/bench 500 10 20000   # D84/D87: memory and step-time flatness — throughput steady over the 20 s (85–95k, "Percent of CPU this job got" ≈ 475 % — D93; 115–122k at ≈ 1000 % in D92), and each process's "step first quarter / last quarter" within ~15 % (the first quarter includes the cheaper ramp-up second; the leader's +5 % per 20 s drift of D87 was an `IO.Ref` array marked multi-threaded and copied at every push — gone in D92, 383/381 µs). Per-process memory: sample `ps -o rss,args` on the `bench --proc P` processes during the run — flat, leader ≈ 220 MB, the others 160–185 MB (D90, D92; `time -v`'s "Maximum resident set size" on the launcher reports ≈ 910 MB, summed over the ten OS processes, and is not a per-process figure). A process holds no history: a growing RSS is a leak (D84 found 30 GB in 6 s)
```

`falsify` is the proof that the premises bite: the *faithful* paxos.rs variants commit
two values at one slot (B1 at `paxos.rs:311–317`, B2 the per-tick rebase — at module
scope on `sequence_payload`'s output and at whole-program scope on `paxos_core`), the
*guarded* variants do not. `paxos`/`twopc` are the D15 non-vacuity witnesses — the
`Values` programs run directly (what they print is the denotation; the compiled seams
are `MemberMap.mkImpl` and the D74 memo tables `memoBoxImpl`/`memoBox2Impl`, each
authorized by its `@[csimp]` theorem). `wired` is the D74 witness: `paxos_core` as two
proposers and three acceptors over real TCP (a witness, not a benchmark — D92), each process stepping only its own cells
(`Dag.iterateBytes` — the byte-level loop body `paxos_safe_wired` is about, D90), each on its
own thread, every message its own frame, the receiver cutting the stream wherever TCP cut it;
it prints the commit `(0, some 1000)` and checks slot-functionality across processes;
`--traces` runs the same body along six scripted traces (`WireTraceCheck`). The
benchmark modules' witness is `bench` (D83): the whole `paxos_bench` graph as ten processes
with the clock and the interval timers read online, printing Rust's `pretty_print_bench_results`
(the checkpoint flow is live in it — without checkpoints the acceptor step climbs to 12 ms). Nothing unverified stands between a program and its binary:
`grep -rn "implemented_by\|@\[extern\|native_decide\|^unsafe" Hydro/ *.lean` must return only `MemberMap.lean`'s
doc comment saying so, and `grep -rn "@\[csimp\]" Hydro/` exactly the sixteen equations
(`MemberMap`, the two memo tables, the six hash-backed keyed operators of D75, D83's
`keyedReduceAssocH_eq_impl`/`SchedKMap.joinH_eq_impl` — the benchmark client's
`reduce`/`join_keyed_singleton` —, D86's `Dag.locStep_eq_impl` — the production step
skipping the nodes a process does not host —, and D92's four: `Dag.iterateBytes_eq_impl`
(`Project/Fused.lean`: each connection parsed once, messages bucketed by node, the graph
walked once — on the very function `paxos_safe_wired` is stated over and the harnesses
call), `getNat_eq_impl` (LEB128 without `Nat.shiftLeft`), `frameAll_eq_impl` (left fold)
and `frameMsgs_eq_impl` (frames written in place)). The
`Wire` class's `put` is not a seam: it is a field with its law `put_eq` in the class, and
`Codec.ofWire_enc` is the theorem that a codec encodes by `encode`. (Queued: once the toolchain is ≥ 4.35, `lake check` — core's
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
| `timeout`, `sample_every` (D89, `Std/Timers.lean`; D91: `sample_every`'s interval timer is a sched-det) | 2 / 0 / 0, 1 / 1 / 0 |
| `p_leader_heartbeat` | 5 / 3 / 0 (D89: `sample_every`'s batch, `timeout`'s batch + clock, its `snapshot` cut, the election-trigger batch; D91: the gossip transport + the two interval timers' fire scripts) |
| `recommit_after_leader_election` | 1 / 0 / 0 (D71: the fold's `assume_ordering` `nondet!`) |
| `sequence_payload` | 6 / 2 / 0 |
| `leader_election` | 10 / 5 / 3 (D91: +2 interval timers) |
| `paxos_core` | 16 / 7 / 5 (D91) |
| `two_pc` | 2 / 4 / 0 |
| `bench_client`, `compute_throughput_latency`, `aggregate_bench_results` (D82; D91: the two measurement-window timers are sched-dets) | 6/0/0, 2/1/0, 3/3/0 |
| `sequence_payloads`, `kv_replica` (D82) | 1/0/0, 1/0/2 |
| `payloads_to_leader`, `paxos_with_client` (D82) | 3/2/0, 19/9/6 (D91) |
| `inc_i32_workload_generator`, `tag_with_sender`, `acceptor_checkpoint_from_replicas`, `paxos_protocol`, `paxos_bench` (D82) | 0/0/0, 1/0/0, 1/1/0, 23/12/9, 34/16/11 (D91) |
| toys (`HydroTickCheck.lean`) `toy_kmax`, `toy_wm`, `toy_pre`, `toy_wm_fold`, `toy_tk_consume`, `toy_tk_order`, `toy_bench_ops`, `toy_kleaf`, `toy_bench_wires`, `toy_defer_next_tick` (D85), `toy_bo_or`, `toy_snap` (D89), `toy_batch_stutter` (D91) | 2/0/0, 1/0/0, 1/0/0, 1/0/0, 2/0/0, 2/0/1, 1/0/0, 1/0/0, 5/0/0, 0/0/0, 0/0/0, 1/0/0, 1/0/0 |

A census change is a statement about the Rust mirror and is ratified explicitly
(D54, D55, D61).

## 6. Budgets

Heartbeats are a proxy; **proof time is the goal** (user ruling, D72). The engine
(`HydroDef`/`HydroGen*`/`HydroTick`) carries no budget. Two program files carry a
measured `set_option maxHeartbeats … in` on their def — `Paxos/LeaderElection.lean`
(600k) and `Paxos/PaxosCore.lean` (300k, re-measured at D91: ≈ 251M) — because one `hydro def` command elaborates
the def AND every declaration it generates (knot stacks, couplings, causality, wf, the
free theorem; ≈500M raw for `leader_election`, no single theorem above 90M) and Lean
counts them together. An override is a measured fact: report it with its
per-declaration table (`scripts/heartbeat_table.sh <file>`), and treat a change that
needs a third site or a higher value as an engine blowup to diagnose (`trace.profiler`;
D56's keyed mono resolver, D72's keyed rule dispatch `hydro_rule_apply`, D81's
`hydro_wf_close` head rule and reducible `kabstract` — 3.6 G and 419 M heartbeats found
and removed rather than budgeted), never a number to raise. The benchmark modules
(`PaxosBench.lean`, the deepest nesting in the tree) carry no override: 122 M / 85 M raw.

```bash
grep -rn "^set_option maxHeartbeats" Hydro/ --include="*.lean"     # exactly the two
```

## 7. Statements

The headline statements are byte-identical across refactors unless a checkpoint
ratified a change (`DOCTRINE.md` R9). Diff `paxos_safe_sched'`, `cq_safe_sched'`,
`cq_live`/`cq_live_chain`, `PCEnsures`, and every `…Ensures` that a
*different* module consumes. Ratified representation changes: D91 wrote the grade into
every `BatchDec n α` (`.noOrder .exactlyOnce` at `cq_safe_sched`'s witness — the same
decision set) and added the interval timers' `IntervalDec` fields to the `…Sched` bundles
(`paxos_safe_sched'` quantifies over two more sched-dets).

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
