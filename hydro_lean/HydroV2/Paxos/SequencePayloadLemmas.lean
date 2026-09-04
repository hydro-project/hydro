import HydroV2.Paxos.Recommit
import HydroV2.Paxos.IndexPayloads
import HydroV2.Paxos.AcceptorP2

/-!
# `sequence_payload` · the pure sequencing layer and its lemmas

The commit-collection closures (`collect_quorum` + `join_responses` as
canonical multiset functions), the sent-trace vocabulary (the leader's
pipeline `gate → recommit → index → stamp` as one pure function, plus
its fused single-scan form), the guarded key calculus (B2 + slot
freshness ⇒ send-once per key), and the devices `paxos_core` consumes
(`SPEmission`/`SPChosen`/`SPRequires`). The program text and its
colocated contract live in `SequencePayload.lean`.
-/

namespace HydroV2

variable {P : Type} [DecidableEq P]

/-- The B2 recommit-gate step (`use::state`; FINDINGS B2): fire on a
nonempty view when **freshly** leading at a **new** ballot; the faithful
variant fires on every nonempty view. State: the ballot last
recommitted at. Input: (tick's view batch, ((ballot, leader), leader
deferred a tick)). -/
def spGateStep {nP : Nat} (recommitOnce : Bool)
    (recommittedAt : Option (Ballot nP))
    (logs : Multiset (ALog P nP)) (x : (Ballot nP × Bool) × Bool) :
    Option (Ballot nP) × Multiset (ALog P nP) :=
  let fire := decide (logs ≠ 0) &&
    (!recommitOnce ||
      (x.1.2 && !x.2 && !decide (recommittedAt = some x.1.1)))
  ((if fire then some x.1.1 else recommittedAt),
   if fire then logs else 0)

