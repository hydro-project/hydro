import Hydro.MonoRel
import Hydro.Paxos.Types
import Hydro.HydroTick

/-!
# `recommit_after_leader_election` (paxos.rs:595–672)

A fresh leader reconciles the quorum's accepted logs: per slot, keep the
max-ballot entry (recommitting it under the new ballot unless already
committed on more than `f` acceptors or checkpointed away), and fill the
log holes below the max slot with no-ops.

**The per-slot fold is order-dependent** (D71). paxos.rs:617–640's
`(count, champion)` closure counts entries carrying the champion's value
and resets the count when a higher ballot brings a new value; two orders
of one batch can disagree on whether a slot is skipped (`count > f`) or
recommitted — FINDINGS D71's X·Y·Z. Both decisions are safe (a skip means
more than `f` acceptors hold the champion's value; a recommit proposes
the champion's value), so the hole is not a bug but an honest
nondeterminism: the program realizes the tick's order by an
`assume_ordering` decision (`tassume_ordering`, the Rust edit's
`nondet!`), the fold is the literal closure (`rcStep`) with NO
commutativity prerequisite, and every fact below is proven for EVERY
order (the champion is one of the slot's entries and dominates them all
in ballot; the count never overshoots the entries carrying its value).

The contract (`RCEnsures`) is stated over the module's OUTPUT wires in
the consumer's vocabulary — one tick's recommit list and max slot against
the tick's input batch and ballot (`RCTick`) — and every fact is proven
INLINE as a ghost about the program's own tick (its generated step, read
at the denotation by `den`): no pure twin of the body, no bridge lemma
(FINDINGS D64, E6/E9).
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-! ## Prerequisites for the contract face — the program starts at
`hydro def recommit_after_leader_election` -/

/-- `recommit_after_leader_election`'s decisions. -/
structure RCDec (H : HydroSem L mem) (nP : Nat) (P : Type) [DecidableEq P] where
  /-- `.into_keyed().assume_ordering::<TotalOrder>(nondet!(…))`
  (paxos.rs:616–617, D71): the per-tick order the champion fold consumes
  the view's entries in. -/
  order : H.TickOrderSelDec nP (Nat × LogValue P nP)

/-- **One tick of the recommit computation**, in the consumer's
vocabulary: at a tick whose input batch is `gv` and whose ballot is `b`,
the module publishes the recommit list `rcl` and the max slot `m`
(paxos.rs:611–670). -/
structure RCTick {nP : Nat} (variant : PaxosVariant) (f : Nat) (gv : Multiset (ALog P nP))
    (b : Ballot nP) (rcl : List ((Nat × Ballot nP) × Option P)) (m : Option Nat) : Prop where
  /-- **Recommits are owned**: every entry quotes the tick's own ballot. -/
  owned : ∀ e ∈ rcl, (e : (Nat × Ballot nP) × Option P).1.2 = b
  /-- **Slots are distinct** within the tick (one champion per slot, holes
  below the max are each filled once). -/
  slots_nodup : (rcl.map (fun e => (e : (Nat × Ballot nP) × Option P).1.1)).Nodup
  /-- **Every recommit sits at or below the max slot.** -/
  slot_le_max : ∀ e ∈ rcl, ∃ mm, m = some mm ∧ (e : (Nat × Ballot nP) × Option P).1.1 ≤ mm
  /-- **The max slot dominates the view**: every accepted entry's slot is
  at or below it. -/
  max_ge : ∀ (slot : Nat) (e₀ : LogValue P nP), (slot, e₀) ∈ rcEntries gv →
    ∃ mm, m = some mm ∧ slot ≤ mm
  /-- **The max slot dominates the view's checkpoints** (guarded, B4): a
  fresh payload is indexed above every slot the view reports executed. -/
  max_ge_ckpt : variant = .guarded → ∀ lg ∈ gv, ∀ c : Nat, (lg : ALog P nP).1 = some c →
    ∃ mm, m = some mm ∧ c ≤ mm
  /-- **Recommits sit above the view's checkpoints**: a slot at or below
  a reported checkpoint is executed — never re-proposed (paxos.rs:648–651,
  :661–662). -/
  above_ckpt : ∀ e ∈ rcl, ∀ lg ∈ gv, ∀ c : Nat, (lg : ALog P nP).1 = some c →
    c < (e : (Nat × Ballot nP) × Option P).1.1
  /-- **An empty view recommits nothing** and has no max slot. -/
  view_empty : gv = 0 → rcl = [] ∧ m = none
  /-- **The champion's value**: a recommit at a slot carries the value of
  one of the view's entries there whose ballot dominates every accepted
  entry at the slot (the fold's champion, in whatever order the entries
  arrived). -/
  value_best : ∀ e ∈ rcl, ∀ e₀ : LogValue P nP,
    ((e : (Nat × Ballot nP) × Option P).1.1, e₀) ∈ rcEntries gv →
    ∃ best : LogValue P nP, (e.1.1, best) ∈ rcEntries gv
      ∧ e.2 = best.value ∧ e₀.ballot.ble best.ballot = true
  /-- **Why skipping is safe** (D71): a slot the view has entries at but
  the tick does not recommit is either executed (at or below a reported
  checkpoint) or held by more than `f` acceptors at the champion's value —
  the fold's count never overshoots the holders, in any order. -/
  skipped_holders : ∀ (slot : Nat) (e₀ : LogValue P nP), (slot, e₀) ∈ rcEntries gv →
    (∀ e ∈ rcl, (e : (Nat × Ballot nP) × Option P).1.1 ≠ slot) →
    (∃ c : Nat, (∃ lg ∈ gv, (lg : ALog P nP).1 = some c) ∧ slot ≤ c)
    ∨ ∃ best : LogValue P nP, (slot, best) ∈ rcEntries gv
        ∧ (∀ e₁ : LogValue P nP, (slot, e₁) ∈ rcEntries gv → e₁.ballot.ble best.ballot = true)
        ∧ f < ((rcEntries gv).filter (fun e => e.1 = slot ∧ e.2.value = best.value)).card

