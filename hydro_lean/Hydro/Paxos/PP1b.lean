import Hydro.MonoRel
import Hydro.Paxos.Types
import Hydro.Std.Quorum
import Hydro.HydroDef

/-!
# `p_p1b` (paxos.rs:527–593)

Proposer logic for processing P1bs: split the unordered reply pool into
`Ok` responses and fail ballots via the shared verified
`collect_quorum_with_response` stage (`Std/Quorum`), group the `Ok`
logs by ballot (`into_keyed`), realize a per-key order (`assume_ordering`
— Rust's `nondet!(/** We use flatten_unordered later */)`, a selection
decision over the pair stream), bucket up to `quorum_size` logs per
ballot (`fold_early_stop` — a key's entry is its first `quorum_size`
logs, in selection order), take `get_max_key`, snapshot at the proposer
tick (the *stale snapshot* `nondet!` — the keyed read's increment cuts;
staleness only delays leadership, paxos.rs:561–572), keep the quorum
only if it is for **our** ballot, and gate `p_is_leader` on
`p_has_largest_ballot`.

The accepted logs leave as **unordered** per-tick batches (Rust's
`flatten_unordered` type) — the selection order cannot leak. Fail
ballots are a pure `filter_map` of the raw pool, feeding the `p1b_fail`
cycle.

The contract (`PP1bEnsures`) is stated over the module's OUTPUT wires
(the leader flag, the accepted batches, the fail ballots) against its
inputs, read per tick; every fact — including the **fabricated-reign
regress** (FINDINGS D21: consecutive leader ticks keep the ballot) — is
proven INLINE as a ghost about the program's own wires, read at the
denotation. No pure twin of the pipeline, no bridge lemma (FINDINGS
D64/D66). The keyed chain is read through its closed forms (`Keyed.lean`,
D71): a view is a legal cumulative cut of the selected pairs, ascending
along ticks; per ballot it exposes a prefix of the ballot's selected
logs; a bucket exists iff `quorum_size` logs arrived at the ballot and is
their first `quorum_size` (`ofFoldEarlyStop_push_get_eq_some_iff`);
`get_max_key` answers the entry at the largest ballot
(`maxEntry_eq_some_iff`).
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-! ## Prerequisites for the contract face — the program starts at
`hydro def p_p1b` -/

/-! ### The program's closures (paxos.rs:553–580) -/

/-- `fold_early_stop`'s closure (paxos.rs:553–559): push into the
ballot's bucket, stop once `quorum_size` logs are collected. -/
abbrev p1bPush {nP : Nat} (quorumSize : Nat)
    (logs : List (ALog P nP)) (log : ALog P nP) : List (ALog P nP) × Bool :=
  (logs ++ [log], decide (quorumSize ≤ logs.length + 1))

/-- `.zip(p_ballot).filter_map(quorum_ballot == my_ballot)`
(paxos.rs:573–581) on `get_max_key`'s answer. -/
def pP1bQuorum {nP : Nat}
    (view : Option (Ballot nP × List (ALog P nP))) (myBallot : Ballot nP) :
    Option (List (ALog P nP)) :=
  match view with
  | some (qb, qlogs) => if qb = myBallot then some qlogs else none
  | none => none

/-- The `Ok`-response projection (`collect_quorum_with_response`'s
success leg, on the keyed reply). -/
def p1bOkPair {nP : Nat} (m : P1b P nP) :
    Option (Ballot nP × ALog P nP) :=
  cqOkProj (m.ballot, m.res)

/-- The keyed-reply pair's decidable equality, assembled explicitly
(the nested `Except`/`LogMap` shape exceeds the default instance
synthesis depth). -/
instance p1bPairDecEq {nP : Nat} :
    DecidableEq (Ballot nP × Except (Option (Ballot nP)) (ALog P nP)) :=
  instDecidableEqProd

/-! ### Reading the closures -/

/-- **The quorum dichotomy**: the gate's bucket is `some qlogs` exactly
when `get_max_key` answered my ballot with those logs. -/
theorem pP1bQuorum_eq_some_iff {nP : Nat}
    (view : Option (Ballot nP × List (ALog P nP))) (myBallot : Ballot nP)
    (qlogs : List (ALog P nP)) :
    pP1bQuorum view myBallot = some qlogs ↔ view = some (myBallot, qlogs) := by
  unfold pP1bQuorum
  cases view with
  | none => exact ⟨(fun h => by cases h), (fun h => by cases h)⟩
  | some r =>
    obtain ⟨rb, rlogs⟩ := r
    dsimp only
    constructor
    · intro h
      by_cases hrb : rb = myBallot
      · rw [if_pos hrb] at h
        rw [hrb, Option.some.inj h]
      · rw [if_neg hrb] at h
        cases h
    · intro h
      obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj h)
      rw [if_pos rfl]

