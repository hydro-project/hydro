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
  `HydroLean/Flo/AxiomCheck.lean` and `HydroV2/AxCheck.lean`.

## Build

```bash
# One-time toolchain setup (HydroV2 uses Mathlib; `lake exe cache get` fetches it):
curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -o /tmp/elan-init.sh
ELAN_HOME="$HOME/.elan" sh /tmp/elan-init.sh -y --no-modify-path --default-toolchain none
export PATH="$HOME/.elan/bin:$PATH"
elan toolchain install "$(cat lean-toolchain)"

lake build   # compiles every module under HydroLean/ (lakefile glob; no manual imports)
```

`#guard`/`#eval` tests (the Lean mirror of Rust sim tests — same specs, concrete
schedules) run during compilation; a green `lake build` means all of them passed.
The executables (all at the `Eager` interpretation, milliseconds each):
`lake exe falsify` (the B1/B2 falsifications), `lake exe explore` (the D21
causality record), `lake exe v2paxos` (the guarded-commit non-vacuity
witness), `lake exe v2twopc` (the 2PC commit-iff witness) — (if the
toolchain's bundled
`clang` needs a newer glibc than the host has, set `LEAN_CC` to a wrapper
around the system `cc` that adds `-L <toolchain>/lib`).


## HydroV2: the graded tagless-final surface

`HydroV2/` is the second-generation surface: programs are written **once**
against the `HydroSem` signature (Rust-named operators; grades =
ordering/retry markers select the carrier quotient), and every artifact
about a program is an *instantiation* of the same text —

- **`Values`** — the graded denotation (realized runs over all ticks);
  demo executable: `lake exe v2paxos` (end-to-end guarded commit).
- **`MonoRel`** — the diagonal relational gluing: the program instantiated
  at `MonoRel` IS its Flo-monotonicity proof (two-liner per program).
- **Colocated contracts** (FINDINGS D27) — each module returns a subtype
  whose clause is the dependent implication "if the instantiation is
  `Values`, the output satisfies `…Ensures`"; the proof lives inside the
  definition, over the body's own `let`s.

The V2 headline (FINDINGS D28) is again `paxos_core`'s type:
`variant = .guarded → mem acc ≤ 2f + 1 → SlotFunctional out.2`, proved
end-to-end from module contracts (`HydroV2/Paxos/PaxosCore.lean`; audit:
`HydroV2/AxCheck.lean` — standard three axioms; zero sorries).

## Tour: where each result lives

| Result | File |
|---|---|
| Flo metatheory: determinism + eager execution (Lemma 2.4.3), unique stuck state | `HydroLean/Flo/Theorems.lean` |
| Streaming progress for graphs (Lemma 2.4.4) | `HydroLean/Flo/Theorems.lean`, `Flo/Progress.lean` |
| Cluster upgrade lawfulness (Fig 3.5/3.6) | `HydroLean/Gyatso/Multibuffer.lean` |
| Network operators + `fold_commutative` lawfulness (§3.5) | `HydroLean/Gyatso/Network.lean` |
| Monotone outputs (Thm 3.4.2), eventual determinism (Thm 3.4.1) | `HydroLean/Gyatso/Correctness.lean` |
| Everything below this row: the **HydroV2** generation (tagless-final signature, graded quotient carriers, five+ interpretations) — see `HydroV2/README.md` | `HydroV2/` |
| Nondeterminism as instance-declared decision vocabularies | `HydroV2/Sem.lean`, `HydroV2/Decisions.lean` |
| `collect_quorum`(+`_with_response`) shared verified stage | `HydroV2/Std/Quorum.lean`, `Std/QuorumTheory.lean` |
| `two_pc` (over the shared quorum stage): unanimity, at-most-once, commit-iff + `lake exe v2twopc` witness | `HydroV2/TwoPC.lean`, `V2TwoPC.lean` |
| `index_payloads`, `join_responses` | `HydroV2/Paxos/IndexPayloads.lean`, `HydroV2/Std/RequestResponse.lean` |
| **Paxos safety** — the headline IS `paxos_core`'s type (`variant = .guarded → nA ≤ 2f+1 → SlotFunctional`), closed onto machine runs at every schedule, premise-free (`paxos_safe_sched'`) | `HydroV2/Paxos/PaxosCore.lean`, `HydroV2/Paxos/CoupleWf.lean` |
| Paxos port (1:1 files per Rust function) + executable bug falsifications (`lake exe falsify`) | `HydroV2/Paxos/`, `HydroV2/Paxos/Falsification.lean`, `Falsify.lean` |
| Causality: `leader_election`'s colocated `stable` face; the D21 acausal-fabrication record (`lake exe explore`) | `HydroV2/Paxos/LeaderElection.lean`, `HydroV2/Paxos/Exploration.lean` |
| Fast verified execution: the `Eager` interpretation (materialized `Values`), pinned by generic projection theorems | `HydroV2/Eager.lean`, `EagerProj.lean`, `HydroV2/Paxos/EagerCheck.lean` |
| The V1 decisions-as-inputs generation (retired; ported to HydroV2 — recover from jj history) | `docs/10-decisions-as-inputs.md` (historical) |
| Choreographic layer (historical; code deleted — recover from jj history) | `docs/08-choreographic-layer.md`, `docs/09-configuration-calculus.md` |