/-- What `recommit_after_leader_election` **ensures**, over the `Values`
denotation, on its output wires: a published tick is an input tick's
`RCTick`, read from either side. (A tick is published when its order
decision realizes the batch — `realizeTicks`; the outputs are traces, so
every earlier tick is published too.) -/
structure RCEnsures (variant : PaxosVariant) (prop : L) (f : Nat)
    (bs : MemberMap (mem prop) (Trace (Multiset (ALog P (mem prop)))))
    (pb : TickV (mem prop) (Ballot (mem prop)))
    (out : TickV (mem prop) (List ((Nat × Ballot (mem prop)) × Option P))
      × TickV (mem prop) (Option Nat)) : Prop where
  /-- **Read from the commits**: a published recommit list is its tick's
  `RCTick` against the tick's batch and ballot. -/
  commits_at : ∀ (i : Fin (mem prop)) {t : Nat}
    {rcl : List ((Nat × Ballot (mem prop)) × Option P)},
    (out.1 i)[t]? = some rcl →
    ∃ gv b m, (bs i)[t]? = some gv ∧ (pb i)[t]? = some b ∧ (out.2 i)[t]? = some m
      ∧ RCTick variant f gv b rcl m
  /-- **Read from the max slots**: likewise. -/
  maxslot_at : ∀ (i : Fin (mem prop)) {t : Nat} {m : Option Nat},
    (out.2 i)[t]? = some m →
    ∃ gv b rcl, (bs i)[t]? = some gv ∧ (pb i)[t]? = some b ∧ (out.1 i)[t]? = some rcl
      ∧ RCTick variant f gv b rcl m

