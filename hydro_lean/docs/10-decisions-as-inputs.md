# The decisions-as-inputs architecture

**Status**: user-ratified final architecture (supersedes both the
clock-shaped `ConfigInv*` organization AND the earlier fragment/Choreo
encoding — all legacy Paxos files are deleted; recover from jj history if
needed). Programs are literal Lean functions 1:1 with the Rust signatures;
nondeterminism is explicit decision inputs; `forward_ref` is a higher-order
fixpoint; monotonicity is type-derived (`Hydro/Growth.lean`). **The plan
below is fully executed**: the headline `commit_agreement` is proven in
`Programs/Paxos/Safety.lean` (axiom audit in `Paxos/AxCheck.lean`).

## The semantics (all landed, build green, zero sorry)

- `Hydro/TStream.lean`: `TStream α = List (List α)` (ticks × in-tick
  collection, Flo §2.5), `TSing β` per-tick singletons; per-tick operators;
  **blocking** zips/`crossSingleton`/`filterIf` (multi-tick-input operators
  emit only up to jointly-realized ticks — Flo streaming progress, never a
  partial tick); scans (`scan`/`scanSt` = `use::state`/`across_ticks` faces)
  with `getElem`/`append`/`prefix` lemmas; `TickLoop.states/outputs` lifts +
  `mem_outputs_elim` (the provenance eliminator).
- **Batch/snapshot sites by ordering marker**: `batch` (TotalOrder: demand
  counts, prefix cuts), `batchC` (NoOrder+ExactlyOnce: the decision IS the
  consumed batch, multiset-legal against the `unionF` member-union —
  adversary order = the `assume_ordering` shuffle), `batchD`/`snapshotD`
  (AtLeastOnce: membership-legal, duplication free), `snapshotC` (NoOrder
  fold snapshots: accumulated adversary-ordered increments). Realized-view
  lemmas ("what can appear") + count/membership provenance + monotonicity
  (prefix in ⇒ prefix out, with decisions fixed). Illegal decisions BLOCK
  (semantics), so causality facts are stated as realized-⇒-legal lemmas.
- `Hydro/ForwardRef.lean`: `forward_ref fuel init body` (Kleene iteration;
  fuel is an adversarial decision), `iterate_ind` (fixpoint induction — THE
  generic induction), `iterate_chain`/`iterate_rel` (chains under
  body-preserved relations), `memoF` (semantic-identity family memoization —
  operational only), and the **gas-less layer**: `iterate_fixed_of_bounded`
  (bounded strictly-growing measure ⇒ true fixpoint), `forward_ref_of_bounded`
  + bridges `forward_ref_ge_budget` (fuel ≥ budget gives THE fixpoint run) and
  `forward_ref_le_budget` (below-budget runs are ⊑-prefixes ⇒ prefix-closed
  safety transfers to all fuels).
- `Hydro/Growth.lean` + `Hydro/MonoSing.lean` (**final form** of the typed
  discipline): the `Growth` (⊑) class's carrier instances are the ordering
  markers (List = prefix for TotalOrder, `Cnt` = count-domination for
  NoOrder+ExactlyOnce, `Mem` = membership for AtLeastOnce; families
  pointwise); bundled monotone maps `α →ₘ β` with one bundled primitive per
  framework combinator and **reader-lifted wire combinators**
  (`w.map/.zip/.batchC/.member/…`, defeq to `∘ₘ` chains) so module bodies
  read in Rust method-chaining order; cycles close with `MonoMap.fix`/
  `fix₀` (generic `fixHist_chain`/`fixHist_rel` faces). Every module's
  dataflow exists **once**, as its typed stage; `MonoSing` wires (Rust's
  `Monotonic` singleton bound) carry value-ascent in the stage signatures
  via `fold_monotonic` (the `monotone =` closure obligation paid at the
  definition site); `.f` appears only at boundaries (run views, face
  statements, the falsifier). No hand-written growth lemmas remain.

## The program (1:1, function-per-file; paxos.rs line cites inline)

- `Programs/CollectQuorumStreams.lean`: `collect_quorum` /
  `collect_quorum_with_response` over member-indexed families + faces:
  `collect_quorum_distinct_members` (emitted key ⇒ ≥ min distinct members,
  hypothesis = ≤1 success per member per key),
  `collect_quorum_with_response_providers`,
  `collect_quorum_with_response_mem` (provenance, contract-free),
  `wrRun_emit_mem`.
- `Paxos/BallotCalc.lean`: `p_ballot_calc` + face (`p_ballot_calc_own`,
  `_hasLargest` (≡ true), `_getElem`; ballot-number ascent is the stage's
  `MonoSing` output type, not a lemma).
- `Paxos/PLeaderHeartbeat.lean`: `p_leader_heartbeat` (no safety clauses —
  discharge by absence) + ownership.
