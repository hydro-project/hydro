# hydro_lean

A Lean 4 formalization of [Hydro](https://hydro.run)'s semantic foundations and a
proof-oriented surface language for writing Hydro programs in Lean and proving
**unbounded correctness**: all input sizes, all materializations of nondeterminism.

**Start with the documentation in [`docs/`](docs/00-overview.md)** — architecture,
the Lean APIs layer by layer, the proof methodology, the Paxos capstone, and a
Rust ↔ Lean ↔ paper glossary.

- `docs/00-overview.md` — project overview and reading order.
- `DESIGN.md` — architecture decisions and the Rust-parity naming policy.
- `ACCEPTANCE.md` — the binding final-state bar.
- `FINDINGS.md` — paper gaps, candidate Rust bugs (with executable falsifications),
  and implicit contracts made explicit by the formalization.
- `SORRIES.md` — deferred work. The codebase has **zero `sorry`**; headline theorems
  are axiom-audited (`propext`/`Classical.choice`/`Quot.sound` only) in
  `HydroLean/Flo/AxiomCheck.lean`, `HydroLean/Programs/AxCheck.lean`, and
  `HydroLean/Programs/Paxos/AxCheck.lean`.

## Build

```bash
# One-time toolchain setup (no Mathlib; core Lean only):
curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -o /tmp/elan-init.sh
ELAN_HOME="$HOME/.elan" sh /tmp/elan-init.sh -y --no-modify-path --default-toolchain none
export PATH="$HOME/.elan/bin:$PATH"
elan toolchain install "$(cat lean-toolchain)"

lake build   # compiles every module under HydroLean/ (lakefile glob; no manual imports)
```

`#guard`/`#eval` tests (the Lean mirror of Rust sim tests — same specs, concrete
schedules) run during compilation; a green `lake build` means all of them passed.
The executable falsifier is `lake exe falsify`, and `lake exe explore`
replays the D21 causality record (if the toolchain's bundled
`clang` needs a newer glibc than the host has, set `LEAN_CC` to a wrapper
around the system `cc` that adds `-L <toolchain>/lib`).

## Tour: where each result lives

| Result | File |
|---|---|
| Flo metatheory: determinism + eager execution (Lemma 2.4.3), unique stuck state | `HydroLean/Flo/Theorems.lean` |
| Streaming progress for graphs (Lemma 2.4.4) | `HydroLean/Flo/Theorems.lean`, `Flo/Progress.lean` |
| Cluster upgrade lawfulness (Fig 3.5/3.6) | `HydroLean/Gyatso/Multibuffer.lean` |
| Network operators + `fold_commutative` lawfulness (§3.5) | `HydroLean/Gyatso/Network.lean` |
| Monotone outputs (Thm 3.4.2), eventual determinism (Thm 3.4.1) | `HydroLean/Gyatso/Correctness.lean` |
| Nondeterminism as data: batchings and decision guards | `HydroLean/Hydro/NonDet.lean` (the full decision surface: `TStream.lean`) |
| `collect_quorum` unbounded correctness (goal a): engine proof + typed-stage face (`collect_quorum_spec`) | `HydroLean/Programs/CollectQuorumProof.lean`, `CollectQuorumStreams.lean` |
| `two_pc` (decisions-as-inputs, over the shared quorum stage): master theorem + corollaries (goal b) | `HydroLean/Programs/TwoPC.lean`, `TwoPCProof.lean` |
| `index_payloads`, `join_responses`: typed stages + verified faces | `HydroLean/Programs/IndexPayloads.lean`, `JoinResponses.lean` |
| The decisions-as-inputs surface: tick streams, batch/snapshot decisions, `forward_ref` fixpoints | `HydroLean/Hydro/TStream.lean`, `ForwardRef.lean`, `docs/10-decisions-as-inputs.md` |
| Type-derived monotonicity (`→ₘ` wire combinators, growth orders = ordering markers, `MonoMap.fix` cycles) + `Monotonic` singleton wires | `HydroLean/Hydro/Growth.lean`, `MonoSing.lean` |
| **Paxos `commit_agreement` (goal c)** — the headline, over the real `paxos_core` | `HydroLean/Programs/Paxos/Safety.lean` (audit: `Paxos/AxCheck.lean`) |
| Paxos port (1:1 files per Rust function) + executable bug falsifications (`lake exe falsify`) | `HydroLean/Programs/Paxos/`, `Falsify.lean` |
| Causality boundary rule: `le_ballot_stable` derived; shelved `causalTickLoop` design for gateless loops | `HydroLean/Programs/Paxos/LeaderElection.lean`, `Hydro/CausalAvail.lean`, `docs/11-causal-availability.md` |
| Choreographic layer (historical; code deleted — recover from jj history) | `docs/08-choreographic-layer.md`, `docs/09-configuration-calculus.md` |
