import HydroV2.HydroParam
import HydroV2.EagerRel
import HydroV2.MonoHRel
import HydroV2.CausalHRel
import HydroV2.Paxos.LeaderElection
import HydroV2.Paxos.EagerCheck

/-!
# HydroV2 · free-theorem consumption checks

The `_param` theorems are generated at the program-file tails
(`hydro_param` next to each module's other command blocks); this file
checks consumption: instantiating them at the eager-agreement
instance reproduces the eager naming machinery as corollaries.
-/

namespace HydroV2


#check @p_ballot_calc_param₁

/-- Consumption check: the eager naming for `p_ballot_calc`, leg 1,
as a corollary of the free theorem at the eager instance. -/
example {L : Type} {mem : L → Nat} {ℓ : L}
    (r : (Eager L mem).TickSingleton ℓ (Option (Ballot (mem ℓ)))
      .unbounded) :
    ETick.den (p_ballot_calc (Eager L mem) ℓ r).val.1
      = (p_ballot_calc (Values L mem) ℓ (ETick.den r)).val.1 :=
  p_ballot_calc_param₁ (eagC L mem) eagLaws () ℓ r (ETick.den r)
    (fun _ _ => rfl) () le_rfl





/-! ## The eager knot machinery as corollaries of the free theorems
at the eager instance -/

section EagerCorollaries

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]
  (variant : PaxosVariant) (prop acc : L) (qs np : Nat)

/-- The `leader_election.p1b_fail` eager naming (formerly a
hand-written 1.6M-heartbeat `eag_knot` call in the retired
`EagerKnots.lean`) as a two-line corollary. -/
example (dc : LEDec (Eager L mem) (mem prop) (mem acc) P)
    (p2b : (Eager L mem).Stream prop
      (Ballot (mem prop)) .noOrder .exactlyOnce)
    (al : (Eager L mem).TickSingleton acc
      (ALog P (mem prop)) .unbounded)
    (ia : (Eager L mem).Stream prop
      (Ballot (mem prop)) .noOrder .atLeastOnce)
    (ff : (Eager L mem).TickSingleton prop Bool .unbounded) :
    (leader_election.p1b_fail (Eager L mem) variant prop acc qs np
      dc p2b al ia ff).den
    = leader_election.p1b_fail (Values L mem) variant prop acc qs np
        (ledecVE dc) p2b.den al.den ia.den ff.den :=
  leader_election.p1b_fail_param (eagC L mem) eagLaws ()
    variant prop acc qs np
    dc (ledecVE dc)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    p2b p2b.den (fun _ _ => rfl)
    al al.den (fun _ _ => rfl)
    ia ia.den (fun _ _ => rfl)
    ff ff.den (fun _ _ => rfl)
    () le_rfl

/-- The `leader_election.p_is_leader` eager naming (the tick knot over
the full `leCore` composition) as a corollary. -/
example (dc : LEDec (Eager L mem) (mem prop) (mem acc) P)
    (p2b : (Eager L mem).Stream prop
      (Ballot (mem prop)) .noOrder .exactlyOnce)
    (al : (Eager L mem).TickSingleton acc
      (ALog P (mem prop)) .unbounded) :
    ETick.den (leader_election.p_is_leader (Eager L mem)
      variant prop acc qs np dc p2b al)
    = leader_election.p_is_leader (Values L mem) variant prop acc qs np
        (ledecVE dc) p2b.den al.den :=
  leader_election.p_is_leader_param (eagC L mem) eagLaws ()
    variant prop acc qs np
    dc (ledecVE dc)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    p2b p2b.den (fun _ _ => rfl)
    al al.den (fun _ _ => rfl)
    () le_rfl

end EagerCorollaries

/-! ## Program-scale guarantees for free: one `_param` theorem, many
instances -/

section ProgramScale

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]
  (variant : PaxosVariant) (prop acc : L) (f : Nat)

/-- **Whole-program Flo monotonicity for free**: both output legs of
`paxos_core` are ⊑-monotone in the client inputs — `paxos_core_param`
at the mono instance. No walker, no per-module generation. -/
example (cp cp' : (Values L mem).Stream prop P .totalOrder .exactlyOnce)
    (ck ck' : (Values L mem).TickSingleton acc (Option Nat) .unbounded)
    (d : PaxosCoreDec (Values L mem) (mem prop) (mem acc) P)
    (hc : ∀ i, PoolLe _ _ (cp i) (cp' i))
    (hk : ∀ i, ck i <+: ck' i) :
    ∀ i, PoolLe _ _
      ((paxos_core (Values L mem) variant prop acc f cp ck d).val.2 i)
      ((paxos_core (Values L mem) variant prop acc f cp' ck' d).val.2
        i) :=
  paxos_core_param₂ (monoC L mem) monoLaws () variant prop acc f
    cp cp' (fun _ _ => hc) ck ck' (fun _ _ => hk) d d
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl)
    () le_rfl

variable {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool}

/-- **Whole-program step causality for free**: `paxos_core`'s machine
run below any horizon depends only on the inputs below it —
`paxos_core_param` at the causal instance. -/
example (h : Nat)
    (cp cp' : (SchedSem L mem pacing).Stream prop P .totalOrder
      .exactlyOnce)
    (ck ck' : (SchedSem L mem pacing).TickSingleton acc (Option Nat)
      .unbounded)
    (d : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc) P)
    (hc : SAgree h cp cp') (hk : TAgree h ck ck') :
    SAgree h
      ((paxos_core (SchedSem L mem pacing) variant prop acc f cp ck
        d).val.2)
      ((paxos_core (SchedSem L mem pacing) variant prop acc f cp' ck'
        d).val.2) :=
  paxos_core_param₂ (causalC L mem pacing) causalLaws h
    variant prop acc f
    cp cp' (fun _ hj => SAgree.mono hj hc)
    ck ck' (fun _ hj => TAgree.mono hj hk) d d
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl)
    h le_rfl

end ProgramScale

end HydroV2
