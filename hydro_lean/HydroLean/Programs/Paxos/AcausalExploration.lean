import HydroLean.Programs.Paxos.PaxosCore

/-!
# EXPLORATION record (E1): can an acausal decision script fabricate leadership?

**Status: executable record (keep; `lake exe explore`)** — the experiment
behind FINDINGS D21 and the derivation of `leader_ballot_stable`
(docs/11-causal-availability.md).

**Question**: is `LeaderBallotStable` load-bearing as a *hypothesis* — i.e.
does the decision space's acausal freedom (batch/snapshot legality checked
against completed streams) admit a run where even the **guarded** variant
double-commits?

**ANSWER (hand analysis + the executable runs below): NO — the violation
is not representable in the current model; `LeaderBallotStable` is
DERIVABLE with no framework change.** The fabrication self-destructs
through four facts the model already has:

1. **Solicitation is fixpoint-staged.** `p_trigger_election` is gated on
   `!p_is_leader` (paxos.rs:449), and `p_is_leader` is a `forward_ref`
   CYCLE wire — the trigger at any iterate reads the *previous* iterate's
   flag. Kleene iteration from ⊥ means a fabricated reign cannot
   bootstrap: leadership at ballot `b` needs a full `b`-bucket in the
   view, which needs `Ok @ b` replies, which (echo face + batch legality)
   need `b` solicited, which needs a flag-false `b`-tick — and at the
   fabricated reign every `b`-tick's flag is true. The executable shows
   the acausal script simply STARVES (P0's flag stream blocks; no `bC`
   ever solicited; no second commit).
2. **`p_has_largest_ballot` ≡ true** at realized ticks (the ballot jumps
   over the received max in the same tick), so the ONLY flag-false
   mechanism at a `b`-tick is bucket masking: the view's max FULL bucket
   differing from `b`.
3. **Masking self-poisons.** A mask needs a strictly larger full OWN
   bucket `bD` (replies are addressed to the ballot's owner) delivered
   before the masked tick `u`; but own ballots are num-monotone, so
   `bD`'s own solicitation tick `w` comes after `u`, and views only
   accumulate — at `w` the full `bD` bucket is already in view, making
   `w`'s flag TRUE and `bD` unsolicitable. Fabrication levels strictly
   ascend; the run is finite; the topmost fabricated ballot has no mask.
   Contradiction.
4. Everything else the safety proof needs already holds ∀ decisions
   (promise order, write-before-ack, covered values) — leadership
   stability was the ONLY causal fact, and it is derivable.

**Consequence (now realized)**: the user's "causality should come from the
fixpoint" instinct is exactly right, and no diagonal-availability machinery
was needed for the headline — `commit_agreement` dropped `hstab` via the
protocol-level derivation `leader_ballot_stable` (`PaxosCore.lean`: the
well-founded regress `saPv_no_false_full` over fabricated-ballot levels
along the LE fixpoint chain). The generic causal-availability options
(A/B/C) remain relevant only as *proof-simplification* and for future
programs whose gates are less protective — see docs/11 and
`Hydro/CausalAvail.lean`.

The two scripts below make both halves executable (`lake exe explore`):
- `nd` (acausal attempt): P0 tick 2 would keep the reign through the
  `bA→bC` switch via an early-delivered `bC` quorum; the run starves
  instead — flags block at `[false, true]`, commits: P1 `x@0` only.
- `ndCausal` (control): the same `bC` entries delivered at the earliest
  causal tick; the reign breaks (flag `[false,true,false,true,true]`),
  `just_became_leader` fires at the `bC` reign's start, the recommit
  reads the carried log — slot 0 (count 2 > f) is recognized as committed
  and skipped, `max_slot = 0` bases fresh payloads at slot 1: P0 commits
  `y@1`, P1 `x@0`. Agreement, with the healing visible.
-/

namespace HydroLean.Programs.Paxos.Acausal

open HydroLean.Hydro
open HydroLean.Programs.Paxos

def bA : Ballot 2 := ⟨0, 0⟩
def bB : Ballot 2 := ⟨0, 1⟩
def bC : Ballot 2 := ⟨1, 0⟩

def yv : Nat := 20
def xv : Nat := 10

/-- The log acceptors 1/2 carry once P1's `x` is written at slot 0. -/
def logX : P1bPayload Nat 2 := (none, [(0, ⟨bB, some xv⟩)])

/-! ## The acausal attempt -/

def leN : LENondet Nat 2 3 where
  -- P0: tick 2 receives bB (jump to bC); tick 3 nothing (stays bC).
  maxSnap := fun i => if i.val = 0 then [[], [], [bB], []] else [[], []]
  -- P0 triggers at ticks 0 and 3 (flag-false ticks); P1 at its tick 0.
  heartbeat := fun i =>
    if i.val = 0 then ⟨[], [true, false, false, true], [true, false, false, true]⟩
    else ⟨[1], [true, false], [true, false]⟩
  p1b := fun i =>
    if i.val = 0 then
      { -- quorum stream: bA entries (replies to tick-0 solicitation),
        -- then bC entries (replies to the tick-3 solicitation).
        quorumBatch := [[(bA, .ok (none, [])), (bA, .ok (none, []))],
                        [(bC, .ok logX), (bC, .ok logX)]]
        -- snapshot increments: tick 1 sees the bA quorum (causal);
        -- tick 2 sees the bC quorum (ACAUSAL — replies to tick 3's
        -- solicitation); ticks 0,3 empty.
        quorumSnap := [[],
                       [(bA, (none, [])), (bA, (none, []))],
                       [(bC, logX), (bC, logX)],
                       []] }
    else
      { quorumBatch := [[(bB, .ok (none, [])), (bB, .ok (none, []))]]
        quorumSnap := [[], [(bB, (none, [])), (bB, (none, []))]] }
  -- acceptors 1,2: bA; bB; (tick 2 = P2a x); bC at tick 3.
  p1aBatch := fun j =>
    if j.val = 0 then []
    else [[bA], [bB], [], [bC], []]
  fuel := 10

def p2aX : P2a Nat 2 := ⟨1, bB, 0, some xv⟩
/-- P0's fresh `y` at slot 0 @ `bC` (the poisoned emission, if tick 2
realizes with a continuous reign). -/
def p2aY : P2a Nat 2 := ⟨0, bC, 0, some yv⟩

