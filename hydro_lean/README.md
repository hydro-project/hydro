# hydro_lean

[Hydro](https://hydro.run) programs written in Lean 4 as line-for-line mirrors of the Rust
sources in this repository (`hydro_std/src/quorum.rs`, `hydro_test/src/cluster/paxos.rs`,
`two_pc.rs`, the Paxos benchmark and its replicas), with safety proofs that hold for every
input, every resolution of the program's nondeterminism, every schedule of the step machine,
and every byte-level trace of the deployed processes. No `sorry`; every theorem rests on
`propext`, `Classical.choice` and `Quot.sound` only (`Hydro/AxCheck.lean`, `Examples/AxCheck.lean`).

The documentation is on the Hydro website, section **Hydro Lean (experimental)**; its sources
are in this repository under `docs/docs/hydro-lean/`:

- `using/` — **Using Hydro Lean**: introduction, setup, a tutorial translating
  `collect_quorum` from Rust, writing programs (locations and networking, ticks and cycles,
  carriers, decisions, contracts), proving inside programs, machine
  theorems.
- `internals/` — **Framework Architecture & Internals**: the approach, how proofs are
  generated, the semantics, the coupling corner, the generator phase by phase, the graph and
  execution layers, soundness and the trust base, liveness, traps, and the design doctrine.

`internal/` holds the documents for contributors:

- `internal/FIDELITY.md` — how each module transcribes its Rust function and how the census
  checks the transcription, where the Lean departs from the Rust, what the step machine and the
  trace model cover, and the findings B1 to B6 in the Rust code.
- `internal/ADDING_A_MODULE.md` — the order of work for adding a Rust function, with the
  checks between the steps.
- `internal/GATE.md` — the checks a change to this directory passes before it lands: the
  build, the greps over the repository and the build log, and the executables with their
  expected output and running times.
- `internal/FINDINGS.md` — the design ledger: every decision, measurement and ruling in order.

## Layout

- `Hydro/` — the framework (library `Hydro`, root `Hydro.lean`): the semantics, the generator,
  the graph and execution layers, the standard library `Hydro/Std/`, the engine's toys and checks.
- `Examples/` — the programs built on it (library `Examples`, root `Examples.lean`): Paxos
  (`Examples/Paxos/`), 2PC (`Examples/TwoPC.lean`), the quickstart's programs
  (`Examples/Tutorials/`) and their intermediate stages (`Examples/TutorialSteps/`, library
  `HydroTutorialSteps`), and the executables' entry points (`Examples/Demo/`).
- `Comparisons/` — the quickstart's quorum in TLA+ and Veil; `scripts/` — the census and
  profiling scripts.

## Build and run

```bash
curl -sSf https://elan.lean-lang.org/elan-init.sh | sh -s -- -y --no-modify-path --default-toolchain none
export PATH="$HOME/.elan/bin:$PATH"
elan toolchain install "$(cat lean-toolchain)"
lake exe cache get          # Mathlib
lake build Hydro Examples HydroTutorialSteps falsify explore paxos twopc wired bench benchprof quorum
```

A green build means every `#guard`/`#guard_eval` passed and every `#nondet_census` matched.
Then, one at a time:

```bash
lake exe falsify            # the unguarded paxos.rs commits two values at one slot; the guarded one does not
lake exe explore            # an acausal decision script cannot fabricate a leader
lake exe paxos              # one proposer, one acceptor, f = 0: a payload is committed
lake exe twopc              # the vetoed payload never commits
lake exe wired              # paxos_core as five processes over local TCP
lake exe bench 500 10 6000  # paxos_bench as ten processes over TCP: throughput and latency
```

`internal/GATE.md` gives the expected output and running time of each executable.
