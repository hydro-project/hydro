import HydroV2.MonoRel
import HydroV2.Paxos.Types
import Mathlib.Data.Multiset.Filter

/-!
# `acceptor_p1` (paxos.rs:484–524)

Acceptor phase-1 logic: maintain `a_max_ballot` — `across_ticks(|s|
s.max())` over the **unordered** P1a tick batches, safe because max is
commutative (the `NoOrder` fold obligation, consumed by the batch
quotient) and `Monotonic` by the inflation obligation — and reply to
each P1a with `Ok(log)` iff its ballot *is* the current max, else
`Err(max_ballot)`, routed to `ballot.proposer_id` by `demux`.

The `a_log` singleton is the same-tick wire from `acceptor_p2` (the
`a_log` `forward_ref` knot, closed in `paxos_core`): a tick's replies
exist only once its log value is realized — the *write-before-ack*
staging of Rust's `snapshot_atomic`.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-! ## The contract: vocabulary and guarantees (proofs below) -/

/-- The `a_max_ballot` batch step (`s.max()` absorbed batchwise — the
element closure's lift to the batch multiset). -/
def aMaxStep {nP : Nat} (s : Option (Ballot nP))
    (b : Multiset (Ballot nP)) : Option (Ballot nP) :=
  @Multiset.foldl _ _ Ballot.maxFold
    ⟨fun s x y => Ballot.maxFold_comm s x y⟩ s b


/-- The canonical reply multiset acceptor `j` addresses to proposer `r`
(contract vocabulary: the per-sender summand of the merged reply pool —
sender identity is channel structure, not message content). -/
def ap1From {nA nP : Nat} {P : Type} [DecidableEq P]
    (bs : Fin nA → Trace (Multiset (Ballot nP)))
    (alog : TickV nA (ALog P nP) .unbounded) (j : Fin nA) (r : Fin nP) :
    Multiset (P1b P nP) :=
  Multiset.filterMap
    (fun dx : Nat × P1b P nP => if dx.1 = r.val then some dx.2 else none)
    ((Trace.zip (bs j)
        (Trace.zip (foldAcrossTicksTrace aMaxStep none (bs j)) (alog j))).map
      (fun bx => bx.1.map (fun a =>
        (a.proposerId.val,
          (⟨a, if some a = bx.2.1 then .ok bx.2.2
            else .error bx.2.1⟩ : P1b P nP))))).sum


/-- What `acceptor_p1` **ensures**, over the `Values` denotation.
`a_max_ballot`'s across-tick ascent is *not* a field: it rides the
output type (`.1 i` is a `MonoTrace Ballot.obtVO`). -/
structure AP1Ensures (acc prop : L)
    (bs : Fin (mem acc) → Trace (Multiset (Ballot (mem prop))))
    (alog : TickV (mem acc) (ALog P (mem prop)) .unbounded)
    (out : TickV (mem acc) (Option (Ballot (mem prop)))
        (.monotonic Ballot.obtVO)
      × (Fin (mem prop) → Multiset (P1b P (mem prop)))) : Prop where
  /-- The max face: per tick, the fold of everything consumed so far
  (including this tick's batch) — the two faces cannot drift. -/
  max_face : ∀ i, (out.1 i).vals = foldAcrossTicksTrace aMaxStep none (bs i)
  /-- **The pool decomposes by sender**: proposer `r`'s replies are the
  sum of the per-acceptor summands (`values ∘ demux` opened once). -/
  reply_decomp : ∀ (r : Fin (mem prop)),
    out.2 r = ((List.finRange (mem acc)).map
      (fun j => ap1From bs alog j r)).sum
  /-- **Per-sender reply cap**: acceptor `j` replies at ballot `b` at
  most once per consumed copy of `b` (replies are an elementwise image
  of the consumed batches). -/
  from_ballot_cap : ∀ (j : Fin (mem acc)) (r : Fin (mem prop))
    (b : Ballot (mem prop)),
    ((ap1From bs alog j r).filter (fun m => m.ballot = b)).card
      ≤ ((bs j).sum).count b
  /-- **Per-sender reply characterization** (echo + routing + promise
  shape at the SENDING acceptor): every reply in `j`'s summand quotes a
  P1a consumed at some realized tick `t` of `j`, is routed to the
  ballot's owner, and its verdict compares against tick `t`'s max and
  quotes tick `t`'s `a_log` wire value (write-before-ack). -/
  from_src : ∀ (j : Fin (mem acc)) (r : Fin (mem prop)),
    ∀ m ∈ ap1From bs alog j r,
    ∃ (t : Nat) (hb : t < (bs j).length)
      (hm : t < (out.1 j).vals.length) (hl : t < (alog j).length),
      m.ballot ∈ (bs j)[t]'hb
      ∧ m.ballot.proposerId.val = r.val
      ∧ m.res = (if some m.ballot = (out.1 j).vals[t]'hm
          then Except.ok ((alog j)[t]'hl)
          else Except.error ((out.1 j).vals[t]'hm))
  /-- **Reply characterization** (merged-pool corollary shape): every
  reply in a proposer's pool quotes a P1a consumed at some realized
  acceptor tick `t`, is routed to the ballot's owner, and its verdict
  compares against tick `t`'s max and quotes tick `t`'s `a_log` wire
  value (write-before-ack). -/
  reply_src : ∀ (r : Fin (mem prop)), ∀ m ∈ out.2 r,
    ∃ (j : Fin (mem acc)) (t : Nat) (hb : t < (bs j).length)
      (hm : t < (out.1 j).vals.length) (hl : t < (alog j).length),
      m.ballot ∈ (bs j)[t]'hb
      ∧ m.ballot.proposerId.val = r.val
      ∧ m.res = (if some m.ballot = (out.1 j).vals[t]'hm
          then Except.ok ((alog j)[t]'hl)
          else Except.error ((out.1 j).vals[t]'hm))


/-- **paxos.rs:484–524 `acceptor_p1`** over the acceptor cluster `acc`,
replying into the proposer cluster `prop`. Returns (`a_max_ballot`,
`a_to_proposers_p1b`). -/
def acceptor_p1 (H : HydroSem L mem) (acc prop : L)
    (p_to_acceptors_p1a :
      H.TickStream acc (Ballot (mem prop)) .noOrder .exactlyOnce)
    (a_log : H.TickSingleton acc (ALog P (mem prop)) .unbounded)
    (chP1b : ChannelId) :
    {out : H.TickSingleton acc (Option (Ballot (mem prop)))
        (.monotonic Ballot.obtVO)
      × H.Stream prop (P1b P (mem prop)) .noOrder .exactlyOnce //
      -- the colocated contract (ghost; proof below the impl)
      ∀ hv : H = Values L mem,
        match H, hv, p_to_acceptors_p1a, a_log, out with
        | _, rfl, bs, al, o => AP1Ensures acc prop bs al o} :=
  -- .across_ticks(|s| s.max()).into_singleton(): the commutativity and
  -- `monotonic =` obligations are paid inline, once
  let a_max_ballot := H.fold_batches_across_ticks_monotone Ballot.obtVO
    (fun _me s b => Ballot.maxFold s b) none
    (fun _i => Ballot.maxFold_comm) (fun _i => Ballot.obtLE_maxFold)
    p_to_acceptors_p1a
  -- .cross_singleton(a_max_ballot).cross_singleton(a_log).map(…)
  let replies := H.mapBatchesWith p_to_acceptors_p1a
    (H.zipTick (H.forgetBound a_max_ballot) a_log)
    (fun _me ballot ml =>
      (ballot.proposerId.val,
        (⟨ballot,
          if some ballot = ml.1 then .ok ml.2 else .error ml.1⟩
          : P1b P (mem prop))))
  -- .all_ticks().demux(proposers, …).values()
  ⟨(a_max_ballot,
    H.values (H.demux chP1b (H.allTicks replies) (fun r => r.val))), by
    intro hv
    subst hv
    -- per-sender reply characterization (local lemma)
    have hsrc : ∀ (j : Fin (mem acc)) (r : Fin (mem prop)),
        ∀ m ∈ ap1From p_to_acceptors_p1a a_log j r,
        ∃ (t : Nat) (hb : t < (p_to_acceptors_p1a j).length)
          (hm : t < (foldAcrossTicksTrace aMaxStep none
            (p_to_acceptors_p1a j)).length)
          (hl : t < (a_log j).length),
          m.ballot ∈ (p_to_acceptors_p1a j)[t]'hb
          ∧ m.ballot.proposerId.val = r.val
          ∧ m.res = (if some m.ballot = (foldAcrossTicksTrace aMaxStep none
                (p_to_acceptors_p1a j))[t]'hm
              then Except.ok ((a_log j)[t]'hl)
              else Except.error ((foldAcrossTicksTrace aMaxStep none
                (p_to_acceptors_p1a j))[t]'hm)) := by
      intro j r m hm1
      -- demux: the address filter pins the routing
      obtain ⟨dx, hdx, haddr⟩ := (Multiset.mem_filterMap _ _).mp hm1
      by_cases hr : dx.1 = r.val
      · rw [if_pos hr] at haddr
        injection haddr with haddr'
        -- all_ticks: pick the tick t
        obtain ⟨mt, hmt, hdx2⟩ := mem_list_sum.mp hdx
        obtain ⟨t, ht, hmt'⟩ := List.mem_iff_getElem.mp hmt
        have hzlen : t < (Trace.zip (p_to_acceptors_p1a j)
            (Trace.zip (foldAcrossTicksTrace aMaxStep none (p_to_acceptors_p1a j)) (a_log j))).length := by
          rwa [List.length_map] at ht
        have hb : t < (p_to_acceptors_p1a j).length := by
          have := hzlen
          simp only [Trace.zip, List.length_zip, foldAcrossTicksTrace_length] at this
          omega
        have hm2 : t < (foldAcrossTicksTrace aMaxStep none (p_to_acceptors_p1a j)).length := by
          rw [foldAcrossTicksTrace_length]
          have := hzlen
          simp only [Trace.zip, List.length_zip, foldAcrossTicksTrace_length] at this
          omega
        have hl : t < (a_log j).length := by
          have := hzlen
          simp only [Trace.zip, List.length_zip, foldAcrossTicksTrace_length] at this
          omega
        -- the tick's summand is the mapped batch against the tick's reads
        rw [List.getElem_map] at hmt'
        have hpair : (Trace.zip (p_to_acceptors_p1a j)
            (Trace.zip (foldAcrossTicksTrace aMaxStep none (p_to_acceptors_p1a j)) (a_log j)))[t]'hzlen
            = ((p_to_acceptors_p1a j)[t]'hb,
               ((foldAcrossTicksTrace aMaxStep none (p_to_acceptors_p1a j))[t]'hm2, (a_log j)[t]'hl)) := by
          simp only [Trace.zip]
          rw [List.getElem_zip, List.getElem_zip]
        rw [hpair] at hmt'
        rw [← hmt'] at hdx2
        obtain ⟨a, ha, hpay⟩ := Multiset.mem_map.mp hdx2
        -- the payload shape pins everything
        have hfst : a.proposerId.val = dx.1 := congrArg Prod.fst hpay
        have hsnd : (⟨a, if some a = (foldAcrossTicksTrace aMaxStep none (p_to_acceptors_p1a j))[t]'hm2
            then .ok ((a_log j)[t]'hl)
            else .error ((foldAcrossTicksTrace aMaxStep none (p_to_acceptors_p1a j))[t]'hm2)⟩
              : P1b P (mem prop)) = dx.2 := congrArg Prod.snd hpay
        rw [haddr'] at hsnd
        refine ⟨t, hb, hm2, hl, ?_, ?_, ?_⟩
        · rw [show m.ballot = a from by rw [← hsnd]]
          exact ha
        · rw [show m.ballot = a from by rw [← hsnd], hfst, hr]
        · rw [show m.ballot = a from by rw [← hsnd], ← hsnd]
      · rw [if_neg hr] at haddr
        cases haddr
    -- per-sender ballot cap (local lemma)
    have hcap : ∀ (j : Fin (mem acc)) (r : Fin (mem prop))
        (b : Ballot (mem prop)),
        ((ap1From p_to_acceptors_p1a a_log j r).filter
          (fun m => m.ballot = b)).card
          ≤ ((p_to_acceptors_p1a j).sum).count b := by
      intro j r b
      unfold ap1From
      rw [← Multiset.countP_eq_card_filter]
      refine le_trans (countP_filterMap_le _ _
        (fun dx : Nat × P1b P (mem prop) => dx.2.ballot = b)
        (fun dx m hdx hp => by
          by_cases hr : dx.1 = r.val
          · rw [if_pos hr] at hdx; injection hdx with h; rw [h]; exact hp
          · rw [if_neg hr] at hdx; cases hdx) _) ?_
      rw [countP_list_sum, count_list_sum]
      simp only [List.map_map, Trace.zip]
      refine sum_map_zip_le _ _ (fun x => ?_) (p_to_acceptors_p1a j) _
      show ((x.1.map (fun a =>
          (a.proposerId.val,
            (⟨a, if some a = x.2.1 then .ok x.2.2
              else .error x.2.1⟩ : P1b P (mem prop))))).countP
          (fun dx => dx.2.ballot = b)) ≤ (x.1).count b
      exact countP_map_le_count _ _ b (fun a hp => hp) x.1
    exact ⟨fun i => rfl, fun r => rfl, hcap, hsrc, fun r m hm => by
      have hm0 : m ∈ ((List.finRange (mem acc)).map
          (fun j => ap1From p_to_acceptors_p1a a_log j r)).sum := hm
      obtain ⟨mm, hmm, hm1⟩ := mem_list_sum.mp hm0
      obtain ⟨j, -, rfl⟩ := List.mem_map.mp hmm
      obtain ⟨t, hb, hm2, hl, hmem, hrt, hres⟩ := hsrc j r m hm1
      exact ⟨j, t, hb, hm2, hl, hmem, hrt, hres⟩⟩⟩


/-- **`Ok` promises pin the max** (corollary): an `Ok` reply's ballot
*is* `a_max_ballot` at its promise tick — with the output's
`.ascending`, it bounds the max from below forever after. -/
theorem acceptor_p1_ok_pins (acc prop : L)
    (bs : Fin (mem acc) → Trace (Multiset (Ballot (mem prop))))
    (alog : TickV (mem acc) (ALog P (mem prop)) .unbounded) (ch : ChannelId)
    (r : Fin (mem prop)) (m : P1b P (mem prop))
    (hm : m ∈ (acceptor_p1 (Values L mem) acc prop bs alog ch).val.2 r)
    (pl : ALog P (mem prop)) (hok : m.res = .ok pl) :
    ∃ (j : Fin (mem acc)) (t : Nat)
      (hm2 : t < ((acceptor_p1 (Values L mem) acc prop bs alog ch).val.1
        j).vals.length),
      ((acceptor_p1 (Values L mem) acc prop bs alog ch).val.1 j).vals[t]'hm2
        = some m.ballot := by
  obtain ⟨j, t, hb, hm2, hl, -, -, hres⟩ :=
    ((acceptor_p1 (Values L mem) acc prop bs alog ch).property
      rfl).reply_src r m hm
  refine ⟨j, t, hm2, ?_⟩
  by_cases hc : some m.ballot
      = ((acceptor_p1 (Values L mem) acc prop bs alog ch).val.1 j).vals[t]'hm2
  · exact hc.symm
  · rw [if_neg hc] at hres
    rw [hres] at hok
    cases hok

/-! ## Executable smoke tests -/

-- One acceptor, one proposer: tick 0 consumes `{(1,0)}` (Ok, quoting the
-- tick's log wire), tick 1 consumes the stale `{(0,0)}` (Err, quoting
-- the pinned max).
#guard ((acceptor_p1 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (fun _ => [{Ballot.mk 1 0}, {Ballot.mk 0 0}])
    (fun _ => [(none, []), (none, [(0, ⟨Ballot.mk 1 0, some 42⟩)])]
      : TickV 1 (ALog Nat 1) .unbounded) 0).val.1 0).vals
  = [some (Ballot.mk 1 0), some (Ballot.mk 1 0)]

#guard (acceptor_p1 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (fun _ => [{Ballot.mk 1 0}, {Ballot.mk 0 0}])
    (fun _ => [(none, []), (none, [(0, ⟨Ballot.mk 1 0, some 42⟩)])]
      : TickV 1 (ALog Nat 1) .unbounded) 0).val.2 0
  = {⟨Ballot.mk 1 0, .ok (none, [])⟩,
     ⟨Ballot.mk 0 0, .error (some (Ballot.mk 1 0))⟩}

end HydroV2