/-- **paxos.rs:595–672 `recommit_after_leader_election`** over the
proposer cluster. Returns (`p_log_to_try_commit.chain(p_log_holes)`,
`p_max_slot`) per tick. -/
hydro def recommit_after_leader_election (H : HydroSem L mem) (variant : PaxosVariant)
    (prop : L)
    (accepted_logs :
      H.AtomicStream prop (ALog P (mem prop)) .noOrder .exactlyOnce)
    (p_ballot : H.Atomic prop (Ballot (mem prop)))
    (f : Nat) (dec : RCDec H (mem prop) P) :
    (H.Atomic prop (List ((Nat × Ballot (mem prop)) × Option P))
      × H.Atomic prop (Option Nat))
  ensures out => RCEnsures variant prop f accepted_logs p_ballot out :=
  -- let p_p1b_highest_entries_and_count = accepted_logs
  --   .map(q!(|(_checkpoint, log)| log))
  --   .flatten_unordered() // Convert HashMap log back to stream
  tick (input logs := accepted_logs) :=
    yield (p_entries := H.bflatMapUnordered (H.bmap logs (fun lg => lg.2))
      (fun log => H.bofList log.toList));
  --   .into_keyed()
  --   .assume_ordering::<TotalOrder>(nondet!(/** the per-slot fold is
  --     order-dependent; every order is safe — FINDINGS D71 */))
  -- (the Lean mirror orders the pair batch and groups in the tick body:
  -- every per-key order family arises from one pair order)
  let p_entries_ordered := H.tassume_ordering p_entries dec.order
  -- the rest of the function is tick-located dataflow over this tick's
  -- ordered entries, `accepted_logs` batch and `p_ballot`
  tick (input ents := p_entries_ordered) (input logs := accepted_logs) (input pb := p_ballot) :=
    -- let p_p1b_max_checkpoint = accepted_logs.clone()
    --   .filter_map(q!(|(checkpoint, _log)| checkpoint))
    --   .max()
    --   .into_singleton();
    let p_p1b_max_checkpoint := H.bmax (H.bfilterMap logs (fun lg => lg.1))
    --   .fold(q!(|| (0, None)), q!(|curr_entry, new_entry| { … }))
    -- (the literal closure, `rcStep`; the `commutative = manual_proof!(…)`
    -- hole is gone — the order is the decision above)
    let champions := H.bkfold rcStep (0, none) (fun _ _ => True) trivial (H.binto_keyed ents)
    --   .map(q!(|(count, entry)| (count, entry.unwrap())));
    -- (`.entries()` of the keyed singleton, as the HashMap's canonical
    -- association list — slots ascending, one champion per slot)
    let champion_list := H.bfold insertSorted [] NoKeyOverlap insertSorted_foldOk
      (H.bksEntries champions)
    let p_p1b_highest_entries_and_count := H.bsMap champion_list
      (fun l => l.filterMap (fun x => x.2.2.map (fun e => (x.2.1, (x.1, e)))))
    -- let p_log_to_try_commit = p_p1b_highest_entries_and_count.clone()
    --   .entries()
    --   .cross_singleton(p_ballot.clone())
    --   .cross_singleton(p_p1b_max_checkpoint.clone())
    --   .filter_map(q!(move |(((slot, (count, entry)), ballot), checkpoint)| {
    --     if count > f { return None; }
    --     else if let Some(checkpoint) = checkpoint && slot <= checkpoint { return None; }
    --     Some(((slot, ballot), entry.value)) }));
    let p_log_to_try_commit := H.bsMap
      (H.bsZip (H.bsZip p_p1b_highest_entries_and_count pb) p_p1b_max_checkpoint)
      (fun x => x.1.1.filterMap (fun csl =>
        if f < csl.1 then none
        else if (match x.2 with
          | some c => decide (csl.2.1 ≤ c)
          | none => false) then none
        else some ((csl.2.1, x.1.2), csl.2.2.value)))
    -- let p_max_slot = p_p1b_highest_entries_and_count.clone().keys().max();
    -- GUARDED (B4 fix, D71): the max slot is lifted to the view's
    -- checkpoint — fresh payloads are indexed above every executed slot
    -- (an acceptor has collected those; a never-led proposer must not
    -- reuse one). FAITHFUL: paxos.rs verbatim. (No Rust line yet: upstream
    -- `p_max_slot.zip(p_p1b_max_checkpoint).map(max)`, SCHED_AUDIT.)
    let p_max_slot := H.bsMap (H.bsZip p_p1b_highest_entries_and_count p_p1b_max_checkpoint)
      (fun x => rcMaxSlot variant
        (x.1.map (fun (csl : Nat × Nat × LogValue P (mem prop)) => csl.2.1)).max? x.2)
    -- let p_proposed_slots = p_p1b_highest_entries_and_count.clone().keys();
    -- let p_log_holes = p_max_slot.clone().zip(p_p1b_max_checkpoint)
    --   .flat_map_ordered(q!(|(max_slot, checkpoint)| {
    --     if let Some(checkpoint) = checkpoint { (checkpoint + 1)..max_slot } else { 0..max_slot } }))
    --   .filter_not_in(p_proposed_slots)
    --   .cross_singleton(p_ballot.clone())
    --   .map(q!(move |(slot, ballot)| ((slot, ballot), None)));
    let p_log_holes := H.bsMap
      (H.bsZip (H.bsZip p_max_slot p_p1b_max_checkpoint)
        (H.bsZip p_p1b_highest_entries_and_count pb))
      (fun x =>
        (match x.1.1 with
          | some maxSlot =>
            (List.range' (match x.1.2 with | some c => c + 1 | none => 0)
              (maxSlot
                - (match x.1.2 with | some c => c + 1 | none => 0))).filter
              (fun slot => !(x.2.1.map
                (fun (csl : Nat × Nat × LogValue P (mem prop)) =>
                  csl.2.1)).contains slot)
          | none => []).map
            (fun slot => ((slot, x.2.2), (none : Option P))))
    -- (p_log_to_try_commit.chain(p_log_holes), p_max_slot)
    emit (p_log_chained := H.bsMap (H.bsZip p_log_to_try_commit p_log_holes)
        (fun x => x.1 ++ x.2),
      p_max_slot := p_max_slot);
  -- ===== the champion list of an ordered batch, read =====
  -- the counted champions of the ordered entries `l` (the Rust lines
  -- 616–641 at the denotation): the keyed fold's map, its slot-sorted
  -- entries, unwrapped
  ghost let chOf := fun (l : List (Nat × LogValue P (mem prop))) =>
    (Multiset.foldlOn NoKeyOverlap insertSorted [] insertSorted_foldOk
      (KeyedMap.ofFold rcStep (0, none) (fun _ _ => True) trivial (KeyedPool.ofList l)).entries).filterMap
      (fun x => x.2.2.map (fun e => (x.2.1, (x.1, e))))
  -- a counted champion is its slot's fold result
  ghost have hch_mem : ∀ (l : List (Nat × LogValue P (mem prop))) (csl : Nat × Nat × LogValue P (mem prop)),
      csl ∈ chOf l ↔ csl.2.1 ∈ l.map Prod.fst
        ∧ (keyVals csl.2.1 l).foldl rcStep (0, none) = (csl.1, some csl.2.2) := fun l csl => by
    simp only [chOf, List.mem_filterMap]
    constructor
    · rintro ⟨x, hx, hfx⟩
      rw [mem_foldlOn_insertSorted _ (KeyedMap.entries_pairwise_key _), KeyedMap.mem_entries_iff,
        KeyedMap.get_ofFold_ofList] at hx
      simp only [List.not_mem_nil, false_or] at hx
      split at hx
      · rename_i hk
        obtain ⟨e, he, rfl⟩ := Option.map_eq_some_iff.mp hfx
        refine ⟨hk, ?_⟩
        have hx' := Option.some.inj hx
        show (keyVals x.1 l).foldl rcStep (0, none) = (x.2.1, some e)
        rw [hx']
        exact Prod.ext rfl he
      · cases hx
    · rintro ⟨hk, hfold⟩
      refine ⟨(csl.2.1, (csl.1, some csl.2.2)), ?_, rfl⟩
      rw [mem_foldlOn_insertSorted _ (KeyedMap.entries_pairwise_key _), KeyedMap.mem_entries_iff,
        KeyedMap.get_ofFold_ofList, if_pos hk, hfold]
      simp
  -- every slot with an entry has a champion
  ghost have hch_cover : ∀ (l : List (Nat × LogValue P (mem prop))) (slot : Nat) (e : LogValue P (mem prop)),
      (slot, e) ∈ l → ∃ cnt c, (cnt, (slot, c)) ∈ chOf l := fun l slot e he => by
    have hk : slot ∈ l.map Prod.fst := List.mem_map.mpr ⟨(slot, e), he, rfl⟩
    have hne : keyVals slot l ≠ [] := by
      intro hnil
      have : e ∈ keyVals slot l :=
        List.mem_map.mpr ⟨(slot, e), List.mem_filter.mpr ⟨he, by simp⟩, rfl⟩
      rw [hnil] at this
      exact absurd this List.not_mem_nil
    rcases hf : (keyVals slot l).foldl rcStep (0, none) with ⟨cnt, c?⟩
    cases c? with
    | none => exact absurd (rcFold_none hf) hne
    | some c => exact ⟨cnt, c, (hch_mem l (cnt, (slot, c))).mpr ⟨hk, hf⟩⟩
  -- a champion is an entry of its slot, dominates its slot's entries in
  -- ballot, and its count is bounded by the entries carrying its value
  ghost have hch_champ : ∀ (l : List (Nat × LogValue P (mem prop))) (csl : Nat × Nat × LogValue P (mem prop)),
      csl ∈ chOf l →
      (csl.2.1, csl.2.2) ∈ l
        ∧ (∀ e : LogValue P (mem prop), (csl.2.1, e) ∈ l → e.ballot ≤ csl.2.2.ballot)
        ∧ csl.1 ≤ (l.filter (fun p => p.1 = csl.2.1 ∧ p.2.value = csl.2.2.value)).length :=
    fun l csl hcsl => by
    obtain ⟨-, hfold⟩ := (hch_mem l csl).mp hcsl
    obtain ⟨hmem, hdom, hcnt⟩ := rcFold_champ hfold
    refine ⟨mem_keyVals hmem, fun e he => hdom e ?_, ?_⟩
    · exact List.mem_map.mpr ⟨(csl.2.1, e), List.mem_filter.mpr ⟨he, by simp⟩, rfl⟩
    · have hlen : ((keyVals csl.2.1 l).filter (fun e => e.value = csl.2.2.value)).length
          = (l.filter (fun p => p.1 = csl.2.1 ∧ p.2.value = csl.2.2.value)).length := by
        unfold keyVals
        rw [List.filter_map, List.length_map, List.filter_filter]
        refine congrArg List.length (List.filter_congr fun p _ => ?_)
        simp only [Function.comp, Bool.decide_and]
        exact Bool.and_comm _ _
      exact hlen ▸ hcnt
  -- the champions' slots are distinct (one entry per key of the map)
  ghost have hch_nodup : ∀ (l : List (Nat × LogValue P (mem prop))),
      ((chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).Nodup := fun l => by
    simp only [chOf]
    refine List.Sublist.nodup (List.map_filterMap_sublist _ _ Prod.fst (fun x y hxy => ?_) _) ?_
    · obtain ⟨e, -, rfl⟩ := Option.map_eq_some_iff.mp hxy
      rfl
    · rw [← Multiset.coe_nodup, ← Multiset.map_coe,
        coe_foldlOn_insertSorted _ (KeyedMap.entries_pairwise_key _)]
      exact KeyedMap.entries_map_fst_nodup _
  -- the empty batch has no champions
  ghost have hch_nil : chOf [] = [] := by
    refine List.eq_nil_iff_forall_not_mem.mpr fun csl hcsl => ?_
    obtain ⟨hk, -⟩ := (hch_mem [] csl).mp hcsl
    exact absurd hk List.not_mem_nil
  -- ===== the ticks, read =====
  -- **the entries tick, opened**: this tick's flattened view
  ghost have hents : ∀ (i : Fin (mem prop)) (gv : Multiset (ALog P (mem prop))),
      (@id (Multiset (Nat × LogValue P (mem prop))) (p_entries_step i () gv).2) = rcEntries gv := by
    intro i gv
    simp only [p_entries_step, den, id, rcEntries, poolFlatMapUnordered, mapPool, Multiset.bind_map,
      HMap.coe_toList]
  -- **a realized ordered tick is a permutation of the view's entries**,
  -- read at its index
  ghost have hord : ∀ (i : Fin (mem prop)) {t : Nat} {l : List (Nat × LogValue P (mem prop))},
      (p_entries_ordered i)[t]? = some l →
      ∃ gv : Multiset (ALog P (mem prop)), (accepted_logs i)[t]? = some gv
        ∧ (↑l : Multiset (Nat × LogValue P (mem prop))) = rcEntries gv := fun i {t l} h => by
    simp only [p_entries_ordered, den] at h
    obtain ⟨hp, -⟩ := realizeTicks_getElem? _ _ h
    obtain ⟨gv, hgv, hpe⟩ := (hp_entries_at i t _).mp hp
    exact ⟨gv, hgv, by rw [hpe]; exact hents i gv⟩
  -- **one tick, opened** (the Rust lines at the denotation): on the
  -- ordered entries `l`, the checkpoint max `C` and the counted champions
  -- `ch`; the commits leg is the champions' try-commit chained with the
  -- holes, the max-slot leg the champions' max slot — stated once, cited
  -- by every per-tick fact below
  ghost have hopen : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop)),
      p_log_chained_step i () (l, (gv, b)) =
        (let C := Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv)
         let ch := chOf l
         let ms := rcMaxSlot variant (ch.map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max? C
         ((),
          (ch.filterMap (fun csl : Nat × Nat × LogValue P (mem prop) =>
              if f < csl.1 then none
              else if (match C with | some c => decide (csl.2.1 ≤ c) | none => false) then none
              else some ((csl.2.1, b), csl.2.2.value))
            ++ (match ms with
              | some maxSlot =>
                (List.range' (match C with | some c => c + 1 | none => 0)
                  (maxSlot - (match C with | some c => c + 1 | none => 0))).filter
                  (fun slot => !(ch.map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).contains slot)
              | none => []).map (fun slot => ((slot, b), (none : Option P))),
           ms))) := by
    intro i l gv b
    simp only [p_log_chained_step, den, chOf]
  -- the tryCommit closure keeps a champion's slot, and emits exactly
  -- `((slot, b), value)` when it emits
  ghost have htry : ∀ (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop))
      (csl : Nat × Nat × LogValue P (mem prop)) (e : (Nat × Ballot (mem prop)) × Option P),
      (if f < csl.1 then none
       else if (match Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) with
         | some c => decide (csl.2.1 ≤ c) | none => false) then none
       else some ((csl.2.1, b), csl.2.2.value)) = some e →
      e = ((csl.2.1, b), csl.2.2.value) := fun gv b csl e hfn => by
    by_cases h1 : f < csl.1
    · rw [if_pos h1] at hfn; cases hfn
    · rw [if_neg h1] at hfn
      by_cases h2 : (match Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) with
        | some c => decide (csl.2.1 ≤ c) | none => false) = true
      · rw [if_pos h2] at hfn; cases hfn
      · rw [if_neg h2] at hfn
        exact (Option.some.inj hfn).symm
  -- the tryCommit closure's refusals: more than `f` holders, or an
  -- executed slot
  ghost have htry_none : ∀ (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop))
      (csl : Nat × Nat × LogValue P (mem prop)),
      (if f < csl.1 then none
       else if (match Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) with
         | some c => decide (csl.2.1 ≤ c) | none => false) then none
       else some ((csl.2.1, b), csl.2.2.value)) = none →
      f < csl.1 ∨ ∃ c, Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) = some c
        ∧ csl.2.1 ≤ c := fun gv b csl hfn => by
    by_cases h1 : f < csl.1
    · exact Or.inl h1
    · rw [if_neg h1] at hfn
      right
      cases hC : Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) with
      | none => rw [hC] at hfn; simp at hfn
      | some c =>
        rw [hC] at hfn
        by_cases h2 : csl.2.1 ≤ c
        · exact ⟨c, rfl, h2⟩
        · simp only [h2, decide_false, Bool.false_eq_true, if_false] at hfn
          cases hfn
  -- **ownership**: every recommit quotes the tick's own ballot
  ghost have howned : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop)),
      ∀ e ∈ (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (l, (gv, b))).2.1),
      (e : (Nat × Ballot (mem prop)) × Option P).1.2 = b := fun i l gv b e he => by
    rw [hopen i l gv b] at he
    simp only [id] at he
    rcases List.mem_append.mp he with h | h
    · obtain ⟨csl, -, hfn⟩ := List.mem_filterMap.mp h
      rw [htry gv b csl e hfn]
    · obtain ⟨slot, -, hslot⟩ := List.mem_map.mp h
      rw [← hslot]
  -- **slots are distinct** within a tick: champions' slots (one per key),
  -- holes (a filter of `range'`), and never both
  ghost have hslots_nodup : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop)),
      ((@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (l, (gv, b))).2.1).map
        (fun e => (e : (Nat × Ballot (mem prop)) × Option P).1.1)).Nodup := fun i l gv b => by
    rw [hopen i l gv b]
    simp only [id, List.map_append]
    refine List.Nodup.append ?_ ?_ ?_
    · refine List.Sublist.nodup (List.map_filterMap_sublist _ _
        (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)
        (fun csl e hfn => by rw [htry gv b csl e hfn]) _) ?_
      exact hch_nodup l
    · rw [List.map_map]
      have hcomp : ((fun e => (e : (Nat × Ballot (mem prop)) × Option P).1.1)
          ∘ (fun slot => ((slot, b), (none : Option P)))) = id := rfl
      rw [hcomp, List.map_id]
      cases rcMaxSlot variant ((chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max?
          (Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv)) with
      | none => exact List.Pairwise.nil
      | some maxSlot => exact List.Nodup.filter _ List.nodup_range'
    · intro a ha hb
      have hprop : a ∈ (chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1) :=
        (List.map_filterMap_sublist _ _
          (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)
          (fun csl e hfn => by rw [htry gv b csl e hfn]) _).subset ha
      rw [List.map_map] at hb
      have hcomp : ((fun e => (e : (Nat × Ballot (mem prop)) × Option P).1.1)
          ∘ (fun slot => ((slot, b), (none : Option P)))) = id := rfl
      rw [hcomp, List.map_id] at hb
      revert hb
      cases rcMaxSlot variant ((chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max?
          (Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv)) with
      | none => intro hb; cases hb
      | some maxSlot =>
        intro hb
        have hnp := (List.mem_filter.mp hb).2
        simp only [Bool.not_eq_true'] at hnp
        have hc : ((chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).contains a = true :=
          List.elem_iff.mpr hprop
        rw [hnp] at hc
        cases hc
  -- **every recommit sits at or below the max slot**: a champion's slot is
  -- below the champions' max; a hole is below it by construction
  ghost have hslot_le_max : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop)),
      ∀ e ∈ (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (l, (gv, b))).2.1),
      ∃ mm, (@id (Option Nat) (p_log_chained_step i () (l, (gv, b))).2.2) = some mm
        ∧ (e : (Nat × Ballot (mem prop)) × Option P).1.1 ≤ mm := fun i l gv b e he => by
    rw [hopen i l gv b] at he ⊢
    simp only [id] at he ⊢
    rcases List.mem_append.mp he with h | h
    · obtain ⟨csl, hcsl, hfn⟩ := List.mem_filterMap.mp h
      have hmem : csl.2.1 ∈ (chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1) :=
        List.mem_map.mpr ⟨csl, hcsl, rfl⟩
      obtain ⟨m, hm, hle⟩ := List.nat_max?_ge _ hmem
      obtain ⟨mm, hmm, hle'⟩ := rcMaxSlot_ge_left variant _ hm
      exact ⟨mm, hmm, by rw [htry gv b csl e hfn]; exact le_trans hle hle'⟩
    · obtain ⟨slot, hslot, hpair⟩ := List.mem_map.mp h
      revert hslot
      cases hmax : rcMaxSlot variant ((chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max?
          (Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv)) with
      | none => intro hslot; cases hslot
      | some maxSlot =>
        intro hslot
        have hmem := List.mem_range'_1.mp (List.mem_filter.mp hslot).1
        refine ⟨maxSlot, rfl, ?_⟩
        rw [← hpair]
        show slot ≤ maxSlot
        omega
  -- **the max slot dominates the view**: a slot with an accepted entry
  -- has a champion, whose slot the max covers
  ghost have hmax_ge : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop)),
      (↑l : Multiset (Nat × LogValue P (mem prop))) = rcEntries gv →
      ∀ (slot : Nat) (e₀ : LogValue P (mem prop)), (slot, e₀) ∈ rcEntries gv →
      ∃ mm, (@id (Option Nat) (p_log_chained_step i () (l, (gv, b))).2.2) = some mm ∧ slot ≤ mm :=
    fun i l gv b hl slot e₀ h => by
    rw [hopen i l gv b]
    simp only [id]
    rw [← hl, Multiset.mem_coe] at h
    obtain ⟨cnt, c, hc⟩ := hch_cover l slot e₀ h
    have hmem : slot ∈ (chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1) :=
      List.mem_map.mpr ⟨(cnt, (slot, c)), hc, rfl⟩
    obtain ⟨m, hm, hle⟩ := List.nat_max?_ge _ hmem
    obtain ⟨mm, hmm, hle'⟩ := rcMaxSlot_ge_left variant _ hm
    exact ⟨mm, hmm, le_trans hle hle'⟩
  -- the tryCommit closure emits only above the view's checkpoint max
  ghost have htry_ckpt : ∀ (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop))
      (csl : Nat × Nat × LogValue P (mem prop)) (e : (Nat × Ballot (mem prop)) × Option P),
      (if f < csl.1 then none
       else if (match Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) with
         | some c => decide (csl.2.1 ≤ c) | none => false) then none
       else some ((csl.2.1, b), csl.2.2.value)) = some e →
      ∀ c, Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) = some c →
      c < csl.2.1 := fun gv b csl e hfn c hc => by
    by_cases h1 : f < csl.1
    · rw [if_pos h1] at hfn; cases hfn
    · rw [if_neg h1] at hfn
      rw [hc] at hfn
      by_cases h2 : csl.2.1 ≤ c
      · simp only [h2, decide_true, if_true] at hfn
        cases hfn
      · exact Nat.lt_of_not_le h2
  -- a reported checkpoint sits under the view's checkpoint max
  ghost have hckpt_le_max : ∀ (gv : Multiset (ALog P (mem prop))), ∀ lg ∈ gv, ∀ c : Nat,
      (lg : ALog P (mem prop)).1 = some c →
      ∃ C, Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) = some C ∧ c ≤ C :=
    fun gv lg hlg c hc =>
    Multiset.foldl_maxStep_ge _ none ((Multiset.mem_filterMap _ _).mpr ⟨lg, hlg, hc⟩)
  -- the view's checkpoint max is a reported checkpoint
  ghost have hckpt_src : ∀ (gv : Multiset (ALog P (mem prop))) (C : Nat),
      Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) = some C →
      ∃ lg ∈ gv, (lg : ALog P (mem prop)).1 = some C := fun gv C hC => by
    rcases Multiset.foldl_maxStep_mem _ none C hC with hm | hm
    · exact (Multiset.mem_filterMap _ _).mp hm
    · cases hm
  -- **the max slot dominates the checkpoints** (guarded, B4): `rcMaxSlot`
  -- lifts the champions' max to the view's checkpoint max
  ghost have hmax_ge_ckpt : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop)),
      variant = .guarded → ∀ lg ∈ gv, ∀ c : Nat,
      (lg : ALog P (mem prop)).1 = some c →
      ∃ mm, (@id (Option Nat) (p_log_chained_step i () (l, (gv, b))).2.2) = some mm ∧ c ≤ mm :=
    fun i l gv b hvar lg hlg c hc => by
    rw [hopen i l gv b]
    simp only [id]
    obtain ⟨C, hC, hcC⟩ := hckpt_le_max gv lg hlg c hc
    obtain ⟨mm, hmm, hle⟩ := rcMaxSlot_ge_ckpt (variant := variant) (by rw [hvar]; rfl) _ hC
    exact ⟨mm, hmm, le_trans hcC hle⟩
  -- **recommits sit above the checkpoints**: a champion passes the
  -- `slot ≤ checkpoint` filter, a hole starts at `checkpoint + 1`
  ghost have habove_ckpt : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop)),
      ∀ e ∈ (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (l, (gv, b))).2.1),
      ∀ lg ∈ gv, ∀ c : Nat, (lg : ALog P (mem prop)).1 = some c →
      c < (e : (Nat × Ballot (mem prop)) × Option P).1.1 := fun i l gv b e he lg hlg c hc => by
    obtain ⟨C, hC, hcC⟩ := hckpt_le_max gv lg hlg c hc
    rw [hopen i l gv b] at he
    simp only [id] at he
    rcases List.mem_append.mp he with h | h
    · obtain ⟨csl, -, hfn⟩ := List.mem_filterMap.mp h
      have hlt := htry_ckpt gv b csl e hfn C hC
      rw [htry gv b csl e hfn]
      exact Nat.lt_of_le_of_lt hcC hlt
    · obtain ⟨slot, hslot, hpair⟩ := List.mem_map.mp h
      rw [← hpair]
      show c < slot
      revert hslot
      cases rcMaxSlot variant ((chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max?
          (Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv)) with
      | none => intro hslot; cases hslot
      | some maxSlot =>
        intro hslot
        have hmem := List.mem_range'_1.mp (List.mem_filter.mp hslot).1
        rw [hC] at hmem
        simp only at hmem
        omega
  -- **the empty view**: no entries, no champions, no max slot, nothing
  -- recommitted
  ghost have hview_empty : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (b : Ballot (mem prop)),
      (↑l : Multiset (Nat × LogValue P (mem prop))) = rcEntries 0 →
      (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (l, ((0 : Multiset (ALog P (mem prop))), b))).2.1) = []
        ∧ (@id (Option Nat) (p_log_chained_step i () (l, ((0 : Multiset (ALog P (mem prop))), b))).2.2) = none :=
    fun i l b hl => by
    have hnil : l = [] := by
      rw [rcEntries, Multiset.zero_bind] at hl
      exact (Multiset.coe_eq_zero l).mp hl
    subst hnil
    have hms : (@id (Option Nat) (p_log_chained_step i () (([] : List (Nat × LogValue P (mem prop))), ((0 : Multiset (ALog P (mem prop))), b))).2.2) = none := by
      rw [hopen i [] 0 b]
      simp only [id, hch_nil, List.map_nil, List.max?_nil, Multiset.filterMap_zero, Multiset.foldl_zero]
      exact rcMaxSlot_none_none variant
    refine ⟨?_, hms⟩
    refine List.eq_nil_iff_forall_not_mem.mpr fun e he => ?_
    obtain ⟨m, hm, -⟩ := hslot_le_max i [] 0 b e he
    rw [hms] at hm
    cases hm
  -- **the champion's value**: a recommit at a covered slot carries its
  -- champion's value — an entry of the view at the slot dominating every
  -- accepted entry there (holes never sit under a covered slot)
  ghost have hvalue_best : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop)),
      (↑l : Multiset (Nat × LogValue P (mem prop))) = rcEntries gv →
      ∀ e ∈ (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (l, (gv, b))).2.1),
      ∀ e₀ : LogValue P (mem prop),
      ((e : (Nat × Ballot (mem prop)) × Option P).1.1, e₀) ∈ rcEntries gv →
      ∃ best : LogValue P (mem prop), (e.1.1, best) ∈ rcEntries gv
        ∧ e.2 = best.value ∧ e₀.ballot.ble best.ballot = true := fun i l gv b hl e he e₀ he₀ => by
    rw [← hl, Multiset.mem_coe] at he₀
    rw [hopen i l gv b] at he
    simp only [id] at he
    rcases List.mem_append.mp he with h | h
    · obtain ⟨csl, hcsl, hfn⟩ := List.mem_filterMap.mp h
      have heq := htry gv b csl e hfn
      subst heq
      obtain ⟨hmem, hdom, -⟩ := hch_champ l csl hcsl
      exact ⟨csl.2.2, by rw [← hl, Multiset.mem_coe]; exact hmem, rfl,
        Ballot.le_iff_ble.mp (hdom e₀ he₀)⟩
    · exfalso
      obtain ⟨slot', hslot', hpair⟩ := List.mem_map.mp h
      have hslot : slot' = e.1.1 := congrArg (fun e => e.1.1) hpair
      revert hslot'
      cases rcMaxSlot variant ((chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max?
          (Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv)) with
      | none => intro hslot'; cases hslot'
      | some maxSlot =>
        intro hslot'
        have hnp := (List.mem_filter.mp hslot').2
        simp only [Bool.not_eq_true'] at hnp
        obtain ⟨cnt, c, hc⟩ := hch_cover l e.1.1 e₀ he₀
        have hcont : ((chOf l).map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).contains slot' = true :=
          List.elem_iff.mpr (by rw [hslot]; exact List.mem_map.mpr ⟨(cnt, (e.1.1, c)), hc, rfl⟩)
        rw [hnp] at hcont
        cases hcont
  -- **why skipping is safe**: a slot with entries but no recommit was
  -- refused by the tryCommit closure — executed, or held by more than `f`
  -- acceptors at the champion's value (the count is below the holders)
  ghost have hskipped : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop)),
      (↑l : Multiset (Nat × LogValue P (mem prop))) = rcEntries gv →
      ∀ (slot : Nat) (e₀ : LogValue P (mem prop)), (slot, e₀) ∈ rcEntries gv →
      (∀ e ∈ (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (l, (gv, b))).2.1),
        (e : (Nat × Ballot (mem prop)) × Option P).1.1 ≠ slot) →
      (∃ c : Nat, (∃ lg ∈ gv, (lg : ALog P (mem prop)).1 = some c) ∧ slot ≤ c)
      ∨ ∃ best : LogValue P (mem prop), (slot, best) ∈ rcEntries gv
          ∧ (∀ e₁ : LogValue P (mem prop), (slot, e₁) ∈ rcEntries gv → e₁.ballot.ble best.ballot = true)
          ∧ f < ((rcEntries gv).filter (fun e => e.1 = slot ∧ e.2.value = best.value)).card :=
    fun i l gv b hl slot e₀ he₀ hno => by
    rw [← hl, Multiset.mem_coe] at he₀
    obtain ⟨cnt, c, hc⟩ := hch_cover l slot e₀ he₀
    obtain ⟨hmem, hdom, hcnt⟩ := hch_champ l (cnt, (slot, c)) hc
    -- the closure refused the champion (else its recommit would sit at `slot`)
    have hrefused : (if f < (cnt, (slot, c)).1 then none
        else if (match Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) with
          | some c' => decide ((cnt, (slot, c)).2.1 ≤ c') | none => false) then none
        else some (((cnt, (slot, c)).2.1, b), (cnt, (slot, c)).2.2.value)) = none := by
      cases hfn : (if f < (cnt, (slot, c)).1 then none
          else if (match Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv) with
            | some c' => decide ((cnt, (slot, c)).2.1 ≤ c') | none => false) then none
          else some (((cnt, (slot, c)).2.1, b), (cnt, (slot, c)).2.2.value)) with
      | none => rfl
      | some e =>
        exfalso
        refine hno e ?_ (by rw [htry gv b _ e hfn])
        rw [hopen i l gv b]
        simp only [id]
        exact List.mem_append_left _ (List.mem_filterMap.mpr ⟨_, hc, hfn⟩)
    rcases htry_none gv b _ hrefused with hcount | ⟨C, hC, hle⟩
    · right
      refine ⟨c, by rw [← hl, Multiset.mem_coe]; exact hmem,
        fun e₁ he₁ => Ballot.le_iff_ble.mp (hdom e₁ (by rw [← hl, Multiset.mem_coe] at he₁; exact he₁)), ?_⟩
      rw [← hl, Multiset.filter_coe, Multiset.coe_card]
      exact Nat.lt_of_lt_of_le hcount hcnt
    · left
      exact ⟨C, hckpt_src gv C hC, hle⟩
  -- the tick's two legs, bundled: one `RCTick`
  ghost have htick : ∀ (i : Fin (mem prop)) (l : List (Nat × LogValue P (mem prop)))
      (gv : Multiset (ALog P (mem prop))) (b : Ballot (mem prop)),
      (↑l : Multiset (Nat × LogValue P (mem prop))) = rcEntries gv →
      RCTick variant f gv b (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (l, (gv, b))).2.1)
        (@id (Option Nat) (p_log_chained_step i () (l, (gv, b))).2.2) := fun i l gv b hl =>
    { owned := howned i l gv b
      slots_nodup := hslots_nodup i l gv b
      slot_le_max := hslot_le_max i l gv b
      max_ge := hmax_ge i l gv b hl
      max_ge_ckpt := hmax_ge_ckpt i l gv b
      above_ckpt := habove_ckpt i l gv b
      view_empty := fun hgv => by subst hgv; exact hview_empty i l b hl
      value_best := hvalue_best i l gv b hl
      skipped_holders := hskipped i l gv b hl }
  (p_log_chained, p_max_slot)
  prove
    commits_at := fun i {t rcl} hrcl => by
      obtain ⟨l, gv, b, hl, hg, hb, rfl⟩ := (hp_log_chained_at i t rcl).mp hrcl
      obtain ⟨gv', hg', hperm⟩ := hord i hl
      obtain rfl := Trace.read_inj hg hg'
      exact ⟨gv, b, _, hg, hb, (hp_max_slot_at i t _).mpr ⟨l, gv, b, hl, hg, hb, rfl⟩, htick i l gv b hperm⟩,
    maxslot_at := fun i {t m} hm => by
      obtain ⟨l, gv, b, hl, hg, hb, rfl⟩ := (hp_max_slot_at i t m).mp hm
      obtain ⟨gv', hg', hperm⟩ := hord i hl
      obtain rfl := Trace.read_inj hg hg'
      exact ⟨gv, b, _, hg, hb, (hp_log_chained_at i t _).mpr ⟨l, gv, b, hl, hg, hb, rfl⟩, htick i l gv b hperm⟩