def spN : SPNondet Nat 2 3 where
  payloadBatch := fun i => if i.val = 0 then [0, 0, 1, 0] else [0, 1]
  p2aBatch := fun j =>
    if j.val = 0 then []
    else [[], [], [p2aX], [], [p2aY], []]
  quorumBatch := fun i =>
    if i.val = 0 then [[((0, bC), .ok ()), ((0, bC), .ok ())]]
    else [[((0, bB), .ok ()), ((0, bB), .ok ())]]
  joinBatch := fun i =>
    if i.val = 0 then [[], [], [(0, bC)]] else [[], [(0, bB)]]

def nd : PaxosNondet Nat 2 3 := ⟨leN, spN, 10⟩

def clients : (Fin 2 → Stream (Ballot 2)) →ₘ (Fin 2 → Stream Nat) :=
  ⟨fun _ i => if i.val = 0 then [yv] else [xv],
   fun _ _ => List.prefix_refl _⟩

/-! ## The causal control

Same bC data, causal order: P0 tick 2 = flag-false bC-tick (view still
maxes at the full bA bucket ≠ bC ⇒ flag false) and triggers; acceptors
promise bC with logs carrying x; tick 3 sees the bC quorum, flag rises,
`just_became_leader` ⇒ recommit reads logX — slot 0 has count 2 > f ⇒
already committed ⇒ SKIPPED (paxos.rs:640–654), max_slot = 0 ⇒ fresh
payload `y` indexed at slot 1. Expected: P1 commits x@0, P0 commits y@1 —
agreement, and the healed run never touches slot 0 again. -/

def p2aY1 : P2a Nat 2 := ⟨0, bC, 1, some yv⟩

def leNCausal : LENondet Nat 2 3 where
  maxSnap := fun i => if i.val = 0 then [[], [], [bB], [], []] else [[], []]
  heartbeat := fun i =>
    if i.val = 0 then
      ⟨[], [true, false, true, false, false], [true, false, true, false, false]⟩
    else ⟨[1], [true, false], [true, false]⟩
  p1b := fun i =>
    if i.val = 0 then
      { quorumBatch := [[(bA, .ok (none, [])), (bA, .ok (none, []))],
                        [(bC, .ok logX), (bC, .ok logX)]]
        quorumSnap := [[],
                       [(bA, (none, [])), (bA, (none, []))],
                       [],
                       [(bC, logX), (bC, logX)],
                       []] }
    else
      { quorumBatch := [[(bB, .ok (none, [])), (bB, .ok (none, []))]]
        quorumSnap := [[], [(bB, (none, [])), (bB, (none, []))]] }
  p1aBatch := fun j =>
    if j.val = 0 then []
    else [[bA], [bB], [], [bC], [], [], []]
  fuel := 10

def spNCausal : SPNondet Nat 2 3 where
  payloadBatch := fun i => if i.val = 0 then [0, 0, 0, 0, 1] else [0, 1]
  p2aBatch := fun j =>
    if j.val = 0 then []
    else [[], [], [p2aX], [], [], [p2aY1], []]
  quorumBatch := fun i =>
    if i.val = 0 then
      [[((1, bC), .ok ()), ((1, bC), .ok ())]]
    else [[((0, bB), .ok ()), ((0, bB), .ok ())]]
  joinBatch := fun i =>
    if i.val = 0 then [[], [], [], [], [(1, bC)]] else [[], [(0, bB)]]

def ndCausal : PaxosNondet Nat 2 3 := ⟨leNCausal, spNCausal, 10⟩

end HydroLean.Programs.Paxos.Acausal
