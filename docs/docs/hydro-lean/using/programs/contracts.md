---
sidebar_position: 5
title: "Contracts"
---

# Contracts

A program is generic over its interpretation `H`, the instance of `HydroSem` it is read at ([The signature](./signature.md)). Its contract is stated at one of them, the denotational semantics `Values`, where a stream captures all the histories it could have had and every choice the program observes is an explicit decision argument ([Nondeterminism](./nondeterminism.md)). There a contract is a statement about pools (one member's stream content) and decisions, with no schedule, no message in flight and no step in sight. The step-machine and process-level theorems are derived from it later, by generated lemmas you do not write.

You write the contract in the definition's result type with `ensures`, and you prove it inside the definition with `prove`, next to the lines it is about, with every intermediate **wire** (a collection-valued `let` or binder of the program) in scope. `hydro def` then splits the definition in two: a plain function, which a caller applies at any interpretation, and a theorem, which the caller applies at `Values`. The contract is called the module's **face**, because it is all that callers see of the module: a `Prop` over its inputs and output. This page covers the syntax and what it elaborates to, how to write a face that callers can use, preconditions with `requires`, and how `two_pc` consumes `collect_quorum`'s face.

## The Face Type

`τ ensures out => P` in the result position elaborates to a subtype whose property holds whenever the interpretation is `Values` (`hydro_lean/Hydro/HydroDef.lean`):

```lean
/-- `τ ensures out => P`: the contract-face former, in the type position of a
`hydro def`. Elaborates to `{out : τ // ∀ hv : H = Values L mem, match … => P}`
with every `H`-dependent binder transported through the match. -/
syntax:10 (name := ensuresTerm) term:11 " ensures " ident " => " term : term
```

`hydro def collect_quorum` elaborates its definition under the name `collect_quorum._spec`. Its type, by `#check`:

```lean
@collect_quorum._spec : {L : Type} →
  {mem : L → ℕ} →
    {K E : Type} →
      [inst : DecidableEq K] →
        [inst_1 : DecidableEq E] →
          [Hashable K] →
            (H : HydroSem L mem) →
              (ℓ : L) →
                (responses : HydroSem.Stream mem ℓ (K × Except E Unit) StrOrd.noOrder Retries.exactlyOnce) →
                  (min max : ℕ) →
                    (dec : HydroSem.BatchDec mem (mem ℓ) (K × Except E Unit) StrOrd.noOrder Retries.exactlyOnce) →
                      { out //
                        ∀ (hv : H = Values L mem),
                          match H, hv, responses, dec, out with
                          | .(Values L mem), ⋯, responses, dec, out => CQEnsures ℓ min max responses dec out }
```

At any `H` other than `Values` the property is vacuous, so the definition is a program at every interpretation. The `match` transports each binder whose type mentions `H` (`responses`, `dec`, `out`), so inside the arm they have their `Values` types: `responses` is a per-member multiset, `dec` is a `BatchCuts`, and `CQEnsures` can name `cqConsumed (resp i) (dec i)`, the part of the response pool the realized cuts consumed.

## The `prove` Clause

The body ends with the computational value followed by `prove` and one proof per field of the face:

```lean
/-- `value prove f₁ := e₁, f₂ := e₂, …`: close an `ensures`-faced def with the
computational value and the field-by-field contract assembly. Elaborates to
`⟨value, by intro hv; subst hv; ‹replayed ghosts›; exact {f₁ := e₁, …}⟩`. -/
syntax proveField := ident " := " term
syntax:10 (name := proveTerm)
  term:11 " prove " proveField,+ : term
```

`subst hv` replaces `H` by `Values L mem` everywhere, so in a proof of one field every `let` of the program is in scope as its `Values` value, and `simp only [w, den]` unfolds a wire `w` one operator at a time to its `Values` value; `den` is the simp set of the operators' `Values` meanings.

A **ghost** is a proof-only clause between the lines of the program (`ghost have`, `ghost obtain`, `ghost let`), stated over the program's own wires at the point where it holds, with no restatement. None of them is part of the computational value. `hydro def` proves a `ghost have` where it stands, as its own theorem `M.ghost.h…`, whether or not the module has a `prove`; the field proofs see it as a hypothesis of the same name. `hydro def` queues the clauses that bind or rewrite (`ghost obtain`, `ghost let`, and any `ghost have` after a `ghost intro`, `ghost subst` or `witness`) instead, and runs them in order after the `subst`, when it elaborates the field proofs. [Proofs over Values](../proving/proofs-over-values.md#ghost-facts) gives the rule and covers `den`.

## Derived Declarations

Consumers never see `M._spec`. `hydro def` derives two declarations from it (`HydroGhost.deriveModule`): the plain program `M`, the pair's value with the proof stripped, callable at any interpretation; and the face theorem `M.ensures`, the property at `Values` without the `hv`/`match` wrapper, for every input and every decision. For `collect_quorum` its conclusion is `CQEnsures ℓ min max responses dec (collect_quorum (Values L mem) ℓ responses min max dec)`; [Generated artifacts](../../internals/generator/generated-artifacts.md#the-module-and-its-face) shows both statements. `M` is a separate, non-dependent function because a result type `{out // … args …}` would bake the callee's arguments into its type, and none of the generated rewrite lemmas could fire under a callee application ([Generated artifacts](../../internals/generator/generated-artifacts.md#the-module-and-its-face)).

## Writing a Face

A face is a `structure` of `Prop` fields, stated before the program. A face that callers can use follows three rules.

**State it over the inputs and the return value, in vocabulary a caller can compute.** A face stated in the callee's internal vocabulary (its fold, its **registers**, the state its `tick` blocks carry across ticks) forces every caller to restate its own wires in that vocabulary before it can apply the face. `CQEnsures` speaks of `cqConsumed (resp i) (dec i)`, `cqOkCount` and `cqKeyCount`: functions of a pool and a decision that any caller can apply to the wire it passed in. The register discipline `CQRegInv` stays inside `Quorum.lean`; no consumer mentions it. A module-internal fact that a proof outside must see is exported existentially (the `tick` construct's `h<out>_reg` hides the register behind `∃ reg`).

**Separate the unconditional fields from the conditional ones.** `emit_sound` (an emitted key holds `min` `Ok` votes among the consumed responses) holds for every input. `emit_count` (a key is emitted exactly once if it crossed `min`) holds under the usage contract of quorum.rs, `1 ≤ min ≤ max` and at most `max` responses per key:

```lean
  emit_count : ∀ (i : Fin (mem ℓ)) (k : K), 1 ≤ min → min ≤ max →
    cqKeyCount (cqConsumed (resp i) (dec i)) k ≤ max →
    (out.1 i).count k
      = if min ≤ cqOkCount (cqConsumed (resp i) (dec i)) k
        then 1 else 0
```

The cap is a condition of the Rust program itself. The `not_all` register holds the window of responses still being counted. At `min < max` a key leaves the window once it has `max` responses and leaves `min_but_not_max` with it; a further response starts a new window with zero counts, and if `min` more `Ok`s arrive the key is emitted again. At `min = max` the window drops the key on emission, and the next `min` `Ok`s re-emit it. A quorum of `max` participants that each answer once never exceeds the cap; a caller that can exceed it gets no exactly-once guarantee, and the face says so.

**Use implication for premises about the arguments.** `two_pc`'s face holds when the `num_participants` argument is the participant cluster's size:

```lean
  ensures out => num_participants = mem part →
    TPCEnsures (mem part) vote payloads out :=
  ghost intro hn
  ghost subst hn
```

`ghost intro` binds the premise as `hn` and `ghost subst` substitutes it into the goal and every later ghost, so the body's proofs see `mem part` where the program says `num_participants`.

Some properties are not faces. Monotonicity (outputs grow when inputs grow) is a relation between two runs; the generator derives it as `M_mono₁` by instantiating the program at `MonoRel`, the interpretation whose carriers are pairs of runs, one below the other. Causality is a property of the production graph, the dataflow graph `hydro def` generates for the executable processes (`M.dag_agreeL`). Completeness of a pipeline (a unanimous, fully consumed vote eventually commits) is relative to the pool each decision consumes, which for `two_pc`'s second stage is an internal wire; it is stated as liveness ([Liveness](../../internals/liveness.md)).

## Preconditions with `requires`

A fold whose closure commutes only under a protocol fact carries that fact as its `Pre` ([Fold Proofs](./signature.md#fold-proofs)). The program must show that the content reaching the fold satisfies `Pre`. It does so with a `ghost pre` at the site, usually from a precondition on the module's inputs written `requires`, between the result type and `ensures`:

```lean
hydro def toy_pre (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ (Nat × (Nat × Nat)) .noOrder .exactlyOnce)
    (wm : H.Atomic ℓ (Option Nat))
    (dec : H.BatchDec (mem ℓ) (Nat × (Nat × Nat)) .noOrder .exactlyOnce) :
    H.Stream ℓ (Nat × (Nat × Nat)) .noOrder .exactlyOnce
  requires (∀ i, ∀ a ∈ inp i, ∀ b ∈ inp i, a.1 = b.1 → ToyTie a.2 b.2)
  ensures _out => ToyPreEnsures inp dec :=
  let batched := H.batch inp dec
  let keyed := H.tinto_keyed batched
  -- the site fact, before the operator: the accumulated content at every key
  -- satisfies the tie (from the module's `requires`)
  ghost pre hpre : ∀ i, (KeyedPool.accum (keyed i)).Pairwise ToyTie :=
    fun i => KeyedPool.accum_tinto_keyed_batch_pairwise inp dec i (hreq i)
  let m := H.reduce_watermark keyed wm toyMerge ToyTie toyMerge_comm
```

`toy_pre` (`hydro_lean/Hydro/HydroTickCheck.lean`) reproduces the Paxos acceptor log's merge at small scale: `toyMerge_comm` proves that the closure commutes on pairs satisfying `ToyTie`, and `ghost pre hpre` proves, over the program's wire `keyed`, that every two values meeting at a key satisfy `ToyTie`, using the `requires` clause as `hreq`.

The `requires` clause does not appear in the face, and `M.ensures` has no premise for it. At `Values` a conditional fold is defined on every input, because it refuses content that violates `Pre` and yields its seed, so the face can be proved, and used, unconditionally. The step machine folds its lists in arrival order and refuses nothing, so the two semantics agree only on content that satisfies `Pre`. The clause is therefore a premise of the generated coupling lemma `M_co_wfᵢ`, the lemma a machine theorem uses to relate the machine run to the denotational run, and a caller discharges it at its call site with its own `ghost pre`. [Proofs over Values](../proving/proofs-over-values.md#requires-and-ghost-pre) covers the packaging of `ghost pre` facts, the three-level chain in Paxos, and how the generator discharges them.

## Consuming a Callee's Face

`two_pc` (`hydro_lean/Examples/TwoPC.lean`) runs `collect_quorum` twice, on the votes and on the acknowledgements. Its face is stated over its input `payloads`, the participants' `vote` function and its output only:

```lean
structure TPCEnsures {nC : Nat} (n : Nat) (vote : Fin n → P → Bool)
    (payloads : Fin nC → Multiset P) (out : Fin nC → Multiset P) : Prop where
  /-- Soundness: a committed payload was proposed. -/
  commit_proposed : ∀ c, ∀ p ∈ out c, 1 ≤ n → p ∈ tpcProposed payloads
  /-- Unanimity, the safety headline: a once-proposed payload that commits
  was voted yes by every participant, under every decision. -/
  commit_unanimous : ∀ c, ∀ p ∈ out c, 1 ≤ n →
    (tpcProposed payloads).count p = 1 → ∀ j, vote j p = true
  /-- At-most-once: with the one coordinator Rust has, a payload proposed
  at most once commits at most once, under every decision. -/
  commit_once : ∀ c (p : P), 1 ≤ n → nC = 1 →
    (tpcProposed payloads).count p ≤ 1 → (out c).count p ≤ 1
```

The callee enters the program twice, as code and as a fact:

```lean
  let cq1 := collect_quorum H coord c_votes num_participants
    num_participants dec.votes
  ghost have hcq1 := collect_quorum.ensures coord c_votes (mem part)
    (mem part) dec.votes
```

The `let` is an ordinary application at `H`. The ghost instantiates the callee's face at `Values` with the caller's own wire `c_votes` and the caller's decision `dec.votes`; after the `subst` above, `num_participants` is `mem part`. No proof ever unfolds `collect_quorum`. The caller describes its own wires at `Values`, where a fan-in of broadcasts is a sum of pools by definition:

```lean
  ghost have hvotes : ∀ c, c_votes c
      = ((List.finRange (mem part)).map (fun j =>
          (tpcProposed payloads).map (fun p => (p, voteE vote j p)))).sum :=
    fun _ => rfl
```

From that it computes the counts `CQEnsures` speaks of: a once-proposed payload's `Ok` votes are its yes-voters (`hok_votes`), and its responses number `mem part` per proposal (`hkey_votes`). `commit_unanimous` then needs only `emit_sound` of both stages, which is unconditional: some coordinator emitted a committed payload from its vote stage, that emission needed `mem part` yes-votes, and `mem part` yes-votes on a once-proposed payload come from every participant.

`commit_once` needs `emit_count`, so it must discharge the cap:

```lean
      have hvy1 : ∀ c', (cq1.1 c').count p ≤ 1 := fun c' => by
        have hcap1 : cqKeyCount (cqConsumed (c_votes c') (dec.votes c')) p ≤ mem part := by
          refine le_trans (cqKeyCount_mono (cqConsumed_le _ _) p) ?_
          rw [hkey_votes]
          exact le_trans (Nat.mul_le_mul_left _ hone) (by omega)
        rw [hcq1.emit_count c' p hn le_rfl hcap1]
        split <;> omega
```

The consumed pool sits below the whole vote pool (`cqConsumed_le`), whose count for `p` is `mem part` times the number of proposals of `p`, at most one. The second stage needs the same cap on the acknowledgement pool, which holds `mem part` copies per vote-yes emission summed over the coordinators. It is at most `mem part` only when there is one coordinator. With two, every coordinator receives every vote, both emit the same payload, the acknowledgement pool holds `2n` copies, and the second `collect_quorum` re-emits. `commit_once` therefore carries `nC = 1`, Rust's single `Process` coordinator: the callee's conditional field becomes a premise of the consumer's, with the Rust reason attached.

`two_pc` has one line that is not the Rust's: each participant tags its echo with `voteE vote j p` before sending, where the Rust tags every echo `Ok(())` at the coordinator. At `vote = fun _ _ => true` every tag `voteE vote j p` evaluates to the Rust's `Ok(())`, and the `vote` tag gives `commit_unanimous` content. The machine forms, `two_pc_safe_sched` and `two_pc_once_sched`, apply `two_pc.ensures` at the decisions the machine run realized ([Transferring to the Machine and the Network](../proving/transferring-to-the-machine.md)).