/-! ## Executable smoke tests (the program at the denotation, one tick) -/

-- A quorum of two logs: slot 0's champion is the ballot-1 entry (value
-- 9); slot 2 forces a hole at slot 1; everything recommits at the new
-- leader's ballot 3.
#guard_eval (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .guarded .prop
    (.mk' fun _ => [({((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 1 0, some 9⟩)]),
      ((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 0 0, some 4⟩), (2, ⟨Ballot.mk 0 0, some 7⟩)])}
      : Multiset (ALog Nat 1))])
    (.mk' fun _ => [Ballot.mk 3 0]) 1
    ⟨fun _ => [[(0, ⟨Ballot.mk 0 0, some 4⟩), (2, ⟨Ballot.mk 0 0, some 7⟩), (0, ⟨Ballot.mk 1 0, some 9⟩)]]⟩).1 0
  = [[((0, Ballot.mk 3 0), some 9), ((2, Ballot.mk 3 0), some 7),
      ((1, Ballot.mk 3 0), none)]]

-- The same view in the other arrival order: the champion is the same
-- (the max ballot is unique here).
#guard_eval (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .guarded .prop
    (.mk' fun _ => [({((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 1 0, some 9⟩)]),
      ((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 0 0, some 4⟩), (2, ⟨Ballot.mk 0 0, some 7⟩)])}
      : Multiset (ALog Nat 1))])
    (.mk' fun _ => [Ballot.mk 3 0]) 1
    ⟨fun _ => [[(0, ⟨Ballot.mk 1 0, some 9⟩), (2, ⟨Ballot.mk 0 0, some 7⟩), (0, ⟨Ballot.mk 0 0, some 4⟩)]]⟩).1 0
  = [[((0, Ballot.mk 3 0), some 9), ((2, Ballot.mk 3 0), some 7),
      ((1, Ballot.mk 3 0), none)]]

