# The gate

Every change to `hydro_lean/` passes all of this before it is accepted. Run from
`hydro_lean/` with the toolchain on `PATH` (`README.md` §Build). Run the executables
one at a time — concurrent `lake exe` invocations race on the build directory.

## 1. Build

```bash
lake build          # the `Hydro` library (the default target); `lake build Hydro falsify explore paxos twopc wired bench benchprof` for the library + the seven exes, ≈ 1970 jobs, 6–8 min (32 cores)
```

The lakefile runs every `lean` under **`-M 4500`** (D95: 4000; D98: 4500, provisional —
see below): the heaviest library file, `Paxos/PaxosBench`, peaks at 3.4 GB, `LeaderElection`
at 2.7; a kernel judgement that evaluates a module application, a lag analysis or a knot
wire exceeds the cap at once (19 GB observed), so the cap is the regression detector —
never raise it to make a file pass (ENGINE_NOTES "The kernel and knots"). To measure one
file: `/usr/bin/time -v lake env lean -M 4500 -DElab.async=false Hydro/Paxos/<F>.lean`
(synchronous kernel: a failure names its declaration and the heartbeat budget is the file's
whole; `PaxosBench` needs the asynchronous default for its 200k). The two files that stage
an activation (`hydro_run`: `Paxos/WiredSafety`'s `paxosRun`, `BenchDemo`'s `benchRun`) are
the cap's neighbours: their kernel bridge (`name.eq_impl`, FINDINGS D96) is a `rfl` over a
364- / 668-let body and peaks at 4.2 / 5.2 GB of RSS (synchronous, imports 2.3 GB of it);
what `-M` meters is Lean's own accounting at the kernel's checks and under the
asynchronous default it depends on what else is live — `BenchDemo` needs > 5000
synchronously and passes at 4500 under `lake build`; `WiredSafety` passes at 4000
synchronously and failed at 4000 under `lake build` after D98. D98 (the batch carrier as an
`Array`) grew these terms ≈ 7 % with the same heartbeats and kernel time (FINDINGS D98) and
the cap went 4000 → 4500, the smallest step at which `lake build` is green; a further rise
is a finding, not a fix.

Green means: every `#guard`/`#guard_eval` passed, every `#nondet_census` matched, every
`hydro def` pipeline generated its artifacts. Per-file envelope (report anything above):
`LeaderElection` ≤ ~90 s, `SequencePayload` ≤ ~60 s, `HydroTickCheck` ≤ ~50 s,
`PaxosCore` ≤ ~65 s, `Project/Model/WiredMain` ≤ ~110 s (D74: the 32-constructor `step_wired`
mutual block; D95: its `fixN` case by induction on the knot rounds), `Project/Process` ≤ ~85 s,
`Project/Model/DistributedModel`/`WireTrace`/`StepOps` ≤ ~70 s, `Project/Model/SweepSpec` ≤ ~55 s, `Paxos/PaxosBench` ≤ ~150 s (D82: the deepest nesting — `paxos_protocol`
44 s, `paxos_bench` 96 s, half of it the production-graph lemmas; queued in D83),
`Paxos/PaxosWithClient` ≤ ~35 s, no other library file above 30 s. The exe roots that stage
their activation (D96, `hydro_run`): `BenchDemo` ≤ ~60 s (its info line reports the three
phases — `stage ≈ 9.4 s`, and with `-DElab.async=false` `compile ≈ 6 s`, `kernel ≈ 20 s`;
asynchronously the latter two print as a few hundred ms), `WiredDemo` ≤ ~35 s
(`stage ≈ 4.6 s`) and `BenchProfile` ≤ ~30 s (D95: its fused-sweep line); in the library,
`Paxos/WiredSafety` ≤ ~35 s (D96: `hydro_run paxosRun` — stage 4.9 s, compile 3.3 s,
kernel 12.4 s synchronous — plus the headline). A new file above 15 s is reported with
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

