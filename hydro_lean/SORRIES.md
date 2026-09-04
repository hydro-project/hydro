# Tracked `sorry`s

None. The three Flo metatheorems (Lemmas 2.4.2, 2.4.3, 2.4.4) in
`Flo/Theorems.lean` are fully proven.

Deferred (statement not yet formalized, no `sorry` in code):

| Item | Owner | Plan |
|------|-------|------|
| Output *maximality* half of graph streaming progress (Def 2.3.2 lifted to graphs, second bullet of Lemma 2.4.4) | metatheory follow-up | state via `Operator.OutputsMaximal` analogue on graph configs; the canonical-run machinery in `Flo/Progress.lean` (`progress_aux`) already provides the needed decomposition |

Status note (decisions-as-inputs wave, completed): zero `sorry` throughout;
deliverable (c) Paxos agreement is **done** — the headline
`commit_agreement` (cross-proposer slot-functionality of `p_to_replicas`
for the guarded variant, over the full decision space) is proven in
`Programs/Paxos/Safety.lean` over the real `paxos_core`, with the axiom
audit in `Programs/Paxos/AxCheck.lean` (`propext`/`Classical.choice`/
`Quot.sound` only) and the executable falsifications passing
(`lake exe falsify`; the causality record `lake exe explore`). The former
leader-ballot-stability hypothesis is now derived (`le_ballot_stable`,
FINDINGS D21) — the headline's only input is `nA ≤ 2f + 1`. The
operational-lens transport (wiring uniformization) is explicitly out of
scope per ACCEPTANCE §7.