/-- An `Ok` projection names the reply's ballot and log. -/
theorem p1bOkPair_eq_some {nP : Nat} {m : P1b P nP} {b : Ballot nP}
    {v : ALog P nP} :
    p1bOkPair m = some (b, v) ↔ m.ballot = b ∧ m.res = .ok v := by
  unfold p1bOkPair
  cases hres : m.res with
  | ok pl =>
    show some (m.ballot, pl) = some (b, v) ↔ _
    simp only [Option.some.injEq, Prod.mk.injEq, Except.ok.injEq]
  | error e =>
    show none = some (b, v) ↔ _
    simp only [reduceCtorEq, and_false]

/-! ### Requirements and the contract -/

/-- What `p_p1b` **requires** of its caller for ballot stability (the
fabricated-reign regress, FINDINGS D21), stated over its inputs and its
OWN leader flag `pl`: the election loop discharges these from its wires —
`p_has_largest_ballot` is identically true, ballots are owned and
`num`-monotone, and every `Ok`-promised ballot was **solicited at a
flag-`false` tick** (the feedback fact through the election cycle:
`p_to_acceptors_p1a` is gated on the trigger, which fires only at
non-leader ticks). -/
structure PP1bRequires (nP : Nat) (pool : Multiset (P1b P nP))
    (pb : Trace (Ballot nP)) (phl : Trace Bool) (pl : Trace Bool)
    (me : Fin nP) : Prop where
  /-- `p_has_largest_ballot` is identically `true` on realized ticks
  (`PBCEnsures.hasLargest_true`). -/
  has_largest : ∀ g ∈ phl, g = true
  /-- Every realized ballot is the member's own (`PBCEnsures.own`). -/
  ballot_own : ∀ b ∈ pb, (b : Ballot nP).proposerId = me
  /-- Ballot numbers only ascend along the tick trace. -/
  ballot_mono : List.Pairwise (fun (a b : Ballot nP) => a.num ≤ b.num) pb
  /-- **Solicitation staging**: every `Ok` promise in the reply pool was
  solicited at a realized tick that carried its ballot and read a
  `false` leader flag. -/
  solicited_at_follower : ∀ m ∈ pool, (∃ v, (m : P1b P nP).res = .ok v) →
    ∃ u : Nat, pb[u]? = some m.ballot ∧ pl[u]? = some false