`Hydro/AxCheck.lean` is the sole axiom oracle, and its own section comments are the
authoritative list of what it audits. In outline (≈90 `#print axioms`): the programs'
colocated contracts (`<M>.ensures`, the hoisted invariants such as `paxos_core.a_log.inv`);
the machine-run headlines (`paxos_safe_sched'`/`paxos_safe_sched`, `cq_*`, `two_pc_*`,
`paxos_safe_wired`); the staged activations' equations (`paxosRun.eq`/`paxosBody_eq`,
`StageCheck.toyRun.eq`; `BenchDemo.benchRun.eq` is an exe root); the executable-witness
`@[csimp]` pins (`MemberMap.mk'_eq_mkImpl`, `memoBox*_eq_*Impl`, `Dag.iterateBytes_eq_impl`,
the hash-backed keyed operators); the corner combinators (`co_fix_cpl`/`co_tick_fix_cpl`)
and spot checks of the generated naming surface (`_co_sr`/`_co_rr`/`_co_wf` of the
whole-program glue stack and the deepest knot); the graph layer (`<M>.dag_denote`,
`dag_valid`, `K_strict₁`, `Dag.denote_agreeL`, `schedOf_recordOf`); and the interpreter's
correctness chain (`Dag.run_spec`, `step_wired`, `pass_eq`, `iterate_eq_pass`,
`Interleaving.*`, `WireTrace.streamOutputsAt_eq`).
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
lake exe wired     # last line: "wired: OK" (five independent TCP processes running `paxosBody`, the headline's function; on 127.0.0.1:9400–9404; the ports must be free); "≈ 20 activations in a few ms" (D93: a process idles unless bytes arrived or a timer is due — before D93 it ticked every millisecond; D94: the sockets are `Sockets.lean`, shared with `bench`). The witness only: its open-loop `--bench` mode and harness checkpoint channel were removed in D92 — `bench` is the measured system
lake exe wired --traces   # D90/D93: the six scripted byte-level traces of `WireTraceCheck` (sync / trickle at 5 bytes per read / silent / stalled acceptor / lopsided schedule / lazy = DFIR's wake rule as a greedy schedule) run natively through `WireTrace.run`; eight lines "… : true", then "lazy schedule: 15 activations, payload at step 7: [p0, p1, a0, a1, a2, p0, p1, p0, a1, a2, p0, p1, a0, p0, p1]", last line "wired --traces: OK", ≈ 3 s
lake build bench benchprof && lake exe bench 500 10 6000   # D83/D87: paxos_bench as ten OS processes over TCP (127.0.0.1:9500–9509; the parent spawns `bench --proc P` and relays), 1000 virtual clients in flight, checkpoints every 10 slots, 6 s; "Throughput: … requests/s" / "Latency p50 … ms" lines (expect ≈ 220–232k requests/s, p50 ≈ 4.3 ms at D99 — the one-pass keyed fold, the sends framed from the arrays, the specialized decoder loop, the inlined pair equality and the array channel queue, FINDINGS D99; ≈ 165–175k, p50 ≈ 5.8 ms at D98 — the batches as `Array`s through the machine, FINDINGS D98; ≈ 160–175k, p50 ≈ 6.0 ms at D97 — the staged activation with the wire path's buckets and routing by index, FINDINGS D97; ≈ 145–160k, p50 ≈ 6.5 ms at D96 — the staged activation `BenchDemo.benchRun`, FINDINGS D96; ≈ 86–91k, p50 ≈ 11.5 ms at D95 — the sweep activation, FINDINGS D95; ≈ 100k, p50 ≈ 10 ms at D94 — on a 32-core box at the Rust's 100 ms client / 1000 ms aggregate windows and real 5 s/10 s election timers, every process idle between arrivals — ≈ 350 % CPU for the ten processes at D95, 310 % at D94; 85–95k / 11.5 ms at ≈ 475 % at D93; 115–122k / 8 ms at D92 with every tick-hosting process spinning at 100 %, the drop being the settling tick per burst at the leader; 65–70k / 13 ms at D89; 82k at D87 was a 200 ms window with scripted timers; 22.5k / 43 ms at D83; last line "bench: OK" (a run that completes no request prints "bench: FAILED" and exits 1 — a previous run's ports still bound does that). `lake exe benchprof decode 50` (D94; the wire microbench, four `ns/msg` lines — `Wire.decode (P2b)` ≈ 38 ns since D99's `@[specialize] getNatKLoop`, 105 before) and `lake exe benchprof idle 0 300` (the leader's empty activation — `full activate (compiled, benchRun)` ≈ 56–58 µs at D96–D98 (acceptor 40, client 48, aggregator 36, replica 35: `benchprof idle P 300`), against `full activate (interpreter, Dag.iterateBytes)` ≈ 470 µs, the reference the stages below belong to — ≈ 480 µs at D95 (632 before the `SweepRes` walk; D94's `activateFused` ≈ 60 — the sweep's knot chain re-walks the graph 5–6 times per activation, FINDINGS D95), the inbox/enqueue/memo/timing/oracle stages ≈ 9–17 µs each, `locStep` (the reference, ≈ 4.4 ms), `sweepTop` ≈ 1.2 ms, `wireEncode` ≈ 10 µs, `sweepF` (the fused sweep, what runs) ≈ 450 µs) and `lake exe benchprof idle 0 300 full` (adds the one walk over all-done parts, ≈ 47 µs — the per-visit floor) must run; `benchprof stages 500 10 4000` is the instrumented benchmark (not part of the gate). D93: each client reports "4 activations ended with work still pending (the settling counter ran out; the next activation started at once)" — its ramp-up, one virtual client spawned per added tick; any other process reporting it, or a larger count, is a finding
/usr/bin/time -v .lake/build/bin/bench 500 10 20000   # D84/D87: memory and step-time flatness — throughput steady over the 20 s (228–232k, p50 4.3 ms, p99 4.9, ≈ 380 %, launcher RSS ≈ 187 MB — D99; 170–173k, p50 5.8 ms, p99 6.6–7.1, ≈ 345 %, launcher RSS ≈ 187 MB — D98; 160–175k, p50 6.0 ms, p99 6.8–7.0, ≈ 330 %, launcher RSS ≈ 170–280 MB — D97; 145–160k, p50 6.5–6.6 ms, p99 7.4–7.9, ≈ 330 %, launcher RSS ≈ 166 MB — D96; 85–89k, p50 11.5 ms, ≈ 350 % — D95; 100–107k, p50 10 ms, "Percent of CPU this job got" ≈ 310 % — D94; 85–95k at ≈ 475 % in D93; 115–122k at ≈ 1000 % in D92), and each process's "step first quarter / last quarter" within ~15 % (D95: the leader 1260/1394 µs; the clients' and replicas' +25–36 % drift is an open D96 item) (the first quarter includes the cheaper ramp-up second; the leader's +5 % per 20 s drift of D87 was an `IO.Ref` array marked multi-threaded and copied at every push — gone in D92, 383/381 µs). Per-process memory: sample `ps -o rss,args` on the `bench --proc P` processes during the run — flat, leader ≈ 220 MB, the others 160–185 MB (D90, D92; `time -v`'s "Maximum resident set size" on the launcher is ≈ 150 MB since D94 — the launcher's own, the children are separate processes). A process holds no history: a growing RSS is a leak (D84 found 30 GB in 6 s)
```

`falsify` is the proof that the premises bite: the *faithful* paxos.rs variants commit
two values at one slot (B1 at `paxos.rs:311–317`, B2 the per-tick rebase — at module
scope on `sequence_payload`'s output and at whole-program scope on `paxos_core`, B4 the
rebase below a collected slot), the *guarded* variants do not; B5 (D95): the faithful
recommit never recommits an in-flight slot `C` after a checkpoint `C` (liveness), the
guarded one does (`b5Run`). `paxos`/`twopc` are the D15 non-vacuity witnesses — the
`Values` programs run directly (what they print is the denotation; the compiled seams
are `MemberMap.mkImpl` and the D74 memo tables `memoBoxImpl`/`memoBox2Impl`, each
authorized by its `@[csimp]` theorem). `wired` is the D74 witness: `paxos_core` as two
proposers and three acceptors over real TCP (a witness, not a benchmark — D92), each process stepping only its own cells
(`paxosBody` — the compiled byte-level loop body `paxos_safe_wired` is stated over, D90/D96), each on its
own thread, every message its own frame, the receiver cutting the stream wherever TCP cut it;
it prints the commit `(0, some 1000)` and checks slot-functionality across processes;
`--traces` runs the same body along six scripted traces (`WireTraceCheck`, ten checks: eight on
the compiled body, two comparing it with the interpreter's `Body.ofCodecs` — commits on
`trickle`, bytes on `lazy`). The
benchmark modules' witness is `bench` (D83): the whole `paxos_bench` graph as ten processes
with the clock and the interval timers read online, printing Rust's `pretty_print_bench_results`
(the checkpoint flow is live in it — without checkpoints the acceptor step climbs to 12 ms). Nothing unverified stands between a program and its binary:
`grep -rn "implemented_by\|@\[extern\|native_decide\|^unsafe" Hydro/ *.lean` must return only `MemberMap.lean`'s
doc comment saying so, and `grep -rn "@\[csimp\] theorem" Hydro/` exactly the nineteen equations
(`MemberMap`'s `mk'_eq_mkImpl`, the two memo tables `memoBox_eq_memoBoxImpl`/`memoBox2_eq_memoBox2Impl`, the six hash-backed keyed operators of D75 (`dedupH`, `keyedFoldAssocH`, `reduceWatermarkEagerH`, and — over the `Array` batches since D98 — `arrFilterNotIn`, `arrAntiJoin`, `arrJoinH`, each `arr…_eq_impl` where D75's was `list…`), D98's `keyedFoldArrH_eq_impl` (`KeyedHash.lean`: the in-tick `fold_keyed` on a batch — `groupKeysStep` folded over the array from the right, one pass over the buffer where the list form materialized it first; through `keyedFoldArrImpl_eq` and `keyedFoldAssocH_eq_impl`), D83's
`keyedReduceAssocH_eq_impl`/`SchedKMap.joinH_eq_impl` — the benchmark client's
`reduce`/`join_keyed_singleton` —, D86's `Dag.locStep_eq_impl` — the production step
skipping the nodes a process does not host —, and D92's five: `Dag.iterateBytes_eq_impl`
(`Project/Fused.lean`: each connection parsed once, messages bucketed by node, the graph
walked once — on the very function `paxos_safe_wired` is stated over and the harnesses
call), `parseFrames_eq_impl`, `getNat_eq_impl` (LEB128 without `Nat.shiftLeft`), `frameAll_eq_impl` (left fold)
and `frameMsgs_eq_impl` (frames written in place), and D97's `bucketArray_eq_impl` (`Project/Fused.lean`: a batch's messages consed onto an array of `n` buckets by their node number, where `groupRev` hashed every message into a map and rebuilt every pair)). D96's staged activations
(`paxosRun`, `BenchDemo.benchRun`) are not seams: each is a plain definition with a
kernel-checked theorem `name.eq` to `Dag.iterateBytes`, and `paxos_safe_wired` is stated
over `paxosRun` itself (`paxosBody`); the count is nineteen since D98 (eighteen at D97). The
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
(700k; D95 stage 2: between 600M and 700M with the kernel synchronous — the three knots'
`K_strict₁`/wf stacks at the lowered corner and the cascade validity of its block; 455M at
stage 1) and `Paxos/PaxosCore.lean` (500k; between 400M and 500M at D95 stage 2; D91: ≈ 251M,
stage 1: ≈ 312M through the two knot wfs, 77M/65M) — because one `hydro def` command elaborates
the def AND every declaration it generates (knot stacks, couplings, wf, the graph and
its lemmas; ≈500M raw for `leader_election` before D100, no single theorem above 90M) and Lean
counts them together. An override is a measured fact: report it with its
per-declaration table (`scripts/heartbeat_table.sh <file>`), and treat a change that
needs a third site or a higher value as an engine blowup to diagnose (`trace.profiler`;
D56's keyed mono resolver, D72's keyed rule dispatch `hydro_rule_apply`, D81's
`hydro_wf_close` head rule and reducible `kabstract` — 3.6 G and 419 M heartbeats found
and removed rather than budgeted), never a number to raise. The benchmark modules
(`PaxosBench.lean`, the deepest nesting in the tree) carry no override: 122 M / 85 M raw.
The stager (`hydro_run`, D96) carries none either: it runs its three phases — the symbolic
execution, the compilation of the result, the kernel bridge — each under its own default
budget (`withCurrHeartbeats`, what Lean gives each declaration of a block; the three are
three declarations' work) and prints each phase's raw heartbeats in its info line:
`benchRun` 149 M / 110 M / 163 M, `paxosRun` 74 M / 50 M / 103 M — report a phase above 200 M as a
blowup to diagnose (D96 found and removed three: `whnfCore`'s projection-with-delta, a
nested unreduced argument re-reduced by smart unfolding, and `toLCNF` re-`whnf`ing an
uncached dependent type per subterm).

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