-- An order decision that is not a permutation of the tick's entries does
-- not realize the tick: nothing is published.
#guard_eval (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .guarded .prop
    (.mk' fun _ => [({((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 1 0, some 9⟩)])} : Multiset (ALog Nat 1))])
    (.mk' fun _ => [Ballot.mk 3 0]) 1 ⟨fun _ => [[]]⟩).1 0 = []

-- A value already on more than `f` acceptors is not recommitted.
#guard_eval (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .guarded .prop
    (.mk' fun _ => [({((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 1 0, some 9⟩)]),
      ((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 1 0, some 9⟩)])} : Multiset (ALog Nat 1))])
    (.mk' fun _ => [Ballot.mk 3 0]) 1
    ⟨fun _ => [[(0, ⟨Ballot.mk 1 0, some 9⟩), (0, ⟨Ballot.mk 1 0, some 9⟩)]]⟩).1 0 = [[]]

-- Checkpointed slots are skipped (the checkpoint travels in the logs).
#guard_eval (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .guarded .prop
    (.mk' fun _ => [({((some 0 : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 1 0, some 9⟩)])} : Multiset (ALog Nat 1))])
    (.mk' fun _ => [Ballot.mk 3 0]) 1 ⟨fun _ => [[(0, ⟨Ballot.mk 1 0, some 9⟩)]]⟩).1 0 = [[]]

-- The max slot of the recovered view.
#guard_eval (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .guarded .prop
    (.mk' fun _ => [({((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 1 0, some 9⟩),
      (2, ⟨Ballot.mk 0 0, some 7⟩)])} : Multiset (ALog Nat 1))])
    (.mk' fun _ => [Ballot.mk 3 0]) 1
    ⟨fun _ => [[(0, ⟨Ballot.mk 1 0, some 9⟩), (2, ⟨Ballot.mk 0 0, some 7⟩)]]⟩).2 0 = [some 2]

