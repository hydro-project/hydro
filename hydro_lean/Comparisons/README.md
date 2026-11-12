# Comparisons

The quickstart's voting quorum (`vote_quorum`, `Examples/Tutorials/Quorum.lean`) written as a
TLA+ specification and as a Veil module, for the page
`docs/docs/hydro-lean/using/comparison.mdx` and the section "Compared with a TLA+ Proof" of
`docs/docs/hydro-lean/using/quickstart/proving-quorum.mdx`. Neither file is part of the Hydro
Lean lake package: `lake build` does not see them.

| file | what it is |
|---|---|
| `VoteQuorum.tla` | the protocol as a TLA+ state machine, its three safety properties, the inductive invariant `Inv`, a liveness property under weak fairness |
| `VoteQuorum.cfg` | TLC: every reachable state of 3 participants, 2 requests, `MinYes = 2`, every voting rule |
| `VoteQuorumInd.cfg` | TLC: `Inv` is inductive (every `Inv` state of 2 participants and 2 requests, one step) and implies safety |
| `VoteQuorumLive.cfg` | TLC: `Decides` under `Fairness`, on the instance of `VoteQuorum.cfg` |
| `VoteQuorumVeil.lean` | the protocol as a Veil 2.0 module: `sat trace`, `#model_check`, `#check_invariants`, `#temporal_check` |

## Running TLC

```bash
curl -sSL -o tla2tools.jar https://github.com/tlaplus/tlaplus/releases/download/v1.8.0/tla2tools.jar
java -cp tla2tools.jar tlc2.TLC -config VoteQuorum.cfg VoteQuorum.tla
java -cp tla2tools.jar tlc2.TLC -config VoteQuorumInd.cfg VoteQuorum.tla
java -cp tla2tools.jar tlc2.TLC -config VoteQuorumLive.cfg VoteQuorum.tla
```

Recorded with TLC2 Version 2026.11.06.222357 (rev: d8fb231), 8 workers:

```text
VoteQuorum.cfg
Finished computing initial states: 64 distinct states generated
Model checking completed. No error has been found.
370816 states generated, 50176 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 10.
Finished in 01s

VoteQuorumInd.cfg
Finished computing initial states: 104976 distinct states generated
Model checking completed. No error has been found.
839104 states generated, 104976 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 1.
Finished in 07s

VoteQuorumLive.cfg
Checking 2 branches of temporal properties for the complete state space with 100352 total distinct states
Model checking completed. No error has been found.
370816 states generated, 50176 distinct states found, 0 states left on queue.
Finished in 06s
```

In `VoteQuorumInd.cfg` every successor of an `Inv` state is again one of the 104976 initial
states (depth 1, no new state). Each conjunct of `Inv` is needed: with any one removed, TLC
reports `Inv` or a safety property violated on that run. Without `WF_vars(Tick)`, the liveness
check reports `Temporal property Decides was violated`. With the filter against `decided`
removed from `Tick` (`fresh == reached`), `VoteQuorum.cfg` reports `Invariant Once is violated`
with a six-state trace: a request proposed, two yes votes cast, and two ticks that both emit it.

There is no TLAPS proof: TLC checks the finite instances above, not every size.

## Running Veil

`VoteQuorumVeil.lean` builds inside a Veil checkout (tested at commit
`22cbf1affc1c4b6ab3fe290e773313dba7499165`, Lean `v4.35.0`):

```bash
git clone https://github.com/verse-lab/veil.git && cd veil && lake build
mkdir -p Examples/Hydro && cp <this dir>/VoteQuorumVeil.lean Examples/Hydro/
lake build Examples.Hydro.VoteQuorumVeil
```

Recorded output (the satisfying trace and the per-clause lines shortened):

```text
info: Examples/Hydro/VoteQuorumVeil.lean:122:0: ✅ Satisfying trace found
  Instantiation:
    participant = Fin 1
    qset = Fin 1
    request = Fin 1
  …
  State 4 (via tick):
    …
    decided = {0}
    emitted = {0}
    dup = {}
info: Examples/Hydro/VoteQuorumVeil.lean:127:0: ✅ No violation (explored 270400 states)
info: Examples/Hydro/VoteQuorumVeil.lean:130:0: Initialization must establish the invariant:
  doesNotThrow ... ✅
  decided_proposed ... ✅
  quorum ... ✅
  once ... ✅
  …
The following set of actions must preserve the invariant and successfully terminate:
  propose
    …
  tick
    doesNotThrow ... ✅
    decided_proposed ... ✅
    quorum ... ✅
    once ... ✅
    req_proposed ... ✅
    vote_honest ... ✅
    arrived_honest ... ✅
    seen_honest ... ✅
    emitted_decided ... ✅
info: Examples/Hydro/VoteQuorumVeil.lean:132:0: ✅ No liveness violation (checked 1 properties, explored 770 product states)
Build completed successfully (1276 jobs).
```

All 45 checks (eight clauses and `doesNotThrow`, for initialization and each of the four actions)
are ✅ with `veil.smt.trust false`: Veil reconstructs a Lean proof of each instead of trusting the
solver's answer (with the default `true`, the check ends with a `Trusting SMT solver for … goals`
warning). The build takes about 70 s, most of it compiling and running the two checkers; without
`#model_check` and `#temporal_check` it takes about 9 s. Each `invariant` clause is needed: with
any one removed, `#check_invariants` reports a ❌ (for example, without `emitted_decided`, `tick`
fails `once`). Without the fairness assumption on `deliver`, `#temporal_check` reports
`❌ Violation: liveness_failure (violates: decides)`.
