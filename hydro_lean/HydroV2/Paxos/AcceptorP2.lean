import HydroV2.MonoRel
import HydroV2.Paxos.Types
import Mathlib.Data.Multiset.Filter

/-!
# `acceptor_p2` (paxos.rs:808–899)

Acceptor phase-2 logic: batch the P2as (`nondet!` — safe because the
entries `persist()` before folding, so batch boundaries cannot affect
the eventual log), qualify them against `a_max_ballot`
(`Some(&p2a.ballot) >= max_ballot`), accumulate the qualified entries
across ticks, and ack each P2a to its sender with `Ok(())` iff its
ballot *is* the current max.

**The `manual_proof!` hole, made honest**: Rust's per-slot
`reduce_watermark` merge is not commutative when one slot sees two
entries with equal ballots and different values (paxos.rs:862's
`TODO: need assume`). The V2 model accumulates the raw entry
*multiset* — trivially commutative and inflationary, so the `NoOrder`
fold obligation is paid unconditionally — and computes the log as the
pure canonical view `logView` (per-slot max ballot; equal-ballot value
conflicts degrade to `none` via the commutative `ValWitness` consensus
fold). Under the slot-functional input contract the conflict branch is
unreachable, which is exactly the Rust `assume`, now checkable.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- The realized qualified-entry batches (the tick face of the
qualification filter). -/
def ap2QualBatches (nPr : Nat)
    (mx : Trace (Option (Ballot nPr)))
    (pool : Multiset (P2a P nPr)) (dB : List (Multiset (P2a P nPr))) :
    Trace (Multiset (Nat × LogValue P nPr)) :=
  (Trace.zip (batchCuts pool 0 dB) mx).map
    (fun bx => bx.1.filterMap (fun p2a =>
      if p2aQualifies bx.2 p2a.ballot then
        some (p2a.slot, (⟨p2a.ballot, p2a.value⟩ : LogValue P nPr))
      else none))



/-- The canonical ack multiset acceptor `j` addresses to proposer `r`
(contract vocabulary: the per-sender summand of the merged ack pool —
sender identity is channel structure, not message content). -/
def ap2From {nA nPr : Nat} {P : Type} [DecidableEq P]
    (bs : Fin nA → Trace (Multiset (P2a P nPr)))
    (mx : Fin nA → Trace (Option (Ballot nPr)))
    (j : Fin nA) (r : Fin nPr) : Multiset (P2b nPr) :=
  Multiset.filterMap
    (fun dx : Nat × P2b nPr => if dx.1 = r.val then some dx.2 else none)
    ((Trace.zip (bs j) (mx j)).map
      (fun bx => bx.1.map (fun a =>
        (a.sender.val,
          (⟨a.slot, a.ballot,
            if some a.ballot = bx.2 then .ok () else .error bx.2⟩
            : P2b nPr))))).sum

