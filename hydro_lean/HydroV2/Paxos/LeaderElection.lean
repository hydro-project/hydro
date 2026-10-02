import HydroV2.Paxos.PBallotCalc
import HydroV2.Paxos.PLeaderHeartbeat
import HydroV2.Paxos.AcceptorP1
import HydroV2.Paxos.PP1b
import HydroV2.HydroDef
import HydroV2.HydroTick

/-!
# `leader_election` (paxos.rs:253–346)

The election loop, with its three `forward_ref` cycles closed by
guarded Kleene iteration:

- **`p1b_fail`** (stream, `NoOrder ExactlyOnce`): rejection ballots from
  `p_p1b` feed back into the received-max merge;
- **`p_to_proposers_i_am_leader`** (stream, `NoOrder AtLeastOnce`): the
  heartbeat gossip feeds the same merge — the merge is at-least-once
  because sampling stutters, so the exactly-once legs **weaken** into
  the retry quotient and the max fold pays commutativity *and*
  idempotence (`Ballot.maxFold_comm`/`maxFold_idem`);
- **`p_is_leader`** (tick singleton): `p_p1b`'s verdict closes the
  heartbeat's trigger gate (a standing leader stops soliciting).

Wire production is the `let body` inside `leader_election` (one body,
the Rust text, its per-pass contract colocated); the def then ties the
knots, and its clause carries the closed contract (`LEEnsures`) plus
binary Flo monotonicity. Sub-module behavior enters proofs only
through contracts (`PBCEnsures`, `PLHEnsures`, `AP1Ensures`,
`PP1bEnsures`), each quantified over arbitrary input wires — so they
apply directly to the fixed wires.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- `leader_election`'s `nondet!` sites (paxos.rs:253–346), nested by
owning module: the received-max snapshot and P1a batching are its own;
heartbeat timing (`hb`) and quorum collection (`p1b`) belong to its
callees; the three cycle fuels are the `forward_ref` unfolding
decisions. -/
structure LEDec (H : HydroSem L mem) (nP nA : Nat) (P : Type)
    [DecidableEq P] where
  /-- `p_received_max_ballot.snapshot(&proposer_tick, nondet_leader)`:
  arrival cuts of the unordered received-ballot pool. -/
  receivedMax : H.SnapDec nP (Ballot nP) .noOrder
  /-- `p_leader_heartbeat`'s timing decisions. -/
  hb : PLHDec H nP
  /-- `p_to_acceptors_p1a.batch(&acceptor_tick, nondet!(…))`
  (`acceptor_p1`'s consumption): consumed P1a increments. -/
  p1aBatch : H.BatchDec nA (Ballot nP)
  /-- `p_p1b`'s quorum-collection decisions. -/
  p1b : PP1bDec H nP P
  /-- `p1b_fail` cycle depth (`forward_ref`). -/
  fuelFail : H.FixDec
  /-- `i_am_leader` cycle depth (`forward_ref`). -/
  fuelIAL : H.FixDec
  /-- `p_is_leader` cycle depth (`forward_ref`). -/
  fuelLead : H.FixDec

/-- **`leader_election`'s adversary-side (sched-det) bundle** (`Unit`
at `Values`; see `Sem.lean`'s classification table), nested by owning
module like `LEDec`. -/
structure LESched (H : HydroSem L mem) (nP nA : Nat) (P : Type)
    [DecidableEq P] where
  /-- `p_to_acceptors_p1a.broadcast(&acceptors, TCP…)`: P1a delivery
  cursors. (The Rust site's `nondet!` is `nondet_membership` —
  unmodeled under closed membership; delivery itself is unmarked.) -/
  p1aCh : H.TransportDec nA nP
  /-- `p_leader_heartbeat`'s bundle (the `i_am_leader` gossip
  transport). -/
  hb : PLHSched H nP
  /-- `acceptor_p1`'s bundle (the P1b reply transport). -/
  ap1 : AP1Sched H nP nA
  /-- `p_p1b`'s bundle (the quorum emission linearization). -/
  p1b : PP1bSched H nP P

/-- The trivial bundle at the denotation. -/
def LESched.triv {nP nA : Nat} {P : Type} [DecidableEq P] :
    LESched (Values L mem) nP nA P := ⟨(), .triv, .triv, .triv⟩

/-- Repack `Values`-typed decisions for the `MonoRel` instantiations of
the election body (the two instances declare the same decision
vocabulary, so every field is a definitional coercion). -/
private def ledecMR {nP nA : Nat} {P : Type} [DecidableEq P]
    (d : LEDec (Values L mem) nP nA P) :
    LEDec (MonoRel L mem) nP nA P :=
  ⟨d.receivedMax, ⟨d.hb.sample, d.hb.timeout, d.hb.interval⟩,
   d.p1aBatch,
   ⟨d.p1b.cqwr, d.p1b.order, d.p1b.snap⟩,
   d.fuelFail, d.fuelIAL, d.fuelLead⟩

/-- The trivial sched bundle at `MonoRel` (its sched-det families are
`Unit` too). -/
private def leschedMR {nP nA : Nat} {P : Type} [DecidableEq P] :
    LESched (MonoRel L mem) nP nA P := ⟨(), ⟨()⟩, ⟨()⟩, ⟨⟨()⟩⟩⟩


/-! ## The P1a guard-site vocabulary (FINDINGS B1)

The release site is a cross-tick scan (`use::state`); naming its step
lets the colocated contract speak about emissions: every released
ballot rode a trigger-true tick's ballot leg, and — guarded — each
ballot is released at most once (strict `num` ascent of the emitted
stream over an owned, ascending ballot wire). -/

/-- The P1a release step (the B1 guard site): fire when the trigger is
up, unless the guarded variant already sent this ballot (`lastSent`
dedup — `variant.sendOnce`). The faithful variant re-fires on every
trigger, exactly as paxos.rs is written. -/
def p1aSendStep {nP : Nat} (sendOnce : Bool) (lastSent : Option (Ballot nP))
    (bt : Ballot nP × Bool) : Option (Ballot nP) × List (Ballot nP) :=
  if bt.2 && !(sendOnce && lastSent = some bt.1)
  then (some bt.1, [bt.1]) else (lastSent, [])

