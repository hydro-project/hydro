module

public import Veil

/-! # The Hydro Lean quickstart's voting quorum, in Veil

The protocol of `vote_quorum` (`hydro_lean/Examples/Tutorials/Quorum.lean`) as a Veil 2.0
transition system, for the comparison in `docs/docs/hydro-lean/using/comparison.mdx`.
This file is NOT part of the Hydro Lean lake package: it builds inside a Veil checkout
(`lake build Examples.Hydro.VoteQuorumVeil`), with Veil's own Lean toolchain.

One coordinator. `minYes` is not a number here: counting is outside the first-order
fragment Veil decides, so a quorum is an element of an uninterpreted sort `qset`, and
"`minYes` participants voted yes" is "every member of some quorum voted yes". The model
check instantiates the quorums as the 2-subsets of three participants.

`veil.smt.trust false` makes `#check_invariants` reconstruct a Lean proof of every
verification condition instead of trusting the SMT solver's answer.
-/

set_option veil.smt.trust false

veil module VoteQuorum

type participant
type request
type qset

--[state]--
-- quorums are sets of participants; none is empty (Hydro's `1 ≤ minYes`)
immutable relation member (j : participant) (q : qset)
-- vote j r: participant j's voting rule, chosen at init and never changed
relation vote (j : participant) (r : request)
relation proposed (r : request)
-- requests in flight to participants, and votes in flight to the coordinator
relation reqMsg (r : request) (j : participant)
relation voteMsg (j : participant) (r : request) (yes : Bool)
-- votes that have arrived at the coordinator since its last tick
relation arrived (j : participant) (r : request) (yes : Bool)
-- the coordinator's state, as in the `tick` block
relation seen (j : participant) (r : request) (yes : Bool)
relation decided (r : request)
-- the output stream: emitted at least once, emitted twice
relation emitted (r : request)
relation dup (r : request)
--[/state]--

#gen_state

assumption [quorum_nonempty] ∀ q, ∃ j, member j q

after_init {
  vote J R := *
  proposed R := false
  reqMsg R J := false
  voteMsg J R B := false
  arrived J R B := false
  seen J R B := false
  decided R := false
  emitted R := false
  dup R := false
}

--[actions]--
action propose (r : request) {
  require ¬ proposed r
  proposed r := true
  reqMsg r J := true
}

action cast_vote (j : participant) (r : request) {
  require reqMsg r j
  reqMsg r j := false
  voteMsg j r (vote j r) := true
}

action deliver (j : participant) (r : request) (b : Bool) {
  require voteMsg j r b
  voteMsg j r b := false
  arrived j r b := true
}

-- one coordinator tick: consume what has arrived, emit the requests newly at quorum
action tick {
  seen J R B := decide $ seen J R B ∨ arrived J R B
  arrived J R B := false
  dup R := decide $ dup R ∨
    (emitted R ∧ ¬ decided R ∧ ∃ q, ∀ j, member j q → seen j R true)
  emitted R := decide $ emitted R ∨
    (¬ decided R ∧ ∃ q, ∀ j, member j q → seen j R true)
  decided R := decide $ decided R ∨ ∃ q, ∀ j, member j q → seen j R true
}
--[/actions]--

--[safety]--
safety [decided_proposed] emitted R → proposed R
safety [quorum] emitted R → ∃ q, ∀ j, member j q → vote j R
safety [once] ¬ dup R
--[/safety]--

--[inv]--
-- with the safety clauses, an inductive invariant; dropping any clause makes
-- `#check_invariants` fail
invariant [req_proposed] reqMsg R J → proposed R
invariant [vote_honest] voteMsg J R B → proposed R ∧ B = vote J R
invariant [arrived_honest] arrived J R B → proposed R ∧ B = vote J R
invariant [seen_honest] seen J R B → proposed R ∧ B = vote J R
invariant [emitted_decided] emitted R ↔ decided R
--[/inv]--

--[liveness]--
-- under weak fairness of voting, delivery and ticking, a proposed request that some quorum
-- votes yes on is eventually emitted
temporal [decides]
  (∀ j, ∀ r, 𝒲ℱ (cast_vote j r)) ∧ (∀ j, ∀ r, ∀ b, 𝒲ℱ (deliver j r b)) ∧ 𝒲ℱ tick →
    ∀ r, (⌜ proposed r ∧ ∃ q, ∀ j, member j q → vote j r ⌝ ↝ ⌜ emitted r ⌝)
--[/liveness]--

#gen_spec

-- non-vacuity: some run of four actions emits a request
sat trace [can_decide] {
  any 4 actions
  assert (∃ r, emitted r)
}

#model_check { participant := Fin 3, request := Fin 2, qset := Fin 3 }
  { member := fun j q => j ≠ q }

#check_invariants

#temporal_check { participant := Fin 3, request := Fin 1, qset := Fin 3 }
  { member := fun j q => j ≠ q }

end VoteQuorum
