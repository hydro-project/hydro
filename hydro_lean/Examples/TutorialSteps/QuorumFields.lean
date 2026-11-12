import Examples.Tutorials.QuorumBase
import Hydro.CoupleProj

/-!
# Quorum tutorial, stage 3: two fields, worked back to one tick

The `quorum` and `decided_proposed` fields proved from two ghost facts: `hyes`, which reads the
network hops at `Values`, and `hdecided`, which says every decision has `minYes` yes votes among
the votes the coordinator receives. `hdecided` is proved from `htick`, a fact about one tick,
whose proof stops at a `sorry`: the state before the tick is opaque, and nothing yet says what it
holds. `once` is still `sorry`.

The files under `Examples/TutorialSteps/` hold the quickstart's intermediate stages. They form the
`HydroTutorialSteps` library, which `Examples.lean` does not import and `Examples/AxCheck.lean` does
not check; the `sorry`s in these files are the only ones the tree allows.
`Examples/Tutorials/Quorum.lean` holds the finished, `sorry`-free program.
-/

namespace Hydro.TutorialSteps.QuorumFields

open Hydro.Tutorials

variable {L : Type} {mem : L → Nat}

hydro def vote_quorum (H : HydroSem L mem) (coord part : L) (minYes : Nat)
    (vote : Fin (mem part) → Nat → Bool)
    (requests : H.Stream coord Nat .noOrder .exactlyOnce)
    (dec : QuorumDec H (mem coord)) (sched : QuorumSched H (mem coord) (mem part)) :
    H.Stream coord Nat .noOrder .exactlyOnce
  ensures out => QuorumEnsures minYes vote requests out :=
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
  --[ghost-tick]--
  -- the register before every tick, opaque: tick `n` reads `reg c n` and the batch at `n`
  ghost obtain ⟨reg, -, hreg_at, -, -⟩ := hdecisions_reg
  -- one tick, decoded: tick `n`'s decisions are distinct, each has `minYes` yes votes among
  -- the votes consumed through tick `n`, and none was decided before
  ghost have htick : ∀ (c : Fin (mem coord)) (n : Nat) (e : Multiset Nat),
      (decisions c)[n]? = some e →
      ∃ b_t, (incoming c)[n]? = some b_t ∧ e.Nodup ∧
        ∀ p ∈ e, minYes ≤ yesVotes (((incoming c).take n).sum + b_t) p
          ∧ p ∉ ((decisions c).take n).sum := fun c n e he => by
    -- tick `n`'s decisions are the step on `reg c n` and this tick's batch `b_t`
    obtain ⟨b_t, hb, rfl⟩ := (hreg_at c n e).mp he
    refine ⟨b_t, hb, ?_⟩
    -- evaluate the step at `Values`
    simp only [decisions_step, den]
    sorry
  --[/ghost-tick]--
  --[ghost-votes]--
  -- the votes at the coordinator, read at `Values`: every participant's vote on every
  -- proposed request
  ghost have hvotes : ∀ c, c_votes c = ((List.finRange (mem part)).map (fun j =>
      (proposed requests).map (fun q => (q, vote j q)))).sum := fun _ => rfl
  -- the yes votes for `p`: one per yes voter per copy of `p` proposed
  ghost have hyes : ∀ c p, yesVotes (c_votes c) p = yesVoters vote p * (proposed requests).count p :=
    fun c p => by
    rw [hvotes, yesVotes_listSum, List.map_map]
    have : (List.finRange (mem part)).map
        ((fun m => yesVotes m p) ∘ fun j => (proposed requests).map (fun q => (q, vote j q)))
        = (List.finRange (mem part)).map
            (fun j => if vote j p then (proposed requests).count p else 0) :=
      List.map_congr_left (fun j _ => yesVotes_map_tag _ _ p)
    rw [this, sum_map_ite_const]
    rfl
  --[/ghost-votes]--
  --[hdecided]--
  -- a decision has `minYes` yes votes among all the votes the coordinator receives
  ghost have hdecided : ∀ c, ∀ p ∈ (decisions c).sum, minYes ≤ yesVotes (c_votes c) p :=
    fun c p hp => by
    -- the tick `n` that decided `p`, and its batch `b_t`
    obtain ⟨e, he, hpe⟩ := mem_list_sum.mp hp
    obtain ⟨n, hn⟩ := List.mem_iff_getElem?.mp he
    obtain ⟨b_t, hb, -, hall⟩ := htick c n e hn
    -- the votes consumed through tick `n` sit below everything the coordinator receives
    have hle : ((incoming c).take n).sum + b_t ≤ c_votes c := by
      have : ((incoming c).take n).sum + b_t = ((incoming c).take (n + 1)).sum := by
        rw [List.take_add_one, hb, List.sum_append]
        simp only [Option.toList_some, List.sum_singleton]
      rw [this]
      exact le_trans (sublist_sum_le (List.take_sublist _ _)) (cqConsumed_le _ _)
    exact le_trans (hall p hpe).1 (yesVotes_mono hle p)
  --[/hdecided]--
  H.allTicks decisions
  --[prove]--
  prove
    decided_proposed := fun c p hp h1 => by
      -- `minYes ≤ yesVoters * count`, with `1 ≤ minYes`, forces `count ≥ 1`
      have h := le_trans h1 (hdecided c p hp)
      rw [hyes] at h
      rcases Nat.eq_zero_or_pos ((proposed requests).count p) with h0 | hpos
      · rw [h0, Nat.mul_zero] at h; omega
      · exact Multiset.count_pos.mp hpos,
    quorum := fun c p hp hone => by
      -- `minYes ≤ yesVoters * count ≤ yesVoters` when `count ≤ 1`
      have h := hdecided c p hp
      rw [hyes] at h
      exact le_trans h (le_trans (Nat.mul_le_mul_left _ hone) (Nat.mul_one _).le),
    once := fun c p => by sorry
  --[/prove]--

end Hydro.TutorialSteps.QuorumFields
