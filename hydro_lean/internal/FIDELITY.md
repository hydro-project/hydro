# Fidelity to the Rust

Every theorem in the repository is about a Lean program read under a Lean semantics: the denotation `Values`, the step machine `SchedSem`, or the trace model of processes exchanging bytes ([Wire Codecs and Byte-Level Traces](../../docs/docs/hydro-lean/internals/execution/wire-and-traces.md)). The theorems say something about Hydro only through two correspondences. Each Lean **module** (one `hydro def`) must say what the Rust function it transcribes says, and the step machine and the trace model must behave as `hydro_lang` and DFIR do. Neither correspondence can be a theorem, because the Rust is not a Lean object. So the question for a reader judging the results is how each correspondence is kept checkable by a person and, where possible, by the build.

This page answers it in four parts: how a module transcribes its Rust function and how the census checks the transcription of every `nondet!`; where the Lean departs from the Rust and why; which runtime behaviours the step machine and the trace model cover and which they leave out; and what the formalization found in the Rust code, the findings B1 to B6.

## Line-by-Line Transcription

A module transcribes one Rust function. The Rust is quoted in a comment directly above the Lean line that mirrors it, so a reviewer compares the two in place, one statement at a time. From `hydro_std/src/quorum.rs`:

```rust
        let count_per_key = current_responses.clone().into_keyed().fold(
            q!(move || (0, 0)),
            q!(move |accum, value| {
                if value.is_ok() {
                    accum.0 += 1;
                } else {
                    accum.1 += 1;
                }
            }, commutative = manual_proof!(/** increment counters is commutative */)),
        );
```

and the corresponding lines of `hydro_lean/Hydro/Std/Quorum.lean`:

```lean
    -- let count_per_key = current_responses.clone().into_keyed().fold(
    --   q!(move || (0, 0)),
    --   q!(move |accum, value| { if value.is_ok() { accum.0 += 1; } else { accum.1 += 1; } },
    --      commutative = manual_proof!(/** increment counters is commutative */)));
    let count_per_key := H.bkfold cqCount (0, 0) (fun _ _ => True)
      (fun s x y _ => cqCount_comm s x y) (H.binto_keyed current_responses)
```

The Rust's `manual_proof!` becomes a proof term: `cqCount_comm` proves the commutativity the Rust annotation asserts, and `bkfold` does not typecheck without it. A reviewer therefore checks two things per line, that the Lean operator transcribes the Rust one and that the closure computes the same function, and the build checks the algebraic claims that the Rust's `manual_proof!` annotations only assert.

### The Census

