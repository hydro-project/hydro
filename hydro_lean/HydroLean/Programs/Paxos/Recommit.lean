import HydroLean.Programs.Paxos.PP1b
import HydroLean.Hydro.StreamLemmas

/-!
# `recommit_after_leader_election` (paxos.rs:596–672) — module

From a quorum's accepted logs and the new leader's ballot, compute the
entries to re-propose and the max slot:
- merge the logs per slot, keeping the **max-ballot** entry and counting how
  many quorum members reported it (`p_p1b_highest_entries_and_count`,
  paxos.rs:614–639);
- re-commit entries whose count is `≤ f` (those with `count > f` are already
  known-committed and are skipped), at OUR ballot (`p_log_to_try_commit`,
  paxos.rs:640–654; checkpoint dropped);
- fill the log holes below the max slot with `None` payloads
  (`p_log_holes`, paxos.rs:657–669);
- expose `p_max_slot` (paxos.rs:655) to rebase `index_payloads`.

The per-slot merge closure carries the Rust
`commutative = manual_proof!(/** TODO */)` (paxos.rs:638). Its formal content
— order-independence of the fold — holds because "max ballot per slot" is a
commutative-idempotent reduction *given* one-value-per-(slot, ballot) (the
phase-2 invariant); our `mergeQuorumLogs` computes it as a pure function of
the entry multiset via max-selection, and the spec lemma
`mergeQuorumLogs_max_ballot` states the essential property the safety proof
uses: the merged entry for a slot carries the highest ballot present among
the quorum's entries for that slot.
-/

namespace HydroLean.Programs.Paxos

variable {P : Type} {nP : Nat}

/-- The max-ballot selection step of the per-slot merge (the fold closure at
paxos.rs:618–638, reduced to its ballot-max essence). -/
def maxEntryStep (acc : Option (LogValue P nP)) (e : LogValue P nP) :
    Option (LogValue P nP) :=
  match acc with
  | none => some e
  | some a => if a.ballot.blt e.ballot then some e else some a

/-- Per-slot merge (paxos.rs:614–639): for each slot present in any log, the
max-ballot entry and the count of entries carrying exactly that ballot. -/
def mergeQuorumLogs (logs : List (LogMap P nP)) :
    List (Nat × (Nat × LogValue P nP)) :=
  let entries := logs.flatten
  let slots := (entries.map Prod.fst).eraseDups
  slots.filterMap fun s =>
    let es := (entries.filter (·.1 == s)).map Prod.snd
    match es.foldl maxEntryStep none with
    | none => none
    | some best =>
      some (s, ((es.filter (fun e => e.ballot == best.ballot)).length, best))

/-- `recommit_after_leader_election` (paxos.rs:596–672). Returns
`(p_log_to_try_commit ++ p_log_holes, p_max_slot)`. -/
def recommitAfterLeaderElection (f : Nat)
    (quorumLogs : List (P1bPayload P nP)) (ballot : Ballot nP) :
    List ((Nat × Ballot nP) × Option P) × Option Nat :=
  let merged := mergeQuorumLogs (quorumLogs.map Prod.snd)
  -- p_log_to_try_commit (paxos.rs:640–654); checkpoint = none
  let toCommit := merged.filterMap fun (s, (count, entry)) =>
    if count > f then none else some ((s, ballot), entry.value)
  -- p_max_slot (paxos.rs:655)
  let maxSlot := (merged.map Prod.fst).foldl (fun acc s =>
    match acc with
    | none => some s
    | some a => some (Nat.max a s)) none
  -- p_log_holes (paxos.rs:657–669)
  let proposed := merged.map Prod.fst
  let holes := match maxSlot with
    | none => []
    | some m => ((List.range m).filter (fun s => !proposed.contains s)).map
        fun s => ((s, ballot), (none : Option P))
  (toCommit ++ holes, maxSlot)

/-! ## Module spec -/

