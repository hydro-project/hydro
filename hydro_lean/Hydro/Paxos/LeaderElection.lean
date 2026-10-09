import Hydro.Paxos.PBallotCalc
import Hydro.Paxos.PLeaderHeartbeat
import Hydro.Paxos.AcceptorP1
import Hydro.Paxos.PP1b
import Hydro.HydroDef
import Hydro.HydroTick

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

namespace Hydro

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

/-- The trivial bundle at the denotation. -/
def LESched.triv {nP nA : Nat} {P : Type} [DecidableEq P] :
    LESched (Values L mem) nP nA P := ⟨(), .triv, .triv⟩

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
    LESched (MonoRel L mem) nP nA P := ⟨(), ⟨()⟩, ⟨()⟩⟩


/-- What the closed `leader_election` **ensures** against the `a_log`
input: the per-pass facts at the knot, with `stable`'s flag-prefix
hypothesis discharged by Kleene ascent (`iterate_le_succ` through the
body's `MonoRel` monotonicity). Output tuple:
(`p_ballot`, `p_is_leader`, `p_accepted_values`, `a_max_ballot`). -/
structure LEEnsures (variant : PaxosVariant) (prop acc : L) (qs : Nat)
    (al : TickV (mem acc) (ALog P (mem prop)))
    (o : TickV (mem prop) (Ballot (mem prop))
      × TickV (mem prop) Bool
      × (Fin (mem prop) → Trace (Multiset (ALog P (mem prop))))
      × TickV (mem acc) (Option (Ballot (mem prop)))) : Prop where
  /-- **The leader wires' discipline** (ownership, ascent, nonempty
  leader views, pinned views, reigns) — the consumer-shaped face
  `sequence_payload` requires (`LeaderDiscipline`), read per tick. -/
  discipline : 1 ≤ qs → LeaderDiscipline (mem prop) P o.1 o.2.1 o.2.2.1
  /-- **Max ascent** along an acceptor's ticks (`acceptor_p1`'s promised
  max only grows). -/
  max_mono : ∀ (j : Fin (mem acc)), Ascending Ballot.obtVO (o.2.2.2 j)
  /-- Leader-view promise. -/
  view_promise : ∀ (i : Fin (mem prop)) {t : Nat}
    (ht : t < (o.2.1 i).length), (o.2.1 i)[t]'ht = true →
    ∃ (hpr : t < (o.2.2.1 i).length) (hpb : t < (o.1 i).length),
      qs ≤ ((o.2.2.1 i)[t]'hpr).card ∧
      ∀ v ∈ (o.2.2.1 i)[t]'hpr,
        ∃ (j : Fin (mem acc)) (tj : Nat) (htj : tj < (al j).length),
          v = (al j)[tj]'htj ∧
          ∃ hm : tj < (o.2.2.2 j).length,
            (o.2.2.2 j)[tj]'hm = some ((o.1 i)[t]'hpb)
  /-- Distinct providers (guarded). -/
  providers : variant = .guarded → 1 ≤ qs →
    ∀ (i : Fin (mem prop)) {t : Nat}
    (ht : t < (o.2.1 i).length), (o.2.1 i)[t]'ht = true →
    ∃ (hpr : t < (o.2.2.1 i).length) (hpb : t < (o.1 i).length)
      (S : List (Fin (mem acc))), S.Nodup ∧ qs ≤ S.length ∧
      ∀ j ∈ S, ∃ v ∈ (o.2.2.1 i)[t]'hpr,
        ∃ (tj : Nat) (htj : tj < (al j).length),
          v = (al j)[tj]'htj ∧
          ∃ hm : tj < (o.2.2.2 j).length,
            (o.2.2.2 j)[tj]'hm = some ((o.1 i)[t]'hpb)

