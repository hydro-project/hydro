import HydroLean.Hydro.Stream

/-!
# Paxos port: core types (Rust: `hydro_test/src/cluster/paxos.rs` lines 19–76)

Types for the Lean port of `paxos_core`. The proposer cluster has `nP` members
and the acceptor cluster `nA` members (cluster members are `Fin n`, matching
`MemberId<L>` erased to its raw id, §4.4.2).

`Ballot` mirrors paxos.rs lines 38–56: ordered by `num`, tie-broken by
`proposer_id` (`impl Ord for Ballot`), which guarantees **ballot uniqueness**:
distinct proposers never own the same ballot — by construction of the type.
-/

namespace HydroLean.Programs.Paxos

/-- Rust `Ballot { num: u32, proposer_id: MemberId<Proposer> }`
(paxos.rs:38–42). -/
structure Ballot (nP : Nat) where
  num : Nat
  proposerId : Fin nP
deriving DecidableEq, Repr

namespace Ballot

@[simp] theorem num_mk {nP : Nat} (n : Nat) (p : Fin nP) :
    (Ballot.mk n p).num = n := rfl

@[simp] theorem proposerId_mk {nP : Nat} (n : Nat) (p : Fin nP) :
    (Ballot.mk n p).proposerId = p := rfl

/-- Strict lexicographic order: `num` first, then `proposer_id`
(paxos.rs:44–50). -/
def blt {nP : Nat} (a b : Ballot nP) : Bool :=
  a.num < b.num || (a.num = b.num && a.proposerId < b.proposerId)

/-- Non-strict lexicographic order. -/
def ble {nP : Nat} (a b : Ballot nP) : Bool :=
  a.num < b.num || (a.num = b.num && a.proposerId ≤ b.proposerId)

instance {nP : Nat} : LT (Ballot nP) := ⟨fun a b => blt a b⟩
instance {nP : Nat} : LE (Ballot nP) := ⟨fun a b => ble a b⟩
instance {nP : Nat} (a b : Ballot nP) : Decidable (a < b) := by
  unfold LT.lt instLT; exact inferInstance
instance {nP : Nat} (a b : Ballot nP) : Decidable (a ≤ b) := by
  unfold LE.le instLE; exact inferInstance

/-- Max of two ballots (used for `max()` folds over ballot streams). -/
def max {nP : Nat} (a b : Ballot nP) : Ballot nP := if a.blt b then b else a

theorem blt_iff {nP : Nat} {a b : Ballot nP} :
    a.blt b = true ↔ a.num < b.num ∨ (a.num = b.num ∧ a.proposerId.val < b.proposerId.val) := by
  unfold blt
  simp [Bool.or_eq_true_iff, Bool.and_eq_true_iff, decide_eq_true_iff, Fin.lt_def]

theorem ble_iff {nP : Nat} {a b : Ballot nP} :
    a.ble b = true ↔ a.num < b.num ∨ (a.num = b.num ∧ a.proposerId.val ≤ b.proposerId.val) := by
  unfold ble
  simp [Bool.or_eq_true_iff, Bool.and_eq_true_iff, decide_eq_true_iff, Fin.le_def]

/-- Order-API completer (extensionality; kept with `blt_trans`/`blt_irrefl`
as the `Ballot` strict/weak order interface, whether or not each lemma is
currently consumed). -/
theorem ext' {nP : Nat} {a b : Ballot nP} (hn : a.num = b.num)
    (hp : a.proposerId = b.proposerId) : a = b := by
  cases a; cases b; simp_all

/-- Totality: if `¬(a < b)` then `b ≤ a`. -/
theorem ble_of_not_blt {nP : Nat} {a b : Ballot nP}
    (h : ¬ a.blt b = true) : b.ble a = true := by
  rw [Ballot.blt_iff] at h
  rw [Ballot.ble_iff]
  omega