- `Paxos/PP1b.lean`: `p_p1b` (WR slice + `snapshotC` view + `pP1bView`);
  the bucket calculus (`pP1bView_some_bucket`,
  `foldEarlyStop_count`, …) intact.
- `Paxos/AcceptorP1.lean`/`AcceptorP2.lean`: single-tick transcriptions +
  run-level faces (`ap1_max_mono/pinsMax/echo/okCnt/okOnce`,
  `ap2_ackLb/okCnt/okOnce/voteEcho/logEcho`,
  `acceptor_p2_step_ok_max`) + `acceptor_p1_ticks`/`acceptor_p2_ticks`
  (blocking-zip lifts; max/coverage ascent are `MonoSing` wires in the
  stage signatures).
- `Paxos/LeaderElection.lean`: `leader_election` (Rust signature; internal
  3-cycle `LERef` via one `forward_ref`; stages `le_*` one def per Rust let;
  `sendGuard`/`dedupLast` = the guarded B1 send-once).
- `Paxos/SequencePayload.lean`: `sequence_payload` (Rust signature;
  `sp_send_step` scan = recommit→index→payloads_to_send with the guarded B2
  gate; stages `sp_*`).
- `Paxos/PaxosCore.lean`: `paxos_core` (callback + `PaxosNondet`; outer
  2-cycle `PaxosRef`; body = the Rust let-chain; spec projections
  `pcLE/pcJust/pcBallots/pcSP` + `paxos_core_body_fst/snd` rfl-bridges).
- `Paxos/Falsification.lean`: the B1 script (two proposers, duplicate P1a,
  fake 1-acceptor quorum ⇒ x and y both committed at slot 0; guarded blocks
  it) + the B2 module-face witness. **Executable**: `lake exe falsify`
  (63 ms via the IO-materialization harness in `Falsify.lean` — FINDINGS
  D14); verified outputs recorded in FINDINGS.md.

## The proof stack (landed — colocated: every fact lives with the
function that owns the data)

- **Monotonicity is type-derived** (`Hydro/Growth.lean`, Step 0): the
  `Growth` class's carrier instances are the ordering markers (prefix /
  count / membership; families pointwise); bundled `→ₘ` maps close under
  composition; each module's dataflow exists **once**, as a `→ₘ` stage
  written with the reader-lifted wire combinators
  (`leader_election_bodyM`, `sequence_payloadM`, `paxos_core_bodyM`,
  `p_ballot_calcM`, …), cycles close with `MonoMap.fix` (generic
  `fixHist_chain`/`fixHist_rel` faces), and `paxos_core_hist_chain` is
  `iterate_chain` through the typed body. Value-level (`Monotonic`
  singleton) growth is `Hydro/MonoSing.lean`'s `fold_monotonic`, surfaced
  as `MonoSing` wires in the stage signatures (ballot number, `a_max`,
  coverage) — tick ordering projects from the type
  (`MonoSing.tick_lt_of_not_le`), not from per-stage lemmas.
- `AcceptorP1.lean`/`AcceptorP2.lean`: output eliminators
  (`acceptorP1_out_elim`, `acceptorP2_out_elim`, `mem_ap2Acks_of_tick`)
  + the run faces (`ap1_*`, `ap2_*`).
- `LeaderElection.lean`: `dedupLast_num_lt` ⇒ **`le_p1a_nodup`** (guarded
  send-once, B1) + `le_p1a_own`.
- `Recommit.lean`: output structure
  (`recommit_eq/_ballot/_none/_slot_le/_slots_nodup` over
  `recommitToCommit/recommitHoles/recommitMax`).