/-! The X·Y·Z witness (FINDINGS D71): one slot, `f = 1`, three logs —
X = (ballot 1, v), Z = (ballot 2, w), Y = (ballot 3, v). Order X·Y·Z: the
champion `v` is counted twice (X, then Y at the higher ballot keeps the
value) → `count 2 > f` → SKIP. Order X·Z·Y: Z resets the count to one
with `w`, Y resets it to one with `v` → `count 1` → RECOMMIT `v`. Same
multiset, same champion, opposite decisions; both safe. -/
#guard_eval (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .guarded .prop
    (.mk' fun _ => [({((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 1 0, some 5⟩)]),
      ((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 2 0, some 6⟩)]),
      ((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 3 0, some 5⟩)])} : Multiset (ALog Nat 1))])
    (.mk' fun _ => [Ballot.mk 4 0]) 1
    ⟨fun _ => [[(0, ⟨Ballot.mk 1 0, some 5⟩), (0, ⟨Ballot.mk 3 0, some 5⟩),
      (0, ⟨Ballot.mk 2 0, some 6⟩)]]⟩).1 0 = [[]]
#guard_eval (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .guarded .prop
    (.mk' fun _ => [({((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 1 0, some 5⟩)]),
      ((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 2 0, some 6⟩)]),
      ((none : Option Nat), HMap.ofList [(0, ⟨Ballot.mk 3 0, some 5⟩)])} : Multiset (ALog Nat 1))])
    (.mk' fun _ => [Ballot.mk 4 0]) 1
    ⟨fun _ => [[(0, ⟨Ballot.mk 1 0, some 5⟩), (0, ⟨Ballot.mk 2 0, some 6⟩),
      (0, ⟨Ballot.mk 3 0, some 5⟩)]]⟩).1 0 = [[((0, Ballot.mk 4 0), some 5)]]

-- The Rust `nondet!` tally at paxos.rs:596–672 after D71: the fold's
-- `assume_ordering` (one content site), no scheds, no cycles.
#nondet_census recommit_after_leader_election (nondets := 1) (scheds := 0) (fuels := 0)

end Hydro