/-- `≤` splits into `=` or `<`. -/
theorem eq_or_blt_of_ble {nP : Nat} {a b : Ballot nP}
    (h : a.ble b = true) : a = b ∨ a.blt b = true := by
  rw [Ballot.ble_iff] at h
  by_cases hn : a.num = b.num
  · by_cases hp : a.proposerId.val = b.proposerId.val
    · exact Or.inl (ext' hn (Fin.val_inj.mp hp))
    · exact Or.inr (Ballot.blt_iff.mpr (by omega))
  · exact Or.inr (Ballot.blt_iff.mpr (by omega))

theorem ble_refl {nP : Nat} (a : Ballot nP) : a.ble a = true :=
  ble_iff.mpr (Or.inr ⟨rfl, Nat.le_refl _⟩)

theorem ble_trans {nP : Nat} {a b c : Ballot nP} (h₁ : a.ble b = true)
    (h₂ : b.ble c = true) : a.ble c = true := by
  rw [ble_iff] at *
  omega

theorem ble_of_blt {nP : Nat} {a b : Ballot nP} (h : a.blt b = true) :
    a.ble b = true := by
  rw [blt_iff] at h
  rw [ble_iff]
  omega

theorem blt_trans {nP : Nat} {a b c : Ballot nP} (h₁ : a.blt b = true)
    (h₂ : b.blt c = true) : a.blt c = true := by
  rw [blt_iff] at *
  omega

/-- Max of an optional ballot and a ballot. -/
def maxOpt {nP : Nat} : Option (Ballot nP) → Ballot nP → Ballot nP
  | none, b => b
  | some a, b => a.max b

/-- Fold `max` over a list of ballots starting from an optional current max
(Rust: `stream.max().into_singleton()` merged across ticks,
paxos.rs:281–285). -/
def maxList {nP : Nat} (cur : Option (Ballot nP)) (l : List (Ballot nP)) :
    Option (Ballot nP) :=
  l.foldl (fun acc b => some (maxOpt acc b)) cur

/-- `Option Ballot ≤ Ballot`-style comparison used by `p_has_largest_ballot`
(paxos.rs:386–390: `received_max_ballot <= Some(cur_ballot)`). `none` is the
bottom element. -/
def optLe {nP : Nat} : Option (Ballot nP) → Ballot nP → Bool
  | none, _ => true
  | some a, b => a.ble b

/-- `blt` is irreflexive. -/
theorem blt_irrefl {nP : Nat} (b : Ballot nP) : b.blt b = false := by
  rw [Bool.eq_false_iff]
  intro h
  rw [Ballot.blt_iff] at h
  rcases h with h | ⟨-, h⟩
  · omega
  · omega

/-- `ble` and strict `blt` in opposite directions contradict. -/
theorem ble_blt_asymm {nP : Nat} {a b : Ballot nP} (h₁ : a.ble b = true)
    (h₂ : b.blt a = true) : False := by
  rw [Ballot.ble_iff] at h₁
  rw [Ballot.blt_iff] at h₂
  rcases h₁ with h₁ | ⟨h₁, h₁'⟩ <;> rcases h₂ with h₂ | ⟨h₂, h₂'⟩ <;> omega

end Ballot

/-! ## Option-level ballot order (the growth order of `a_max_ballot`) -/

/-- `none`-bottomed ballot order: the growth order of `a_max_ballot`. -/
def obtLE {nP : Nat} : Option (Ballot nP) → Option (Ballot nP) → Prop
  | none, _ => True
  | some _, none => False
  | some a, some b => a.ble b = true

theorem obtLE_refl {nP : Nat} (o : Option (Ballot nP)) : obtLE o o := by
  cases o with
  | none => trivial
  | some a => exact Ballot.ble_refl a

theorem obtLE_trans {nP : Nat} {a b c : Option (Ballot nP)} (h₁ : obtLE a b)
    (h₂ : obtLE b c) : obtLE a c := by
  cases a with
  | none => trivial
  | some x =>
    cases b with
    | none => cases h₁
    | some y =>
      cases c with
      | none => cases h₂
      | some z => exact Ballot.ble_trans h₁ h₂

/-- Rust `LogValue<P> { ballot, value }` (paxos.rs:58–62); `value = none`
represents a hole re-committed by a recovering leader. -/
structure LogValue (P : Type) (nP : Nat) where
  ballot : Ballot nP
  value : Option P
deriving DecidableEq, Repr

/-- Rust `P2a<P, S> { sender, ballot, slot, value }` (paxos.rs:64–70). -/
structure P2a (P : Type) (nP : Nat) where
  sender : Fin nP
  ballot : Ballot nP
  slot : Nat
  value : Option P
deriving DecidableEq, Repr

/-- An acceptor's log: a slot-keyed association list of accepted entries
(Rust: keyed state `reduce_watermark` + `HashMap` snapshot, paxos.rs:851–873).
Keys are kept unique by construction of `logInsert`. -/
abbrev LogMap (P : Type) (nP : Nat) : Type := List (Nat × LogValue P nP)

/-- Look up a slot in the log. -/
def LogMap.find? {P : Type} {nP : Nat} (log : LogMap P nP) (slot : Nat) :
    Option (LogValue P nP) :=
  (List.find? (fun e => e.1 == slot) log).map Prod.snd

@[simp] theorem LogMap.find?_nil {P : Type} {nP : Nat} (s : Nat) :
    LogMap.find? ([] : LogMap P nP) s = none := rfl

theorem LogMap.find?_cons {P : Type} {nP : Nat} (s' : Nat)
    (w : LogValue P nP) (rest : LogMap P nP) (s : Nat) :
    LogMap.find? ((s', w) :: rest) s
      = if s' = s then some w else LogMap.find? rest s := by
  simp only [LogMap.find?, List.find?]
  by_cases h : s' = s
  · simp [h]
  · simp [beq_eq_false_iff_ne.mpr h, h]

/-- Insert an accepted entry: keep the entry with the higher ballot
(paxos.rs:851–864 `reduce_watermark`: "Insert p2a into the log if it has a
higher ballot than what was there before"; ties keep the existing entry). -/
def LogMap.insertMax {P : Type} {nP : Nat} (log : LogMap P nP) (slot : Nat)
    (v : LogValue P nP) : LogMap P nP :=
  match log with
  | [] => [(slot, v)]
  | (s, w) :: rest =>
    if s = slot then
      if w.ballot.blt v.ballot then (s, v) :: rest else (s, w) :: rest
    else
      (s, w) :: LogMap.insertMax rest slot v

/-- A found entry is a member (`find?`-to-membership). -/
theorem LogMap.find?_mem {P : Type} {nP : Nat} {log : LogMap P nP}
    {slot : Nat} {e : LogValue P nP} (h : log.find? slot = some e) :
    (slot, e) ∈ log := by
  induction log with
  | nil => cases h
  | cons hd rest ih =>
    obtain ⟨s', w⟩ := hd
    rw [LogMap.find?_cons] at h
    by_cases hs : s' = slot
    · rw [if_pos hs] at h
      cases h
      exact hs ▸ List.mem_cons_self ..
    · rw [if_neg hs] at h
      exact List.mem_cons_of_mem _ (ih h)

/-- Entries after an insert are old entries or the inserted one
(`insertMax` membership cases). -/
theorem LogMap.insertMax_mem_cases {P : Type} {nP : Nat} {log : LogMap P nP}
    {slot : Nat} {v : LogValue P nP} {x : Nat × LogValue P nP}
    (h : x ∈ log.insertMax slot v) : x ∈ log ∨ x = (slot, v) := by
  induction log with
  | nil =>
    rcases List.mem_singleton.mp h with rfl
    exact Or.inr rfl
  | cons hd rest ih =>
    obtain ⟨s, w⟩ := hd
    simp only [LogMap.insertMax] at h
    by_cases hs : s = slot
    · rw [if_pos hs] at h
      by_cases hb : w.ballot.blt v.ballot
      · rw [if_pos hb] at h
        rcases List.mem_cons.mp h with rfl | hmem
        · exact Or.inr (by rw [hs])
        · exact Or.inl (List.mem_cons_of_mem _ hmem)
      · rw [if_neg hb] at h
        exact Or.inl h
    · rw [if_neg hs] at h
      rcases List.mem_cons.mp h with rfl | hmem
      · exact Or.inl (List.mem_cons_self ..)
      · rcases ih hmem with hold | hnew
        · exact Or.inl (List.mem_cons_of_mem _ hold)
        · exact Or.inr hnew

/-- The p1b message: `(Ballot, Result<(Option<usize>, LogMap), Option<Ballot>>)`
(paxos.rs:485–496 `acceptor_p1`): `ok` carries the checkpoint and the
acceptor's accepted log; `error` carries the acceptor's larger max ballot. -/
structure P1b (P : Type) (nP : Nat) where
  ballot : Ballot nP
  res : Except (Option (Ballot nP)) (Option Nat × LogMap P nP)

/-- The p2b message: `((slot, Ballot), Result<(), Option<Ballot>>)`
(paxos.rs:875–890). The `(slot, ballot)` pair is the quorum-counting key. -/
structure P2b (nP : Nat) where
  slot : Nat
  ballot : Ballot nP
  res : Except (Option (Ballot nP)) Unit

/-- Model variant: `faithful` mirrors paxos.rs exactly; `guarded` adds the
minimal contract-restoring fixes for FINDINGS.md B1 (P1a re-broadcasts of the
same ballot) and B2 (every-tick re-recommit) — see `Paxos/Falsification.lean`
for the executable violations of the faithful variant. -/
structure PaxosVariant where
  p1aSendOnce : Bool
  recommitOnce : Bool
deriving Repr, DecidableEq

/-- Mirrors paxos.rs as written. -/
def PaxosVariant.faithful : PaxosVariant := ⟨false, false⟩
/-- Minimal fixes restoring the component usage contracts. -/
def PaxosVariant.guarded : PaxosVariant := ⟨true, true⟩

end HydroLean.Programs.Paxos
