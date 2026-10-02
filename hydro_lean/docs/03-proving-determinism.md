# Proving Determinism Through Types: Sealing

> **Status: historical.** This document describes the pre-decisions-as-inputs
> layers (multiset carriers, sealed components, `Holds` simulation runs),
> which were deleted as unused after the (a)/(b) deliverables were migrated
> to the decisions-as-inputs surface (docs/10; typed M-form stages with
> colocated verified faces). Names below refer to deleted code — recover
> from jj history. Kept as the record of the design and of why it was
> superseded.

This is the layer that answers the question *"why isn't determinism a theorem
someone has to remember to state?"* — in HydroLean, a component's determinism is the
**existence of its denotation at a quotient type**, forced by its signature.

## The sealing obligation (`Hydro/Sealed.lean`)

A Rust component whose signature has a `NoOrder` output and **no `NonDet`
parameter** — e.g. `collect_quorum : Stream<(K, Result<(), E>), _, _, Order> → Stream<K, _, _, NoOrder>`
— *claims by its type* that its output is a well-defined multiset, independent of
the internally guarded batching and of input arrival order. The Lean form of that
claim is a `SlicedComponent`:

```lean
structure SlicedComponent (α β) where
  St      : Type
  tick    : TickLoop (List α) St (List β)
  Dom     : List α → Prop          -- usage contract (e.g. AtMostMaxResponses)
  domDec  : DecidablePred Dom
  domPerm : -- Dom is arrival-order-invariant
  inv     : -- THE sealing witness:
    ∀ {l₁ l₂}, l₁.Perm l₂ → Dom l₁ → ∀ {b₁ b₂}, b₁.of l₁ → b₂.of l₂ →
      (tick.allTicksOutput b₁).Perm (tick.allTicksOutput b₂)
```

The single `inv` field is forced jointly by the two type-level facts: instantiate it
with `l₁ = l₂` and two batchings to get *batching invariance* (no `NonDet` in the
signature), and with genuinely permuted lists to get *representative invariance*
(the `NoOrder` input marker). Given the witness:

```lean
def denote (c : SlicedComponent α β) : Multiset α → Multiset β   -- via Quotient.lift
```

**The mere existence of `denote` is the determinism theorem.** It is produced by
`Quotient.lift` from `inv` exactly as `Multiset.foldComm` is produced from a
commutativity witness — the marker discipline *is* quotient soundness.

`adequacy` then removes the schedule quantifier from every downstream statement:

```lean
theorem adequacy : Dom l → b.of l →
    Multiset.ofList (c.tick.allTicksOutput b) = c.denote (Multiset.ofList l)
```

*every* adversarial execution denotes `denote` — the batching ∀ lives here, once,
for all components (it is the unbounded counterpart of what the Rust simulator
checks per-instance when fuzzing batch hooks).

Variants: `OrderedComponent` seals `TotalOrder`-output components at `List → List`
under an *exact-equality* obligation; `SlicedComponent.total` seals contract-free
loops; `GuardedComponent H α β := H → SlicedComponent α β` keeps visible nondet in
the signature (mirroring Rust `NonDet` parameters). Composition (`comp`,
`denote_comp`, `mapOut`) transfers sealedness with no new determinism proofs.

## Instance-picking: sealedness makes model proofs one-run computations

Because all instances coincide once `inv` is discharged, you never redo a batching
induction:

- `denote_eq_whole` / `denote_eq_singletons` — compute `denote` via the
  one-mega-batch run (a single `step` application) or the element-at-a-time run.
- `instance_swap` — inside any proof, replace a sealed component's run on one
  instance by its run on any other instance of the same multiset.
- `denote_eq_spec_of_whole` / `denote_eq_spec_of_singletons` — verifying a candidate
  spec for `denote` collapses to checking it against ONE convenient instance.

The intended workflow: discharge `inv` once (usually via count-based invariants that
are manifestly order-insensitive), then verify the shallow model on the easiest
instance.

## Worked positive example: `collect_quorum` sealed

`Programs/CollectQuorumSealed.lean`:

- `QuorumDom max` — the decidable, permutation-invariant contract
  (`quorumDom_iff_atMostMax`, `quorumDom_perm`);
- `collectQuorumC min max hmin hmm : SlicedComponent (κ × Except E Unit) κ` — the
  sealing witness extracted from the correctness proof's count-based invariants;
- `qualifiedKeys min m` and **`collectQuorumC_denote`** / `collectQuorumC_mem` — the
  user-facing spec of a `Multiset → Multiset` function with *no batching or ordering
  quantifier in sight*. (This formally discharges quorum.rs's
  `assert_has_consistency_of(manual_proof!(/** TODO */))` for the membership form.)

## What failing to seal *means*

Sealing failures are findings, not inconveniences — the marker is load-bearing:

1. **Emission order is batching-dependent** (found during the quorum port): an
   order-sensitive output spec is falsified by an explicit batching witness, so
   `collect_quorum` cannot seal at `List` output — which is precisely why its Rust
   output marker is `NoOrder`. The type was already telling the truth.
2. **`collect_quorum_with_response`, min < max** (finding B3): late responses for
   already-emitted keys are included or not depending on batch boundaries — only the
   membership/threshold spec is batching-invariant; the port documents that it
   cannot seal at multiset level off-contract (`Programs/CollectQuorumWithResponse.lean`,
   `Programs/PaxosQuorumModel.lean`).
3. **Open-membership TwoPC — the worked negative example**
   (`Programs/TwoPCOpenMembership.lean`, with `Hydro/OpenBroadcast.lean`): under
   dynamic `Stream::broadcast` (membership as an input stream; each payload's
   recipient set is a snapshot-cut choice), the theorem
   `open_twoPC_not_deterministic` exhibits identical inputs and batching shapes
   whose committed outputs *differ* under two cut choices — the exact failure of the
   sealing/erasure obligation, i.e. a machine-checked proof that **no deterministic
   denotation exists** and the component can only be a `GuardedComponent` with the
   cuts visible. Companion results: `open_twoPC_overshoot_batching_dependent`
   (membership growth violates `AtMostMaxResponses` ⇒ batching-dependent duplicate
   commits) and the correct weaker guarantee `open_twoPC_relativized_unanimity`
   (epoch-pinned recipients). Contrast: the closed variant (Rust
   `broadcast_closed`, our `Hydro.broadcast_closed`) supports the adversary-free
   master theorem `twoPC_committed_eq` (`Programs/TwoPCProof.lean`). The Rust
   two_pc.rs currently calls dynamic `broadcast` with `nondet!(/** TODO */)`; the
   negative theorem shows that TODO is not dischargeable as locally-resolved
   ([FINDINGS.md](../FINDINGS.md) §C).

## Sealed *trajectories*: determinism of evolving values

For a singleton output that downstream code snapshots, endpoint determinism is not
enough — intermediates are observable. The sealed analogue (`Hydro/MTrajCore.lean`):
`Traj.sealFold f init (hcomm : Multiset.AccComm f) : Multiset α → β`, with the
fundamental lemma `observeAt_seal` (and `observeAtomic_seal`): **the value at any
cut of any arrival order is `sealFold` of the consumed sub-multiset.** So a sealed
singleton's entire trajectory *set* across all schedules is the image of one
function on the sub-multiset lattice — the adversary only picks a monotone chain
through it (`sealFold_chain`); no trajectory enumeration ever occurs. What sealing
does *not* give — cross-observer coherence — is doc 04's `SharedObs` story.
