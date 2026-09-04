import HydroLean.Programs.CollectQuorumStreams
import HydroLean.Programs.CollectQuorumMinMax

/-!
# Port of `two_pc` (Rust: `hydro_test/src/cluster/two_pc.rs`)

Two-phase commit in the decisions-as-inputs surface (docs/10): a thin wire
composition over the **shared** `hydro_std::quorum` stage
(`Programs/CollectQuorumStreams.lean` — the same `collect_quorumM` the Paxos
port consumes):

```rust
pub fn two_pc(coordinator, participants, num_participants, payloads) -> Stream<Payload, …, NoOrder> {
    let p_prepare = payloads.broadcast(participants, …, nondet!(/** TODO */));
    let c_votes   = p_prepare.send(coordinator, …).values();
    let (c_all_vote_yes, _) = collect_quorum(c_votes.map(|kv| (kv, Ok(()))), n, n);
    let p_commit  = c_all_vote_yes.broadcast(participants, …, nondet!(/** TODO */));
    let c_commits = p_commit.send(coordinator, …).values();
    let (c_all_commit, _) = collect_quorum(c_commits.map(|kv| (kv, Ok(()))), n, n);
    c_all_commit
}
```

## Where the nondeterminism lives

The cluster→coordinator fan-ins are `NoOrder` streams; per the decisions-as-
inputs semantics their arrival order is *quotiented into the decision*: each
`collect_quorum` consumes a `batchC` decision over the member-family union
(`unionF`), so one execution is pinned by exactly **two decisions**
(`TwoPCDecisions`) — the contents of the two batching `nondet!` guards, with
the arrival shuffle absorbed as batch data. The broadcasts need no decision:
this file models the *closed*-membership broadcast (every member sees the
payload stream — what today's deploy targets provide); the dynamic
`broadcast` with its `nondet_membership` guard is
`Programs/TwoPCOpenMembership.lean`, where sealing fails.

The Rust `nondet!(/** TODO */)` justifications are supplied as *theorems*
(`TwoPCProof.lean`): the committed output is invariant (as a multiset —
`List.Perm`) under **all** complete decisions.

## Generalization: a vote function

The Rust code maps every echoed payload to `Ok(())` (all participants vote
yes; the "participant 1 aborts transaction 1" comment is an unimplemented
TODO). We generalize with a `vote : Fin n → P → Bool` hook: participant `i`
responds `Ok ()` to payload `p` iff `vote i p`, recovering the Rust code at
`vote = fun _ _ => true`. This makes unanimity safety a non-vacuous theorem.

Note (`n = 0`): Rust `two_pc` with zero participants commits nothing (no
echoes ever arrive), while "all participants voted yes" is vacuously true;
our correctness theorems therefore assume `1 ≤ n`.
-/

namespace HydroLean.Programs

open HydroLean.Hydro

variable {P : Type} [DecidableEq P]

/-- Participant `i`'s vote on payload `p`, as the `Result<(), ()>` it sends
back to the coordinator (`Ok(())` = commit vote). -/
def voteE {n : Nat} (vote : Fin n → P → Bool) (i : Fin n) (p : P) :
    Except Unit Unit :=
  if vote i p then .ok () else .error ()

/-- The decisions of one 2PC execution — the exact contents of the two
`collect_quorum` batching guards (`NoOrder` fan-in: the consumed batch IS the
decision, shuffle included). Nothing else is adversarial: everything between
the two sites is a pure function. -/
structure TwoPCDecisions (n : Nat) (P : Type) where
  /-- Consumed batches of the phase-1 `collect_quorum` (votes). -/
  votes : List (List (P × Except Unit Unit))
  /-- Consumed batches of the phase-2 `collect_quorum` (acks). -/
  acks : List (List (P × Except Unit Unit))

/-! ## The stages (one def per Rust `let`, over the payload wire) -/

/-- `c_votes` (two_pc.rs: `p_prepare` = closed broadcast — every member sees
the payload stream — then the participants' echo `.map(|kv| (kv, Ok(())))`
with the vote hook, sent to the coordinator; `.values()` drops sender tags,
which the family view keeps until the quorum's fan-in decision). -/
def tpc_c_votesM (n : Nat) (vote : Fin n → P → Bool) :
    Stream P →ₘ (Fin n → Stream (P × Except Unit Unit)) :=
  let payloads := MonoMap.id (α := Stream P)
  MonoMap.pi (fun i => payloads.map (fun p => (p, voteE vote i p)))

/-- `c_all_vote_yes` (two_pc.rs: `collect_quorum(c_votes…, n, n).0`): the
first quorum, at its `batchC` decision — the **shared** `hydro_std` stage
(`Programs/CollectQuorumStreams.lean`, the same one Paxos consumes). -/
def tpc_vote_yesM (n : Nat) (vote : Fin n → P → Bool)
    (dvotes : List (List (P × Except Unit Unit))) : Stream P →ₘ Stream P :=
  let c_votes := tpc_c_votesM n vote
  -- let (c_all_vote_yes, _) = collect_quorum(c_votes…, n, n)
  (collect_quorumM n n dvotes ∘ₘ c_votes).fstOf

/-- `c_commits` (two_pc.rs: `p_commit` = closed broadcast of the committed
stream, then the always-`Ok` ack echo). -/
def tpc_c_commitsM (n : Nat) (vote : Fin n → P → Bool)
    (dvotes : List (List (P × Except Unit Unit))) :
    Stream P →ₘ (Fin n → Stream (P × Except Unit Unit)) :=
  let c_all_vote_yes := tpc_vote_yesM n vote dvotes
  MonoMap.pi (fun _i => c_all_vote_yes.map (fun p => (p, Except.ok ())))

/-- `two_pc` (the Rust function): the second quorum over the ack fan-in —
the typed wire composition is the single source of truth; its `.mono` is the
growth face. -/
def two_pcM (n : Nat) (vote : Fin n → P → Bool) (d : TwoPCDecisions n P) :
    Stream P →ₘ Stream P :=
  let c_commits := tpc_c_commitsM n vote d.votes
  -- let (c_all_commit, _) = collect_quorum(c_commits…, n, n); c_all_commit
  (collect_quorumM n n d.acks ∘ₘ c_commits).fstOf

/-! ## Run views (`.f` faces of the top-level protocol only) -/

/-- Phase-1 output (Rust: `c_all_vote_yes`) at the run. -/
def two_pcVoteYes (n : Nat) (vote : Fin n → P → Bool) (payloads : Stream P)
    (dvotes : List (List (P × Except Unit Unit))) : Stream P :=
  (tpc_vote_yesM n vote dvotes).f payloads

/-- Committed output (Rust: `c_all_commit`, the return value of `two_pc`). -/
def two_pc (n : Nat) (vote : Fin n → P → Bool) (payloads : Stream P)
    (d : TwoPCDecisions n P) : Stream P :=
  (two_pcM n vote d).f payloads

/-! ## Decision legality: complete consumption (`Hydro.Consumes`)

A `batchC` decision is *complete* for an availability when it consumes it
exactly (as a multiset — `Consumes`, `Hydro/TStream.lean`). Legality
(`batchC` emits every decided batch) is a consequence
(`Consumes.batchC_eq`), so completeness is the only hypothesis theorems
need. -/

/-- Validity of a decision pair: each quorum's decision completely consumes
its fan-in union. The phase-2 availability depends on the phase-1 *output* —
the adversary decides the second round after seeing the first, as in a real
execution. -/
def TwoPCDecisions.Valid (n : Nat) (vote : Fin n → P → Bool)
    (payloads : Stream P) (d : TwoPCDecisions n P) : Prop :=
  Consumes (unionF ((tpc_c_votesM n vote).f payloads)) d.votes ∧
  Consumes (unionF ((tpc_c_commitsM n vote d.votes).f payloads)) d.acks

/-! ## Module face: counting the broadcast-echo traffic

The coordinator's inputs are `batchC` consumptions of the member-family
unions. All observables of `collect_quorum` are per-key counts, and counts
are permutation-invariant — so under complete decisions they are a
*deterministic* function of the participant buffers. -/

section Counting

variable {P : Type} [DecidableEq P]

omit [DecidableEq P] in
/-- `voteE` is a successful response exactly when the participant votes yes. -/
@[simp] theorem isOk_voteE {n : Nat} (vote : Fin n → P → Bool) (i : Fin n)
    (p : P) : (voteE vote i p).isOk = vote i p := by
  unfold voteE
  by_cases h : vote i p
  · simp only [h, if_true]
    rfl
  · simp only [Bool.not_eq_true] at h
    simp only [h, Bool.false_eq_true, if_false]
    rfl

/-- Per-member keyed counts of the phase-1 vote buffers. -/
theorem votesBuf_countKeyP {n : Nat} (vote : Fin n → P → Bool)
    (payloads : Stream P) (i : Fin n) (k : P) (Pr : Except Unit Unit → Bool) :
    Stream.countKeyP ((tpc_c_votesM n vote).f payloads i) k Pr
      = if Pr (voteE vote i k) then payloads.count k else 0 := by
  show List.countP _ (List.map (fun p => (p, voteE vote i p)) payloads) = _
  rw [List.countP_map]
  exact List.countP_eq_and_ite payloads k (fun p => Pr (voteE vote i p))

/-- Per-member keyed counts of the phase-2 ack buffers. -/
theorem acksBuf_countKeyP {n : Nat} (vote : Fin n → P → Bool)
    (payloads : Stream P) (dvotes : List (List (P × Except Unit Unit)))
    (i : Fin n) (k : P) (Pr : Except Unit Unit → Bool) :
    Stream.countKeyP ((tpc_c_commitsM n vote dvotes).f payloads i) k Pr
      = if Pr (.ok ())
        then (two_pcVoteYes n vote payloads dvotes).count k else 0 := by
  show List.countP _ (List.map (fun p => (p, Except.ok ()))
    (two_pcVoteYes n vote payloads dvotes)) = _
  rw [List.countP_map]
  exact List.countP_eq_and_ite (two_pcVoteYes n vote payloads dvotes) k
    (fun _ => Pr (.ok ()))

end Counting

/-! ## Canonical decisions (for executable tests)

Whole-batch or singleton-batch consumption of the canonical (member-major)
availability order — executable instances of the adversary for `#guard`
smoke tests. -/

/-- A canonical valid decision pair, given batching strategies for the two
quorum sites. -/
def TwoPCDecisions.canonical (n : Nat) (vote : Fin n → P → Bool)
    (payloads : Stream P)
    (strat₁ strat₂ :
      Stream (P × Except Unit Unit) → List (List (P × Except Unit Unit))) :
    TwoPCDecisions n P :=
  let votes := strat₁ (unionF ((tpc_c_votesM n vote).f payloads))
  { votes := votes
    acks := strat₂ (unionF ((tpc_c_commitsM n vote votes).f payloads)) }

/-- Canonical decisions are valid (face — the executable adversary instances
inhabit the theorems' hypothesis space). -/
theorem TwoPCDecisions.canonical_valid (n : Nat) (vote : Fin n → P → Bool)
    (payloads : Stream P)
    (strat₁ strat₂ :
      Stream (P × Except Unit Unit) → List (List (P × Except Unit Unit)))
    (h₁ : ∀ s, (strat₁ s).flatten.Perm s) (h₂ : ∀ s, (strat₂ s).flatten.Perm s) :
    (TwoPCDecisions.canonical n vote payloads strat₁ strat₂).Valid n vote
      payloads :=
  ⟨h₁ _, h₂ _⟩

/-! ## Executable smoke tests (3 participants)

What the Rust simulator would explore with `exhaustive()` on bounded inputs:
mixed votes, whole vs singleton batching decisions. -/

private def yesVote : Fin 3 → Nat → Bool := fun _ _ => true
/-- Participant 1 aborts payload 10 (the Rust comment's scenario). -/
private def mixedVote : Fin 3 → Nat → Bool := fun i p => !(i.val = 1 && p = 10)

private def runTwoPC (vote : Fin 3 → Nat → Bool) (payloads : List Nat)
    (strat₁ strat₂ :
      Stream (Nat × Except Unit Unit) → List (List (Nat × Except Unit Unit))) :
    List Nat :=
  two_pc 3 vote payloads (TwoPCDecisions.canonical 3 vote payloads strat₁ strat₂)

-- All-yes: everything commits (in some NoOrder-irrelevant order).
#guard (runTwoPC yesVote [10, 20, 30] Batching.whole Batching.whole).mergeSort (· ≤ ·) = [10, 20, 30]
#guard (runTwoPC yesVote [10, 20, 30] Batching.singletons Batching.singletons).mergeSort (· ≤ ·) = [10, 20, 30]
#guard (runTwoPC yesVote [10, 20, 30] Batching.singletons Batching.whole).mergeSort (· ≤ ·) = [10, 20, 30]
-- Mixed votes: payload 10 is aborted by participant 1, everything else commits.
#guard (runTwoPC mixedVote [10, 20, 30] Batching.whole Batching.whole).mergeSort (· ≤ ·) = [20, 30]
#guard (runTwoPC mixedVote [10, 20, 30] Batching.singletons Batching.singletons).mergeSort (· ≤ ·) = [20, 30]
-- Empty input commits nothing.
#guard runTwoPC yesVote [] Batching.whole Batching.whole = []

end HydroLean.Programs
