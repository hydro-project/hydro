# 07 — Liveness as decision chains: `cq_live_chain`

Safety said: whatever the machine emits is covered by the denotation. Liveness asks:
does the machine *eventually* emit what the denotation says it should? Here fairness is
not a property of machine steps but a class of **chains in decision space** — and the
protocol content still lives entirely at `Values`. Design of record: `../LIVENESS.md`.

## The idea in one paragraph

The machine run at horizon `T` corresponds (chapter 06) to a derived decision `d(T)`,
and derivation is horizon-monotone — longer runs consume more. So an infinite behavior
of the machine denotes an **ascending chain** `d(0) ≤ d(1) ≤ …` of decisions. Temporal
operators become quantifiers over chain positions (`ChEventually P := ∃ n, P n`,
`ChAlways`, … — `Hydro/LivenessChain.lean:72–`); fairness is a property of the chain;
"the state at a point" is the `Values` run at that decision. Every liveness theorem then
factors as **V ∘ K1 ∘ K2**: a `Values` fact at good decisions, a transfer lemma "fair
schedules produce good chains", and a tightness lemma "the machine's output at `T` *is*
the `Values` run at `d(T)`".

## The theorem

```lean
-- Hydro/LivenessChain.lean:433–450
theorem cq_live {pacing : Unit → Fin 1 → Nat → Bool}
    (h : Fin 1 → StepHist (K × Except E Unit)) {T₀ : Nat}
    (hstab : StabilizesAt (h 0) T₀)
    (mn mx : Nat) (k : K)
    (h1 : 1 ≤ mn) (hmm : mn ≤ mx)
    (hcap : cqKeyCount (Multiset.ofList ((h 0).view T₀)) k ≤ mx)
    (hq : mn ≤ cqOkCount (Multiset.ofList ((h 0).view T₀)) k)
    (hsupply : ∃ Tw, T₀ ≤ Tw ∧ pacing () 0 Tw = true) :
    ∃ T, k ∈ ((collect_quorum (SchedSem Unit (fun _ => 1) pacing) ()
      h mn mx ()).1 0).view T := by
  obtain ⟨Tw, hT₀Tw, hpTw⟩ := hsupply
  refine ⟨Tw, ?_⟩
  -- (V): the `Values` run at the (complete) derived chain point has `k`.
  have hval := cq_complete_mem
    (fun _ => Multiset.ofList ((h 0).view T₀))
    (fun i => cqDerivedChain (h i) (pacing () i) Tw) mn mx k h1 hmm
    (cqDerivedChain_complete_at hstab hT₀Tw hpTw) hcap hq
  -- (tightness): that run IS the machine's wire at `Tw`.
  rw [← collect_quorum_tight h hstab mn mx Tw] at hval
  exact Multiset.mem_coe.mp hval
```

Premises: the input history stabilizes at `T₀` (`StabilizesAt`, `TransferTheory.lean`);
the usage caps of the contract; `k` has reached quorum in the stabilized pool; and one
tick happens at or after `T₀` (`hsupply` — the `FairTicks` shape `∀ n, ∃ t ≥ n, p t`
specialized to the one tick the proof needs). Conclusion: the machine emits `k` by some
horizon. `cq_live_chain` (`:455`) is the same theorem under its chain-factored name.

## The three factors

- **(V)** `cq_complete_mem` (`:203`): at any *complete* batch decision — one whose
  consumed pool is the whole input — the quorum key is in `collect_quorum`'s `Values`
  output. Pure contract reasoning: `collect_quorum.ensures … .emit_count` with the cap
  (chapter 03). Along chains: `cq_chain_live` (`◇ emitted` along any exhausting chain,
  `:221`), `cq_chain_live_stable` (`◇□`, since legal ascending chains make the
  observation persist). **All protocol content is here**, and none of it mentions the
  machine.
- **(K1)** `cqDerivedChain` (`:261`) is the chain the machine's run induces — the
  per-tick batches as multisets, `batchesFrom hi.view (tickSteps p T) 0` — and
  `cqDerivedChain_complete_at`/`cqDerivedChain_exhausts` (`:328`) say: once the input
  has stabilized, a fair skeleton (`FairTicks p`, `Liveness.lean:62`) drives the derived
  chain to exhaust the pool (`Exhausts v c := ChEventually fun n => cqConsumed v (c n) =
  v`, `:164`). This is the only place schedule vocabulary appears, and it is generic per
  operator.
- **(K2)** `collect_quorum_tight` (`:350`): the machine's output at horizon `T` equals
  (as a multiset) `collect_quorum` at `Values` at the derived chain point. Proven from the
  **generated** coupling artifacts — `collect_quorum_co_wf₁` (the `tick` block's body
  coupling, `CoTick.scanWf`), `CoTick.scan_comm` (the machine former coerced is the
  denotation former on coerced inputs), `corr_batch` (the derived decision is exact at
  its horizon), with the corner's `Values` leg named at the explicit derived decision by
  `co_transfer [collect_quorum]`. No hand mirror of the block (D65 deleted the one that
  used to anchor this proof).

## Why this is the right shape

- It is the TLA+ story with the state machine replaced by the denotation
  (`../LIVENESS.md` §Rosetta): `WF(tick)` is the exhaustion class; `◇□` is persistence
  along legal chains; enabledness is a predicate on the `Values` run at a chain point.
- It dodges the classical "fairness is not denotational" obstruction: the semantics
  stays finite-horizon and fueled; fairness lives in the logic over chains.
- Zero changes to `Sched.lean`, `Values.lean`, or any safety artifact — fairness only
  shrinks a quantifier safety leaves universal.
- **Non-vacuity** (`DOCTRINE.md` R7): `fairTicks_odd` is a fair witness that is not the
  generous schedule; `#guard`s compute the derived chain's exhaustion point; an `example`
  discharges all premises jointly.

## What is next on this ladder (queued)

Paxos-scale **quiescent-completeness**: inputs stabilize + fairness ⟹ the derived chain
exhausts every consumption site (K1 composed through `paxos_core`'s five knots) ⟹ the
`Values` run at the exhausted point is the total denotation (completeness clauses on
the contracts) ⟹ the machine attains it (a generic tightness kit generalizing
`collect_quorum_tight`). Then commit liveness under leader stability (`ChStabilizes` of
the election component). The `EmitDec` supply premise that once complicated this is
gone with the decision itself (D60–D61).
