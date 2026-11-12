# Adding a Module

Adding a Rust function means writing its decision records, its contract and its program in one file, checking what `hydro def` generates, and passing the gate. The worked example is `Stream::timeout` (`hydro_lang/src/live_collections/stream/mod.rs`, mirrored in `hydro_lean/Hydro/Std/Timers.lean`): it is small, it has a `sliced!` block with state, and it has two `nondet!` sites, so every step below has something to show. The constructs used here are covered in [Writing Programs](../../docs/docs/hydro-lean/using/programs/signature.md) and the proof mechanisms in [Proving Programs](../../docs/docs/hydro-lean/using/proving/index.md); this page explains the order of work and the checks between the steps.

## 1. File and Imports

A module is one `hydro def`, and a file holds one module or a few related ones: library functions under `Hydro/Std/`, `hydro_test` programs under `Examples/` (Paxos under `Examples/Paxos/`). The `lean_lib` `Hydro` in `lakefile.toml` globs `Hydro.**`, so `lake build Hydro` builds every file under `Hydro/` whether or not anything imports it; the `lean_lib` `Examples` lists its globs (`Examples.Paxos.+`, `Examples.Tutorials.+`, …) so that the tutorial steps and the executables' roots stay out of it, and a new file directly under `Examples/` gets its own entry there. Add an `import` to `Hydro.lean` (a library module) or `Examples.lean` (an example) as well, so that `import Hydro` or `import Examples` (what the executables use) sees the module.

`Timers.lean` imports what any module needs:

```lean
import Hydro.Values
import Hydro.Duration
import Hydro.HydroTick
import Hydro.HydroDef
```

A module that calls another module must import it. The generator builds the caller's graph from the callee's generated graph, so a callee that is not elaborated first fails with ``project: callee … has no `….dag` (phase order)``.

## 2. Records and Contract

The order in a file is fixed: the decision and schedule records, the vocabulary the contract needs, then the contract structure, then the program.

**The decision record** has one field per `nondet!` site whose choice changes what the program computes at `Values`. The field's type is the decision family for that construct, and its docstring quotes the Rust site:

```lean
structure TimeoutDec (H : HydroSem L mem) (n : Nat) (α : Type) where
  /-- `use::batch(self, nondet!(/** sampling timing … */ nondet))`. -/
  arrivals : H.BatchDec n α .noOrder .atLeastOnce
  /-- `use::time(nondet!(/** the verdict depends on when the tick runs */ nondet))`. -/
  clock : H.ClockDec n
```

**The schedule record** holds what changes when things happen but not what the program computes at `Values`: delivery and timers. At `Values` its fields are `Unit`. `sample_every` in the same file has one, with the trivial record that callers at `Values` pass:

```lean
structure SampleEverySched (H : HydroSem L mem) (n : Nat) where
  /-- `self.location.source_interval(interval)`. -/
  interval : H.IntervalDec n

def SampleEverySched.triv {n : Nat} : SampleEverySched (Values L mem) n := ⟨()⟩
```

Which family a Rust construct takes:

| Rust | field type | counted as |
|---|---|---|
| `.batch(…, nondet!)`, `use::batch` | `H.BatchDec n α ord ret` | nondet |
| `.snapshot(…, nondet!)`, `use::snapshot` | `H.SnapDec n α ord` | nondet |
| `.assume_ordering(nondet!)` outside / inside a tick | `H.OrderSelDec n α` / `H.TickOrderSelDec n α` | nondet |
| `use::time(nondet!)` | `H.ClockDec n` | nondet |
| `use::snapshot` of a bounded singleton | `H.TicksDec n` (per-member tick counts) | nondet |
| `.broadcast`, `.demux`, `.send` | `H.TransportDec p c` | sched |
| `source_interval` | `H.IntervalDec n` | sched |
| a `forward_ref` cycle | `H.FixDec` (via `fix … via dec.fuel`) | fuel |

**The contract** is a structure of `Prop` fields over the module's inputs, its decision record and its output, stated at `Values`. Each field is one guarantee:

```lean
structure TimeoutEnsures {α : Type} [DecidableEq α] (ℓ : L) (duration : Nat)
    (arrivals : MemberMap (mem ℓ) (RetryPool α))
    (dec : TimeoutDec (Values L mem) (mem ℓ) α)
    (out : MemberMap (mem ℓ) (CutDec (Option Unit) .totalOrder → Trace (Option Unit))) : Prop where
  fresh_not_expired : ∃ verdict : MemberMap (mem ℓ) (Trace (Option Unit)),
    (∀ i, out i = latestCell (verdict i)) ∧
    ∀ (i : Fin (mem ℓ)) (u : Nat) (b : RetryPool α),
      (retryBatchCuts (arrivals i) (dec.arrivals i))[u]? = some b →
      poolIsEmpty (ord := .noOrder) (ret := .atLeastOnce) b = false →
      ∀ e, (verdict i)[u]? = some e → e = none
```

