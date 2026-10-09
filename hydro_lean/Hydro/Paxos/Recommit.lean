import Hydro.MonoRel
import Hydro.Paxos.Types
import Hydro.HydroTick

/-!
# `recommit_after_leader_election` (paxos.rs:595–672)

A fresh leader reconciles the quorum's accepted logs: per slot, keep the
max-ballot entry (recommitting it under the new ballot unless already
committed on more than `f` acceptors or checkpointed away), and fill the
log holes below the max slot with no-ops.

The per-slot fold carries the same `commutative = manual_proof!(TODO)`
hole as `acceptor_p2`'s log merge; the model computes the whole tick
value as **canonical functions of the batch multiset** (`logView` for
the champions, cardinality for the counts) — functions out of the
quotient are order-safe by construction, so batch-arrival order cannot
leak.

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

/-- **One tick of the recommit computation**, in the consumer's
vocabulary: at a tick whose input batch is `gv` and whose ballot is `b`,
the module publishes the recommit list `rcl` and the max slot `m`
(paxos.rs:611–670). -/
structure RCTick {nP : Nat} (f : Nat) (gv : Multiset (ALog P nP)) (b : Ballot nP)
    (rcl : List ((Nat × Ballot nP) × Option P)) (m : Option Nat) : Prop where
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
  /-- **An empty view recommits nothing** and has no max slot. -/
  view_empty : gv = 0 → rcl = [] ∧ m = none
  /-- **The champion's value**: a recommit at a slot carries the value of
  the view's max-ballot entry there, which dominates every accepted entry
  at the slot. -/
  value_best : ∀ e ∈ rcl, ∀ e₀ : LogValue P nP,
    ((e : (Nat × Ballot nP) × Option P).1.1, e₀) ∈ rcEntries gv →
    ∃ best : LogValue P nP, (e.1.1, best) ∈ logView (rcEntries gv)
      ∧ e.2 = best.value ∧ e₀.ballot.ble best.ballot = true

/-- What `recommit_after_leader_election` **ensures**, over the `Values`
denotation, on its output wires: a published tick is an input tick's
`RCTick`, read from either side. -/
structure RCEnsures (prop : L) (f : Nat)
    (bs : Fin (mem prop) → Trace (Multiset (ALog P (mem prop))))
    (pb : TickV (mem prop) (Ballot (mem prop)))
    (out : TickV (mem prop) (List ((Nat × Ballot (mem prop)) × Option P))
      × TickV (mem prop) (Option Nat)) : Prop where
  /-- **Read from the inputs**: a tick with a batch and a ballot publishes
  its recommit list and max slot, an `RCTick`. -/
  tick_at : ∀ (i : Fin (mem prop)) {t : Nat} {gv : Multiset (ALog P (mem prop))}
    {b : Ballot (mem prop)},
    (bs i)[t]? = some gv → (pb i)[t]? = some b →
    ∃ rcl m, (out.1 i)[t]? = some rcl ∧ (out.2 i)[t]? = some m ∧ RCTick f gv b rcl m
  /-- **Read from the commits**: a published recommit list is its tick's
  `RCTick` against the tick's batch and ballot. -/
  commits_at : ∀ (i : Fin (mem prop)) {t : Nat}
    {rcl : List ((Nat × Ballot (mem prop)) × Option P)},
    (out.1 i)[t]? = some rcl →
    ∃ gv b m, (bs i)[t]? = some gv ∧ (pb i)[t]? = some b ∧ (out.2 i)[t]? = some m
      ∧ RCTick f gv b rcl m
  /-- **Read from the max slots**: likewise. -/
  maxslot_at : ∀ (i : Fin (mem prop)) {t : Nat} {m : Option Nat},
    (out.2 i)[t]? = some m →
    ∃ gv b rcl, (bs i)[t]? = some gv ∧ (pb i)[t]? = some b ∧ (out.1 i)[t]? = some rcl
      ∧ RCTick f gv b rcl m