/-- What `p_p1b` **ensures**, over the `Values` denotation, on its
output wires (`p_is_leader`, the accepted batches, `fail_ballots`)
against its inputs, read per tick. -/
structure PP1bEnsures (prop : L) (quorumSize : Nat)
    (pool : MemberMap (mem prop) (Multiset (P1b P (mem prop))))
    (pb : TickV (mem prop) (Ballot (mem prop)))
    (phl : TickV (mem prop) Bool)
    (out : TickV (mem prop) Bool
      × (MemberMap (mem prop) (Trace (Multiset (ALog P (mem prop)))))
      × (MemberMap (mem prop) (Multiset (Ballot (mem prop))))) : Prop where
  /-- **Accepted-log traceability**: every log in a tick's accepted
  batch was carried by an `Ok` P1b in the pool *at the tick's own
  ballot*. -/
  accepted_src : ∀ (i : Fin (mem prop)) {t : Nat}
    {bt : Multiset (ALog P (mem prop))},
    (out.2.1 i)[t]? = some bt → ∀ lg ∈ bt,
    ∃ b, (pb i)[t]? = some b ∧ ∃ m ∈ pool i, m.res = .ok lg ∧ m.ballot = b
  /-- **The leader batch**: at a leader tick, the accepted batch is a
  full quorum of `Ok` logs for the tick's own ballot (exactly
  `quorumSize` of them for `1 ≤ quorumSize`), `p_has_largest_ballot` is
  up, and the batch embeds **with multiplicity** in the pool's `Ok`
  projection (collected quorum entries are genuine promises). -/
  leader_batch : ∀ (i : Fin (mem prop)) {t : Nat},
    (out.1 i)[t]? = some true →
    ∃ bt b, (out.2.1 i)[t]? = some bt ∧ (pb i)[t]? = some b
      ∧ (phl i)[t]? = some true
      ∧ quorumSize ≤ Multiset.card bt
      ∧ (1 ≤ quorumSize → Multiset.card bt = quorumSize)
      ∧ bt.map (fun v => (b, v)) ≤ Multiset.filterMap p1bOkPair (pool i)
  /-- **Pinned quorum**: two leader ticks at one ballot read the same
  accepted batch — full buckets freeze. -/
  pinned : 1 ≤ quorumSize → ∀ (i : Fin (mem prop)) {t t' : Nat}
    {b : Ballot (mem prop)},
    (out.1 i)[t]? = some true → (out.1 i)[t']? = some true →
    (pb i)[t]? = some b → (pb i)[t']? = some b →
    (out.2.1 i)[t]? = (out.2.1 i)[t']?
  /-- **Ballot stability along reigns** (FINDINGS D21): under the
  solicitation discipline, consecutive leader ticks keep the ballot — a
  fabricated reign cannot bootstrap. -/
  ballot_stable : 1 ≤ quorumSize → ∀ (i : Fin (mem prop)),
    PP1bRequires (mem prop) (pool i) (pb i) (phl i) (out.1 i) i →
    ∀ {t : Nat} {b b' : Ballot (mem prop)},
    (out.1 i)[t + 1]? = some true → (out.1 i)[t]? = some true →
    (pb i)[t + 1]? = some b' → (pb i)[t]? = some b → b' = b
  /-- **Fail traceability**: every fail ballot quotes an `Err` P1b. -/
  fails_src : ∀ (i : Fin (mem prop)), ∀ b ∈ out.2.2 i,
    ∃ m ∈ pool i, m.res = .error (some b)

/-- `p_p1b`'s `nondet!` sites (paxos.rs:527–593), incl. the ack
consumption it hands to `collect_quorum_with_response`. -/
structure PP1bDec (H : HydroSem L mem) (nP : Nat) (P : Type)
    [DecidableEq P] where
  /-- `collect_quorum_with_response`'s `use::batch(responses,
  nondet!(…))` (quorum.rs:12) on the keyed replies. -/
  cqwr : H.BatchDec nP
    (Ballot nP × Except (Option (Ballot nP)) (ALog P nP)) .noOrder .exactlyOnce
  /-- `.assume_ordering::<TotalOrder>(nondet!(…))`: the
  quorum-collection consumption order. -/
  order : H.OrderSelDec nP (Ballot nP × ALog P nP)
  /-- `.get_max_key().snapshot(&proposer_tick, nondet!(stale ok))`: the
  keyed read's per-tick consumed increments of the selected pairs (legal
  iff, per ballot, the next prefix of the ballot's selected logs). -/
  snap : H.SnapDec nP (Ballot nP × ALog P nP) .noOrder

/-- **paxos.rs:527–593 `p_p1b`** over the proposer cluster `prop`.
Returns (`p_is_leader`, the accepted quorum logs as unordered per-tick
batches, `fail_ballots`). -/
hydro [p1bPairDecEq] def p_p1b (H : HydroSem L mem) (prop : L)
    (a_to_proposers_p1b :
      H.Stream prop (P1b P (mem prop)) .noOrder .exactlyOnce)
    (p_ballot : H.Atomic prop (Ballot (mem prop)))
    (p_has_largest_ballot : H.Atomic prop Bool)
    (quorum_size num_participants : Nat)
    (dec : PP1bDec H (mem prop) P) :
    (H.Atomic prop Bool
      × H.AtomicStream prop (ALog P (mem prop)) .noOrder .exactlyOnce
      × H.Stream prop (Ballot (mem prop)) .noOrder .exactlyOnce)
  ensures out => PP1bEnsures prop quorum_size
    a_to_proposers_p1b p_ballot p_has_largest_ballot out :=
  -- collect_quorum_with_response(a_to_proposers_p1b, quorum_size,
  --   num_quorum_participants) (paxos.rs:546) on the keyed replies
  let p1b_pairs := H.map a_to_proposers_p1b
    (fun _me m => (m.ballot, m.res))
  let cqwr := collect_quorum_with_response H prop p1b_pairs
    quorum_size num_participants dec.cqwr
  ghost have hcqwr := collect_quorum_with_response.ensures prop p1b_pairs
    quorum_size num_participants dec.cqwr
  -- **the realized success pool** embeds in the reply pool's `Ok`
  -- projection: the shared stage's `emit_pool_le` through the consumed
  -- batches (`cqOkProj ∘ (ballot, res)` IS `p1bOkPair`)
  ghost have hok_le : ∀ i, cqwr.1 i
      ≤ Multiset.filterMap p1bOkPair (a_to_proposers_p1b i) := fun i =>
    le_trans (hcqwr.emit_pool_le i)
      (le_trans (Multiset.filterMap_le_filterMap _ (cqConsumed_le _ _))
        (le_of_eq (by simp only [p1b_pairs, den, mapPool, Multiset.filterMap_map]; rfl)))
  -- .into_keyed().assume_ordering::<TotalOrder>(nondet!(…))
  let quorum_outs := H.kassume_ordering (H.into_keyed cqwr.1) dec.order
  -- .fold_early_stop(q!(Vec::new), q!(|logs, log| { logs.push(log);
  --   logs.len() >= quorum_size })): bucket per ballot up to quorum_size
  let folded := H.kfold_early_stop quorum_outs (p1bPush quorum_size) []
  -- .get_max_key().snapshot(proposer_tick, nondet!(stale ok))
  let views := H.snapshot (H.kget_max_key folded) dec.snap
  -- **the selected pairs**: the keyed source is the per-ballot grouping
  -- of ONE selection of the realized success pool
  ghost have hQ : ∀ i, quorum_outs i
      = KeyedPool.ofList (selectOrder (cqwr.1 i) (dec.order i)) := by
    intro i
    simp only [quorum_outs, den]
    exact congrArg KeyedPool.ofList
      (congrArg (selectOrder · (dec.order i)) (KeyedPool.toMultiset_ofMultiset _))
  -- **a view, read**: the tick's view is `get_max_key` over the buckets
  -- of a realized cumulative cut of the selected pairs
  ghost have hviews_at : ∀ (i : Fin (mem prop)) (t : Nat)
      (vw : Option (Ballot (mem prop) × List (ALog P (mem prop)))),
      (views i)[t]? = some vw ↔
        ∃ M, ((quorum_outs i).kviews 0 (dec.snap i))[t]? = some M
          ∧ vw = KeyedMap.maxEntry (KeyedMap.ofFoldEarlyStop (p1bPush quorum_size) []
              ((quorum_outs i).readAt M)) := by
    intro i t vw
    simp only [views, folded, den, List.map_map, Function.comp_def]
    exact Trace.getElem?_map_eq_some
  -- **a ballot's content at a view**: a prefix of the ballot's selected
  -- logs (`readAt` at the ordered grade)
  ghost have hrd : ∀ (i : Fin (mem prop)) (M : Multiset (Ballot (mem prop) × ALog P (mem prop)))
      (b : Ballot (mem prop)),
      ((quorum_outs i).readAt M).grp b <+: keyVals b (selectOrder (cqwr.1 i) (dec.order i)) := by
    intro i M b
    rw [hQ]
    exact KeyedPool.readAt_grp_prefix _ M b
  -- **a bucket at a view**: `b` has the entry `vs` iff `quorum_size`
  -- logs of `b` are in view and `vs` is their first `quorum_size`
  ghost have hbucket : ∀ (i : Fin (mem prop)) (M : Multiset (Ballot (mem prop) × ALog P (mem prop)))
      (b : Ballot (mem prop)) (vs : List (ALog P (mem prop))),
      (KeyedMap.ofFoldEarlyStop (p1bPush quorum_size) [] ((quorum_outs i).readAt M)).get b
          = some vs ↔
        max quorum_size 1 ≤ (((quorum_outs i).readAt M).grp b).length
          ∧ vs = (((quorum_outs i).readAt M).grp b).take (max quorum_size 1) :=
    fun i M b vs => KeyedMap.ofFoldEarlyStop_push_get_eq_some_iff quorum_size _ b vs
  -- **a selected log is a promise**: a log in view at `b` quotes an `Ok`
  -- reply at `b`
  ghost have hsel_src : ∀ (i : Fin (mem prop)) {M : Multiset (Ballot (mem prop) × ALog P (mem prop))}
      {b : Ballot (mem prop)} {v : ALog P (mem prop)},
      v ∈ ((quorum_outs i).readAt M).grp b →
      ∃ m ∈ a_to_proposers_p1b i, m.res = .ok v ∧ m.ballot = b := by
    intro i M b v hmem
    have hv : v ∈ keyVals b (selectOrder (cqwr.1 i) (dec.order i)) := (hrd i M b).subset hmem
    have hpool : ((b, v) : Ballot (mem prop) × ALog P (mem prop))
        ∈ Multiset.filterMap p1bOkPair (a_to_proposers_p1b i) :=
      Multiset.mem_of_le (hok_le i) (selectOrder_mem _ (mem_keyVals hv))
    obtain ⟨m, hm, hok⟩ := (Multiset.mem_filterMap _ _).mp hpool
    obtain ⟨hb, hres⟩ := p1bOkPair_eq_some.mp hok
    exact ⟨m, hm, hres, hb⟩
  let p_received_quorum_of_p1bs := H.mapTick (H.zipTick views p_ballot)
    (fun _me vb => pP1bQuorum vb.1 vb.2)
  -- p_is_leader = .is_some().and(p_has_largest_ballot)
  let p_is_leader := H.mapTick
    (H.zipTick p_received_quorum_of_p1bs p_has_largest_ballot)
    (fun _me ql => ql.1.isSome && ql.2)
  -- **the flag, read**: a flag tick reads the tick's view, ballot and
  -- `p_has_largest_ballot`
  ghost have hflag_at : ∀ (i : Fin (mem prop)) (t : Nat) (g : Bool),
      (p_is_leader i)[t]? = some g ↔
        ∃ vw b hl, (views i)[t]? = some vw ∧ (p_ballot i)[t]? = some b
          ∧ (p_has_largest_ballot i)[t]? = some hl
          ∧ g = ((pP1bQuorum vw b).isSome && hl) := by
    intro i t g
    simp only [p_is_leader, p_received_quorum_of_p1bs, den]
    constructor
    · intro h
      obtain ⟨⟨ql, hl⟩, hz, rfl⟩ := Trace.getElem?_map_eq_some.mp h
      obtain ⟨hq, hhl⟩ := Trace.getElem?_zip_eq_some.mp hz
      obtain ⟨⟨vw, b⟩, hvb, rfl⟩ := Trace.getElem?_map_eq_some.mp hq
      obtain ⟨hv, hb⟩ := Trace.getElem?_zip_eq_some.mp hvb
      exact ⟨vw, b, hl, hv, hb, hhl, rfl⟩
    · rintro ⟨vw, b, hl, hv, hb, hhl, rfl⟩
      exact Trace.getElem?_map_eq_some.mpr ⟨(pP1bQuorum vw b, hl),
        Trace.getElem?_zip_eq_some.mpr
          ⟨Trace.getElem?_map_eq_some.mpr ⟨(vw, b), Trace.getElem?_zip_eq_some.mpr ⟨hv, hb⟩, rfl⟩,
           hhl⟩, rfl⟩
  -- .flatten_unordered(): the selection order cannot leak
  let accepted_logs := H.flattenUnordered
    (H.mapTick p_received_quorum_of_p1bs (fun _me ql => ql.getD []))
  -- **the accepted batch, read**: the tick's gated quorum bucket, as a
  -- multiset
  ghost have haccept_at : ∀ (i : Fin (mem prop)) (t : Nat)
      (bt : Multiset (ALog P (mem prop))),
      (accepted_logs i)[t]? = some bt ↔
        ∃ vw b, (views i)[t]? = some vw ∧ (p_ballot i)[t]? = some b
          ∧ bt = Multiset.ofList ((pP1bQuorum vw b).getD []) := by
    intro i t bt
    simp only [accepted_logs, p_received_quorum_of_p1bs, den, List.map_map]
    constructor
    · intro h
      obtain ⟨⟨vw, b⟩, hvb, rfl⟩ := Trace.getElem?_map_eq_some.mp h
      obtain ⟨hv, hb⟩ := Trace.getElem?_zip_eq_some.mp hvb
      exact ⟨vw, b, hv, hb, rfl⟩
    · rintro ⟨vw, b, hv, hb, rfl⟩
      exact Trace.getElem?_map_eq_some.mpr ⟨(vw, b), Trace.getElem?_zip_eq_some.mpr ⟨hv, hb⟩, rfl⟩
  -- **a leader tick, opened**: the flag is up iff `get_max_key` answered
  -- the tick's own ballot — a full bucket, the first `quorum_size` of
  -- the ballot's logs in the tick's view — and `p_has_largest_ballot`
  -- holds; the accepted batch is that bucket
  ghost have hleader : ∀ (i : Fin (mem prop)) {t : Nat},
      (p_is_leader i)[t]? = some true →
      ∃ M b qlogs,
        ((quorum_outs i).kviews 0 (dec.snap i))[t]? = some M
        ∧ (p_ballot i)[t]? = some b ∧ (p_has_largest_ballot i)[t]? = some true
        ∧ (accepted_logs i)[t]? = some (Multiset.ofList qlogs)
        ∧ KeyedMap.maxEntry (KeyedMap.ofFoldEarlyStop (p1bPush quorum_size) []
            ((quorum_outs i).readAt M)) = some (b, qlogs)
        ∧ max quorum_size 1 ≤ (((quorum_outs i).readAt M).grp b).length
        ∧ qlogs = (((quorum_outs i).readAt M).grp b).take (max quorum_size 1) := by
    intro i t h
    obtain ⟨vw, b, hl, hv, hb, hhl, heq⟩ := (hflag_at i t true).mp h
    obtain ⟨M, hM, rfl⟩ := (hviews_at i t vw).mp hv
    obtain ⟨hsome, rfl⟩ := (Bool.and_eq_true _ _).mp heq.symm
    obtain ⟨qlogs, hq⟩ := Option.isSome_iff_exists.mp hsome
    have hmax := (pP1bQuorum_eq_some_iff _ _ _).mp hq
    obtain ⟨hget, -⟩ := (KeyedMap.maxEntry_eq_some_iff _ _ _).mp hmax
    obtain ⟨hfull, hvs⟩ := (hbucket i M b qlogs).mp hget
    refine ⟨M, b, qlogs, hM, hb, hhl, ?_, hmax, hfull, hvs⟩
    rw [haccept_at]
    exact ⟨_, b, (hviews_at i t _).mpr ⟨M, hM, rfl⟩, hb, by rw [hq]; rfl⟩
  -- **a full own bucket carries forward**: views ascend along ticks and
  -- a ballot's content at a view is a prefix of its content at any
  -- later view, so a full bucket reads identically later
  ghost have hcarry : ∀ (i : Fin (mem prop)) {t t' : Nat}
      {M M' : Multiset (Ballot (mem prop) × ALog P (mem prop))}
      {b : Ballot (mem prop)} {vs : List (ALog P (mem prop))},
      t ≤ t' →
      ((quorum_outs i).kviews 0 (dec.snap i))[t]? = some M →
      ((quorum_outs i).kviews 0 (dec.snap i))[t']? = some M' →
      (KeyedMap.ofFoldEarlyStop (p1bPush quorum_size) [] ((quorum_outs i).readAt M)).get b
        = some vs →
      (KeyedMap.ofFoldEarlyStop (p1bPush quorum_size) [] ((quorum_outs i).readAt M')).get b
        = some vs := by
    intro i t t' M M' b vs htt hM hM' hget
    have hle : M ≤ M' := KeyedPool.kviews_le _ 0 (dec.snap i) htt hM hM'
    obtain ⟨hfull, rfl⟩ := (hbucket i M b vs).mp hget
    have hpre : ((quorum_outs i).readAt M).grp b <+: ((quorum_outs i).readAt M').grp b := by
      rw [hQ]
      exact KeyedPool.readAt_grp_prefix_of_le _ hle b
    refine (hbucket i M' b _).mpr ⟨le_trans hfull hpre.length_le, ?_⟩
    exact (List.take_eq_of_prefix_of_le hpre hfull).symm
  -- **no flag-`false` tick sees its own ballot's bucket full** — the
  -- masking regress (FINDINGS D21): a mask needs a strictly higher full
  -- bucket, whose ballot was promised, hence solicited strictly later at
  -- a flag-false tick, where the mask carries forward; the ascent
  -- exhausts the trace
  ghost have hno_false_full : 1 ≤ quorum_size → ∀ (i : Fin (mem prop)),
      PP1bRequires (mem prop) (a_to_proposers_p1b i) (p_ballot i)
        (p_has_largest_ballot i) (p_is_leader i) i →
      ∀ (n u : Nat), (p_is_leader i).length - u ≤ n →
      (p_is_leader i)[u]? = some false →
      ∀ {M : Multiset (Ballot (mem prop) × ALog P (mem prop))}
        {b : Ballot (mem prop)} {vs : List (ALog P (mem prop))},
        ((quorum_outs i).kviews 0 (dec.snap i))[u]? = some M →
        (p_ballot i)[u]? = some b →
        (KeyedMap.ofFoldEarlyStop (p1bPush quorum_size) [] ((quorum_outs i).readAt M)).get b
          = some vs → False := by
    intro hq1 i req n
    induction n with
    | zero =>
      intro u hn hfalse _ _ _ _ _ _
      have := Trace.read_lt hfalse
      omega
    | succ n ih =>
      intro u hn hfalse M b vs hM hb hget
      -- the flag reads the view: `get_max_key` did not answer `b`
      obtain ⟨vw, b₀, hl, hv, hb₀, hhl, heq⟩ := (hflag_at i u false).mp hfalse
      obtain ⟨M₀, hM₀, rfl⟩ := (hviews_at i u vw).mp hv
      obtain rfl := Trace.read_inj hM hM₀
      obtain rfl := Trace.read_inj hb hb₀
      rw [req.has_largest hl (Trace.mem_of_read hhl), Bool.and_true] at heq
      -- the max entry exists and dominates ours, but is not ours
      obtain ⟨⟨rb, rlogs⟩, hmax, hble⟩ := KeyedMap.maxEntry_of_get hget
      have hne : rb ≠ b := by
        rintro rfl
        rw [(pP1bQuorum_eq_some_iff _ _ _).mpr hmax] at heq
        exact absurd heq (by simp)
      have hlt : b < rb := lt_of_le_of_ne hble (fun h => hne h.symm)
      obtain ⟨hrget, -⟩ := (KeyedMap.maxEntry_eq_some_iff _ _ _).mp hmax
      -- its ballot was promised, hence solicited at a flag-false tick
      obtain ⟨hrfull, hrvs⟩ := (hbucket i M rb rlogs).mp hrget
      obtain ⟨v, hvr⟩ := List.exists_mem_of_length_pos
        (Nat.lt_of_lt_of_le (Nat.lt_of_lt_of_le Nat.zero_lt_one (le_max_right _ _)) hrfull)
      obtain ⟨m, hm, hmres, hmb⟩ := hsel_src i hvr
      obtain ⟨u', hpbu', hflagu'⟩ := req.solicited_at_follower m hm ⟨v, hmres⟩
      rw [hmb] at hpbu'
      -- ownership forces a strict `num` ascent, so `u < u'`
      have hnum : b.num < rb.num := Ballot.num_lt_of_lt_of_owner hlt (by
        rw [req.ballot_own _ (Trace.mem_of_read hb), req.ballot_own _ (Trace.mem_of_read hpbu')])
      have huu' : u < u' := by
        rcases Nat.lt_trichotomy u u' with h | rfl | h
        · exact h
        · exact absurd (Trace.read_inj hb hpbu') (fun h => hne h.symm)
        · have := Trace.pairwise_reads req.ballot_mono h hpbu' hb
          omega
      -- the mask carries forward to its own solicitation tick
      obtain ⟨vw', b', -, hv', hb', -, -⟩ := (hflag_at i u' false).mp hflagu'
      obtain ⟨M', hM', rfl⟩ := (hviews_at i u' vw').mp hv'
      obtain rfl := Trace.read_inj hpbu' hb'
      -- recurse strictly later
      have hlen' := Trace.read_lt hflagu'
      exact ih u' (by omega) hflagu' hM' hpbu'
        (hcarry i (Nat.le_of_lt huu') hM hM' hrget)
  -- fails.flat_map_ordered(q!(|(_, ballot)| ballot))
  let fail_ballots := H.filterMap cqwr.2 (fun _me ke => ke.2)
  (p_is_leader, accepted_logs, fail_ballots)
  prove
    accepted_src := fun i {t} {bt} hbt lg hlg => by
      obtain ⟨vw, b, hv, hb, rfl⟩ := (haccept_at i t bt).mp hbt
      obtain ⟨M, hM, rfl⟩ := (hviews_at i t vw).mp hv
      refine ⟨b, hb, ?_⟩
      -- the batch is nonempty, so the gate answered with `b`'s bucket
      cases hq : pP1bQuorum (KeyedMap.maxEntry (KeyedMap.ofFoldEarlyStop (p1bPush quorum_size) []
          ((quorum_outs i).readAt M))) b with
      | none => rw [hq] at hlg; cases hlg
      | some qlogs =>
        rw [hq] at hlg
        obtain ⟨hget, -⟩ := (KeyedMap.maxEntry_eq_some_iff _ _ _).mp
          ((pP1bQuorum_eq_some_iff _ _ _).mp hq)
        obtain ⟨-, rfl⟩ := (hbucket i M b qlogs).mp hget
        exact hsel_src i (List.mem_of_mem_take (Multiset.mem_coe.mp hlg)),
    leader_batch := fun i {t} h => by
      obtain ⟨M, b, qlogs, hM, hb, hhl, hacc, -, hfull, rfl⟩ := hleader i h
      refine ⟨_, b, hacc, hb, hhl, ?_, ?_, ?_⟩
      · rw [Multiset.coe_card, List.length_take]
        exact le_trans (le_max_left _ _) (Nat.le_min.mpr ⟨le_rfl, hfull⟩)
      · intro hq1
        rw [Multiset.coe_card, List.length_take, Nat.max_eq_left hq1]
        rw [Nat.max_eq_left hq1] at hfull
        omega
      · -- the bucket, re-keyed, is a sublist of the selected pairs, hence
        -- of the selected pool, hence of the `Ok` projection
        rw [Multiset.map_coe]
        have hsub : List.Sublist
            (((((quorum_outs i).readAt M).grp b).take (max quorum_size 1)).map (fun v => (b, v)))
            (selectOrder (cqwr.1 i) (dec.order i)) :=
          (((List.take_sublist _ _).trans (hrd i M b).sublist).map _).trans
            (keyVals_map_pair_sublist b _)
        exact le_trans (Multiset.coe_le.mpr hsub.subperm)
          (le_trans selectOrder_subpool (hok_le i)),
    pinned := fun hq1 i {t t'} {b} ht ht' hb hb' => by
      obtain ⟨M, b₁, qlogs, hM, hb₁, -, hacc, hmax, -, -⟩ := hleader i ht
      obtain ⟨M', b₂, qlogs', hM', hb₂, -, hacc', hmax', -, -⟩ := hleader i ht'
      obtain rfl := Trace.read_inj hb hb₁
      obtain rfl := Trace.read_inj hb' hb₂
      rw [hacc, hacc']
      obtain ⟨hget, -⟩ := (KeyedMap.maxEntry_eq_some_iff _ _ _).mp hmax
      obtain ⟨hget', -⟩ := (KeyedMap.maxEntry_eq_some_iff _ _ _).mp hmax'
      rcases Nat.le_total t t' with hle | hle
      · rw [Option.some.inj ((hcarry i hle hM hM' hget).symm.trans hget')]
      · rw [Option.some.inj ((hcarry i hle hM' hM hget').symm.trans hget)],
    ballot_stable := fun hq1 i req {t} {b b'} h1 h0 hb1 hb0 => by
      by_contra hne
      -- `num`s ascend; distinct own ballots ascend strictly
      have hmono : b.num ≤ b'.num :=
        Trace.pairwise_reads req.ballot_mono (Nat.lt_succ_self t) hb0 hb1
      have hnum : b.num < b'.num := by
        rcases Nat.lt_or_ge b.num b'.num with hlt | hge
        · exact hlt
        · exact absurd (Ballot.eq_of_num_owner (by omega) (by
            rw [req.ballot_own _ (Trace.mem_of_read hb1),
              req.ballot_own _ (Trace.mem_of_read hb0)])) hne
      -- the reign's quorum is realized at `t + 1`: a full own bucket …
      obtain ⟨M, b₁, qlogs, hM, hb₁, -, -, hmax, hfull, rfl⟩ := hleader i h1
      obtain rfl := Trace.read_inj hb1 hb₁
      obtain ⟨hget, -⟩ := (KeyedMap.maxEntry_eq_some_iff _ _ _).mp hmax
      -- … whose ballot was promised, hence solicited flag-false
      obtain ⟨v, hvq⟩ := List.exists_mem_of_length_pos
        (Nat.lt_of_lt_of_le (Nat.lt_of_lt_of_le hq1 (le_max_left _ _)) hfull)
      obtain ⟨m, hm, hmres, hmb⟩ := hsel_src i hvq
      obtain ⟨u, hpbu, hflagu⟩ := req.solicited_at_follower m hm ⟨v, hmres⟩
      rw [hmb] at hpbu
      -- the solicitation is not before the reign, and not at it: strictly
      -- after — the masking regress kills it
      have hut : t + 1 < u := by
        rcases Nat.lt_trichotomy u (t + 1) with h | rfl | h
        · exfalso
          rcases Nat.lt_or_eq_of_le (Nat.le_of_lt_succ h) with h' | rfl
          · have := Trace.pairwise_reads req.ballot_mono h' hpbu hb0
            omega
          · exact hne (Trace.read_inj hpbu hb0)
        · rw [h1] at hflagu
          cases hflagu
        · exact h
      obtain ⟨vw', b₂, -, hv', hb₂, -, -⟩ := (hflag_at i u false).mp hflagu
      obtain ⟨M', hM', rfl⟩ := (hviews_at i u vw').mp hv'
      obtain rfl := Trace.read_inj hpbu hb₂
      exact hno_false_full hq1 i req _ u le_rfl hflagu hM' hpbu
        (hcarry i (Nat.le_of_lt hut) hM hM' hget),
    fails_src := fun i b hb => by
      -- through the shared stage's pure error leg
      have hb' : b ∈ Multiset.filterMap (fun ke : Ballot (mem prop) × Option (Ballot (mem prop)) => ke.2)
          (Multiset.filterMap cqErrProj
            (Multiset.map (fun m : P1b P (mem prop) => (m.ballot, m.res))
              (a_to_proposers_p1b i))) := hb
      obtain ⟨ke, hke, hke2⟩ := (Multiset.mem_filterMap _ _).mp hb'
      obtain ⟨pr, hpr, hprE⟩ := (Multiset.mem_filterMap _ _).mp hke
      obtain ⟨m, hm, rfl⟩ := Multiset.mem_map.mp hpr
      refine ⟨m, hm, ?_⟩
      cases hr : m.res with
      | ok pl =>
        rw [hr] at hprE
        cases hprE
      | error e =>
        rw [hr] at hprE
        obtain rfl := Option.some.inj hprE
        rw [← hke2]

/-! ## Executable smoke tests -/

-- One proposer, quorum of 1: the Ok p1b at my ballot elects; the
-- accepted batch carries its log; the Err feeds the fail stream.
#guard_eval (p_p1b (Values PaxLoc (paxMem 1 1)) .prop
    (.mk' fun _ => ({⟨Ballot.mk 0 0, .ok (none, HMap.ofList [])⟩} : Multiset (P1b Nat 1)))
    (.mk' fun _ => [Ballot.mk 0 0]) (.mk' fun _ => [true]) 1 1
    ⟨fun _ => [{(Ballot.mk 0 0, .ok (none, HMap.ofList []))}],
     fun _ => [(Ballot.mk 0 0, (none, HMap.ofList []))], fun _ => [{(Ballot.mk 0 0, (none, HMap.ofList []))}]⟩).1 0
  = [true]

#guard_eval (p_p1b (Values PaxLoc (paxMem 1 1)) .prop
    (.mk' fun _ => ({⟨Ballot.mk 0 0, .ok (none, HMap.ofList [])⟩} : Multiset (P1b Nat 1)))
    (.mk' fun _ => [Ballot.mk 0 0]) (.mk' fun _ => [true]) 1 1
    ⟨fun _ => [{(Ballot.mk 0 0, .ok (none, HMap.ofList []))}],
     fun _ => [(Ballot.mk 0 0, (none, HMap.ofList []))], fun _ => [{(Ballot.mk 0 0, (none, HMap.ofList []))}]⟩).2.1 0
  = [({((none : Option Nat), (HMap.ofList [] : LogMap Nat 1))} : Multiset (ALog Nat 1))]

#guard_eval (p_p1b (Values PaxLoc (paxMem 1 1)) .prop
    (.mk' fun _ => ({⟨Ballot.mk 0 0, .error (some (Ballot.mk 5 0))⟩}
      : Multiset (P1b Nat 1)))
    (.mk' fun _ => [Ballot.mk 0 0]) (.mk' fun _ => [true]) 1 1
    ⟨fun _ => [], fun _ => [], fun _ => [{}]⟩).2.2 0
  = {Ballot.mk 5 0}

-- A stale ballot's quorum does not elect (quorum for someone else).
#guard_eval (p_p1b (Values PaxLoc (paxMem 1 1)) .prop
    (.mk' fun _ => ({⟨Ballot.mk 7 0, .ok (none, HMap.ofList [])⟩} : Multiset (P1b Nat 1)))
    (.mk' fun _ => [Ballot.mk 0 0]) (.mk' fun _ => [true]) 1 1
    ⟨fun _ => [{(Ballot.mk 7 0, .ok (none, HMap.ofList []))}],
     fun _ => [(Ballot.mk 7 0, (none, HMap.ofList []))], fun _ => [{(Ballot.mk 7 0, (none, HMap.ofList []))}]⟩).1 0
  = [false]

#nondet_census p_p1b (nondets := 3) (scheds := 0) (fuels := 0)

end Hydro
