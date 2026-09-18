import HydroV2.Paxos.PP1bLemmas

/-!
# `p_p1b` (paxos.rs:527–593)

Proposer logic for processing P1bs: split the unordered reply pool into
`Ok` responses and fail ballots (`collect_quorum_with_response`), bucket
the `Ok` logs per ballot up to `quorum_size` (`fold_early_stop` — the
bucketing consumes the pool through an **`assume_ordering` selection
decision**, Rust's `nondet!(/** We use flatten_unordered later */)`),
take `get_max_key`, snapshot at the proposer tick (the *stale snapshot*
`nondet!` — a prefix-cut decision; staleness only delays leadership,
paxos.rs:561–572), keep the quorum only if it is for **our** ballot, and
gate `p_is_leader` on `p_has_largest_ballot`.

The accepted logs leave as **unordered** per-tick batches (Rust's
`flatten_unordered` type) — the selection order cannot leak. Fail
ballots are a pure `filter_map` of the raw pool, feeding the `p1b_fail`
cycle.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- `p_p1b`'s `nondet!` sites (paxos.rs:527–593). -/
structure PP1bDec (nP : Nat) (P : Type) [DecidableEq P] where
  /-- `.assume_ordering::<TotalOrder>(nondet!(…))`: the
  quorum-collection consumption order. -/
  order : OrderSelection nP (Ballot nP × ALog P nP)
  /-- `.get_max_key().snapshot(&proposer_tick, nondet!(stale ok))`:
  prefix cuts of the ordered quorum outs. -/
  snap : SnapshotCuts nP (Ballot nP × ALog P nP) .totalOrder

/-- What `p_p1b` **ensures**, over the `Values` denotation. -/
structure PP1bEnsures (prop : L) (quorumSize : Nat)
    (pool : Fin (mem prop) → Multiset (P1b P (mem prop)))
    (pb : TickV (mem prop) (Ballot (mem prop)) .unbounded)
    (phl : TickV (mem prop) Bool .unbounded)
    (dOrd : OrderSelection (mem prop)
      (Ballot (mem prop) × ALog P (mem prop)))
    (dSnap : SnapshotCuts (mem prop)
      (Ballot (mem prop) × ALog P (mem prop)) .totalOrder)
    (out : TickV (mem prop) Bool .unbounded
      × (Fin (mem prop) → Trace (Multiset (ALog P (mem prop))))
      × (Fin (mem prop) → Multiset (Ballot (mem prop)))) : Prop where
  /-- **The leader gate**: `p_is_leader` at a tick requires a full
  quorum of `Ok` responses *for the tick's own ballot* in the tick's
  realized view, and `p_has_largest_ballot`. -/
  leader_gate : ∀ (i : Fin (mem prop)) {t : Nat}
    (ht : t < (out.1 i).length), (out.1 i)[t]'ht = true →
    ∃ (hv : t < (pP1bViews (mem prop) quorumSize (pool i) (dOrd i)
        (dSnap i)).length)
      (hb : t < (pb i).length) (hl : t < (phl i).length)
      (qlogs : List (ALog P (mem prop))),
      pP1bQuorum quorumSize
          ((pP1bViews (mem prop) quorumSize (pool i) (dOrd i)
            (dSnap i))[t]'hv) ((pb i)[t]'hb)
        = some qlogs
      ∧ quorumSize ≤ qlogs.length
      ∧ (phl i)[t]'hl = true
  /-- **Accepted-log traceability**: every log in a tick's accepted
  batch was carried by an `Ok` P1b in the pool *at the tick's own
  ballot*. -/
  accepted_src : ∀ (i : Fin (mem prop)) {t : Nat}
    (ht : t < (out.2.1 i).length), ∀ lg ∈ (out.2.1 i)[t]'ht,
    ∃ (hb : t < (pb i).length), ∃ m ∈ pool i,
      m.res = .ok lg ∧ m.ballot = (pb i)[t]'hb
  /-- **Fail traceability**: every fail ballot quotes an `Err` P1b. -/
  fails_src : ∀ (i : Fin (mem prop)), ∀ b ∈ out.2.2 i,
    ∃ m ∈ pool i, m.res = .error (some b)
  /-- **The flag face**: the leader-flag output IS the pure flag trace
  (the vocabulary `pP1b_ballot_stable` / `pP1b_no_false_full` and the
  solicitation requirement speak). -/
  flags_eq : ∀ (i : Fin (mem prop)),
    out.1 i = pP1bFlags (mem prop) quorumSize (pool i) (pb i) (phl i)
      (dOrd i) (dSnap i)
  /-- **The accepted-batch face**: per tick, the batch is the gated
  quorum bucket (as its multiset). -/
  accepted_eq : ∀ (i : Fin (mem prop)),
    out.2.1 i = List.map (fun l => Multiset.ofList l)
      ((((pP1bViews (mem prop) quorumSize (pool i) (dOrd i)
          (dSnap i)).zip (pb i)).map
        (fun vb => pP1bQuorum quorumSize vb.1 vb.2)).map
        (fun ql => ql.getD []))

/-- **paxos.rs:527–593 `p_p1b`** over the proposer cluster `prop`.
Returns (`p_is_leader`, the accepted quorum logs as unordered per-tick
batches, `fail_ballots`). -/
def p_p1b (H : HydroSem L mem) (prop : L)
    (a_to_proposers_p1b :
      H.Stream prop (P1b P (mem prop)) .noOrder .exactlyOnce)
    (p_ballot : H.TickSingleton prop (Ballot (mem prop)) .unbounded)
    (p_has_largest_ballot : H.TickSingleton prop Bool .unbounded)
    (quorum_size : Nat)
    (dec : PP1bDec (mem prop) P) :
    {out : H.TickSingleton prop Bool .unbounded
      × H.TickStream prop (ALog P (mem prop)) .noOrder .exactlyOnce
      × H.Stream prop (Ballot (mem prop)) .noOrder .exactlyOnce //
      ∀ hv : H = Values L mem,
        match H, hv, a_to_proposers_p1b, p_ballot,
            p_has_largest_ballot, out with
        | _, rfl, pool, pb, phl, o =>
          PP1bEnsures prop quorum_size pool pb phl dec.order dec.snap o} :=
  -- collect_quorum_with_response: the error leg (pure filter_map)
  let fail_ballots := H.filterMap a_to_proposers_p1b
    (fun _me m =>
      match m.res with
      | .error (some b) => some b
      | _ => none)
  -- … and the success leg
  let oks := H.filterMap a_to_proposers_p1b (fun _me => p1bOkPair)
  -- .into_keyed().assume_ordering::<TotalOrder>(nondet!(…))
  let quorum_outs := H.assume_ordering oks dec.order
  -- .fold_early_stop(…): bucket per ballot up to quorum_size
  let folded := H.fold
    (fun acc bv => p1bLogsInsert quorum_size acc bv.1 bv.2) []
    (by trivial) quorum_outs
  -- .get_max_key().snapshot(proposer_tick, nondet!(stale ok)).zip(p_ballot)
  --   .filter_map(quorum_ballot == my_ballot)
  let views := H.snapshot folded dec.snap
  let p_received_quorum_of_p1bs := H.mapTick (H.zipTick views p_ballot)
    (fun _me vb => pP1bQuorum quorum_size vb.1 vb.2)
  -- p_is_leader = .is_some().and(p_has_largest_ballot)
  let p_is_leader := H.mapTick
    (H.zipTick p_received_quorum_of_p1bs p_has_largest_ballot)
    (fun _me ql => ql.1.isSome && ql.2)
  ⟨(p_is_leader,
    -- .flatten_unordered(): the selection order cannot leak
    H.emitBatchesUnordered
      (H.mapTick p_received_quorum_of_p1bs (fun _me ql => ql.getD [])),
    fail_ballots), by
  intro hv; subst hv
  constructor
  · -- the leader gate
    intro i t ht htrue
    have h1 : ((Trace.zip
        ((Trace.zip (pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
            (dec.snap i)) (p_ballot i)).map
          (fun vb => pP1bQuorum quorum_size vb.1 vb.2)) (p_has_largest_ballot i)).map
        (fun ql => ql.1.isSome && ql.2))[t]'ht = true := htrue
    have ht2 : t < ((Trace.zip
        ((Trace.zip (pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
            (dec.snap i)) (p_ballot i)).map
          (fun vb => pP1bQuorum quorum_size vb.1 vb.2)) (p_has_largest_ballot i)).map
        (fun ql => ql.1.isSome && ql.2)).length := ht
    have hzlen : t < (Trace.zip
        ((Trace.zip (pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
            (dec.snap i)) (p_ballot i)).map
          (fun vb => pP1bQuorum quorum_size vb.1 vb.2)) (p_has_largest_ballot i)).length := by
      rwa [List.length_map] at ht2
    have hlen := hzlen
    simp only [Trace.zip, List.length_zip, List.length_map] at hlen
    have hv : t < (pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
        (dec.snap i)).length := by omega
    have hb : t < (p_ballot i).length := by omega
    have hl : t < (p_has_largest_ballot i).length := by omega
    simp only [Trace.zip, List.getElem_map, List.getElem_zip,
      Bool.and_eq_true] at h1
    obtain ⟨hsome, hphl⟩ := h1
    obtain ⟨qlogs, hq⟩ := Option.isSome_iff_exists.mp hsome
    refine ⟨hv, hb, hl, qlogs, hq, ?_, hphl⟩
    -- the quorum size rides get_max_key
    unfold pP1bQuorum at hq
    cases hmax : p1bMaxQuorumBallot quorum_size
        ((pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
          (dec.snap i))[t]'hv) with
    | none =>
      rw [hmax] at hq
      dsimp only at hq
      cases hq
    | some r =>
      obtain ⟨rb, rlogs⟩ := r
      rw [hmax] at hq
      dsimp only at hq
      by_cases hrb : rb = (p_ballot i)[t]'hb
      · rw [if_pos hrb] at hq
        injection hq with hq'
        rw [← hq']
        exact p1bMaxQuorumBallot_holds_quorum quorum_size _ hmax
      · rw [if_neg hrb] at hq
        cases hq
  · -- accepted-log traceability
    intro i t ht lg hlg
    have hb0 : t < (List.map (fun l => Multiset.ofList l)
        ((((pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
          (dec.snap i)).zip (p_ballot i)).map
          (fun vb => pP1bQuorum quorum_size vb.1 vb.2)).map
          (fun ql => ql.getD []))).length := ht
    have hlen := hb0
    simp only [Trace.zip] at hlen
    rw [List.length_map, List.length_map, List.length_map,
      List.length_zip] at hlen
    have hv : t < (pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
        (dec.snap i)).length := by omega
    have hb : t < (p_ballot i).length := by omega
    refine ⟨hb, ?_⟩
    have hlg' : lg ∈ Multiset.ofList
        ((pP1bQuorum quorum_size
          ((pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
            (dec.snap i))[t]'hv) ((p_ballot i)[t]'hb)).getD []) := by
      have h0 : lg ∈ (List.map (fun l => Multiset.ofList l)
        ((((pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
            (dec.snap i)).zip (p_ballot i)).map
          (fun vb => pP1bQuorum quorum_size vb.1 vb.2)).map
          (fun ql => ql.getD [])))[t]'hb0 := hlg
      simp only [Trace.zip] at h0
      rw [List.getElem_map, List.getElem_map, List.getElem_map] at h0
      rw [List.getElem_zip] at h0
      exact h0
    have hlg2 : lg ∈ (pP1bQuorum quorum_size
        ((pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
          (dec.snap i))[t]'hv) ((p_ballot i)[t]'hb)).getD [] :=
      Multiset.mem_coe.mp hlg'
    -- the quorum came from the tick's own ballot's bucket
    cases hq : pP1bQuorum quorum_size
        ((pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
          (dec.snap i))[t]'hv) ((p_ballot i)[t]'hb) with
    | none =>
      rw [hq] at hlg2
      cases hlg2
    | some qlogs =>
      rw [hq] at hlg2
      unfold pP1bQuorum at hq
      cases hmax : p1bMaxQuorumBallot quorum_size
          ((pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
            (dec.snap i))[t]'hv) with
      | none =>
        rw [hmax] at hq
        dsimp only at hq
        cases hq
      | some r =>
        obtain ⟨rb, rlogs⟩ := r
        rw [hmax] at hq
        dsimp only at hq
        by_cases hrb : rb = (p_ballot i)[t]'hb
        · rw [if_pos hrb] at hq
          injection hq with hq'
          subst hq'
          -- the bucket's payloads trace to selected quorum outputs
          have hmem := p1bMaxQuorumBallot_mem_self hmax
          have hview : (pP1bViews (mem prop) quorum_size (a_to_proposers_p1b i) (dec.order i)
              (dec.snap i))[t]'hv
              ∈ (prefixCuts (selectOrder (Multiset.filterMap p1bOkPair
                  (a_to_proposers_p1b i)) (dec.order i)) 0 (dec.snap i)).map
                (fun sel => foldEarlyStopBallots quorum_size sel) :=
            List.getElem_mem hv
          obtain ⟨sel, hsel, hfold⟩ := List.mem_map.mp hview
          rw [← hfold] at hmem
          have hpair := foldEarlyStop_src quorum_size sel (rb, rlogs)
            hmem lg hlg2
          -- selected prefix → selection → pool
          obtain ⟨k, hk⟩ := prefixCuts_mem_take hsel
          rw [hk] at hpair
          have hsel2 : ((rb, lg) : Ballot (mem prop) × ALog P (mem prop))
              ∈ selectOrder (Multiset.filterMap p1bOkPair (a_to_proposers_p1b i))
                (dec.order i) :=
            (List.take_sublist ..).subset hpair
          have hpool := selectOrder_mem _ hsel2
          obtain ⟨m, hm, hok⟩ := (Multiset.mem_filterMap _ _).mp hpool
          refine ⟨m, hm, ?_, ?_⟩
          · unfold p1bOkPair at hok
            cases hres : m.res with
            | ok pl =>
              rw [hres] at hok
              injection hok with hok'
              have : pl = lg := congrArg Prod.snd hok'
              rw [this]
            | error e =>
              rw [hres] at hok
              cases hok
          · unfold p1bOkPair at hok
            cases hres : m.res with
            | ok pl =>
              rw [hres] at hok
              injection hok with hok'
              have : m.ballot = rb := congrArg Prod.fst hok'
              rw [this, hrb]
            | error e =>
              rw [hres] at hok
              cases hok
        · rw [if_neg hrb] at hq
          cases hq
  · -- fail traceability
    intro i b hb
    have hb' : b ∈ Multiset.filterMap
        (fun m : P1b P (mem prop) =>
          match m.res with
          | .error (some b) => some b
          | _ => none) (a_to_proposers_p1b i) := hb
    obtain ⟨m, hm, hres⟩ := (Multiset.mem_filterMap _ _).mp hb'
    refine ⟨m, hm, ?_⟩
    cases hr : m.res with
    | ok pl =>
      rw [hr] at hres
      cases hres
    | error e =>
      cases e with
      | none =>
        rw [hr] at hres
        cases hres
      | some b' =>
        rw [hr] at hres
        injection hres with hres'
        rw [hres']
  · -- the flag face
    intro i
    rfl
  · -- the accepted-batch face
    intro i
    rfl⟩

/-! ## Executable smoke tests -/

-- One proposer, quorum of 1: the Ok p1b at my ballot elects; the
-- accepted batch carries its log; the Err feeds the fail stream.
#guard (p_p1b (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => ({⟨Ballot.mk 0 0, .ok (none, [])⟩} : Multiset (P1b Nat 1)))
    (fun _ => [Ballot.mk 0 0]) (fun _ => [true]) 1
    ⟨fun _ => [(Ballot.mk 0 0, (none, []))], fun _ => [1]⟩).val.1 0
  = [true]

#guard (p_p1b (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => ({⟨Ballot.mk 0 0, .ok (none, [])⟩} : Multiset (P1b Nat 1)))
    (fun _ => [Ballot.mk 0 0]) (fun _ => [true]) 1
    ⟨fun _ => [(Ballot.mk 0 0, (none, []))], fun _ => [1]⟩).val.2.1 0
  = [({((none : Option Nat), ([] : LogMap Nat 1))} : Multiset (ALog Nat 1))]

#guard (p_p1b (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => ({⟨Ballot.mk 0 0, .error (some (Ballot.mk 5 0))⟩}
      : Multiset (P1b Nat 1)))
    (fun _ => [Ballot.mk 0 0]) (fun _ => [true]) 1
    ⟨fun _ => [], fun _ => [0]⟩).val.2.2 0
  = {Ballot.mk 5 0}

-- A stale ballot's quorum does not elect (quorum for someone else).
#guard (p_p1b (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => ({⟨Ballot.mk 7 0, .ok (none, [])⟩} : Multiset (P1b Nat 1)))
    (fun _ => [Ballot.mk 0 0]) (fun _ => [true]) 1
    ⟨fun _ => [(Ballot.mk 7 0, (none, []))], fun _ => [1]⟩).val.1 0
  = [false]

end HydroV2