State a field in the vocabulary a consumer will use. A field that only restates the program gives a consumer nothing; `fresh_not_expired` says that a tick whose batch is nonempty is never expired, a fact about the timer that a caller (`p_leader_heartbeat` calls `timeout` on the heartbeat stream) can use without unfolding `timeout`.

## 3. The Program

The body transcribes the Rust function line by line, with the Rust quoted above each Lean line:

```lean
hydro def timeout {α : Type} [DecidableEq α] (H : HydroSem L mem) (ℓ : L)
    (arrivals : H.Stream ℓ α .noOrder .atLeastOnce) (duration : Nat)
    (dec : TimeoutDec H (mem ℓ) α) :
    H.Singleton ℓ (Option Unit) (Option Unit) .totalOrder .exactlyOnce .unbounded
  ensures out => TimeoutEnsures ℓ duration arrivals dec out :=
  -- sliced! {
  --   let new_arrivals = use::batch(self, nondet!(…));
  --   let now = use::time(nondet!(…));
  --   let mut latest_received = use::state_null::<Optional<Instant, Tick<_>, Bounded>>();
  tick (state latest_received : Option Nat := none)
      (input new_arrivals := H.batch arrivals dec.arrivals)
      (input now := H.time dec.clock) :=
    …
    rebind (latest_received := stamped)
    emit (expired := expired);
  -- }   — the block's `Optional<(), Tick, Bounded>` leaves it as `Optional<(), L, Unbounded>`: `.latest()`
  H.latest expired
  prove
    fresh_not_expired := ⟨expired, fun _ => rfl, fun i u b hb hne e he => by
      obtain ⟨na, nw, hna, _hnw, he⟩ := (hexpired_at i u e).mp he
      …
      simp [expired_step, den, hne]⟩
```

`hexpired_at`, `den` and the facts `prove` has in scope come from the program text, not from a library:

- `hexpired_at` is a **reader** the `tick` block generates: tick `u`'s emission is the block's step function (`expired_step`) applied to the block's state before `u` (its **register**, here `latest_received`) and the inputs at `u`.
- `den` is the simp set that unfolds an operator to its `Values` meaning, one operator at a time.
- `prove` closes each contract field with every reader and every **ghost fact** (a proof stated at a program point with `ghost have`) in scope.

[Tick invariants](../../docs/docs/hydro-lean/using/proving/tick-invariants.md) covers readers and loop invariants; [Proofs over Values](../../docs/docs/hydro-lean/using/proving/proofs-over-values.md) covers `den` and ghost facts. If the module calls another module, the callee enters the proof through its contract (`callee.ensures …`), never by unfolding it.

## 4. Smoke Tests and the Census

After the program, test the denotation on concrete data. The denotation is executable, so a smoke test applies the module to a decision record and compares the result with the expected output. The quorum tests mirror the Rust unit tests in `quorum.rs`:

```lean
-- `collect_quorum_functionality` (quorum.rs), min 2 of max 3:
-- key 1 reaches quorum with 2 Oks …
#guard (collect_quorum (Values Unit oneLoc) () (.mk' fun _ =>
    ({(1, .ok ()), (1, .ok ())} : Multiset (Nat × Except Nat Unit)))
    2 3 (fun _ => [{(1, .ok ()), (1, .ok ())}])).1 0
  = {1}
```

`#guard` decides by kernel reduction. Keyed reads (`Finset.max`, keyed views) are cheap to run and very slow to reduce, so for those use `#guard_eval`, which runs the compiled evaluator and fails the build on `false`.

Then the census, one line that pins how many decision fields of each kind the module's signature has ([Nondeterminism](../../docs/docs/hydro-lean/using/programs/nondeterminism.md#the-census)):

```lean
#nondet_census timeout (nondets := 2) (scheds := 0) (fuels := 0)
```

Read the expected counts off the Rust function, by the table in step 2, not off your records: the census exists to catch a record that dropped, merged or invented a site. `timeout` shows the case that most often goes wrong. Its one `nondet` guard parameter is forwarded to two sites that choose independently, `use::batch` and `use::time`, so the count is two. Schedule sites count whether or not the Rust marks them with a `nondet!`, and a callee's fields count in its caller. [Fidelity](FIDELITY.md#the-census) has the table for every mirrored module.

## 5. Checking the Generated Artifacts

Build the file (`lake build Hydro.Std.Timers`, or `lake env lean Hydro/Std/Timers.lean`) and inspect the artifacts in a scratch file with `#check`. The full list is in [Generated artifacts](../../docs/docs/hydro-lean/internals/generator/generated-artifacts.md); the table lists the ones whose shape to check:

| artifact | check |
|---|---|
| `M.ensures` | the contract at every input and every decision record, with no premise beyond `requires` |
| `M_co_wfᵢ` | the coupling's residual obligation ([Transferring to the Machine and the Network](../../docs/docs/hydro-lean/using/proving/transferring-to-the-machine.md#the-lemmas-you-cite)); its premises should be only the inputs' `wf` and the `requires`. An extra conjunct points at a fold whose prerequisite needs a `ghost pre` ([Proofs over Values](../../docs/docs/hydro-lean/using/proving/proofs-over-values.md)) |
| `M_co_srᵢ`, `M_co_rrᵢ`, `M_vdec` | the equations a machine theorem rewrites `sr` and `rr` with: the machine run `sr` equals the module's `SchedSem` run, and the denotational run `rr` equals the module's `Values` run at the derived decision record `M_vdec`, which takes the machine inputs and the schedule fields |
| `M_monoᵢ` | at `Values`, outputs grow with inputs, in the order of each stream's grade (prefix for `TotalOrder`, sub-multiset for `NoOrder`, and so on) |
| `M.dag`, `M.dag_denote`, `M.dag_valid` | the module's dataflow graph, read with `SchedSem`'s definitions (`Dag.denote`), equals the module at `SchedSem`; for a module with a knot (a `fix` cycle), `dag_valid` says every cycle crosses a network hop or `defer_tick` |

For `toy_relay` (`Hydro/HydroGenToy.lean`), the residual's only premise is the input's `wf`:

```lean
@toy_relay_co_wf₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Tc Td : ℕ} {hjT : Tc.le Td} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce)
  (dec : ToyRelayDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyRelaySched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  inp.4 → (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.wf
```

When generation fails, the error states the cause. The common messages:

```text
HydroGen: a lag-2 agreement cannot be read at lag 1
HydroGen: {what}: the body's output lag is {μ}, not `0` — the body is not strict in the knot wire (a cycle without a network hop or `defer_tick`)
project: callee {c} has no `{dagName}` (phase order)
project: operator {c} not at the machine instance: {e}
```

The first two are Rust's rule for `forward_ref` cycles, checked while the `fix` elaborates: a cycle must cross a network hop or a `defer_tick` ([Cycles](../../docs/docs/hydro-lean/using/programs/ticks-and-cycles.mdx#cycles) explains both messages). The Rust compiler enforces the same rule, so if the Rust cycle compiles and the Lean one is rejected, the transcription dropped the hop or the `defer_tick`.

## 6. Heartbeats

The engine carries no heartbeat budget, and a new file should elaborate under Lean's default. One `hydro def` command elaborates the def and every declaration it generates, and Lean counts them together, so a large module can exceed the default. Two program files carry a measured override (`Paxos/LeaderElection.lean` 700000, `Paxos/PaxosCore.lean` 500000). Before adding a third, find out which declaration is expensive:

```bash
scripts/heartbeat_table.sh Hydro/Std/Timers.lean 30
```

It runs the file with the generator's diagnostics on (`HYDRO_PHASE_DIAG`, `HYDRO_GHOST_DIAG`, `HYDROGEN_HB_DIAG`) and prints every generated theorem, ghost fact and knot-stack phase with its heartbeats and wall time, largest first. A single declaration far above the rest is an engine blowup to diagnose ([Elaboration and Kernel Cost](../../docs/docs/hydro-lean/internals/cost-model.md)), not a budget to raise.

## 7. Machine Theorem and Axiom Audit

A module that is a headline gets a machine theorem: the four moves of [Transferring to the Machine and the Network](../../docs/docs/hydro-lean/using/proving/transferring-to-the-machine.md) with its artifacts, then the pool premises removed by truncation and `M.dag_denote`.

Every headline, and every contract a later module consumes, is registered in `Hydro/AxCheck.lean` (a library module, such as `Hydro/Std/`) or `Examples/AxCheck.lean` (an example program) in the section that matches it, e.g. in `Examples/AxCheck.lean`:

```lean
-- the machine-run safety headlines, premise-free
#print axioms Hydro.paxos_co_wf
#print axioms Hydro.paxos_co_cpl
#print axioms Hydro.paxos_safe_sched'
#print axioms Hydro.two_pc_co_cpl
#print axioms Hydro.two_pc_safe_sched'
…
```

The gate greps the build log for these lines ([The Gate](GATE.md#3-axioms)), and `scripts/reach.lean` reads the audited names from both files' text as roots of its reachability census, so a declaration nothing reaches is reported as a deletion candidate.

## 8. The Gate

Run [the gate](GATE.md) in full. For a new module the checks that matter most are the build (every `#guard`, census and generated artifact), the axiom grep, and the per-file time: a new file above 15 s is reported with its `heartbeat_table.sh` output.