A transcription can get the decisions wrong without any proof noticing: merging two Rust sites into one decision weakens every theorem, and inventing one models freedom the program does not have. The census checks this part mechanically. A `#nondet_census` line after each module counts its decision fields by kind (content decisions, schedule sites, cycle fuels) and fails the build unless the counts equal the ones written in the line, which are read off the Rust function ([Nondeterminism](../../docs/docs/hydro-lean/using/programs/nondeterminism.md#the-census) defines the rules):

```lean
#nondet_census paxos_core (nondets := 16) (scheds := 7) (fuels := 5)
```

The counts are not the Rust's `nondet!` tally. A schedule site (every network edge and every interval timer) counts whether or not the Rust marks it, and `.send` never carries a `nondet!`. A Rust `nondet!` whose freedom the model removes has no field: the `nondet_membership` guard at each Rust `broadcast` covers dynamic membership, which the model sets at deploy time ([below](#scope-of-the-runtime-model)). One Rust guard forwarded to two sites that choose independently is two decisions (`timeout`'s batch and clock). A callee's decisions are fields of its caller's records, so the counts are transitive.

A mismatch is a build error that lists every field with its access path, so the reviewer sees which site was dropped or added:

```text
#nondet_census M: found (nondets, scheds, fuels) = (…), expected (…); leaves:
  dec.le.p1aBatch : Hydro.HydroGen.Census.DecKind.nondet
  …
```

The mirrored modules and their censuses (nondets / scheds / fuels):

| Lean module | Rust | census |
|---|---|---|
| `Std/Quorum.lean` `collect_quorum`, `collect_quorum_with_response` | `hydro_std/src/quorum.rs` | 1/0/0 each |
| `Std/RequestResponse.lean` `join_responses` | `hydro_std/src/request_response.rs` | 1/0/0 |
| `Std/Timers.lean` `timeout`, `sample_every` | `hydro_lang` `stream/mod.rs`, `optional.rs` | 2/0/0, 1/1/0 |
| `Std/BenchClient.lean` `bench_client`, `compute_throughput_latency`, `aggregate_bench_results` | `hydro_std/src/bench_client/mod.rs` | 6/0/0, 2/1/0, 3/3/0 |
| `TwoPC.lean` `two_pc` | `hydro_test/src/cluster/two_pc.rs` | 2/4/0 |
| `Paxos/AcceptorP1.lean`, `AcceptorP2.lean`, `PP1b.lean` | `paxos.rs` `acceptor_p1`, `acceptor_p2`, `p_p1b` | 0/1/0, 2/1/0, 3/0/0 |
| `Paxos/PLeaderHeartbeat.lean`, `Recommit.lean`, `SequencePayload.lean` | `paxos.rs` | 5/3/0, 1/0/0, 6/2/0 |
| `Paxos/LeaderElection.lean`, `PaxosCore.lean` | `paxos.rs` `leader_election`, `paxos_core` | 10/5/3, 16/7/5 |
| `Paxos/PaxosWithClient.lean` `payloads_to_leader`, `paxos_with_client` | `paxos_with_client.rs` | 3/2/0, 19/9/6 |
| `Paxos/KvReplica.lean` `sequence_payloads`, `kv_replica` | `kv_replica/sequence_payloads.rs`, `kv_replica/mod.rs` | 1/0/0, 1/0/2 |
| `Paxos/PaxosBench.lean` `acceptor_checkpoint_from_replicas`, `paxos_protocol`, `paxos_bench` | `paxos_bench.rs` | 1/1/0, 23/12/9, 34/16/11 |

`PaxosBench.lean` also pins `inc_i32_workload_generator` at 0/0/0 and `tag_with_sender` at 1/0/0. `p_ballot_calc` and `index_payloads` take no decision argument and carry no census line. Their Rust `nondet!` sites are `use::atomic(…)` reads and, at the end of `index_payloads`, a `batch_atomic(&tick, …)`, each justified as "up to date with tick input": the value is read in the tick that produced it, so no choice is made. The Lean reads those wires as same-tick inputs of a `tick` block and returns the block's atomic output, with no decision; the Rust quoted above those lines elides the guard (`use::atomic(p_max_slot.latest_atomic(), …)`).

## Departures from the Rust

Each Lean line without a Rust counterpart says so in its comment.

**A guard the Rust lacks.** `PaxosVariant` has two values. `.faithful` is `paxos.rs` as written; `.guarded` adds the fixes for B1, B2, B4 and B5 below, each behind a flag (`sendOnce`, `recommitOnce`, `rebaseAboveCheckpoint`, `checkpointIsNextSlot`). Each guarded line is written next to the faithful one, and the safety theorems are stated at `.guarded`; the faithful branch claims nothing. Both variants run in `lake exe falsify`.

**A closure written as a function.** `with_client` and `bench_client` take Rust closures, and the cycle a closure participates in is closed inside the callee. The Lean writes each closure as the `hydro def` Rust would write (`payloads_to_leader`, `tag_with_sender`, `paxos_protocol`) and closes the cycle in the caller with `fix … complete`. A `TickCycleHandle` a callee returns for its caller to complete (`sequence_payloads`' `r_next_slot`) becomes a wire passed to the callee and a knot of the caller: `kv_replica` closes `r_next_slot` and `r_checkpointed_seqs` in one `fix`, hence its two fuels.

**The vote hook in `two_pc`.** The Rust tags every echo `Ok(())` at the coordinator (`c_votes.map(q!(|kv| (kv, Ok::<(), ()>(()))))`); participants never vote, so "every participant voted yes" is trivially true. The Lean tags each participant's echo with `vote j p` before the send. At `vote = fun _ _ => true` the Lean computes what the Rust computes, and unanimity becomes a theorem with content.

**A grade the Rust cannot state.** A module's grades are transcribed from the Rust type parameters (`TotalOrder`/`NoOrder`, `ExactlyOnce`/`AtLeastOnce`, `Monotonic`). One is not. `acceptor_p2`'s checkpoint input is `Singleton … (.monotonic ValueOrder.optNat)`. Rust's `Optional` has no `Monotonic` bound, but the protocol relies on the checkpoint only growing: a checkpoint that went back would let an acceptor collect a slot its published view does not cover. The Lean takes the grade the proof needs; at the machine it becomes the premise `CkMono` ([Transferring to the Machine and the Network](../../docs/docs/hydro-lean/using/proving/transferring-to-the-machine.md#unconstrained-inputs)).

## Scope of the Runtime Model

The step machine (`Hydro/Sched.lean`) and the trace model (`Hydro/Project/Model/WireTrace.lean`) describe the runtime. The runtime facts they follow: ticks are per-process logical time with no cross-process synchronization (`dfir_rs/src/scheduled/ticks.rs`); a tick runs on start, on a socket or timer wake, and drains what was buffered at its start; `TCP.fail_stop()` gives per-connection FIFO and never retransmits; cluster membership is fixed per deployment; a `forward_ref` that is acyclic through strata is a same-tick alias.

| axis | the model | relation to the runtime |
|---|---|---|
| ticks | `Pacing`: per member per step, `idle`, `stutter` (active, no tick body) or `tick` | each DFIR tick is one `tick` step of one member |
| delivery | an identity operator at `Values`; at the step machine, `StepHist.deliver`: a cursor per (receiver, sender, edge), read through `cumMax`, one step behind the sender | per-connection FIFO prefix delivery; unbounded delay, silence, a stall at any point, and per-recipient divergence of a broadcast are all cursors |
| fan-in | `union`/`values` merge same-step increments in member order | the machine realizes one order; `assume_ordering` at `Values` ranges over every order, so a fact proved there holds for the ones the machine skips |
| consumption | a tick consumes everything that has arrived | partial batches arise from arrival timing only |
| timing | per-tick scripts of clock readings and fire counts, read once per activation | freer than a wall clock, so safety over all scripts covers every wall clock |
| the one-step floors | delivery and `defer_tick` read one step back; a knot wire is its body's output at the same step | sends land next tick at soonest; `forward_ref` is a same-tick alias |
| in-tick content | a plain `Array` at the machine, the grade's pool type at `Values` | the `NoOrder` API exposes only order-blind operators |
| `reduce_watermark` | DFIR's eager `ReduceKeyedWatermark` fold at the machine, accumulate-then-read at `Values` | equated by `scan_eager_eq_assoc` |
| the network | bytes per connection, read in prefixes cut anywhere (`WireTrace`) | every byte-level trace induces an interleaving of the lock-step machine with the same outputs (`WireTrace.streamOutputsAt_eq`) |

A schedule here is a function of the step only, and the machine is deterministic given it. An adaptive adversary's realized run is therefore some schedule, so safety over all schedules covers adaptive adversaries.

Outside the model:

- **Lossy transports.** `TCP.lossy` and UDP skip messages; prefix cursors cannot. Every mirrored program uses fail-stop.
- **Dynamic membership.** Every `broadcast` is `broadcast_closed`; the `nondet!` at each Rust `broadcast` site (`nondet_membership`) marks this assumption.
- **Adequacy.** There is no theorem that every decision record is realized by some schedule. The denotation keeps an unordered stream's order unresolved, and a later `assume_ordering` may select any permutation of it, while the machine has one order (for example the list `flattenUnordered` publishes). The denotation's decision space is therefore larger than what schedules realize ([Grades](../../docs/docs/hydro-lean/internals/semantics/grades.md)). Safety transfer needs only machine ⊆ denotation; the surplus makes a contract a stronger statement, harder to prove, never unsound.
- **Compiled Rust.** The machine is a semantics of the Hydro combinators, not of the code `hydro_lang` generates. Only the line-for-line transcription, checked by review, ties the Lean to that code.
- **The benchmark's outputs.** `paxos_bench` produces throughput and latency; no safety statement is made about `lake exe bench`.

## Findings in the Rust

| | finding | witness | status |
|---|---|---|---|
| B1 | a ballot's P1a is re-sent on every election trigger; duplicate replies form a fake quorum | `lake exe falsify` (`b1Dec`): the faithful program commits two values at slot 0 | Rust as written; guarded in the Lean (`sendOnce`); [#3372](https://github.com/hydro-project/hydro/issues/3372) |
| B2 | recommit and slot rebase run on every leader tick | `#guard_eval` on `b2Run` in `Paxos/Falsification.lean`; `lake exe falsify` through `paxos_core` | Rust as written; guarded (`recommitOnce`); [#3373](https://github.com/hydro-project/hydro/issues/3373) |
| B3 | `collect_quorum_with_response` at `min < max`: late responses are batching-dependent | the usage cap `cqKeyCount … ≤ max` in `CQWREnsures` | documented by the contract |
| B4 | a fresh payload can be indexed at a garbage-collected slot | `#guard_eval` on `b4Run` | Rust as written; guarded (`rebaseAboveCheckpoint`); [#3374](https://github.com/hydro-project/hydro/issues/3374) |
| B5 | the recommit treats the checkpoint slot as executed; an in-flight slot is lost | `#guard_eval` on `b5Run` | Rust as written; guarded (`checkpointIsNextSlot`); [#3375](https://github.com/hydro-project/hydro/issues/3375) |
| B6 | the `count > f` skip in the recommit leaves a slot unfilled | the contract field `RCTick.skipped_holders` | liveness only; documented in the Rust; no guard in the repository; [#3363](https://github.com/hydro-project/hydro/issues/3363) |

### B1: duplicate P1a broadcasts

`leader_election` sends `p_ballot.clone().filter_if(p_trigger_election).all_ticks()` to the acceptors. When the election timer fires twice without a ballot change, the same P1a goes out twice. An acceptor that consumes both copies in one tick replies `Ok` to each (its max ballot is unchanged by the duplicate), and `collect_quorum_with_response` counts the two replies from one acceptor as two votes. With `f = 1`, three acceptors and a genuine quorum for the other proposer, the faithful program commits two values at slot 0. The guard is a `tick` register that remembers the last ballot sent:

```lean
      -- fire ⇔ trigger ∧ (faithful ∨ ballot ≠ lastSent)
      let fire := H.bsMap (H.bsZip lastSent bt)
        (fun (ls, (b, trig)) => trig && !(variant.sendOnce && decide (ls = some b)))
```

Its loop invariant `OnceInv` says two fires carry distinct ballots. B1 also violates an assumption that `collect_quorum`'s own `nondet!` comment makes, that no key receives more than `max` responses; the Lean contract states that assumption as a premise, the **usage cap** `cqKeyCount … ≤ max` on the field `emit_count`.

### B2: every-tick recommit and rebase

`sequence_payload` chains `p_log_to_recommit` into `payloads_to_send` on every leader tick, and `index_payloads` rebases fresh payloads to `max_slot + 1` whenever the recovered view is nonempty. While a reign re-presents its view, two fresh payloads on two ticks take the same slot at the same ballot, and one proposer commits two values at one slot. The guard fires the recommit and the rebase on the first leader tick of a ballot only:

```lean
    let fire := H.bsMap
      (H.bsZip nonempty (H.bsZip recommittedAt (H.bsZip bl ledLast)))
      (fun (ne, (ra, ((b, l), d))) =>
        ne && (!variant.recommitOnce || (l && !d && !decide (ra = some b))))
```

### B3: `collect_quorum_with_response` at `min < max`

Whether a late response for an already-emitted key is included depends on batch boundaries. Only membership and the threshold are batching-invariant. Paxos uses membership, so this is not a Paxos bug. The contract makes the dependence visible: its multiplicity and completeness fields (`emit_le`, `emit_complete`) hold only under `cqKeyCount … ≤ max`.

### B4: a fresh payload at a collected slot

With garbage collection, `p_max_slot` is the recovered view's maximum slot (`keys().max()`), which can lie below the view's checkpoint because the acceptors have collected those slots. A proposer that has never led falls back to its register (`unwrap_or(next_slot)`) and indexes a fresh payload at an executed, collected slot; the acceptor accepts it at the new ballot. The guard lifts the base to the checkpoint (`rcMaxSlot`):

```lean
def rcMaxSlot (variant : PaxosVariant) (ms C : Option Nat) : Option Nat :=
  if variant.rebaseAboveCheckpoint then wmJoin ms (lastSettled C) else ms
```

### B5: the checkpoint is the first unexecuted slot

The replica reports `next_slot` as its checkpoint `C`, and the acceptor collects log keys `< C`, so slot `C` is live. The recommit treats `C` as executed:

```rust
            } else if let Some(checkpoint) = checkpoint
                && slot <= checkpoint
            {
                return None;
            }
```

```rust
            if let Some(checkpoint) = checkpoint {
                (checkpoint + 1)..max_slot
```

After a re-election, an in-flight slot `C` is neither recommitted nor hole-filled, and the replicas, which apply slots contiguously, wait for it forever. The guard drops only `slot < C` and fills holes from `C`:

```lean
def rcSettled (variant : PaxosVariant) (slot : Nat) (C : Option Nat) : Bool :=
  match C with
  | some c => if variant.checkpointIsNextSlot then decide (slot < c) else decide (slot ≤ c)
  | none => false
```

This one is a liveness failure, so its witness checks a loss, not a disagreement: under `.faithful`, `b5Run` commits the fresh payload at slot 2 and nothing at slot 1.

### B6: the `count > f` skip

The recommit's fold computes, per slot, a champion (the max-ballot entry) and a count of holders of the champion's value, and skips the slot when the count exceeds `f`, on the assumption that the value is chosen. The count is over holders at any ballot: holders at ballots 1 and 3 and a third acceptor at ballot 2 with another value is not a chosen value. Even a chosen value may not have reached the replicas if the previous leader crashed. Skipping proposes nothing, so safety holds (the headline uses only `RCTick.value_best`); the slot stays a hole for the leader's tenure. The Rust carries a doc comment on that `nondet!` describing the hazard. The Lean contract's field `RCTick.skipped_holders` states what a skip means. The fix (drop the skip, recommit the champion) is not in the repository.

The same fold is order-dependent: two orders of one batch can disagree on skip versus recommit. The Rust realizes an order with `.into_keyed().assume_ordering::<TotalOrder>(nondet!(…))` before the fold, and the Lean mirrors it with `tassume_ordering`, so every fact about the recommit is proved for every order.

### Smaller Items

Filed as [#3376](https://github.com/hydro-project/hydro/issues/3376).


- `paxos.rs`'s `nondet!` comment on the acceptor log says "we use `persist()`"; the code is `across_ticks` plus `reduce_watermark`.
- `collect_quorum` re-emits a key that receives more than `max` responses, which only upstream duplicates (B1) cause.
- `p_has_largest_ballot` is identically `true` (`PBCEnsures.hasLargest_true`).
- `acceptor_p2`'s `reduce_watermark` merge is commutative only if two entries at one slot with equal ballots carry equal values. The Lean states this as `acceptor_p2`'s `requires P2aFunctional` and discharges it in `sequence_payload`.
- `paxos_bench.rs`'s acceptor checkpoint gate is `key_count() == f + 1`; with more replicas it closes forever. `paxos_protocol` requires `mem replicas = f + 1`.
- `kv_replica`'s `impl Ord for SequencedKv` compares `seq` only and is not antisymmetric against the derived `Eq`. The Lean order is lexicographic.

`lake exe explore` reports no bug. The denotation's decision space has acausal freedom (batch legality is checked against completed pools); the exploration shows that this freedom cannot fabricate a leader under `.guarded`, because solicitation is gated on a knot wire. Leader stability is therefore a derived fact, not a hypothesis.
