import Examples.Tutorials.QuorumBase
import Hydro.CoupleProj

/-!
# Quorum tutorial, stage 1: the program

`vote_quorum` as it is first written: the two network hops and the coordinator's `tick` block,
with no contract, no loop invariant and no ghost facts. This file holds the program of
[Implementing a Quorum](docs/docs/hydro-lean/using/quickstart/implementing-quorum.mdx).

The files under `Examples/TutorialSteps/` hold the quickstart's intermediate stages. They form the
`HydroTutorialSteps` library, which `Examples.lean` does not import and `Examples/AxCheck.lean` does
not check; the `sorry`s in these files are the only ones the tree allows.
`Examples/Tutorials/Quorum.lean` holds the finished, `sorry`-free program.
-/

namespace Hydro.TutorialSteps.QuorumProgram

open Hydro.Tutorials

variable {L : Type} {mem : L → Nat}

--[signature]--
hydro def vote_quorum (H : HydroSem L mem) (coord part : L) (minYes : Nat)
    (vote : Fin (mem part) → Nat → Bool)
    (requests : H.Stream coord Nat .noOrder .exactlyOnce)
    (dec : QuorumDec H (mem coord)) (sched : QuorumSched H (mem coord) (mem part)) :
    H.Stream coord Nat .noOrder .exactlyOnce :=
  --[/signature]--
  --[network]--
  -- the coordinator broadcasts every request to every participant
  let p_requests := H.values (H.broadcast_closed sched.toParticipants requests)
  -- each participant votes on each request it receives
  let p_votes := H.map p_requests (fun j p => (p, vote j p))
  -- the votes travel back to the coordinator
  let c_votes := H.values (H.broadcast_closed sched.toCoordinator p_votes)
  --[/network]--
  --[tick-header]--
  tick (state seen : H.BoundedStream (Nat × Bool) .noOrder .exactlyOnce)
      (state decided : H.BoundedStream Nat .noOrder .exactlyOnce)
      (input incoming := H.batch c_votes dec.votes) :=
    --[/tick-header]--
    --[tick-body]--
    let all := H.bchain seen incoming
    let yes := H.bkfold yesStep 0 (fun _ _ => True) (fun s x y _ => yesStep_comm s x y)
      (H.binto_keyed all)
    let reached := H.bksKeys (H.bksFilter yes (fun n => decide (minYes ≤ n)))
    let fresh := H.bfilterNotIn reached decided
    rebind (seen := all, decided := H.bchain decided fresh)
    yield (decisions := fresh);
    --[/tick-body]--
  --[result]--
  H.allTicks decisions
  --[/result]--

--[census]--
#nondet_census vote_quorum (nondets := 1) (scheds := 2) (fuels := 0)
--[/census]--

end Hydro.TutorialSteps.QuorumProgram
