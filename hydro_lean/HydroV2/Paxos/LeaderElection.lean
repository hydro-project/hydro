import HydroV2.Paxos.PBallotCalc
import HydroV2.Paxos.PLeaderHeartbeat
import HydroV2.Paxos.AcceptorP1
import HydroV2.Paxos.PP1b

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
structure LEDec (nP nA : Nat) (P : Type) [DecidableEq P] where
  /-- `p_received_max_ballot.snapshot(&proposer_tick, nondet_leader)`:
  arrival cuts of the unordered received-ballot pool. -/
  receivedMax : SnapshotCuts nP (Ballot nP) .noOrder
  /-- `p_leader_heartbeat`'s timing decisions. -/
  hb : PLHDec nP
  /-- `p_to_acceptors_p1a.batch(&acceptor_tick, nondet!(…))`
  (`acceptor_p1`'s consumption): consumed P1a increments. -/
  p1aBatch : BatchCuts nA (Ballot nP)
  /-- `p_p1b`'s quorum-collection decisions. -/
  p1b : PP1bDec nP P
  /-- `p1b_fail` cycle depth (`forward_ref`). -/
  fuelFail : UnfoldFuel
  /-- `i_am_leader` cycle depth (`forward_ref`). -/
  fuelIAL : UnfoldFuel
  /-- `p_is_leader` cycle depth (`forward_ref`). -/
  fuelLead : UnfoldFuel

/-- The wires `leader_election`'s body produces. -/
structure LEWires {L : Type} {mem : L → Nat} (H : HydroSem L mem)
    (prop acc : L) (P : Type) [DecidableEq P] where
  p_ballot : H.TickSingleton prop (Ballot (mem prop))
    (.monotonic Ballot.numVO)
  p_has_largest_ballot : H.TickSingleton prop Bool .unbounded
  i_am_leader : H.Stream prop (Ballot (mem prop)) .noOrder .atLeastOnce
  a_max_ballot : H.TickSingleton acc (Option (Ballot (mem prop)))
    (.monotonic Ballot.obtVO)
  p_is_leader : H.TickSingleton prop Bool .unbounded
  p_accepted_values :
    H.TickStream prop (ALog P (mem prop)) .noOrder .exactlyOnce
  fail_ballots : H.Stream prop (Ballot (mem prop)) .noOrder .exactlyOnce

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

/-- **Emission source, per tick** (any variant): a ballot emitted at
tick `u` is the tick's ballot leg, and the tick's trigger is up. -/
theorem p1aSendStep_mem_tick {nP : Nat} (so : Bool) :
    ∀ (l : List (Ballot nP × Bool)) (s : Option (Ballot nP)) {u : Nat}
      (hu : u < (scanAcrossTicksTrace (p1aSendStep so) s l).length) {b : Ballot nP},
      b ∈ (scanAcrossTicksTrace (p1aSendStep so) s l)[u]'hu →
      ∃ hl : u < l.length, b = (l[u]'hl).1 ∧ (l[u]'hl).2 = true
  | [], _, _, hu, _ => by cases hu
  | x :: xs, s, u, hu, b => by
    cases u with
    | zero =>
      intro hb
      refine ⟨Nat.zero_lt_succ _, ?_⟩
      have hb0 : b ∈ (p1aSendStep so s x).2 := hb
      unfold p1aSendStep at hb0
      by_cases hc : (x.2 && !(so && s = some x.1)) = true
      · rw [if_pos hc] at hb0
        obtain rfl := List.mem_singleton.mp hb0
        have hc' := hc
        simp only [Bool.and_eq_true] at hc'
        exact ⟨rfl, hc'.1⟩
      · rw [if_neg hc] at hb0
        cases hb0
    | succ n =>
      intro hb
      have hn : n < (scanAcrossTicksTrace (p1aSendStep so)
          (p1aSendStep so s x).1 xs).length := by
        have := hu
        simp only [scanAcrossTicksTrace, List.length_cons] at this
        omega
      obtain ⟨hl, h1, h2⟩ := p1aSendStep_mem_tick so xs
        (p1aSendStep so s x).1 hn (show b ∈ _ from hb)
      exact ⟨Nat.succ_lt_succ hl, h1, h2⟩

/-- **Emission source, whole-run form**: a ballot on the released stream
pins a trigger-true tick carrying it on the ballot leg. -/
theorem p1aSendStep_flatten_mem {nP : Nat} (so : Bool)
    (l : List (Ballot nP × Bool)) (s : Option (Ballot nP)) {b : Ballot nP}
    (hb : b ∈ (scanAcrossTicksTrace (p1aSendStep so) s l).flatten) :
    ∃ (u : Nat) (hl : u < l.length),
      b = (l[u]'hl).1 ∧ (l[u]'hl).2 = true := by
  obtain ⟨em, hem, hbm⟩ := List.mem_flatten.mp hb
  obtain ⟨u, hu, rfl⟩ := List.mem_iff_getElem.mp hem
  obtain ⟨hl, h1, h2⟩ := p1aSendStep_mem_tick so l s hu hbm
  exact ⟨u, hl, h1, h2⟩

/-- **Send-once (guarded)**: over an owned, `num`-ascending ballot leg,
the released stream ascends *strictly* in `num` — the `lastSent` state
never re-fires a standing ballot, and ownership makes equal `num`s equal
ballots. The state invariant rides the recursion: whatever `lastSent`
holds is owned and at most every remaining `num`. -/
theorem p1aSendStep_once {nP : Nat} (me : Fin nP) :
    ∀ (l : List (Ballot nP × Bool)) (last : Option (Ballot nP)),
    (∀ x ∈ l, (x : Ballot nP × Bool).1.proposerId = me) →
    List.Pairwise (fun a b => a.num ≤ b.num) (l.map Prod.fst) →
    (∀ a, last = some a →
      a.proposerId = me ∧ ∀ x ∈ l, a.num ≤ x.1.num) →
    List.Pairwise (fun a b => a.num < b.num)
      ((scanAcrossTicksTrace (p1aSendStep true) last l).flatten)
    ∧ (∀ a, last = some a →
        ∀ y ∈ (scanAcrossTicksTrace (p1aSendStep true) last l).flatten,
          a.num < y.num)
  | [], _, _, _, _ => ⟨List.Pairwise.nil, fun _ _ _ h => nomatch h⟩
  | x :: xs, last, hown, hsort, hlast => by
    have hxs_own : ∀ y ∈ xs, (y : Ballot nP × Bool).1.proposerId = me :=
      fun y hy => hown y (List.mem_cons_of_mem _ hy)
    have hxs_sort : List.Pairwise (fun a b => a.num ≤ b.num)
        (xs.map Prod.fst) := (by
      have := hsort
      rw [List.map_cons] at this
      exact this.of_cons)
    have hhead : ∀ y ∈ xs, x.1.num ≤ y.1.num := by
      intro y hy
      have := hsort
      rw [List.map_cons, List.pairwise_cons] at this
      exact this.1 y.1 (List.mem_map_of_mem hy)
    have hexp : (scanAcrossTicksTrace (p1aSendStep true) last (x :: xs)).flatten
        = (p1aSendStep true last x).2
          ++ (scanAcrossTicksTrace (p1aSendStep true)
            (p1aSendStep true last x).1 xs).flatten := rfl
    by_cases hc : (x.2 && !(true && last = some x.1)) = true
    · -- fire: emit `x.1`, state becomes `some x.1`
      have hstep : p1aSendStep true last x = (some x.1, [x.1]) := by
        unfold p1aSendStep
        rw [if_pos hc]
      rw [hstep] at hexp
      have hrec := p1aSendStep_once me xs (some x.1) hxs_own hxs_sort
        (fun a ha => by
          cases Option.some.inj ha
          exact ⟨hown x (List.mem_cons_self ..), hhead⟩)
      have hlx : last ≠ some x.1 := by
        intro heq
        have hc' := hc
        simp only [Bool.and_eq_true] at hc'
        have := hc'.2
        rw [heq] at this
        simp at this
      constructor
      · rw [hexp]
        show List.Pairwise _ (x.1 :: _)
        refine List.pairwise_cons.mpr ⟨?_, hrec.1⟩
        intro y hy
        exact hrec.2 x.1 rfl y hy
      · intro a ha y hy
        rw [hexp] at hy
        obtain ⟨haown, hale⟩ := hlast a ha
        rcases List.mem_cons.mp hy with rfl | hy'
        · -- `y = x.1`: `a ≠ x.1` (no re-fire) with equal owner forces
          -- a strict `num` step
          have hne : a ≠ x.1 := fun heq => hlx (by rw [ha, heq])
          have hnum : a.num ≠ x.1.num := fun heq =>
            hne (Ballot.eq_of_num_owner heq (by
              rw [haown, hown x (List.mem_cons_self ..)]))
          have := hale x (List.mem_cons_self ..)
          omega
        · have hax : a.num ≤ x.1.num := hale x (List.mem_cons_self ..)
          exact Nat.lt_of_le_of_lt hax (hrec.2 x.1 rfl y hy')
    · -- hold: nothing emitted, state unchanged
      have hstep : p1aSendStep true last x = (last, []) := by
        unfold p1aSendStep
        rw [if_neg hc]
      rw [hstep] at hexp
      rw [List.nil_append] at hexp
      have hrec := p1aSendStep_once me xs last hxs_own hxs_sort
        (fun a ha => ⟨(hlast a ha).1,
          fun y hy => (hlast a ha).2 y (List.mem_cons_of_mem _ hy)⟩)
      constructor
      · rw [hexp]
        exact hrec.1
      · intro a ha y hy
        rw [hexp] at hy
        exact hrec.2 a ha y hy

/-- Send-once, consumable form: the guarded released stream is
duplicate-free. -/
theorem p1aSendStep_once_nodup {nP : Nat} (me : Fin nP)
    (l : List (Ballot nP × Bool))
    (hown : ∀ x ∈ l, (x : Ballot nP × Bool).1.proposerId = me)
    (hsort : List.Pairwise (fun a b => a.num ≤ b.num)
      (l.map Prod.fst)) :
    ((scanAcrossTicksTrace (p1aSendStep true) none l).flatten).Nodup := by
  have h := (p1aSendStep_once me l none hown hsort
    (fun a ha => nomatch ha)).1
  exact List.Pairwise.imp (fun {a b} hlt => fun heq => by
    rw [heq] at hlt
    omega) h

/-- What one pass of the election body **ensures**, over the `Values`
denotation, against the `a_log` input and the `p_is_leader` cycle-input
wire `ff` (quantified over arbitrary cycle wires — the closed knot
discharges the prefix hypothesis). These fields are exactly
`sequence_payload`'s requirements plus the K1 promise facts. -/
structure LECoreEnsures (variant : PaxosVariant) (prop acc : L)
    (qs : Nat)
    (al : TickV (mem acc) (ALog P (mem prop)) .unbounded)
    (ff : TickV (mem prop) Bool .unbounded)
    (o : LEWires (Values L mem) prop acc P) : Prop where
  /-- Ballot ownership. -/
  own : ∀ (i : Fin (mem prop)), ∀ b ∈ (o.p_ballot i).vals,
    (b : Ballot (mem prop)).proposerId = i
  /-- Leader ticks see nonempty views. -/
  lead_ne : 1 ≤ qs → ∀ (i : Fin (mem prop)) {t : Nat}
    (hpl : t < (o.p_is_leader i).length)
    (hpr : t < (o.p_accepted_values i).length),
    (o.p_is_leader i)[t]'hpl = true →
    (o.p_accepted_values i)[t]'hpr ≠ 0
  /-- Ballot stability along reigns (FINDINGS D21), **given** that the
  cycle-input flag is a prefix of the output flag (the closed knot's
  Kleene fact): a standing leader cannot have solicited its usurper. -/
  stable : 1 ≤ qs → (∀ i, ff i <+: o.p_is_leader i) →
    ∀ (i : Fin (mem prop)) {t : Nat}
    (ht1 : t + 1 < (o.p_is_leader i).length)
    (hb1 : t + 1 < (o.p_ballot i).vals.length),
    (o.p_is_leader i)[t + 1]'ht1 = true →
    (o.p_is_leader i)[t]'(Nat.lt_of_succ_lt ht1) = true →
    (o.p_ballot i).vals[t + 1]'hb1
      = (o.p_ballot i).vals[t]'(Nat.lt_of_succ_lt hb1)
  /-- View pinning by ballot number (frozen quorum buckets). -/
  pinned : 1 ≤ qs → ∀ (i : Fin (mem prop)) {t t' : Nat}
    (hpl : t < (o.p_is_leader i).length)
    (hpl' : t' < (o.p_is_leader i).length)
    (hpr : t < (o.p_accepted_values i).length)
    (hpr' : t' < (o.p_accepted_values i).length)
    (hpb : t < (o.p_ballot i).vals.length)
    (hpb' : t' < (o.p_ballot i).vals.length),
    (o.p_is_leader i)[t]'hpl = true →
    (o.p_is_leader i)[t']'hpl' = true →
    ((o.p_ballot i).vals[t]'hpb).num
      = ((o.p_ballot i).vals[t']'hpb').num →
    (o.p_accepted_values i)[t]'hpr = (o.p_accepted_values i)[t']'hpr'
  /-- Leader-view promise: at a leader tick the view is full, and every
  payload is the `a_log` **input** value at an acceptor tick where the
  `a_max_ballot` **output** carries the tick's own ballot. -/
  view_promise : ∀ (i : Fin (mem prop)) {t : Nat}
    (ht : t < (o.p_is_leader i).length),
    (o.p_is_leader i)[t]'ht = true →
    ∃ (hpr : t < (o.p_accepted_values i).length)
      (hpb : t < (o.p_ballot i).vals.length),
      qs ≤ ((o.p_accepted_values i)[t]'hpr).card ∧
      ∀ v ∈ (o.p_accepted_values i)[t]'hpr,
        ∃ (j : Fin (mem acc)) (tj : Nat) (htj : tj < (al j).length),
          v = (al j)[tj]'htj ∧
          ∃ hm : tj < (o.a_max_ballot j).vals.length,
            (o.a_max_ballot j).vals[tj]'hm
              = some ((o.p_ballot i).vals[t]'hpb)
  /-- Distinct providers (guarded — the B1 send-once fan-in): a leader
  tick's view was contributed by `qs` **distinct** acceptors. -/
  providers : variant = .guarded → 1 ≤ qs →
    ∀ (i : Fin (mem prop)) {t : Nat}
    (ht : t < (o.p_is_leader i).length),
    (o.p_is_leader i)[t]'ht = true →
    ∃ (hpr : t < (o.p_accepted_values i).length)
      (hpb : t < (o.p_ballot i).vals.length)
      (S : List (Fin (mem acc))), S.Nodup ∧ qs ≤ S.length ∧
      ∀ j ∈ S, ∃ v ∈ (o.p_accepted_values i)[t]'hpr,
        ∃ (tj : Nat) (htj : tj < (al j).length),
          v = (al j)[tj]'htj ∧
          ∃ hm : tj < (o.a_max_ballot j).vals.length,
            (o.a_max_ballot j).vals[tj]'hm
              = some ((o.p_ballot i).vals[t]'hpb)

/-- What the closed `leader_election` **ensures** against the `a_log`
input: `LECoreEnsures` at the knot, with `stable`'s flag-prefix
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
set_option maxRecDepth 16384 in
/-- **paxos.rs:253–346 `leader_election`**: one Rust fn, one Lean def.
The election body — one pass over the assumed cycle wires
(paxos.rs:271–345, the Rust text minus the `forward_ref` plumbing) — is
the `let body` below, with its per-pass contract (`LECoreEnsures`)
colocated; the returned **function** closes the three cycles over
`(p2b_ballots, a_log)`, and the clause carries BOTH the unary contract
(`LEEnsures`) and binary Flo monotonicity between any two runs (the
former external `leader_election_mono`, now colocated so no proof ever
re-spells the pass). Output tuple: (`p_ballot`, `p_is_leader`,
`p_accepted_values`, `a_max_ballot`). -/
def leader_election (H : HydroSem L mem) (variant : PaxosVariant)
    (prop acc : L)
    (quorum_size : Nat)
    (dec : LEDec (mem prop) (mem acc) P)
    (chFail chIAL chP1a chP1b : ChannelId)
    (cycFail cycIAL cycLead : CycleId) :
    {f : H.Stream prop (Ballot (mem prop)) .noOrder .exactlyOnce →
        H.TickSingleton acc (ALog P (mem prop)) .unbounded →
        H.TickSingleton prop (Ballot (mem prop))
            (.monotonic Ballot.numVO)
          × H.TickSingleton prop Bool .unbounded
          × H.TickStream prop (ALog P (mem prop)) .noOrder .exactlyOnce
          × H.TickSingleton acc (Option (Ballot (mem prop)))
              (.monotonic Ballot.obtVO) //
      ∀ hv : H = Values L mem,
        match H, hv, f with
        | _, rfl, f =>
          (∀ p2b al,
            LEEnsures variant prop acc quorum_size al (f p2b al))
          ∧ (∀ p2b p2b' al al',
              (∀ i, p2b i ≤ p2b' i) → (∀ j, al j <+: al' j) →
              (∀ i, ((f p2b al).1 i).vals <+: ((f p2b' al').1 i).vals)
              ∧ (∀ i, (f p2b al).2.1 i <+: (f p2b' al').2.1 i)
              ∧ (∀ i, (f p2b al).2.2.1 i <+: (f p2b' al').2.2.1 i)
              ∧ (∀ j, ((f p2b al).2.2.2 j).vals
                  <+: ((f p2b' al').2.2.2 j).vals))} :=
  -- one pass of the election body over the assumed cycle wires
  let body : (H' : HydroSem L mem) →
      H'.Stream prop (Ballot (mem prop)) .noOrder .exactlyOnce →
      (a_log : H'.TickSingleton acc (ALog P (mem prop)) .unbounded) →
      H'.Stream prop (Ballot (mem prop)) .noOrder .exactlyOnce →
      H'.Stream prop (Ballot (mem prop)) .noOrder .atLeastOnce →
      (ff : H'.TickSingleton prop Bool .unbounded) →
      {out : LEWires H' prop acc P //
        ∀ hv : H' = Values L mem,
          match H', hv, a_log, ff, out with
          | _, rfl, al, ff, o =>
            LECoreEnsures variant prop acc quorum_size al ff o} :=
    fun H p_received_p2b_ballots a_log p1b_fail i_am_leader
        p_is_leader_ff =>
    -- p1b_fail.merge_unordered(p2b_ballots).merge_unordered(i_am_leader)
    --   .max().into_singleton().snapshot(tick, nondet_leader)
    let received := H.union chIAL
      (H.weaken_retries (H.union chFail p1b_fail p_received_p2b_ballots))
      i_am_leader
    let p_received_max_ballot := H.snapshot
      (H.fold_monotone Ballot.obtVO Ballot.maxFold none
        (by exact ⟨fun s x y => Ballot.maxFold_comm s x y,
          fun s x => Ballot.maxFold_idem s x⟩)
        (fun s x => Ballot.obtLE_maxFold s x)
        received)
      dec.receivedMax
    -- p_ballot_calc(p_received_max_ballot)
    let bcS := p_ballot_calc H prop
      (H.forgetBound p_received_max_ballot)
    let bc := bcS.val
    -- p_leader_heartbeat(p_is_leader→, p_ballot, …)
    let hbS := p_leader_heartbeat H prop p_is_leader_ff
      (H.forgetBound bc.1) dec.hb chIAL
    let hb := hbS.val
    -- p_to_acceptors_p1a = p_ballot.filter_if(p_trigger_election)
    --   .all_ticks().broadcast(acceptors, …).values()
    -- GUARDED (B1 fix): send each ballot's P1a once (dedup-last state).
    let p_to_acceptors_p1a := H.values (H.broadcast chP1a
      (H.allTicks (H.emitBatches
        (H.scan_across_ticks
          (H.zipTick (H.forgetBound bc.1) hb.2)
          (fun _me => p1aSendStep variant.sendOnce)
          none))))
    -- acceptor_p1(p1a.batch(acceptor_tick, nondet), a_log)
    let ap1S := acceptor_p1 H acc prop
      (H.batch p_to_acceptors_p1a dec.p1aBatch) a_log chP1b
    let ap1 := ap1S.val
    -- p_p1b(a_to_proposers_p1b, p_ballot, p_has_largest_ballot, …)
    let ppS := p_p1b H prop ap1.2 (H.forgetBound bc.1) bc.2 quorum_size
      dec.p1b
    let pp := ppS.val
    ⟨{ p_ballot := bc.1
       p_has_largest_ballot := bc.2
       i_am_leader := hb.1
       a_max_ballot := ap1.1
       p_is_leader := pp.1
       p_accepted_values := pp.2.1
       fail_ballots := pp.2.2 }, by
    intro hv; subst hv
    -- the sub-module contracts, at the body's own wires
    have hbc := bcS.property rfl
    have hhb := hbS.property rfl
    have hap1 := ap1S.property rfl
    have hpp := ppS.property rfl
    -- the ballot leg, in the contract's own spelling (`forgetBound`)
    have hpbv : ∀ (i : Fin (mem prop)),
        (Values L mem).forgetBound bc.1 i = (bc.1 i).vals :=
      fun i => rfl
    -- the accepted-batch face at one tick: the gated quorum bucket
    have hbatch : ∀ (i : Fin (mem prop)) {t : Nat}
        (hpr : t < (pp.2.1 i).length)
        (hv : t < (pP1bViews (mem prop) quorum_size (ap1.2 i)
          (dec.p1b.order i) (dec.p1b.snap i)).length)
        (hb : t < ((bc.1 i).vals).length),
        (pp.2.1 i)[t]'hpr = Multiset.ofList
          ((pP1bQuorum quorum_size
            ((pP1bViews (mem prop) quorum_size (ap1.2 i)
              (dec.p1b.order i) (dec.p1b.snap i))[t]'hv)
            ((bc.1 i).vals[t]'hb)).getD []) := by
      intro i t hpr hv hb
      have hface : pp.2.1 i = List.map (fun l => Multiset.ofList l)
          ((((pP1bViews (mem prop) quorum_size (ap1.2 i)
              (dec.p1b.order i) (dec.p1b.snap i)).zip
            ((bc.1 i).vals)).map
              (fun vb => pP1bQuorum quorum_size vb.1 vb.2)).map
            (fun ql => ql.getD [])) := hpp.accepted_eq i
      rw [List.getElem_of_eq hface hpr]
      rw [List.getElem_map, List.getElem_map, List.getElem_map]
      simp only [Trace.zip]
      rw [List.getElem_zip]
    -- the merged P1a pool at an acceptor: per-proposer released streams
    have hp1a : ∀ (j : Fin (mem acc)),
        p_to_acceptors_p1a j
          = ((List.finRange (mem prop)).map
            (fun r => (↑((scanAcrossTicksTrace
                (p1aSendStep variant.sendOnce) none
                (Trace.zip ((bc.1 r).vals) (hb.2 r))).flatten)
              : Multiset (Ballot (mem prop))))).sum := fun j => rfl
    refine { own := ?_, lead_ne := ?_, stable := ?_, pinned := ?_,
             view_promise := ?_, providers := ?_ }
    · exact fun i b hb => hbc.own i b hb
    · -- lead_ne: the leader gate answers with a full bucket
      intro hq1 i t hpl hpr htrue
      obtain ⟨hv, hb, hl, qlogs, hq, hlen, -⟩ := hpp.leader_gate i hpl htrue
      intro hzero
      have hq2 : pP1bQuorum quorum_size
          ((pP1bViews (mem prop) quorum_size (ap1.2 i)
            (dec.p1b.order i) (dec.p1b.snap i))[t]'hv)
          ((bc.1 i).vals[t]'hb) = some qlogs := hq
      have hbeq := hbatch i hpr hv hb
      rw [hq2] at hbeq
      rw [hbeq] at hzero
      have hnil : qlogs = [] := by
        simpa using hzero
      rw [hnil] at hlen
      simp at hlen
      omega
    · -- stable: the fabricated-reign regress at the run
      intro hq1 hknot i t ht1 hb1 h1 h0
      have hflag : pp.1 i = pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i) :=
        hpp.flags_eq i
      -- the solicitation chain: an Ok reply pins a trigger-true follower
      -- tick of the sender's own run
      have hsol : ∀ m ∈ ap1.2 i,
          (∃ v, (m : P1b P (mem prop)).res = .ok v) →
          ∃ (u : Nat)
            (hu : u < (pP1bFlags (mem prop) quorum_size (ap1.2 i)
              ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
              (dec.p1b.snap i)).length)
            (hbu : u < ((bc.1 i).vals).length),
            (bc.1 i).vals[u]'hbu = m.ballot
            ∧ (pP1bFlags (mem prop) quorum_size (ap1.2 i) ((bc.1 i).vals)
                (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i))[u]'hu
              = false := by
        intro m hm hok
        -- the reply echoes a consumed P1a at the sending acceptor
        obtain ⟨j, tj, hbj, hmj, hlj, hball, hroute, -⟩ :=
          hap1.reply_src i m hm
        -- consumed batch → merged pool
        have hball2 : m.ballot ∈ (batchCuts (p_to_acceptors_p1a j) 0
            (dec.p1aBatch j))[tj]'hbj := hball
        have hin_pool : m.ballot ∈ p_to_acceptors_p1a j := by
          have hmem_sum : m.ballot ∈ (batchCuts (p_to_acceptors_p1a j) 0
              (dec.p1aBatch j)).sum :=
            mem_list_sum.mpr ⟨_, List.getElem_mem hbj, hball2⟩
          have hle := batchCuts_sum_le (pool := p_to_acceptors_p1a j)
            (d := dec.p1aBatch j) (consumed := 0) (Multiset.zero_le _)
          rw [Multiset.zero_add] at hle
          exact Multiset.mem_of_le hle hmem_sum
        -- pool → some sender's released stream
        rw [hp1a j] at hin_pool
        obtain ⟨ms, hms, hbin⟩ := mem_list_sum.mp hin_pool
        obtain ⟨r, -, rfl⟩ := List.mem_map.mp hms
        have hbin' : m.ballot ∈ (scanAcrossTicksTrace
            (p1aSendStep variant.sendOnce) none
            (Trace.zip ((bc.1 r).vals) (hb.2 r))).flatten :=
          Multiset.mem_coe.mp hbin
        obtain ⟨u, hlz, hbu_eq, htrig⟩ :=
          p1aSendStep_flatten_mem variant.sendOnce _ none hbin'
        -- zip projections at the release tick
        have hu1 : u < ((bc.1 r).vals).length := by
          have := hlz
          simp only [Trace.zip, List.length_zip] at this
          omega
        have hu2 : u < (hb.2 r).length := by
          have := hlz
          simp only [Trace.zip, List.length_zip] at this
          omega
        have hzel : (Trace.zip ((bc.1 r).vals) (hb.2 r))[u]'hlz
            = (((bc.1 r).vals)[u]'hu1, (hb.2 r)[u]'hu2) := by
          simp only [Trace.zip]
          exact List.getElem_zip ..
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
        have hpre : p_is_leader_ff r <+: pp.1 r := hknot r
        have hu_pp : u < (pp.1 r).length :=
          Nat.lt_of_lt_of_le hf hpre.length_le
        have hval := List.IsPrefix.getElem hpre hf
        rw [hff] at hval
        have hflag_r : pp.1 r = pP1bFlags (mem prop) quorum_size (ap1.2 r)
            ((bc.1 r).vals) (bc.2 r) (dec.p1b.order r) (dec.p1b.snap r) :=
          hpp.flags_eq r
        have hu_flags : u < (pP1bFlags (mem prop) quorum_size (ap1.2 r)
            ((bc.1 r).vals) (bc.2 r) (dec.p1b.order r)
            (dec.p1b.snap r)).length := by
          rw [← hflag_r]
          exact hu_pp
        refine ⟨u, hu_flags, hu1, hbu_eq.symm, ?_⟩
        rw [← List.getElem_of_eq hflag_r hu_pp]
        exact hval.symm
      -- discharge `p_p1b`'s requirements and run the regress
      have req : PP1bRequires (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i) i :=
        { has_largest := hbc.hasLargest_true i
          ballot_own := hbc.own i
          ballot_mono := fun h ht' => (bc.1 i).ascending h ht'
          solicited_at_follower := hsol }
      have ht1' : t + 1 < (pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i)).length := by
        rw [← hflag]
        exact ht1
      have h1' : (pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i))[t + 1]'ht1' = true := by
        rw [← List.getElem_of_eq hflag ht1]
        exact h1
      have h0' : (pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i))[t]'(Nat.lt_of_succ_lt ht1') = true := by
        rw [← List.getElem_of_eq hflag (Nat.lt_of_succ_lt ht1)]
        exact h0
      obtain ⟨hb1', heq⟩ := pP1b_ballot_stable quorum_size hq1 (ap1.2 i)
        ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i) i req
        ht1' h1' h0'
      exact heq
    · -- pinned: frozen quorum buckets across same-ballot leader ticks
      intro hq1 i t t' hpl hpl' hpr hpr' hpb hpb' hl hl' hnum
      have hflag : pp.1 i = pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i) :=
        hpp.flags_eq i
      have hbeq : (bc.1 i).vals[t]'hpb = (bc.1 i).vals[t']'hpb' :=
        Ballot.eq_of_num_owner hnum (by
          rw [hbc.own i _ (List.getElem_mem hpb),
            hbc.own i _ (List.getElem_mem hpb')])
      have hplf : t < (pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i)).length := by
        rw [← hflag]
        exact hpl
      have hplf' : t' < (pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i)).length := by
        rw [← hflag]
        exact hpl'
      have hlf : (pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i))[t]'hplf = true := by
        rw [← List.getElem_of_eq hflag hpl]
        exact hl
      have hlf' : (pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i))[t']'hplf' = true := by
        rw [← List.getElem_of_eq hflag hpl']
        exact hl'
      rcases Nat.le_total t t' with hle | hle
      · obtain ⟨hv, hv', qlogs, hqt, hqt'⟩ := pP1b_quorum_pinned
          quorum_size hq1 (ap1.2 i) ((bc.1 i).vals) (bc.2 i)
          (dec.p1b.order i) (dec.p1b.snap i) hle hplf hplf' hlf hlf'
          (hb := hpb) (hb' := hpb') hbeq
        have h1 := hbatch i hpr hv hpb
        have h2 := hbatch i hpr' hv' hpb'
        rw [hqt] at h1
        rw [hqt'] at h2
        rw [h1, h2]
      · obtain ⟨hv', hv, qlogs, hqt', hqt⟩ := pP1b_quorum_pinned
          quorum_size hq1 (ap1.2 i) ((bc.1 i).vals) (bc.2 i)
          (dec.p1b.order i) (dec.p1b.snap i) hle hplf' hplf hlf' hlf
          (hb := hpb') (hb' := hpb) hbeq.symm
        have h1 := hbatch i hpr hv hpb
        have h2 := hbatch i hpr' hv' hpb'
        rw [hqt] at h1
        rw [hqt'] at h2
        rw [h1, h2]
    · -- view_promise: the leader's view opens to acceptor-tick promises
      intro i t ht htrue
      obtain ⟨hv, hb, hl, qlogs, hq, hlen, -⟩ := hpp.leader_gate i ht htrue
      have hq2 : pP1bQuorum quorum_size
          ((pP1bViews (mem prop) quorum_size (ap1.2 i)
            (dec.p1b.order i) (dec.p1b.snap i))[t]'hv)
          ((bc.1 i).vals[t]'hb) = some qlogs := hq
      have hface : pp.2.1 i = List.map (fun l => Multiset.ofList l)
          ((((pP1bViews (mem prop) quorum_size (ap1.2 i)
              (dec.p1b.order i) (dec.p1b.snap i)).zip
            ((bc.1 i).vals)).map
              (fun vb => pP1bQuorum quorum_size vb.1 vb.2)).map
            (fun ql => ql.getD [])) := hpp.accepted_eq i
      have hbv2 : t < ((bc.1 i).vals).length := hb
      have hpr : t < (pp.2.1 i).length := by
        rw [hface]
        simp only [List.length_map, Trace.zip, List.length_zip]
        omega
      refine ⟨hpr, hb, ?_, ?_⟩
      · -- fullness
        have hb1 := hbatch i hpr hv hb
        rw [hq2] at hb1
        rw [hb1]
        simpa using hlen
      · -- each payload opens to an acceptor promise tick
        intro v hvmem
        obtain ⟨hb0, m, hm, hres, hball⟩ := hpp.accepted_src i hpr v hvmem
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
          simp at hshape
    · -- providers: the guarded send-once fan-in yields distinct acceptors
      intro hvar hq1 i t ht htrue
      subst hvar
      -- the leader bucket at the pure layer
      have hflag : pp.1 i = pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i) :=
        hpp.flags_eq i
      have hu_flags : t < (pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i)).length := by
        rw [← hflag]
        exact ht
      have htrue' : (pP1bFlags (mem prop) quorum_size (ap1.2 i)
          ((bc.1 i).vals) (bc.2 i) (dec.p1b.order i)
          (dec.p1b.snap i))[t]'hu_flags = true := by
        rw [← List.getElem_of_eq hflag ht]
        exact htrue
      obtain ⟨hv, hbt, hl, qlogs, hq, hmem_bucket, hlen_eq, -⟩ :=
        pP1b_leader_bucket quorum_size hq1 (ap1.2 i) ((bc.1 i).vals)
          (bc.2 i) (dec.p1b.order i) (dec.p1b.snap i) hu_flags htrue'
      have hface : pp.2.1 i = List.map (fun l => Multiset.ofList l)
          ((((pP1bViews (mem prop) quorum_size (ap1.2 i)
              (dec.p1b.order i) (dec.p1b.snap i)).zip
            ((bc.1 i).vals)).map
              (fun vb => pP1bQuorum quorum_size vb.1 vb.2)).map
            (fun ql => ql.getD [])) := hpp.accepted_eq i
      have hpr : t < (pp.2.1 i).length := by
        rw [hface]
        simp only [List.length_map, Trace.zip, List.length_zip]
        omega
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
        pP1bViews_bucket_sub_oks quorum_size (ap1.2 i) (dec.p1b.order i)
          (dec.p1b.snap i) hv hmem_bucket
      -- the pool decomposes by sending acceptor
      have hdec : ap1.2 i = ((List.finRange (mem acc)).map
          (fun j => ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
            (dec.p1aBatch j')) a_log j i)).sum := hap1.reply_decomp i
      -- each ballot is on the wire at most once (send-once + ownership)
      have hcount_pool : ∀ (j : Fin (mem acc)),
          (p_to_acceptors_p1a j).count ((bc.1 i).vals[t]'hbt) ≤ 1 := by
        intro j
        rw [hp1a j, count_list_sum, List.map_map]
        refine sum_map_le_single (List.nodup_finRange _) _ i ?_ ?_
        · -- other proposers never carry `i`'s ballot
          intro r _ hri
          show Multiset.count ((bc.1 i).vals[t]'hbt)
            (↑((scanAcrossTicksTrace
              (p1aSendStep PaxosVariant.guarded.sendOnce) none
              (Trace.zip ((bc.1 r).vals) (hb.2 r))).flatten)) = 0
          rw [Multiset.count_eq_zero]
          intro hbin
          obtain ⟨u, hlz, hbeq2, -⟩ := p1aSendStep_flatten_mem
            PaxosVariant.guarded.sendOnce _ none
            (Multiset.mem_coe.mp hbin)
          have hu1 : u < ((bc.1 r).vals).length := by
            have := hlz
            simp only [Trace.zip, List.length_zip] at this
            omega
          have hu2 : u < (hb.2 r).length := by
            have := hlz
            simp only [Trace.zip, List.length_zip] at this
            omega
          have hzel : (Trace.zip ((bc.1 r).vals) (hb.2 r))[u]'hlz
              = (((bc.1 r).vals)[u]'hu1, (hb.2 r)[u]'hu2) := by
            simp only [Trace.zip]
            exact List.getElem_zip ..
          rw [hzel] at hbeq2
          have : ((bc.1 i).vals[t]'hbt).proposerId = r := by
            rw [hbeq2]
            exact hbc.own r _ (List.getElem_mem hu1)
          rw [hbown] at this
          exact hri this.symm
        · -- the owner releases it at most once (B1)
          show Multiset.count ((bc.1 i).vals[t]'hbt)
            (↑((scanAcrossTicksTrace
              (p1aSendStep PaxosVariant.guarded.sendOnce) none
              (Trace.zip ((bc.1 i).vals) (hb.2 i))).flatten)) ≤ 1
          have hown_zip : ∀ x ∈ Trace.zip ((bc.1 i).vals) (hb.2 i),
              (x : Ballot (mem prop) × Bool).1.proposerId = i := by
            intro x hx
            have hx' : x ∈ List.zip ((bc.1 i).vals) (hb.2 i) := hx
            exact hbc.own i _ (List.of_mem_zip hx').1
          have hsort_zip : List.Pairwise (fun a b => a.num ≤ b.num)
              ((Trace.zip ((bc.1 i).vals) (hb.2 i)).map Prod.fst) := by
            rw [List.pairwise_iff_getElem]
            intro u u' hu hu' hlt
            have hvu : u < ((bc.1 i).vals).length
                ∧ u < (hb.2 i).length := by
              have h0 := hu
              simp only [List.length_map, Trace.zip, List.length_zip,
                Nat.lt_min] at h0
              exact h0
            have hvu' : u' < ((bc.1 i).vals).length
                ∧ u' < (hb.2 i).length := by
              have h0 := hu'
              simp only [List.length_map, Trace.zip, List.length_zip,
                Nat.lt_min] at h0
              exact h0
            have he1 : ((Trace.zip ((bc.1 i).vals) (hb.2 i)).map
                Prod.fst)[u]'hu = (bc.1 i).vals[u]'hvu.1 := by
              simp only [Trace.zip, List.getElem_map, List.getElem_zip]
            have he2 : ((Trace.zip ((bc.1 i).vals) (hb.2 i)).map
                Prod.fst)[u']'hu' = (bc.1 i).vals[u']'hvu'.1 := by
              simp only [Trace.zip, List.getElem_map, List.getElem_zip]
            rw [he1, he2]
            exact (bc.1 i).ascending (Nat.le_of_lt hlt) hvu'.1
          have hnd : ((scanAcrossTicksTrace (p1aSendStep true) none
              (Trace.zip ((bc.1 i).vals) (hb.2 i))).flatten).Nodup :=
            p1aSendStep_once_nodup i _ hown_zip hsort_zip
          rw [Multiset.coe_count]
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
        have hstep3 : (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j')
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
        have hmb : m'.ballot = (bc.1 i).vals[t]'hbt ∧ m'.res = .ok v := by
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
        obtain ⟨tj, hbj, hmj, hlj, -, -, hshape⟩ := hap1.from_src j i m' hm'
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
          simp at hshape⟩
  -- close the three cycles (paxos.rs:253–270): fail feedback, leader
  -- gossip, and the is-leader flag knot
  let core := fun p2b al p1b_fail i_am_leader p_is_leader_ff =>
    (body H p2b al p1b_fail i_am_leader p_is_leader_ff).val
  let fails := fun p2b al i_am_leader ff =>
    H.fix_stream cycFail dec.fuelFail
      (fun fl => (core p2b al fl i_am_leader ff).fail_ballots)
  let iam := fun p2b al ff => H.fix_stream cycIAL dec.fuelIAL
    (fun ia => (core p2b al (fails p2b al ia ff) ia ff).i_am_leader)
  let ffx := fun p2b al => H.fix_tick cycLead dec.fuelLead
    (fun f => (core p2b al (fails p2b al (iam p2b al f) f)
      (iam p2b al f) f).p_is_leader)
  ⟨fun p2b al =>
    let w := core p2b al
      (fails p2b al (iam p2b al (ffx p2b al)) (ffx p2b al))
      (iam p2b al (ffx p2b al)) (ffx p2b al)
    (w.p_ballot, w.p_is_leader, w.p_accepted_values, w.a_max_ballot),
   by
    intro hv; subst hv
    refine ⟨?_, ?_⟩
    · -- the unary contract: `LECoreEnsures` at the knot, `stable`'s
      -- flag-prefix hypothesis discharged by Kleene ascent
      intro p2b al
      have hcore := (body (Values L mem) p2b al
        (fails p2b al (iam p2b al (ffx p2b al)) (ffx p2b al))
        (iam p2b al (ffx p2b al)) (ffx p2b al)).property rfl
      -- one Kleene step of the flag body preserves prefixes (`MonoRel`)
      have hstep : ∀ {a b : TickV (mem prop) Bool .unbounded},
          (∀ i, a i <+: b i) →
          ∀ i, (core p2b al (fails p2b al (iam p2b al a) a)
              (iam p2b al a) a).p_is_leader i
            <+: (core p2b al (fails p2b al (iam p2b al b) b)
              (iam p2b al b) b).p_is_leader i := by
        intro a b hab
        -- the fail leg, coupled through its own fixpoint
        have hfl : ∀ (x y : Fin (mem prop)
            → RetryPool (Ballot (mem prop))),
            (∀ i, RetryPool.le (x i) (y i)) →
            ∀ i, fails p2b al x a i ≤ fails p2b al y b i := by
          intro x y hxy
          exact iterate_mono_param
            (R := fun (u v : Fin (mem prop)
              → Multiset (Ballot (mem prop))) => ∀ i, u i ≤ v i)
            (F := fun fl => (core p2b al fl x a).fail_ballots)
            (F' := fun fl => (core p2b al fl y b).fail_ballots)
            (fun {u v} huv => (body (MonoRel L mem)
              ⟨(p2b, p2b), fun _ => le_refl _⟩
              ⟨(al, al), fun _ => List.prefix_refl _⟩
              ⟨(u, v), huv⟩ ⟨(x, y), hxy⟩
              ⟨(a, b), hab⟩).val.fail_ballots.property)
            (x := fun _ => 0) (x' := fun _ => 0) (fun _ => le_refl _)
            dec.fuelFail
        -- the gossip leg, coupled through its own fixpoint
        have hia : ∀ i, RetryPool.le (iam p2b al a i) (iam p2b al b i) :=
          iterate_mono_param
            (R := fun (x y : Fin (mem prop)
              → RetryPool (Ballot (mem prop))) =>
              ∀ i, RetryPool.le (x i) (y i))
            (F := fun ia => (core p2b al (fails p2b al ia a) ia
              a).i_am_leader)
            (F' := fun ia => (core p2b al (fails p2b al ia b) ia
              b).i_am_leader)
            (fun {x y} hxy => (body (MonoRel L mem)
              ⟨(p2b, p2b), fun _ => le_refl _⟩
              ⟨(al, al), fun _ => List.prefix_refl _⟩
              ⟨(fails p2b al x a, fails p2b al y b), hfl x y hxy⟩
              ⟨(x, y), hxy⟩
              ⟨(a, b), hab⟩).val.i_am_leader.property)
            (x := fun _ => RetryPool.mk 0) (x' := fun _ => RetryPool.mk 0)
            (fun _ => RetryPool.le_refl _)
            dec.fuelIAL
        exact (body (MonoRel L mem)
          ⟨(p2b, p2b), fun _ => le_refl _⟩
          ⟨(al, al), fun _ => List.prefix_refl _⟩
          ⟨(fails p2b al (iam p2b al a) a, fails p2b al (iam p2b al b) b),
            hfl (iam p2b al a) (iam p2b al b) hia⟩
          ⟨(iam p2b al a, iam p2b al b), hia⟩
          ⟨(a, b), hab⟩).val.p_is_leader.property
      -- the knot: the cycle-input flag is a prefix of the output flag
      have hknot : ∀ i, ffx p2b al i
          <+: (core p2b al
            (fails p2b al (iam p2b al (ffx p2b al)) (ffx p2b al))
            (iam p2b al (ffx p2b al)) (ffx p2b al)).p_is_leader i :=
        iterate_le_succ
          (R := fun (a b : TickV (mem prop) Bool .unbounded) =>
            ∀ i, a i <+: b i)
          (F := fun f => (core p2b al (fails p2b al (iam p2b al f) f)
            (iam p2b al f) f).p_is_leader)
          (x := fun _ => [])
          (fun _ => List.nil_prefix)
          (fun {a b} hab => hstep hab)
          dec.fuelLead
      exact { own := hcore.own
              lead_ne := hcore.lead_ne
              stable := fun hq1 => hcore.stable hq1 hknot
              pinned := hcore.pinned
              view_promise := hcore.view_promise
              providers := hcore.providers }
    · -- binary Flo monotonicity: grow the received pool and the `a_log`
      -- wire, keep every decision — all four output legs extend; the
      -- knots are re-crossed with the Kleene lemmas, `iterate_mono_param`
      -- coupling the two runs' fixpoints with the pass's `MonoRel`
      -- instantiation as the step
      intro p2b p2b' al al' hp2b hal
      -- the fail-cycle fixpoints, coupled
      have hfails : ∀ (x y : Fin (mem prop)
          → RetryPool (Ballot (mem prop)))
          (a b : TickV (mem prop) Bool .unbounded),
          (∀ i, RetryPool.le (x i) (y i)) → (∀ i, a i <+: b i) →
          ∀ i, fails p2b al x a i ≤ fails p2b' al' y b i := by
        intro x y a b hxy hab
        exact iterate_mono_param
          (R := fun (u v : Fin (mem prop)
            → Multiset (Ballot (mem prop))) => ∀ i, u i ≤ v i)
          (F := fun fl => (core p2b al fl x a).fail_ballots)
          (F' := fun fl => (core p2b' al' fl y b).fail_ballots)
          (fun {u v} huv => (body (MonoRel L mem)
            ⟨(p2b, p2b'), hp2b⟩ ⟨(al, al'), hal⟩
            ⟨(u, v), huv⟩ ⟨(x, y), hxy⟩
            ⟨(a, b), hab⟩).val.fail_ballots.property)
          (fun _ => le_refl _) dec.fuelFail
      -- the gossip-cycle fixpoints, coupled
      have hiam : ∀ (a b : TickV (mem prop) Bool .unbounded),
          (∀ i, a i <+: b i) →
          ∀ i, RetryPool.le (iam p2b al a i) (iam p2b' al' b i) := by
        intro a b hab
        exact iterate_mono_param
          (R := fun (x y : Fin (mem prop)
            → RetryPool (Ballot (mem prop))) =>
            ∀ i, RetryPool.le (x i) (y i))
          (F := fun ia => (core p2b al (fails p2b al ia a) ia
            a).i_am_leader)
          (F' := fun ia => (core p2b' al' (fails p2b' al' ia b) ia
            b).i_am_leader)
          (fun {x y} hxy => (body (MonoRel L mem)
            ⟨(p2b, p2b'), hp2b⟩ ⟨(al, al'), hal⟩
            ⟨(fails p2b al x a, fails p2b' al' y b),
              hfails x y a b hxy hab⟩
            ⟨(x, y), hxy⟩
            ⟨(a, b), hab⟩).val.i_am_leader.property)
          (fun _ => RetryPool.le_refl _) dec.fuelIAL
      -- the flag-cycle fixpoints, coupled
      have hff : ∀ i, ffx p2b al i <+: ffx p2b' al' i := by
        exact iterate_mono_param
          (R := fun (a b : TickV (mem prop) Bool .unbounded) =>
            ∀ i, a i <+: b i)
          (F := fun f => (core p2b al (fails p2b al (iam p2b al f) f)
            (iam p2b al f) f).p_is_leader)
          (F' := fun f => (core p2b' al'
            (fails p2b' al' (iam p2b' al' f) f)
            (iam p2b' al' f) f).p_is_leader)
          (fun {a b} hab => (body (MonoRel L mem)
            ⟨(p2b, p2b'), hp2b⟩ ⟨(al, al'), hal⟩
            ⟨(fails p2b al (iam p2b al a) a,
              fails p2b' al' (iam p2b' al' b) b),
              hfails _ _ a b (hiam a b hab) hab⟩
            ⟨(iam p2b al a, iam p2b' al' b), hiam a b hab⟩
            ⟨(a, b), hab⟩).val.p_is_leader.property)
          (fun _ => List.nil_prefix) dec.fuelLead
      -- assemble the four output legs at the coupled knots
      refine ⟨(body (MonoRel L mem) ⟨(p2b, p2b'), hp2b⟩ ⟨(al, al'), hal⟩
          ⟨(fails p2b al (iam p2b al (ffx p2b al)) (ffx p2b al),
            fails p2b' al' (iam p2b' al' (ffx p2b' al')) (ffx p2b' al')),
            hfails _ _ _ _ (hiam _ _ hff) hff⟩
          ⟨(iam p2b al (ffx p2b al), iam p2b' al' (ffx p2b' al')),
            hiam _ _ hff⟩
          ⟨(ffx p2b al, ffx p2b' al'), hff⟩).val.p_ballot.property,
        (body (MonoRel L mem) ⟨(p2b, p2b'), hp2b⟩ ⟨(al, al'), hal⟩
          ⟨(fails p2b al (iam p2b al (ffx p2b al)) (ffx p2b al),
            fails p2b' al' (iam p2b' al' (ffx p2b' al')) (ffx p2b' al')),
            hfails _ _ _ _ (hiam _ _ hff) hff⟩
          ⟨(iam p2b al (ffx p2b al), iam p2b' al' (ffx p2b' al')),
            hiam _ _ hff⟩
          ⟨(ffx p2b al, ffx p2b' al'), hff⟩).val.p_is_leader.property,
        (body (MonoRel L mem) ⟨(p2b, p2b'), hp2b⟩ ⟨(al, al'), hal⟩
          ⟨(fails p2b al (iam p2b al (ffx p2b al)) (ffx p2b al),
            fails p2b' al' (iam p2b' al' (ffx p2b' al')) (ffx p2b' al')),
            hfails _ _ _ _ (hiam _ _ hff) hff⟩
          ⟨(iam p2b al (ffx p2b al), iam p2b' al' (ffx p2b' al')),
            hiam _ _ hff⟩
          ⟨(ffx p2b al, ffx p2b' al'), hff⟩).val.p_accepted_values.property,
        (body (MonoRel L mem) ⟨(p2b, p2b'), hp2b⟩ ⟨(al, al'), hal⟩
          ⟨(fails p2b al (iam p2b al (ffx p2b al)) (ffx p2b al),
            fails p2b' al' (iam p2b' al' (ffx p2b' al')) (ffx p2b' al')),
            hfails _ _ _ _ (hiam _ _ hff) hff⟩
          ⟨(iam p2b al (ffx p2b al), iam p2b' al' (ffx p2b' al')),
            hiam _ _ hff⟩
          ⟨(ffx p2b al, ffx p2b' al'), hff⟩).val.a_max_ballot.property⟩⟩

/-! ## Executable non-vacuity: a leader gets elected

One proposer, one acceptor, quorum of 1. Nothing is received at first
(`p_ballot` stays `(0,0)`); the timer fires at the non-leader tick 0, a
P1a goes out, the acceptor promises, and the quorum elects the proposer
at tick 1. The Kleene fuel `3` pins the leader fixpoint: iteration 1
finds no trigger (no ticks realized on the flag), iteration 2 triggers
and elects, iteration 3 confirms stability. -/

private def leScenario :=
  ((leader_election (Values PaxLoc (paxMem 1 1)) .guarded .prop .acc 1
    { receivedMax := fun _ => [{}, {}]            -- empty increments
      hb := { sample := fun _ => []                 -- no heartbeat samples
              timeout := fun _ => [true, true]      -- timer fires
              interval := fun _ => [true, true] }
      p1aBatch := fun _ => [{Ballot.mk 0 0}, {}]    -- p1a batch cuts
      p1b := { order := fun _ => [(Ballot.mk 0 0, (none, []))]
               snap := fun _ => [0, 1] }            -- p1b snapshot cuts
      fuelFail := 1, fuelIAL := 1, fuelLead := 3 }
    0 1 2 3 0 1 2).val
    (fun _ => (0 : Multiset (Ballot 1)))          -- no p2b feedback
    ((fun _ => [(none, []), (none, [])]) :
      TickV 1 (ALog Nat 1) .unbounded))           -- a_log wire

#guard leScenario.2.1 0 = [false, true]

#guard (leScenario.1 0).vals = [Ballot.mk 0 0, Ballot.mk 0 0]

#guard (leScenario.2.2.2 0).vals = [some (Ballot.mk 0 0),
  some (Ballot.mk 0 0)]

end HydroV2
