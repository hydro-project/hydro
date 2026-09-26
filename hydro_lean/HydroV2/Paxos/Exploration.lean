import HydroV2.Paxos.PaxosCore
import HydroV2.Paxos.EagerCheck

/-!
# EXPLORATION record (E1): can an acausal decision script fabricate
leadership? — V2

The V1 experiment behind FINDINGS D21 and the derivation of
`leader_ballot_stable`, re-expressed in the V2 decision vocabulary
(`lake exe explore`; see the V1 record for the full analysis).

**Question**: is leader-ballot stability load-bearing as a
*hypothesis* — does the decision space's acausal freedom (batch and
snapshot legality checked against completed pools) admit a run where
even the **guarded** variant double-commits?

**ANSWER: NO — the fabrication self-destructs.** Solicitation is
fixpoint-staged: `p_trigger_election` is gated on `!p_is_leader`, a
`forward_ref` cycle wire, so a fabricated reign cannot bootstrap —
leadership at ballot `b` needs `Ok @ b` replies, which need `b`
solicited, which needs a flag-false `b`-tick, and at the fabricated
reign every `b`-tick's flag is true. The acausal script below simply
**starves** (P0's flag trace truncates; no `bC` ever solicited; no
second commit), while the causal control (same `bC` data, delivered at
the earliest causal tick) breaks the reign, heals through the
recommit (slot 0 recognized as committed and skipped, fresh payload
based at slot 1), and agrees.

Scenario (nP = 2, nA = 3, f = 1): P0 owns `bA = (0,0)` then jumps to
`bC = (1,0)` after seeing P1's `bB = (0,1)`; P1 commits `x` at slot 0
under `bB`; the scripts differ only in *when* P0's quorum snapshot
shows the `bC` bucket (acausally at tick 2 — replies to tick 3's
solicitation — vs causally at tick 3).
-/

namespace HydroV2
namespace Exploration

/-- The `Values` instance at two proposers, three acceptors. -/
abbrev HX : HydroSem PaxLoc (paxMem 2 3) := Values PaxLoc (paxMem 2 3)

def bA : Ballot 2 := ⟨0, 0⟩
def bB : Ballot 2 := ⟨0, 1⟩
def bC : Ballot 2 := ⟨1, 0⟩

def yv : Nat := 20
def xv : Nat := 10

/-- The log acceptors 1/2 carry once P1's `x` is written at slot 0. -/
def logX : Option Nat × LogMap Nat 2 := (none, [(0, ⟨bB, some xv⟩)])

def p2aX : P2a Nat 2 := ⟨1, bB, 0, some xv⟩
/-- P0's fresh `y` at slot 0 @ `bC` (the poisoned emission, if tick 2
realizes with a continuous reign). -/
def p2aY : P2a Nat 2 := ⟨0, bC, 0, some yv⟩
/-- P0's fresh `y` at slot 1 @ `bC` (the healed emission). -/
def p2aY1 : P2a Nat 2 := ⟨0, bC, 1, some yv⟩

/-! ## The acausal attempt -/