/-- The P1a release register's loop invariant (B1), in loop-variable
normal form: `out` = emissions so far (one batch per tick), `reg` =
the register (last sent), `bts` = the zipped (ballot, trigger) input
(a spectator — P slices it by `|out|` itself).
Clause 1 (lengths) indexes emissions into `bts`; clause 2 (source):
every emission batch is its tick's own ballot, at a trigger-true
tick; clause 3 (guarded dedup): the released stream is duplicate-free,
num-dominated by the register, which was itself released. The clause's
same-owner/num-ascending INPUT premises are not part of the invariant:
they are wire facts, proven at the loop obligation itself from the
colocated contracts. -/
def p1aInvariant {nP : Nat} (sendOnce : Bool)
    (out : List (List (Ballot nP))) (reg : Option (Ballot nP))
    (bts : List (Ballot nP × Bool)) : Prop :=
  out.length ≤ bts.length
  ∧ (∀ (u : Nat) (hu : u < out.length), out[u]'hu = []
      ∨ ∃ (hub : u < bts.length),
          out[u]'hu = [(bts[u]'hub).1] ∧ (bts[u]'hub).2 = true)
  ∧ (sendOnce = true →
      match reg with
      | none => out.flatten = []
      | some r => r ∈ out.flatten
          ∧ (∀ b ∈ out.flatten, b.num ≤ r.num)
          ∧ out.flatten.Nodup)

/-- One tick of the release register preserves the invariant (the B1
fire/hold case split — the `tick` construct's loop-body obligation, in
its append form: the register is handed directly, the appended batch
and the next register are the step's own components). -/
theorem p1aInvariant_tick {nP : Nat} (so : Bool)
    {out : List (List (Ballot nP))} {reg : Option (Ballot nP)}
    {bts : List (Ballot nP × Bool)} {n : Nat}
    (hown : ∀ x ∈ bts, ∀ y ∈ bts,
      (x : Ballot nP × Bool).1.proposerId = y.1.proposerId)
    (hsort : List.Pairwise
      (fun (a b : Ballot nP × Bool) => a.1.num ≤ b.1.num) bts)
    (hnb : n < bts.length) (hlen : out.length = n)
    (ih : p1aInvariant so out reg bts) :
    p1aInvariant so (out ++ [(p1aSendStep so reg (bts[n]'hnb)).2])
      (p1aSendStep so reg (bts[n]'hnb)).1 bts := by
  subst hlen
  refine ⟨?_, ?_, ?_⟩
  · simp only [List.length_append, List.length_cons, List.length_nil]
    omega
  · -- source: old ticks from `ih`, the new tick from the step
    intro u hu
    have hub : u < out.length + 1 := by
      simpa using hu
    rcases Nat.lt_or_ge u out.length with hun | hun
    · have he : (out
          ++ [(p1aSendStep so reg (bts[out.length]'hnb)).2])[u]'hu
          = out[u]'hun := List.getElem_append_left hun
      rw [he]
      exact ih.2.1 u hun
    · have hu' : u = out.length := by omega
      subst hu'
      have he : (out
          ++ [(p1aSendStep so reg (bts[out.length]'hnb)).2])[out.length]'hu
          = (p1aSendStep so reg (bts[out.length]'hnb)).2 := by
        simp
      rw [he]
      unfold p1aSendStep
      split
      next hcond =>
        right
        rw [Bool.and_eq_true] at hcond
        exact ⟨hnb, rfl, hcond.1⟩
      next => left; rfl
  · -- guarded dedup
    intro hso
    subst hso
    have hded := ih.2.2 rfl
    have hflat : (out
        ++ [(p1aSendStep true reg (bts[out.length]'hnb)).2]).flatten
        = out.flatten
          ++ (p1aSendStep true reg (bts[out.length]'hnb)).2 := by
      rw [List.flatten_append]
      simp
    by_cases hcond : ((bts[out.length]'hnb).2
        && !(true && decide (reg = some ((bts[out.length]'hnb).1))))
        = true
    · -- FIRE: trigger up, not a guarded duplicate
      have hnotdup : ¬(reg = some ((bts[out.length]'hnb).1)) := by
        rw [Bool.and_eq_true] at hcond
        have h3 := hcond.2
        simp at h3
        exact h3
      have hstep2 : (p1aSendStep true reg (bts[out.length]'hnb)).2
          = [(bts[out.length]'hnb).1] := by
        unfold p1aSendStep
        rw [if_pos hcond]
      have hstep1 : (p1aSendStep true reg (bts[out.length]'hnb)).1
          = some ((bts[out.length]'hnb).1) := by
        unfold p1aSendStep
        rw [if_pos hcond]
      rw [hstep1, hflat, hstep2]
      rcases hr : reg with - | r
      · -- first fire: nothing released before
        rw [hr] at hded
        have hded' : out.flatten = [] := hded
        rw [hded']
        refine ⟨by simp, ?_, by simp⟩
        intro e he
        simp at he
        rw [he]
      · -- subsequent fire: strictly fresher than every past release
        rw [hr] at hded
        obtain ⟨hrmem, hdom, hnd⟩ := hded
        -- the register's own source tick, before `out.length`
        obtain ⟨sub, hsub, hrs⟩ := List.mem_flatten.mp hrmem
        obtain ⟨v, hv, rfl⟩ := List.mem_iff_getElem.mp hsub
        rcases ih.2.1 v hv with hnil | ⟨hvb, heqv, -⟩
        · rw [hnil] at hrs
          exact absurd hrs (List.not_mem_nil)
        have hr_src : r = (bts[v]'hvb).1 := by
          rw [heqv] at hrs
          exact List.mem_singleton.mp hrs
        -- ascent: the register's num never exceeds the new ballot's
        have hrb_le : r.num ≤ ((bts[out.length]'hnb).1).num := by
          rw [hr_src]
          exact List.pairwise_iff_getElem.mp hsort v out.length hvb
            hnb hv
        -- strict: equal nums + shared owner would equate them
        have hrb_lt : r.num < ((bts[out.length]'hnb).1).num := by
          rcases Nat.lt_or_ge r.num ((bts[out.length]'hnb).1).num
            with h | h
          · exact h
          · exfalso
            have hnum : r.num = ((bts[out.length]'hnb).1).num := by
              omega
            have howner : r.proposerId
                = ((bts[out.length]'hnb).1).proposerId := by
              rw [hr_src]
              exact hown _ (List.getElem_mem hvb) _
                (List.getElem_mem hnb)
            exact hnotdup (by
              rw [hr, Ballot.eq_of_num_owner hnum howner])
        -- the new ballot is fresh: it dominates every past release
        have hfresh : (bts[out.length]'hnb).1 ∉ out.flatten := by
          intro hmem
          have := hdom _ hmem
          omega
        refine ⟨List.mem_append_right _ (by simp), ?_, ?_⟩
        · intro e he
          rcases List.mem_append.mp he with h | h
          · exact Nat.le_of_lt (Nat.lt_of_le_of_lt (hdom e h) hrb_lt)
          · simp at h
            rw [h]
        · rw [List.nodup_append]
          refine ⟨hnd, by simp, ?_⟩
          intro e he f hf
          rw [List.eq_of_mem_singleton hf]
          intro heq
          rw [heq] at he
          exact hfresh he
    · -- HOLD: nothing released, the register carries
      have hstep2 : (p1aSendStep true reg (bts[out.length]'hnb)).2
          = [] := by
        unfold p1aSendStep
        rw [if_neg hcond]
      have hstep1 : (p1aSendStep true reg (bts[out.length]'hnb)).1
          = reg := by
        unfold p1aSendStep
        rw [if_neg hcond]
      rw [hstep1, hflat, hstep2, List.append_nil]
      exact hded

/-- What the closed `leader_election` **ensures** against the `a_log`
input: the per-pass facts at the knot, with `stable`'s flag-prefix
hypothesis discharged by Kleene ascent (`iterate_le_succ` through the
body's `MonoRel` monotonicity). Output tuple:
(`p_ballot`, `p_is_leader`, `p_accepted_values`, `a_max_ballot`). -/
structure LEEnsures (variant : PaxosVariant) (prop acc : L) (qs : Nat)
    (al : TickV (mem acc) (ALog P (mem prop)) .unbounded)
    (o : TickV (mem prop) (Ballot (mem prop))
        (.monotonic (Ballot.numVO (nP := mem prop)))
      × TickV (mem prop) Bool .unbounded
      × (Fin (mem prop) → Trace (Multiset (ALog P (mem prop))))
      × TickV (mem acc) (Option (Ballot (mem prop)))
          (.monotonic Ballot.obtVO)) : Prop where
  /-- Ballot ownership. -/
  own : ∀ (i : Fin (mem prop)), ∀ b ∈ (o.1 i).vals,
    (b : Ballot (mem prop)).proposerId = i
  /-- Leader ticks see nonempty views. -/
  lead_ne : 1 ≤ qs → ∀ (i : Fin (mem prop)) {t : Nat}
    (hpl : t < (o.2.1 i).length) (hpr : t < (o.2.2.1 i).length),
    (o.2.1 i)[t]'hpl = true → (o.2.2.1 i)[t]'hpr ≠ 0
  /-- Ballot stability along reigns (FINDINGS D21), unconditional at the
  closed knot. -/
  stable : 1 ≤ qs → ∀ (i : Fin (mem prop)) {t : Nat}
    (ht1 : t + 1 < (o.2.1 i).length)
    (hb1 : t + 1 < (o.1 i).vals.length),
    (o.2.1 i)[t + 1]'ht1 = true →
    (o.2.1 i)[t]'(Nat.lt_of_succ_lt ht1) = true →
    (o.1 i).vals[t + 1]'hb1 = (o.1 i).vals[t]'(Nat.lt_of_succ_lt hb1)
  /-- View pinning by ballot number. -/
  pinned : 1 ≤ qs → ∀ (i : Fin (mem prop)) {t t' : Nat}
    (hpl : t < (o.2.1 i).length) (hpl' : t' < (o.2.1 i).length)
    (hpr : t < (o.2.2.1 i).length) (hpr' : t' < (o.2.2.1 i).length)
    (hpb : t < (o.1 i).vals.length) (hpb' : t' < (o.1 i).vals.length),
    (o.2.1 i)[t]'hpl = true → (o.2.1 i)[t']'hpl' = true →
    ((o.1 i).vals[t]'hpb).num = ((o.1 i).vals[t']'hpb').num →
    (o.2.2.1 i)[t]'hpr = (o.2.2.1 i)[t']'hpr'
  /-- Leader-view promise. -/
  view_promise : ∀ (i : Fin (mem prop)) {t : Nat}
    (ht : t < (o.2.1 i).length), (o.2.1 i)[t]'ht = true →
    ∃ (hpr : t < (o.2.2.1 i).length) (hpb : t < (o.1 i).vals.length),
      qs ≤ ((o.2.2.1 i)[t]'hpr).card ∧
      ∀ v ∈ (o.2.2.1 i)[t]'hpr,
        ∃ (j : Fin (mem acc)) (tj : Nat) (htj : tj < (al j).length),
          v = (al j)[tj]'htj ∧
          ∃ hm : tj < (o.2.2.2 j).vals.length,
            (o.2.2.2 j).vals[tj]'hm = some ((o.1 i).vals[t]'hpb)
  /-- Distinct providers (guarded). -/
  providers : variant = .guarded → 1 ≤ qs →
    ∀ (i : Fin (mem prop)) {t : Nat}
    (ht : t < (o.2.1 i).length), (o.2.1 i)[t]'ht = true →
    ∃ (hpr : t < (o.2.2.1 i).length) (hpb : t < (o.1 i).vals.length)
      (S : List (Fin (mem acc))), S.Nodup ∧ qs ≤ S.length ∧
      ∀ j ∈ S, ∃ v ∈ (o.2.2.1 i)[t]'hpr,
        ∃ (tj : Nat) (htj : tj < (al j).length),
          v = (al j)[tj]'htj ∧
          ∃ hm : tj < (o.2.2.2 j).vals.length,
            (o.2.2.2 j).vals[tj]'hm = some ((o.1 i).vals[t]'hpb)

set_option maxHeartbeats 3200000 in
set_option maxRecDepth 65536 in
/-- **paxos.rs:253–346 `leader_election`**: one Rust fn, one Lean def.
The `fix` block is the Rust `forward_ref` plumbing (paxos.rs:253–270,
the three cycles: fail feedback ⊂ leader gossip ⊂ the is-leader flag
knot); the election body — paxos.rs:271–345 — is the single-source
chain inside it, its per-pass ghost facts stated at their wires, and
`complete` closes the cycles where Rust calls `complete_cycle`.
`stable`'s flag-prefix fact (`hknot`) is discharged by Kleene ascent
through the generated body constant (`leader_election.body`) at the
`MonoRel` instance. Binary Flo monotonicity between two runs is NOT
a face: it is the generated diagonal mono corollaries
`leader_election_mono₁…₄`, consumed at the generated
`paxos_core.a_log.stages` chain (`PaxosCore.lean`). Output tuple: (`p_ballot`,
`p_is_leader`, `p_accepted_values`, `a_max_ballot`). -/
hydro [p1bPairDecEq] def leader_election (H : HydroSem L mem)
    (variant : PaxosVariant) (prop acc : L)
    (quorum_size num_participants : Nat)
    (dec : LEDec H (mem prop) (mem acc) P)
    (sched : LESched H (mem prop) (mem acc) P)
    (p_received_p2b_ballots :
      H.Stream prop (Ballot (mem prop)) .noOrder .exactlyOnce)
    (a_log : H.TickSingleton acc (ALog P (mem prop)) .unbounded) :
    (H.TickSingleton prop (Ballot (mem prop)) (.monotonic Ballot.numVO)
      × H.TickSingleton prop Bool .unbounded
      × H.TickStream prop (ALog P (mem prop)) .noOrder .exactlyOnce
      × H.TickSingleton acc (Option (Ballot (mem prop)))
          (.monotonic Ballot.obtVO))
  ensures out => LEEnsures variant prop acc quorum_size a_log out :=
  -- paxos.rs:253–270: the three forward_ref cycles, closed mutually
  fix (p1b_fail : H.Stream prop (Ballot (mem prop)) .noOrder
        .exactlyOnce)
      (i_am_leader : H.Stream prop (Ballot (mem prop)) .noOrder
        .atLeastOnce)
      (p_is_leader : H.TickSingleton prop Bool .unbounded)
      via (dec.fuelFail, dec.fuelIAL, dec.fuelLead) :=
    -- p1b_fail.merge_unordered(p2b_ballots).merge_unordered(i_am_leader)
    --   .max().into_singleton().snapshot(tick, nondet_leader)
    let received := H.union
      (H.weaken_retries (H.union p1b_fail p_received_p2b_ballots))
      i_am_leader
    let receivedFold := H.fold_monotone Ballot.obtVO Ballot.maxFold none
      (by exact ⟨fun s x y => Ballot.maxFold_comm s x y,
        fun s x => Ballot.maxFold_idem s x⟩)
      (fun s x => Ballot.obtLE_maxFold s x)
      received
    let p_received_max_ballot := H.snapshot receivedFold
      dec.receivedMax
    -- p_ballot_calc(p_received_max_ballot)
    let bcS := p_ballot_calc H prop
      (H.forgetBound p_received_max_ballot)
    let bc := bcS.val
    -- p_leader_heartbeat(p_is_leader→, p_ballot, …)
    let hbS := p_leader_heartbeat H prop p_is_leader
      (H.forgetBound bc.1) dec.hb sched.hb
    let hb := hbS.val
    -- bound BEFORE the construct so the loop obligation can cite
    -- them (`bt` does not exist yet — the zip spelling is
    -- definitionally its value); both also serve the later legs
    ghost have hbc := bcS.property rfl
    ghost have hzip_at : ∀ (r : Fin (mem prop)) {u : Nat}
        (hlz : u < (Trace.zip ((bc.1 r).vals) (hb.2 r)).length),
        ∃ (hu1 : u < ((bc.1 r).vals).length)
          (hu2 : u < (hb.2 r).length),
          (Trace.zip ((bc.1 r).vals) (hb.2 r))[u]'hlz
            = (((bc.1 r).vals)[u]'hu1, (hb.2 r)[u]'hu2) := by
      intro r u hlz
      have hu1 : u < ((bc.1 r).vals).length := by
        have := hlz
        simp only [Trace.zip, List.length_zip] at this
        omega
      have hu2 : u < (hb.2 r).length := by
        have := hlz
        simp only [Trace.zip, List.length_zip] at this
        omega
      refine ⟨hu1, hu2, ?_⟩
      simp only [Trace.zip]
      exact List.getElem_zip ..
    -- p_to_acceptors_p1a = p_ballot.filter_if(p_trigger_election)
    --   .all_ticks().broadcast(acceptors, …).values()
    -- GUARDED (B1 fix): send each ballot's P1a once — the dedup-last
    -- register is a `use::state` `tick` block; its loop invariant
    -- (`p1aInvariant`, normal form) carries the source and send-once
    -- facts the proof legs consume at the closed knot
    tick (state lastSent : Option (Ballot (mem prop)) := none)
        (input bt := H.zipTick (H.forgetBound bc.1) hb.2)
        (invariant (p1a_out lastSent bt) =>
          p1aInvariant variant.sendOnce p1a_out lastSent bt) :=
      rebind (lastSent := (p1aSendStep variant.sendOnce lastSent bt).1)
      emit (p1a_out := (p1aSendStep variant.sendOnce lastSent bt).2)
      prove init := fun _i =>
          ⟨by simp, fun u hu => absurd hu (by simp), fun _ => by simp⟩,
        tick := fun i _n hn _out _st hlen ih =>
          -- the input is owned and num-ascending (wire facts — plain
          -- `have`s: the leg sees the streams and the ghosts above)
          have hown : ∀ x ∈ Trace.zip ((bc.1 i).vals) (hb.2 i),
              ∀ y ∈ Trace.zip ((bc.1 i).vals) (hb.2 i),
              (x : Ballot (mem prop) × Bool).1.proposerId
                = (y : Ballot (mem prop) × Bool).1.proposerId := by
            intro x hx y hy
            have hx' : x ∈ List.zip ((bc.1 i).vals) (hb.2 i) := hx
            have hy' : y ∈ List.zip ((bc.1 i).vals) (hb.2 i) := hy
            rw [hbc.own i _ (List.of_mem_zip hx').1,
              hbc.own i _ (List.of_mem_zip hy').1]
          have hsort : List.Pairwise
              (fun (a b : Ballot (mem prop) × Bool) =>
                a.1.num ≤ b.1.num)
              (Trace.zip ((bc.1 i).vals) (hb.2 i)) := by
            rw [List.pairwise_iff_getElem]
            intro u u' hu hu' hlt
            obtain ⟨hu1, hu2, he1⟩ := hzip_at i hu
            obtain ⟨hu1', hu2', he2⟩ := hzip_at i hu'
            rw [he1, he2]
            exact (bc.1 i).ascending (Nat.le_of_lt hlt) hu1'
          p1aInvariant_tick variant.sendOnce hown hsort hn hlen ih;
    let p_to_acceptors_p1a := H.values (H.broadcast_closed sched.p1aCh
      (H.allTicks (H.emitBatches p1a_out)))
    -- acceptor_p1(p1a.batch(acceptor_tick, nondet), a_log)
    let ap1S := acceptor_p1 H acc prop
      (H.batch p_to_acceptors_p1a dec.p1aBatch) a_log sched.ap1
    let ap1 := ap1S.val
    -- p_p1b(a_to_proposers_p1b, p_ballot, p_has_largest_ballot, …)
    let ppS := p_p1b H prop ap1.2 (H.forgetBound bc.1) bc.2 quorum_size
      num_participants dec.p1b sched.p1b
    let pp := ppS.val
    -- ============ ghost layer: the pass contract, decomposed ============
    -- the sub-module contracts, at the body's own wires
    ghost have hhb := hbS.property rfl
    ghost have hap1 := ap1S.property rfl
    ghost obtain ⟨okPool, hpp⟩ := ppS.property rfl
    -- the ballot leg, in the contract's own spelling (`forgetBound`)
    ghost have hpbv : ∀ (i : Fin (mem prop)),
        (Values L mem).forgetBound bc.1 i = (bc.1 i).vals :=
      fun i => rfl
    -- the flag/accepted faces at the pure layer (previously re-derived
    -- inside every contract leg)
    ghost have hflag : ∀ (i : Fin (mem prop)),
        pp.1 i = pP1bFlags (mem prop) quorum_size (okPool i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i) :=
      fun i => hpp.flags_eq i
    ghost have hface : ∀ (i : Fin (mem prop)),
        pp.2.1 i = List.map (fun l => Multiset.ofList l)
          ((((pP1bViews (mem prop) quorum_size (okPool i)
              (dec.p1b.order i) (dec.p1b.snap i)).zip
            ((bc.1 i).vals)).map
              (fun vb => pP1bQuorum quorum_size vb.1 vb.2)).map
            (fun ql => ql.getD [])) :=
      fun i => hpp.accepted_eq i
    -- index transport across the flag face (the `getElem` casts,
    -- previously six per-leg copies)
    ghost have hflag_lt : ∀ (i : Fin (mem prop)) {t : Nat},
        t < (pp.1 i).length →
        t < (pP1bFlags (mem prop) quorum_size (okPool i) ((bc.1 i).vals)
          (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i)).length := by
      intro i t ht
      rw [← hflag i]
      exact ht
    ghost have hflag_true : ∀ (i : Fin (mem prop)) {t : Nat}
        (ht : t < (pp.1 i).length), (pp.1 i)[t]'ht = true →
        (pP1bFlags (mem prop) quorum_size (okPool i) ((bc.1 i).vals)
          (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i))[t]'(hflag_lt i ht) = true := by
      intro i t ht h
      rw [← List.getElem_of_eq (hflag i) ht]
      exact h
    -- index transport across the accepted face
    ghost have hacc_len : ∀ (i : Fin (mem prop)) {t : Nat},
        t < (pP1bViews (mem prop) quorum_size (okPool i)
          (dec.p1b.order i) (dec.p1b.snap i)).length →
        t < ((bc.1 i).vals).length →
        t < (pp.2.1 i).length := by
      intro i t hv hb
      rw [hface i]
      simp only [List.length_map, Trace.zip, List.length_zip]
      omega
    -- the accepted-batch face at one tick: the gated quorum bucket
    ghost have hbatch : ∀ (i : Fin (mem prop)) {t : Nat}
        (hpr : t < (pp.2.1 i).length)
        (hv : t < (pP1bViews (mem prop) quorum_size (okPool i)
          (dec.p1b.order i) (dec.p1b.snap i)).length)
        (hb : t < ((bc.1 i).vals).length),
        (pp.2.1 i)[t]'hpr = Multiset.ofList
          ((pP1bQuorum quorum_size
            ((pP1bViews (mem prop) quorum_size (okPool i)
              (dec.p1b.order i) (dec.p1b.snap i))[t]'hv)
            ((bc.1 i).vals[t]'hb)).getD []) := by
      intro i t hpr hv hb
      rw [List.getElem_of_eq (hface i) hpr]
      rw [List.getElem_map, List.getElem_map, List.getElem_map]
      simp only [Trace.zip]
      rw [List.getElem_zip]
    -- the merged P1a pool at an acceptor: per-proposer released streams
    ghost have hp1a : ∀ (j : Fin (mem acc)),
        p_to_acceptors_p1a j
          = ((List.finRange (mem prop)).map
            (fun r => (↑((p1a_out r).flatten)
              : Multiset (Ballot (mem prop))))).sum := fun j => rfl
    -- a released ballot's source tick (any variant): the invariant's
    -- source clause, flattened at the wires
    ghost have hsrc : ∀ (r : Fin (mem prop)) {b : Ballot (mem prop)},
        b ∈ (p1a_out r).flatten →
        ∃ (u : Nat)
          (hub : u < (Trace.zip ((bc.1 r).vals) (hb.2 r)).length),
          b = ((Trace.zip ((bc.1 r).vals) (hb.2 r))[u]'hub).1
          ∧ ((Trace.zip ((bc.1 r).vals) (hb.2 r))[u]'hub).2 = true := by
      intro r b hbin
      obtain ⟨sub, hsub, hbs⟩ := List.mem_flatten.mp hbin
      obtain ⟨u, hu, rfl⟩ := List.mem_iff_getElem.mp hsub
      rcases (hp1a_out_inv r).2.1 u hu with hnil | ⟨hub, heq, htrig⟩
      · rw [hnil] at hbs
        exact absurd hbs (List.not_mem_nil)
      · rw [heq] at hbs
        exact ⟨u, hub, (List.mem_singleton.mp hbs), htrig⟩
    -- send-once (B1): the guarded released stream is duplicate-free
    -- (the invariant's dedup clause, closed at the wires)
    ghost have hnodup : variant = .guarded → ∀ (r : Fin (mem prop)),
        ((p1a_out r).flatten).Nodup := by
      intro hvar r
      have h := (hp1a_out_inv r).2.2 (by rw [hvar]; rfl)
      split at h
      · rw [h]
        exact List.nodup_nil
      · exact h.2.2
    -- ==================== the wires, and the contract ====================
    -- p1b_fail_complete.complete(fail_ballots);
    -- p_to_proposers_i_am_leader_complete_cycle.complete(…);
    -- p_is_leader_complete_cycle.complete(p_is_leader.clone())
    complete (pp.2.2, hb.1, pp.1)
    -- ===== the closed knots: the Kleene flag-prefix fact =====
    -- ghost aliases: two knots and the generated pass body, open in
    -- their wire arguments
    ghost let fails := _root_.HydroV2.leader_election.p1b_fail
      (P := P) (Values L mem) variant prop acc quorum_size
      num_participants dec sched p_received_p2b_ballots a_log
    ghost let iam := _root_.HydroV2.leader_election.i_am_leader
      (P := P) (Values L mem) variant prop acc quorum_size
      num_participants dec sched p_received_p2b_ballots a_log
    ghost let pass := fun fl ia f =>
      _root_.HydroV2.leader_election.body (P := P) (Values L mem)
        variant prop acc quorum_size num_participants dec sched
        p_received_p2b_ballots a_log fl ia f
    -- one Kleene step of the flag body preserves prefixes (the inner
    -- fail/gossip fixpoints coupled by `iterate_mono_param` through
    -- the pass's `MonoRel` instantiation)
    ghost have hstep : ∀ {a b : TickV (mem prop) Bool .unbounded},
        (∀ i, a i <+: b i) →
        ∀ i, (pass (fails (iam a) a) (iam a) a).2.2.1 i
          <+: (pass (fails (iam b) b) (iam b) b).2.2.1 i := by
      intro a b hab
      -- the fail leg, coupled through its own fixpoint
      have hfl : ∀ (x y : Fin (mem prop)
          → RetryPool (Ballot (mem prop))),
          (∀ i, RetryPool.le (x i) (y i)) →
          ∀ i, fails x a i ≤ fails y b i := by
        intro x y hxy
        exact iterate_mono_param
          (R := fun (u v : Fin (mem prop)
            → Multiset (Ballot (mem prop))) => ∀ i, u i ≤ v i)
          (F := fun fl => (pass fl x a).1)
          (F' := fun fl => (pass fl y b).1)
          (fun {u v} huv => (_root_.HydroV2.leader_election.body
            (P := P) (MonoRel L mem) variant prop acc quorum_size
            num_participants (ledecMR dec) leschedMR
            ⟨(p_received_p2b_ballots, p_received_p2b_ballots),
              fun _ => le_refl _⟩
            ⟨(a_log, a_log), fun _ => List.prefix_refl _⟩
            ⟨(u, v), huv⟩ ⟨(x, y), hxy⟩
            ⟨(a, b), hab⟩).1.property)
          (x := fun _ => 0) (x' := fun _ => 0) (fun _ => le_refl _)
          dec.fuelFail
      -- the gossip leg, coupled through its own fixpoint
      have hia : ∀ i, RetryPool.le (iam a i) (iam b i) :=
        iterate_mono_param
          (R := fun (x y : Fin (mem prop)
            → RetryPool (Ballot (mem prop))) =>
            ∀ i, RetryPool.le (x i) (y i))
          (F := fun ia => (pass (fails ia a) ia a).2.1)
          (F' := fun ia => (pass (fails ia b) ia b).2.1)
          (fun {x y} hxy => (_root_.HydroV2.leader_election.body
            (P := P) (MonoRel L mem) variant prop acc quorum_size
            num_participants (ledecMR dec) leschedMR
            ⟨(p_received_p2b_ballots, p_received_p2b_ballots),
              fun _ => le_refl _⟩
            ⟨(a_log, a_log), fun _ => List.prefix_refl _⟩
            ⟨(fails x a, fails y b), hfl x y hxy⟩
            ⟨(x, y), hxy⟩
            ⟨(a, b), hab⟩).2.1.property)
          (x := fun _ => RetryPool.mk 0) (x' := fun _ => RetryPool.mk 0)
          (fun _ => RetryPool.le_refl _)
          dec.fuelIAL
      exact (_root_.HydroV2.leader_election.body
        (P := P) (MonoRel L mem) variant prop acc quorum_size
        num_participants (ledecMR dec) leschedMR
        ⟨(p_received_p2b_ballots, p_received_p2b_ballots),
          fun _ => le_refl _⟩
        ⟨(a_log, a_log), fun _ => List.prefix_refl _⟩
        ⟨(fails (iam a) a, fails (iam b) b),
          hfl (iam a) (iam b) hia⟩
        ⟨(iam a, iam b), hia⟩
        ⟨(a, b), hab⟩).2.2.1.property
    -- the knot: the cycle-input flag is a prefix of the output flag
    ghost have hknot : ∀ i, p_is_leader i <+: pp.1 i :=
      iterate_le_succ
        (R := fun (a b : TickV (mem prop) Bool .unbounded) =>
          ∀ i, a i <+: b i)
        (F := fun f => (pass (fails (iam f) f) (iam f) f).2.2.1)
        (x := fun _ => [])
        (fun _ => List.nil_prefix)
        (fun {a b} hab => hstep hab)
        dec.fuelLead
    -- (p_ballot, p_is_leader, p_accepted_values, a_max_ballot)
    (bc.1, pp.1, pp.2.1, ap1.1)
    prove
      own := fun i b hb => hbc.own i b hb,
      lead_ne := by
        -- the leader gate answers with a full bucket
        intro hq1 i t hpl hpr htrue
        obtain ⟨hv, hb, hl, qlogs, hq, hlen, -⟩ :=
          hpp.leader_gate i hpl htrue
        intro hzero
        have hq2 : pP1bQuorum quorum_size
            ((pP1bViews (mem prop) quorum_size (okPool i)
              (dec.p1b.order i) (dec.p1b.snap i))[t]'hv)
            ((bc.1 i).vals[t]'hb) = some qlogs := hq
        have hbeq := hbatch i hpr hv hb
        rw [hq2] at hbeq
        rw [hbeq] at hzero
        have hnil : qlogs = [] := by
          simpa using hzero
        rw [hnil] at hlen
        simp at hlen
        omega,
      stable := by
        -- the fabricated-reign regress at the run
        intro hq1 i t ht1 hb1 h1 h0
        -- the solicitation chain: an Ok reply pins a trigger-true
        -- follower tick of the sender's own run
        have hsol : ∀ m ∈ ap1.2 i,
            (∃ v, (m : P1b P (mem prop)).res = .ok v) →
            ∃ (u : Nat)
              (hu : u < (pP1bFlags (mem prop) quorum_size (okPool i)
                ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
                (dec.p1b.snap i)).length)
              (hbu : u < ((bc.1 i).vals).length),
              (bc.1 i).vals[u]'hbu = m.ballot
              ∧ (pP1bFlags (mem prop) quorum_size (okPool i)
                  ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
                  (dec.p1b.snap i))[u]'hu
                = false := by
          intro m hm hok
          -- the reply echoes a consumed P1a at the sending acceptor
          obtain ⟨j, tj, hbj, hmj, hlj, hball, hroute, -⟩ :=
            hap1.reply_src i m hm
          -- consumed batch → merged pool
          have hball2 : m.ballot ∈ (batchCuts (p_to_acceptors_p1a j) 0
              (dec.p1aBatch j))[tj]'hbj := hball
          have hin_pool : m.ballot ∈ p_to_acceptors_p1a j := by
            have hmem_sum : m.ballot ∈ (batchCuts (p_to_acceptors_p1a j)
                0 (dec.p1aBatch j)).sum :=
              mem_list_sum.mpr ⟨_, List.getElem_mem hbj, hball2⟩
            have hle := batchCuts_sum_le (pool := p_to_acceptors_p1a j)
              (d := dec.p1aBatch j) (consumed := 0) (Multiset.zero_le _)
            rw [Multiset.zero_add] at hle
            exact Multiset.mem_of_le hle hmem_sum
          -- pool → some sender's released stream
          rw [hp1a j] at hin_pool
          obtain ⟨ms, hms, hbin⟩ := mem_list_sum.mp hin_pool
          obtain ⟨r, -, rfl⟩ := List.mem_map.mp hms
          have hbin' : m.ballot ∈ (p1a_out r).flatten :=
            Multiset.mem_coe.mp hbin
          obtain ⟨u, hlz, hbu_eq, htrig⟩ := hsrc r hbin'
          -- zip projections at the release tick
          obtain ⟨hu1, hu2, hzel⟩ := hzip_at r hlz
          rw [hzel] at hbu_eq htrig
          -- ownership routes the release to the requester itself
          have hown_r : m.ballot.proposerId = r := by
            rw [hbu_eq]
            exact hbc.own r _ (List.getElem_mem hu1)
          have hri : r = i := by
            apply Fin.ext
            rw [← hroute, hown_r]
          subst hri
          -- the trigger gate reads a false flag off the cycle wire
          obtain ⟨hf, hff⟩ := hhb.trigger_gate r hu2 htrig
          -- the knot: the cycle input is a prefix of the output flags
          have hpre : p_is_leader r <+: pp.1 r := hknot r
          have hu_pp : u < (pp.1 r).length :=
            Nat.lt_of_lt_of_le hf hpre.length_le
          have hval := List.IsPrefix.getElem hpre hf
          rw [hff] at hval
          refine ⟨u, hflag_lt r hu_pp, hu1, hbu_eq.symm, ?_⟩
          rw [← List.getElem_of_eq (hflag r) hu_pp]
          exact hval.symm
        -- discharge `p_p1b`'s requirements and run the regress
        have req : PP1bRequires (mem prop) quorum_size (ap1.2 i)
            (okPool i)
            ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i)
            i :=
          { ok_pool_le := hpp.ok_pool_le i
            has_largest := hbc.hasLargest_true i
            ballot_own := hbc.own i
            ballot_mono := fun h ht' => (bc.1 i).ascending h ht'
            solicited_at_follower := hsol }
        obtain ⟨hb1', heq⟩ := pP1b_ballot_stable quorum_size hq1
          (ap1.2 i) ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i) i req
          (hflag_lt i ht1) (hflag_true i ht1 h1)
          (hflag_true i (Nat.lt_of_succ_lt ht1) h0)
        exact heq,
      pinned := by
        -- frozen quorum buckets across same-ballot leader ticks
        intro hq1 i t t' hpl hpl' hpr hpr' hpb hpb' hl hl' hnum
        have hbeq : (bc.1 i).vals[t]'hpb = (bc.1 i).vals[t']'hpb' :=
          Ballot.eq_of_num_owner hnum (by
            rw [hbc.own i _ (List.getElem_mem hpb),
              hbc.own i _ (List.getElem_mem hpb')])
        rcases Nat.le_total t t' with hle | hle
        · obtain ⟨hv, hv', qlogs, hqt, hqt'⟩ := pP1b_quorum_pinned
            quorum_size hq1 (ap1.2 i) ((bc.1 i).vals) (bc.2 i)
            (dec.p1b.order i) (dec.p1b.snap i) hle
            (hflag_lt i hpl) (hflag_lt i hpl')
            (hflag_true i hpl hl) (hflag_true i hpl' hl')
            (hb := hpb) (hb' := hpb') hbeq
          have h1 := hbatch i hpr hv hpb
          have h2 := hbatch i hpr' hv' hpb'
          rw [hqt] at h1
          rw [hqt'] at h2
          rw [h1, h2]
        · obtain ⟨hv', hv, qlogs, hqt', hqt⟩ := pP1b_quorum_pinned
            quorum_size hq1 (ap1.2 i) ((bc.1 i).vals) (bc.2 i)
            (dec.p1b.order i) (dec.p1b.snap i) hle
            (hflag_lt i hpl') (hflag_lt i hpl)
            (hflag_true i hpl' hl') (hflag_true i hpl hl)
            (hb := hpb') (hb' := hpb) hbeq.symm
          have h1 := hbatch i hpr hv hpb
          have h2 := hbatch i hpr' hv' hpb'
          rw [hqt] at h1
          rw [hqt'] at h2
          rw [h1, h2],
      view_promise := by
        -- the leader's view opens to acceptor-tick promises
        intro i t ht htrue
        obtain ⟨hv, hb, hl, qlogs, hq, hlen, -⟩ :=
          hpp.leader_gate i ht htrue
        have hq2 : pP1bQuorum quorum_size
            ((pP1bViews (mem prop) quorum_size (okPool i)
              (dec.p1b.order i) (dec.p1b.snap i))[t]'hv)
            ((bc.1 i).vals[t]'hb) = some qlogs := hq
        have hpr : t < (pp.2.1 i).length := hacc_len i hv hb
        refine ⟨hpr, hb, ?_, ?_⟩
        · -- fullness
          have hb1 := hbatch i hpr hv hb
          rw [hq2] at hb1
          rw [hb1]
          simpa using hlen
        · -- each payload opens to an acceptor promise tick
          intro v hvmem
          obtain ⟨hb0, m, hm, hres, hball⟩ :=
            hpp.accepted_src i hpr v hvmem
          have hball2 : m.ballot = (bc.1 i).vals[t]'hb := hball
          obtain ⟨j, tj, hbj, hmj, hlj, -, -, hshape⟩ :=
            hap1.reply_src i m hm
          by_cases hcond : some m.ballot = (ap1.1 j).vals[tj]'hmj
          · rw [if_pos hcond] at hshape
            rw [hres] at hshape
            have hveq : v = (a_log j)[tj]'hlj := by
              injection hshape
            refine ⟨j, tj, hlj, hveq, hmj, ?_⟩
            rw [← hcond, hball2]
          · rw [if_neg hcond] at hshape
            rw [hres] at hshape
            simp at hshape,
      providers := by
        -- the guarded send-once fan-in yields distinct acceptors
        intro hvar hq1 i t ht htrue
        subst hvar
        -- the leader bucket at the pure layer
        obtain ⟨hv, hbt, hl, qlogs, hq, hmem_bucket, hlen_eq, -⟩ :=
          pP1b_leader_bucket quorum_size hq1 (ap1.2 i) ((bc.1 i).vals)
            (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i)
            (hflag_lt i ht) (hflag_true i ht htrue)
        have hpr : t < (pp.2.1 i).length := hacc_len i hv hbt
        have hbv := hbatch i hpr hv hbt
        rw [hq] at hbv
        -- b, the tick's own ballot; it is owned by `i`
        have hbown : ((bc.1 i).vals[t]'hbt).proposerId = i :=
          hbc.own i _ (List.getElem_mem hbt)
        -- the bucket, with multiplicity, sits inside the Ok pool
        have hsub : (Multiset.ofList (qlogs.map (fun v =>
            (((bc.1 i).vals[t]'hbt, v)
              : Ballot (mem prop) × ALog P (mem prop)))))
            ≤ Multiset.filterMap p1bOkPair (ap1.2 i) :=
          pP1bViews_bucket_sub_oks quorum_size (ap1.2 i)
            (hpp.ok_pool_le i)
            (dec.p1b.order i) (dec.p1b.snap i) hv hmem_bucket
        -- the pool decomposes by sending acceptor
        have hdec : ap1.2 i = ((List.finRange (mem acc)).map
            (fun j => ap1From (fun j' => batchCuts (p_to_acceptors_p1a j')
              0 (dec.p1aBatch j')) a_log j i)).sum :=
          hap1.reply_decomp i
        -- each ballot is on the wire at most once (send-once + ownership)
        have hcount_pool : ∀ (j : Fin (mem acc)),
            (p_to_acceptors_p1a j).count ((bc.1 i).vals[t]'hbt) ≤ 1 := by
          intro j
          rw [hp1a j, count_list_sum, List.map_map]
          refine sum_map_le_single (List.nodup_finRange _) _ i ?_ ?_
          · -- other proposers never carry `i`'s ballot
            intro r _ hri
            show Multiset.count ((bc.1 i).vals[t]'hbt)
              (↑((p1a_out r).flatten)) = 0
            rw [Multiset.count_eq_zero]
            intro hbin
            obtain ⟨u, hlz, hbeq2, -⟩ :=
              hsrc r (Multiset.mem_coe.mp hbin)
            obtain ⟨hu1, hu2, hzel⟩ := hzip_at r hlz
            rw [hzel] at hbeq2
            have : ((bc.1 i).vals[t]'hbt).proposerId = r := by
              rw [hbeq2]
              exact hbc.own r _ (List.getElem_mem hu1)
            rw [hbown] at this
            exact hri this.symm
          · -- the owner releases it at most once (B1): the loop
            -- invariant's guarded-dedup clause over owned, ascending
            -- inputs
            show Multiset.count ((bc.1 i).vals[t]'hbt)
              (↑((p1a_out i).flatten)) ≤ 1
            rw [Multiset.coe_count]
            have hnd : ((p1a_out i).flatten).Nodup := hnodup rfl i
            exact List.nodup_iff_count_le_one.mp hnd _
        -- the per-sender Ok-at-b caps
        have hcaps : ∀ j ∈ List.finRange (mem acc),
            ((Multiset.filterMap p1bOkPair
              (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
                (dec.p1aBatch j')) a_log j i)).filter
              (fun x => x.1 = (bc.1 i).vals[t]'hbt)).card ≤ 1 := by
          intro j _
          have hstep1 : ((Multiset.filterMap p1bOkPair
              (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
                (dec.p1aBatch j')) a_log j i)).filter
              (fun x => x.1 = (bc.1 i).vals[t]'hbt)).card
              = (Multiset.filterMap p1bOkPair
                (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
                  (dec.p1aBatch j')) a_log j i)).countP
                (fun x => x.1 = (bc.1 i).vals[t]'hbt) :=
            (Multiset.countP_eq_card_filter _ _).symm
          rw [hstep1]
          have hstep2 := countP_filterMap_le p1bOkPair
            (fun x : Ballot (mem prop) × ALog P (mem prop) =>
              x.1 = (bc.1 i).vals[t]'hbt)
            (fun m' : P1b P (mem prop) =>
              m'.ballot = (bc.1 i).vals[t]'hbt)
            (fun a b' hfa hp => by
              unfold p1bOkPair at hfa
              cases hres : a.res with
              | ok pl =>
                rw [hres] at hfa
                injection hfa with h'
                rw [← h'] at hp
                exact hp
              | error e =>
                rw [hres] at hfa
                cases hfa)
            (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
              (dec.p1aBatch j')) a_log j i)
          refine Nat.le_trans hstep2 ?_
          have hstep3 : (ap1From (fun j' =>
              batchCuts (p_to_acceptors_p1a j')
              0 (dec.p1aBatch j')) a_log j i).countP
              (fun m' => m'.ballot = (bc.1 i).vals[t]'hbt)
              = ((ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
                (dec.p1aBatch j')) a_log j i).filter
                (fun m' => m'.ballot = (bc.1 i).vals[t]'hbt)).card :=
            Multiset.countP_eq_card_filter _ _
          rw [hstep3]
          refine Nat.le_trans (hap1.from_ballot_cap j i _) ?_
          refine Nat.le_trans ?_ (hcount_pool j)
          have hle := batchCuts_sum_le (pool := p_to_acceptors_p1a j)
            (d := dec.p1aBatch j) (consumed := 0) (Multiset.zero_le _)
          rw [Multiset.zero_add] at hle
          exact Multiset.count_le_of_le _ hle
        -- distinct representatives from the unit caps
        have hle2 : (Multiset.ofList (qlogs.map (fun v =>
            (((bc.1 i).vals[t]'hbt, v)
              : Ballot (mem prop) × ALog P (mem prop)))))
            ≤ ((List.finRange (mem acc)).map
              (fun j => (Multiset.filterMap p1bOkPair
                (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
                  (dec.p1aBatch j')) a_log j i)).filter
                (fun x => x.1 = (bc.1 i).vals[t]'hbt))).sum := by
          have hMfil : (Multiset.ofList (qlogs.map (fun v =>
              (((bc.1 i).vals[t]'hbt, v)
                : Ballot (mem prop) × ALog P (mem prop))))).filter
              (fun x => x.1 = (bc.1 i).vals[t]'hbt)
              = Multiset.ofList (qlogs.map (fun v =>
                (((bc.1 i).vals[t]'hbt, v)
                  : Ballot (mem prop) × ALog P (mem prop)))) := by
            refine Multiset.filter_eq_self.mpr ?_
            intro x hx
            obtain ⟨v, -, rfl⟩ := List.mem_map.mp (Multiset.mem_coe.mp hx)
            rfl
          have h1 : (Multiset.ofList (qlogs.map (fun v =>
              (((bc.1 i).vals[t]'hbt, v)
                : Ballot (mem prop) × ALog P (mem prop)))))
              ≤ (Multiset.filterMap p1bOkPair (ap1.2 i)).filter
                (fun x => x.1 = (bc.1 i).vals[t]'hbt) := by
            rw [← hMfil]
            exact Multiset.filter_le_filter _ hsub
          rw [hdec, filterMap_list_sum p1bOkPair, List.map_map,
            filter_list_sum _, List.map_map] at h1
          exact h1
        obtain ⟨S, hSnd, hSsub, hScard, hSrep⟩ := exists_distinct_reps
          (List.finRange (mem acc))
          (fun j => (Multiset.filterMap p1bOkPair
            (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
              (dec.p1aBatch j')) a_log j i)).filter
            (fun x => x.1 = (bc.1 i).vals[t]'hbt))
          _ (List.nodup_finRange _) hle2 hcaps
        have hMcard : Multiset.card (Multiset.ofList (qlogs.map (fun v =>
            (((bc.1 i).vals[t]'hbt, v)
              : Ballot (mem prop) × ALog P (mem prop)))))
            = quorum_size := by
          rw [Multiset.coe_card, List.length_map, hlen_eq]
        refine ⟨hpr, hbt, S, hSnd, ?_, ?_⟩
        · rw [hMcard] at hScard
          exact hScard
        · intro j hj
          obtain ⟨x, hxM, hxq⟩ := hSrep j hj
          obtain ⟨v, hvq, rfl⟩ := List.mem_map.mp (Multiset.mem_coe.mp hxM)
          have hxq' : (((bc.1 i).vals[t]'hbt, v)
              : Ballot (mem prop) × ALog P (mem prop))
              ∈ Multiset.filterMap p1bOkPair
                (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
                  (dec.p1aBatch j')) a_log j i) :=
            Multiset.mem_of_le (Multiset.filter_le _ _) hxq
          obtain ⟨m', hm', hfm⟩ := (Multiset.mem_filterMap _ _).mp hxq'
          have hmb : m'.ballot = (bc.1 i).vals[t]'hbt
              ∧ m'.res = .ok v := by
            unfold p1bOkPair at hfm
            cases hres : m'.res with
            | ok pl =>
              rw [hres] at hfm
              injection hfm with h'
              have hsnd : pl = v := congrArg Prod.snd h'
              exact ⟨congrArg Prod.fst h', by rw [hsnd]⟩
            | error e =>
              rw [hres] at hfm
              cases hfm
          obtain ⟨tj, hbj, hmj, hlj, -, -, hshape⟩ :=
            hap1.from_src j i m' hm'
          by_cases hcond : some m'.ballot = (ap1.1 j).vals[tj]'hmj
          · rw [if_pos hcond] at hshape
            rw [hmb.2] at hshape
            have hveq : v = (a_log j)[tj]'hlj := by
              injection hshape
            refine ⟨v, ?_, tj, hlj, hveq, hmj, ?_⟩
            · rw [hbv]
              exact Multiset.mem_coe.mpr hvq
            · rw [← hcond, hmb.1]
          · rw [if_neg hcond] at hshape
            rw [hmb.2] at hshape
            simp at hshape


/-! ## Executable non-vacuity: a leader gets elected

One proposer, one acceptor, quorum of 1. Nothing is received at first
(`p_ballot` stays `(0,0)`); the timer fires at the non-leader tick 0, a
P1a goes out, the acceptor promises, and the quorum elects the proposer
at tick 1. The Kleene fuel `3` pins the leader fixpoint: iteration 1
finds no trigger (no ticks realized on the flag), iteration 2 triggers
and elects, iteration 3 confirms stability. -/

private def leScenario :=
  (leader_election (Values PaxLoc (paxMem 1 1)) .guarded .prop .acc 1 1
    { receivedMax := fun _ => [{}, {}]            -- empty increments
      hb := { sample := fun _ => []                 -- no heartbeat samples
              timeout := fun _ => [true, true]      -- timer fires
              interval := fun _ => [true, true] }   -- trigger pulses
      p1aBatch := fun _ => [{Ballot.mk 0 0}, {}]    -- p1a batch cuts
      p1b := { cqwr := fun _ =>
                 [{(Ballot.mk 0 0, Except.ok (none, []))}, {}]
               order := fun _ => [(Ballot.mk 0 0, (none, []))]
               snap := fun _ => [0, 1] }            -- p1b snapshot cuts
      fuelFail := (1 : UnfoldFuel), fuelIAL := (1 : UnfoldFuel)
      fuelLead := (3 : UnfoldFuel) }
    .triv                                         -- sched-det bundle
    (fun _ => (0 : Multiset (Ballot 1)))          -- no p2b feedback
    ((fun _ => [(none, []), (none, [])]) :
      TickV 1 (ALog Nat 1) .unbounded)).val      -- a_log wire

#nondet_census leader_election (nondets := 8) (scheds := 4) (fuels := 3)

#guard leScenario.2.1 0 = [false, true]

#guard (leScenario.1 0).vals = [Ballot.mk 0 0, Ballot.mk 0 0]

#guard (leScenario.2.2.2 0).vals = [some (Ballot.mk 0 0),
  some (Ballot.mk 0 0)]


end HydroV2