/-- **paxos.rs:595–672 `recommit_after_leader_election`** over the
proposer cluster. Returns (`p_log_to_try_commit.chain(p_log_holes)`,
`p_max_slot`) per tick. -/
hydro def recommit_after_leader_election (H : HydroSem L mem) (prop : L)
    (accepted_logs :
      H.TickStream prop (ALog P (mem prop)) .noOrder .exactlyOnce)
    (p_ballot : H.Ticked prop (Ballot (mem prop)))
    (f : Nat) :
    (H.Ticked prop (List ((Nat × Ballot (mem prop)) × Option P))
      × H.Ticked prop (Option Nat))
  ensures out => RCEnsures prop f accepted_logs p_ballot out :=
  -- the whole function is tick-located dataflow over this tick's
  -- `accepted_logs` batch and `p_ballot`: one stateless tick block
  tick (input logs := accepted_logs) (input pb := p_ballot) :=
    -- let p_p1b_max_checkpoint = accepted_logs.clone()
    --   .filter_map(q!(|(checkpoint, _log)| checkpoint))
    --   .max()
    --   .into_singleton();
    let p_p1b_max_checkpoint := H.bmax (H.bfilterMap logs (fun lg => lg.1))
    -- let p_p1b_highest_entries_and_count = accepted_logs
    --   .map(q!(|(_checkpoint, log)| log))
    --   .flatten_unordered() // Convert HashMap log back to stream
    let ents := H.bflatMapUnordered (H.bmap logs (fun lg => lg.2))
      (fun log => H.bofList log)
    --   .into_keyed()
    --   .fold(q!(|| (0, None)), q!(|curr_entry, new_entry| { … },
    --     commutative = manual_proof!(/** TODO */)))
    --   .map(q!(|(count, entry)| (count, entry.unwrap())));
    -- (the fold's `manual_proof!` hole, made honest: the per-slot champion
    -- and its count collapse onto the canonical view of the POOLED
    -- entries — `logView` + `rcCount`; equal-ballot value conflicts
    -- degrade to `none` instead of depending on arrival order)
    let pooled := H.bfold (fun s e => s + {e}) 0
      (fun s x y => add_singleton_comm s x y) ents
    let p_p1b_highest_entries_and_count := H.bsMap pooled
      (fun E => (logView E).map (fun sl => (rcCount E sl.1 sl.2.value, sl)))
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
    let p_max_slot := H.bsMap p_p1b_highest_entries_and_count
      (fun ch => (ch.map
        (fun (csl : Nat × Nat × LogValue P (mem prop)) => csl.2.1)).max?)
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
  -- **one tick, opened** (the Rust lines at the denotation): the pooled
  -- entries `E`, the checkpoint max `C`, the counted champions `ch`; the
  -- commits leg is the champions' try-commit chained with the holes, the
  -- max-slot leg the champions' max slot — stated once, cited by every
  -- per-tick fact below
  ghost have hopen : ∀ (i : Fin (mem prop)) (gv : Multiset (ALog P (mem prop)))
      (b : Ballot (mem prop)),
      p_log_chained_step i () (gv, b) =
        (let E := rcEntries gv
         let C := Multiset.foldl maxStep none (Multiset.filterMap (fun lg => lg.1) gv)
         let ch := (logView E).map (fun sl => (rcCount E sl.1 sl.2.value, sl))
         let ms := (ch.map (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max?
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
    intro i gv b
    simp only [p_log_chained_step, den, foldl_add_singleton, Multiset.zero_add,
      Multiset.bind_map, rcEntries]
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
  -- **ownership**: every recommit quotes the tick's own ballot
  ghost have howned : ∀ (i : Fin (mem prop)) (gv : Multiset (ALog P (mem prop)))
      (b : Ballot (mem prop)), ∀ e ∈ (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (gv, b)).2.1),
      (e : (Nat × Ballot (mem prop)) × Option P).1.2 = b := fun i gv b e he => by
    rw [hopen i gv b] at he
    simp only [id] at he
    rcases List.mem_append.mp he with h | h
    · obtain ⟨csl, -, hfn⟩ := List.mem_filterMap.mp h
      rw [htry gv b csl e hfn]
    · obtain ⟨slot, -, hslot⟩ := List.mem_map.mp h
      rw [← hslot]
  -- **slots are distinct** within a tick: champions' slots (`logView` is
  -- one per slot), holes (a filter of `range'`), and never both
  ghost have hslots_nodup : ∀ (i : Fin (mem prop)) (gv : Multiset (ALog P (mem prop)))
      (b : Ballot (mem prop)),
      ((@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (gv, b)).2.1).map
        (fun e => (e : (Nat × Ballot (mem prop)) × Option P).1.1)).Nodup := fun i gv b => by
    rw [hopen i gv b]
    simp only [id, List.map_append]
    refine List.Nodup.append ?_ ?_ ?_
    · refine List.Sublist.nodup (List.map_filterMap_sublist _ _
        (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)
        (fun csl e hfn => by rw [htry gv b csl e hfn]) _) ?_
      rw [List.map_map]
      exact logView_keys_nodup (rcEntries gv)
    · rw [List.map_map]
      have hcomp : ((fun e => (e : (Nat × Ballot (mem prop)) × Option P).1.1)
          ∘ (fun slot => ((slot, b), (none : Option P)))) = id := rfl
      rw [hcomp, List.map_id]
      cases (((logView (rcEntries gv)).map (fun sl => (rcCount (rcEntries gv) sl.1 sl.2.value, sl))).map
          (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max? with
      | none => exact List.Pairwise.nil
      | some maxSlot => exact List.Nodup.filter _ List.nodup_range'
    · intro a ha hb
      have hprop : a ∈ ((logView (rcEntries gv)).map
          (fun sl => (rcCount (rcEntries gv) sl.1 sl.2.value, sl))).map
          (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1) :=
        (List.map_filterMap_sublist _ _
          (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)
          (fun csl e hfn => by rw [htry gv b csl e hfn]) _).subset ha
      rw [List.map_map] at hb
      have hcomp : ((fun e => (e : (Nat × Ballot (mem prop)) × Option P).1.1)
          ∘ (fun slot => ((slot, b), (none : Option P)))) = id := rfl
      rw [hcomp, List.map_id] at hb
      revert hb
      cases (((logView (rcEntries gv)).map (fun sl => (rcCount (rcEntries gv) sl.1 sl.2.value, sl))).map
          (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max? with
      | none => intro hb; cases hb
      | some maxSlot =>
        intro hb
        have hnp := (List.mem_filter.mp hb).2
        simp only [Bool.not_eq_true'] at hnp
        have hc : (((logView (rcEntries gv)).map (fun sl => (rcCount (rcEntries gv) sl.1 sl.2.value, sl))).map
            (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).contains a = true :=
          List.elem_iff.mpr hprop
        rw [hnp] at hc
        cases hc
  -- **every recommit sits at or below the max slot**: a champion's slot is
  -- below the champions' max; a hole is below it by construction
  ghost have hslot_le_max : ∀ (i : Fin (mem prop)) (gv : Multiset (ALog P (mem prop)))
      (b : Ballot (mem prop)), ∀ e ∈ (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (gv, b)).2.1),
      ∃ mm, (@id (Option Nat) (p_log_chained_step i () (gv, b)).2.2) = some mm
        ∧ (e : (Nat × Ballot (mem prop)) × Option P).1.1 ≤ mm := fun i gv b e he => by
    rw [hopen i gv b] at he ⊢
    simp only [id] at he ⊢
    rcases List.mem_append.mp he with h | h
    · obtain ⟨csl, hcsl, hfn⟩ := List.mem_filterMap.mp h
      have hmem : csl.2.1 ∈ (((logView (rcEntries gv)).map (fun sl => (rcCount (rcEntries gv) sl.1 sl.2.value, sl))).map
          (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)) := List.mem_map.mpr ⟨csl, hcsl, rfl⟩
      obtain ⟨m, hm, hle⟩ := List.nat_max?_ge _ hmem
      exact ⟨m, hm, by rw [htry gv b csl e hfn]; exact hle⟩
    · obtain ⟨slot, hslot, hpair⟩ := List.mem_map.mp h
      revert hslot
      cases hmax : (((logView (rcEntries gv)).map (fun sl => (rcCount (rcEntries gv) sl.1 sl.2.value, sl))).map
          (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max? with
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
  ghost have hmax_ge : ∀ (i : Fin (mem prop)) (gv : Multiset (ALog P (mem prop)))
      (b : Ballot (mem prop)) (slot : Nat) (e₀ : LogValue P (mem prop)),
      (slot, e₀) ∈ rcEntries gv →
      ∃ mm, (@id (Option Nat) (p_log_chained_step i () (gv, b)).2.2) = some mm ∧ slot ≤ mm := fun i gv b slot e₀ h => by
    rw [hopen i gv b]
    simp only [id]
    obtain ⟨e', he', -⟩ := logView_covers (rcEntries gv) h
    have hmem : slot ∈ (((logView (rcEntries gv)).map (fun sl => (rcCount (rcEntries gv) sl.1 sl.2.value, sl))).map
          (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)) := List.mem_map.mpr
      ⟨(rcCount (rcEntries gv) slot e'.value, (slot, e')),
        List.mem_map.mpr ⟨(slot, e'), he', rfl⟩, rfl⟩
    exact List.nat_max?_ge _ hmem
  -- **the empty view**: no champions, no max slot, nothing recommitted
  ghost have hview_empty : ∀ (i : Fin (mem prop)) (b : Ballot (mem prop)),
      (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () ((0 : Multiset (ALog P (mem prop))), b)).2.1) = []
        ∧ (@id (Option Nat) (p_log_chained_step i () ((0 : Multiset (ALog P (mem prop))), b)).2.2) = none := fun i b => by
    have hms : (@id (Option Nat) (p_log_chained_step i () ((0 : Multiset (ALog P (mem prop))), b)).2.2) = none := by
      rw [hopen i 0 b]
      simp [rcEntries, logView]
    refine ⟨?_, hms⟩
    refine List.eq_nil_iff_forall_not_mem.mpr fun e he => ?_
    obtain ⟨m, hm, -⟩ := hslot_le_max i 0 b e he
    rw [hms] at hm
    cases hm
  -- **the champion's value**: a recommit at a covered slot carries the
  -- slot's `logView` champion value, which dominates every accepted entry
  -- there (holes never sit under a covered slot)
  ghost have hvalue_best : ∀ (i : Fin (mem prop)) (gv : Multiset (ALog P (mem prop)))
      (b : Ballot (mem prop)), ∀ e ∈ (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (gv, b)).2.1),
      ∀ e₀ : LogValue P (mem prop),
      ((e : (Nat × Ballot (mem prop)) × Option P).1.1, e₀) ∈ rcEntries gv →
      ∃ best : LogValue P (mem prop), (e.1.1, best) ∈ logView (rcEntries gv)
        ∧ e.2 = best.value ∧ e₀.ballot.ble best.ballot = true := fun i gv b e he e₀ he₀ => by
    obtain ⟨e', he', hble⟩ := logView_covers (rcEntries gv) he₀
    rw [hopen i gv b] at he
    simp only [id] at he
    rcases List.mem_append.mp he with h | h
    · obtain ⟨csl, hcsl, hfn⟩ := List.mem_filterMap.mp h
      have heq := htry gv b csl e hfn
      obtain ⟨sl, hsl, rfl⟩ := List.mem_map.mp hcsl
      subst heq
      have huniq : sl.2 = e' :=
        List.eq_of_keys_nodup (logView_keys_nodup (rcEntries gv)) hsl he'
      exact ⟨sl.2, hsl, rfl, by rw [huniq]; exact hble⟩
    · exfalso
      obtain ⟨slot', hslot', hpair⟩ := List.mem_map.mp h
      have hslot : slot' = e.1.1 := congrArg (fun e => e.1.1) hpair
      revert hslot'
      cases (((logView (rcEntries gv)).map (fun sl => (rcCount (rcEntries gv) sl.1 sl.2.value, sl))).map
          (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).max? with
      | none => intro hslot'; cases hslot'
      | some maxSlot =>
        intro hslot'
        have hnp := (List.mem_filter.mp hslot').2
        simp only [Bool.not_eq_true'] at hnp
        have hc : (((logView (rcEntries gv)).map
            (fun sl => (rcCount (rcEntries gv) sl.1 sl.2.value, sl))).map
            (fun csl : Nat × Nat × LogValue P (mem prop) => csl.2.1)).contains slot' = true :=
          List.elem_iff.mpr (by
            rw [hslot]
            exact List.mem_map.mpr ⟨(rcCount (rcEntries gv) e.1.1 e'.value, (e.1.1, e')),
              List.mem_map.mpr ⟨(e.1.1, e'), he', rfl⟩, rfl⟩)
        rw [hnp] at hc
        cases hc
  -- the tick's two legs, bundled: one `RCTick`
  ghost have htick : ∀ (i : Fin (mem prop)) (gv : Multiset (ALog P (mem prop)))
      (b : Ballot (mem prop)),
      RCTick f gv b (@id (List ((Nat × Ballot (mem prop)) × Option P)) (p_log_chained_step i () (gv, b)).2.1)
        (@id (Option Nat) (p_log_chained_step i () (gv, b)).2.2) := fun i gv b =>
    { owned := howned i gv b
      slots_nodup := hslots_nodup i gv b
      slot_le_max := hslot_le_max i gv b
      max_ge := hmax_ge i gv b
      view_empty := fun hgv => by subst hgv; exact hview_empty i b
      value_best := hvalue_best i gv b }
  (p_log_chained, p_max_slot)
  prove
    tick_at := fun i {t gv b} hg hb =>
      ⟨_, _, (hp_log_chained_at i t _).mpr ⟨gv, b, hg, hb, rfl⟩,
        (hp_max_slot_at i t _).mpr ⟨gv, b, hg, hb, rfl⟩, htick i gv b⟩,
    commits_at := fun i {t rcl} hrcl => by
      obtain ⟨gv, b, hg, hb, rfl⟩ := (hp_log_chained_at i t rcl).mp hrcl
      exact ⟨gv, b, _, hg, hb, (hp_max_slot_at i t _).mpr ⟨gv, b, hg, hb, rfl⟩, htick i gv b⟩,
    maxslot_at := fun i {t m} hm => by
      obtain ⟨gv, b, hg, hb, rfl⟩ := (hp_max_slot_at i t m).mp hm
      exact ⟨gv, b, _, hg, hb, (hp_log_chained_at i t _).mpr ⟨gv, b, hg, hb, rfl⟩, htick i gv b⟩

/-! ## Executable smoke tests (the program at the denotation, one tick) -/

-- A quorum of two logs: slot 0's champion is the ballot-1 entry (value
-- 9); slot 2 forces a hole at slot 1; everything recommits at the new
-- leader's ballot 3.
#guard (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => [({((none : Option Nat), [(0, ⟨Ballot.mk 1 0, some 9⟩)]),
      ((none : Option Nat), [(0, ⟨Ballot.mk 0 0, some 4⟩), (2, ⟨Ballot.mk 0 0, some 7⟩)])}
      : Multiset (ALog Nat 1))])
    (fun _ => [Ballot.mk 3 0]) 1).1 0
  = [[((0, Ballot.mk 3 0), some 9), ((2, Ballot.mk 3 0), some 7),
      ((1, Ballot.mk 3 0), none)]]

-- A value already on more than `f` acceptors is not recommitted.
#guard (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => [({((none : Option Nat), [(0, ⟨Ballot.mk 1 0, some 9⟩)]),
      ((none : Option Nat), [(0, ⟨Ballot.mk 1 0, some 9⟩)])} : Multiset (ALog Nat 1))])
    (fun _ => [Ballot.mk 3 0]) 1).1 0 = [[]]

-- Checkpointed slots are skipped (the checkpoint travels in the logs).
#guard (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => [({((some 0 : Option Nat), [(0, ⟨Ballot.mk 1 0, some 9⟩)])} : Multiset (ALog Nat 1))])
    (fun _ => [Ballot.mk 3 0]) 1).1 0 = [[]]

-- The max slot of the recovered view.
#guard (recommit_after_leader_election (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => [({((none : Option Nat), [(0, ⟨Ballot.mk 1 0, some 9⟩),
      (2, ⟨Ballot.mk 0 0, some 7⟩)])} : Multiset (ALog Nat 1))])
    (fun _ => [Ballot.mk 3 0]) 1).2 0 = [some 2]

end Hydro