- `SequencePayload.lean`: `sp_send_step_guarded_eq` (the guarded step via
  `spJustB/spRecEn/spRm/spBase`); **`spKeys_inv`** — the guarded P2a key
  discipline: over any `SPChain`-respecting input, emitted `(slot, ballot)`
  keys are nodup and owned. `SPChain` packages: ballot ownership,
  num-monotonicity, leader ⇒ nonempty quorum view, and leader-ballot
  stability (a proposer that stays leader across consecutive ticks keeps
  its ballot — paxos.rs:186–189's `nondet!` guarantee, **derived** as
  `leader_election`'s contract `le_ballot_stable`, FINDINGS D21 — no
  longer a proof input). The module exports its **contracts** on its own
  signature: the abstract witnesses `SPEmission`/`SPChosen` and
  `sp_commit_spec`/`sp_emission_spec`/`sp_log_entry_spec`/
  `sp_emission_functional`, over any carrier satisfying
  `SPWireDiscipline` (exactly `leader_election`'s output contracts).
- `AcceptorP1.lean`/`AcceptorP2.lean` export their contracts at the ticks
  signature (`ap1t_ok_spec`/`ap1t_reply_echo`/`ap1t_reply_dst`/
  `ap1t_decode_cap`; `ap2t_ok_spec` — write-before-ack on the published
  log output — /`ap2t_log_entry`/`ap2t_decode_cap`); `PP1b.lean` exports
  `pP1bPv_ballot_stable`/`pP1bPv_no_false_full` given a solicitation
  oracle.
- `PaxosCore.lean` (wiring + contract instantiation only): the
  `sequence_payload` carrier at each cycle (`pcG`/`spInputs`),
  `run_discipline` (the `SPWireDiscipline` discharge from
  `leader_election`'s contracts), the contract instantiations
  (`run_leader_providers`/`run_leader_view_promise`/`run_emission_zero`),
  **`alog_succ`** (the `a_log` knot at the signature), wire growth along
  the chain (`pcG_le`), and **`run_promise_covers` (K1)** — zero event
  induction.

## Remaining plan — **ALL COMPLETED** (see `Programs/Paxos/Safety.lean`)

0. ✅ **Type-level monotonicity**: `Hydro/Growth.lean` (`Growth` class,
   carrier instances = the ordering markers, bundled `→ₘ` maps, one bundled
   primitive per combinator) + `Hydro/MonoSing.lean` (`fold_monotonic`,
   `MonoSing` = Rust's `Monotonic` singleton bound). Every module's dataflow
   is a `→ₘ` composition `rfl`-equal to its transcription
   (`leader_election_bodyM`, `sequence_payloadM`, `paxos_core_bodyM`); the
   hand-written `*_prefix` stacks are deleted; `paxos_core_hist_chain` =
   `iterate_chain` through the typed body. FINDINGS D13.
1. ✅ **Run-level chain instantiation**: `SPChainQ` (SPChain + frozen-bucket
   pinning) constructed **inside `sequence_payload`** (`spTicks_chain`)
   from the `SPWireDiscipline` hypotheses, which `PaxosCore.run_discipline`
   discharges from `leader_election`'s contracts (`le_out_ballot_own`,
   `le_ballot_stable` — the trigger's `!p_is_leader` gate crosses the
   `forward_ref` cycle, FINDINGS D21 —, `le_leader_view_promise`,
   `le_view_pinned`); the pinned-view map (`spPin`, classical choice) is
   SP-internal, well-defined by the discipline's pin clause ←
   `pP1bPv_qlogs_pinned` ← `foldEarlyStop_full_pin` through
   `snapshotC_getElem_prefix`.
2. ✅ **K2/K3 counting**: `leB1_flatten_nodup` (guarded B1 + ownership
   disjointness) → `ap1t_decode_cap`/`leRs_cap`; `spB2_keys_nodup` →
   `ap2t_decode_cap`/`sp_p2b_cap`; `SPChosen` via `sp_commit_spec`
   (`collect_quorum_distinct_members` + `ap2t_ok_spec`) and
   `le_leader_providers`/`run_leader_providers`
   (`collect_quorum_with_response_providers` through `foldEarlyStop_count` +
   `snapshotC_view_count`).
3. ✅ **K4**: `emission_chosen_agree` — the ONE `Nat`-induction over the
   unfolding depth, with the **`spCovered_inv`** scan invariant (an emitted
   key whose slot is covered by its ballot's pinned quorum-view merge
   carries the merged max-ballot value; fresh emissions never land on
   covered slots) absorbing the fresh/recommit case split, quorum
   intersection (`nA ≤ 2f+1`), K1 at a common index, and the m₃ provenance
   regress `k+1 → k` through `alog_succ`/`ap2t_log_entry`/`batchC_mem`. Base
   case `pcP2As_zero` (the empty cycle realizes no acceptor tick).
4. ✅ **Agreement**: `commit_elim` (through the new unconditional
   `joinResponses_emit_mem`) + `commit_agreement` in `Paxos/Safety.lean`;
   axiom audit in `Paxos/AxCheck.lean` (standard axioms only).
5. ✅ **Wrap-up**: falsification compiled to `lake exe falsify` (63 ms; B1
   disagreement + B2 duplicate keys verified, outputs in FINDINGS.md).
   Running it exposed and fixed a **model vacuity**: the `a_log` knot as
   first modeled was deadlocked (max wire blocked on the log); the max wire
   is now the productive batch fold, as in paxos.rs (FINDINGS D15). The
   compiled-code memoization hazard is FINDINGS D14.

## Design rulings to preserve (user-ratified)

- Decisions are data; blocked ticks model unavailable cuts; safety = ∀
  decisions. Where the loose cut space would admit acausal traces, check
  the loop's trigger first: if it routes through a `forward_ref` cycle
  wire, the causal fact is **derivable** (FINDINGS D21 — derive it, don't
  hypothesize); only gateless loops need a causality face or
  `Hydro/CausalAvail.lean`'s `causalTickLoop`.
- `forward_ref` is the only cycle mechanism; fuel quantified; gas-less via
  budget measure when needed.
- Theorems live at the module owning the data; `paxos_core`-level files
  contain wiring, handoffs, and the final assembly only.
- No event/schedule induction anywhere; the K4 `Nat`-induction over
  unfolding depth is the single protocol induction.