/-- `Ok` votes for a key in the collected P2b pool
(`collect_quorum`'s count). -/
def spOkCount {nP : Nat} (pool : Multiset (P2b nP))
    (k : Nat × Ballot nP) : Nat :=
  (pool.filter (fun m => m.slot = k.1 ∧ m.ballot = k.2
    ∧ m.res = .ok ())).card

/-- The keys crossing the quorum threshold on this batch (emitted once:
below the bar before, at or above it after). -/
def spNewCommits {nP : Nat} (f : Nat) (old : Multiset (P2b nP))
    (batch : Multiset (P2b nP)) : Multiset (Nat × Ballot nP) :=
  ((batch.map (fun m => (m.slot, m.ballot))).dedup.filter
    (fun k => ¬ f < spOkCount old k ∧ f < spOkCount (old + batch) k))

/-- One commit-collection tick (`collect_quorum` + `join_responses`):
state is (accumulated P2b pool, sent metadata); the batch is consumed
as its multiset — order-safe by construction. -/
def spCommitStep {nP : Nat} (f : Nat)
    (st : Multiset (P2b nP) × List ((Nat × Ballot nP) × Option P))
    (batch : Multiset (P2b nP))
    (sentNow : List ((Nat × Ballot nP) × Option P)) :
    (Multiset (P2b nP) × List ((Nat × Ballot nP) × Option P))
      × Multiset (Nat × Option P) :=
  let sent' := st.2 ++ sentNow
  ((st.1 + batch, sent'),
   (spNewCommits f st.1 batch).filterMap (fun k =>
     (List.find? (fun e => e.1 = k) sent').map (fun e => (k.1, e.2))))


/-! ## The commit calculus (pure faces of `collect_quorum` +
`join_responses`) -/

/-- **Commits need quorums**: an emitted key holds more than `f` `Ok`
votes in the accumulated pool. -/
theorem spNewCommits_quorum {nP : Nat} (f : Nat)
    (old batch : Multiset (P2b nP)) (k : Nat × Ballot nP)
    (hk : k ∈ spNewCommits f old batch) :
    f < spOkCount (old + batch) k := by
  unfold spNewCommits at hk
  exact (Multiset.of_mem_filter hk).2

/-- **Commits are emitted once**: a key already at quorum is never
re-emitted. -/
theorem spNewCommits_fresh {nP : Nat} (f : Nat)
    (old batch : Multiset (P2b nP)) (k : Nat × Ballot nP)
    (hk : k ∈ spNewCommits f old batch) :
    ¬ f < spOkCount old k := by
  unfold spNewCommits at hk
  exact (Multiset.of_mem_filter hk).1

/-- **Replica values are the leader's own**: every joined output quotes
the sent metadata for its key. -/
theorem spCommitStep_join {nP : Nat} (f : Nat)
    (st : Multiset (P2b nP) × List ((Nat × Ballot nP) × Option P))
    (batch : Multiset (P2b nP))
    (sentNow : List ((Nat × Ballot nP) × Option P))
    (sv : Nat × Option P)
    (hsv : sv ∈ (spCommitStep f st batch sentNow).2) :
    ∃ k ∈ spNewCommits f st.1 batch, ∃ e ∈ st.2 ++ sentNow,
      (e : (Nat × Ballot nP) × Option P).1 = k ∧ sv = (k.1, e.2) := by
  unfold spCommitStep at hsv
  obtain ⟨k, hk, hjoin⟩ := (Multiset.mem_filterMap _ _).mp hsv
  cases hfind : List.find? (fun e => e.1 = k) (st.2 ++ sentNow) with
  | none =>
    rw [hfind] at hjoin
    cases hjoin
  | some e =>
    rw [hfind] at hjoin
    injection hjoin with hjoin'
    refine ⟨k, hk, e, List.mem_of_find?_eq_some hfind, ?_, ?_⟩
    · have := List.find?_some hfind
      exact of_decide_eq_true this
    · rw [← hjoin']


/-! ## The sent-trace vocabulary (the leader's realized emissions)

The pipeline `gate → recommit → index → stamp` as one pure function of
the member's inputs, plus its fused single-scan form (`spSendStep`) —
the induction handle for the guarded key calculus. -/

/-- The realized gated-view trace (the B2 gate's emissions). -/
def spGatedTrace {nP : Nat} (recommitOnce : Bool)
    (pb : Trace (Ballot nP)) (pl : Trace Bool)
    (p1bs : Trace (Multiset (ALog P nP))) :
    Trace (Multiset (ALog P nP)) :=
  scanAcrossTicksTrace
    (fun s bt => spGateStep recommitOnce s bt.1 bt.2) none
    (Trace.zip p1bs (Trace.zip (Trace.zip pb pl) (false :: pl)))

/-- The realized sent trace (`payloads_to_send`): per tick, the indexed
fresh payloads and the recommit list, keyed by the tick's ballot, when
leading. -/
def spSentTrace {nP : Nat} (variant : PaxosVariant) (f : Nat)
    (cp : List P) (dPayload : List Nat)
    (pb : Trace (Ballot nP)) (pl : Trace Bool)
    (p1bs : Trace (Multiset (ALog P nP))) :
    Trace (List ((Nat × Ballot nP) × Option P)) :=
  let gv := spGatedTrace variant.recommitOnce pb pl p1bs
  let commits := (Trace.zip gv pb).map
    (fun bx => recommitList f bx.2 bx.1)
  let maxSlots := (Trace.zip gv pb).map (fun bx => rcMaxSlot bx.1)
  let gated := (Trace.zip (sliceCuts cp 0 dPayload) pl).map
    (fun bx => if bx.2 then bx.1 else [])
  let indexed := scanAcrossTicksTrace
    (fun s bt => ipStep s bt.1 bt.2) 0 (Trace.zip gated maxSlots)
  (Trace.zip (Trace.zip indexed commits) (Trace.zip pb pl)).map
    (fun x =>
      if x.2.2 then
        x.1.1.map (fun sp => ((sp.1, x.2.1), some sp.2)) ++ x.1.2
      else [])

/-- One fused sequencing tick: the gate, the recommit, the indexing and
the stamp in one step. State: (ballot last recommitted at, next fresh
slot). Input: ((tick's view batch, tick's payload slice),
((ballot, leader), leader deferred)). -/
def spSendStep {nP : Nat} (variant : PaxosVariant) (f : Nat)
    (st : Option (Ballot nP) × Nat)
    (x : (Multiset (ALog P nP) × List P)
      × ((Ballot nP × Bool) × Bool)) :
    (Option (Ballot nP) × Nat)
      × List ((Nat × Ballot nP) × Option P) :=
  let g := spGateStep variant.recommitOnce st.1 x.1.1 x.2
  let gated := if x.2.1.2 then x.1.2 else []
  let ip := ipStep st.2 gated (rcMaxSlot g.2)
  ((g.1, ip.1),
   if x.2.1.2 then
     ip.2.map (fun sp => ((sp.1, x.2.1.1), some sp.2))
       ++ recommitList f x.2.1.1 g.2
   else [])

/-- The fusion, generalized to arbitrary scan states and an arbitrary
deferred-flag trace (the induction form: heads are definitional, tails
recurse with the stepped states). -/
private theorem spSent_fuse_go {nP : Nat} (variant : PaxosVariant)
    (f : Nat) :
    ∀ (p1bs : Trace (Multiset (ALog P nP))) (sl : Trace (List P))
      (pb : Trace (Ballot nP)) (pl : Trace Bool) (dfl : Trace Bool)
      (ra : Option (Ballot nP)) (ns : Nat),
      (Trace.zip (Trace.zip
          (scanAcrossTicksTrace (fun s bt => ipStep s bt.1 bt.2) ns
            (Trace.zip
              ((Trace.zip sl pl).map
                (fun bx => if bx.2 then bx.1 else []))
              ((Trace.zip (scanAcrossTicksTrace
                  (fun s bt => spGateStep variant.recommitOnce s bt.1
                    bt.2) ra
                  (Trace.zip p1bs (Trace.zip (Trace.zip pb pl) dfl)))
                  pb).map
                (fun bx => rcMaxSlot bx.1))))
          ((Trace.zip (scanAcrossTicksTrace
              (fun s bt => spGateStep variant.recommitOnce s bt.1 bt.2)
              ra
              (Trace.zip p1bs (Trace.zip (Trace.zip pb pl) dfl)))
              pb).map
            (fun bx => recommitList f bx.2 bx.1)))
        (Trace.zip pb pl)).map
        (fun x =>
          if x.2.2 then
            x.1.1.map (fun sp => ((sp.1, x.2.1), some sp.2)) ++ x.1.2
          else [])
      = scanAcrossTicksTrace (spSendStep variant f) (ra, ns)
          (Trace.zip (Trace.zip p1bs sl)
            (Trace.zip (Trace.zip pb pl) dfl))
  | [], _, _, _, _, _, _ => by
    simp [Trace.zip, scanAcrossTicksTrace]
  | _ :: _, [], _, _, _, _, _ => by
    simp [Trace.zip, scanAcrossTicksTrace]
  | _ :: _, _ :: _, [], _, _, _, _ => by
    simp [Trace.zip, scanAcrossTicksTrace]
  | _ :: _, _ :: _, _ :: _, [], _, _, _ => by
    simp [Trace.zip, scanAcrossTicksTrace]
  | _ :: _, _ :: _, _ :: _, _ :: _, [], _, _ => by
    simp [Trace.zip, scanAcrossTicksTrace]
  | g :: gs, s :: ss, b :: bs, l :: ls, d :: ds, ra, ns =>
    congrArg₂ List.cons rfl
      (spSent_fuse_go variant f gs ss bs ls ds
        (spGateStep variant.recommitOnce ra g ((b, l), d)).1
        (ipStep ns (if l then s else [])
          (rcMaxSlot
            (spGateStep variant.recommitOnce ra g ((b, l), d)).2)).1)

/-- **The fusion face**: the four-stage pipeline IS the fused scan over
the product input trace. -/
theorem spSentTrace_eq_scan {nP : Nat} (variant : PaxosVariant)
    (f : Nat) (cp : List P) (dPayload : List Nat)
    (pb : Trace (Ballot nP)) (pl : Trace Bool)
    (p1bs : Trace (Multiset (ALog P nP))) :
    spSentTrace variant f cp dPayload pb pl p1bs
      = scanAcrossTicksTrace (spSendStep variant f) (none, 0)
        (Trace.zip (Trace.zip p1bs (sliceCuts cp 0 dPayload))
          (Trace.zip (Trace.zip pb pl) (false :: pl))) :=
  spSent_fuse_go variant f p1bs (sliceCuts cp 0 dPayload) pb pl
    (false :: pl) none 0


/-! ## The recommit/log-view helper cluster (the key calculus's pure
facts) -/

private theorem nat_max?_ge : ∀ (l : List Nat) {x : Nat}, x ∈ l →
    ∃ m, l.max? = some m ∧ x ≤ m
  | [], _, h => nomatch h
  | y :: ys, x, h => by
    rcases List.mem_cons.mp h with rfl | h'
    · cases hys : ys.max? with
      | none =>
        exact ⟨x, by rw [List.max?_cons, hys]; rfl, Nat.le_refl _⟩
      | some m =>
        exact ⟨max x m, by rw [List.max?_cons, hys]; rfl,
          Nat.le_max_left ..⟩
    · obtain ⟨m, hm, hx⟩ := nat_max?_ge ys h'
      refine ⟨max y m, by rw [List.max?_cons, hm]; rfl, ?_⟩
      exact Nat.le_trans hx (Nat.le_max_right ..)

/-- `filterMap`s that preserve a projection keep it a sublist. -/
private theorem map_filterMap_sublist {α β γ : Type _}
    (fn : α → Option β) (g : β → γ) (g' : α → γ)
    (h : ∀ a b, fn a = some b → g b = g' a) :
    ∀ (l : List α), ((l.filterMap fn).map g).Sublist (l.map g')
  | [] => List.Sublist.refl _
  | a :: l => by
    rw [List.filterMap_cons]
    cases hfa : fn a with
    | none =>
      rw [List.map_cons]
      exact (map_filterMap_sublist fn g g' h l).cons _
    | some b =>
      rw [List.map_cons, List.map_cons, h a b hfa]
      exact (map_filterMap_sublist fn g g' h l).cons_cons _

/-- The tryCommit closure preserves the slot. -/
private theorem tryCommit_fn_slot {nP : Nat} (f : Nat) (b : Ballot nP)
    (logs : Multiset (ALog P nP)) (sl : Nat × LogValue P nP)
    (e : (Nat × Ballot nP) × Option P)
    (hfn : (if f < rcCount (rcEntries logs) sl.1 sl.2.value then none
      else if (match rcMaxCheckpoint logs with
        | some c => decide (sl.1 ≤ c)
        | none => false) then none
      else some ((sl.1, b), sl.2.value)) = some e) :
    e.1.1 = sl.1 := by
  by_cases h1 : f < rcCount (rcEntries logs) sl.1 sl.2.value
  · rw [if_pos h1] at hfn
    cases hfn
  · rw [if_neg h1] at hfn
    by_cases h2 : (match rcMaxCheckpoint logs with
      | some c => decide (sl.1 ≤ c)
      | none => false) = true
    · rw [if_pos h2] at hfn
      cases hfn
    · rw [if_neg h2] at hfn
      injection hfn with h'
      rw [← h']

/-- `logView`'s slots are duplicate-free (one champion per slot). -/
theorem logView_keys_nodup {nP : Nat}
    (entries : Multiset (Nat × LogValue P nP)) :
    ((logView entries).map Prod.fst).Nodup := by
  unfold logView
  refine List.Sublist.nodup
    (map_filterMap_sublist _ Prod.fst id ?_ _) ?_
  · intro a e hae
    cases hm : Ballot.maxFoldBatch none
        ((((entries.filter (fun e => e.1 = a)).map Prod.snd).map
          (·.ballot))) with
    | none =>
      simp only [hm] at hae
      cases hae
    | some bb =>
      simp only [hm] at hae
      injection hae with h'
      rw [← h']
      rfl
  · rw [List.map_id]
    exact Finset.sort_nodup _ _

/-- A slot's presence forces `rcMaxSlot` to answer, at or above it. -/
theorem rcMaxSlot_ge {nP : Nat} {logs : Multiset (ALog P nP)}
    {slot : Nat} {e : LogValue P nP}
    (h : (slot, e) ∈ rcEntries logs) :
    ∃ m, rcMaxSlot logs = some m ∧ slot ≤ m := by
  obtain ⟨e', he', -⟩ := logView_covers (rcEntries logs) h
  exact nat_max?_ge _ (List.mem_map.mpr ⟨(slot, e'), he', rfl⟩)

/-- Recommit slots never exceed the view's max slot. -/
theorem recommitList_slot_le {nP : Nat} (f : Nat) (b : Ballot nP)
    (logs : Multiset (ALog P nP)) :
    ∀ e ∈ recommitList f b logs,
      ∃ m, rcMaxSlot logs = some m
        ∧ (e : (Nat × Ballot nP) × Option P).1.1 ≤ m := by
  intro e he
  unfold recommitList at he
  rcases List.mem_append.mp he with h | h
  · -- tryCommit: the slot is a champion's
    obtain ⟨sl, hsl, hfn⟩ := List.mem_filterMap.mp h
    obtain ⟨m, hm, hle⟩ := nat_max?_ge
      ((logView (rcEntries logs)).map
        (fun sl : Nat × LogValue P nP => sl.1))
      (List.mem_map.mpr ⟨sl, hsl, rfl⟩)
    exact ⟨m, hm, by
      rw [tryCommit_fn_slot f b logs sl e hfn]
      exact hle⟩
  · -- holes: below the max by construction
    obtain ⟨slot, hslot, hpair⟩ := List.mem_map.mp h
    revert hslot
    cases hmax : ((logView (rcEntries logs)).map
        (fun sl : Nat × LogValue P nP => sl.1)).max? with
    | none =>
      intro hslot
      cases hslot
    | some maxSlot =>
      intro hslot
      have hrange := (List.mem_filter.mp hslot).1
      have hmem := List.mem_range'_1.mp hrange
      refine ⟨maxSlot, hmax, ?_⟩
      rw [← hpair]
      show slot ≤ maxSlot
      omega

/-- An empty-keyed view recommits nothing. -/
theorem recommitList_eq_nil_of_max_none {nP : Nat} (f : Nat)
    (b : Ballot nP) {logs : Multiset (ALog P nP)}
    (h : rcMaxSlot logs = none) : recommitList f b logs = [] := by
  refine List.eq_nil_iff_forall_not_mem.mpr ?_
  intro e he
  obtain ⟨m, hm, -⟩ := recommitList_slot_le f b logs e he
  rw [h] at hm
  cases hm

/-- The gated-off view recommits nothing (`rcMaxSlot` of nothing). -/
theorem rcMaxSlot_zero {nP : Nat} :
    rcMaxSlot (0 : Multiset (ALog P nP)) = none := by
  unfold rcMaxSlot rcEntries
  simp [logView]

/-- Recommit keys are duplicate-free (one commitment per slot per
reign). -/
theorem recommitList_keys_nodup {nP : Nat} (f : Nat) (b : Ballot nP)
    (logs : Multiset (ALog P nP)) :
    ((recommitList f b logs).map
      (fun e => (e : (Nat × Ballot nP) × Option P).1.1)).Nodup := by
  unfold recommitList
  rw [List.map_append]
  refine List.Nodup.append ?_ ?_ ?_
  · -- tryCommit slots: a projection-preserving filterMap of champions
    exact List.Sublist.nodup (map_filterMap_sublist _ _
      (fun sl : Nat × LogValue P nP => sl.1)
      (fun sl e hfn => tryCommit_fn_slot f b logs sl e hfn) _)
      (logView_keys_nodup (rcEntries logs))
  · -- hole slots: a filter of `range'`
    rw [List.map_map]
    have hcomp : ((fun e => (e : (Nat × Ballot nP) × Option P).1.1)
        ∘ (fun slot => ((slot, b), (none : Option P)))) = id := rfl
    rw [hcomp, List.map_id]
    cases hmax : ((logView (rcEntries logs)).map
        (fun sl : Nat × LogValue P nP => sl.1)).max? with
    | none => exact List.Pairwise.nil
    | some maxSlot => exact List.Nodup.filter _ List.nodup_range'
  · -- disjoint: holes are filtered off the proposed slots
    intro a ha hb
    have hprop : a ∈ (logView (rcEntries logs)).map
        (fun sl : Nat × LogValue P nP => sl.1) :=
      (map_filterMap_sublist _ _
        (fun sl : Nat × LogValue P nP => sl.1)
        (fun sl e hfn => tryCommit_fn_slot f b logs sl e hfn)
        _).subset ha
    rw [List.map_map] at hb
    have hcomp : ((fun e => (e : (Nat × Ballot nP) × Option P).1.1)
        ∘ (fun slot => ((slot, b), (none : Option P)))) = id := rfl
    rw [hcomp, List.map_id] at hb
    revert hb
    cases hmax : ((logView (rcEntries logs)).map
        (fun sl : Nat × LogValue P nP => sl.1)).max? with
    | none =>
      intro hb
      cases hb
    | some maxSlot =>
      intro hb
      have hnp := (List.mem_filter.mp hb).2
      simp only [Bool.not_eq_true'] at hnp
      have hc : ((logView (rcEntries logs)).map
          (fun sl : Nat × LogValue P nP => sl.1)).contains a = true :=
        List.elem_iff.mpr hprop
      rw [hnp] at hc
      cases hc

/-- **Sent traces are final**: growing every input extends the sent
trace by prefix (the pure form of Flo monotonicity at this wire). -/
theorem spSentTrace_prefix {nP : Nat} (variant : PaxosVariant)
    (f : Nat) {cp cp' : List P} (dPayload : List Nat)
    {pb pb' : Trace (Ballot nP)} {pl pl' : Trace Bool}
    {p1bs p1bs' : Trace (Multiset (ALog P nP))}
    (hcp : cp <+: cp') (hpb : pb <+: pb') (hpl : pl <+: pl')
    (hp1bs : p1bs <+: p1bs') :
    spSentTrace variant f cp dPayload pb pl p1bs
      <+: spSentTrace variant f cp' dPayload pb' pl' p1bs' := by
  have hgv : spGatedTrace variant.recommitOnce pb pl p1bs
      <+: spGatedTrace variant.recommitOnce pb' pl' p1bs' :=
    scanAcrossTicksTrace_prefix _ _
      (zip_prefix hp1bs (zip_prefix (zip_prefix hpb hpl)
        (List.cons_prefix_cons.mpr ⟨rfl, hpl⟩)))
  exact List.IsPrefix.map _
    (zip_prefix
      (zip_prefix
        (scanAcrossTicksTrace_prefix _ _
          (zip_prefix (List.IsPrefix.map _ (zip_prefix
            (sliceCuts_le hcp 0 dPayload) hpl))
            (List.IsPrefix.map _ (zip_prefix hgv hpb))))
        (List.IsPrefix.map _ (zip_prefix hgv hpb)))
      (zip_prefix hpb hpl))


private theorem eq_of_keys_nodup {α κ : Type _} {l : List (κ × α)}
    (hnd : (l.map Prod.fst).Nodup) {k : κ} {a a' : α}
    (h : (k, a) ∈ l) (h' : (k, a') ∈ l) : a = a' := by
  induction l with
  | nil => cases h
  | cons x xs ih =>
    rw [List.map_cons] at hnd
    rcases List.mem_cons.mp h with rfl | h2
    · rcases List.mem_cons.mp h' with h1' | h2'
      · exact (congrArg Prod.snd h1'.symm :)
      · exfalso
        exact (List.nodup_cons.mp hnd).1
          (List.mem_map.mpr ⟨(k, a'), h2', rfl⟩)
    · rcases List.mem_cons.mp h' with rfl | h2'
      · exfalso
        exact (List.nodup_cons.mp hnd).1
          (List.mem_map.mpr ⟨(k, a), h2, rfl⟩)
      · exact ih (List.nodup_cons.mp hnd).2 h2 h2'

/-- The tryCommit closure's full output shape. -/
private theorem tryCommit_fn_eq {nP : Nat} (f : Nat) (b : Ballot nP)
    (logs : Multiset (ALog P nP)) (sl : Nat × LogValue P nP)
    (e : (Nat × Ballot nP) × Option P)
    (hfn : (if f < rcCount (rcEntries logs) sl.1 sl.2.value then none
      else if (match rcMaxCheckpoint logs with
        | some c => decide (sl.1 ≤ c)
        | none => false) then none
      else some ((sl.1, b), sl.2.value)) = some e) :
    e = ((sl.1, b), sl.2.value) := by
  by_cases h1 : f < rcCount (rcEntries logs) sl.1 sl.2.value
  · rw [if_pos h1] at hfn
    cases hfn
  · rw [if_neg h1] at hfn
    by_cases h2 : (match rcMaxCheckpoint logs with
      | some c => decide (sl.1 ≤ c)
      | none => false) = true
    · rw [if_pos h2] at hfn
      cases hfn
    · rw [if_neg h2] at hfn
      injection hfn with h'
      rw [← h']

/-- **Recommit values are champions**: an emitted recommit at a covered
slot carries the slot's `logView` champion value, which dominates every
entry at the slot. -/
theorem recommitList_value_best {nP : Nat} (f : Nat) (b : Ballot nP)
    (logs : Multiset (ALog P nP)) {slot : Nat} {v : Option P}
    (hmem : ((slot, b), v) ∈ recommitList f b logs)
    {e₀ : LogValue P nP} (he₀ : (slot, e₀) ∈ rcEntries logs) :
    ∃ best : LogValue P nP,
      (slot, best) ∈ logView (rcEntries logs)
      ∧ v = best.value ∧ e₀.ballot.ble best.ballot = true := by
  obtain ⟨e', he', hble⟩ := logView_covers (rcEntries logs) he₀
  unfold recommitList at hmem
  rcases List.mem_append.mp hmem with h | h
  · -- tryCommit: the emission is the champion's value
    obtain ⟨sl, hsl, hfn⟩ := List.mem_filterMap.mp h
    have heq := tryCommit_fn_eq f b logs sl _ hfn
    have hslot : sl.1 = slot := (congrArg (fun e => e.1.1) heq).symm
    have hval : v = sl.2.value := congrArg (fun e => e.2) heq
    have hsl2 : ((slot, sl.2) : Nat × LogValue P nP)
        ∈ logView (rcEntries logs) := by
      rw [← hslot]
      exact hsl
    have huniq : sl.2 = e' :=
      eq_of_keys_nodup (logView_keys_nodup (rcEntries logs)) hsl2 he'
    exact ⟨sl.2, hsl2, hval, by rw [huniq]; exact hble⟩
  · -- holes never sit under a covered slot
    exfalso
    obtain ⟨slot', hslot', hpair⟩ := List.mem_map.mp h
    have hslot : slot' = slot := congrArg (fun e => e.1.1) hpair
    revert hslot'
    cases hmax : ((logView (rcEntries logs)).map
        (fun sl : Nat × LogValue P nP => sl.1)).max? with
    | none =>
      intro hslot'
      cases hslot'
    | some maxSlot =>
      intro hslot'
      have hnp := (List.mem_filter.mp hslot').2
      simp only [Bool.not_eq_true'] at hnp
      have hc : ((logView (rcEntries logs)).map
          (fun sl : Nat × LogValue P nP => sl.1)).contains slot'
          = true :=
        List.elem_iff.mpr (by
          rw [hslot]
          exact List.mem_map.mpr ⟨(slot, e'), he', rfl⟩)
      rw [hnp] at hc
      cases hc


/-- The guarded send-once induction: over the fused scan, with the
reign discipline as structural hypotheses and the two-register state
invariant — `ra` names the reign already recommitted (all later
ballots strictly outnumber it), and keys of the recommitted reign sit
at or above `ns`. -/
private theorem spSent_nodup_go {nP : Nat} (f : Nat) (me : Fin nP) :
    ∀ (L : List ((Multiset (ALog P nP) × List P)
        × ((Ballot nP × Bool) × Bool)))
      (ra : Option (Ballot nP)) (ns : Nat),
      (∀ x ∈ L, (x : (Multiset (ALog P nP) × List P)
        × ((Ballot nP × Bool) × Bool)).2.1.1.proposerId = me) →
      List.Pairwise (fun x y => x.2.1.1.num ≤ y.2.1.1.num) L →
      List.IsChain (fun x y => y.2.1.2 = true → x.2.1.2 = true →
        y.2.1.1 = x.2.1.1) L →
      List.IsChain (fun x y => y.2.2 = x.2.1.2) L →
      (∀ x ∈ L, (x : (Multiset (ALog P nP) × List P)
        × ((Ballot nP × Bool) × Bool)).2.1.2 = true → x.1.1 ≠ 0) →
      (∀ a, ra = some a → a.proposerId = me
        ∧ ∀ x ∈ L, a.num ≤ x.2.1.1.num) →
      (∀ x, L.head? = some x → x.2.2 = true → x.2.1.2 = true →
        ra = some x.2.1.1) →
      (((scanAcrossTicksTrace (spSendStep .guarded f) (ra, ns)
          L).flatten).map Prod.fst).Nodup
      ∧ (∀ k ∈ ((scanAcrossTicksTrace (spSendStep .guarded f) (ra, ns)
          L).flatten).map Prod.fst,
          ∃ x ∈ L, (k : Nat × Ballot nP).2 = x.2.1.1)
      ∧ (∀ k ∈ ((scanAcrossTicksTrace (spSendStep .guarded f) (ra, ns)
          L).flatten).map Prod.fst,
          ra = some (k : Nat × Ballot nP).2 → ns ≤ k.1)
  | [], ra, ns, _, _, _, _, _, _, _ => by
    refine ⟨List.Pairwise.nil, ?_, ?_⟩ <;> intro k hk <;> cases hk
  | x :: L', ra, ns, hown, hmono, hstable, hdfl, hlead, hinvra,
      hinvprev => by
    obtain ⟨⟨g, s⟩, ⟨b0, l⟩, d⟩ := x
    -- structural hypotheses, restricted to the tail
    have hown' : ∀ y ∈ L', (y : (Multiset (ALog P nP) × List P)
        × ((Ballot nP × Bool) × Bool)).2.1.1.proposerId = me :=
      fun y hy => hown y (List.mem_cons_of_mem _ hy)
    have hmono' := (List.pairwise_cons.mp hmono).2
    have hmono0 := (List.pairwise_cons.mp hmono).1
    have hstable' := hstable.tail
    have hdfl' := hdfl.tail
    have hdfl0' : ∀ y, y ∈ L'.head? → y.2.2
        = ((⟨⟨g, s⟩, ⟨b0, l⟩, d⟩ : (Multiset (ALog P nP) × List P)
          × ((Ballot nP × Bool) × Bool))).2.1.2 :=
      fun y hy => hdfl.rel_head? hy
    have hlead' : ∀ y ∈ L', (y : (Multiset (ALog P nP) × List P)
        × ((Ballot nP × Bool) × Bool)).2.1.2 = true → y.1.1 ≠ 0 :=
      fun y hy => hlead y (List.mem_cons_of_mem _ hy)
    -- the tail's deferred flag reads this tick's leader flag
    have hdfl0 : ∀ y, L'.head? = some y → y.2.2 = l := by
      intro y hy
      exact hdfl0' y (by rw [hy]; exact rfl)
    -- the tail's stability reads this tick's ballot
    have hstable0 : ∀ y, L'.head? = some y → y.2.1.2 = true →
        l = true → y.2.1.1 = b0 := by
      intro y hy hylead hl
      exact hstable.rel_head? (by rw [hy]; exact rfl) hylead hl
    by_cases hl : l = true
    case neg =>
      -- a follower tick emits nothing and holds every register
      have hl' : l = false := by
        cases l
        · rfl
        · exact absurd rfl hl
      subst hl'
      have hfire : (decide (g ≠ 0)
          && (false && !d && !decide (ra = some b0))) = false := by
        simp
      have hra' : (spGateStep PaxosVariant.guarded.recommitOnce ra g
          ((b0, false), d)).1 = ra := by
        show (if (decide (g ≠ 0)
          && (false && !d && !decide (ra = some b0))) = true
          then some b0 else ra) = ra
        rw [hfire]
        rfl
      have hgv : (spGateStep PaxosVariant.guarded.recommitOnce ra g
          ((b0, false), d)).2 = 0 := by
        show (if (decide (g ≠ 0)
          && (false && !d && !decide (ra = some b0))) = true
          then g else 0) = 0
        rw [hfire]
        rfl
      have hstep : spSendStep .guarded f (ra, ns)
          ((g, s), ((b0, false), d)) = ((ra, ns), []) := by
        show ((( spGateStep PaxosVariant.guarded.recommitOnce ra g
            ((b0, false), d)).1,
          (ipStep ns (if false then s else [])
            (rcMaxSlot (spGateStep PaxosVariant.guarded.recommitOnce
              ra g ((b0, false), d)).2)).1),
          if false then _ else []) = ((ra, ns), [])
        rw [hra', hgv, rcMaxSlot_zero]
        rfl
      have hinvprev' : ∀ y, L'.head? = some y → y.2.2 = true →
          y.2.1.2 = true → ra = some y.2.1.1 := by
        intro y hy hprev _
        rw [hdfl0 y hy] at hprev
        cases hprev
      have ih := spSent_nodup_go f me L' ra ns hown' hmono' hstable'
        hdfl' hlead' (fun a ha => ⟨(hinvra a ha).1,
          fun y hy => (hinvra a ha).2 y (List.mem_cons_of_mem _ hy)⟩)
        hinvprev'
      have hscan : scanAcrossTicksTrace (spSendStep .guarded f)
          (ra, ns) (((g, s), ((b0, false), d)) :: L')
          = [] :: scanAcrossTicksTrace (spSendStep .guarded f)
            (ra, ns) L' := by
        show (spSendStep .guarded f (ra, ns)
            ((g, s), ((b0, false), d))).2
          :: scanAcrossTicksTrace (spSendStep .guarded f)
            (spSendStep .guarded f (ra, ns)
              ((g, s), ((b0, false), d))).1 L' = _
        rw [hstep]
      rw [hscan]
      simp only [List.flatten_cons, List.nil_append]
      exact ⟨ih.1,
        fun k hk => (ih.2.1 k hk).imp (fun y hy =>
          ⟨List.mem_cons_of_mem _ hy.1, hy.2⟩),
        ih.2.2⟩
    case pos =>
      subst hl
      have hg0 : g ≠ 0 := hlead _ (List.mem_cons_self ..) rfl
      by_cases hra : ra = some b0
      · -- reign continuation: the gate stays closed; fresh payloads
        -- from the running slot register
        have hfire : (decide (g ≠ 0)
            && (true && !d && !decide (ra = some b0))) = false := by
          rw [decide_eq_true hra]
          simp
        have hra' : (spGateStep PaxosVariant.guarded.recommitOnce ra g
            ((b0, true), d)).1 = ra := by
          show (if (decide (g ≠ 0)
            && (true && !d && !decide (ra = some b0))) = true
            then some b0 else ra) = ra
          rw [hfire]
          rfl
        have hgv : (spGateStep PaxosVariant.guarded.recommitOnce ra g
            ((b0, true), d)).2 = 0 := by
          show (if (decide (g ≠ 0)
            && (true && !d && !decide (ra = some b0))) = true
            then g else 0) = 0
          rw [hfire]
          rfl
        have hout : (spSendStep .guarded f (ra, ns)
            ((g, s), ((b0, true), d))).2
            = (ipStep ns s none).2.map
              (fun sp => ((sp.1, b0), some sp.2)) := by
          show (if true then
              (ipStep ns (if true then s else [])
                (rcMaxSlot (spGateStep
                  PaxosVariant.guarded.recommitOnce ra g
                  ((b0, true), d)).2)).2.map
                (fun sp => ((sp.1, b0), some sp.2))
              ++ recommitList f b0 (spGateStep
                PaxosVariant.guarded.recommitOnce ra g
                ((b0, true), d)).2
            else []) = _
          rw [hgv, rcMaxSlot_zero,
            recommitList_eq_nil_of_max_none f b0 rcMaxSlot_zero,
            List.append_nil]
          rfl
        have hst' : (spSendStep .guarded f (ra, ns)
            ((g, s), ((b0, true), d))).1 = (ra, ns + s.length) := by
          show ((spGateStep PaxosVariant.guarded.recommitOnce ra g
              ((b0, true), d)).1,
            (ipStep ns (if true then s else [])
              (rcMaxSlot (spGateStep
                PaxosVariant.guarded.recommitOnce ra g
                ((b0, true), d)).2)).1) = _
          rw [hra', hgv, rcMaxSlot_zero]
          rfl
        have hscan : scanAcrossTicksTrace (spSendStep .guarded f)
            (ra, ns) (((g, s), ((b0, true), d)) :: L')
            = (ipStep ns s none).2.map
                (fun sp => ((sp.1, b0), some sp.2))
              :: scanAcrossTicksTrace (spSendStep .guarded f)
                (ra, ns + s.length) L' := by
          show (spSendStep .guarded f (ra, ns)
              ((g, s), ((b0, true), d))).2
            :: scanAcrossTicksTrace (spSendStep .guarded f)
              (spSendStep .guarded f (ra, ns)
                ((g, s), ((b0, true), d))).1 L' = _
          rw [hst', hout]
        have hinvprev₂ : ∀ y, L'.head? = some y → y.2.2 = true →
            y.2.1.2 = true → ra = some y.2.1.1 := by
          intro y hy _ hylead
          rw [hstable0 y hy hylead rfl]
          exact hra
        have ih := spSent_nodup_go f me L' ra (ns + s.length) hown'
          hmono' hstable' hdfl' hlead'
          (fun a ha => ⟨(hinvra a ha).1,
            fun y hy => (hinvra a ha).2 y (List.mem_cons_of_mem _ hy)⟩)
          hinvprev₂
        rw [hscan]
        simp only [List.flatten_cons, List.map_append]
        -- the head keys: fresh slots from `ns`, all at ballot `b0`
        have hhd_key : ∀ k ∈ ((ipStep ns s none).2.map
            (fun sp => ((sp.1, b0), some sp.2))).map Prod.fst,
            (k : Nat × Ballot nP).2 = b0
            ∧ ns ≤ k.1 ∧ k.1 < ns + s.length := by
          intro k hk
          rw [List.map_map] at hk
          obtain ⟨sp, hsp, rfl⟩ := List.mem_map.mp hk
          have hsl : sp.1 ∈ List.range' ns s.length := by
            have h1 : sp.1 ∈ (ipStep ns s none).2.map Prod.fst :=
              List.mem_map.mpr ⟨sp, hsp, rfl⟩
            rw [ipStep_slots] at h1
            exact h1
          have := List.mem_range'_1.mp hsl
          exact ⟨rfl, this.1, this.2⟩
        refine ⟨List.Nodup.append ?_ ih.1 ?_, ?_, ?_⟩
        · -- distinct fresh slots at one ballot
          refine List.Nodup.of_map Prod.fst ?_
          rw [List.map_map, List.map_map]
          show ((ipStep ns s none).2.map
            (fun sp => sp.1)).Nodup
          have := ipStep_slots (P := P) ns s none
          rw [show ((ipStep ns s none).2.map
            (fun sp : Nat × P => sp.1))
            = (ipStep ns s none).2.map Prod.fst from rfl, this]
          exact List.nodup_range'
        · -- the reign's later keys sit above this tick's
          intro k hk1 hk2
          obtain ⟨hkb, -, hlt⟩ := hhd_key k hk1
          have hge := ih.2.2 k hk2 (by rw [hkb]; exact hra)
          omega
        · -- ballots come from the trace
          intro k hk
          rcases List.mem_append.mp hk with hk | hk
          · exact ⟨_, List.mem_cons_self .., (hhd_key k hk).1⟩
          · exact (ih.2.1 k hk).imp (fun y hy =>
              ⟨List.mem_cons_of_mem _ hy.1, hy.2⟩)
        · -- the recommitted reign's keys sit at or above `ns`
          intro k hk hra2
          rcases List.mem_append.mp hk with hk | hk
          · exact (hhd_key k hk).2.1
          · exact Nat.le_trans (Nat.le_add_right ..)
              (ih.2.2 k hk hra2)
      · by_cases hd : d = true
        · -- mid-run without a recommit on record: the knot invariant
          -- says the reign already fired
          exact absurd (hinvprev _ rfl hd rfl) hra
        · -- the reign's first leader tick: the gate fires
          have hdf : d = false := by
            cases d
            · rfl
            · exact absurd rfl hd
          subst hdf
          have hfire : (decide (g ≠ 0)
              && (true && !false && !decide (ra = some b0))) = true := by
            rw [decide_eq_true hg0, decide_eq_false hra]
            rfl
          have hra' : (spGateStep PaxosVariant.guarded.recommitOnce ra
              g ((b0, true), false)).1 = some b0 := by
            show (if (decide (g ≠ 0)
              && (true && !false && !decide (ra = some b0))) = true
              then some b0 else ra) = some b0
            rw [hfire]
            rfl
          have hgv : (spGateStep PaxosVariant.guarded.recommitOnce ra
              g ((b0, true), false)).2 = g := by
            show (if (decide (g ≠ 0)
              && (true && !false && !decide (ra = some b0))) = true
              then g else 0) = g
            rw [hfire]
            rfl
          have hb0own : b0.proposerId = me :=
            hown _ (List.mem_cons_self ..)
          have hinvra₂ : ∀ a, (some b0 : Option (Ballot nP)) = some a →
              a.proposerId = me ∧ ∀ y ∈ L', a.num ≤ y.2.1.1.num := by
            intro a ha
            cases Option.some.inj ha
            exact ⟨hb0own, fun y hy => hmono0 y hy⟩
          have hinvprev₂ : ∀ y, L'.head? = some y → y.2.2 = true →
              y.2.1.2 = true →
              (some b0 : Option (Ballot nP)) = some y.2.1.1 := by
            intro y hy _ hylead
            rw [hstable0 y hy hylead rfl]
          -- an already-recommitted older reign cannot resurface
          have hCtail : ∀ (k : Nat × Ballot nP),
              (∃ y ∈ L', k.2 = y.2.1.1) → ra = some k.2 → False := by
            intro k hky hra2
            obtain ⟨y, hy, hkb⟩ := hky
            have h1 : k.2.num ≤ b0.num :=
              (hinvra _ hra2).2 _ (List.mem_cons_self ..)
            have h2 : b0.num ≤ y.2.1.1.num := hmono0 y hy
            have hkne : k.2 ≠ b0 := fun he => hra (he ▸ hra2)
            have hnum : k.2.num ≠ b0.num := fun he =>
              hkne (Ballot.eq_of_num_owner he
                (by rw [(hinvra _ hra2).1, hb0own]))
            rw [← hkb] at h2
            omega
          cases hm : rcMaxSlot g with
          | none =>
            -- a keyless view: nothing to recommit, payloads from `ns`
            have hout : (spSendStep .guarded f (ra, ns)
                ((g, s), ((b0, true), false))).2
                = (ipStep ns s none).2.map
                  (fun sp => ((sp.1, b0), some sp.2)) := by
              show (if true then
                  (ipStep ns (if true then s else [])
                    (rcMaxSlot (spGateStep
                      PaxosVariant.guarded.recommitOnce ra g
                      ((b0, true), false)).2)).2.map
                    (fun sp => ((sp.1, b0), some sp.2))
                  ++ recommitList f b0 (spGateStep
                    PaxosVariant.guarded.recommitOnce ra g
                    ((b0, true), false)).2
                else []) = _
              rw [hgv, hm, recommitList_eq_nil_of_max_none f b0 hm,
                List.append_nil]
              rfl
            have hst' : (spSendStep .guarded f (ra, ns)
                ((g, s), ((b0, true), false))).1
                = (some b0, ns + s.length) := by
              show ((spGateStep PaxosVariant.guarded.recommitOnce ra g
                  ((b0, true), false)).1,
                (ipStep ns (if true then s else [])
                  (rcMaxSlot (spGateStep
                    PaxosVariant.guarded.recommitOnce ra g
                    ((b0, true), false)).2)).1) = _
              rw [hra', hgv, hm]
              rfl
            have hscan : scanAcrossTicksTrace (spSendStep .guarded f)
                (ra, ns) (((g, s), ((b0, true), false)) :: L')
                = (ipStep ns s none).2.map
                    (fun sp => ((sp.1, b0), some sp.2))
                  :: scanAcrossTicksTrace (spSendStep .guarded f)
                    (some b0, ns + s.length) L' := by
              show (spSendStep .guarded f (ra, ns)
                  ((g, s), ((b0, true), false))).2
                :: scanAcrossTicksTrace (spSendStep .guarded f)
                  (spSendStep .guarded f (ra, ns)
                    ((g, s), ((b0, true), false))).1 L' = _
              rw [hst', hout]
            have ih := spSent_nodup_go f me L' (some b0)
              (ns + s.length) hown' hmono' hstable' hdfl' hlead'
              hinvra₂ hinvprev₂
            rw [hscan]
            simp only [List.flatten_cons, List.map_append]
            have hhd_key : ∀ k ∈ ((ipStep ns s none).2.map
                (fun sp => ((sp.1, b0), some sp.2))).map Prod.fst,
                (k : Nat × Ballot nP).2 = b0
                ∧ ns ≤ k.1 ∧ k.1 < ns + s.length := by
              intro k hk
              rw [List.map_map] at hk
              obtain ⟨sp, hsp, rfl⟩ := List.mem_map.mp hk
              have hsl : sp.1 ∈ List.range' ns s.length := by
                have h1 : sp.1 ∈ (ipStep ns s none).2.map Prod.fst :=
                  List.mem_map.mpr ⟨sp, hsp, rfl⟩
                rw [ipStep_slots] at h1
                exact h1
              have := List.mem_range'_1.mp hsl
              exact ⟨rfl, this.1, this.2⟩
            refine ⟨List.Nodup.append ?_ ih.1 ?_, ?_, ?_⟩
            · refine List.Nodup.of_map Prod.fst ?_
              rw [List.map_map, List.map_map]
              show ((ipStep ns s none).2.map
                (fun sp => sp.1)).Nodup
              have := ipStep_slots (P := P) ns s none
              rw [show ((ipStep ns s none).2.map
                (fun sp : Nat × P => sp.1))
                = (ipStep ns s none).2.map Prod.fst from rfl, this]
              exact List.nodup_range'
            · intro k hk1 hk2
              obtain ⟨hkb, -, hlt⟩ := hhd_key k hk1
              have hge := ih.2.2 k hk2 (by rw [hkb])
              omega
            · intro k hk
              rcases List.mem_append.mp hk with hk | hk
              · exact ⟨_, List.mem_cons_self .., (hhd_key k hk).1⟩
              · exact (ih.2.1 k hk).imp (fun y hy =>
                  ⟨List.mem_cons_of_mem _ hy.1, hy.2⟩)
            · intro k hk hra2
              rcases List.mem_append.mp hk with hk | hk
              · exact absurd ((hhd_key k hk).1 ▸ hra2) hra
              · exact absurd hra2 (fun hra2 =>
                  hCtail k ((ih.2.1 k hk).imp
                    (fun y hy => ⟨hy.1, hy.2⟩)) hra2)
          | some m =>
            -- the rebase: recommits at or below `m`, payloads above
            have hout : (spSendStep .guarded f (ra, ns)
                ((g, s), ((b0, true), false))).2
                = (ipStep ns s (some m)).2.map
                    (fun sp => ((sp.1, b0), some sp.2))
                  ++ recommitList f b0 g := by
              show (if true then
                  (ipStep ns (if true then s else [])
                    (rcMaxSlot (spGateStep
                      PaxosVariant.guarded.recommitOnce ra g
                      ((b0, true), false)).2)).2.map
                    (fun sp => ((sp.1, b0), some sp.2))
                  ++ recommitList f b0 (spGateStep
                    PaxosVariant.guarded.recommitOnce ra g
                    ((b0, true), false)).2
                else []) = _
              rw [hgv, hm]
              rfl
            have hst' : (spSendStep .guarded f (ra, ns)
                ((g, s), ((b0, true), false))).1
                = (some b0, m + 1 + s.length) := by
              show ((spGateStep PaxosVariant.guarded.recommitOnce ra g
                  ((b0, true), false)).1,
                (ipStep ns (if true then s else [])
                  (rcMaxSlot (spGateStep
                    PaxosVariant.guarded.recommitOnce ra g
                    ((b0, true), false)).2)).1) = _
              rw [hra', hgv, hm]
              rfl
            have hscan : scanAcrossTicksTrace (spSendStep .guarded f)
                (ra, ns) (((g, s), ((b0, true), false)) :: L')
                = ((ipStep ns s (some m)).2.map
                      (fun sp => ((sp.1, b0), some sp.2))
                    ++ recommitList f b0 g)
                  :: scanAcrossTicksTrace (spSendStep .guarded f)
                    (some b0, m + 1 + s.length) L' := by
              show (spSendStep .guarded f (ra, ns)
                  ((g, s), ((b0, true), false))).2
                :: scanAcrossTicksTrace (spSendStep .guarded f)
                  (spSendStep .guarded f (ra, ns)
                    ((g, s), ((b0, true), false))).1 L' = _
              rw [hst', hout]
            have ih := spSent_nodup_go f me L' (some b0)
              (m + 1 + s.length) hown' hmono' hstable' hdfl' hlead'
              hinvra₂ hinvprev₂
            rw [hscan]
            simp only [List.flatten_cons, List.map_append]
            have hpay_key : ∀ k ∈ ((ipStep ns s (some m)).2.map
                (fun sp => ((sp.1, b0), some sp.2))).map Prod.fst,
                (k : Nat × Ballot nP).2 = b0
                ∧ m + 1 ≤ k.1 ∧ k.1 < m + 1 + s.length := by
              intro k hk
              rw [List.map_map] at hk
              obtain ⟨sp, hsp, rfl⟩ := List.mem_map.mp hk
              have hsl : sp.1 ∈ List.range' (m + 1) s.length := by
                have h1 : sp.1 ∈ (ipStep ns s (some m)).2.map
                    Prod.fst :=
                  List.mem_map.mpr ⟨sp, hsp, rfl⟩
                rw [ipStep_slots] at h1
                exact h1
              have := List.mem_range'_1.mp hsl
              exact ⟨rfl, this.1, this.2⟩
            have hrc_key : ∀ k ∈ (recommitList f b0 g).map Prod.fst,
                (k : Nat × Ballot nP).2 = b0 ∧ k.1 ≤ m := by
              intro k hk
              obtain ⟨e, he, rfl⟩ := List.mem_map.mp hk
              refine ⟨recommitList_ballot f b0 g e he, ?_⟩
              obtain ⟨m', hm', hle⟩ := recommitList_slot_le f b0 g e he
              rw [hm] at hm'
              injection hm' with hmm
              rw [hmm]
              exact hle
            refine ⟨List.Nodup.append (List.Nodup.append ?_ ?_ ?_)
              ih.1 ?_, ?_, ?_⟩
            · refine List.Nodup.of_map Prod.fst ?_
              rw [List.map_map, List.map_map]
              show ((ipStep ns s (some m)).2.map
                (fun sp => sp.1)).Nodup
              have := ipStep_slots (P := P) ns s (some m)
              rw [show ((ipStep ns s (some m)).2.map
                (fun sp : Nat × P => sp.1))
                = (ipStep ns s (some m)).2.map Prod.fst from rfl,
                this]
              exact List.nodup_range'
            · refine List.Nodup.of_map Prod.fst ?_
              rw [List.map_map]
              exact recommitList_keys_nodup f b0 g
            · intro k hk1 hk2
              have h1 := (hpay_key k hk1).2.1
              have h2 := (hrc_key k hk2).2
              omega
            · intro k hk1 hk2
              rcases List.mem_append.mp hk1 with hk1 | hk1
              · have hlt := (hpay_key k hk1).2.2
                have hge := ih.2.2 k hk2
                  (by rw [(hpay_key k hk1).1])
                omega
              · have hle := (hrc_key k hk1).2
                have hge := ih.2.2 k hk2
                  (by rw [(hrc_key k hk1).1])
                omega
            · intro k hk
              rcases List.mem_append.mp hk with hk | hk
              · rcases List.mem_append.mp hk with hk | hk
                · exact ⟨_, List.mem_cons_self .., (hpay_key k hk).1⟩
                · exact ⟨_, List.mem_cons_self .., (hrc_key k hk).1⟩
              · exact (ih.2.1 k hk).imp (fun y hy =>
                  ⟨List.mem_cons_of_mem _ hy.1, hy.2⟩)
            · intro k hk hra2
              rcases List.mem_append.mp hk with hk | hk
              · rcases List.mem_append.mp hk with hk | hk
                · exact absurd ((hpay_key k hk).1 ▸ hra2) hra
                · exact absurd ((hrc_key k hk).1 ▸ hra2) hra
              · exact absurd hra2 (fun hra2 =>
                  hCtail k ((ih.2.1 k hk).imp
                    (fun y hy => ⟨hy.1, hy.2⟩)) hra2)

/-- **The guarded key calculus** (B2 + slot freshness): over an owned,
`num`-ascending ballot wire whose leader flags are reign-stable and
whose views are nonempty at leader ticks, every `(slot, ballot)` key is
sent **at most once** along the whole run. -/
theorem spSentTrace_key_nodup {nP : Nat} (f : Nat)
    (cp : List P) (dPayload : List Nat) (me : Fin nP)
    (pb : Trace (Ballot nP)) (pl : Trace Bool)
    (p1bs : Trace (Multiset (ALog P nP)))
    (hown : ∀ b ∈ pb, (b : Ballot nP).proposerId = me)
    (hmono : ∀ {t t' : Nat} (h : t ≤ t') (ht' : t' < pb.length),
      (pb[t]'(Nat.lt_of_le_of_lt h ht')).num ≤ (pb[t']'ht').num)
    (hstable : ∀ {t : Nat} (ht1 : t + 1 < pl.length)
      (hb1 : t + 1 < pb.length),
      pl[t + 1]'ht1 = true → pl[t]'(Nat.lt_of_succ_lt ht1) = true →
      pb[t + 1]'hb1 = pb[t]'(Nat.lt_of_succ_lt hb1))
    (hlead_ne : ∀ {t : Nat} (hpl : t < pl.length)
      (hpr : t < p1bs.length),
      pl[t]'hpl = true → p1bs[t]'hpr ≠ 0) :
    (((spSentTrace .guarded f cp dPayload pb pl p1bs).flatten).map
      Prod.fst).Nodup := by
  rw [spSentTrace_eq_scan]
  -- component extraction over the product trace
  have hlen : ∀ {u : Nat}, u < (Trace.zip
      (Trace.zip p1bs (sliceCuts cp 0 dPayload))
      (Trace.zip (Trace.zip pb pl) (false :: pl))).length →
      u < p1bs.length ∧ u < (sliceCuts cp 0 dPayload).length
      ∧ u < pb.length ∧ u < pl.length := by
    intro u hu
    simp only [Trace.zip, List.length_zip, List.length_cons,
      Nat.lt_min] at hu
    omega
  have hget : ∀ (u : Nat) (hu : u < (Trace.zip
      (Trace.zip p1bs (sliceCuts cp 0 dPayload))
      (Trace.zip (Trace.zip pb pl) (false :: pl))).length),
      (Trace.zip (Trace.zip p1bs (sliceCuts cp 0 dPayload))
        (Trace.zip (Trace.zip pb pl) (false :: pl)))[u]'hu
      = ((p1bs[u]'(hlen hu).1,
          (sliceCuts cp 0 dPayload)[u]'(hlen hu).2.1),
         ((pb[u]'(hlen hu).2.2.1, pl[u]'(hlen hu).2.2.2),
          (false :: pl)[u]'(by
            simp only [List.length_cons]
            exact Nat.lt_succ_of_lt (hlen hu).2.2.2))) := by
    intro u hu
    simp only [Trace.zip]
    rw [List.getElem_zip, List.getElem_zip, List.getElem_zip,
      List.getElem_zip]
  refine (spSent_nodup_go f me _ none 0 ?_ ?_ ?_ ?_ ?_ ?_ ?_).1
  · -- ownership rides the ballot leg
    intro x hx
    obtain ⟨u, hu, rfl⟩ := List.mem_iff_getElem.mp hx
    rw [hget u hu]
    exact hown _ (List.getElem_mem _)
  · -- `num` ascent, pairwise
    rw [List.pairwise_iff_getElem]
    intro u u' hu hu' huu
    rw [hget u hu, hget u' hu']
    exact hmono (Nat.le_of_lt huu) (hlen hu').2.2.1
  · -- reign stability, chained
    rw [List.isChain_iff_getElem]
    intro u hu1
    have hu : u < (Trace.zip
        (Trace.zip p1bs (sliceCuts cp 0 dPayload))
        (Trace.zip (Trace.zip pb pl) (false :: pl))).length :=
      Nat.lt_of_succ_lt hu1
    rw [hget u hu, hget (u + 1) hu1]
    intro h1 h0
    exact hstable (hlen hu1).2.2.2 (hlen hu1).2.2.1 h1 h0
  · -- the deferred flag reads the previous tick
    rw [List.isChain_iff_getElem]
    intro u hu1
    have hu : u < (Trace.zip
        (Trace.zip p1bs (sliceCuts cp 0 dPayload))
        (Trace.zip (Trace.zip pb pl) (false :: pl))).length :=
      Nat.lt_of_succ_lt hu1
    rw [hget u hu, hget (u + 1) hu1]
    rfl
  · -- nonempty views at leader ticks
    intro x hx
    obtain ⟨u, hu, rfl⟩ := List.mem_iff_getElem.mp hx
    rw [hget u hu]
    intro hl
    exact hlead_ne (hlen hu).2.2.2 (hlen hu).1 hl
  · -- the seed register names no reign
    intro a ha
    cases ha
  · -- the seed deferred flag is down
    intro x hx hprev _
    have h0 : 0 < (Trace.zip
        (Trace.zip p1bs (sliceCuts cp 0 dPayload))
        (Trace.zip (Trace.zip pb pl) (false :: pl))).length := by
      cases hL : (Trace.zip
          (Trace.zip p1bs (sliceCuts cp 0 dPayload))
          (Trace.zip (Trace.zip pb pl) (false :: pl))) with
      | nil =>
        rw [hL] at hx
        cases hx
      | cons z zs =>
        exact Nat.zero_lt_succ _
    have hx0 : x = (Trace.zip
        (Trace.zip p1bs (sliceCuts cp 0 dPayload))
        (Trace.zip (Trace.zip pb pl) (false :: pl)))[0]'h0 := by
      revert hx h0
      cases (Trace.zip (Trace.zip p1bs (sliceCuts cp 0 dPayload))
          (Trace.zip (Trace.zip pb pl) (false :: pl))) with
      | nil =>
        intro hx
        cases hx
      | cons z zs =>
        intro hx h0
        exact (Option.some.inj hx).symm
    rw [hx0] at hprev
    have hcmp := congrArg (fun w : (Multiset (ALog P nP) × List P)
        × ((Ballot nP × Bool) × Bool) => w.2.2) (hget 0 h0)
    rw [hcmp] at hprev
    cases hprev

/-- The emission opening, generalized to arbitrary scan states (the
induction form over the fused scan; the gate trace rides along). -/
private theorem spSent_open_go {nP : Nat} (variant : PaxosVariant)
    (f : Nat) :
    ∀ (p1bs : Trace (Multiset (ALog P nP))) (sl : Trace (List P))
      (pb : Trace (Ballot nP)) (pl : Trace Bool) (dfl : Trace Bool)
      (ra : Option (Ballot nP)) (ns : Nat)
      {slot : Nat} {b : Ballot nP} {v : Option P},
      ((slot, b), v) ∈ (scanAcrossTicksTrace (spSendStep variant f)
        (ra, ns) (Trace.zip (Trace.zip p1bs sl)
          (Trace.zip (Trace.zip pb pl) dfl))).flatten →
      ∃ (t : Nat) (hpl : t < pl.length) (hpb : t < pb.length)
        (hgv : t < (scanAcrossTicksTrace
          (fun st bt => spGateStep variant.recommitOnce st bt.1 bt.2)
          ra (Trace.zip p1bs (Trace.zip (Trace.zip pb pl)
            dfl))).length),
        pl[t]'hpl = true ∧ pb[t]'hpb = b ∧
        ∀ e₀ : LogValue P nP,
          (slot, e₀) ∈ rcEntries ((scanAcrossTicksTrace
            (fun st bt => spGateStep variant.recommitOnce st bt.1 bt.2)
            ra (Trace.zip p1bs (Trace.zip (Trace.zip pb pl)
              dfl)))[t]'hgv) →
          ∃ best : LogValue P nP,
            (slot, best) ∈ logView (rcEntries ((scanAcrossTicksTrace
              (fun st bt => spGateStep variant.recommitOnce st bt.1
                bt.2) ra (Trace.zip p1bs (Trace.zip (Trace.zip pb pl)
                dfl)))[t]'hgv))
            ∧ v = best.value ∧ e₀.ballot.ble best.ballot = true
  | [], _, _, _, _, ra, ns, slot, b, v => by
    intro hmem
    simp [Trace.zip, scanAcrossTicksTrace] at hmem
  | _ :: _, [], _, _, _, ra, ns, slot, b, v => by
    intro hmem
    simp [Trace.zip, scanAcrossTicksTrace] at hmem
  | _ :: _, _ :: _, [], _, _, ra, ns, slot, b, v => by
    intro hmem
    simp [Trace.zip, scanAcrossTicksTrace] at hmem
  | _ :: _, _ :: _, _ :: _, [], _, ra, ns, slot, b, v => by
    intro hmem
    simp [Trace.zip, scanAcrossTicksTrace] at hmem
  | _ :: _, _ :: _, _ :: _, _ :: _, [], ra, ns, slot, b, v => by
    intro hmem
    simp [Trace.zip, scanAcrossTicksTrace] at hmem
  | g :: gs, s :: ss, b0 :: bs, l :: ls, d :: ds, ra, ns, slot, b,
      v => by
    intro hmem
    rcases List.mem_append.mp hmem with hhd | htl
    · -- the head tick emitted the key
      have hout : ((slot, b), v) ∈ (if l then
          (ipStep ns (if l then s else [])
            (rcMaxSlot (spGateStep variant.recommitOnce ra g
              ((b0, l), d)).2)).2.map
            (fun sp => ((sp.1, b0), some sp.2))
          ++ recommitList f b0
            (spGateStep variant.recommitOnce ra g ((b0, l), d)).2
        else []) := hhd
      by_cases hl : l = true
      case neg =>
        rw [if_neg hl] at hout
        cases hout
      subst hl
      rw [if_pos rfl] at hout
      have hgv0 : (scanAcrossTicksTrace (fun st bt =>
          spGateStep variant.recommitOnce st bt.1 bt.2) ra
          (Trace.zip (g :: gs) (Trace.zip (Trace.zip (b0 :: bs)
            (true :: ls)) (d :: ds))))[0]'(Nat.zero_lt_succ _)
          = (spGateStep variant.recommitOnce ra g ((b0, true), d)).2 :=
        rfl
      rcases List.mem_append.mp hout with hpay | hrc
      · -- fresh payload: strictly above the view's slots
        obtain ⟨sp, hsp, heq⟩ := List.mem_map.mp hpay
        have hb0 : b0 = b := congrArg (fun kv :
          (Nat × Ballot nP) × Option P => kv.1.2) heq
        have hslot : sp.1 = slot := congrArg (fun kv :
          (Nat × Ballot nP) × Option P => kv.1.1) heq
        refine ⟨0, Nat.zero_lt_succ _, Nat.zero_lt_succ _,
          Nat.zero_lt_succ _, rfl, hb0, ?_⟩
        intro e₀ he₀
        rw [hgv0] at he₀
        exfalso
        obtain ⟨m, hm, hslm⟩ := rcMaxSlot_ge he₀
        have hsp1 : sp.1 ∈ (ipStep ns (if true then s else [])
            (rcMaxSlot (spGateStep variant.recommitOnce ra g
              ((b0, true), d)).2)).2.map Prod.fst :=
          List.mem_map.mpr ⟨sp, hsp, rfl⟩
        rw [ipStep_slots, hm] at hsp1
        have hsp2 : sp.1 ∈ List.range' (m + 1)
            (if true then s else []).length := hsp1
        have := List.mem_range'_1.mp hsp2
        omega
      · -- recommit: the champion's value
        have hb0 : b0 = b :=
          (recommitList_ballot f b0 _ _ hrc).symm
        subst hb0
        refine ⟨0, Nat.zero_lt_succ _, Nat.zero_lt_succ _,
          Nat.zero_lt_succ _, rfl, rfl, ?_⟩
        intro e₀ he₀
        rw [hgv0] at he₀ ⊢
        exact recommitList_value_best f b0 _ hrc he₀
    · -- the tail
      obtain ⟨t, hpl', hpb', hgv', hflag, hball, hcov⟩ :=
        spSent_open_go variant f gs ss bs ls ds
          (spGateStep variant.recommitOnce ra g ((b0, l), d)).1
          (ipStep ns (if l then s else [])
            (rcMaxSlot (spGateStep variant.recommitOnce ra g
              ((b0, l), d)).2)).1 htl
      exact ⟨t + 1, Nat.succ_lt_succ hpl', Nat.succ_lt_succ hpb',
        Nat.succ_lt_succ hgv', hflag, hball, hcov⟩

/-- **Emission keys are owned and lead**: an emission pins a leader
tick carrying its ballot, and characterizes its value against the
tick's gated view (fresh payloads sit above the view's slots; recommit
values are the view's per-slot champions). -/
theorem spSentTrace_open {nP : Nat} (variant : PaxosVariant) (f : Nat)
    (cp : List P) (dPayload : List Nat)
    (pb : Trace (Ballot nP)) (pl : Trace Bool)
    (p1bs : Trace (Multiset (ALog P nP)))
    {slot : Nat} {b : Ballot nP} {v : Option P}
    (h : ((slot, b), v)
      ∈ (spSentTrace variant f cp dPayload pb pl p1bs).flatten) :
    ∃ (t : Nat) (hpl : t < pl.length) (hpb : t < pb.length)
      (hgv : t < (spGatedTrace variant.recommitOnce pb pl
        p1bs).length),
      pl[t]'hpl = true ∧ pb[t]'hpb = b ∧
      ∀ e₀ : LogValue P nP,
        (slot, e₀) ∈ rcEntries ((spGatedTrace variant.recommitOnce pb
          pl p1bs)[t]'hgv) →
        ∃ best : LogValue P nP,
          (slot, best) ∈ logView (rcEntries
            ((spGatedTrace variant.recommitOnce pb pl p1bs)[t]'hgv))
          ∧ v = best.value ∧ e₀.ballot.ble best.ballot = true := by
  rw [spSentTrace_eq_scan] at h
  exact spSent_open_go variant f p1bs (sliceCuts cp 0 dPayload) pb pl
    (false :: pl) none 0 h

/-! ## The devices (paxos_core's interface) -/

/-- An **emission**: `((slot, b), v)` left member `i`'s sequencing
pipeline at some realized tick (inputs are the member's own). -/
def SPEmission {nP : Nat} (variant : PaxosVariant) (f : Nat)
    (cp : List P) (dPayload : List Nat)
    (pb : Trace (Ballot nP)) (pl : Trace Bool)
    (p1bs : Trace (Multiset (ALog P nP)))
    (slot : Nat) (b : Ballot nP) (v : Option P) : Prop :=
  ((slot, b), v)
    ∈ (spSentTrace variant f cp dPayload pb pl p1bs).flatten

/-- A **chosen key**: `f + 1` distinct acceptors each carry a vote —
the `a_max_ballot` input holds exactly the ballot at a tick whose
published `a_log` output already covers the slot at that ballot
(write-before-ack, on the output wire). -/
def SPChosen {nA nP : Nat} (f : Nat)
    (ck : Fin nA → Trace (Option Nat))
    (mx : Fin nA → Trace (Option (Ballot nP)))
    (alog : Fin nA → Trace (ALog P nP))
    (slot : Nat) (b : Ballot nP) : Prop :=
  ∃ C : List (Fin nA), C.Nodup ∧ f + 1 ≤ C.length ∧
    ∀ j ∈ C, ∃ (t : Nat) (hta : t < (mx j).length),
      (mx j)[t]'hta = some b
      ∧ (t < (ck j).length →
          ∃ htl : t < (alog j).length,
            LogCovers ((alog j)[t]'htl).2 slot b)

/-- What `sequence_payload` **requires** of its ballot/leader/view
inputs — exactly `leader_election`'s guarantees (`LEEnsures`),
projected. -/
structure SPRequires (nP : Nat) (P : Type) [DecidableEq P]
    (pb : Fin nP → Trace (Ballot nP)) (pl : Fin nP → Trace Bool)
    (p1bs : Fin nP → Trace (Multiset (ALog P nP))) : Prop where
  /-- Every realized ballot is its member's own
  (`LEEnsures.own`). -/
  own : ∀ (i : Fin nP), ∀ b ∈ pb i, (b : Ballot nP).proposerId = i
  /-- Ballot numbers only ascend along the tick trace — `p_ballot`'s
  `Monotonic` wire type, projected. -/
  mono : ∀ (i : Fin nP) {t t' : Nat} (h : t ≤ t')
    (ht' : t' < (pb i).length),
    ((pb i)[t]'(Nat.lt_of_le_of_lt h ht')).num ≤ ((pb i)[t']'ht').num
  /-- Leader ticks see nonempty views (`LEEnsures.lead_ne`) — the
  gate's rebase always has a view to rebase on. -/
  lead_ne : ∀ (i : Fin nP) {t : Nat} (hpl : t < (pl i).length)
    (hpr : t < (p1bs i).length),
    (pl i)[t]'hpl = true → (p1bs i)[t]'hpr ≠ 0
  /-- Consecutive leader ticks share the ballot
  (`LEEnsures.stable`, FINDINGS D21) — a reign is one ballot. -/
  stable : ∀ (i : Fin nP) {t : Nat} (ht1 : t + 1 < (pl i).length)
    (hb1 : t + 1 < (pb i).length),
    (pl i)[t + 1]'ht1 = true →
    (pl i)[t]'(Nat.lt_of_succ_lt ht1) = true →
    (pb i)[t + 1]'hb1 = (pb i)[t]'(Nat.lt_of_succ_lt hb1)
  /-- Same-ballot leader ticks see the same view
  (`LEEnsures.pinned`) — frozen quorum buckets. -/
  pinned : ∀ (i : Fin nP) {t t' : Nat} (hpl : t < (pl i).length)
    (hpl' : t' < (pl i).length)
    (hpr : t < (p1bs i).length) (hpr' : t' < (p1bs i).length)
    (hpb : t < (pb i).length) (hpb' : t' < (pb i).length),
    (pl i)[t]'hpl = true → (pl i)[t']'hpl' = true →
    ((pb i)[t]'hpb).num = ((pb i)[t']'hpb').num →
    (p1bs i)[t]'hpr = (p1bs i)[t']'hpr'


/-- The input-view emission opening, generalized to arbitrary scan
states: the reign registers carry "every view of the recommitted reign
sits below the slot register", so payload emissions are strictly above
their tick's own view and recommit emissions read it verbatim. -/
private theorem spSent_open_input_go {nP : Nat} (f : Nat) :
    ∀ (L : List ((Multiset (ALog P nP) × List P)
        × ((Ballot nP × Bool) × Bool)))
      (ra : Option (Ballot nP)) (ns : Nat),
      List.Pairwise (fun x y =>
        (x : (Multiset (ALog P nP) × List P)
          × ((Ballot nP × Bool) × Bool)).2.1.2 = true →
        (y : (Multiset (ALog P nP) × List P)
          × ((Ballot nP × Bool) × Bool)).2.1.2 = true →
        x.2.1.1.num = y.2.1.1.num → x.1.1 = y.1.1) L →
      List.IsChain (fun x y => y.2.1.2 = true → x.2.1.2 = true →
        y.2.1.1 = x.2.1.1) L →
      List.IsChain (fun x y => y.2.2 = x.2.1.2) L →
      (∀ x ∈ L, (x : (Multiset (ALog P nP) × List P)
        × ((Ballot nP × Bool) × Bool)).2.1.2 = true → x.1.1 ≠ 0) →
      (∀ a, ra = some a → ∀ y ∈ L,
        (y : (Multiset (ALog P nP) × List P)
          × ((Ballot nP × Bool) × Bool)).2.1.2 = true →
        y.2.1.1 = a →
        ∀ (sl : Nat) (e₀ : LogValue P nP),
          (sl, e₀) ∈ rcEntries y.1.1 → sl < ns) →
      (∀ x, L.head? = some x → (x : (Multiset (ALog P nP) × List P)
        × ((Ballot nP × Bool) × Bool)).2.2 = true →
        x.2.1.2 = true → ra = some x.2.1.1) →
      ∀ (slot : Nat) (b : Ballot nP) (v : Option P),
      ((slot, b), v) ∈ (scanAcrossTicksTrace (spSendStep .guarded f)
        (ra, ns) L).flatten →
      ∃ (t : Nat) (ht : t < L.length),
        (L[t]'ht).2.1.2 = true ∧ (L[t]'ht).2.1.1 = b ∧
        ∀ e₀ : LogValue P nP,
          (slot, e₀) ∈ rcEntries (L[t]'ht).1.1 →
          ∃ best : LogValue P nP,
            (slot, best) ∈ logView (rcEntries (L[t]'ht).1.1)
            ∧ v = best.value ∧ e₀.ballot.ble best.ballot = true
  | [], ra, ns, _, _, _, _, _, _, slot, b, v => by
    intro hmem
    simp [scanAcrossTicksTrace] at hmem
  | x :: L', ra, ns, hpin, hstable, hdfl, hlead, hinvview, hinvprev,
      slot, b, v => by
    intro hmem
    obtain ⟨⟨g, s⟩, ⟨b0, l⟩, d⟩ := x
    have hpin' := (List.pairwise_cons.mp hpin).2
    have hpin0 := (List.pairwise_cons.mp hpin).1
    have hstable' := hstable.tail
    have hdfl' := hdfl.tail
    have hlead' : ∀ y ∈ L', (y : (Multiset (ALog P nP) × List P)
        × ((Ballot nP × Bool) × Bool)).2.1.2 = true → y.1.1 ≠ 0 :=
      fun y hy => hlead y (List.mem_cons_of_mem _ hy)
    have hdfl0 : ∀ y, L'.head? = some y → y.2.2 = l := by
      intro y hy
      exact hdfl.rel_head? (by rw [hy]; exact rfl)
    have hstable0 : ∀ y, L'.head? = some y → y.2.1.2 = true →
        l = true → y.2.1.1 = b0 := by
      intro y hy hylead hl
      exact hstable.rel_head? (by rw [hy]; exact rfl) hylead hl
    by_cases hl : l = true
    case neg =>
      have hl' : l = false := by
        cases l
        · rfl
        · exact absurd rfl hl
      subst hl'
      have hfire : (decide (g ≠ 0)
          && (false && !d && !decide (ra = some b0))) = false := by
        simp
      have hra' : (spGateStep PaxosVariant.guarded.recommitOnce ra g
          ((b0, false), d)).1 = ra := by
        show (if (decide (g ≠ 0)
          && (false && !d && !decide (ra = some b0))) = true
          then some b0 else ra) = ra
        rw [hfire]
        rfl
      have hgv : (spGateStep PaxosVariant.guarded.recommitOnce ra g
          ((b0, false), d)).2 = 0 := by
        show (if (decide (g ≠ 0)
          && (false && !d && !decide (ra = some b0))) = true
          then g else 0) = 0
        rw [hfire]
        rfl
      have hstep : spSendStep .guarded f (ra, ns)
          ((g, s), ((b0, false), d)) = ((ra, ns), []) := by
        show ((( spGateStep PaxosVariant.guarded.recommitOnce ra g
            ((b0, false), d)).1,
          (ipStep ns (if false then s else [])
            (rcMaxSlot (spGateStep PaxosVariant.guarded.recommitOnce
              ra g ((b0, false), d)).2)).1),
          if false then _ else []) = ((ra, ns), [])
        rw [hra', hgv, rcMaxSlot_zero]
        rfl
      have hscan : scanAcrossTicksTrace (spSendStep .guarded f)
          (ra, ns) (((g, s), ((b0, false), d)) :: L')
          = [] :: scanAcrossTicksTrace (spSendStep .guarded f)
            (ra, ns) L' := by
        show (spSendStep .guarded f (ra, ns)
            ((g, s), ((b0, false), d))).2
          :: scanAcrossTicksTrace (spSendStep .guarded f)
            (spSendStep .guarded f (ra, ns)
              ((g, s), ((b0, false), d))).1 L' = _
        rw [hstep]
      rw [hscan] at hmem
      simp only [List.flatten_cons, List.nil_append] at hmem
      have hinvview' : ∀ a, ra = some a → ∀ y ∈ L',
          (y : (Multiset (ALog P nP) × List P)
            × ((Ballot nP × Bool) × Bool)).2.1.2 = true →
          y.2.1.1 = a →
          ∀ (sl : Nat) (e₀ : LogValue P nP),
            (sl, e₀) ∈ rcEntries y.1.1 → sl < ns :=
        fun a ha y hy => hinvview a ha y (List.mem_cons_of_mem _ hy)
      have hinvprev' : ∀ y, L'.head? = some y → y.2.2 = true →
          y.2.1.2 = true → ra = some y.2.1.1 := by
        intro y hy hprev _
        rw [hdfl0 y hy] at hprev
        cases hprev
      obtain ⟨t, ht, h1, h2, h3⟩ := spSent_open_input_go f L' ra ns
        hpin' hstable' hdfl' hlead' hinvview' hinvprev' slot b v hmem
      exact ⟨t + 1, Nat.succ_lt_succ ht, h1, h2, h3⟩
    case pos =>
      subst hl
      have hg0 : g ≠ 0 := hlead _ (List.mem_cons_self ..) rfl
      by_cases hra : ra = some b0
      · -- reign continuation: gate closed, fresh payloads from `ns`
        have hfire : (decide (g ≠ 0)
            && (true && !d && !decide (ra = some b0))) = false := by
          rw [decide_eq_true hra]
          simp
        have hra' : (spGateStep PaxosVariant.guarded.recommitOnce ra g
            ((b0, true), d)).1 = ra := by
          show (if (decide (g ≠ 0)
            && (true && !d && !decide (ra = some b0))) = true
            then some b0 else ra) = ra
          rw [hfire]
          rfl
        have hgv : (spGateStep PaxosVariant.guarded.recommitOnce ra g
            ((b0, true), d)).2 = 0 := by
          show (if (decide (g ≠ 0)
            && (true && !d && !decide (ra = some b0))) = true
            then g else 0) = 0
          rw [hfire]
          rfl
        have hout : (spSendStep .guarded f (ra, ns)
            ((g, s), ((b0, true), d))).2
            = (ipStep ns s none).2.map
              (fun sp => ((sp.1, b0), some sp.2)) := by
          show (if true then
              (ipStep ns (if true then s else [])
                (rcMaxSlot (spGateStep
                  PaxosVariant.guarded.recommitOnce ra g
                  ((b0, true), d)).2)).2.map
                (fun sp => ((sp.1, b0), some sp.2))
              ++ recommitList f b0 (spGateStep
                PaxosVariant.guarded.recommitOnce ra g
                ((b0, true), d)).2
            else []) = _
          rw [hgv, rcMaxSlot_zero,
            recommitList_eq_nil_of_max_none f b0 rcMaxSlot_zero,
            List.append_nil]
          rfl
        have hst' : (spSendStep .guarded f (ra, ns)
            ((g, s), ((b0, true), d))).1 = (ra, ns + s.length) := by
          show ((spGateStep PaxosVariant.guarded.recommitOnce ra g
              ((b0, true), d)).1,
            (ipStep ns (if true then s else [])
              (rcMaxSlot (spGateStep
                PaxosVariant.guarded.recommitOnce ra g
                ((b0, true), d)).2)).1) = _
          rw [hra', hgv, rcMaxSlot_zero]
          rfl
        have hscan : scanAcrossTicksTrace (spSendStep .guarded f)
            (ra, ns) (((g, s), ((b0, true), d)) :: L')
            = (ipStep ns s none).2.map
                (fun sp => ((sp.1, b0), some sp.2))
              :: scanAcrossTicksTrace (spSendStep .guarded f)
                (ra, ns + s.length) L' := by
          show (spSendStep .guarded f (ra, ns)
              ((g, s), ((b0, true), d))).2
            :: scanAcrossTicksTrace (spSendStep .guarded f)
              (spSendStep .guarded f (ra, ns)
                ((g, s), ((b0, true), d))).1 L' = _
          rw [hst', hout]
        rw [hscan] at hmem
        simp only [List.flatten_cons] at hmem
        rcases List.mem_append.mp hmem with hhd | htl
        · -- head payload: fresh slots above this reign's pinned view
          obtain ⟨sp, hsp, heq⟩ := List.mem_map.mp hhd
          have hb0 : b0 = b := congrArg (fun kv :
            (Nat × Ballot nP) × Option P => kv.1.2) heq
          have hslot : sp.1 = slot := congrArg (fun kv :
            (Nat × Ballot nP) × Option P => kv.1.1) heq
          refine ⟨0, Nat.zero_lt_succ _, rfl, hb0, ?_⟩
          intro e₀ he₀
          simp only [List.getElem_cons_zero] at he₀
          exfalso
          have hview := hinvview b0 hra _ (List.mem_cons_self ..)
            rfl rfl slot e₀ he₀
          have hsp1 : sp.1 ∈ (ipStep ns s none).2.map Prod.fst :=
            List.mem_map.mpr ⟨sp, hsp, rfl⟩
          rw [ipStep_slots] at hsp1
          have hrng := List.mem_range'_1.mp hsp1
          have hrng2 : ns ≤ sp.1 ∧ sp.1 < ns + s.length := hrng
          omega
        · have hinvview₂ : ∀ a, ra = some a → ∀ y ∈ L',
              (y : (Multiset (ALog P nP) × List P)
                × ((Ballot nP × Bool) × Bool)).2.1.2 = true →
              y.2.1.1 = a →
              ∀ (sl : Nat) (e₀ : LogValue P nP),
                (sl, e₀) ∈ rcEntries y.1.1 → sl < ns + s.length := by
            intro a ha y hy hylead hyb sl e₀ he
            exact Nat.lt_of_lt_of_le
              (hinvview a ha y (List.mem_cons_of_mem _ hy) hylead hyb
                sl e₀ he)
              (Nat.le_add_right ..)
          have hinvprev₂ : ∀ y, L'.head? = some y → y.2.2 = true →
              y.2.1.2 = true → ra = some y.2.1.1 := by
            intro y hy _ hylead
            rw [hstable0 y hy hylead rfl]
            exact hra
          obtain ⟨t, ht, h1, h2, h3⟩ := spSent_open_input_go f L' ra
            (ns + s.length) hpin' hstable' hdfl' hlead' hinvview₂
            hinvprev₂ slot b v htl
          exact ⟨t + 1, Nat.succ_lt_succ ht, h1, h2, h3⟩
      · by_cases hd : d = true
        · -- mid-run without a recommit on record: impossible
          exact absurd (hinvprev _ rfl hd rfl) hra
        · -- the reign's first leader tick: the gate fires
          have hdf : d = false := by
            cases d
            · rfl
            · exact absurd rfl hd
          subst hdf
          have hfire : (decide (g ≠ 0)
              && (true && !false && !decide (ra = some b0)))
              = true := by
            rw [decide_eq_true hg0, decide_eq_false hra]
            rfl
          have hra' : (spGateStep PaxosVariant.guarded.recommitOnce ra
              g ((b0, true), false)).1 = some b0 := by
            show (if (decide (g ≠ 0)
              && (true && !false && !decide (ra = some b0))) = true
              then some b0 else ra) = some b0
            rw [hfire]
            rfl
          have hgv : (spGateStep PaxosVariant.guarded.recommitOnce ra
              g ((b0, true), false)).2 = g := by
            show (if (decide (g ≠ 0)
              && (true && !false && !decide (ra = some b0))) = true
              then g else 0) = g
            rw [hfire]
            rfl
          have hinvprev₂ : ∀ y, L'.head? = some y → y.2.2 = true →
              y.2.1.2 = true →
              (some b0 : Option (Ballot nP)) = some y.2.1.1 := by
            intro y hy _ hylead
            rw [hstable0 y hy hylead rfl]
          have hpin₂ : ∀ y ∈ L',
              (y : (Multiset (ALog P nP) × List P)
                × ((Ballot nP × Bool) × Bool)).2.1.2 = true →
              y.2.1.1 = b0 → y.1.1 = g := by
            intro y hy hylead hyb
            have := hpin0 y hy rfl hylead (by rw [hyb])
            exact this.symm
          cases hm : rcMaxSlot g with
          | none =>
            -- a keyless view: nothing to recommit, payloads from `ns`
            have hout : (spSendStep .guarded f (ra, ns)
                ((g, s), ((b0, true), false))).2
                = (ipStep ns s none).2.map
                  (fun sp => ((sp.1, b0), some sp.2)) := by
              show (if true then
                  (ipStep ns (if true then s else [])
                    (rcMaxSlot (spGateStep
                      PaxosVariant.guarded.recommitOnce ra g
                      ((b0, true), false)).2)).2.map
                    (fun sp => ((sp.1, b0), some sp.2))
                  ++ recommitList f b0 (spGateStep
                    PaxosVariant.guarded.recommitOnce ra g
                    ((b0, true), false)).2
                else []) = _
              rw [hgv, hm, recommitList_eq_nil_of_max_none f b0 hm,
                List.append_nil]
              rfl
            have hst' : (spSendStep .guarded f (ra, ns)
                ((g, s), ((b0, true), false))).1
                = (some b0, ns + s.length) := by
              show ((spGateStep PaxosVariant.guarded.recommitOnce ra g
                  ((b0, true), false)).1,
                (ipStep ns (if true then s else [])
                  (rcMaxSlot (spGateStep
                    PaxosVariant.guarded.recommitOnce ra g
                    ((b0, true), false)).2)).1) = _
              rw [hra', hgv, hm]
              rfl
            have hscan : scanAcrossTicksTrace (spSendStep .guarded f)
                (ra, ns) (((g, s), ((b0, true), false)) :: L')
                = (ipStep ns s none).2.map
                    (fun sp => ((sp.1, b0), some sp.2))
                  :: scanAcrossTicksTrace (spSendStep .guarded f)
                    (some b0, ns + s.length) L' := by
              show (spSendStep .guarded f (ra, ns)
                  ((g, s), ((b0, true), false))).2
                :: scanAcrossTicksTrace (spSendStep .guarded f)
                  (spSendStep .guarded f (ra, ns)
                    ((g, s), ((b0, true), false))).1 L' = _
              rw [hst', hout]
            rw [hscan] at hmem
            simp only [List.flatten_cons] at hmem
            rcases List.mem_append.mp hmem with hhd | htl
            · obtain ⟨sp, hsp, heq⟩ := List.mem_map.mp hhd
              have hb0 : b0 = b := congrArg (fun kv :
                (Nat × Ballot nP) × Option P => kv.1.2) heq
              refine ⟨0, Nat.zero_lt_succ _, rfl, hb0, ?_⟩
              intro e₀ he₀
              simp only [List.getElem_cons_zero] at he₀
              exfalso
              obtain ⟨m, hm', -⟩ := rcMaxSlot_ge he₀
              rw [hm] at hm'
              cases hm'
            · have hinvview₂ : ∀ a,
                  (some b0 : Option (Ballot nP)) = some a →
                  ∀ y ∈ L', (y : (Multiset (ALog P nP) × List P)
                    × ((Ballot nP × Bool) × Bool)).2.1.2 = true →
                  y.2.1.1 = a →
                  ∀ (sl : Nat) (e₀ : LogValue P nP),
                    (sl, e₀) ∈ rcEntries y.1.1 →
                    sl < ns + s.length := by
                intro a ha y hy hylead hyb sl e₀ he
                cases Option.some.inj ha
                rw [hpin₂ y hy hylead hyb] at he
                exfalso
                obtain ⟨m, hm', -⟩ := rcMaxSlot_ge he
                rw [hm] at hm'
                cases hm'
              obtain ⟨t, ht, h1, h2, h3⟩ := spSent_open_input_go f L'
                (some b0) (ns + s.length) hpin' hstable' hdfl' hlead'
                hinvview₂ hinvprev₂ slot b v htl
              exact ⟨t + 1, Nat.succ_lt_succ ht, h1, h2, h3⟩
          | some m =>
            -- the rebase: recommits read the view, payloads sit above
            have hout : (spSendStep .guarded f (ra, ns)
                ((g, s), ((b0, true), false))).2
                = (ipStep ns s (some m)).2.map
                    (fun sp => ((sp.1, b0), some sp.2))
                  ++ recommitList f b0 g := by
              show (if true then
                  (ipStep ns (if true then s else [])
                    (rcMaxSlot (spGateStep
                      PaxosVariant.guarded.recommitOnce ra g
                      ((b0, true), false)).2)).2.map
                    (fun sp => ((sp.1, b0), some sp.2))
                  ++ recommitList f b0 (spGateStep
                    PaxosVariant.guarded.recommitOnce ra g
                    ((b0, true), false)).2
                else []) = _
              rw [hgv, hm]
              rfl
            have hst' : (spSendStep .guarded f (ra, ns)
                ((g, s), ((b0, true), false))).1
                = (some b0, m + 1 + s.length) := by
              show ((spGateStep PaxosVariant.guarded.recommitOnce ra g
                  ((b0, true), false)).1,
                (ipStep ns (if true then s else [])
                  (rcMaxSlot (spGateStep
                    PaxosVariant.guarded.recommitOnce ra g
                    ((b0, true), false)).2)).1) = _
              rw [hra', hgv, hm]
              rfl
            have hscan : scanAcrossTicksTrace (spSendStep .guarded f)
                (ra, ns) (((g, s), ((b0, true), false)) :: L')
                = ((ipStep ns s (some m)).2.map
                      (fun sp => ((sp.1, b0), some sp.2))
                    ++ recommitList f b0 g)
                  :: scanAcrossTicksTrace (spSendStep .guarded f)
                    (some b0, m + 1 + s.length) L' := by
              show (spSendStep .guarded f (ra, ns)
                  ((g, s), ((b0, true), false))).2
                :: scanAcrossTicksTrace (spSendStep .guarded f)
                  (spSendStep .guarded f (ra, ns)
                    ((g, s), ((b0, true), false))).1 L' = _
              rw [hst', hout]
            rw [hscan] at hmem
            simp only [List.flatten_cons] at hmem
            rcases List.mem_append.mp hmem with hhd | htl
            · rcases List.mem_append.mp hhd with hpay | hrc
              · -- fresh payload: above the view's slots
                obtain ⟨sp, hsp, heq⟩ := List.mem_map.mp hpay
                have hb0 : b0 = b := congrArg (fun kv :
                  (Nat × Ballot nP) × Option P => kv.1.2) heq
                have hslot : sp.1 = slot := congrArg (fun kv :
                  (Nat × Ballot nP) × Option P => kv.1.1) heq
                refine ⟨0, Nat.zero_lt_succ _, rfl, hb0, ?_⟩
                intro e₀ he₀
                simp only [List.getElem_cons_zero] at he₀
                exfalso
                obtain ⟨m', hm', hslm⟩ := rcMaxSlot_ge he₀
                rw [hm] at hm'
                injection hm' with hmm
                have hsp1 : sp.1 ∈ (ipStep ns s (some m)).2.map
                    Prod.fst :=
                  List.mem_map.mpr ⟨sp, hsp, rfl⟩
                rw [ipStep_slots] at hsp1
                have hrng := List.mem_range'_1.mp hsp1
                have hrng2 : m + 1 ≤ sp.1 ∧ sp.1 < m + 1 + s.length :=
                  hrng
                omega
              · -- recommit: the champion's value, off the input view
                have hb0 : b0 = b :=
                  (recommitList_ballot f b0 _ _ hrc).symm
                subst hb0
                refine ⟨0, Nat.zero_lt_succ _, rfl, rfl, ?_⟩
                intro e₀ he₀
                simp only [List.getElem_cons_zero] at he₀ ⊢
                exact recommitList_value_best f b0 _ hrc he₀
            · have hinvview₂ : ∀ a,
                  (some b0 : Option (Ballot nP)) = some a →
                  ∀ y ∈ L', (y : (Multiset (ALog P nP) × List P)
                    × ((Ballot nP × Bool) × Bool)).2.1.2 = true →
                  y.2.1.1 = a →
                  ∀ (sl : Nat) (e₀ : LogValue P nP),
                    (sl, e₀) ∈ rcEntries y.1.1 →
                    sl < m + 1 + s.length := by
                intro a ha y hy hylead hyb sl e₀ he
                cases Option.some.inj ha
                rw [hpin₂ y hy hylead hyb] at he
                obtain ⟨m', hm', hslm⟩ := rcMaxSlot_ge he
                rw [hm] at hm'
                injection hm' with hmm
                omega
              obtain ⟨t, ht, h1, h2, h3⟩ := spSent_open_input_go f L'
                (some b0) (m + 1 + s.length) hpin' hstable' hdfl'
                hlead' hinvview₂ hinvprev₂ slot b v htl
              exact ⟨t + 1, Nat.succ_lt_succ ht, h1, h2, h3⟩

/-- **Emission opening, at the input view** (`spSentTrace_open`
strengthened through the reign invariants): under the `SPRequires`-shape
hypotheses, an emission pins a leader tick carrying its ballot whose
**input** view characterizes its value — payload slots sit strictly
above every covered slot of the tick's view (one rebase per reign +
pinned views), and recommit values are the view's champions. -/
theorem spSentTrace_open_input {nP : Nat} (f : Nat)
    (cp : List P) (dPayload : List Nat) (me : Fin nP)
    (pb : Trace (Ballot nP)) (pl : Trace Bool)
    (p1bs : Trace (Multiset (ALog P nP)))
    (hown : ∀ b ∈ pb, (b : Ballot nP).proposerId = me)
    (hmono : ∀ {t t' : Nat} (h : t ≤ t') (ht' : t' < pb.length),
      (pb[t]'(Nat.lt_of_le_of_lt h ht')).num ≤ (pb[t']'ht').num)
    (hstable : ∀ {t : Nat} (ht1 : t + 1 < pl.length)
      (hb1 : t + 1 < pb.length),
      pl[t + 1]'ht1 = true → pl[t]'(Nat.lt_of_succ_lt ht1) = true →
      pb[t + 1]'hb1 = pb[t]'(Nat.lt_of_succ_lt hb1))
    (hlead_ne : ∀ {t : Nat} (hpl : t < pl.length)
      (hpr : t < p1bs.length),
      pl[t]'hpl = true → p1bs[t]'hpr ≠ 0)
    (hpinned : ∀ {t t' : Nat} (hpl : t < pl.length)
      (hpl' : t' < pl.length)
      (hpr : t < p1bs.length) (hpr' : t' < p1bs.length)
      (hpb : t < pb.length) (hpb' : t' < pb.length),
      pl[t]'hpl = true → pl[t']'hpl' = true →
      (pb[t]'hpb).num = (pb[t']'hpb').num →
      p1bs[t]'hpr = p1bs[t']'hpr')
    {slot : Nat} {b : Ballot nP} {v : Option P}
    (h : ((slot, b), v)
      ∈ (spSentTrace .guarded f cp dPayload pb pl p1bs).flatten) :
    ∃ (t : Nat) (hpl : t < pl.length) (hpb : t < pb.length)
      (hpr : t < p1bs.length),
      pl[t]'hpl = true ∧ pb[t]'hpb = b ∧
      ∀ e₀ : LogValue P nP,
        (slot, e₀) ∈ rcEntries (p1bs[t]'hpr) →
        ∃ best : LogValue P nP,
          (slot, best) ∈ logView (rcEntries (p1bs[t]'hpr))
          ∧ v = best.value ∧ e₀.ballot.ble best.ballot = true := by
  rw [spSentTrace_eq_scan] at h
  -- component extraction over the product trace
  have hlen : ∀ {u : Nat}, u < (Trace.zip
      (Trace.zip p1bs (sliceCuts cp 0 dPayload))
      (Trace.zip (Trace.zip pb pl) (false :: pl))).length →
      u < p1bs.length ∧ u < (sliceCuts cp 0 dPayload).length
      ∧ u < pb.length ∧ u < pl.length := by
    intro u hu
    simp only [Trace.zip, List.length_zip, List.length_cons,
      Nat.lt_min] at hu
    omega
  have hget : ∀ (u : Nat) (hu : u < (Trace.zip
      (Trace.zip p1bs (sliceCuts cp 0 dPayload))
      (Trace.zip (Trace.zip pb pl) (false :: pl))).length),
      (Trace.zip (Trace.zip p1bs (sliceCuts cp 0 dPayload))
        (Trace.zip (Trace.zip pb pl) (false :: pl)))[u]'hu
      = ((p1bs[u]'(hlen hu).1,
          (sliceCuts cp 0 dPayload)[u]'(hlen hu).2.1),
         ((pb[u]'(hlen hu).2.2.1, pl[u]'(hlen hu).2.2.2),
          (false :: pl)[u]'(by
            simp only [List.length_cons]
            exact Nat.lt_succ_of_lt (hlen hu).2.2.2))) := by
    intro u hu
    simp only [Trace.zip]
    rw [List.getElem_zip, List.getElem_zip, List.getElem_zip,
      List.getElem_zip]
  obtain ⟨t, ht, hflag, hball, hcov⟩ := spSent_open_input_go f _
    none 0
    (by -- the pin, pairwise
      rw [List.pairwise_iff_getElem]
      intro u u' hu hu' huu
      rw [hget u hu, hget u' hu']
      intro h1 h2 hnum
      exact hpinned (hlen hu).2.2.2 (hlen hu').2.2.2 (hlen hu).1
        (hlen hu').1 (hlen hu).2.2.1 (hlen hu').2.2.1 h1 h2 hnum)
    (by -- reign stability, chained
      rw [List.isChain_iff_getElem]
      intro u hu1
      have hu : u < (Trace.zip
          (Trace.zip p1bs (sliceCuts cp 0 dPayload))
          (Trace.zip (Trace.zip pb pl) (false :: pl))).length :=
        Nat.lt_of_succ_lt hu1
      rw [hget u hu, hget (u + 1) hu1]
      intro h1 h0
      exact hstable (hlen hu1).2.2.2 (hlen hu1).2.2.1 h1 h0)
    (by -- the deferred flag reads the previous tick
      rw [List.isChain_iff_getElem]
      intro u hu1
      have hu : u < (Trace.zip
          (Trace.zip p1bs (sliceCuts cp 0 dPayload))
          (Trace.zip (Trace.zip pb pl) (false :: pl))).length :=
        Nat.lt_of_succ_lt hu1
      rw [hget u hu, hget (u + 1) hu1]
      rfl)
    (by -- nonempty views at leader ticks
      intro x hx
      obtain ⟨u, hu, rfl⟩ := List.mem_iff_getElem.mp hx
      rw [hget u hu]
      intro hl
      exact hlead_ne (hlen hu).2.2.2 (hlen hu).1 hl)
    (by -- the seed register names no reign
      intro a ha
      cases ha)
    (by -- the seed deferred flag is down
      intro x hx hprev _
      have h0 : 0 < (Trace.zip
          (Trace.zip p1bs (sliceCuts cp 0 dPayload))
          (Trace.zip (Trace.zip pb pl) (false :: pl))).length := by
        cases hL : (Trace.zip
            (Trace.zip p1bs (sliceCuts cp 0 dPayload))
            (Trace.zip (Trace.zip pb pl) (false :: pl))) with
        | nil =>
          rw [hL] at hx
          cases hx
        | cons z zs =>
          exact Nat.zero_lt_succ _
      have hx0 : x = (Trace.zip
          (Trace.zip p1bs (sliceCuts cp 0 dPayload))
          (Trace.zip (Trace.zip pb pl) (false :: pl)))[0]'h0 := by
        revert hx h0
        cases (Trace.zip (Trace.zip p1bs (sliceCuts cp 0 dPayload))
            (Trace.zip (Trace.zip pb pl) (false :: pl))) with
        | nil =>
          intro hx
          cases hx
        | cons z zs =>
          intro hx h0
          exact (Option.some.inj hx).symm
      rw [hx0] at hprev
      have hcmp := congrArg (fun w : (Multiset (ALog P nP) × List P)
          × ((Ballot nP × Bool) × Bool) => w.2.2) (hget 0 h0)
      rw [hcmp] at hprev
      cases hprev)
    slot b v h
  -- transfer the structural conclusion to the component traces
  refine ⟨t, (hlen ht).2.2.2, (hlen ht).2.2.1, (hlen ht).1,
    ?_, ?_, ?_⟩
  · have h1 := hflag
    rw [hget t ht] at h1
    exact h1
  · have h1 := hball
    rw [hget t ht] at h1
    exact h1
  · intro e₀ he₀
    have hcov' := hcov e₀ (by
      rw [hget t ht]
      exact he₀)
    rw [hget t ht] at hcov'
    exact hcov'

end HydroV2
