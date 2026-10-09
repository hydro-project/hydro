# hydro_lean

[Hydro](https://hydro.run) programs written in Lean 4 — as **line-for-line
mirrors** of the Rust sources in this repository (`hydro_test/src/cluster/paxos.rs`,
`hydro_std/src/quorum.rs`, …) — with **unbounded correctness proofs**: every input
size, every materialization of the protocol's nondeterminism, every schedule of the
concurrent step machine. The proofs are written *inside* the programs, over the
denotational semantics, in a shape designed to be ported to
[Verus](https://github.com/verus-lang/verus) proofs in the Rust tree later
(`DOCTRINE.md` keeps the translation table).

Zero `sorry`. Every headline theorem is axiom-audited to Lean's standard three
(`propext`, `Classical.choice`, `Quot.sound`) in `Hydro/AxCheck.lean`, and every
headline has an executable or structural witness (`GATE.md`).

## What is here

| | |
|---|---|
| `Hydro/` | the library: semantics, engine, the verified `hydro_std` stages (`Std/`) and the Paxos port (`Paxos/`) |
| `docs/` | **the guided tour** — read in order if you know Hydro and some Lean but have never seen this tree |
| `ARCHITECTURE.md` | reference: the signature, the interpretations, the surface syntax, the generation pipeline, the module map |
| `DOCTRINE.md` | the binding rules for programs and proofs, in the words they were ruled in; the Verus translation table |
| `CORRESPONDENCE.md` | how a denotational contract becomes a theorem about the concurrent step machine (the coupling corner); what is deliberately not proven; the trust base |
| `SCHED_AUDIT.md` | red-team fidelity ledger: the step machine vs the Rust runtime, finding by finding |
| `LIVENESS.md` | design of record for liveness under fairness (decision chains); what is proven, what is queued |
| `GATE.md` | the per-change acceptance gate, as executable steps |
| `ENGINE_NOTES.md` | elaboration gotchas for anyone touching `HydroDef`/`HydroGen*`/`HydroTick` |
| `FINDINGS.md` | the D-numbered ledger (D1–D68): every design decision, bug, measurement and ruling, in order. History lives here; the other docs cite it by number |

## Build

```bash
# one-time toolchain (Mathlib is a dependency; the cache download is large)
curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -o elan-init.sh
ELAN_HOME="$HOME/.elan" sh elan-init.sh -y --no-modify-path --default-toolchain none
export PATH="$HOME/.elan/bin:$PATH"
elan toolchain install "$(cat lean-toolchain)"

lake exe cache get      # Mathlib oleans
lake build              # the `Hydro` library (lakefile glob `Hydro.**`) — ~6 min, LeaderElection is the long pole
```

If the toolchain's bundled `clang` needs a newer glibc than the host has, set
`LEAN_CC` to a wrapper around the system `cc` that adds `-L <toolchain>/lib`.

`#guard`/`#eval` checks run during compilation — a green `lake build` means every
in-file test passed and every `#nondet_census` matched. The executables (all at the
`Eager` interpretation, milliseconds each; run them one at a time):

```bash
lake exe falsify   # the B1/B2 falsifications: the faithful variants commit two values at one slot, the guarded ones do not
lake exe explore   # the D21 causality record (acausal vs causal control)
lake exe paxos     # end-to-end guarded commit: one proposer, one acceptor, f = 0
lake exe twopc     # the 2PC commit-iff witness
```

## How to read a module

Every program file has the same shape (`DOCTRINE.md` §layout):

1. **Prerequisites for the contract face** — only what the `ensures` type needs:
   the module's decision record (`…Dec`, the Rust `nondet!` sites), its scheduling
   bundle (`…Sched`, the adversary's cursors — `Unit` at `Values`), the vocabulary the
   contract is stated in, and the `…Ensures` structure itself.
2. **`hydro def M … ensures out => MEnsures … out :=`** — the program: a `let`-chain
   mirroring the Rust function body with the Rust line quoted above each line;
   `tick (state …)` blocks for `sliced!`/`use::state`; `fix … complete` for
   `forward_ref` cycles. Proof text is interleaved as `ghost have`s at the program
   points it concerns and as `invariant` clauses on loops; the contract's fields are
   discharged in the trailing `prove` section.
3. **Smoke tests** (`#guard` runs at `Values`/`Eager`) and the module's
   `#nondet_census`.

Start with `Hydro/Std/Quorum.lean` (`collect_quorum`, one `tick` block, one loop
invariant) next to `hydro_std/src/quorum.rs:88–160`; then `Hydro/TwoPC.lean` (a
consumer composing by contract alone); then `Hydro/Paxos/PaxosCore.lean` (the knots
and the K4 invariant). `docs/01-from-rust-to-lean.md` walks the first of these.

## The headline theorems

| theorem | says | where |
|---|---|---|
| `paxos_core.ensures … .slot_functional` (field of `PCEnsures`) | at `Values`, under the guarded variant and `mem acc ≤ 2f+1`, the replica stream is slot-functional — Paxos safety, for all inputs and all decisions | `Hydro/Paxos/PaxosCore.lean` |
| `paxos_safe_sched'` | the same agreement for the **concurrent step machine**: any pacing, any delivery schedule, any horizon, no satisfiability premise, no decision witness | `Hydro/Paxos/CoupleWf.lean` |
| `paxos_eager_den`, `paxos_eager_den_ballots`, `paxos_eager_commits` | what `lake exe paxos` computes **is** the `Values` run the headline quantifies over (the module's free theorem at the eager relation) | `Hydro/Paxos/EagerCheck.lean` |
| `collect_quorum.ensures` (`CQEnsures`: `emit_sound`, `emit_count`, `fails_eq`), `cq_safe_sched'` | the shared quorum stage: emitted keys hold `min` `Ok` votes; crossing is an iff, once; the machine-run version | `Hydro/Std/Quorum.lean` |
| `cq_live_chain` (= `cq_live`), `collect_quorum_tight` | liveness: under fair ticking a key that has reached quorum is eventually emitted by the machine; the machine observation at the derived decision is exact | `Hydro/LivenessChain.lean` |
| `two_pc.ensures` (`TPCEnsures`), `two_pc_unanimous`, `two_pc_once`, `two_pc_commit_iff` | 2PC over the shared quorum stage: unanimity, at-most-once, commit-iff | `Hydro/TwoPC.lean` |
| every module's `<M>_mono…`, `<M>_param…`, `<M>_co_wf…` | generated per module by `hydro def`: Flo monotonicity, the H-relative free theorem, the machine/denotation coupling well-formedness | the module's file, after the def |

The gate that keeps all of this true on every change is `GATE.md`.

## Relationship to the Rust tree

The Lean programs are not a re-implementation; they are the Rust programs under a
different interpretation of the same operators. The 1:1 discipline is enforced by
review (`DOCTRINE.md` §program text) and by the `#nondet_census` of each module, which
must equal the Rust `nondet!` tally at that function. Where the formalization found
something about the Rust code — candidate bugs (B1/B2/B3, executable in
`lake exe falsify`), implicit contracts, stale comments — `FINDINGS.md` records it and
`SCHED_AUDIT.md` keeps the open upstream-report list.