def ndDec : PaxosCoreDec HX 2 3 Nat where
  le :=
    { -- P0 tick 2 receives bB (jump to bC); P1 sees nothing
      receivedMax := fun i =>
        if i.val = 0 then [{}, {}, {bB}, {}] else [{}, {}]
      hb := { -- P1 gossips its reign at tick 1 (P0's bB source)
              sample := fun i => if i.val = 0 then [] else [1]
              -- P0 triggers at ticks 0 and 3; P1 at its tick 0
              timeout := fun i =>
                if i.val = 0 then [true, false, false, true]
                else [true, false]
              interval := fun i =>
                if i.val = 0 then [true, false, false, true]
                else [true, false]
              ial := () }
      p1aCh := ()
      -- acceptors 1, 2: bA; bB; (tick 2 = P2a x); bC at tick 3
      p1aBatch := fun j =>
        if j.val = 0 then []
        else [{bA}, {bB}, {}, {bC}, {}]
      p1bCh := ()
      p1b := { -- P0: the bA quorum at tick 1, the bC quorum at tick 2
               -- (ACAUSAL — replies to tick 3's solicitation)
               cqwr := fun i =>
                 if i.val = 0 then
                   [{}, {(bA, .ok (none, [])), (bA, .ok (none, []))},
                    {(bC, .ok logX), (bC, .ok logX)}, {}]
                 else
                   [{}, {(bB, .ok (none, [])), (bB, .ok (none, []))}]
               cqwrEmit := ()
               order := fun i =>
                 if i.val = 0 then
                   [(bA, (none, [])), (bA, (none, [])),
                    (bC, logX), (bC, logX)]
                 else [(bB, (none, [])), (bB, (none, []))]
               snap := fun i =>
                 if i.val = 0 then [0, 2, 2, 0] else [0, 2] }
      fuelFail := (1 : UnfoldFuel)
      fuelIAL := (1 : UnfoldFuel)
      fuelLead := (5 : UnfoldFuel) }
  sp :=
    { payloadBatch := fun i =>
        if i.val = 0 then [0, 0, 1, 0] else [0, 1]
      rcEmit := ()
      p2aCh := ()
      p2aBatch := fun j =>
        if j.val = 0 then []
        else [{}, {}, {p2aX}, {}, {p2aY}]
      p2bCh := ()
      cqBatch := fun i =>
        if i.val = 0 then
          [{((0, bC), .ok ()), ((0, bC), .ok ())}]
        else [{}, {((0, bB), .ok ()), ((0, bB), .ok ())}]
      cqEmit := ()
      jrBatch := fun i =>
        if i.val = 0 then [{}, {}, {((0, bC), ())}]
        else [{}, {((0, bB), ())}]
      jrEmit := () }
  fuelSeqMax := (1 : UnfoldFuel)
  fuelALog := (3 : UnfoldFuel)

/-! ## The causal control

Same `bC` data, causal order: P0 tick 2 = flag-false `bC`-tick (view
still maxes at the full `bA` bucket ≠ `bC` ⇒ flag false) and triggers;
acceptors promise `bC` with logs carrying `x`; tick 3 sees the `bC`
quorum, the flag rises, `just_became_leader` fires ⇒ the recommit
reads `logX` — slot 0 has count 2 > f ⇒ already committed ⇒ SKIPPED,
`max_slot = 0` ⇒ fresh payload `y` indexed at slot 1. Expected: P1
commits `x@0`, P0 commits `y@1` — agreement, healing visible. -/

def ndCausalDec : PaxosCoreDec HX 2 3 Nat where
  le :=
    { receivedMax := fun i =>
        if i.val = 0 then [{}, {}, {bB}, {}, {}] else [{}, {}]
      hb := { sample := fun i => if i.val = 0 then [] else [1]
              -- P0 triggers at ticks 0 and 2 (the causal bC tick)
              timeout := fun i =>
                if i.val = 0 then [true, false, true, false, false]
                else [true, false]
              interval := fun i =>
                if i.val = 0 then [true, false, true, false, false]
                else [true, false]
              ial := () }
      p1aCh := ()
      p1aBatch := fun j =>
        if j.val = 0 then []
        else [{bA}, {bB}, {}, {bC}, {}, {}, {}]
      p1bCh := ()
      p1b := { cqwr := fun i =>
                 if i.val = 0 then
                   [{}, {(bA, .ok (none, [])), (bA, .ok (none, []))},
                    {}, {(bC, .ok logX), (bC, .ok logX)}, {}]
                 else
                   [{}, {(bB, .ok (none, [])), (bB, .ok (none, []))}]
               cqwrEmit := ()
               order := fun i =>
                 if i.val = 0 then
                   [(bA, (none, [])), (bA, (none, [])),
                    (bC, logX), (bC, logX)]
                 else [(bB, (none, [])), (bB, (none, []))]
               snap := fun i =>
                 if i.val = 0 then [0, 2, 0, 2, 0] else [0, 2] }
      fuelFail := (1 : UnfoldFuel)
      fuelIAL := (1 : UnfoldFuel)
      fuelLead := (6 : UnfoldFuel) }
  sp :=
    { payloadBatch := fun i =>
        if i.val = 0 then [0, 0, 0, 0, 1] else [0, 1]
      rcEmit := ()
      p2aCh := ()
      p2aBatch := fun j =>
        if j.val = 0 then []
        else [{}, {}, {p2aX}, {}, {}, {p2aY1}, {}]
      p2bCh := ()
      cqBatch := fun i =>
        if i.val = 0 then
          [{((1, bC), .ok ()), ((1, bC), .ok ())}]
        else [{}, {((0, bB), .ok ()), ((0, bB), .ok ())}]
      cqEmit := ()
      jrBatch := fun i =>
        if i.val = 0 then [{}, {}, {}, {}, {((1, bC), ())}]
        else [{}, {((0, bB), ())}]
      jrEmit := () }
  fuelSeqMax := (1 : UnfoldFuel)
  fuelALog := (3 : UnfoldFuel)

/-- Clients: P0's client sends `y`, P1's sends `x`. -/
def clients : Fin 2 → List Nat :=
  fun i => if i.val = 0 then [yv] else [xv]

/-- No checkpoints. -/
def ckpt : Fin 3 → Trace (Option Nat) :=
  fun _ => [none, none, none, none, none, none, none]

end Exploration
end HydroV2