set_option maxHeartbeats 1600000 in
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
    (a_log : H.Ticked acc (ALog P (mem prop))) :
    (H.Ticked prop (Ballot (mem prop))
      × H.Ticked prop Bool
      × H.TickStream prop (ALog P (mem prop)) .noOrder .exactlyOnce
      × H.Ticked acc (Option (Ballot (mem prop))))
  ensures out => LEEnsures variant prop acc quorum_size a_log out :=
  -- paxos.rs:253–270: the three forward_ref cycles, closed mutually
  fix (p1b_fail : H.Stream prop (Ballot (mem prop)) .noOrder
        .exactlyOnce)
      (i_am_leader : H.Stream prop (Ballot (mem prop)) .noOrder
        .atLeastOnce)
      (p_is_leader : H.Ticked prop Bool)
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
    let bc := p_ballot_calc H prop
      p_received_max_ballot
    -- p_leader_heartbeat(p_is_leader→, p_ballot, …)
    let hb := p_leader_heartbeat H prop p_is_leader
      bc.1 dec.hb sched.hb
    ghost have hbc := p_ballot_calc.ensures prop p_received_max_ballot
    -- **ballot ascent**, read pairwise (the calc's register only jumps up)
    ghost have hmono : ∀ (i : Fin (mem prop)),
        List.Pairwise (fun (a b : Ballot (mem prop)) => a.num ≤ b.num) (bc.1 i) := by
      intro i
      rw [List.pairwise_iff_getElem]
      intro t t' ht ht' htt
      exact hbc.mono i (Nat.le_of_lt htt) ht'
    -- p_to_acceptors_p1a = p_ballot.filter_if(p_trigger_election)
    --   .all_ticks().broadcast(acceptors, …).values()
    -- GUARDED (B1 fix): send each ballot's P1a once — the dedup-last
    -- register is a `use::state` `tick` block; its loop invariant is the
    -- once-per-ballot register's (`OnceInv`), under the ballot wire's
    -- ownership/ascent (input facts here, so premises).
    -- FAITHFUL: re-fires on every trigger, exactly as paxos.rs is written.
    tick (state lastSent : Option (Ballot (mem prop)) := none)
        (input bt := H.zipTick bc.1 hb.2)
        (invariant ((p1a_out : List (List (Ballot (mem prop))))
            (lastSent : Option (Ballot (mem prop)))
            (bt : Trace (Ballot (mem prop) × Bool))) =>
          ∀ (me : Fin (mem prop)),
            (∀ x ∈ bt, (x : Ballot (mem prop) × Bool).1.proposerId = me) →
            List.Pairwise (fun (x y : Ballot (mem prop) × Bool) => x.1.num ≤ y.1.num) bt →
            OnceInv (fun x : Ballot (mem prop) × Bool => x.1)
              (fun e : List (Ballot (mem prop)) => e ≠ []) variant.sendOnce
              lastSent (Trace.hist bt p1a_out)) :=
      -- fire ⇔ trigger ∧ (faithful ∨ ballot ≠ lastSent)
      let fire := H.bsMap (H.bsZip lastSent bt)
        (fun (ls, (b, trig)) => trig && !(variant.sendOnce && decide (ls = some b)))
      rebind (lastSent := H.bsMap (H.bsZip fire (H.bsZip lastSent bt))
        (fun (f, (ls, (b, _))) => if f then some b else ls))
      emit (p1a_out := H.bsMap (H.bsZip fire bt) (fun (f, (b, _)) => if f then [b] else []))
      -- the loop obligations: the empty run, and ONE TICK — FIRE (this
      -- ballot goes out; the register names it, and no earlier fire
      -- carries it) or HOLD (nothing released, the register carries)
      prove init := fun _i _me _ _ => OnceInv.init,
        tick := fun i _n _out st x hx hlen ih me hown hmono' => by
          obtain ⟨b, trig⟩ := x
          have ih' := ih me hown hmono'
          simp only [p1a_out_step, den]
          by_cases hc : (trig && !(variant.sendOnce && decide (st = some b))) = true
          · rw [if_pos hc, if_pos hc]
            exact OnceInv.fire hx hlen (fun y hy => by rw [hown y hy, hown _ (Trace.mem_of_read hx)])
              hmono' ih' (fun hro heq => by simp [hro, heq] at hc) (List.cons_ne_nil _ _)
          · rw [if_neg hc, if_neg hc]
            exact OnceInv.hold hx hlen ih' (fun h => h rfl);
    -- **the release register, named**: read at a tick and its invariant
    -- before every tick — the construct's `_reg`, the one place the
    -- block's fold is read
    ghost obtain ⟨_ls, -, hp1a_at, -, -, hp1a_inv⟩ := hp1a_out_reg
    let p_to_acceptors_p1a := H.values (H.broadcast_closed sched.p1aCh
      (H.allTicks (H.flattenOrdered p1a_out)))
    -- acceptor_p1(p1a.batch(acceptor_tick, nondet), a_log)
    let ap1 := acceptor_p1 H acc prop
      (H.batch p_to_acceptors_p1a dec.p1aBatch) a_log sched.ap1
    -- p_p1b(a_to_proposers_p1b, p_ballot, p_has_largest_ballot, …)
    let pp := p_p1b H prop ap1.2 bc.1 bc.2 quorum_size
      num_participants dec.p1b
    -- ============ ghost layer: the pass contract, decomposed ============
    -- the sub-module contracts, at the body's own wires
    ghost have hhb := p_leader_heartbeat.ensures prop p_is_leader bc.1 dec.hb sched.hb
    ghost have hap1 := acceptor_p1.ensures acc prop
      ((Values L mem).batch p_to_acceptors_p1a dec.p1aBatch) a_log sched.ap1
    ghost have hpp := p_p1b.ensures prop ap1.2 bc.1 bc.2 quorum_size
      num_participants dec.p1b
    -- the merged P1a pool at an acceptor: per-proposer released streams
    ghost have hp1a : ∀ (j : Fin (mem acc)),
        p_to_acceptors_p1a j
          = ((List.finRange (mem prop)).map
            (fun r => (↑((p1a_out r).flatten)
              : Multiset (Ballot (mem prop))))).sum := fun j => rfl
    -- **a released ballot's source tick** (any variant): a release is its
    -- tick's own ballot, at a trigger-true tick — the block read at the
    -- tick, its step under `den`
    ghost have hsrc : ∀ (r : Fin (mem prop)) {b : Ballot (mem prop)},
        b ∈ (p1a_out r).flatten →
        ∃ (u : Nat), (bc.1 r)[u]? = some b ∧ (hb.2 r)[u]? = some true := by
      intro r b hbin
      obtain ⟨sub, hsub, hbs⟩ := List.mem_flatten.mp hbin
      obtain ⟨u, hu⟩ := List.mem_iff_getElem?.mp hsub
      obtain ⟨⟨b', trig⟩, hx, rfl⟩ := (hp1a_at r u sub).mp hu
      have hx' : (Trace.zip (bc.1 r) (hb.2 r))[u]? = some (b', trig) := hx
      obtain ⟨hb', htrig⟩ := Trace.getElem?_zip_eq_some'.mp hx'
      simp only [p1a_out_step, den] at hbs
      split at hbs
      next hc =>
        obtain rfl := List.mem_singleton.mp hbs
        exact ⟨u, hb', by rw [htrig, ((Bool.and_eq_true _ _).mp hc).1]⟩
      next => exact absurd hbs (List.not_mem_nil)
    -- **send-once** (B1, guarded): the released stream is duplicate-free —
    -- each release is `[]` or the tick's own ballot, and two fires never
    -- carry one ballot (the register's `once`, closed at the wires)
    ghost have hnodup : variant = .guarded → ∀ (r : Fin (mem prop)),
        ((p1a_out r).flatten).Nodup := by
      intro hvar r
      have hro : variant.sendOnce = true := by rw [hvar]; rfl
      -- a release batch is empty or its tick's ballot
      have hshape : ∀ (u : Nat) (e : List (Ballot (mem prop))), (p1a_out r)[u]? = some e →
          ∃ x : Ballot (mem prop) × Bool,
            (Trace.zip (bc.1 r) (hb.2 r))[u]? = some x ∧ (e = [] ∨ e = [x.1]) := by
        intro u e he
        obtain ⟨x, hx, rfl⟩ := (hp1a_at r u e).mp he
        refine ⟨x, hx, ?_⟩
        simp only [p1a_out_step, den]
        split
        · exact Or.inr rfl
        · exact Or.inl rfl
      have honce := (hp1a_inv r (p1a_out r).length r
        (fun x hx => hbc.own r x.1 (List.of_mem_zip hx).1)
        (Trace.pairwise_of_reads (fun x : Ballot (mem prop) × Bool => x.1)
          (fun _ x hx => (Trace.getElem?_zip_eq_some'.mp hx).1) (hmono r))).once hro
      rw [List.take_length] at honce
      rw [List.nodup_flatten]
      refine ⟨fun e he => ?_, ?_⟩
      · obtain ⟨u, hu⟩ := List.mem_iff_getElem?.mp he
        obtain ⟨x, -, h | h⟩ := hshape u e hu
        · rw [h]; exact List.nodup_nil
        · rw [h]; exact List.nodup_singleton _
      · rw [List.pairwise_iff_getElem]
        intro u v hu hv huv
        obtain ⟨xu, hxu, hsu⟩ := hshape u _ (List.getElem?_eq_getElem hu)
        obtain ⟨xv, hxv, hsv⟩ := hshape v _ (List.getElem?_eq_getElem hv)
        have hzu : (Trace.hist (Trace.zip (bc.1 r) (hb.2 r)) (p1a_out r))[u]?
            = some (xu, (p1a_out r)[u]'hu) :=
          List.getElem?_zip_eq_some.mpr ⟨hxu, List.getElem?_eq_getElem hu⟩
        have hzv : (Trace.hist (Trace.zip (bc.1 r) (hb.2 r)) (p1a_out r))[v]?
            = some (xv, (p1a_out r)[v]'hv) :=
          List.getElem?_zip_eq_some.mpr ⟨hxv, List.getElem?_eq_getElem hv⟩
        have hR := Trace.pairwise_reads honce huv hzu hzv
        intro a ha hb
        rcases hsu with h | h
        · rw [h] at ha; exact absurd ha (List.not_mem_nil)
        rcases hsv with h' | h'
        · rw [h'] at hb; exact absurd hb (List.not_mem_nil)
        rw [h] at ha hR
        rw [h'] at hb hR
        exact hR (List.cons_ne_nil _ _) (List.cons_ne_nil _ _)
          ((List.mem_singleton.mp ha).symm.trans (List.mem_singleton.mp hb))
    -- ==================== the wires, and the contract ====================
    -- p1b_fail_complete.complete(fail_ballots);
    -- p_to_proposers_i_am_leader_complete_cycle.complete(…);
    -- p_is_leader_complete_cycle.complete(p_is_leader.clone())
    complete (pp.2.2, hb.1, pp.1)
    -- ===== the closed knots: the Kleene flag-prefix fact =====
    -- ghost aliases: two knots and the generated pass body, open in
    -- their wire arguments
    ghost let fails := _root_.Hydro.leader_election.p1b_fail
      (P := P) (Values L mem) variant prop acc quorum_size
      num_participants dec sched p_received_p2b_ballots a_log
    ghost let iam := _root_.Hydro.leader_election.i_am_leader
      (P := P) (Values L mem) variant prop acc quorum_size
      num_participants dec sched p_received_p2b_ballots a_log
    ghost let pass := fun fl ia f =>
      _root_.Hydro.leader_election.body (P := P) (Values L mem)
        variant prop acc quorum_size num_participants dec sched
        p_received_p2b_ballots a_log fl ia f
    -- one Kleene step of the flag body preserves prefixes (the inner
    -- fail/gossip fixpoints coupled by `iterate_mono_param` through
    -- the pass's `MonoRel` instantiation)
    ghost have hstep : ∀ {a b : TickV (mem prop) Bool},
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
          (fun {u v} huv => (_root_.Hydro.leader_election.body
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
          (fun {x y} hxy => (_root_.Hydro.leader_election.body
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
      exact (_root_.Hydro.leader_election.body
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
        (R := fun (a b : TickV (mem prop) Bool) =>
          ∀ i, a i <+: b i)
        (F := fun f => (pass (fails (iam f) f) (iam f) f).2.2.1)
        (x := fun _ => [])
        (fun _ => List.nil_prefix)
        (fun {a b} hab => hstep hab)
        dec.fuelLead
    -- **ballot stability along a reign** (FINDINGS D21): consecutive leader
    -- ticks share the ballot — `p_p1b`'s fabricated-reign regress, under
    -- the solicitation discipline this loop supplies from its own wires
    ghost have hstable : 1 ≤ quorum_size → ∀ (i : Fin (mem prop)) {t : Nat}
        {b b' : Ballot (mem prop)},
        (pp.1 i)[t + 1]? = some true → (pp.1 i)[t]? = some true →
        (bc.1 i)[t + 1]? = some b' → (bc.1 i)[t]? = some b → b' = b := by
      intro hq1 i t b b' h1 h0 hb1 hb0
      -- the solicitation chain: an Ok reply pins a trigger-true
      -- follower tick of the sender's own run
      have hsol : ∀ m ∈ ap1.2 i,
          (∃ v, (m : P1b P (mem prop)).res = .ok v) →
          ∃ u : Nat, (bc.1 i)[u]? = some m.ballot ∧ (pp.1 i)[u]? = some false := by
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
        obtain ⟨u, hbu, htr⟩ := hsrc r hbin'
        -- ownership routes the release to the requester itself
        have hown_r : m.ballot.proposerId = r :=
          hbc.own r _ (Trace.mem_of_read hbu)
        have hri : r = i := by
          apply Fin.ext
          rw [← hroute, hown_r]
        subst hri
        -- the trigger gate reads a false flag off the cycle wire
        obtain ⟨hu2, htrig⟩ := List.getElem?_eq_some_iff.mp htr
        obtain ⟨hf, hff⟩ := hhb.trigger_gate r hu2 htrig
        -- the knot: the cycle input is a prefix of the output flags
        have hpre : p_is_leader r <+: pp.1 r := hknot r
        have hu_pp : u < (pp.1 r).length :=
          Nat.lt_of_lt_of_le hf hpre.length_le
        refine ⟨u, hbu, ?_⟩
        rw [List.getElem?_eq_getElem hu_pp, ← List.IsPrefix.getElem hpre hf, hff]
      -- discharge `p_p1b`'s requirements and read its stability face
      have req : PP1bRequires (mem prop) (ap1.2 i) (bc.1 i) (bc.2 i) (pp.1 i) i :=
        { has_largest := hbc.hasLargest_true i
          ballot_own := hbc.own i
          ballot_mono := hmono i
          solicited_at_follower := hsol }
      exact hpp.ballot_stable hq1 i req h1 h0 hb1 hb0
    -- **the reign of a ballot**: every leader tick carrying `b` sits in a
    -- contiguous stretch of `b`-ticks (ascent + ownership) whose first tick
    -- is fresh (did not lead the tick before — else, by stability, an
    -- earlier tick of the reign), before which no leader tick carried `b`
    ghost have hreign : 1 ≤ quorum_size → ∀ (i : Fin (mem prop)) {t : Nat}
        {b : Ballot (mem prop)},
        (pp.1 i)[t]? = some true → (bc.1 i)[t]? = some b →
        ∃ t₀, t₀ ≤ t ∧ (pp.1 i)[t₀]? = some true ∧ (bc.1 i)[t₀]? = some b
          ∧ (false :: pp.1 i)[t₀]? = some false
          ∧ (∀ u, t₀ ≤ u → u ≤ t → (bc.1 i)[u]? = some b)
          ∧ (∀ u, u < t₀ → (pp.1 i)[u]? = some true → (bc.1 i)[u]? ≠ some b) := by
      intro hq1 i t b hl hb
      obtain ⟨t₀, x₀, hle, hx₀, hp, hmin⟩ := Trace.first_tick
        (fun x : Ballot (mem prop) × Bool => x.2 && decide (x.1 = b))
        (l := Trace.zip (bc.1 i) (pp.1 i))
        (Trace.getElem?_zip_eq_some.mpr ⟨hb, hl⟩) (by simp)
      obtain ⟨hb₀, hl₀⟩ := Trace.getElem?_zip_eq_some'.mp hx₀
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hp
      obtain ⟨hl₀', hb₀'⟩ := hp
      rw [hl₀'] at hl₀
      rw [hb₀'] at hb₀
      -- minimality, in reads
      have hmin' : ∀ u, u < t₀ → (pp.1 i)[u]? = some true → (bc.1 i)[u]? ≠ some b := by
        intro u hu hlu hbu
        have := hmin u (b, true) hu (Trace.getElem?_zip_eq_some.mpr ⟨hbu, hlu⟩)
        simp at this
      refine ⟨t₀, hle, hl₀, hb₀, ?_, ?_, hmin'⟩
      · -- fresh: the tick before did not lead
        cases t₀ with
        | zero => rfl
        | succ k =>
          rw [Trace.getElem?_cons_succ']
          cases hk : (pp.1 i)[k]? with
          | none =>
            exfalso
            have := List.getElem?_eq_none_iff.mp hk
            have := Trace.read_lt hl₀
            omega
          | some lk =>
            cases lk with
            | false => rfl
            | true =>
              exfalso
              obtain ⟨bk, hbk⟩ : ∃ bk, (bc.1 i)[k]? = some bk :=
                ⟨_, List.getElem?_eq_getElem (Nat.lt_of_succ_lt (Trace.read_lt hb₀))⟩
              have := hstable hq1 i hl₀ hk hb₀ hbk
              exact hmin' k (Nat.lt_succ_self k) hk (by rw [hbk, this])
      · -- contiguity: between two ticks carrying `b`, every tick carries `b`
        intro u hu₀ hut
        obtain ⟨bu, hbu⟩ : ∃ bu, (bc.1 i)[u]? = some bu :=
          ⟨_, List.getElem?_eq_getElem (Nat.lt_of_le_of_lt hut (Trace.read_lt hb))⟩
        rw [hbu]
        congr 1
        rcases Nat.lt_or_eq_of_le hu₀ with h₀ | rfl
        · rcases Nat.lt_or_eq_of_le hut with h₁ | rfl
          · have h1 := Trace.pairwise_reads (hmono i) h₀ hb₀ hbu
            have h2 := Trace.pairwise_reads (hmono i) h₁ hbu hb
            refine Ballot.eq_of_num_owner (Nat.le_antisymm h2 h1) ?_
            rw [hbc.own i _ (Trace.mem_of_read hbu), hbc.own i _ (Trace.mem_of_read hb)]
          · exact Trace.read_inj hbu hb
        · exact Trace.read_inj hbu hb₀
    -- (p_ballot, p_is_leader, p_accepted_values, a_max_ballot)
    (bc.1, pp.1, pp.2.1, ap1.1)
    prove
      discipline := fun hq1 =>
        { own := fun i b hb => hbc.own i b hb
          mono := hmono
          lead_ne := fun i {t v} hpl hpr => by
            obtain ⟨bt, b, hbt, -, -, -, hcard, -⟩ := hpp.leader_batch i hpl
            obtain rfl := Trace.read_inj hpr hbt
            intro hzero
            have := hcard hq1
            rw [hzero, Multiset.card_zero] at this
            omega
          pinned := fun i {t t' b b' v v'} hpl hpl' hpb hpb' hnum hv hv' => by
            obtain rfl : b = b' := Ballot.eq_of_num_owner hnum (by
              rw [hbc.own i _ (Trace.mem_of_read hpb), hbc.own i _ (Trace.mem_of_read hpb')])
            have := hpp.pinned hq1 i hpl hpl' hpb hpb'
            rw [hv, hv'] at this
            exact Option.some.inj this
          reign := hreign hq1 },
      -- the ascent face, from the acceptor's contract
      max_mono := fun j => hap1.max_mono j,
      view_promise := by
        -- the leader's view opens to acceptor-tick promises
        intro i t ht htrue
        obtain ⟨bt, b, hbt, hb, -, hle, -, -⟩ := hpp.leader_batch i
          (by rw [List.getElem?_eq_getElem ht, htrue])
        obtain ⟨hpr, hbt'⟩ := List.getElem?_eq_some_iff.mp hbt
        obtain ⟨hpb, hb'⟩ := List.getElem?_eq_some_iff.mp hb
        refine ⟨hpr, hpb, ?_, ?_⟩
        · -- fullness
          rw [hbt']
          exact hle
        · -- each payload opens to an acceptor promise tick
          intro v hvmem
          rw [hbt'] at hvmem
          obtain ⟨b₀, hb₀, m, hm, hres, hball⟩ := hpp.accepted_src i hbt v hvmem
          obtain rfl := Trace.read_inj hb hb₀
          obtain ⟨j, tj, hbj, hmj, hlj, -, -, hshape⟩ :=
            hap1.reply_src i m hm
          by_cases hcond : some m.ballot = (ap1.1 j)[tj]'hmj
          · rw [if_pos hcond] at hshape
            rw [hres] at hshape
            have hveq : v = (a_log j)[tj]'hlj := by
              injection hshape
            refine ⟨j, tj, hlj, hveq, hmj, ?_⟩
            rw [← hcond, hball, hb']
          · rw [if_neg hcond] at hshape
            rw [hres] at hshape
            simp at hshape,
      providers := by
        -- the guarded send-once fan-in yields distinct acceptors
        intro hvar hq1 i t ht htrue
        subst hvar
        -- the leader batch: `quorum_size` logs at the tick's own ballot `b`,
        -- embedded with multiplicity in the Ok pool
        obtain ⟨bt, b, hbt, hb, -, -, hcard, hsub⟩ := hpp.leader_batch i
          (by rw [List.getElem?_eq_getElem ht, htrue])
        obtain ⟨hpr, hbt'⟩ := List.getElem?_eq_some_iff.mp hbt
        obtain ⟨hpb, hb'⟩ := List.getElem?_eq_some_iff.mp hb
        -- b is owned by `i`
        have hbown : b.proposerId = i := hbc.own i _ (Trace.mem_of_read hb)
        -- the pool decomposes by sending acceptor
        have hdec : ap1.2 i = ((List.finRange (mem acc)).map
            (fun j => ap1From (fun j' => batchCuts (p_to_acceptors_p1a j')
              0 (dec.p1aBatch j')) a_log j i)).sum :=
          hap1.reply_decomp i
        -- each ballot is on the wire at most once (send-once + ownership)
        have hcount_pool : ∀ (j : Fin (mem acc)),
            (p_to_acceptors_p1a j).count b ≤ 1 := by
          intro j
          rw [hp1a j, count_list_sum, List.map_map]
          refine sum_map_le_single (List.nodup_finRange _) _ i ?_ ?_
          · -- other proposers never carry `i`'s ballot
            intro r _ hri
            show Multiset.count b (↑((p1a_out r).flatten)) = 0
            rw [Multiset.count_eq_zero]
            intro hbin
            obtain ⟨u, hbu, -⟩ := hsrc r (Multiset.mem_coe.mp hbin)
            have : b.proposerId = r := hbc.own r _ (Trace.mem_of_read hbu)
            rw [hbown] at this
            exact hri this.symm
          · -- the owner releases it at most once (B1): the loop
            -- invariant's guarded-dedup clause over owned, ascending
            -- inputs
            show Multiset.count b (↑((p1a_out i).flatten)) ≤ 1
            rw [Multiset.coe_count]
            have hnd : ((p1a_out i).flatten).Nodup := hnodup rfl i
            exact List.nodup_iff_count_le_one.mp hnd _
        -- the per-sender Ok-at-b caps
        have hcaps : ∀ j ∈ List.finRange (mem acc),
            ((Multiset.filterMap p1bOkPair
              (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
                (dec.p1aBatch j')) a_log j i)).filter
              (fun x => x.1 = b)).card ≤ 1 := by
          intro j _
          rw [← Multiset.countP_eq_card_filter]
          refine Nat.le_trans (countP_filterMap_le p1bOkPair
            (fun x : Ballot (mem prop) × ALog P (mem prop) => x.1 = b)
            (fun m' : P1b P (mem prop) => m'.ballot = b)
            (fun a b' hfa hp => by
              obtain ⟨b', v⟩ := b'
              rw [(p1bOkPair_eq_some.mp hfa).1]
              exact hp) _) ?_
          rw [Multiset.countP_eq_card_filter]
          refine Nat.le_trans (hap1.from_ballot_cap j i _) ?_
          refine Nat.le_trans ?_ (hcount_pool j)
          have hle := batchCuts_sum_le (pool := p_to_acceptors_p1a j)
            (d := dec.p1aBatch j) (consumed := 0) (Multiset.zero_le _)
          rw [Multiset.zero_add] at hle
          exact Multiset.count_le_of_le _ hle
        -- distinct representatives from the unit caps
        have hle2 : bt.map (fun v => (b, v))
            ≤ ((List.finRange (mem acc)).map
              (fun j => (Multiset.filterMap p1bOkPair
                (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
                  (dec.p1aBatch j')) a_log j i)).filter
                (fun x => x.1 = b))).sum := by
          have hMfil : (bt.map (fun v => (b, v))).filter (fun x => x.1 = b)
              = bt.map (fun v => (b, v)) := by
            refine Multiset.filter_eq_self.mpr ?_
            intro x hx
            obtain ⟨v, -, rfl⟩ := Multiset.mem_map.mp hx
            rfl
          have h1 : bt.map (fun v => (b, v))
              ≤ (Multiset.filterMap p1bOkPair (ap1.2 i)).filter
                (fun x => x.1 = b) := by
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
            (fun x => x.1 = b))
          _ (List.nodup_finRange _) hle2 hcaps
        refine ⟨hpr, hpb, S, hSnd, ?_, ?_⟩
        · rw [Multiset.card_map, hcard hq1] at hScard
          exact hScard
        · intro j hj
          obtain ⟨x, hxM, hxq⟩ := hSrep j hj
          obtain ⟨v, hvq, rfl⟩ := Multiset.mem_map.mp hxM
          have hxq' : ((b, v) : Ballot (mem prop) × ALog P (mem prop))
              ∈ Multiset.filterMap p1bOkPair
                (ap1From (fun j' => batchCuts (p_to_acceptors_p1a j') 0
                  (dec.p1aBatch j')) a_log j i) :=
            Multiset.mem_of_le (Multiset.filter_le _ _) hxq
          obtain ⟨m', hm', hfm⟩ := (Multiset.mem_filterMap _ _).mp hxq'
          obtain ⟨hmb, hmres⟩ := p1bOkPair_eq_some.mp hfm
          obtain ⟨tj, hbj, hmj, hlj, -, -, hshape⟩ :=
            hap1.from_src j i m' hm'
          by_cases hcond : some m'.ballot = (ap1.1 j)[tj]'hmj
          · rw [if_pos hcond] at hshape
            rw [hmres] at hshape
            have hveq : v = (a_log j)[tj]'hlj := by
              injection hshape
            refine ⟨v, ?_, tj, hlj, hveq, hmj, ?_⟩
            · rw [hbt']
              exact hvq
            · rw [← hcond, hmb, hb']
          · rw [if_neg hcond] at hshape
            rw [hmres] at hshape
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
      TickV 1 (ALog Nat 1)))      -- a_log wire

#nondet_census leader_election (nondets := 8) (scheds := 3) (fuels := 3)

#guard leScenario.2.1 0 = [false, true]

#guard leScenario.1 0 = [Ballot.mk 0 0, Ballot.mk 0 0]

#guard leScenario.2.2.2 0 = [some (Ballot.mk 0 0),
  some (Ballot.mk 0 0)]


end Hydro
