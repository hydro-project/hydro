---
sidebar_position: 1
title: "Proofs over Values"
---

# Proofs over Values

In a state-machine proof of a distributed protocol, as in TLA+, most of the work is bookkeeping about the network: which messages are in flight, which interleavings of send and deliver are possible, and an inductive invariant that survives all of them. In Hydro Lean you prove a module's face about its **denotation**, the program read at the instance `Values L mem`, where that bookkeeping does not exist. Every collection is an ordinary value: per cluster member, a pool of everything that ever arrives on a stream, or a list with one entry per tick for a tick-located collection. The network's delay and reordering are encoded in the types of these values (an unordered pool stands for every arrival order, and holds everything ever sent), and every `nondet!` that observes them is an explicit argument, a **decision** ([The Network at `Values`](../programs/distributed-programs.mdx#the-network-at-values), [Nondeterminism and Decisions](../programs/nondeterminism.md)). A proof is equational reasoning about those values, written inside the module, between the program's lines.

This page builds one proof from start to finish: two-phase commit's unanimity, from `hydro_lean/Examples/TwoPC.lean`, the Lean form of `hydro_test/src/cluster/two_pc.rs`. Along the way it uses every mechanism a proof over `Values` needs: facts stated at program points, reading a collection operator by operator, a callee's face used in place of its body, and a precondition passed from one module to another, for which the example is Paxos. The [Quickstart](../quickstart/) proves a smaller quorum end to end. Carrying the finished face to the step machine and the processes is a separate, mechanical proof ([Transferring to the Machine and the Network](./transferring-to-the-machine.md)).

## The Goal

A module's face is a structure of `Prop` fields over its inputs, its decisions and its outputs, stated in the `ensures` clause ([Contracts](../programs/contracts.md)). The field of `two_pc`'s face that matters is unanimity: a payload that was proposed once and commits was voted yes by every participant.

```lean
  /-- Unanimity, the safety headline: a once-proposed payload that commits
  was voted yes by every participant, under every decision. -/
  -- for every coordinator `c` and payload `p` it commits, if there is at least
  -- one participant and `p` was proposed once, every participant `j` voted yes
  commit_unanimous : ∀ c, ∀ p ∈ out c, 1 ≤ n →
    (tpcProposed payloads).count p = 1 → ∀ j, vote j p = true
```

`out c` is the pool of payloads coordinator `c` commits, `tpcProposed payloads` is the pool of all proposals, `vote j p` is participant `j`'s vote, and `n` is the number of participants. The field mentions no schedule and no message. `two_pc.ensures` takes the decision record as an argument, so the field holds at every record: for every way the program's batches could have been cut.

The whole face is guarded by a premise about the module's arguments, `num_participants = mem part`. The body opens such a premise before its first line with `ghost intro hn` and `ghost subst hn`, so the rest of the proof works with `mem part` directly ([Writing a Face](../programs/contracts.md#writing-a-face)).

Inside the body, every `let` of the program is a local at its `Values` type, and the proof talks about those locals. A **wire** is a collection-valued `let` or binder of the program; a wire `w` at a location with `mem ℓ` members holds one value per member, `w i`. A stream at `.noOrder .exactlyOnce` is a `Multiset`, one at `.totalOrder` a `List`, and a tick-located wire is a list with one entry per tick, read as `(w i)[n]? = some e` ([The Signature](../programs/signature.md)).

## Ghost Facts

The proof of `commit_unanimous` needs facts about collections in the middle of the program: what the coordinator's vote pool holds, and what the first quorum stage emitted. You state each one between the lines of the program, right after the collection it is about, with `ghost have`. The first is the content of the coordinator's vote pool:

```lean
  -- every participant sends its tagged votes to the coordinator
  let c_votes := H.values (H.broadcast_closed sched.voteCh p_votes)
  …
  -- the fact: coordinator `c`'s vote pool is, summed over every participant `j`,
  -- each proposal `p` paired with `j`'s vote on it (`Ok` for yes, `Err` for no)
  ghost have hvotes : ∀ c, c_votes c
      = ((List.finRange (mem part)).map (fun j =>
          (tpcProposed payloads).map (fun p => (p, voteE vote j p)))).sum :=
    -- the proof: true by definition, because the transport changes no content at `Values`
    fun _ => rfl
```

The proof is `rfl` because the network's reordering and delay are in the type of `c_votes`, not in its content: at `Values`, a broadcast followed by `.values()` hands each receiver the sum of the senders' pools, a multiset that stands for every arrival order ([The Network at `Values`](../programs/distributed-programs.mdx#the-network-at-values)). Explicit delivery exists only in the operational semantics `SchedSem`, and the transfer lemmas account for it once for every program.

A ghost is proof-only: the compiled program never sees it, and it adds nothing to the module's interface. Later ghosts and the `prove` clause cite it by name, so a long proof is a chain of ghosts, each about the collections just defined and each proved from the ones before. Two more forms help build the chain: `ghost obtain ⟨…⟩ := h` destructures an existential fact and binds its parts, and `ghost let` binds a term that later facts mention.

## Reading Wires

`rfl` works while the fact is one or two operators away from a definition. Once the wire sits behind several operators or a module boundary, the kernel has to unfold all of them at once, and `rfl` gets slow or times out. The **`den`** simp set reads the wire one operator at a time instead, rewriting each to its `Values` meaning: `map` to `Multiset.map`, a broadcast to the sum over senders, `allTicks` to the sum of the per-tick emissions, `batch` to the cut of the pool at its decision. `simp only [w, den]` unfolds the one wire `w` and reads it operator by operator. In Paxos, the pool of P2a messages each acceptor receives is everything every proposer sent, tagged with its sender (`Examples/Paxos/SequencePayload.lean`):

```lean
  -- the fact: acceptor `j`'s P2a pool is, summed over every proposer `r`,
  -- everything `r` sent in any tick, turned into a P2a message with sender `r`
  ghost have hp2apool : ∀ (j : Fin (mem acc)),
      p2as j = ((List.finRange (mem prop)).map
        (fun r => ((payloads_to_send r).sum).map
          (fun kv => (⟨r, kv.1.2, kv.1.1, kv.2⟩ : P2a P (mem prop))))).sum :=
    -- the proof: unfold the wire `p2as` and rewrite each operator to its meaning
    fun j => by simp only [p2as, den]
```

Listing a wire in `simp only` unfolds that one `let`, so you open as much of the program as the fact is about. A decision stays visible as an argument: a batch reads as `batchCuts pool 0 (dec.batch i)`, the pool cut at the batch boundaries `dec.batch i`.

A `tick` block is read through the facts it generates, not through `den` alone ([Tick Blocks and Their Invariants](./tick-invariants.md)), and a cycle through its Kleene stages ([Knots and Loop Invariants](./knot-invariants.md)).

## Callee Contracts

Two-phase commit runs `collect_quorum` twice, once on the votes and once on the acknowledgements. Its proof never looks inside `collect_quorum`, whose own proof is a loop invariant over ticks ([Tick Blocks and Their Invariants](./tick-invariants.md)). Instead, right after each call, it states the callee's face at the call's own input and decision, and from then on reasons only with that face:

```lean
  -- the first quorum stage, over the coordinator's vote pool: emit a payload
  -- once all `num_participants` participants have voted yes
  let cq1 := collect_quorum H coord c_votes num_participants
    num_participants dec.votes
  -- its contract, at this call's input `c_votes` and batch decision `dec.votes`
  ghost have hcq1 := collect_quorum.ensures coord c_votes (mem part)
    (mem part) dec.votes
```

`collect_quorum.ensures` is the callee's face as a theorem, generated by its `hydro def`. The field this proof needs, `emit_sound`, says that an emitted key has at least `min` `Ok` responses among the responses the stage consumed:

```lean
  -- for every member `i` and every key `k` it emits, `k` has at least `min`
  -- `Ok` responses among the responses consumed under batch decision `dec i`
  emit_sound : ∀ (i : Fin (mem ℓ)), ∀ k ∈ out.1 i,
    min ≤ cqOkCount (cqConsumed (resp i) (dec i)) k
```

The consumed responses depend on how the stage's input was batched, that is, on the decision `dec.votes`. Two-phase commit needs a bound in terms of the whole vote pool, which `hvotes` describes, and the consumed responses are a sub-multiset of the pool (`cqConsumed_le`). Weakening to the pool makes the decision drop out of the fact:

```lean
  -- the fact: if coordinator `c'` emits `p` from the vote stage, its whole
  -- vote pool holds at least `mem part` yes votes for `p`
  ghost have hvy_votes : ∀ c', ∀ p ∈ cq1.1 c',
      mem part ≤ cqOkCount (c_votes c') p := fun c' p hp =>
    -- `emit_sound` bounds the consumed votes; the consumed votes are part of
    -- the whole pool (`cqConsumed_le`), and counting is monotone
    le_trans (hcq1.emit_sound c' p hp) (cqOkCount_mono (cqConsumed_le _ _) p)
```

A coordinator that emits `p` from the vote stage has therefore seen at least `mem part` yes votes for it. One more ghost, `hok_votes`, rewrites with `hvotes` and counts over the sum: for a payload proposed once, the `Ok` votes for it in the pool are exactly the participants that voted yes.

A face holds for every input, so you can state it at any wire, including one inside a cycle ([Knots and Loop Invariants](./knot-invariants.md#facts-inside-a-cycle)). A field with premises is usable once you prove its premises at your wires. `collect_quorum`'s exactly-once field assumes that no key gets more responses than there are participants. For the acknowledgement stage that holds only when there is one coordinator, so `two_pc`'s at-most-once field `commit_once` carries the premise `nC = 1` ([Consuming a Callee's Face](../programs/contracts.md#consuming-a-callees-face)).

## The `prove` Clause

With the chain of ghosts in place, the face itself is short work. The body ends with its result, then `prove` and one proof per face field, each with every ghost in scope:

```lean
  cq2.1
  prove
    …
    -- given: coordinator `c` committed `p` (`hp`), `1 ≤ mem part` (`hn`),
    -- `p` was proposed once (`hone`); to show: every participant voted yes
    commit_unanimous := fun c p hp hn hone => by
      -- some coordinator `c'` emitted `p` from the vote stage
      obtain ⟨c', hpc'⟩ := hcommit_src c p hp hn
      -- so `c'` saw at least `mem part` yes votes for `p`
      have h1 := hvy_votes c' p hpc'
      -- and those are the participants that voted yes:
      -- h1 : mem part ≤ (participants j with vote j p = true).length
      rw [hok_votes c' p hone] at h1
      …
      -- a sublist of the participants that is as long as all of them is all
      -- of them, so participant `j` voted yes
      intro j
      simpa using List.length_filter_eq_length_iff.mp hlen j (List.mem_finRange j),
```

The proof follows the protocol argument, one ghost per step. `hcommit_src`, derived from the acknowledgement stage's `emit_sound` in the same way as `hvy_votes`, says a committed `p` was emitted from the vote stage by some coordinator `c'`. `hvy_votes` gives `c'` at least `mem part` `Ok` votes for `p`, and `hok_votes` turns that into: at least `mem part` of the `mem part` participants voted yes. The elided lines conclude that the yes-voters are all of them.

Not every field needs a chain. A field that restates the program holds by `rfl`: `collect_quorum`'s error output is its input filtered to the errors, and its face says exactly that:

```lean
    -- the error output is the input's `Err` responses, by definition
    fails_eq := fun i => rfl
```

## Preconditions with `requires` and `ghost pre` {#requires-and-ghost-pre}

Two-phase commit's proof needs nothing from its callers. Paxos's does: some facts a module relies on are about the content it receives, and only the module that sends that content can prove them. A Paxos acceptor merges its log by keeping the higher ballot per slot. That merge is commutative only if two entries with the same slot and ballot carry the same value, which holds because a proposer sends one value per slot and ballot. The Rust records the gap:

```rust
            }, commutative = manual_proof!(/** max by ballot (TODO: not if two entries with same ballot, need assume) */)),
```

In Lean the merge is a **conditional fold**: it takes a pairwise predicate, here `LogTie`, and its commutativity proof covers only pairs that satisfy it. The acceptor does two things (`Examples/Paxos/AcceptorP2.lean`). It states what it assumes about its input with `requires`; inside the module that assumption is in scope as `hreq`. And it proves the fold's precondition from `hreq` with a `ghost pre` placed before the fold:

```lean
  -- commutative = manual_proof!(requires: same slot ∧ same ballot ⇒ same value)
  -- the assumption about the input: the received P2a messages agree on the
  -- value at every (slot, ballot)
  requires P2aFunctional p_to_acceptors_p2a
  …
  -- the merge's precondition: every two log entries that meet at a slot
  -- satisfy `LogTie`, proved from the assumption `hreq`
  ghost pre hpre : ∀ j, (KeyedPool.accum (a_log_keyed j)).Pairwise LogTie := fun j => by
    …
    exact hreq j pa hpa pb hpb hk hbal
  -- the merge itself, which needs `LogTie` to be commutative
  let a_log := H.reduce_watermark a_log_keyed a_checkpoint_tick logMerge LogTie logMerge_comm
```

The acceptor's `requires` is now its caller's obligation, which the caller proves with a `ghost pre` before the call. The caller is the proposer side, `sequence_payload`, which shows that the P2a messages it sends agree at every slot and ballot:

```lean
  -- the acceptor's assumption, proved for the messages this proposer sends:
  -- two P2as with the same slot and ballot carry the same value
  ghost pre hp2a_fun : P2aFunctional p2as :=
    fun j a ha b hb hs hb' => hp2a_agree hreq.1 hreq.2 j j a b ha hb hs hb'
  -- acceptor_p2(a_max_ballot, p2as, a_checkpoint)
  let ap2 := acceptor_p2 H acc prop a_max_ballot p2as a_checkpoint
    dec.ap2 sched.ap2
```

`sequence_payload` can prove this only under its own `requires`, the leader discipline `LeaderDiscipline`, so the obligation moves up one more level: `paxos_core` proves the leader discipline from `leader_election`'s face. What remains at the top is `paxos_core`'s own `requires variant = .guarded`, which the machine theorem discharges by `rfl` ([The Paxos Machine Theorem](./transferring-to-the-machine.md#the-paxos-machine-theorem)).

`M.ensures` does not assume the `requires`. At `Values` a conditional fold is defined on every input: on content with a pair that violates its predicate it returns its seed (`Multiset.foldlOn`, [Grades](../../internals/semantics/grades.md)), so the program means something whether or not the clause holds. A `ghost have` that needs the clause therefore takes it as a hypothesis, `∀ (hreq : LeaderDiscipline …), …`, and so do the face fields that depend on it.

The `ghost pre` facts are for the machine. You never cite one yourself: when `hydro def` proves that the machine run stays within the denotational run (the residual lemmas `M_co_wf`, [Transferring to the Machine and the Network](./transferring-to-the-machine.md#the-lemmas-you-cite)), the generated proof needs each conditional fold's predicate to hold on the content the machine actually folds, and it looks for a `ghost pre` that closes that goal. If none matches, the build stops with

```text
hydro_pre_close: no `ghost pre` fact closes the goal:
```

followed by the required statement. Add a `ghost pre` with that statement, over your wires, before the fold or call.
