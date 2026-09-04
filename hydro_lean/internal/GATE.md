# The Gate

A change to `hydro_lean/` is accepted when it passes the gate: the build, a set of greps over the repository and the build log, and the executables. Each check exists because a green `lake build` alone does not establish what the theorems claim. A file can elaborate with a `sorry`, with a theorem resting on an extra axiom, with the compiler running code other than the definitions, or with a model that admits no behaviour. This page lists the checks, the failure each one catches, and the expected output. The toolchain setup is in [Setup](../../docs/docs/hydro-lean/using/setup.md).

Run everything from `hydro_lean/`. Run the executables one at a time: concurrent `lake exe` invocations race on the build directory.

## 1. Build

```bash
lake build Hydro Examples HydroTutorialSteps falsify explore paxos twopc wired bench benchprof quorum 2>&1 | tee build.log
```

About 1975 jobs, 6 to 10 minutes on 32 cores. A green build means every `#guard` and `#guard_eval` passed, every `#nondet_census` (a module's count of decision fields by kind, [§5](#5-censuses-and-heartbeats)) found the counts its line states, and every `hydro def` generated its artifacts; each of those is a build error on failure.

The heaviest library file, `Paxos/PaxosBench`, peaks near 3.4 GB. A file whose build suddenly needs much more memory or time usually runs a definitional-equality check that unfolds a module application or the wire of a **knot** (a `fix … complete` cycle) instead of rewriting with its generated equations. That check's cost grows exponentially with knot depth ([Elaboration and Kernel Cost](../../docs/docs/hydro-lean/internals/cost-model.md)). To measure one file, and to make such a blowup fail fast while you debug it, run it under a memory cap:

```bash
/usr/bin/time -v lake env lean -M 4500 -DElab.async=false Examples/Paxos/<F>.lean
```

(`PaxosBench` needs the asynchronous default.) Per-file wall time is part of the gate. The reference envelope, with every file not listed at most 30 s:

| file | at most |
|---|---|
| `Paxos/PaxosBench`, `WireTraceChecks` | 150 s |
| `Project/Model/WiredMain` | 110 s |
| `Paxos/LeaderElection` | 90 s |
| `Project/Process` | 85 s |
| `Project/Model/DistributedModel`, `Project/Model/WireTrace`, `Project/Model/StepOps` | 70 s |
| `Paxos/PaxosCore` | 65 s |
| `Paxos/SequencePayload` | 60 s |
| `Project/Model/SweepSpec` | 55 s |
| `HydroTickCheck` | 50 s |
| `Paxos/PaxosWithClient`, `Paxos/WiredSafety` | 35 s |
| the executable roots that run `hydro_run` (a process's per-wake function compiled to straight-line code): `BenchDemo`, `WiredDemo`, `BenchProfile` | 60 s, 35 s, 30 s |

A file above its line, or a new file above 15 s, is reported with its `trace.profiler` breakdown and `scripts/heartbeat_table.sh` output.

## 2. No `sorry`

```bash
grep -rn "sorry" Hydro/ Examples/ *.lean --include="*.lean" --exclude-dir=TutorialSteps | grep -v -i "vacuous\|-- \|prose"
```

prints nothing (the tutorial steps, `Examples/TutorialSteps/`, are the excluded library `HydroTutorialSteps`). A `sorry` in an audited theorem would also show up as `sorryAx` in its axiom report (§3); the grep covers declarations the audit does not list, such as a lemma only an executable uses.

## 3. Axioms

`Hydro/AxCheck.lean` (the framework and the standard library) and `Examples/AxCheck.lean` (the example protocols and the tutorials) print the axioms of every headline theorem: the contracts, the hoisted loop invariants, the machine-run theorems, `paxos_safe_wired`, the staged activations' equations, the `@[csimp]` equations, the coupling combinators and the graph layer. With the checks in `HydroGenCheck`, `HydroTickCheck` and `LivenessChain`, the build log carries 140 reports. Each must be a subset of `[propext, Classical.choice, Quot.sound]`:

```bash
grep -i "depends on axioms" build.log | grep -v -E "\[propext(, Classical\.choice)?(, Quot\.sound)?\]|\[Classical\.choice(, Quot\.sound)?\]|\[propext, Quot\.sound\]|\[Quot\.sound\]"   # prints nothing
grep -ci sorryAx build.log        # 0
grep -ci ofReduceBool build.log   # 0
```

`sorryAx` is an unproved step. `Lean.ofReduceBool` is the axiom `native_decide` introduces: it trusts compiled code to agree with the kernel. The executables exist to witness that agreement, not to assume it. `propext`, `Quot.sound` and `Classical.choice` are Lean's standard foundation; the `Multiset` and `Finset` theory of Mathlib that the carriers of `Values` build on uses all three.

A new headline is added to the matching `AxCheck.lean` in the same change; [Adding a Module](ADDING_A_MODULE.md#7-machine-theorem-and-axiom-audit) shows where.

## 4. No Unverified Seams

The executables (`falsify`, `wired`, `bench`) are witnesses that the definitions the theorems are about behave as claimed on real data. That holds only if the compiled code is the definitions. These mechanisms let the compiler run something else with no proof, and none is used:

```bash
grep -rn "implemented_by\|@\[extern\|native_decide\|^unsafe \|ofReduceBool" Hydro/ Examples/ *.lean
```

returns exactly one line, the doc comment in `Hydro/MemberMap.lean` that says these are not used. The compiler may substitute one term for another only through `@[csimp]`, whose argument is a kernel-checked equation `f = fImpl`. There are nineteen:

```bash
grep -rn "@\[csimp\] theorem" Hydro/ Examples/ | wc -l    # 19
```

| file | equations | why the implementation differs |
|---|---|---|
| `Hydro/MemberMap.lean` | `mk'_eq_mkImpl` | the per-member map is built as a vector, so each entry is computed once instead of at every read |
| `Hydro/Project/Wired.lean` | `memoBox_eq_memoBoxImpl`, `memoBox2_eq_memoBox2Impl`, `Dag.locStep_eq_impl` | memo tables the compiler materializes; the per-process step skips nodes at locations the process does not host |
| `Hydro/KeyedHash.lean` | `dedupH_eq_dedupImpl`, `arrFilterNotIn_eq_impl`, `arrAntiJoin_eq_impl`, `arrJoinH_eq_impl`, `keyedFoldAssocH_eq_impl`, `keyedFoldArrH_eq_impl`, `keyedReduceAssocH_eq_impl`, `SchedKMap.joinH_eq_impl`, `reduceWatermarkEagerH_eq_impl` | hash-backed keyed operators over the machine's list semantics |
| `Hydro/Wire.lean` | `getNat_eq_impl`, `frameAll_eq_impl` | LEB128 decoding and framing on byte buffers |
| `Hydro/Project/Process.lean` | `parseFrames_eq_impl`, `frameMsgs_eq_impl` | frame parsing and encoding in one pass |
| `Hydro/Project/Fused.lean` | `bucketArray_eq_impl`, `Dag.iterateBytes_eq_impl` | the fused activation: one sweep instead of separate walks |

Each equation changes how fast something runs, never what it computes. A new `@[csimp]` changes the count and is reviewed for that property.

Some of what the compiler does is outside every theorem: whether it shares a value or recomputes it. A sharing leak shows up as time, so the runtimes in §6 are part of the check. The staged activations (`paxosRun`, `BenchDemo.benchRun`, compiled by `hydro_run`; see [Staging the Activation](../../docs/docs/hydro-lean/internals/execution/staging.md#the-hydro_run-command)) are not seams: each is an ordinary definition with a kernel-checked equation `name.eq` to `Dag.iterateBytes`.

## 5. Censuses and Heartbeats

**Censuses.** There are 42 `#nondet_census` lines (`grep -rn "^#nondet_census" Hydro/ Examples/ *.lean | wc -l`), and the build enforces each ([Nondeterminism](../../docs/docs/hydro-lean/using/programs/nondeterminism.md#the-census)). A changed count passes the build as long as the line changes with it, so the gate's check is review: a census change is a claim about how the Lean transcribes the Rust's decisions and is reviewed as one, against the Rust function and the per-module values in [Fidelity](FIDELITY.md#the-census).

**Heartbeats.** The engine carries no heartbeat budget. Exactly three sites override the default:

```bash
grep -rn "^set_option maxHeartbeats" Hydro/ Examples/ *.lean --include="*.lean"
```

```text
Examples/Paxos/LeaderElection.lean:143:set_option maxHeartbeats 700000 in
Examples/Paxos/PaxosCore.lean:105:set_option maxHeartbeats 500000 in
Examples/Demo/BenchProfile.lean:149:set_option maxHeartbeats 600000 in
```

The two program sites exist because one `hydro def` command elaborates the def and every declaration it generates, and Lean counts them together; each value is the measured sum. `BenchProfile`'s is on its staged activation. A change that needs a new site or a higher value is an engine blowup to diagnose with `trace.profiler` and `scripts/heartbeat_table.sh <file>`. `hydro_run` runs its three phases (stage, compile, kernel) each under its own default budget and prints each phase's heartbeats; a phase approaching the default budget is diagnosed the same way.

## 6. Executables

| command | expected | what it witnesses |
|---|---|---|
| `lake exe falsify` | last line `All falsification checks PASSED.` | the faithful `paxos.rs` variants violate slot functionality (B1, B2, B4) or lose a slot (B5); the guarded ones do not |
| `lake exe explore` | `acausal script agreement: true`, `causal control agreement: true` | acausal decision freedom cannot fabricate a leader |
| `lake exe paxos` | last line `paxos: OK` | one proposer, one acceptor, `f = 0`: election, a payload sequenced and committed |
| `lake exe twopc` | last line `twopc: OK` | the vetoed payload never commits |
| `lake exe wired` | last line `wired: OK` (ports 9400–9404 free) | `paxos_core` as five processes over TCP running `paxosBody` |
| `lake exe wired --traces` | ten `wired --traces: <check>: true (… ms)` lines, the lazy schedule, `wired --traces: OK` | the `WireTrace` scenarios; compiled body and interpreter agree |
| `lake exe bench 500 10 6000` | `Throughput: …` ≈ 220–240k requests/s, p50 ≈ 4.3 ms, last line `bench: OK` (ports 9500–9509 free) | the whole benchmark as ten OS processes |
| `lake exe benchprof decode 50` | four `ns/msg` lines; `Wire.decode (P2b)` ≈ 40 ns | the codec's cost |
| `lake exe benchprof idle 0 300` | the leader's empty activation, compiled ≈ 55–60 µs | the staged activation's cost |

`falsify`, `explore`, `paxos` and `twopc` evaluate programs at the denotational semantics `Values` and finish in well under a second; a run that takes seconds means a value is being recomputed instead of shared. A `bench` run that completes no request prints `bench: FAILED (no request completed)` and exits 1; ports still bound by a previous run cause that. For the long run:

```bash
/usr/bin/time -v .lake/build/bin/bench 500 10 20000
```

Throughput stays steady over 20 s, each process's step time in the last quarter is within about 15 % of the first, and the launcher's RSS is about 190 MB. Per-process memory is flat (`ps -o rss,args` on the `bench --proc P` processes): a process holds no history, so growing memory is a leak.

`paxos_safe_wired` quantifies over all traces. The `wired --traces` scenarios check that the trace model is not vacuous and that the compiled body commits what the interpreter commits, which tests the compiler's output where the theorems stop at the definitions ([Wire Codecs and Byte-Level Traces](../../docs/docs/hydro-lean/internals/execution/wire-and-traces.md#witnesses-for-the-model)).

## 7. Statements

The headline statements do not change as a side effect: `paxos_safe_sched'`, `paxos_safe_sched`, `paxos_safe_wired`, `cq_safe_sched'`, `cq_safe_sched`, `cq_live`, `cq_live_chain`, `two_pc_safe_sched`, `two_pc_once_sched`, and every `…Ensures` structure another module consumes. Diff them against the previous revision. A proof can be repaired by weakening what it proves, and the build stays green when it is; only reading the statement catches that.
