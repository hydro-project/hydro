import Examples.Tutorials.QuorumBase
import Hydro.CoupleProj

/-!
# Quorum tutorial, stage 2: the contract, every field `sorry`

`vote_quorum` with its contract attached (`ensures out => QuorumEnsures …`) and a `prove`
clause whose three fields are `sorry`: the file elaborates, and Lean reports the `sorry`.

The files under `Examples/TutorialSteps/` hold the quickstart's intermediate stages. They form the
`HydroTutorialSteps` library, which `Examples.lean` does not import and `Examples/AxCheck.lean` does
not check; the `sorry`s in these files are the only ones the tree allows.
`Examples/Tutorials/Quorum.lean` holds the finished, `sorry`-free program.
-/

namespace Hydro.TutorialSteps.QuorumSpec

open Hydro.Tutorials

variable {L : Type} {mem : L → Nat}

--[signature]--
hydro def vote_quorum (H : HydroSem L mem) (coord part : L) (minYes : Nat)
    (vote : Fin (mem part) → Nat → Bool)
    (requests : H.Stream coord Nat .noOrder .exactlyOnce)
    (dec : QuorumDec H (mem coord)) (sched : QuorumSched H (mem coord) (mem part)) :
    H.Stream coord Nat .noOrder .exactlyOnce
  ensures out => QuorumEnsures minYes vote requests out :=
  --[/signature]--
  let p_requests := H.values (H.broadcast_closed sched.toParticipants requests)
  let p_votes := H.map p_requests (fun j p => (p, vote j p))
  let c_votes := H.values (H.broadcast_closed sched.toCoordinator p_votes)
  tick (state seen : H.BoundedStream (Nat × Bool) .noOrder .exactlyOnce)
      (state decided : H.BoundedStream Nat .noOrder .exactlyOnce)
      (input incoming := H.batch c_votes dec.votes) :=
    let all := H.bchain seen incoming
    let yes := H.bkfold yesStep 0 (fun _ _ => True) (fun s x y _ => yesStep_comm s x y)
      (H.binto_keyed all)
    let reached := H.bksKeys (H.bksFilter yes (fun n => decide (minYes ≤ n)))
    let fresh := H.bfilterNotIn reached decided
    rebind (seen := all, decided := H.bchain decided fresh)
    yield (decisions := fresh);
  --[prove]--
  H.allTicks decisions
  prove
    decided_proposed := fun c p hp h1 => by sorry,
    quorum := fun c p hp hone => by sorry,
    once := fun c p => by sorry
  --[/prove]--

end Hydro.TutorialSteps.QuorumSpec