/-- What `acceptor_p2` **ensures**, over the `Values` denotation. -/
structure AP2Ensures (acc prop : L)
    (mx : TickV (mem acc) (Option (Ballot (mem prop))) .unbounded)
    (pool : Fin (mem acc) → Multiset (P2a P (mem prop)))
    (ck : TickV (mem acc) (Option Nat) .unbounded)
    (dB : BatchCuts (mem acc) (P2a P (mem prop)))
    (out : TickV (mem acc) (ALog P (mem prop)) .unbounded
      × (Fin (mem prop) → Multiset (P2b (mem prop)))) : Prop where
  /-- **The log face**: tick `t`'s `a_log` wire value pairs the tick's
  checkpoint with the canonical view of every qualified entry consumed
  through tick `t` (write-before-ack is the same-tick `zip` in
  `acceptor_p1`). -/
  log_face : ∀ i, out.1 i
    = (Trace.zip (ck i)
        (foldAcrossTicksTrace (· + ·) 0
          (ap2QualBatches (mem prop) (mx i) (pool i) (dB i)))).map
      (fun ce => (ce.1, logView ce.2))
  /-- **Ack characterization** (echo + routing + vote shape): every P2b
  in a proposer's pool acks a P2a consumed at some realized acceptor
  tick `t`, is routed to the P2a's sender, and its verdict compares the
  P2a's ballot against tick `t`'s `a_max_ballot` wire. -/
  ack_src : ∀ (r : Fin (mem prop)), ∀ m ∈ out.2 r,
    ∃ (j : Fin (mem acc)) (t : Nat)
      (hb : t < (batchCuts (pool j) 0 (dB j)).length)
      (hm : t < (mx j).length),
      ∃ p2a ∈ (batchCuts (pool j) 0 (dB j))[t]'hb,
        p2a.sender.val = r.val
        ∧ m.slot = p2a.slot ∧ m.ballot = p2a.ballot
        ∧ m.res = (if some p2a.ballot = (mx j)[t]'hm
            then Except.ok () else Except.error ((mx j)[t]'hm))
  /-- **The ack pool decomposes by sender**: proposer `r`'s acks are
  the sum of the per-acceptor summands (`values ∘ demux` opened
  once). -/
  ack_decomp : ∀ (r : Fin (mem prop)),
    out.2 r = ((List.finRange (mem acc)).map
      (fun j => ap2From (fun j' => batchCuts (pool j') 0 (dB j'))
        (fun j' => mx j') j r)).sum
  /-- **Per-sender ack cap**: acceptor `j` acks a key at most once per
  consumed matching P2a (acks are an elementwise image of the consumed
  batches). -/
  from_key_cap : ∀ (j : Fin (mem acc)) (r : Fin (mem prop))
    (s : Nat) (b : Ballot (mem prop)),
    ((ap2From (fun j' => batchCuts (pool j') 0 (dB j'))
        (fun j' => mx j') j r).filter
      (fun m => m.slot = s ∧ m.ballot = b)).card
      ≤ ((batchCuts (pool j) 0 (dB j)).sum).countP
        (fun a => a.slot = s ∧ a.ballot = b)
  /-- **Per-sender ack characterization** (echo + routing + vote shape
  at the SENDING acceptor), with **write-before-ack coverage**: an `Ok`
  ack pins the tick's max at its ballot and the published log's
  coverage of its key at that very tick. -/
  from_src : ∀ (j : Fin (mem acc)) (r : Fin (mem prop)),
    ∀ m ∈ ap2From (fun j' => batchCuts (pool j') 0 (dB j'))
      (fun j' => mx j') j r,
    ∃ (t : Nat) (hb : t < (batchCuts (pool j) 0 (dB j)).length)
      (hm : t < (mx j).length),
      ∃ p2a ∈ (batchCuts (pool j) 0 (dB j))[t]'hb,
        p2a.sender.val = r.val
        ∧ m.slot = p2a.slot ∧ m.ballot = p2a.ballot
        ∧ m.res = (if some p2a.ballot = (mx j)[t]'hm
            then Except.ok () else Except.error ((mx j)[t]'hm))
        ∧ (m.res = .ok () →
            (mx j)[t]'hm = some m.ballot
            ∧ ∀ hl : t < (out.1 j).length,
              LogCovers (((out.1 j)[t]'hl).2) m.slot m.ballot)


/-- **paxos.rs:808–899 `acceptor_p2`** over the acceptor cluster `acc`,
acking into the proposer cluster `prop`. Returns (`a_log` — the
`(checkpoint, log)` wire for `acceptor_p1` —, `a_to_proposers_p2b`). -/
def acceptor_p2 (H : HydroSem L mem) (acc prop : L)
    (a_max_ballot :
      H.TickSingleton acc (Option (Ballot (mem prop))) .unbounded)
    (p_to_acceptors_p2a :
      H.Stream acc (P2a P (mem prop)) .noOrder .exactlyOnce)
    (a_checkpoint : H.TickSingleton acc (Option Nat) .unbounded)
    (nondet_batch : BatchCuts (mem acc) (P2a P (mem prop)))
    (chP2b : ChannelId) :
    {out : H.TickSingleton acc (ALog P (mem prop)) .unbounded
      × H.Stream prop (P2b (mem prop)) .noOrder .exactlyOnce //
      -- the colocated contract (ghost; the proof is the `by` block
      -- below the body)
      ∀ hv : H = Values L mem,
        match H, hv, a_max_ballot, p_to_acceptors_p2a, a_checkpoint,
            out with
        | _, rfl, mx, pool, ck, o =>
          AP2Ensures acc prop mx pool ck nondet_batch o} :=
  -- .batch(acceptor_tick, nondet!(…))
  let p_to_acceptors_p2a_batch := H.batch p_to_acceptors_p2a nondet_batch
  -- .cross_singleton(a_max_ballot).filter_map(…): the qualification
  let a_p2as_to_place_in_log :=
    H.filterMapBatchesWith p_to_acceptors_p2a_batch a_max_ballot
      (fun _me p2a mb =>
        if p2aQualifies mb p2a.ballot then
          some (p2a.slot, (⟨p2a.ballot, p2a.value⟩ : LogValue P (mem prop)))
        else none)
  -- .across_ticks(reduce_watermark(…)): the persisted entry pool — the
  -- commutativity and inflation obligations are paid unconditionally
  let a_entries := H.fold_batches_across_ticks_monotone
    (ValueOrder.multiset _)
    (fun _me s e => s + {e}) 0
    (fun _i s x y => add_singleton_comm s x y)
    (fun _i s _x => Multiset.le_add_right ..)
    a_p2as_to_place_in_log
  -- a_checkpoint.into_singleton().zip(a_log_snapshot): the canonical view
  let a_log := H.mapTick (H.zipTick a_checkpoint (H.forgetBound a_entries))
    (fun _me ce => ((ce.1, logView ce.2) : ALog P (mem prop)))
  -- the ack leg: .cross_singleton(a_max_ballot).map(…)
  let acks := H.mapBatchesWith p_to_acceptors_p2a_batch a_max_ballot
    (fun _me p2a mb =>
      (p2a.sender.val,
        (⟨p2a.slot, p2a.ballot,
          if some p2a.ballot = mb then .ok () else .error mb⟩
          : P2b (mem prop))))
  -- .all_ticks().demux(proposers, …).values()
  ⟨(a_log,
    H.values (H.demux chP2b (H.allTicks acks) (fun r => r.val))), by
    intro hv
    subst hv
    -- the log face, hoisted (bullet 1 and `from_src`'s coverage share it)
    have hface : ∀ i, a_log i
        = (Trace.zip (a_checkpoint i)
            (foldAcrossTicksTrace (· + ·) 0
              (ap2QualBatches (mem prop) (a_max_ballot i)
                (p_to_acceptors_p2a i) (nondet_batch i)))).map
          (fun ce => (ce.1, logView ce.2)) := by
      intro i
      show (Trace.zip (a_checkpoint i)
          (foldAcrossTicksTrace (fun s b => @Multiset.foldl _ _ (fun s e => s + {e})
            ⟨fun s x y => add_singleton_comm s x y⟩ s b) 0
            (ap2QualBatches (mem prop) (a_max_ballot i) (p_to_acceptors_p2a i) (nondet_batch i)))).map _
        = _
      rw [show (fun (s : Multiset (Nat × LogValue P (mem prop))) b =>
          @Multiset.foldl _ _ (fun s e => s + {e})
            ⟨fun s x y => add_singleton_comm s x y⟩ s b) = (· + ·) from
        funext fun s => funext fun b => foldl_add_singleton s b]
    constructor
    · -- the log face
      exact hface
    · -- the ack characterization
      intro r m hm
      have hm0 : m ∈ (((List.finRange (mem acc)).map
          (fun j => Multiset.filterMap
            (fun dx : Nat × P2b (mem prop) =>
              if dx.1 = r.val then some dx.2 else none)
            (((Trace.zip (batchCuts (p_to_acceptors_p2a j) 0 (nondet_batch j)) (a_max_ballot j)).map
              (fun bx => bx.1.map (fun a =>
                (a.sender.val,
                  (⟨a.slot, a.ballot,
                      if some a.ballot = bx.2 then .ok () else .error bx.2⟩
                      : P2b (mem prop)))))).sum)))).sum := hm
      obtain ⟨mm, hmm, hm1⟩ := mem_list_sum.mp hm0
      obtain ⟨j, -, rfl⟩ := List.mem_map.mp hmm
      obtain ⟨dx, hdx, haddr⟩ := (Multiset.mem_filterMap _ _).mp hm1
      by_cases hr : dx.1 = r.val
      · rw [if_pos hr] at haddr
        injection haddr with haddr'
        obtain ⟨mt, hmt, hdx2⟩ := mem_list_sum.mp hdx
        obtain ⟨t, ht, hmt'⟩ := List.mem_iff_getElem.mp hmt
        have hzlen : t < (Trace.zip (batchCuts (p_to_acceptors_p2a j) 0 (nondet_batch j))
            (a_max_ballot j)).length := by
          rwa [List.length_map] at ht
        have hb : t < (batchCuts (p_to_acceptors_p2a j) 0 (nondet_batch j)).length := by
          have := hzlen
          simp only [Trace.zip, List.length_zip] at this
          omega
        have hmx : t < (a_max_ballot j).length := by
          have := hzlen
          simp only [Trace.zip, List.length_zip] at this
          omega
        rw [List.getElem_map] at hmt'
        have hpair : (Trace.zip (batchCuts (p_to_acceptors_p2a j) 0 (nondet_batch j)) (a_max_ballot j))[t]'hzlen
            = ((batchCuts (p_to_acceptors_p2a j) 0 (nondet_batch j))[t]'hb, (a_max_ballot j)[t]'hmx) := by
          simp only [Trace.zip]
          rw [List.getElem_zip]
        rw [hpair] at hmt'
        rw [← hmt'] at hdx2
        obtain ⟨a, ha, hpay⟩ := Multiset.mem_map.mp hdx2
        have hfst : a.sender.val = dx.1 := congrArg Prod.fst hpay
        have hsnd : (⟨a.slot, a.ballot,
            if some a.ballot = (a_max_ballot j)[t]'hmx then .ok ()
            else .error ((a_max_ballot j)[t]'hmx)⟩ : P2b (mem prop)) = dx.2 :=
          congrArg Prod.snd hpay
        rw [haddr'] at hsnd
        refine ⟨j, t, hb, hmx, a, ha, ?_, ?_, ?_, ?_⟩
        · rw [hfst, hr]
        · rw [← hsnd]
        · rw [← hsnd]
        · rw [← hsnd]
      · rw [if_neg hr] at haddr
        cases haddr
    · -- ack_decomp: the `values ∘ demux` boundary, opened once
      intro r
      rfl
    · -- from_key_cap: acks are an elementwise image of the batches
      intro j r sl b
      unfold ap2From
      rw [← Multiset.countP_eq_card_filter]
      refine le_trans (countP_filterMap_le _ _
        (fun dx : Nat × P2b (mem prop) =>
          dx.2.slot = sl ∧ dx.2.ballot = b)
        (fun dx m hdx hp => by
          by_cases hr : dx.1 = r.val
          · rw [if_pos hr] at hdx
            injection hdx with h
            rw [h]
            exact hp
          · rw [if_neg hr] at hdx
            cases hdx) _) ?_
      rw [countP_list_sum, countP_list_sum]
      simp only [List.map_map, Trace.zip]
      refine sum_map_zip_le _ _ (fun x => ?_)
        (batchCuts (p_to_acceptors_p2a j) 0 (nondet_batch j)) _
      show ((x.1.map (fun a =>
          (a.sender.val,
            (⟨a.slot, a.ballot,
              if some a.ballot = x.2 then .ok () else .error x.2⟩
              : P2b (mem prop))))).countP
          (fun dx => dx.2.slot = sl ∧ dx.2.ballot = b))
        ≤ (x.1).countP (fun a => a.slot = sl ∧ a.ballot = b)
      exact countP_map_le_countP _ _ _ (fun a hp => hp) x.1
    · -- from_src: echo + routing + vote shape + write-before-ack
      intro j r m hm1
      obtain ⟨dx, hdx, haddr⟩ := (Multiset.mem_filterMap _ _).mp hm1
      by_cases hr : dx.1 = r.val
      · rw [if_pos hr] at haddr
        injection haddr with haddr'
        obtain ⟨mt, hmt, hdx2⟩ := mem_list_sum.mp hdx
        obtain ⟨t, ht, hmt'⟩ := List.mem_iff_getElem.mp hmt
        have hzlen : t < (Trace.zip
            (batchCuts (p_to_acceptors_p2a j) 0 (nondet_batch j))
            (a_max_ballot j)).length := by
          rwa [List.length_map] at ht
        have hb : t < (batchCuts (p_to_acceptors_p2a j) 0
            (nondet_batch j)).length := by
          have := hzlen
          simp only [Trace.zip, List.length_zip] at this
          omega
        have hmx : t < (a_max_ballot j).length := by
          have := hzlen
          simp only [Trace.zip, List.length_zip] at this
          omega
        rw [List.getElem_map] at hmt'
        have hpair : (Trace.zip
            (batchCuts (p_to_acceptors_p2a j) 0 (nondet_batch j))
            (a_max_ballot j))[t]'hzlen
            = ((batchCuts (p_to_acceptors_p2a j) 0 (nondet_batch j))[t]'hb,
               (a_max_ballot j)[t]'hmx) := by
          simp only [Trace.zip]
          rw [List.getElem_zip]
        rw [hpair] at hmt'
        rw [← hmt'] at hdx2
        obtain ⟨a, ha, hpay⟩ := Multiset.mem_map.mp hdx2
        have hfst : a.sender.val = dx.1 := congrArg Prod.fst hpay
        have hsnd : (⟨a.slot, a.ballot,
            if some a.ballot = (a_max_ballot j)[t]'hmx then .ok ()
            else .error ((a_max_ballot j)[t]'hmx)⟩ : P2b (mem prop))
            = dx.2 := congrArg Prod.snd hpay
        rw [haddr'] at hsnd
        refine ⟨t, hb, hmx, a, ha, ?_, ?_, ?_, ?_, ?_⟩
        · rw [hfst, hr]
        · rw [← hsnd]
        · rw [← hsnd]
        · rw [← hsnd]
        · -- write-before-ack coverage
          intro hok
          by_cases hcond : some a.ballot = (a_max_ballot j)[t]'hmx
          · refine ⟨?_, ?_⟩
            · rw [show m.ballot = a.ballot from by rw [← hsnd],
                ← hcond]
            · intro hl
              -- the consumed p2a qualifies at its own max
              have hqual : p2aQualifies ((a_max_ballot j)[t]'hmx)
                  a.ballot = true := by
                rw [← hcond]
                show a.ballot.ble a.ballot = true
                exact Ballot.ble_iff_key.mpr (le_refl _)
              -- … so it lands in the tick's qualified batch
              have hqb : t < (ap2QualBatches (mem prop) (a_max_ballot j)
                  (p_to_acceptors_p2a j) (nondet_batch j)).length := by
                unfold ap2QualBatches
                rw [List.length_map]
                exact hzlen
              have hqel : (ap2QualBatches (mem prop) (a_max_ballot j)
                  (p_to_acceptors_p2a j) (nondet_batch j))[t]'hqb
                  = ((batchCuts (p_to_acceptors_p2a j) 0
                      (nondet_batch j))[t]'hb).filterMap
                    (fun p2a =>
                      if p2aQualifies ((a_max_ballot j)[t]'hmx)
                          p2a.ballot then
                        some (p2a.slot,
                          (⟨p2a.ballot, p2a.value⟩
                            : LogValue P (mem prop)))
                      else none) := by
                unfold ap2QualBatches at hqb ⊢
                rw [List.getElem_map, hpair]
              have hent : ((a.slot,
                  (⟨a.ballot, a.value⟩ : LogValue P (mem prop)))
                  : Nat × LogValue P (mem prop))
                  ∈ (ap2QualBatches (mem prop) (a_max_ballot j)
                    (p_to_acceptors_p2a j) (nondet_batch j))[t]'hqb := by
                rw [hqel]
                exact (Multiset.mem_filterMap _ _).mpr
                  ⟨a, ha, by rw [if_pos hqual]⟩
              -- … and the accumulated pool at `t` holds the batch
              have hfl : t < (foldAcrossTicksTrace (· + ·)
                  (0 : Multiset (Nat × LogValue P (mem prop)))
                  (ap2QualBatches (mem prop) (a_max_ballot j)
                    (p_to_acceptors_p2a j) (nondet_batch j))).length := by
                rw [foldAcrossTicksTrace_length]
                exact hqb
              have hcum : ((a.slot,
                  (⟨a.ballot, a.value⟩ : LogValue P (mem prop)))
                  : Nat × LogValue P (mem prop))
                  ∈ (foldAcrossTicksTrace (· + ·)
                    (0 : Multiset (Nat × LogValue P (mem prop)))
                    (ap2QualBatches (mem prop) (a_max_ballot j)
                      (p_to_acceptors_p2a j) (nondet_batch j)))[t]'hfl := by
                rw [foldAcrossTicksTrace_getElem]
                rw [← List.sum_eq_foldl]
                refine mem_list_sum.mpr
                  ⟨(ap2QualBatches (mem prop) (a_max_ballot j)
                    (p_to_acceptors_p2a j) (nondet_batch j))[t]'hqb,
                   ?_, hent⟩
                have htk : t < ((ap2QualBatches (mem prop)
                    (a_max_ballot j) (p_to_acceptors_p2a j)
                    (nondet_batch j)).take (t + 1)).length := by
                  rw [List.length_take]
                  omega
                have := List.getElem_mem htk
                rwa [List.getElem_take] at this
              -- the checkpoint leg also realized tick `t`
              have hck : t < (a_checkpoint j).length := by
                have h0 : t < (a_log j).length := hl
                rw [hface j] at h0
                simp only [List.length_map, Trace.zip,
                  List.length_zip] at h0
                omega
              -- the published log at `t` is the canonical view
              have hlogel : (a_log j)[t]'hl
                  = ((a_checkpoint j)[t]'hck,
                     logView ((foldAcrossTicksTrace (· + ·)
                       (0 : Multiset (Nat × LogValue P (mem prop)))
                       (ap2QualBatches (mem prop) (a_max_ballot j)
                         (p_to_acceptors_p2a j)
                         (nondet_batch j)))[t]'hfl)) := by
                rw [List.getElem_of_eq (hface j) hl, List.getElem_map]
                simp only [Trace.zip]
                rw [List.getElem_zip]
              have hgoal : LogCovers (((a_log j)[t]'hl).2) m.slot
                  m.ballot := by
                rw [show m.slot = a.slot from by rw [← hsnd],
                  show m.ballot = a.ballot from by rw [← hsnd], hlogel]
                exact logView_covers _ hcum
              exact hgoal
          · exfalso
            rw [← hsnd] at hok
            rw [if_neg hcond] at hok
            simp at hok
      · rw [if_neg hr] at haddr
        cases haddr⟩

/-- **`Ok` acks pin the max** (corollary): an `Ok` P2b's ballot *is*
the `a_max_ballot` wire value at its ack tick (paxos.rs:881) — with
`acceptor_p1`'s ascent, late low-ballot votes are impossible. -/
theorem acceptor_p2_ok_max (acc prop : L)
    (mx : TickV (mem acc) (Option (Ballot (mem prop))) .unbounded)
    (pool : Fin (mem acc) → Multiset (P2a P (mem prop)))
    (ck : TickV (mem acc) (Option Nat) .unbounded)
    (dB : BatchCuts (mem acc) (P2a P (mem prop))) (ch : ChannelId)
    (r : Fin (mem prop)) (m : P2b (mem prop))
    (hm : m ∈ (acceptor_p2 (Values L mem) acc prop mx pool ck dB ch).val.2 r)
    (hok : m.res = .ok ()) :
    ∃ (j : Fin (mem acc)) (t : Nat) (hmx : t < (mx j).length),
      (mx j)[t]'hmx = some m.ballot := by
  obtain ⟨j, t, hb, hmx, p2a, -, -, -, hballot, hres⟩ :=
    ((acceptor_p2 (Values L mem) acc prop mx pool ck dB ch).property rfl).ack_src r m hm
  refine ⟨j, t, hmx, ?_⟩
  by_cases hc : some p2a.ballot = (mx j)[t]'hmx
  · rw [← hc, hballot]
  · rw [if_neg hc] at hres
    rw [hres] at hok
    cases hok

/-! ## Executable smoke tests -/

-- One acceptor, one proposer: the qualified P2a lands in the log view
-- and is acked `Ok`; a stale-ballot P2a in the same batch is rejected
-- and kept out of the log.
#guard (acceptor_p2 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (fun _ => [some (Ballot.mk 1 0)])
    (fun _ => {⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩})
    (fun _ => [none])
    (fun _ => [{⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩}])
    1).val.1 0
  = [((none : Option Nat),
      [(0, (⟨Ballot.mk 1 0, some 42⟩ : LogValue Nat 1))])]

#guard (acceptor_p2 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (fun _ => [some (Ballot.mk 1 0)])
    (fun _ => ({⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩}
      : Multiset (P2a Nat 1)))
    (fun _ => [none])
    (fun _ => [{⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩}])
    1).val.2 0
  = {⟨0, Ballot.mk 1 0, .ok ()⟩, ⟨0, Ballot.mk 0 0, .error (some (Ballot.mk 1 0))⟩}

-- The conflict tie-break degrades to `none` (the `assume`, visible):
-- two same-slot same-ballot entries with different values.
#guard logView ({(0, ⟨Ballot.mk 1 0, some 42⟩), (0, ⟨Ballot.mk 1 0, some 7⟩)}
    : Multiset (Nat × LogValue Nat 1))
  = [(0, ⟨Ballot.mk 1 0, none⟩)]

end HydroV2