/-- Fold invariant for max-ballot selection: the result comes from the
accumulator or the list, dominates the accumulator, and dominates every list
element. -/
theorem maxEntry_fold_spec (l : List (LogValue P nP))
    (acc : Option (LogValue P nP)) {best : LogValue P nP}
    (h : l.foldl maxEntryStep acc = some best) :
    (acc = some best ∨ best ∈ l) ∧
    (∀ a, acc = some a → a.ballot.ble best.ballot = true) ∧
    (∀ e ∈ l, e.ballot.ble best.ballot = true) := by
  induction l generalizing acc with
  | nil =>
    refine ⟨Or.inl h, fun a ha => ?_, fun e he => absurd he List.not_mem_nil⟩
    have h' : acc = some best := h
    rw [h'] at ha
    cases ha
    exact Ballot.ble_refl _
  | cons e rest ih =>
    rw [List.foldl_cons] at h
    obtain ⟨hsrc, hacc, hrest⟩ := ih (maxEntryStep acc e) h
    have hstep_le : ∀ a, acc = some a → a.ballot.ble best.ballot = true := by
      intro a ha
      subst ha
      by_cases hb : a.ballot.blt e.ballot
      · exact Ballot.ble_trans (Ballot.ble_of_blt hb)
          (hacc e (by simp [maxEntryStep, hb]))
      · exact hacc a (by simp [maxEntryStep, hb])
    have he_le : e.ballot.ble best.ballot = true := by
      cases hacc' : acc with
      | none => exact hacc e (by simp [maxEntryStep, hacc'])
      | some a =>
        by_cases hb : a.ballot.blt e.ballot
        · exact hacc e (by simp [maxEntryStep, hacc', hb])
        · exact Ballot.ble_trans (Ballot.ble_of_not_blt hb)
            (hacc a (by simp [maxEntryStep, hacc', hb]))
    refine ⟨?_, hstep_le, fun x hx => ?_⟩
    · rcases hsrc with hs | hmem
      · cases hacc' : acc with
        | none =>
          rw [hacc'] at hs
          simp only [maxEntryStep] at hs
          cases hs
          exact Or.inr List.mem_cons_self
        | some a =>
          rw [hacc'] at hs
          simp only [maxEntryStep] at hs
          by_cases hb : a.ballot.blt e.ballot
          · rw [if_pos hb] at hs
            cases hs
            exact Or.inr List.mem_cons_self
          · rw [if_neg hb] at hs
            cases hs
            exact Or.inl rfl
      · exact Or.inr (List.mem_cons_of_mem _ hmem)
    · rcases List.mem_cons.mp hx with rfl | hmem
      · exact he_le
      · exact hrest x hmem

/-- The merged entry for a slot dominates (by ballot) every quorum entry for
that slot — the property the phase-2 safety proof uses: a value chosen at
ballot `b` appears with the *max* ballot in any later quorum's merge, so the
recovering leader re-proposes it. -/
theorem mergeQuorumLogs_max_ballot (logs : List (LogMap P nP))
    {s : Nat} {count : Nat} {best : LogValue P nP}
    (h : (s, (count, best)) ∈ mergeQuorumLogs logs) :
    (s, best) ∈ logs.flatten ∧
    ∀ e : LogValue P nP, (s, e) ∈ logs.flatten →
      e.ballot.ble best.ballot = true := by
  unfold mergeQuorumLogs at h
  obtain ⟨s', hs', hmap⟩ := List.mem_filterMap.mp h
  simp only [] at hmap
  cases hfold : List.foldl maxEntryStep none
      ((logs.flatten.filter (·.1 == s')).map Prod.snd) with
  | none =>
    rw [hfold] at hmap
    cases hmap
  | some b =>
    rw [hfold] at hmap
    have hmap' : some (s', ((((logs.flatten.filter (·.1 == s')).map
        Prod.snd).filter (fun e => e.ballot == b.ballot)).length, b))
        = some (s, (count, best)) := hmap
    cases hmap'
    obtain ⟨hsrc, -, hall⟩ := maxEntry_fold_spec _ none hfold
    have hmem_of_es : ∀ x ∈ (logs.flatten.filter (·.1 == s)).map Prod.snd,
        (s, x) ∈ logs.flatten := by
      intro x hx
      obtain ⟨⟨s'', y⟩, hy, rfl⟩ := List.mem_map.mp hx
      have hs'' : s'' = s := by
        simpa [beq_iff_eq] using (List.mem_filter.mp hy).2
      subst hs''
      exact (List.mem_filter.mp hy).1
    refine ⟨?_, fun e he => ?_⟩
    · rcases hsrc with hs | hmem
      · cases hs
      · exact hmem_of_es _ hmem
    · refine hall e (List.mem_map.mpr ⟨(s, e), List.mem_filter.mpr ⟨he, ?_⟩, rfl⟩)
      simp

/-! ## Coverage, uniqueness, and dominance of the merge (safety-proof
interface) -/

/-- `filterMap` with a key-preserving function keeps a duplicate-free key
list duplicate-free. -/
theorem nodup_map_filterMap_of_key {α β κ : Type _} (l : List α)
    (f : α → Option β) (key : β → κ) (g : α → κ)
    (hkey : ∀ a b, f a = some b → key b = g a)
    (hnd : (l.map g).Nodup) : ((l.filterMap f).map key).Nodup := by
  induction l with
  | nil => exact List.Pairwise.nil
  | cons a rest ih =>
    rw [List.map_cons] at hnd
    rw [List.filterMap_cons]
    cases hfa : f a with
    | none => exact ih (List.Pairwise.of_cons hnd)
    | some b =>
      rw [List.map_cons]
      refine List.Pairwise.cons (fun x hx => ?_) (ih (List.Pairwise.of_cons hnd))
      obtain ⟨b', hb', rfl⟩ := List.mem_map.mp hx
      obtain ⟨a', ha', hfa'⟩ := List.mem_filterMap.mp hb'
      rw [hkey a b hfa, hkey a' b' hfa']
      exact List.rel_of_pairwise_cons hnd (List.mem_map.mpr ⟨a', ha', rfl⟩)

private theorem maxEntry_fold_isSome (l : List (LogValue P nP))
    (a : LogValue P nP) :
    (l.foldl maxEntryStep (some a)).isSome = true := by
  induction l generalizing a with
  | nil => rfl
  | cons e rest ih =>
    rw [List.foldl_cons]
    by_cases hb : a.ballot.blt e.ballot
    · rw [show maxEntryStep (some a) e = some e from by
        unfold maxEntryStep
        dsimp only
        rw [if_pos hb]]
      exact ih e
    · rw [show maxEntryStep (some a) e = some a from by
        unfold maxEntryStep
        dsimp only
        rw [if_neg hb]]
      exact ih a

/-- **Merge coverage**: every slot present in some quorum log is covered by
the merge, with a dominating (max-ballot) entry. -/
theorem mergeQuorumLogs_covers (logs : List (LogMap P nP))
    {s : Nat} {e : LogValue P nP} (h : (s, e) ∈ logs.flatten) :
    ∃ (count : Nat) (best : LogValue P nP),
      (s, (count, best)) ∈ mergeQuorumLogs logs ∧
      e.ballot.ble best.ballot = true := by
  -- the slot is among the deduplicated slots
  have hslot : s ∈ (logs.flatten.map Prod.fst).eraseDups := by
    rw [List.mem_eraseDups]
    exact List.mem_map.mpr ⟨(s, e), h, rfl⟩
  -- the filtered entry list is nonempty, so the max fold yields an entry
  have hes : e ∈ (logs.flatten.filter (·.1 == s)).map Prod.snd :=
    List.mem_map.mpr ⟨(s, e), List.mem_filter.mpr ⟨h, by simp⟩, rfl⟩
  cases hfold : List.foldl maxEntryStep none
      ((logs.flatten.filter (·.1 == s)).map Prod.snd) with
  | none =>
    -- impossible: fold over a list containing `e` is `some`
    exfalso
    obtain ⟨pre, post, heq⟩ := List.mem_iff_append.mp hes
    rw [heq, List.foldl_append, List.foldl_cons] at hfold
    have hsome : (post.foldl maxEntryStep
        (maxEntryStep (pre.foldl maxEntryStep none) e)).isSome = true := by
      cases hpre : pre.foldl maxEntryStep none with
      | none =>
        show (post.foldl maxEntryStep (some e)).isSome = true
        exact maxEntry_fold_isSome post e
      | some a =>
        by_cases hb : a.ballot.blt e.ballot
        · rw [show maxEntryStep (some a) e = some e from by
            unfold maxEntryStep
            dsimp only
            rw [if_pos hb]]
          exact maxEntry_fold_isSome post e
        · rw [show maxEntryStep (some a) e = some a from by
            unfold maxEntryStep
            dsimp only
            rw [if_neg hb]]
          exact maxEntry_fold_isSome post a
    rw [hfold] at hsome
    cases hsome
  | some best =>
    obtain ⟨-, -, hall⟩ := maxEntry_fold_spec _ none hfold
    refine ⟨(((logs.flatten.filter (·.1 == s)).map Prod.snd).filter
        (fun e => e.ballot == best.ballot)).length, best, ?_, hall e hes⟩
    unfold mergeQuorumLogs
    refine List.mem_filterMap.mpr ⟨s, hslot, ?_⟩
    simp only []
    rw [hfold]

/-- The merge has at most one entry per slot. -/
theorem mergeQuorumLogs_slots_nodup (logs : List (LogMap P nP)) :
    ((mergeQuorumLogs logs).map Prod.fst).Nodup := by
  unfold mergeQuorumLogs
  refine nodup_map_filterMap_of_key _ _ Prod.fst id ?_ ?_
  · intro s p hp
    dsimp only at hp
    split at hp
    · cases hp
    · cases hp
      rfl
  · rw [List.map_id]
    exact List.eraseDups_nodup _

/-- A keys-nodup list has equal entries at equal keys. -/
theorem eq_of_mem_of_nodup_keys {α κ : Type _} {l : List α} {g : α → κ}
    (hl : (l.map g).Nodup) {x y : α} (hx : x ∈ l) (hy : y ∈ l)
    (hxy : g x = g y) : x = y := by
  induction l with
  | nil => cases hx
  | cons hd rest ih =>
    rw [List.map_cons, List.nodup_cons] at hl
    rcases List.mem_cons.mp hx with rfl | hxr
    · rcases List.mem_cons.mp hy with rfl | hyr
      · rfl
      · exact absurd (List.mem_map.mpr ⟨y, hyr, hxy.symm⟩) hl.1
    · rcases List.mem_cons.mp hy with rfl | hyr
      · exact absurd (List.mem_map.mpr ⟨x, hxr, hxy⟩) hl.1
      · exact ih hl.2 hxr hyr

/-- Two merge entries at the same slot are equal (slot-uniqueness). -/
theorem mergeQuorumLogs_slot_unique (logs : List (LogMap P nP))
    {s : Nat} {c₁ c₂ : Nat} {b₁ b₂ : LogValue P nP}
    (h₁ : (s, (c₁, b₁)) ∈ mergeQuorumLogs logs)
    (h₂ : (s, (c₂, b₂)) ∈ mergeQuorumLogs logs) :
    c₁ = c₂ ∧ b₁ = b₂ := by
  have hnd := mergeQuorumLogs_slots_nodup logs
  have := eq_of_mem_of_nodup_keys hnd h₁ h₂ rfl
  have h2 := congrArg Prod.snd this
  exact ⟨congrArg Prod.fst h2, congrArg Prod.snd h2⟩


/-! ## Recommit output structure (paxos.rs:596–672) -/

section RecommitFacts

variable (f : Nat) (qlogs : List (P1bPayload P nP)) (b : Ballot nP)

/-- The merged quorum-log view. -/
def recommitMerged : List (Nat × Nat × LogValue P nP) :=
  mergeQuorumLogs (qlogs.map Prod.snd)

/-- `p_max_slot`. -/
def recommitMax : Option Nat :=
  ((recommitMerged qlogs).map Prod.fst).foldl
    (fun acc s =>
      match acc with
      | none => some s
      | some a => some (Nat.max a s)) none

/-- `p_log_to_try_commit`. -/
def recommitToCommit : List ((Nat × Ballot nP) × Option P) :=
  (recommitMerged qlogs).filterMap fun se =>
    if se.2.1 > f then none else some ((se.1, b), se.2.2.value)

/-- `p_log_holes`. -/
def recommitHoles : List ((Nat × Ballot nP) × Option P) :=
  match recommitMax (P := P) (nP := nP) qlogs with
  | none => []
  | some m =>
    ((List.range m).filter
      (fun s => !((recommitMerged qlogs).map Prod.fst).contains s)).map
      fun s => ((s, b), (none : Option P))

theorem recommit_eq :
    recommitAfterLeaderElection f qlogs b
      = (recommitToCommit f qlogs b ++ recommitHoles qlogs b,
         recommitMax qlogs) := rfl

variable {f qlogs b}

theorem recommit_ballot :
    ∀ e ∈ (recommitAfterLeaderElection f qlogs b).1,
      (e : (Nat × Ballot nP) × Option P).1.2 = b := by
  intro e he
  rw [recommit_eq] at he
  rcases List.mem_append.mp he with he | he
  · obtain ⟨se, -, hx⟩ := List.mem_filterMap.mp he
    by_cases hc : se.2.1 > f
    · rw [if_pos hc] at hx
      cases hx
    · rw [if_neg hc] at hx
      cases hx
      rfl
  · unfold recommitHoles at he
    revert he
    cases recommitMax (P := P) (nP := nP) qlogs with
    | none => intro he; cases he
    | some m =>
      intro he
      obtain ⟨s, -, rfl⟩ := List.mem_map.mp he
      rfl

/-- Fold-max dominates the list. -/
private theorem foldMax_le : ∀ (l : List Nat) (acc : Option Nat) (m : Nat),
    l.foldl (fun acc s =>
      match acc with
      | none => some s
      | some a => some (Nat.max a s)) acc = some m →
    (∀ s ∈ l, s ≤ m) ∧ (∀ a, acc = some a → a ≤ m)
  | [], acc, m => by
    intro h
    rw [List.foldl_nil] at h
    refine ⟨fun s hs => (List.not_mem_nil hs).elim, fun a ha => ?_⟩
    rw [ha] at h
    cases h
    exact Nat.le_refl _
  | s :: l, acc, m => by
    intro h
    rw [List.foldl_cons] at h
    have hrec := foldMax_le l _ m h
    cases acc with
    | none =>
      refine ⟨fun s' hs' => ?_, fun a ha => by cases ha⟩
      rcases List.mem_cons.mp hs' with rfl | hs'
      · exact (hrec.2 s' rfl)
      · exact hrec.1 s' hs'
    | some a =>
      have hsm : Nat.max a s ≤ m := hrec.2 _ rfl
      refine ⟨fun s' hs' => ?_, fun a' ha' => ?_⟩
      · rcases List.mem_cons.mp hs' with rfl | hs'
        · exact Nat.le_trans (Nat.le_max_right a s') hsm
        · exact hrec.1 s' hs'
      · cases Option.some.inj ha'
        exact Nat.le_trans (Nat.le_max_left _ s) hsm

/-- Fold-max from `none` is `none` only on the empty list. -/
private theorem foldMax_none : ∀ (l : List Nat),
    l.foldl (fun acc s =>
      match acc with
      | none => some s
      | some a => some (Nat.max a s)) none = none → l = [] := by
  have hsome : ∀ (l : List Nat) (a : Nat), l.foldl (fun acc s =>
      match acc with
      | none => some s
      | some a => some (Nat.max a s)) (some a) ≠ none := by
    intro l
    induction l with
    | nil => intro a h; cases h
    | cons s l ih =>
      intro a h
      rw [List.foldl_cons] at h
      exact ih _ h
  intro l h
  cases l with
  | nil => rfl
  | cons s l =>
    rw [List.foldl_cons] at h
    exact absurd h (hsome l s)

theorem recommit_none
    (h : (recommitAfterLeaderElection f qlogs b).2 = none) :
    (recommitAfterLeaderElection f qlogs b).1 = [] := by
  rw [recommit_eq] at h ⊢
  have hmax : recommitMax (P := P) (nP := nP) qlogs = none := h
  have hmerged : recommitMerged (P := P) (nP := nP) qlogs = [] := by
    have := foldMax_none _ hmax
    cases hm : recommitMerged (P := P) (nP := nP) qlogs with
    | nil => rfl
    | cons x xs =>
      rw [hm, List.map_cons] at this
      cases this
  show recommitToCommit f qlogs b ++ recommitHoles qlogs b = []
  unfold recommitToCommit recommitHoles
  rw [hmerged, hmax]
  rfl

theorem recommit_slot_le {ms : Nat}
    (h : (recommitAfterLeaderElection f qlogs b).2 = some ms) :
    ∀ e ∈ (recommitAfterLeaderElection f qlogs b).1,
      (e : (Nat × Ballot nP) × Option P).1.1 ≤ ms := by
  rw [recommit_eq] at h ⊢
  have hmax : recommitMax (P := P) (nP := nP) qlogs = some ms := h
  intro e he
  rcases List.mem_append.mp he with he | he
  · obtain ⟨se, hse, hx⟩ := List.mem_filterMap.mp he
    by_cases hc : se.2.1 > f
    · rw [if_pos hc] at hx
      cases hx
    · rw [if_neg hc] at hx
      cases hx
      exact (foldMax_le _ _ _ hmax).1 se.1 (List.mem_map.mpr ⟨se, hse, rfl⟩)
  · unfold recommitHoles at he
    rw [hmax] at he
    obtain ⟨s, hs, rfl⟩ := List.mem_map.mp he
    have := List.mem_range.mp (List.mem_filter.mp hs).1
    show s ≤ ms
    omega

/-- A merged slot is bounded by `p_max_slot`. -/
theorem recommitMax_mem_le {s : Nat}
    (h : s ∈ (recommitMerged (P := P) (nP := nP) qlogs).map Prod.fst) :
    ∃ m, recommitMax (P := P) (nP := nP) qlogs = some m ∧ s ≤ m := by
  cases hm : recommitMax (P := P) (nP := nP) qlogs with
  | none =>
    exfalso
    have hnil := foldMax_none _ hm
    rw [hnil] at h
    cases h
  | some m => exact ⟨m, rfl, (foldMax_le _ _ _ hm).1 s h⟩

/-- `p_log_to_try_commit` membership, inverted: each entry quotes a merged
slot with a low report count, at OUR ballot, carrying the max-ballot
entry's value. -/
theorem recommitToCommit_mem {e : (Nat × Ballot nP) × Option P}
    (h : e ∈ recommitToCommit f qlogs b) :
    ∃ cnt entry, (e.1.1, (cnt, entry)) ∈ recommitMerged (P := P) qlogs ∧
      cnt ≤ f ∧ e.1.2 = b ∧ e.2 = entry.value := by
  obtain ⟨se, hse, hx⟩ := List.mem_filterMap.mp h
  by_cases hc : se.2.1 > f
  · rw [if_pos hc] at hx
    cases hx
  · rw [if_neg hc] at hx
    cases hx
    exact ⟨se.2.1, se.2.2, hse, by omega, rfl, rfl⟩

/-- `p_log_holes` entries are at *uncovered* slots. -/
theorem recommitHoles_not_covered {e : (Nat × Ballot nP) × Option P}
    (h : e ∈ recommitHoles qlogs b) :
    e.1.1 ∉ (recommitMerged (P := P) (nP := nP) qlogs).map Prod.fst := by
  unfold recommitHoles at h
  revert h
  cases recommitMax (P := P) (nP := nP) qlogs with
  | none =>
    intro h
    cases h
  | some m =>
    intro h hmem
    obtain ⟨s, hs, rfl⟩ := List.mem_map.mp h
    have hcont := (List.mem_filter.mp hs).2
    rw [List.contains_iff_mem.mpr hmem] at hcont
    cases hcont

theorem recommit_slots_nodup :
    (((recommitAfterLeaderElection f qlogs b).1).map
      (fun e => e.1.1)).Nodup := by
  rw [recommit_eq]
  show ((recommitToCommit f qlogs b ++ recommitHoles qlogs b).map
    (fun e => e.1.1)).Nodup
  rw [List.map_append]
  have hmerged := mergeQuorumLogs_slots_nodup
    (P := P) (nP := nP) (qlogs.map Prod.snd)
  refine List.nodup_append.mpr ⟨?_, ?_, ?_⟩
  · -- toCommit slots: a slot-preserving selection of the (nodup) merged
    have hsel : ∀ (l : List (Nat × Nat × LogValue P nP)),
        (l.map Prod.fst).Nodup →
        ((l.filterMap (fun se =>
          if se.2.1 > f then none
          else some ((se.1, b), se.2.2.value))).map
            (fun e => e.1.1)).Nodup := by
      intro l
      induction l with
      | nil => intro _; exact List.nodup_nil
      | cons x xs ih =>
        intro hnd
        rw [List.map_cons] at hnd
        have hx := (List.nodup_cons.mp hnd).1
        have hxs := (List.nodup_cons.mp hnd).2
        rw [List.filterMap_cons]
        by_cases hc : x.2.1 > f
        · rw [if_pos hc]
          exact ih hxs
        · rw [if_neg hc]
          rw [List.map_cons]
          refine List.nodup_cons.mpr ⟨fun hmem => ?_, ih hxs⟩
          obtain ⟨e, he, he2⟩ := List.mem_map.mp hmem
          obtain ⟨se, hse, hse2⟩ := List.mem_filterMap.mp he
          by_cases hc2 : se.2.1 > f
          · rw [if_pos hc2] at hse2
            cases hse2
          · rw [if_neg hc2] at hse2
            cases hse2
            have : se.1 = x.1 := he2
            exact hx (List.mem_map.mpr ⟨se, hse, this⟩)
    exact hsel _ hmerged
  · -- hole slots
    unfold recommitHoles
    cases recommitMax (P := P) (nP := nP) qlogs with
    | none => exact List.nodup_nil
    | some m =>
      rw [List.map_map]
      have hnd : ((List.range m).filter (fun s =>
          !((recommitMerged (P := P) (nP := nP) qlogs).map
            Prod.fst).contains s)).Nodup :=
        (List.nodup_range).filter _
      show (((List.range m).filter _).map (fun s => s)).Nodup
      rw [List.map_id']
      exact hnd
  · -- disjoint: toCommit slots are merged slots, holes are not
    intro s hs s' hs'
    intro hss
    subst hss
    have hmem : s ∈ (recommitMerged (P := P) (nP := nP) qlogs).map
        Prod.fst := by
      obtain ⟨e, he, rfl⟩ := List.mem_map.mp hs
      obtain ⟨se, hse, hse2⟩ := List.mem_filterMap.mp he
      by_cases hc : se.2.1 > f
      · rw [if_pos hc] at hse2
        cases hse2
      · rw [if_neg hc] at hse2
        cases hse2
        exact List.mem_map.mpr ⟨se, hse, rfl⟩
    revert hs'
    show s ∈ (recommitHoles (P := P) (nP := nP) qlogs b).map
      (fun e => e.1.1) → False
    unfold recommitHoles
    cases recommitMax (P := P) (nP := nP) qlogs with
    | none => intro hs'; cases hs'
    | some m =>
      intro hs'
      rw [List.map_map] at hs'
      obtain ⟨s'', hs'', heq⟩ := List.mem_map.mp hs'
      have hss : s'' = s := heq
      subst hss
      have hcont := (List.mem_filter.mp hs'').2
      rw [List.contains_iff_mem.mpr hmem] at hcont
      cases hcont

end RecommitFacts

end HydroLean.Programs.Paxos
